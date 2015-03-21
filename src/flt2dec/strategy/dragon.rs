/*
almost direct (but slightly optimized) Rust translation of Figure 3 of [1].

[1] Burger, R. G. and Dybvig, R. K. 1996. Printing floating-point numbers
    quickly and accurately. SIGPLAN Not. 31, 5 (May. 1996), 108-116.
*/

use core::prelude::*;
use core::num::{Int, Float};
use core::cmp::Ordering;

use flt2dec::{Decoded, MAX_SIG_DIGITS, round_up};
use flt2dec::estimator::estimate_scaling_factor;
use flt2dec::bignum::Digit32 as Digit;
use flt2dec::bignum::Big32x36 as Big;
#[cfg(test)] use std::{i16, f64};
#[cfg(test)] use flt2dec::testing;

// XXX const ref to static array seems to ICE (#22540)
static POW10: [Digit; 10] = [1, 10, 100, 1000, 10000, 100000,
                             1000000, 10000000, 100000000, 1000000000];
static TWOPOW10: [Digit; 10] = [2, 20, 200, 2000, 20000, 200000,
                                2000000, 20000000, 200000000, 2000000000];

// precalculated arrays of `Digit`s for 10^(2^n)
static POW10TO16: [Digit; 2] = [0x6fc10000, 0x2386f2];
static POW10TO32: [Digit; 4] = [0, 0x85acef81, 0x2d6d415b, 0x4ee];
static POW10TO64: [Digit; 7] = [0, 0, 0xbf6a1f01, 0x6e38ed64, 0xdaa797ed, 0xe93ff9f4, 0x184f03];
static POW10TO128: [Digit; 14] =
    [0, 0, 0, 0, 0x2e953e01, 0x3df9909, 0xf1538fd, 0x2374e42f, 0xd3cff5ec, 0xc404dc08,
     0xbccdb0da, 0xa6337f19, 0xe91f2603, 0x24e];
static POW10TO256: [Digit; 27] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0x982e7c01, 0xbed3875b, 0xd8d99f72, 0x12152f87, 0x6bde50c6,
     0xcf4a6e70, 0xd595d80f, 0x26b2716e, 0xadc666b0, 0x1d153624, 0x3c42d35a, 0x63ff540e,
     0xcc5573c0, 0x65f9ef17, 0x55bc28f2, 0x80dcc7f7, 0xf46eeddc, 0x5fdcefce, 0x553f7];

fn mul_pow10(mut x: Big, n: usize) -> Big {
    debug_assert!(n < 512);
    if n &   7 != 0 { x = x.mul_small(POW10[n & 7]); }
    if n &   8 != 0 { x = x.mul_small(POW10[8]); }
    if n &  16 != 0 { x = x.mul_digits(&POW10TO16); }
    if n &  32 != 0 { x = x.mul_digits(&POW10TO32); }
    if n &  64 != 0 { x = x.mul_digits(&POW10TO64); }
    if n & 128 != 0 { x = x.mul_digits(&POW10TO128); }
    if n & 256 != 0 { x = x.mul_digits(&POW10TO256); }
    x
}

fn div_2pow10(mut x: Big, mut n: usize) -> Big {
    let largest = POW10.len() - 1;
    while n > largest {
        x = x.div_rem_small(POW10[largest]).0;
        n -= largest;
    }
    x.div_rem_small(TWOPOW10[n]).0
}

#[cfg(test)] #[test]
fn test_mul_pow10() {
    let mut prevpow10 = Big::from_small(1);
    for i in 1..340 {
        let curpow10 = mul_pow10(Big::from_small(1), i);
        assert_eq!(curpow10, prevpow10.mul_small(10));
        prevpow10 = curpow10;
    }
}

// only usable when `x < 16 * scale`; `scaleN` should be `scale.mul_small(N)`
fn div_rem_upto_16(mut x: Big, scale: &Big, scale2: &Big, scale4: &Big, scale8: &Big) -> (u8, Big) {
    let mut d = 0;
    if x >= *scale8 { x = x.sub(scale8); d += 8; }
    if x >= *scale4 { x = x.sub(scale4); d += 4; }
    if x >= *scale2 { x = x.sub(scale2); d += 2; }
    if x >= *scale  { x = x.sub(scale);  d += 1; }
    debug_assert!(x < *scale);
    (d, x)
}

pub fn format_shortest(d: &Decoded, buf: &mut [u8]) -> (/*#digits*/ usize, /*exp*/ i16) {
    // the number `v` to format is known to be:
    // - equal to `mant * 2^exp`;
    // - preceded by `(mant - 2 * minus) * 2^exp` in the original type; and
    // - followed by `(mant + 2 * plus) * 2^exp` in the original type.
    //
    // obviously, `minus` and `plus` cannot be zero. (for infinities, we use out-of-range values.)
    // also we assume that at least one digit is generated, i.e. `mant` cannot be zero too.
    //
    // this also means that any number between `low = (mant - minus) * 2^exp` and
    // `high = (mant + plus) * 2^exp` will map to this exact floating point number,
    // with bounds included when the original mantissa was even (i.e. `!mant_was_odd`).

    assert!(d.mant > 0);
    assert!(d.minus > 0);
    assert!(d.plus > 0);
    assert!(d.mant.checked_add(d.plus).is_some());
    assert!(d.mant.checked_sub(d.minus).is_some());
    assert!(buf.len() >= MAX_SIG_DIGITS);

    // `a.cmp(&b) < rounding` is `if d.inclusive {a <= b} else {a < b}`
    let rounding = if d.inclusive {Ordering::Greater} else {Ordering::Equal};

    // estimate `k_0` from original inputs satisfying `10^(k_0-1) < high <= 10^(k_0+1)`.
    // the tight bound `k` satisfying `10^(k-1) < high <= 10^k` is calculated later.
    let mut k = estimate_scaling_factor(d.mant + d.plus, d.exp);

    // convert `{mant, plus, minus} * 2^exp` into the fractional form so that:
    // - `v = mant / scale`
    // - `low = (mant - minus) / scale`
    // - `high = (mant + plus) / scale`
    let mut mant = Big::from_u64(d.mant);
    let mut minus = Big::from_u64(d.minus);
    let mut plus = Big::from_u64(d.plus);
    let mut scale = Big::from_small(1);
    if d.exp < 0 {
        scale = scale.mul_pow2(-d.exp as usize);
    } else {
        mant = mant.mul_pow2(d.exp as usize);
        minus = minus.mul_pow2(d.exp as usize);
        plus = plus.mul_pow2(d.exp as usize);
    }

    // divide `mant` by `10^k`. now `scale / 10 < mant + plus <= scale * 10`.
    if k >= 0 {
        scale = mul_pow10(scale, k as usize);
    } else {
        mant = mul_pow10(mant, -k as usize);
        minus = mul_pow10(minus, -k as usize);
        plus = mul_pow10(plus, -k as usize);
    }

    // fixup when `mant + plus > scale` (or `>=`).
    // we are not actually modifying `scale`, since we can skip the initial multiplication instead.
    // now `scale < mant + plus <= scale * 10` and we are ready to generate digits.
    //
    // note that `d[0]` *can* be zero, when `scale - plus < mant < scale`.
    // in this case rounding-up condition (`up` below) will be triggered immediately.
    if scale.cmp(&mant.clone().add(&plus)) < rounding {
        // equivalent to scaling `scale` by 10
        k += 1;
    } else {
        mant = mant.mul_small(10);
        minus = minus.mul_small(10);
        plus = plus.mul_small(10);
    }

    // cache `(2, 4, 8) * scale` for digit generation.
    let scale2 = scale.mul_pow2(1);
    let scale4 = scale.mul_pow2(2);
    let scale8 = scale.mul_pow2(3);

    let mut down;
    let mut up;
    let mut i = 0;
    loop {
        // invariants, where `d[0..n-1]` are digits generated so far:
        // - `v = mant / scale * 10^(k-n-1) + d[0..n-1] * 10^(k-n)`
        // - `v - low = minus / scale * 10^(k-n-1)`
        // - `high - v = plus / scale * 10^(k-n-1)`
        // - `(mant + plus) / scale <= 10` (thus `mant / scale < 10`)
        // where `d[i..j]` is a shorthand for `d[i] * 10^(j-i) + ... + d[j-1] * 10 + d[j]`.

        // generate one digit: `d[n] = floor(mant / scale) < 10`.
        let (d, rem) = div_rem_upto_16(mant, &scale, &scale2, &scale4, &scale8);
        mant = rem;
        debug_assert!(d < 10);
        buf[i] = b'0' + d;
        i += 1;

        // this is a simplified description of the modified Dragon algorithm.
        // many intermediate derivations and completeness arguments are omitted for convenience.
        //
        // start with modified invariants, as we've updated `n`:
        // - `v = mant / scale * 10^(k-n) + d[0..n-1] * 10^(k-n)`
        // - `v - low = minus / scale * 10^(k-n)`
        // - `high - v = plus / scale * 10^(k-n)`
        //
        // assume that `d[0..n-1]` is the shortest representation between `low` and `high`,
        // i.e. `d[0..n-1]` satisfies both of the following but `d[0..n-2]` doesn't:
        // - `low < d[0..n-1] * 10^(k-n) < high` (bijectivity: digits round to `v`); and
        // - `abs(v / 10^(k-n) - d[0..n-1]) <= 1/2` (the last digit is correct).
        //
        // the second condition simplifies to `2 * mant <= scale`.
        // solving invariants in terms of `mant`, `low` and `high` yields
        // a simpler version of the first condition: `-plus < mant < minus`.
        // since `-plus < 0 <= mant`, we have the correct shortest representation
        // when `mant < minus` and `2 * mant <= scale`.
        // (the former becomes `mant <= minus` when the original mantissa is even.)
        //
        // when the second doesn't hold (`2 * mant > scale`), we need to increase the last digit.
        // this is enough for restoring that condition: we already know that
        // the digit generation guarantees `0 <= v / 10^(k-n) - d[0..n-1] < 1`.
        // in this case, the first condition becomes `-plus < mant - scale < minus`.
        // since `mant < scale` after the generation, we have `scale < mant + plus`.
        // (again, this becomes `scale <= mant + plus` when the original mantissa is even.)
        //
        // in short:
        // - stop and round `down` (keep digits as is) when `mant < minus` (or `<=`).
        // - stop and round `up` (increase the last digit) when `scale < mant + plus` (or `<=`).
        // - keep generating otherwise.
        down = mant.cmp(&minus) < rounding;
        up = scale.cmp(&mant.clone().add(&plus)) < rounding;
        if down || up { break; } // we have the shortest representation, proceed to the rounding

        // restore the invariants.
        // this makes the algorithm always terminating: `minus` and `plus` always increases,
        // but `mant` is clipped modulo `scale` and `scale` is fixed.
        mant = mant.mul_small(10);
        minus = minus.mul_small(10);
        plus = plus.mul_small(10);
    }

    // rounding up happens when
    // i) only the rounding-up condition was triggered, or
    // ii) both conditions were triggered and tie breaking prefers rounding up.
    if up && (!down || mant.mul_pow2(1) >= scale) {
        // if rounding up changes the length, the exponent should also change.
        // it seems that this condition is very hard to satisfy (possibly impossible),
        // but we are just being safe and consistent here.
        if let Some(c) = round_up(buf, i) {
            buf[i] = c;
            i += 1;
            k += 1;
        }
    }

    (i, k)
}

pub fn format_exact(d: &Decoded, buf: &mut [u8], limit: i16) -> (/*#digits*/ usize, /*exp*/ i16) {
    assert!(d.mant > 0);
    assert!(d.minus > 0);
    assert!(d.plus > 0);
    assert!(d.mant.checked_add(d.plus).is_some());
    assert!(d.mant.checked_sub(d.minus).is_some());

    // estimate `k_0` from original inputs satisfying `10^(k_0-1) < v <= 10^(k_0+1)`.
    let mut k = estimate_scaling_factor(d.mant, d.exp);

    // `v = mant / scale`.
    let mut mant = Big::from_u64(d.mant);
    let mut scale = Big::from_small(1);
    if d.exp < 0 {
        scale = scale.mul_pow2(-d.exp as usize);
    } else {
        mant = mant.mul_pow2(d.exp as usize);
    }

    // divide `mant` by `10^k`. now `scale / 10 < mant <= scale * 10`.
    if k >= 0 {
        scale = mul_pow10(scale, k as usize);
    } else {
        mant = mul_pow10(mant, -k as usize);
    }

    // fixup when `mant + plus >= scale`, where `plus / scale = 10^-buf.len() / 2`.
    // in order to keep the fixed-size bignum, we actually use `mant + floor(plus) >= scale`.
    // we are not actually modifying `scale`, since we can skip the initial multiplication instead.
    // again with the shortest algorithm, `d[0]` can be zero but will be eventually rounded up.
    if div_2pow10(scale.clone(), buf.len()).add(&mant) >= scale {
        // equivalent to scaling `scale` by 10
        k += 1;
    } else {
        mant = mant.mul_small(10);
    }

    // if we are working with the last-digit limitation, we need to shorten the buffer
    // before the actual rendering in order to avoid double rounding.
    // note that we have to enlarge the buffer again when rounding up happens!
    let mut len = if k < limit {
        // oops, we cannot even produce *one* digit.
        // this is possible when, say, we've got something like 9.5 and it's being rounded to 10.
        // we return an empty buffer, with an exception of the later rounding-up case
        // which occurs when `k == limit` and has to produce exactly one digit.
        0
    } else if ((k as i32 - limit as i32) as usize) < buf.len() {
        (k - limit) as usize
    } else {
        buf.len()
    };

    if len > 0 {
        // cache `(2, 4, 8) * scale` for digit generation.
        // (this can be expensive, so do not calculate them when the buffer is empty.)
        let scale2 = scale.mul_pow2(1);
        let scale4 = scale.mul_pow2(2);
        let scale8 = scale.mul_pow2(3);

        for i in 0..len {
            if mant.is_zero() { // following digits are all zeroes, we stop here
                // do *not* try to perform rounding! rather, fill remaining digits.
                for c in &mut buf[i..len] { *c = b'0'; }
                return (len, k);
            }

            let mut d = 0;
            if mant >= scale8 { mant = mant.sub(&scale8); d += 8; }
            if mant >= scale4 { mant = mant.sub(&scale4); d += 4; }
            if mant >= scale2 { mant = mant.sub(&scale2); d += 2; }
            if mant >= scale  { mant = mant.sub(&scale);  d += 1; }
            debug_assert!(mant < scale);
            debug_assert!(d < 10);
            buf[i] = b'0' + d;
            mant = mant.mul_small(10);
        }
    }

    // rounding up if we stop in the middle of digits
    if mant >= scale.mul_small(5) {
        // if rounding up changes the length, the exponent should also change.
        // but we've been requested a fixed number of digits, so do not alter the buffer...
        if let Some(c) = round_up(buf, len) {
            // ...unless we've been requested the fixed precision instead.
            // we also need to check that, if the original buffer was empty,
            // the additional digit can only be added when `k == limit` (edge case).
            k += 1;
            if k > limit && len < buf.len() {
                buf[len] = c;
                len += 1;
            }
        }
    }

    (len, k)
}

#[cfg(test)] #[test]
fn shortest_sanity_test() {
    testing::f64_shortest_sanity_test(format_shortest);
    testing::f32_shortest_sanity_test(format_shortest);
    testing::more_shortest_sanity_test(format_shortest);
}

#[cfg(test)] #[test]
fn exact_sanity_test() {
    testing::f64_exact_sanity_test(format_exact);
    testing::f32_exact_sanity_test(format_exact);
}

#[cfg(test)] #[bench]
fn bench_small_shortest(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(3.141592f64);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_big_shortest(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(f64::MAX);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_small_exact_3(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(3.141592f64);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[bench]
fn bench_big_exact_3(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(f64::MAX);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[bench]
fn bench_small_exact_12(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(3.141592f64);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[bench]
fn bench_big_exact_12(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(f64::MAX);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[bench]
fn bench_small_exact_inf(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(3.141592f64);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[bench]
fn bench_big_exact_inf(b: &mut testing::Bencher) {
    let decoded = testing::decode_finite(f64::MAX);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[cfg(test)] #[test]
fn test_to_shortest_str() {
    testing::to_shortest_str_test(format_shortest);
}

#[cfg(test)] #[test]
fn test_to_shortest_exp_str() {
    testing::to_shortest_exp_str_test(format_shortest);
}

#[cfg(test)] #[test]
fn test_to_exact_exp_str() {
    testing::to_exact_exp_str_test(format_exact);
}

#[cfg(test)] #[test]
fn test_to_exact_fixed_str() {
    testing::to_exact_fixed_str_test(format_exact);
}


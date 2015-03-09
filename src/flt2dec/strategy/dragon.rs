/*
almost direct (but slightly optimized) Rust translation of Figure 3 of [1].

[1] Burger, R. G. and Dybvig, R. K. 1996. Printing floating-point numbers
    quickly and accurately. SIGPLAN Not. 31, 5 (May. 1996), 108-116.
*/

use std::num::{Int, Float};
use std::cmp::Ordering::{Greater, Equal};
#[cfg(test)] use test;

use flt2dec::{Decoded, MAX_SIG_DIGITS, round_up};
use flt2dec::bignum::Digit32 as Digit;
use flt2dec::bignum::Big32x36 as Big;
#[cfg(test)] use flt2dec::testing;

// approximate k_0 = ceil(log_10 (mant * 2^exp))
fn estimate_scaling_factor(mant: u64, exp: i16) -> i16 {
    // 2^(nbits-1) < mant <= 2^nbits if mant > 0
    let nbits = 64 - (mant - 1).leading_zeros() as i64;
    (((nbits + exp as i64) * 1292913986) >> 32) as i16
}

#[cfg(test)] #[test]
fn test_estimate_scaling_factor() {
    macro_rules! assert_almost_eq {
        ($actual:expr, $expected:expr) => ({
            let actual = $actual;
            let expected = $expected;
            println!("{} - {} = {} - {} = {}", stringify!($expected), stringify!($actual),
                     expected, actual, expected - actual);
            assert!(expected == actual || expected == actual + 1,
                    "expected {}, actual {}", expected, actual);
        })
    }

    assert_almost_eq!(estimate_scaling_factor(1, 0), 0);
    assert_almost_eq!(estimate_scaling_factor(2, 0), 1);
    assert_almost_eq!(estimate_scaling_factor(10, 0), 1);
    assert_almost_eq!(estimate_scaling_factor(11, 0), 2);
    assert_almost_eq!(estimate_scaling_factor(100, 0), 2);
    assert_almost_eq!(estimate_scaling_factor(101, 0), 3);
    assert_almost_eq!(estimate_scaling_factor(10000000000000000000, 0), 19);
    assert_almost_eq!(estimate_scaling_factor(10000000000000000001, 0), 20);

    // 1/2^20 = 0.00000095367...
    assert_almost_eq!(estimate_scaling_factor(1 * 1048576 / 1000000, -20), -6);
    assert_almost_eq!(estimate_scaling_factor(1 * 1048576 / 1000000 + 1, -20), -5);
    assert_almost_eq!(estimate_scaling_factor(10 * 1048576 / 1000000, -20), -5);
    assert_almost_eq!(estimate_scaling_factor(10 * 1048576 / 1000000 + 1, -20), -4);
    assert_almost_eq!(estimate_scaling_factor(100 * 1048576 / 1000000, -20), -4);
    assert_almost_eq!(estimate_scaling_factor(100 * 1048576 / 1000000 + 1, -20), -3);
    assert_almost_eq!(estimate_scaling_factor(1048575, -20), 0);
    assert_almost_eq!(estimate_scaling_factor(1048576, -20), 0);
    assert_almost_eq!(estimate_scaling_factor(1048577, -20), 1);
    assert_almost_eq!(estimate_scaling_factor(10485759999999999999, -20), 13);
    assert_almost_eq!(estimate_scaling_factor(10485760000000000000, -20), 13);
    assert_almost_eq!(estimate_scaling_factor(10485760000000000001, -20), 14);

    // extreme values:
    // 2^-1074 = 4.94065... * 10^-324
    // (2^53-1) * 2^971 = 1.79763... * 10^308
    assert_almost_eq!(estimate_scaling_factor(1, -1074), -323);
    assert_almost_eq!(estimate_scaling_factor(0x1fffffffffffff, 971), 309);

    for i in range(-1074, 972) {
        // XXX powi does not work for i < -1023
        let expected = 2.0f64.powf(i as f64).log10().ceil();
        assert_almost_eq!(estimate_scaling_factor(1, i), expected as i16);
    }
}

// XXX const ref to static array seems to ICE (#22540)
static POW10: [Digit; 10] = [1, 10, 100, 1000, 10000, 100000,
                             1000000, 10000000, 100000000, 1000000000];
static TWOPOW10: [Digit; 10] = [2, 20, 200, 2000, 20000, 200000,
                                2000000, 20000000, 200000000, 2000000000];

fn mul_pow10(mut x: Big, mut n: usize) -> Big {
    let largest = POW10.len() - 1;
    while n > largest {
        x = x.mul_small(POW10[largest]);
        n -= largest;
    }
    if n > 0 {
        x = x.mul_small(POW10[n]);
    }
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
    for i in range(1, 20) {
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
    let rounding = if d.inclusive {Greater} else {Equal};

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
        // if rounding up changes the length, the exponent should also change
        if round_up(buf, i) {
            buf[i] = b'0';
            i += 1;
            k += 1;
        }
    }

    (i, k)
}

pub fn format_exact(d: &Decoded, buf: &mut [u8]) -> (/*#digits*/ usize, /*exp*/ i16) {
    // the stripped-down version of Dragon for fixed-size output.

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

    // cache `(2, 4, 8) * scale` for digit generation.
    let scale2 = scale.mul_pow2(1);
    let scale4 = scale.mul_pow2(2);
    let scale8 = scale.mul_pow2(3);

    let len = buf.len();
    for i in range(0, len) {
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

    // rounding up if we stop in the middle of digits
    if mant >= scale.mul_small(5) {
        // if rounding up changes the length, the exponent should also change
        // (but we've been requested a fixed number of digits, so do not alter the buffer)
        if round_up(buf, len) {
            k += 1;
        }
    }

    (buf.len(), k)
}

#[cfg(test)] #[test]
fn shortest_sanity_test() {
    testing::f64_shortest_sanity_test(format_shortest);
    testing::f32_shortest_sanity_test(format_shortest);
}

#[cfg(test)] #[test]
fn exact_sanity_test() {
    testing::f64_exact_sanity_test(format_exact);
    testing::f32_exact_sanity_test(format_exact);
}

#[cfg(test)] #[bench]
fn bench_small_shortest(b: &mut test::Bencher) {
    use flt2dec::decode;
    let decoded = decode(3.141592f64);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_big_shortest(b: &mut test::Bencher) {
    use flt2dec::decode;
    let v: f64 = Float::max_value();
    let decoded = decode(v);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_small_exact_3(b: &mut test::Bencher) {
    use flt2dec::decode;
    let decoded = decode(3.141592f64);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_big_exact_3(b: &mut test::Bencher) {
    use flt2dec::decode;
    let v: f64 = Float::max_value();
    let decoded = decode(v);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_small_exact_12(b: &mut test::Bencher) {
    use flt2dec::decode;
    let decoded = decode(3.141592f64);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_big_exact_12(b: &mut test::Bencher) {
    use flt2dec::decode;
    let v: f64 = Float::max_value();
    let decoded = decode(v);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_small_exact_inf(b: &mut test::Bencher) {
    use flt2dec::decode;
    let decoded = decode(3.141592f64);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf));
}

#[cfg(test)] #[bench]
fn bench_big_exact_inf(b: &mut test::Bencher) {
    use flt2dec::decode;
    let v: f64 = Float::max_value();
    let decoded = decode(v);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf));
}


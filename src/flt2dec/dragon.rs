use std::char;
use std::num::{mod, Float};
#[cfg(test)] use test;
use super::intrin;

/*
almost direct (but slightly optimized) Rust translation of Figure 3 of [1].

[1] Burger, R. G. and Dybvig, R. K. 1996. Printing floating-point numbers
    quickly and accurately. SIGPLAN Not. 31, 5 (May. 1996), 108-116.
*/

// enough for f64
pub type Digit = u32;
define_bignum!(Big: [Digit, ..36]);
static POW10: &'static [Digit] = &[1, 10, 100, 1000, 10000, 100000,
                                   1000000, 10000000, 100000000, 1000000000];

// approximate k_0 = ceil(log_10 (mant * 2^exp))
fn estimate_scaling_factor(mant: u64, exp: i16) -> i16 {
    // 2^(nbits-1) < mant <= 2^nbits if mant > 0
    let nbits = 64 - intrin::ctlz64(mant - 1) as i64;
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

fn mul_pow10(mut x: Big, mut n: uint) -> Big {
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

fn format_decoded(mant: u64, mant_was_odd: bool, minus: Digit, plus: Digit,
                  exp: i16, sign: i8) -> String {
    // `a.cmp(&b) < rounding` is `if mant_was_odd {a < b} else {a <= b}`
    let rounding = if mant_was_odd {Equal} else {Greater};

    // estimate the scaling factor from original inputs
    let mut k = estimate_scaling_factor(mant + plus as u64, exp);

    let mut mant = Big::from_u64(mant);     // `r` from [1]
    let mut minus = Big::from_small(minus); // `m^+` from [1]
    let mut plus = Big::from_small(plus);   // `m^-` from [1]
    let mut scale = Big::from_small(1);     // `s` from [1]

    if exp < 0 {
        scale = scale.mul_pow2(-exp as uint);
    } else {
        mant = mant.mul_pow2(exp as uint);
        minus = minus.mul_pow2(exp as uint);
        plus = plus.mul_pow2(exp as uint);
    }

    // v = mant / s
    // low = (mant - minus) / s
    // high = (mant + plus) / s

    if k >= 0 {
        scale = mul_pow10(scale, k as uint);
    } else {
        mant = mul_pow10(mant, -k as uint);
        minus = mul_pow10(minus, -k as uint);
        plus = mul_pow10(plus, -k as uint);
    }

    // fixup when `mant + plus > scale` (or `>=`)
    let mut skip = false;
    if scale.cmp(&mant.clone().add(&plus)) < rounding {
        k += 1;
        skip = true; // equivalent to scaling `scale` by 10
    }

    let scale2 = scale.mul_pow2(1);
    let scale4 = scale.mul_pow2(2);
    let scale8 = scale.mul_pow2(3);

    let mut ret = if sign > 0 {"0.".into_string()} else {"-0.".into_string()};
    loop {
        if skip {
            skip = false;
        } else {
            mant = mant.mul_small(10);
            minus = minus.mul_small(10);
            plus = plus.mul_small(10);
        }

        let mut d = 0u;
        if mant >= scale8 { mant = mant.sub(&scale8); d += 8; }
        if mant >= scale4 { mant = mant.sub(&scale4); d += 4; }
        if mant >= scale2 { mant = mant.sub(&scale2); d += 2; }
        if mant >= scale  { mant = mant.sub(&scale);  d += 1; }
        debug_assert!(mant < scale);
        ret.push(char::from_digit(d, 10).unwrap());

        // the end condition
        // note: the original paper got the second inequality incorrect :)
        let down = mant.cmp(&minus) < rounding;
        let up = scale.cmp(&mant.clone().add(&plus)) < rounding;
        match (down, up) {
            (true, false) => { break; }
            (false, true) => { ret.push_str("+1"); break; }
            (true, true) => {
                // tie breaking
                if mant.clone().mul_pow2(1) >= scale {
                    ret.push_str("+1");
                }
                break;
            }
            (false, false) => {} // continue generating digits
        }
    }

    ret.push_str(format!("e{:+}", k)[]);
    ret
}

// Float::integer_decode always preserves the exponent, so the mantissa is scaled for subnormals
fn decode<T: Float>(v: T) -> (u64, /*lsb of mant*/ bool,
                              /*lower ulp*/ Digit, /*upper ulp*/ Digit, /*exp*/ i16, /*sign*/ i8) {
    let zero: T = Float::zero();
    let minnorm: T = Float::min_pos_value(None);

    let (mant, exp, sign) = v.integer_decode();
    let (_, zeroexp, _) = zero.integer_decode();
    let (minnormmant, _, _) = minnorm.integer_decode();

    if exp == zeroexp { // subnormal
        // (mant - 2, exp) -- (mant, exp) -- (mant + 2, exp)
        (mant, ((mant >> 1) & 1) != 0, 1, 1, exp, sign)
    } else if mant == minnormmant {
        // (maxmant, exp - 1) -- (minnormmant, exp) -- (minnormmant + 1, exp)
        // where maxmant = minnormmant * 2 - 1
        (mant << 1, (mant & 1) != 0, 1, 2, exp - 1, sign)
    } else {
        // (mant - 1, exp) -- (mant, exp) -- (mant + 1, exp)
        (mant << 1, (mant & 1) != 0, 1, 1, exp - 1, sign)
    }
}

pub fn format<T: Float>(v: T) -> String {
    match v.classify() {
        num::FPNaN => "nan".into_string(),
        num::FPInfinite if v.is_positive() => "inf".into_string(),
        num::FPInfinite => "-inf".into_string(),
        num::FPZero => "0".into_string(),
        num::FPNormal | num::FPSubnormal => {
            let (mant, mant_was_odd, minus, plus, exp, sign) = decode(v);
            format_decoded(mant, mant_was_odd, minus, plus, exp, sign)
        }
    }
}

#[cfg(test)] #[test]
fn wtf(){
    macro_rules! f { ($e:expr) => ({
        let small: f32 = $e;
        let big: f64 = $e;
        println!("{} as f32 = {} / {}", stringify!($e), small, format(small));
        println!("{} as f64 = {} / {}", stringify!($e), big, format(big));
    }) }
    println!("");
    f!(0.1);
    f!(42.0);
    f!(3.141592);
    f!(3.141592e17);
    f!(1.0e23);
    f!(Float::max_value());
    f!(Float::min_pos_value(None));
    f!(2.0.powf(-149.0));
    f!(2.0.powf(-1074.0));
    assert!(false);
}

#[cfg(test)] #[bench]
fn bench_small(b: &mut test::Bencher) {
    b.iter(|| format(3.141592f64));
}

#[cfg(test)] #[bench]
fn bench_small_system(b: &mut test::Bencher) {
    b.iter(|| 3.141592f64.to_string());
}

#[cfg(test)] #[bench]
fn bench_big(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| format(v));
}

#[cfg(test)] #[bench]
fn bench_big_system(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| v.to_string());
}


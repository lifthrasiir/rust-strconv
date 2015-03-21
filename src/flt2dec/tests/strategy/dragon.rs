use std::{i16, f64};
use super::super::*;
use flt2dec::*;
use flt2dec::bignum::Big32x36 as Big;
use flt2dec::strategy::dragon::*;

#[test]
fn test_mul_pow10() {
    let mut prevpow10 = Big::from_small(1);
    for i in 1..340 {
        let curpow10 = mul_pow10(Big::from_small(1), i);
        assert_eq!(curpow10, prevpow10.mul_small(10));
        prevpow10 = curpow10;
    }
}

#[test]
fn shortest_sanity_test() {
    f64_shortest_sanity_test(format_shortest);
    f32_shortest_sanity_test(format_shortest);
    more_shortest_sanity_test(format_shortest);
}

#[test]
fn exact_sanity_test() {
    f64_exact_sanity_test(format_exact);
    f32_exact_sanity_test(format_exact);
}

#[bench]
fn bench_small_shortest(b: &mut Bencher) {
    let decoded = decode_finite(3.141592f64);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[bench]
fn bench_big_shortest(b: &mut Bencher) {
    let decoded = decode_finite(f64::MAX);
    let mut buf = [0; MAX_SIG_DIGITS];
    b.iter(|| format_shortest(&decoded, &mut buf));
}

#[bench]
fn bench_small_exact_3(b: &mut Bencher) {
    let decoded = decode_finite(3.141592f64);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[bench]
fn bench_big_exact_3(b: &mut Bencher) {
    let decoded = decode_finite(f64::MAX);
    let mut buf = [0; 3];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[bench]
fn bench_small_exact_12(b: &mut Bencher) {
    let decoded = decode_finite(3.141592f64);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[bench]
fn bench_big_exact_12(b: &mut Bencher) {
    let decoded = decode_finite(f64::MAX);
    let mut buf = [0; 12];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[bench]
fn bench_small_exact_inf(b: &mut Bencher) {
    let decoded = decode_finite(3.141592f64);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[bench]
fn bench_big_exact_inf(b: &mut Bencher) {
    let decoded = decode_finite(f64::MAX);
    let mut buf = [0; 1024];
    b.iter(|| format_exact(&decoded, &mut buf, i16::MIN));
}

#[test]
fn test_to_shortest_str() {
    to_shortest_str_test(format_shortest);
}

#[test]
fn test_to_shortest_exp_str() {
    to_shortest_exp_str_test(format_shortest);
}

#[test]
fn test_to_exact_exp_str() {
    to_exact_exp_str_test(format_exact);
}

#[test]
fn test_to_exact_fixed_str() {
    to_exact_fixed_str_test(format_exact);
}


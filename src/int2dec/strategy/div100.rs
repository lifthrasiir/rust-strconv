use std::num::div_rem;

use int2dec::digits::{Digits64, Digits32, Digits16, Digits8};
use int2dec::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};
use int2dec::digits::{ONES, TENS};
#[cfg(test)] use int2dec::test;

pub fn u64_to_digits(n: u64) -> Digits64 {
    let mut buf: Digits64 = [0, ..NDIGITS64];
    let (n, r) = div_rem(n, 100); buf[18] = TENS[r as uint]; buf[19] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[16] = TENS[r as uint]; buf[17] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[14] = TENS[r as uint]; buf[15] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[12] = TENS[r as uint]; buf[13] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[10] = TENS[r as uint]; buf[11] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 8] = TENS[r as uint]; buf[ 9] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 6] = TENS[r as uint]; buf[ 7] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 4] = TENS[r as uint]; buf[ 5] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 2] = TENS[r as uint]; buf[ 3] = ONES[r as uint];
    let r = n;                    buf[ 0] = TENS[r as uint]; buf[ 1] = ONES[r as uint];
    buf
}

pub fn u32_to_digits(n: u32) -> Digits32 {
    let mut buf: Digits32 = [0, ..NDIGITS32];
    let (n, r) = div_rem(n, 100); buf[ 8] = TENS[r as uint]; buf[ 9] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 6] = TENS[r as uint]; buf[ 7] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 4] = TENS[r as uint]; buf[ 5] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 2] = TENS[r as uint]; buf[ 3] = ONES[r as uint];
    let r = n;                    buf[ 0] = TENS[r as uint]; buf[ 1] = ONES[r as uint];
    buf
}

pub fn u16_to_digits(n: u16) -> Digits16 {
    let mut buf: Digits16 = [0, ..NDIGITS16];
    let (n, r) = div_rem(n, 100); buf[ 3] = TENS[r as uint]; buf[ 4] = ONES[r as uint];
    let (n, r) = div_rem(n, 100); buf[ 1] = TENS[r as uint]; buf[ 2] = ONES[r as uint];
    let r = n;                                               buf[ 0] = r as u8 + b'0';
    buf
}

pub fn u8_to_digits(n: u8) -> Digits8 {
    let mut buf: Digits8 = [0, ..NDIGITS8];
    let (n, r) = div_rem(n, 100); buf[ 1] = TENS[r as uint]; buf[ 2] = ONES[r as uint];
    let r = n;                                               buf[ 0] = r as u8 + b'0';
    buf
}

#[cfg(test)] #[test]
fn sanity_test() {
    test::u64_sanity_test(u64_to_digits);
    test::u32_sanity_test(u32_to_digits);
    test::u16_sanity_test(u16_to_digits);
    test::u8_sanity_test(u8_to_digits);
}

#[cfg(test)] #[bench]
fn bench_u64(b: &mut test::Bencher) {
    test::rotating_bench(u64_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u32(b: &mut test::Bencher) {
    test::rotating_bench(u32_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u16(b: &mut test::Bencher) {
    test::rotating_bench(u16_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u8(b: &mut test::Bencher) {
    test::rotating_bench(u8_to_digits, b);
}


use num::div_rem;

use int2dec::digits::{Digits64, Digits16, Digits8};
use int2dec::digits::{NDIGITS64, NDIGITS16, NDIGITS8};
use int2dec::digits::{ONES, TENS};
#[cfg(test)] use int2dec::testing;

pub fn u64_to_digits(n: u64) -> Digits64 {
    let mut buf: Digits64 = [0; NDIGITS64];

    let (xy, z) = div_rem(n, 10000);

    let n = z as u32;
    let (n, r) = div_rem(n, 100); buf[18] = tens!(r); buf[19] = ones!(r);
    let r = n;                    buf[16] = tens!(r); buf[17] = ones!(r);

    let (x, y) = div_rem(xy, 100000000);

    let n = y as u32;
    let (n, r) = div_rem(n, 100); buf[14] = tens!(r); buf[15] = ones!(r);
    let (n, r) = div_rem(n, 100); buf[12] = tens!(r); buf[13] = ones!(r);
    let (n, r) = div_rem(n, 100); buf[10] = tens!(r); buf[11] = ones!(r);
    let r = n;                    buf[ 8] = tens!(r); buf[ 9] = ones!(r);

    let n = x as u32;
    let (n, r) = div_rem(n, 100); buf[ 6] = tens!(r); buf[ 7] = ones!(r);
    let (n, r) = div_rem(n, 100); buf[ 4] = tens!(r); buf[ 5] = ones!(r);
    let (n, r) = div_rem(n, 100); buf[ 2] = tens!(r); buf[ 3] = ones!(r);
    let r = n;                    buf[ 0] = tens!(r); buf[ 1] = ones!(r);

    buf
}

pub fn u16_to_digits(n: u16) -> Digits16 {
    let mut buf: Digits16 = [0; NDIGITS16];
    let n = n as u32;
    let (n, r) = div_rem(n, 100); buf[ 3] = tens!(r); buf[ 4] = ones!(r);
    let (n, r) = div_rem(n, 100); buf[ 1] = tens!(r); buf[ 2] = ones!(r);
    let r = n;                    buf[ 0] = r as u8 + b'0';
    buf
}

pub fn u8_to_digits(n: u8) -> Digits8 {
    let mut buf: Digits8 = [0; NDIGITS8];
    let n = n as u32;
    let (n, r) = div_rem(n, 100); buf[ 1] = tens!(r); buf[ 2] = ones!(r);
    let r = n;                    buf[ 0] = r as u8 + b'0';
    buf
}

#[cfg(test)] #[test]
fn sanity_test() {
    testing::u64_sanity_test(u64_to_digits);
    testing::u16_sanity_test(u16_to_digits);
    testing::u8_sanity_test(u8_to_digits);
}

#[cfg(test)] #[bench]
fn bench_u64(b: &mut testing::Bencher) {
    testing::rotating_bench(u64_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u16(b: &mut testing::Bencher) {
    testing::rotating_bench(u16_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u8(b: &mut testing::Bencher) {
    testing::rotating_bench(u8_to_digits, b);
}


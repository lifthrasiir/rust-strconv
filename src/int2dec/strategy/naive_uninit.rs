use std::mem;
use std::num::div_rem;

use int2dec::digits::{Digits64, Digits32, Digits16, Digits8};
use int2dec::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};
#[cfg(test)] use int2dec::test;

pub fn u64_to_digits(mut n: u64) -> Digits64 {
    let mut buf: Digits64 = unsafe {mem::uninitialized()};
    for i in range(0, NDIGITS64).rev() {
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u32_to_digits(mut n: u32) -> Digits32 {
    let mut buf: Digits32 = unsafe {mem::uninitialized()};
    for i in range(0, NDIGITS32).rev() {
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u16_to_digits(mut n: u16) -> Digits16 {
    let mut buf: Digits16 = unsafe {mem::uninitialized()};
    for i in range(0, NDIGITS16).rev() {
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u8_to_digits(mut n: u8) -> Digits8 {
    let mut buf: Digits8 = unsafe {mem::uninitialized()};
    for i in range(0, NDIGITS8).rev() {
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
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


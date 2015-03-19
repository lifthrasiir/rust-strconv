use std::num::{Int, NumCast};
use test;

use super::digits::{Digits64, Digits32, Digits16, Digits8};

pub use test::Bencher;

pub fn u64_sanity_test<F: FnMut(u64) -> Digits64>(mut f: F) {
    assert_eq!(f(                   0), b"00000000000000000000");
    assert_eq!(f(                   1), b"00000000000000000001");
    assert_eq!(f(                  12), b"00000000000000000012");
    assert_eq!(f(                 123), b"00000000000000000123");
    assert_eq!(f(             1234567), b"00000000000001234567");
    assert_eq!(f(     123456789012345), b"00000123456789012345");
    assert_eq!(f(18446744073709551615), b"18446744073709551615");
}

pub fn u32_sanity_test<F: FnMut(u32) -> Digits32>(mut f: F) {
    assert_eq!(f(         0), b"0000000000");
    assert_eq!(f(         1), b"0000000001");
    assert_eq!(f(        12), b"0000000012");
    assert_eq!(f(       123), b"0000000123");
    assert_eq!(f(   1234567), b"0001234567");
    assert_eq!(f(4294967295), b"4294967295");
}

pub fn u16_sanity_test<F: FnMut(u16) -> Digits16>(mut f: F) {
    assert_eq!(f(    0), b"00000");
    assert_eq!(f(    1), b"00001");
    assert_eq!(f(   12), b"00012");
    assert_eq!(f(  123), b"00123");
    assert_eq!(f(65535), b"65535");
}

pub fn u8_sanity_test<F: FnMut(u8) -> Digits8>(mut f: F) {
    assert_eq!(f(  0), b"000");
    assert_eq!(f(  1), b"001");
    assert_eq!(f( 12), b"012");
    assert_eq!(f(123), b"123");
    assert_eq!(f(255), b"255");
}

#[inline(always)]
pub fn rotating_bench<I: Int, T, F: FnMut(I) -> T>(mut f: F, b: &mut Bencher) {
    b.iter(|| {
        // small integers (4, 5, 6, ..., 3424806)
        let mut n = NumCast::from(4).unwrap();
        for _ in 0..64 {
            test::black_box(f(n));
            n = n.wrapping_add(n >> 2);
        }

        // large integers
        let mut n = NumCast::from(1).unwrap();
        for _ in 0..64 {
            test::black_box(f(n));
            n = n.wrapping_mul(NumCast::from(3).unwrap());
        }
    });
}


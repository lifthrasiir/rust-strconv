use std::prelude::v1::*;
use std::num::{Int, NumCast};
use test;

use int2dec::digits::{Digits64, Digits32, Digits16, Digits8};
use int2dec::UintToDec;

pub use test::Bencher;

pub fn u64_sanity_test<F: FnMut(u64) -> Digits64>(mut f: F) {
    assert_eq!(&f(                   0), b"00000000000000000000");
    assert_eq!(&f(                   1), b"00000000000000000001");
    assert_eq!(&f(                  12), b"00000000000000000012");
    assert_eq!(&f(                 123), b"00000000000000000123");
    assert_eq!(&f(             1234567), b"00000000000001234567");
    assert_eq!(&f(     123456789012345), b"00000123456789012345");
    assert_eq!(&f(18446744073709551615), b"18446744073709551615");
}

pub fn u32_sanity_test<F: FnMut(u32) -> Digits32>(mut f: F) {
    assert_eq!(&f(         0), b"0000000000");
    assert_eq!(&f(         1), b"0000000001");
    assert_eq!(&f(        12), b"0000000012");
    assert_eq!(&f(       123), b"0000000123");
    assert_eq!(&f(   1234567), b"0001234567");
    assert_eq!(&f(4294967295), b"4294967295");
}

pub fn u16_sanity_test<F: FnMut(u16) -> Digits16>(mut f: F) {
    assert_eq!(&f(    0), b"00000");
    assert_eq!(&f(    1), b"00001");
    assert_eq!(&f(   12), b"00012");
    assert_eq!(&f(  123), b"00123");
    assert_eq!(&f(65535), b"65535");
}

pub fn u8_sanity_test<F: FnMut(u8) -> Digits8>(mut f: F) {
    assert_eq!(&f(  0), b"000");
    assert_eq!(&f(  1), b"001");
    assert_eq!(&f( 12), b"012");
    assert_eq!(&f(123), b"123");
    assert_eq!(&f(255), b"255");
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

// per-strategy tests
macro_rules! per_strategy {
    ($strategy:ident: $($t:tt)*) => (
        mod $strategy {
            use super::super::*;
            use int2dec::strategy::$strategy::*;

            #[test] fn sanity_test() { per_strategy_sanity_test!($($t)*) }
            per_strategy_bench!($($t)*);
        }
    )
}

macro_rules! per_strategy_sanity_test {
    () => ({});
    (u8 $($t:tt)*) => ({ u8_sanity_test(u8_to_digits); per_strategy_sanity_test!($($t)*) });
    (u16 $($t:tt)*) => ({ u16_sanity_test(u16_to_digits); per_strategy_sanity_test!($($t)*) });
    (u32 $($t:tt)*) => ({ u32_sanity_test(u32_to_digits); per_strategy_sanity_test!($($t)*) });
    (u64 $($t:tt)*) => ({ u64_sanity_test(u64_to_digits); per_strategy_sanity_test!($($t)*) });
}

macro_rules! per_strategy_bench {
    () => ();
    (u8 $($t:tt)*) => (
        #[bench] fn bench_u8(b: &mut Bencher) { rotating_bench(u8_to_digits, b) }
        per_strategy_bench!($($t)*);
    );
    (u16 $($t:tt)*) => (
        #[bench] fn bench_u16(b: &mut Bencher) { rotating_bench(u16_to_digits, b) }
        per_strategy_bench!($($t)*);
    );
    (u32 $($t:tt)*) => (
        #[bench] fn bench_u32(b: &mut Bencher) { rotating_bench(u32_to_digits, b) }
        per_strategy_bench!($($t)*);
    );
    (u64 $($t:tt)*) => (
        #[bench] fn bench_u64(b: &mut Bencher) { rotating_bench(u64_to_digits, b) }
        per_strategy_bench!($($t)*);
    );
}

mod strategy {
    per_strategy!(bcd:                  u64 u32       );
    per_strategy!(bcd_earlyexit:        u64 u32       );
    per_strategy!(div100:               u64 u32 u16 u8);
    per_strategy!(div100_earlyexit:     u64 u32 u16 u8);
    per_strategy!(div100_u32:           u64     u16 u8);
    per_strategy!(div100_u32_earlyexit: u64     u16 u8);
    per_strategy!(naive:                u64 u32 u16 u8);
    per_strategy!(naive_earlyexit:      u64 u32 u16 u8);

    mod best {
        use super::super::*;
        use int2dec::strategy::best::*;
        per_strategy_bench!(u64 u32 u16 u8);
    }
}

#[test]
fn uint_to_dec_sanity_test() {
    let mut n = 1u64;
    for _ in 0..64 {
        assert_eq!((n as u64).to_string(), UintToDec(n as u64).to_string());
        assert_eq!((n as u32).to_string(), UintToDec(n as u32).to_string());
        assert_eq!((n as u16).to_string(), UintToDec(n as u16).to_string());
        assert_eq!((n as u8).to_string(), UintToDec(n as u8).to_string());
        n = n.wrapping_mul(3);
    }
}

macro_rules! make_bench {
    ($t:ty: $system:ident vs $best:ident) => (
        #[bench]
        fn $system(b: &mut test::Bencher) {
            b.iter(|| {
                use std::io::{Cursor, Write};
                let mut n: $t = 1;
                let mut buf = [0; 4096];
                let mut w = Cursor::new(&mut buf[..]);
                for _ in 0..64 {
                    let _ = write!(&mut w, "{}", n);
                    n = n.wrapping_mul(3);
                }
            });
        }

        #[bench]
        fn $best(b: &mut test::Bencher) {
            b.iter(|| {
                use std::io::{Cursor, Write};
                let mut n: $t = 1;
                let mut buf = [0; 4096];
                let mut w = Cursor::new(&mut buf[..]);
                for _ in 0..64 {
                    let _ = write!(&mut w, "{}", UintToDec(n));
                    n = n.wrapping_mul(3);
                }
            });
        }
    )
}

make_bench!(u64: bench_u64_system vs bench_u64_best);
make_bench!(u32: bench_u32_system vs bench_u32_best);
make_bench!(u16: bench_u16_system vs bench_u16_best);
make_bench!(u8: bench_u8_system vs bench_u8_best);


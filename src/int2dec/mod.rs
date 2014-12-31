use std::fmt;
use std::num::Int;
#[cfg(test)] use test;

pub use self::digits::Digit;
pub use self::digits::{Digits64, Digits32, Digits16, Digits8};
pub use self::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};

mod digits;
#[cfg(test)] mod testing;

pub mod strategy {
    pub mod naive;
    pub mod naive_earlyexit;
    pub mod div100;
    pub mod div100_earlyexit;
    pub mod bcd;
    pub mod bcd_earlyexit;
}

pub mod best {
    pub use super::strategy::bcd_earlyexit::u64_to_digits;

    #[cfg(target_arch = "i686")] pub use super::strategy::div100::u32_to_digits;
    #[cfg(not(target_arch = "i686"))] pub use super::strategy::div100_earlyexit::u32_to_digits;

    pub use super::strategy::div100::u16_to_digits;

    #[cfg(target_arch = "i686")] pub use super::strategy::div100_earlyexit::u8_to_digits;
    #[cfg(not(target_arch = "i686"))] pub use super::strategy::div100::u8_to_digits;
}

pub struct UintToDecFunc<I, T>(pub I, pub fn(I) -> T);
pub struct UintToDec<I>(pub I);

macro_rules! impl_uint_to_dec {
    ($t:ty, $Digits:ty, $default_conv:ident) => (
        impl<I: Int> fmt::Show for UintToDecFunc<I, $Digits> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let UintToDecFunc(num, conv) = *self;
                let buf = conv(num);
                let last = buf.len() - 1;
                let start = buf[..last].iter().position(|&c| c != b'0').unwrap_or(last);
                f.pad_integral(true, "", buf[start..])
            }
        }

        impl fmt::Show for UintToDec<$t> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let UintToDec(num) = *self;
                UintToDecFunc(num, best::$default_conv).fmt(f)
            }
        }
    )
}

impl_uint_to_dec!(u64, Digits64, u64_to_digits);
impl_uint_to_dec!(u32, Digits32, u32_to_digits);
impl_uint_to_dec!(u16, Digits16, u16_to_digits);
impl_uint_to_dec!(u8, Digits8, u8_to_digits);

#[cfg(test)] #[test]
fn sanity_test() {
    let mut n = 1u64;
    for _ in range(0, 64u) {
        assert_eq!((n as u64).to_string(), UintToDec(n as u64).to_string());
        assert_eq!((n as u32).to_string(), UintToDec(n as u32).to_string());
        assert_eq!((n as u16).to_string(), UintToDec(n as u16).to_string());
        assert_eq!((n as u8).to_string(), UintToDec(n as u8).to_string());
        n *= 3;
    }
}

macro_rules! make_bench {
    ($t:ty: $system:ident vs $best:ident) => (
        #[cfg(test)] #[bench]
        fn $system(b: &mut test::Bencher) {
            b.iter(|| {
                use std::io;
                let mut n: $t = 1;
                let mut buf = [0; 4096];
                let mut w = io::BufWriter::new(&mut buf);
                for _ in range(0, 64u) {
                    let _ = write!(&mut w, "{}", n);
                    n *= 3;
                }
            });
        }

        #[cfg(test)] #[bench]
        fn $best(b: &mut test::Bencher) {
            b.iter(|| {
                use std::io;
                let mut n: $t = 1;
                let mut buf = [0; 4096];
                let mut w = io::BufWriter::new(&mut buf);
                for _ in range(0, 64u) {
                    let _ = write!(&mut w, "{}", UintToDec(n));
                    n *= 3;
                }
            });
        }
    )
}

make_bench!(u64: bench_u64_system vs bench_u64_best);
make_bench!(u32: bench_u32_system vs bench_u32_best);
make_bench!(u16: bench_u16_system vs bench_u16_best);
make_bench!(u8: bench_u8_system vs bench_u8_best);


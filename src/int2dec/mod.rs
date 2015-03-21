use core::prelude::*;
use core::{str, fmt};
use core::num::Int;

pub use self::digits::Digit;
pub use self::digits::{Digits64, Digits32, Digits16, Digits8};
pub use self::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};

pub use self::strategy::best;

#[macro_use] mod digits;

pub mod strategy {
    pub mod naive;
    pub mod naive_earlyexit;
    pub mod div100;
    pub mod div100_earlyexit;
    pub mod div100_u32;
    pub mod div100_u32_earlyexit;
    pub mod bcd;
    pub mod bcd_earlyexit;

    pub mod best {
        #[cfg(target_arch = "x86")] pub use super::bcd_earlyexit::u64_to_digits;
        #[cfg(target_arch = "x86")] pub use super::div100_earlyexit::u32_to_digits;
        #[cfg(target_arch = "x86")] pub use super::div100::u16_to_digits;
        #[cfg(target_arch = "x86")] pub use super::div100_earlyexit::u8_to_digits;

        #[cfg(not(target_arch = "x86"))] pub use super::div100_u32_earlyexit::u64_to_digits;
        #[cfg(not(target_arch = "x86"))] pub use super::div100::u32_to_digits;
        #[cfg(not(target_arch = "x86"))] pub use super::div100::u16_to_digits;
        #[cfg(not(target_arch = "x86"))] pub use super::naive::u8_to_digits;
    }
}

#[cfg(test)] mod tests;

pub struct UintToDecFunc<I, T>(pub I, pub fn(I) -> T);
#[derive(Debug)] pub struct UintToDec<I>(pub I);

macro_rules! impl_uint_to_dec {
    ($t:ty, $Digits:ty, $default_conv:ident) => (
        impl<I: Int> fmt::Display for UintToDecFunc<I, $Digits> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let UintToDecFunc(num, conv) = *self;
                let buf = conv(num);
                let last = buf.len() - 1;
                let start = buf[..last].iter().position(|&c| c != b'0').unwrap_or(last);
                f.pad_integral(true, "", unsafe {str::from_utf8_unchecked(&buf[start..])})
            }
        }

        impl fmt::Display for UintToDec<$t> {
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


pub use self::digits::Digit;
pub use self::digits::{Digits64, Digits32, Digits16, Digits8};
pub use self::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};

mod digits;
#[cfg(test)] mod test;

pub mod strategy {
    pub mod naive;
    pub mod div100;
    pub mod bcd;
}


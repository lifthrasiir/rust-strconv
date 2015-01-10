pub const NDIGITS64: usize = 20; // 1844 6744 0737 0955 1615
pub const NDIGITS32: usize = 10; // 42 9496 7295
pub const NDIGITS16: usize = 5; // 6 5535
pub const NDIGITS8: usize = 3; // 255

pub type Digit = u8;

pub type Digits64 = [Digit; NDIGITS64];
pub type Digits32 = [Digit; NDIGITS32];
pub type Digits16 = [Digit; NDIGITS16];
pub type Digits8 = [Digit; NDIGITS8];

pub static TENS: &'static [u8] = b"00000000001111111111222222222233333333334444444444\
                                   55555555556666666666777777777788888888889999999999";
pub static ONES: &'static [u8] = b"01234567890123456789012345678901234567890123456789\
                                   01234567890123456789012345678901234567890123456789";

macro_rules! tens { ($i:expr) => (TENS[$i as usize]) }
macro_rules! ones { ($i:expr) => (ONES[$i as usize]) }


use num::div_rem;

use int2dec::digits::{Digits64, Digits32};
use int2dec::digits::{NDIGITS64, NDIGITS32};
use int2dec::digits::{ONES, TENS};
#[cfg(test)] use int2dec::testing;

// http://homepage.cs.uiowa.edu/~jones/bcd/decimal.html#sixtyfour
pub fn u64_to_digits(n: u64) -> Digits64 {
    let mut buf: Digits64 = [b'0', ..NDIGITS64];

    let n0 = (n & 0xffff) as u32;
    let n1 = ((n >> 16) & 0xffff) as u32;
    let n2 = ((n >> 32) & 0xffff) as u32;
    let n3 = ((n >> 48) & 0xffff) as u32;

    macro_rules! quad {
        ($d:expr, $i:expr) => ({
            let (qq, rr) = div_rem($d, 100);
            buf[$i  ] = tens!(qq); buf[$i+1] = ones!(qq);
            buf[$i+2] = tens!(rr); buf[$i+3] = ones!(rr);
        })
    }

    if n <= 0 { return buf; }
    let (c0, d0) = div_rem(      656 * n3 + 7296 * n2 + 5536 * n1 + n0, 10000); quad!(d0, 16);
    if n <= 9999 { return buf; }
    let (c1, d1) = div_rem(c0 + 7671 * n3 + 9496 * n2 +    6 * n1,      10000); quad!(d1, 12);
    if n <= 9999_9999 { return buf; }
    let (c2, d2) = div_rem(c1 + 4749 * n3 +   42 * n2,                  10000); quad!(d2, 8);
    if n <= 9999_9999_9999 { return buf; }
    let (d4, d3) = div_rem(c2 +  281 * n3,                              10000); quad!(d3, 4);
    if n <= 9999_9999_9999_9999 { return buf; }
    quad!(d4, 0);

    buf
}

pub fn u32_to_digits(n: u32) -> Digits32 {
    let mut buf: Digits32 = [b'0', ..NDIGITS32];

    let n0 = (n & 0xffff) as u32;
    let n1 = ((n >> 16) & 0xffff) as u32;

    macro_rules! quad {
        ($d:expr, $i:expr) => ({
            let (qq, rr) = div_rem($d, 100);
            buf[$i  ] = tens!(qq); buf[$i+1] = ones!(qq);
            buf[$i+2] = tens!(rr); buf[$i+3] = ones!(rr);
        })
    }

    if n <= 0 { return buf; }
    let (c0, d0) = div_rem(     5536 * n1 + n0, 10000); quad!(d0, 6);
    if n <= 9999 { return buf; };
    let (d2, d1) = div_rem(c0 +    6 * n1,      10000); quad!(d1, 2);
    if n <= 9999_9999 { return buf; }
    buf[0] = tens!(d2); buf[1] = ones!(d2);

    buf
}

#[cfg(test)] #[test]
fn sanity_test() {
    testing::u64_sanity_test(u64_to_digits);
    testing::u32_sanity_test(u32_to_digits);
}

#[cfg(test)] #[bench]
fn bench_u64(b: &mut testing::Bencher) {
    testing::rotating_bench(u64_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u32(b: &mut testing::Bencher) {
    testing::rotating_bench(u32_to_digits, b);
}

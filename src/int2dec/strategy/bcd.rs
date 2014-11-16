use std::mem;
use std::num::div_rem;

use int2dec::digits::{Digits64, Digits32};
use int2dec::digits::{ONES, TENS};
#[cfg(test)] use int2dec::test;

// http://homepage.cs.uiowa.edu/~jones/bcd/decimal.html#sixtyfour
pub fn u64_to_digits(n: u64) -> Digits64 {
    let mut buf: Digits64 = unsafe {mem::uninitialized()};

    let n0 = (n & 0xffff) as u32;
    let n1 = ((n >> 16) & 0xffff) as u32;
    let n2 = ((n >> 32) & 0xffff) as u32;
    let n3 = ((n >> 48) & 0xffff) as u32;

    let (c0, d0) = div_rem(      656 * n3 + 7296 * n2 + 5536 * n1 + n0, 10000);
    let (c1, d1) = div_rem(c0 + 7671 * n3 + 9496 * n2 +    6 * n1,      10000);
    let (c2, d2) = div_rem(c1 + 4749 * n3 +   42 * n2,                  10000);
    let (d4, d3) = div_rem(c2 +  281 * n3,                              10000);

    macro_rules! quad(
        ($d:expr, $i:expr) => ({
            let (qq, rr) = div_rem($d, 100);
            buf[$i  ] = TENS[qq as uint]; buf[$i+1] = ONES[qq as uint];
            buf[$i+2] = TENS[rr as uint]; buf[$i+3] = ONES[rr as uint];
        })
    )

    quad!(d4, 0)
    quad!(d3, 4)
    quad!(d2, 8)
    quad!(d1, 12)
    quad!(d0, 16)

    buf
}

pub fn u32_to_digits(n: u32) -> Digits32 {
    let mut buf: Digits32 = unsafe {mem::uninitialized()};

    let n0 = (n & 0xffff) as u32;
    let n1 = ((n >> 16) & 0xffff) as u32;

    let (c0, d0) = div_rem(     5536 * n1 + n0, 10000);
    let (d2, d1) = div_rem(c0 +    6 * n1,      10000);

    macro_rules! quad(
        ($d:expr, $i:expr) => ({
            let (qq, rr) = div_rem($d, 100);
            buf[$i  ] = TENS[qq as uint]; buf[$i+1] = ONES[qq as uint];
            buf[$i+2] = TENS[rr as uint]; buf[$i+3] = ONES[rr as uint];
        })
    )

    buf[0] = TENS[d2 as uint];
    buf[1] = ONES[d2 as uint];
    quad!(d1, 2)
    quad!(d0, 6)

    buf
}

#[cfg(test)] #[test]
fn sanity_test() {
    test::u64_sanity_test(u64_to_digits);
    test::u32_sanity_test(u32_to_digits);
}

#[cfg(test)] #[bench]
fn bench_u64(b: &mut test::Bencher) {
    test::rotating_bench(u64_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u32(b: &mut test::Bencher) {
    test::rotating_bench(u32_to_digits, b);
}


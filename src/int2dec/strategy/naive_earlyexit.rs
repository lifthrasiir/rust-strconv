use core::prelude::*;
use num::div_rem;

use int2dec::digits::{Digits64, Digits32, Digits16, Digits8};
use int2dec::digits::{NDIGITS64, NDIGITS32, NDIGITS16, NDIGITS8};

pub fn u64_to_digits(mut n: u64) -> Digits64 {
    let mut buf: Digits64 = [b'0'; NDIGITS64];
    for i in (0..NDIGITS64).rev() {
        if n == 0 { return buf; }
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u32_to_digits(mut n: u32) -> Digits32 {
    let mut buf: Digits32 = [b'0'; NDIGITS32];
    for i in (0..NDIGITS32).rev() {
        if n == 0 { return buf; }
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u16_to_digits(mut n: u16) -> Digits16 {
    let mut buf: Digits16 = [b'0'; NDIGITS16];
    for i in (0..NDIGITS16).rev() {
        if n == 0 { return buf; }
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}

pub fn u8_to_digits(mut n: u8) -> Digits8 {
    let mut buf: Digits8 = [b'0'; NDIGITS8];
    for i in (0..NDIGITS8).rev() {
        if n == 0 { return buf; }
        let (q, r) = div_rem(n, 10);
        buf[i] = r as u8 + b'0';
        n = q;
    }
    buf
}


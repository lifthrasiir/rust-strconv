use core::prelude::*;
use core::i16;
use core::num::Float;
pub use self::decoder::{decode, FullDecoded, Decoded};

pub mod estimator;
pub mod bignum;
pub mod decoder;

pub mod strategy {
    pub mod dragon;
    pub mod grisu;
}

#[cfg(test)] mod tests;

// it is a bit non-trivial to derive, but this is one plus the maximal number of
// significant decimal digits from formatting algorithms with the shortest result.
// the exact formula for this is: ceil(# bits in mantissa * log_10 2 + 1).
pub const MAX_SIG_DIGITS: usize = 17;

// when d[..n] contains decimal digits, increase the last digit and propagate carry.
// returns true when it causes the length change.
fn round_up(d: &mut [u8], n: usize) -> Option<u8> {
    match d[..n].iter().rposition(|&c| c != b'9') {
        Some(i) => { // d[i+1..n] is all nines
            d[i] += 1;
            for j in i+1..n { d[j] = b'0'; }
            None
        }
        None if n > 0 => { // 999..999 rounds to 1000..000 with an increased exponent
            d[0] = b'1';
            for j in 1..n { d[j] = b'0'; }
            Some(b'0')
        }
        None => { // an empty buffer rounds up (a bit strange but reasonable)
            Some(b'1')
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Part<'a> {
    Zero(usize),
    Num(u16),
    Copy(&'a [u8]),
}

impl<'a> Part<'a> {
    pub fn len(&self) -> usize {
        match *self {
            Part::Copy(buf) => buf.len(),
            Part::Num(v) => if v < 1_000 { if v < 10 { 1 } else if v < 100 { 2 } else { 3 } }
                            else { if v < 10_000 { 4 } else { 5 } },
            Part::Zero(nzeroes) => nzeroes,
        }
    }
}

fn digits_to_dec_str<'a>(buf: &'a [u8], exp: i16, frac_digits: usize,
                         parts: &mut [Part<'a>]) -> usize {
    assert!(!buf.is_empty());
    assert!(buf[0] > b'0');
    assert!(parts.len() >= 4);

    // if there is the restriction on the last digit position, `buf` is assumed to be
    // left-padded with the virtual zeroes. the number of virtual zeroes, `nzeroes`,
    // equals to `max(0, exp + frag_digits - buf.len())`, so that the position of
    // the last digit `exp - buf.len() - nzeroes` is no more than `-frac_digits`:
    //
    //                       |<-virtual->|
    //       |<---- buf ---->|  zeroes   |     exp
    //    0. 1 2 3 4 5 6 7 8 9 _ _ _ _ _ _ x 10
    //    |                  |           |
    // 10^exp    10^(exp-buf.len())   10^(exp-buf.len()-nzeroes)
    //
    // `nzeroes` is individually calculated for each case in order to avoid overflow.

    if exp <= 0 {
        // the decimal point is before rendered digits: [0.][000...000][1234][____]
        let minus_exp = -(exp as i32) as usize;
        parts[0] = Part::Copy(b"0.");
        parts[1] = Part::Zero(minus_exp);
        parts[2] = Part::Copy(buf);
        if frac_digits > buf.len() && frac_digits - buf.len() > minus_exp {
            parts[3] = Part::Zero((frac_digits - buf.len()) - minus_exp);
            4
        } else {
            3
        }
    } else {
        let exp = exp as usize;
        if exp < buf.len() {
            // the decimal point is inside rendered digits: [12][.][34][____]
            parts[0] = Part::Copy(&buf[..exp]);
            parts[1] = Part::Copy(b".");
            parts[2] = Part::Copy(&buf[exp..]);
            if frac_digits > buf.len() - exp {
                parts[3] = Part::Zero(frac_digits - (buf.len() - exp));
                4
            } else {
                3
            }
        } else {
            // the decimal point is after rendered digits: [1234____0000] or [1234][__][.][__].
            parts[0] = Part::Copy(buf);
            parts[1] = Part::Zero(exp - buf.len());
            if frac_digits > 0 {
                parts[2] = Part::Copy(b".");
                parts[3] = Part::Zero(frac_digits);
                4
            } else {
                2
            }
        }
    }
}

fn digits_to_exp_str<'a>(buf: &'a [u8], exp: i16, min_ndigits: usize, upper: bool,
                         parts: &mut [Part<'a>]) -> usize {
    assert!(!buf.is_empty());
    assert!(buf[0] > b'0');
    assert!(parts.len() >= 6);

    let mut n = 0;

    parts[n] = Part::Copy(&buf[..1]);
    n += 1;

    if buf.len() > 1 || min_ndigits > 1 {
        parts[n] = Part::Copy(b".");
        parts[n + 1] = Part::Copy(&buf[1..]);
        n += 2;
        if min_ndigits > buf.len() {
            parts[n] = Part::Zero(min_ndigits - buf.len());
            n += 1;
        }
    }

    // 0.1234 x 10^exp = 1.234 x 10^(exp-1)
    let exp = exp as i32 - 1; // avoid underflow when exp is i16::MIN
    if exp < 0 {
        parts[n] = Part::Copy(if upper { b"E-" } else { b"e-" });
        parts[n + 1] = Part::Num(-exp as u16);
        n + 2
    } else {
        parts[n] = Part::Copy(if upper { b"E" } else { b"e" });
        parts[n + 1] = Part::Num(exp as u16);
        n + 2
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sign {
    Minus,        // -1  0  0  1
    MinusPlus,    // -1 +0 +0 +1
    MinusPlusRaw, // -1 -0 +0 +1
}

fn determine_sign(sign: Sign, decoded: &FullDecoded, negative: bool) -> Option<Part<'static>> {
    const PLUS:  Part<'static> = Part::Copy(b"+");
    const MINUS: Part<'static> = Part::Copy(b"-");

    match (*decoded, sign) {
        (FullDecoded::Nan, _) => None,
        (FullDecoded::Zero, Sign::Minus) => None,
        (FullDecoded::Zero, Sign::MinusPlus) => Some(PLUS),
        (FullDecoded::Zero, Sign::MinusPlusRaw) =>
            if negative { Some(MINUS) } else { Some(PLUS) },
        (_, Sign::Minus) =>
            if negative { Some(MINUS) } else { None },
        (_, Sign::MinusPlus) | (_, Sign::MinusPlusRaw) =>
            if negative { Some(MINUS) } else { Some(PLUS) },
    }
}

pub fn to_shortest_str<'a, T, F>(mut format_shortest: F, v: T,
                                 sign: Sign, frac_digits: usize, upper: bool,
                                 buf: &'a mut [u8], parts: &mut [Part<'a>]) -> usize
        where T: Float + 'static,
              F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    assert!(parts.len() >= 5);
    assert!(buf.len() >= MAX_SIG_DIGITS);

    let (negative, full_decoded) = decode(v);
    let mut n = 0;
    if let Some(part) = determine_sign(sign, &full_decoded, negative) {
        parts[0] = part;
        n += 1;
    }
    match full_decoded {
        FullDecoded::Nan => {
            parts[n] = Part::Copy(if upper { b"NAN" } else { b"nan" });
            n + 1
        }
        FullDecoded::Infinite => {
            parts[n] = Part::Copy(if upper { b"INF" } else { b"inf" });
            n + 1
        }
        FullDecoded::Zero => {
            if frac_digits > 0 { // [0.][0000]
                parts[n] = Part::Copy(b"0.");
                parts[n + 1] = Part::Zero(frac_digits);
                n + 2
            } else {
                parts[n] = Part::Copy(b"0");
                n + 1
            }
        }
        FullDecoded::Finite(ref decoded) => {
            let (len, exp) = format_shortest(decoded, buf);
            n + digits_to_dec_str(&buf[..len], exp, frac_digits, &mut parts[n..])
        }
    }
}

// dec_bounds == (min, max) s.t. 10^min <= v < 10^max will be rendered as decimal
pub fn to_shortest_exp_str<'a, T, F>(mut format_shortest: F, v: T,
                                     sign: Sign, dec_bounds: (i16, i16), upper: bool,
                                     buf: &'a mut [u8], parts: &mut [Part<'a>]) -> usize
        where T: Float + 'static,
              F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    assert!(parts.len() >= 7);
    assert!(buf.len() >= MAX_SIG_DIGITS);
    assert!(dec_bounds.0 <= dec_bounds.1);

    let (negative, full_decoded) = decode(v);
    let mut n = 0;
    if let Some(part) = determine_sign(sign, &full_decoded, negative) {
        parts[0] = part;
        n += 1;
    }
    match full_decoded {
        FullDecoded::Nan => {
            parts[n] = Part::Copy(if upper { b"NAN" } else { b"nan" });
            n + 1
        }
        FullDecoded::Infinite => {
            parts[n] = Part::Copy(if upper { b"INF" } else { b"inf" });
            n + 1
        }
        FullDecoded::Zero => {
            parts[n] = if dec_bounds.0 <= 0 && 0 < dec_bounds.1 {
                Part::Copy(b"0")
            } else {
                Part::Copy(if upper { b"0E0" } else { b"0e0" })
            };
            n + 1
        }
        FullDecoded::Finite(ref decoded) => {
            let (len, exp) = format_shortest(decoded, buf);
            let vis_exp = exp as i32 - 1;
            if dec_bounds.0 as i32 <= vis_exp && vis_exp < dec_bounds.1 as i32 {
                n + digits_to_dec_str(&buf[..len], exp, 0, &mut parts[n..])
            } else {
                n + digits_to_exp_str(&buf[..len], exp, 0, upper, &mut parts[n..])
            }
        }
    }
}

// rather crude approximation (upper bound) for the maximum buffer size
// calculated from the given `decoded.exp`.
//
// the exact limit is:
// - when `exp < 0`, the maximum length is `ceil(log_10 (5^-exp * (2^64 - 1)))`
// - when `exp >= 0`, the maximum length is `ceil(log_10 (2^exp * (2^64 - 1)))`
//
// `ceil(log_10 (x^exp * (2^64 - 1)))` is less than `ceil(log_10 (2^64 - 1)) +
// ceil(exp * log_10 x)`, which is in turn less than `20 + (1 + exp * log_10 x)`.
// we use the facts that `log_10 2 < 5/16` and `log_10 5 < 12/16`, which is
// enough for our purposes.
//
// why do we need this? `format_exact` functions will fill the entire buffer
// unless limited by the last digit restriction, but it is possible that
// the number of digits requested is ridiculously large (say, 30,000 digits).
// the vast majority of buffer will be filled with zeroes, so we don't want to
// allocate all the buffer beforehand. consequently, for any given arguments,
// 826 bytes of buffer should be sufficient for `f64`. compare this with
// the actual number for the worst case: 770 bytes (when `exp = -1074`).
fn estimate_max_buf_len(exp: i16) -> usize {
    21 + ((if exp < 0 { -12 } else { 5 } * exp as i32) as usize >> 4)
}

pub fn to_exact_exp_str<'a, T, F>(mut format_exact: F, v: T,
                                  sign: Sign, ndigits: usize, upper: bool,
                                  buf: &'a mut [u8], parts: &mut [Part<'a>]) -> usize
        where T: Float + 'static,
              F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    assert!(parts.len() >= 7);
    assert!(ndigits > 0);

    let (negative, full_decoded) = decode(v);
    let mut n = 0;
    if let Some(part) = determine_sign(sign, &full_decoded, negative) {
        parts[0] = part;
        n += 1;
    }
    match full_decoded {
        FullDecoded::Nan => {
            parts[n] = Part::Copy(if upper { b"NAN" } else { b"nan" });
            n + 1
        }
        FullDecoded::Infinite => {
            parts[n] = Part::Copy(if upper { b"INF" } else { b"inf" });
            n + 1
        }
        FullDecoded::Zero => {
            if ndigits > 1 { // [0.][0000][e0]
                parts[n] = Part::Copy(b"0.");
                parts[n + 1] = Part::Zero(ndigits - 1);
                parts[n + 2] = Part::Copy(if upper { b"E0" } else { b"e0" });
                n + 3
            } else {
                parts[n] = Part::Copy(if upper { b"0E0" } else { b"0e0" });
                n + 1
            }
        }
        FullDecoded::Finite(ref decoded) => {
            let maxlen = estimate_max_buf_len(decoded.exp);
            assert!(buf.len() >= ndigits || buf.len() >= maxlen);

            let trunc = if ndigits < maxlen { ndigits } else { maxlen };
            let (len, exp) = format_exact(decoded, &mut buf[..trunc], i16::MIN);
            n + digits_to_exp_str(&buf[..len], exp, ndigits, upper, &mut parts[n..])
        }
    }
}

pub fn to_exact_fixed_str<'a, T, F>(mut format_exact: F, v: T,
                                    sign: Sign, frac_digits: usize, upper: bool,
                                    buf: &'a mut [u8], parts: &mut [Part<'a>]) -> usize
        where T: Float + 'static,
              F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    assert!(parts.len() >= 6);

    let (negative, full_decoded) = decode(v);
    let mut n = 0;
    if let Some(part) = determine_sign(sign, &full_decoded, negative) {
        parts[0] = part;
        n += 1;
    }
    match full_decoded {
        FullDecoded::Nan => {
            parts[n] = Part::Copy(if upper { b"NAN" } else { b"nan" });
            n + 1
        }
        FullDecoded::Infinite => {
            parts[n] = Part::Copy(if upper { b"INF" } else { b"inf" });
            n + 1
        }
        FullDecoded::Zero => {
            if frac_digits > 0 { // [0.][0000]
                parts[n] = Part::Copy(b"0.");
                parts[n + 1] = Part::Zero(frac_digits);
                n + 2
            } else {
                parts[n] = Part::Copy(b"0");
                n + 1
            }
        }
        FullDecoded::Finite(ref decoded) => {
            let maxlen = estimate_max_buf_len(decoded.exp);
            assert!(buf.len() >= maxlen);

            // it *is* possible that `frac_digits` is ridiculously large.
            // `format_exact` will end rendering digits much earlier in this case,
            // because we are strictly limited by `maxlen`.
            let limit = if frac_digits < 0x8000 { -(frac_digits as i16) } else { i16::MIN };
            let (len, exp) = format_exact(decoded, &mut buf[..maxlen], limit);
            if exp <= limit {
                // `format_exact` always returns at least one digit even though the restriction
                // hasn't been met, so we catch this condition and treats as like zeroes.
                // this does not include the case that the restriction has been met
                // only after the final rounding-up; it's a regular case with `exp = limit + 1`.
                debug_assert_eq!(len, 0);
                if frac_digits > 0 { // [0.][0000]
                    parts[n] = Part::Copy(b"0.");
                    parts[n + 1] = Part::Zero(frac_digits);
                    n + 2
                } else {
                    parts[n] = Part::Copy(b"0");
                    n + 1
                }
            } else {
                n + digits_to_dec_str(&buf[..len], exp, frac_digits, &mut parts[n..])
            }
        }
    }
}


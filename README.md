# Rust-strconv

Experimental string-to-number and number-to-string conversion libraries and benchmarks for Rust.

## Notes

Benchmarks are done with `cargo bench` with its pros and cons.
Take a grain of salt when interpreting the result.

The benchmarks refer to the following machines at the author's disposal:

* "Slow laptop": Intel Celeron 1037U, 4G RAM, x86\_64 GNU/Linux 3.13.0

## `int2dec`

Integer to decimal string of the fixed size (zero-padded).

`Cargo bench` result from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1459 (491) | 1560 (105) | 2224 (13) | 6774 (735)
`div100` | 547 (12) | 832 (3) | 1424 (29) | 4048 (32)
`bcd` | N/A | N/A | 1611 (24) | 3531 (31)


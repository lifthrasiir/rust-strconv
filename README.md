# Rust-strconv

Experimental string-to-number and number-to-string conversion libraries and benchmarks for Rust.

## Notes

Benchmarks are done with `cargo bench | tee /dev/stderr | ./mkbenchtab`.
Take a grain of salt when interpreting the result.

The benchmarks refer to the following machines at the author's disposal:

* "Slow laptop": Intel Celeron 1037U, 4G RAM, x86\_64 GNU/Linux 3.13.0
* "Fast server": AMD Phenom II X4 945, 20G RAM, x86\_64 GNU/Linux 3.0.0

## `int2dec`

Integer to decimal string of the fixed size (zero-padded).

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 973 (9) | 1040 (8) | 1482 (4) | 4520 (25)
`bcd` | N/A | N/A | 1065 (16) | **2360 (16)**
`div100` | **370 (3)** | **562 (7)** | **948 (3)** | 2538 (12)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 588 (1) | 778 (4) | 1317 (7) | 5590 (18)
`bcd` | N/A | N/A | 1077 (4) | **1852 (9)**
`div100` | **324 (1)** | **502 (2)** | **975 (6)** | 2338 (6)


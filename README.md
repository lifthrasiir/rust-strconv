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
`naive` | 1457 (432) | 1560 (11) | 2223 (747) | 6775 (1477)
`bcd` | N/A | N/A | 1588 (10) | **3535 (45)**
`div100` | **532 (132)** | **833 (169)** | **1422 (469)** | 3811 (79)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 588 (1) | 778 (4) | 1317 (7) | 5590 (18)
`bcd` | N/A | N/A | 1077 (4) | **1852 (9)**
`div100` | **324 (1)** | **502 (2)** | **975 (6)** | 2338 (6)


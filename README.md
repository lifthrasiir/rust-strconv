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
`naive_uninit` | 1616 (12) | 1380 (580) | 2185 (690) | 6782 (2095)
`bcd` | N/A | N/A | 1588 (10) | 3535 (45)
`bcd_uninit` | N/A | N/A | 1616 (10) | 3510 (1074)
`div100` | 532 (132) | 833 (169) | 1422 (469) | 3811 (79)
`div100_uninit` | 548 (17) | 833 (4) | 1421 (480) | 4038 (434)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 588 (1) | 777 (2) | 1317 (3) | 5586 (14)
`naive_uninit` | 704 (1) | 842 (2) | 1404 (2) | 5542 (12)
`bcd` | N/A | N/A | 1079 (9) | 1846 (5)
`bcd_uninit` | N/A | N/A | 1079 (3) | 1852 (5)
`div100` | 341 (1) | 500 (2) | 975 (2) | 2338 (7)
`div100_uninit` | 319 (0) | 498 (2) | 972 (2) | 2385 (5)


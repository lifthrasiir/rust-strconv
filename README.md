# Rust-strconv

Experimental string-to-number and number-to-string conversion libraries and benchmarks for Rust.

## Notes

Benchmarks are done with `cargo bench strategy | tee /dev/stderr | ./mkbenchtab`.
Take a grain of salt when interpreting the result.

The benchmarks refer to the following machines at the author's disposal:

* "Slow laptop": Intel Celeron 1037U, 4G RAM, x86\_64 GNU/Linux 3.13.0
* "Fast server": AMD Phenom II X4 945, 20G RAM, x86\_64 GNU/Linux 3.0.0

## `int2dec`

Integer to decimal string of the fixed size (zero-padded to the maximal size).

Testing is done by formatting two sets of 64 integers,
one smallish (starts at 4 and multiplies by 5/4 each time, the last number is 3424806) and
one larger (starts at 1 and multiplies by 3 each time, the last number is 100 bits long),
in order to model both a typical behavior and a worst-case behavior.

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1941 (8) | 2154 (19) | 2981 (10) | 8849 (51)
`bcd` | N/A | N/A | 2191 (2) | 5274 (26)
`bcd_earlyexit` | N/A | N/A | 2031 (7) | **3221 (24)**
`div100` | **759 (24)** | **1241 (10)** | 1942 (10) | 5120 (13)
`div100_earlyexit` | 921 (4) | 1307 (23) | **1598 (10)** | 5050 (153)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1234 (282) | 1567 (5) | 2694 (8) | 11198 (103)
`bcd` | N/A | N/A | 2188 (11) | 4382 (9)
`bcd_earlyexit` | N/A | N/A | 2149 (7) | **3210 (17)**
`div100` | **716 (237)** | **1021 (5)** | 1943 (8) | 4693 (16)
`div100_earlyexit` | 746 (285) | 1369 (6) | **1809 (8)** | 6644 (21)


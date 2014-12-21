# Rust-strconv

Experimental string-to-number and number-to-string conversion libraries and benchmarks for Rust.

## Notes

Benchmarks are done with `cargo bench strategy | tee /dev/stderr | ./mkbenchtab`.
Take a grain of salt when interpreting the result.

The benchmarks refer to the following machines at the author's disposal:

* "Slow laptop": Intel Celeron 1037U, 4G RAM, x86\_64 GNU/Linux 3.13.0
* "Fast server": AMD Phenom II X4 945, 20G RAM, x86\_64 GNU/Linux 3.0.0
* "Moderate desktop (32-bit)": AMD Trinity A10 5800K, 8G RAM, Windows 7 (32-bit MinGW-w64)
* "Moderate desktop (64-bit)": the same machine as above, but with 64-bit MinGW-w64

## `int2dec`

Integer to decimal string of the fixed size (zero-padded to the maximal size).

Testing is done by formatting two sets of 64 integers,
one smallish (starts at 4 and multiplies by 5/4 each time, the last number is 3424806) and
one larger (starts at 1 and multiplies by 3 each time, the last number is 100 bits long),
in order to model both a typical behavior and a worst-case behavior.

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1941 (7) | 2148 (3) | 2987 (19) | 8863 (55)
`naive_earlyexit` | 2131 (6) | 2600 (24) | 3016 (41) | 4683 (35)
`bcd` | N/A | N/A | 2195 (3) | 5257 (7)
`bcd_earlyexit` | N/A | N/A | 2039 (21) | **3226 (12)**
`div100` | **769 (5)** | **1230 (1)** | 1936 (3) | 5100 (43)
`div100_earlyexit` | 921 (3) | 1290 (9) | **1625 (5)** | 4915 (97)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1191 (2) | 1555 (3) | 2693 (6) | 11187 (38)
`naive_earlyexit` | 1847 (431) | 2289 (6) | 3210 (8) | 5176 (13)
`bcd` | N/A | N/A | 2185 (9) | 4383 (12)
`bcd_earlyexit` | N/A | N/A | 2173 (4) | **3212 (11)**
`div100` | **652 (1)** | **1028 (3)** | 1944 (5) | 4694 (15)
`div100_earlyexit` | 740 (27) | 1375 (9) | **1846 (8)** | 6684 (22)

Results from the moderate desktop, 32-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 715 (12) | 949 (28) | 2596 (121) | 47132 (7208)
`naive_earlyexit` | 1695 (57) | 2079 (87) | 2342 (56) | 26615 (1453)
`bcd` | N/A | N/A | 1608 (56) | 4914 (270)
`bcd_earlyexit` | N/A | N/A | 1967 (136) | **3571 (113)**
`div100` | 849 (67) | **727 (28)** | **1588 (111)** | 23151 (537)
`div100_earlyexit` | **543 (40)** | 901 (32) | 1785 (120) | 14204 (1450)

Results from the moderate desktop, 64-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1040 (61) | 1280 (71) | 3166 (270) | 7421 (269)
`naive_earlyexit` | 1655 (66) | 2011 (202) | 2578 (63) | 3989 (387)
`bcd` | N/A | N/A | 1879 (177) | 4321 (277)
`bcd_earlyexit` | N/A | N/A | 1790 (79) | **2968 (238)**
`div100` | **570 (23)** | **954 (59)** | 1807 (118) | 4851 (378)
`div100_earlyexit` | 675 (53) | 1060 (57) | **1546 (237)** | 3501 (245)

## `flt2dec`

Floating point number to decimal string for the shortest representation. In progress.

* `dragon` implements a variant of the Dragon algorithm originally described by Steele and White
  and re-refined by Burger and Dybvig (the refinement itself was known but only described later).
  Requires a quite bit of stack (max 2KB), and may pose a problem with constrained environments.
  (Status: Implemented and roughly tested, should convert to the common interface)
* `grisu_inexact` implements the Grisu2 algorithm described by Florian Loitsch.
  This *is* inexact, but is very fast and can be used as a replacement to `dragon`.
  (Status: I have a code but yet to integrate to strconv.)
* `grisu` implements the Grisu3 algorithm, which is a conditional algorithm similar to Grisu2.
  This returns either a formatted number or an error, in which case the caller should fall back.
  Both case is very fast so it is best to use with `dragon`.
  (Status: Not yet implemented.)


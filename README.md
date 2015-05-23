# Rust-strconv

[![Rust-strconv on Travis CI][travis-image]][travis]

[travis-image]: https://travis-ci.org/lifthrasiir/rust-strconv.png

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

Realistic benchmarks (`(1 - best strategy / system) * 100%`, bigger is better):

Machine         | `u8`   | `u16`  | `u32`  | `u64`
----------------|--------|--------|--------|-------
Laptop x86\_64  | -17.3% | -7.8%  | -16.0% | -8.0%
Server x86\_64  | -1.6%  | +0.3%  | -0.9%  | +2.7%
Desktop i686    | +5.8%  | +10.0% | -1.2%  | +69.3%
Desktop x86\_64 | +2.0%  | +3.8%  | +1.1%  | +0.8%

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1781 (5) | **1439 (9)** | **2290 (50)** | 22320 (165)
`naive_earlyexit` | 1810 (5) | 1671 (19) | 2668 (14) | 4143 (35)
`bcd` | N/A | N/A | 2862 (23) | 5598 (18)
`bcd_earlyexit` | N/A | N/A | 2644 (31) | **3512 (20)**
`div100` | **1484 (7)** | **1487 (15)** | 2839 (10) | 13292 (93)
`div100_earlyexit` | 1870 (8) | 2411 (10) | 2526 (26) | 5321 (26)
`div100_u32` | **1487 (16)** | 1655 (35) | N/A | 4683 (24)
`div100_u32_earlyexit` | 1923 (21) | 1615 (21) | N/A | **3503 (22)**
`best` | **1487 (14)** | **1439 (9)** | 2526 (8) | **3504 (49)**

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1138 (2) | **1165 (4)** | 2468 (5) | 22984 (58)
`naive_earlyexit` | 1292 (4) | 1546 (5) | 2184 (9) | 4309 (13)
`bcd` | N/A | N/A | 2175 (5) | 4697 (8)
`bcd_earlyexit` | N/A | N/A | 2012 (7) | **3079 (7)**
`div100` | **946 (2)** | 1312 (3) | 2109 (7) | 12739 (41)
`div100_earlyexit` | 1174 (14) | 1561 (9) | **1879 (9)** | 6269 (21)
`div100_u32` | **946 (2)** | 1249 (3) | N/A | 4522 (40)
`div100_u32_earlyexit` | 1155 (8) | 1494 (7) | N/A | 3314 (11)
`best` | **946 (3)** | **1164 (3)** | **1881 (7)** | 3314 (9)

Results from the moderate desktop, 32-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 787 (18) | 940 (30) | 2506 (642) | 42927 (829)
`naive_earlyexit` | 1058 (57) | 1134 (42) | 2608 (78) | 26858 (1664)
`bcd` | N/A | N/A | 2624 (175) | 4722 (415)
`bcd_earlyexit` | N/A | N/A | 2126 (121) | **3643 (1086)**
`div100` | 853 (162) | 938 (25) | 1603 (82) | 21678 (6470)
`div100_earlyexit` | 764 (37) | 993 (31) | **1352 (140)** | 14658 (2228)
`div100_u32` | **661 (20)** | **856 (91)** | N/A | 8729 (363)
`div100_u32_earlyexit` | 695 (38) | 937 (84) | N/A | 7099 (494)
`best` | 765 (27) | 939 (73) | **1350 (88)** | **3623 (198)**

Results from the moderate desktop, 64-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 1015 (33) | **974 (39)** | 3013 (46) | 9455 (257)
`naive_earlyexit` | 1149 (142) | 1213 (65) | 2332 (312) | 3817 (212)
`bcd` | N/A | N/A | 2179 (116) | 4487 (434)
`bcd_earlyexit` | N/A | N/A | **1934 (46)** | **3019 (70)**
`div100` | **827 (20)** | 1028 (80) | 2154 (98) | 4772 (90)
`div100_earlyexit` | 1000 (53) | 1543 (61) | **1934 (89)** | 3562 (198)
`div100_u32` | **795 (20)** | 1025 (67) | N/A | 4101 (315)
`div100_u32_earlyexit` | 987 (28) | 1106 (32) | N/A | **2977 (99)**
`best` | **795 (16)** | **977 (151)** | **1932 (53)** | **2975 (113)**

## `flt2dec`

**Note: This is now on the tree ([rust-lang/rust#24612](https://github.com/rust-lang/rust/pull/24612))!
This part of code remains available in the public domain for other uses.**

Floating point number to decimal string for the valid representation (i.e. rounds to
the original value when converted back).

There are three possible modes of string conversion:

* **Shortest**: Produces the shortest representation among all numbers that round to given value.
  If there are multiple shortest representations, the closest one should be used.
* **Exact**: Given the number of digits, produces the exactly rounded representation of given value.
  If the supplied buffer is enough for the exact representation, it stops at the last digit as well.
* **Fixed**: Produces the exactly rounded representation of given value up to
  given decimal position. The caller is expected to provide enough buffer.

In order to reduce the complexity, rust-strconv merges the fixed mode into the exact mode:
the exact mode implementation requires the "last-digit limitation" argument,
which limits the number of digits to be returned in addition to the buffer size.
(This argument is the same type to the exponent, and treated as such.)
The caller is expected to estimate the number of digits required. The number might be off by a bit,
so the caller should allocate a slightly larger buffer for the upper bound of estimate.
Every exact mode implementation is able to calculate the exact exponent,
so it adjusts for the last-digit limitation with almost no additional cost.
The original exact mode can be invoked via the most relaxed limitation, i.e. `i16::MIN`.

There are several strategies available:

* `dragon` implements a variant of the Dragon algorithm originally described by Steele and White
  and re-refined by Burger and Dybvig (the refinement itself was known but only described later).
  Requires a quite bit of stack (max 2KB), and may pose a problem with constrained environments.
  (Status: Implemented. Roughly tested.)
* `grisu` implements the Grisu3 algorithm described by Florian Loitsch.
  This returns either a formatted number or an error, in which case the caller should fall back.
  Both case is very fast so it can be used with other correct but slow algorithms like `dragon`.
  Uses about 1KB of precomputed table.
  (Status: Implemented. f32 shortest is tested exhaustively for f32, others are roughly tested.)
* `system` is a dummy strategy for the comparison; it is Rust's built-in string conversion.
  This incurs the allocation (there is no way to avoid that), and it produces an inexact result.
* `libc` is a dummy strategy for the comparison; it is C's `snprintf`.

We use 6 different benchmarks to see the rough performance characteristics of each strategy:

* `small_*` prints `3.141592f64`, `big_*` prints the maximum value for `f64` (~= `1.8 * 10^308`).
* `*_shortest` tests a "shortest" mode.
* `*_exact_3` tests an "exact" mode with the buffer of 3 significant digits.
* `*_exact_12` tests an "exact" mode with the buffer of 12 significant digits.
* `*_exact_inf` tests an "exact" mode with the large enough buffer that any correct strategy will
  produce all significant digits. (To be exact, we are using 1KB buffer.)

Some notes:

* While `grisu` is very fast, `*_exact_inf` tests are known to be the worst case of `grisu`;
  it *should* fall back to `dragon` strategy unconditionally. This explains seemingly worse
  performance of the corresponding test.
* Most major `libc` implementations use some sort of accurate printing algorithm, but details vary.
  Glibc uses a Dragon-like algorithm with GMP and prints every requested digit.
  Msvcrt, on the other hands, has an unspecified (but probably Dragon-like) algorithm but
  only prints the shortest representation when the large number of digits are requested:
  for example, `printf("%.30lf", 0.1)` would print `0.100000000000000000000000000000` instead of
  the exactly rounded value (`0.100000000000000005551115123126`).

Results from the slow laptop:

Strategy | `big_exact_3` | `big_exact_12` | `big_exact_inf` | `big_shortest` | `small_exact_3` | `small_exact_12` | `small_exact_inf` | `small_shortest`
---------|---------------|----------------|-----------------|----------------|-----------------|------------------|-------------------|-----------------
`dragon` | 2790 (22) | 5443 (43) | **106338 (1243)** | 9703 (251) | 423 (9) | 781 (14) | **3349 (18)** | 882 (14)
`grisu` | **77 (1)** | **158 (5)** | **106545 (705)** | **198 (2)** | **71 (0)** | **114 (1)** | **3496 (27)** | **124 (2)**
`libc` | 1364 (29) | N/A | N/A | N/A | 300 (4) | N/A | N/A | N/A
`system` | 760 (5) | N/A | 290697 (2761) | N/A | 483 (4) | N/A | 312 (5) | N/A

Results from the fast server:

Strategy | `big_exact_3` | `big_exact_12` | `big_exact_inf` | `big_shortest` | `small_exact_3` | `small_exact_12` | `small_exact_inf` | `small_shortest`
---------|---------------|----------------|-----------------|----------------|-----------------|------------------|-------------------|-----------------
`dragon` | 2406 (17) | 4376 (32) | **79446 (302)** | 7992 (26) | 319 (3) | 631 (55) | **2862 (30)** | 760 (12)
`grisu` | **124 (1)** | **242 (0)** | **79696 (372)** | **278 (3)** | **91 (1)** | **135 (2)** | **2947 (44)** | **140 (1)**
`libc` | 996 (17) | N/A | N/A | N/A | 242 (3) | N/A | N/A | N/A
`system` | 646 (12) | N/A | 256574 (754) | N/A | 429 (7) | N/A | 280 (2) | N/A

Results from the moderate desktop, 32-bit:

Strategy | `big_exact_3` | `big_exact_12` | `big_exact_inf` | `big_shortest` | `small_exact_3` | `small_exact_12` | `small_exact_inf` | `small_shortest`
---------|---------------|----------------|-----------------|----------------|-----------------|------------------|-------------------|-----------------
`dragon` | 3167 (395) | 6048 (420) | **133078 (7571)** | 9402 (742) | 368 (53) | 663 (123) | **5739 (335)** | 866 (96)
`grisu` | **76 (4)** | **128 (4)** | **133114 (3272)** | **229 (12)** | **65 (5)** | **124 (11)** | **5977 (588)** | **134 (11)**
`libc` | 739 (81) | N/A | N/A | N/A | 569 (42) | N/A | N/A | N/A
`system` | 741 (64) | N/A | 57011 (5916) | N/A | 448 (25) | N/A | 325 (29) | N/A

Results from the moderate desktop, 64-bit:

Strategy | `big_exact_3` | `big_exact_12` | `big_exact_inf` | `big_shortest` | `small_exact_3` | `small_exact_12` | `small_exact_inf` | `small_shortest`
---------|---------------|----------------|-----------------|----------------|-----------------|------------------|-------------------|-----------------
`dragon` | 2057 (130) | 4201 (719) | **78894 (1550)** | 8139 (819) | 289 (42) | 537 (28) | **2657 (112)** | 700 (55)
`grisu` | **48 (2)** | **94 (1)** | **78816 (3151)** | **123 (35)** | **43 (3)** | **72 (1)** | **2752 (265)** | **80 (6)**
`libc` | 625 (31) | N/A | N/A | N/A | 502 (46) | N/A | N/A | N/A
`system` | 460 (79) | N/A | 53343 (2435) | N/A | 343 (21) | N/A | 301 (99) | N/A


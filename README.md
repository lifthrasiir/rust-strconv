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

Realistic benchmarks (`(1 - best strategy / system) * 100%`, bigger is better):

Machine         | `u8`   | `u16`  | `u32`  | `u64`
----------------|--------|--------|--------|-------
Laptop x86\_64  | -8.9%  | +0.5%  | +11.5% | +7.6%
Server x86\_64  | -10.8% | +13.7% | +14.0% | +14.8%
Desktop i686    | +3.5%  | +13.8% | +7.0%  | +72.4%
Desktop x86\_64 | -9.3%  | +13.1% | +16.5% | +8.4%

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **1036 (5)** | 1192 (22) | 2180 (22) | 7876 (20)
`naive_earlyexit` | 3487 (32) | 3429 (8) | 2880 (10) | 4162 (32)
`bcd` | N/A | N/A | 2650 (7) | 5096 (65)
`bcd_earlyexit` | N/A | N/A | 2435 (13) | 3289 (9)
`div100` | 1460 (6) | 1424 (8) | **1448 (4)** | 12925 (61)
`div100_earlyexit` | 1428 (5) | **1040 (45)** | 2271 (15) | 4874 (8)
`div100_u32` | 1460 (9) | 1422 (14) | N/A | 3915 (20)
`div100_u32_earlyexit` | 1430 (14) | **1041 (6)** | N/A | **2866 (47)**
`best` | **1036 (3)** | 1424 (8) | **1449 (12)** | **2865 (11)**

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **643 (15)** | **972 (6)** | 2631 (20) | 10808 (59)
`naive_earlyexit` | 2164 (17) | 2338 (24) | 2964 (20) | 4614 (31)
`bcd` | N/A | N/A | 2047 (51) | 4211 (24)
`bcd_earlyexit` | N/A | N/A | 1949 (12) | **2893 (13)**
`div100` | 925 (5) | **970 (4)** | **1719 (14)** | 12685 (65)
`div100_earlyexit` | 926 (13) | **1013 (7)** | **1689 (11)** | 6222 (22)
`div100_u32` | 924 (4) | **970 (5)** | N/A | 3797 (59)
`div100_u32_earlyexit` | 927 (10) | **1014 (8)** | N/A | **2702 (6)**
`best` | **642 (2)** | **970 (4)** | **1715 (6)** | **2702 (11)**

Results from the moderate desktop, 32-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 709 (39) | 951 (111) | 2711 (88) | 48754 (3471)
`naive_earlyexit` | 1680 (79) | 2051 (58) | 2302 (199) | 26951 (1296)
`bcd` | N/A | N/A | 2585 (129) | 4988 (175)
`bcd_earlyexit` | N/A | N/A | 2108 (101) | **3654 (366)**
`div100` | 847 (12) | **733 (23)** | 1613 (65) | 22859 (635)
`div100_earlyexit` | **548 (34)** | 895 (72) | **1356 (54)** | 14955 (1292)
`div100_u32` | 849 (46) | 785 (30) | N/A | 9145 (464)
`div100_u32_earlyexit` | **543 (16)** | 901 (33) | N/A | 7309 (553)
`best` | **538 (37)** | **737 (63)** | **1355 (74)** | **3655 (396)**

Results from the moderate desktop, 64-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **582 (19)** | 912 (31) | 3010 (141) | 7245 (271)
`naive_earlyexit` | 2592 (123) | 2524 (134) | 2295 (44) | 3756 (162)
`bcd` | N/A | N/A | 1838 (120) | 3990 (112)
`bcd_earlyexit` | N/A | N/A | 1787 (77) | 2726 (169)
`div100` | 804 (28) | **779 (24)** | **1325 (82)** | 4554 (151)
`div100_earlyexit` | 825 (33) | **814 (18)** | 1484 (76) | 3196 (165)
`div100_u32` | 806 (21) | **778 (27)** | N/A | 3822 (805)
`div100_u32_earlyexit` | 814 (31) | **811 (45)** | N/A | **2482 (142)**
`best` | **583 (17)** | **778 (24)** | **1318 (71)** | **2497 (228)**

## `flt2dec`

Floating point number to decimal string for the valid representation (i.e. rounds to
the original value when converted back). In progress.

There are two possible modes of string conversion:

* **Shortest**: Produces the shortest representation among all numbers that round to given value.
  If there are multiple shortest representations, the closest one should be used.
* **Exact**: Given the number of digits, produces the exactly rounded representation of given value.
  If the supplied buffer is enough for the exact representation, it stops at the last digit as well.

There are several strategies available:

* `dragon` implements a variant of the Dragon algorithm originally described by Steele and White
  and re-refined by Burger and Dybvig (the refinement itself was known but only described later).
  Requires a quite bit of stack (max 2KB), and may pose a problem with constrained environments.
  (Status: Implemented. Roughly tested.)
* `grisu_inexact` implements the Grisu2 algorithm described by Florian Loitsch.
  This *is* inexact, but is very fast and can be used as a replacement to `dragon`.
  Uses about 1KB of precomputed table. (Status: I have a code but yet to integrate to strconv.)
* `grisu` implements the Grisu3 algorithm, which is a conditional algorithm similar to Grisu2.
  This returns either a formatted number or an error, in which case the caller should fall back.
  Both case is very fast so it is best to use with `dragon`. Shares the same precomputed table
  as `grisu_inexact`. (Status: Implemented, exact pending. Tested exhaustively for f32,
  roughly for f64.)
* `system` is a dummy strategy for the comparison; it is Rust's built-in string conversion.
  This incurs the allocation (there is no way to avoid that), and it produces an inexact result.
* `libc` is a dummy strategy for the comparison; it is C's `snprintf`.

We use 6 different benchmarks to see the rough performance characteristics of each strategy:

* `small_*` prints `3.141592f64`, `big_*` prints the maximum value for `f64` (~= `1.8 * 10^308`).
* `*_shortest` tests a "shortest" mode.
* `*_exact_3` tests an "exact" mode with the buffer of 3 significant digits.
* `*_exact_inf` tests an "exact" mode with the large enough buffer that any correct strategy will
  produce all significant digits. (To be exact, we are using 1KB buffer.)

Results from the slow laptop:

Strategy | `big_exact_3` | `big_exact_inf` | `big_shortest` | `small_exact_3` | `small_exact_inf` | `small_shortest`
---------|---------------|-----------------|----------------|-----------------|-------------------|-----------------
`dragon` | 4785 (100) | **134363 (1443)** | 14658 (403) | 864 (14) | 9657 (89) | 2216 (20)
`grisu` | N/A | N/A | **209 (3)** | N/A | N/A | **132 (0)**
`libc` | 1380 (17) | N/A | N/A | **313 (2)** | N/A | N/A
`system` | **747 (5)** | 290187 (2558) | N/A | 500 (16) | **328 (3)** | N/A


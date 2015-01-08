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
Laptop x86\_64  | -11.6% | +3.9%  | +11.7% | +11.7%
Server x86\_64  | -4.4%  | +3.8%  | +14.2% | +6.2%
Desktop i686    | -4.0%  | +15.1% | +12.5% | +71.9%
Desktop x86\_64 | -8.5%  | +12.8% | +18.1% | +13.7%

Results from the slow laptop:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **1036 (5)** | 1178 (3) | 2161 (20) | 7813 (61)
`naive_earlyexit` | 3455 (28) | 3342 (19) | 2891 (20) | 4166 (35)
`bcd` | N/A | N/A | 2691 (19) | 4929 (35)
`bcd_earlyexit` | N/A | N/A | 2456 (15) | **3347 (13)**
`div100` | 1458 (5) | 1426 (8) | **1410 (22)** | 12641 (53)
`div100_earlyexit` | 1439 (9) | **1053 (34)** | 2255 (14) | 4881 (32)

Results from the fast server:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **704 (7)** | 1017 (14) | 2621 (12) | 10830 (42)
`naive_earlyexit` | 2171 (12) | 2279 (13) | 2890 (21) | 4596 (24)
`bcd` | N/A | N/A | 2048 (15) | 4181 (21)
`bcd_earlyexit` | N/A | N/A | 1986 (22) | **2898 (23)**
`div100` | 925 (7) | **992 (4)** | **1571 (10)** | 12772 (54)
`div100_earlyexit` | 908 (15) | 1059 (15) | 1720 (12) | 6248 (32)

Results from the moderate desktop, 32-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | 732 (49) | 950 (34) | 2582 (87) | 48450 (1378)
`naive_earlyexit` | 1661 (55) | 2086 (66) | 2345 (66) | 26729 (964)
`bcd` | N/A | N/A | 2449 (636) | 4866 (263)
`bcd_earlyexit` | N/A | N/A | 2114 (61) | **3636 (643)**
`div100` | 848 (35) | **723 (26)** | 1604 (40) | 22531 (691)
`div100_earlyexit` | **538 (13)** | 903 (212) | **1359 (43)** | 14868 (1045)

Results from the moderate desktop, 64-bit:

Strategy | `u8` | `u16` | `u32` | `u64`
---------|------|-------|-------|------
`naive` | **580 (12)** | 908 (22) | 2991 (66) | 7192 (155)
`naive_earlyexit` | 2487 (74) | 2053 (57) | 2314 (60) | 3708 (54)
`bcd` | N/A | N/A | 1831 (48) | 3955 (82)
`bcd_earlyexit` | N/A | N/A | 1780 (27) | **2697 (47)**
`div100` | 805 (16) | **777 (23)** | **1319 (23)** | 4765 (53)
`div100_earlyexit` | 825 (36) | 843 (27) | 1464 (29) | 3173 (70)

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


/*
Rust adaptation of Grisu3 algorithm described in [1]. It uses about
1KB of precomputed table, and in turn, it's very quick for most inputs.

[1] Florian Loitsch. 2010. Printing floating-point numbers quickly and
    accurately with integers. SIGPLAN Not. 45, 6 (June 2010), 233-243.
*/

use std::num::{Int, Float};
use std::ops::SliceMut;
#[cfg(test)] use test;

use flt2dec::{Decoded, MAX_SIG_DIGITS};
#[cfg(test)] use flt2dec::testing;

#[derive(Copy, Show)]
struct Fp { f: u64, e: i16 }

impl Fp {
    fn mul(&self, other: &Fp) -> Fp {
        const MASK: u64 = 0xffffffff;
        let a = self.f >> 32;
        let b = self.f & MASK;
        let c = other.f >> 32;
        let d = other.f & MASK;
        let ac = a * c;
        let bc = b * c;
        let ad = a * d;
        let bd = b * d;
        let tmp = (bd >> 32) + (ad & MASK) + (bc & MASK) + (1 << 31) /* round */;
        let f = ac + (ad >> 32) + (bc >> 32) + (tmp >> 32);
        let e = self.e + other.e + 64;
        Fp { f: f, e: e }
    }

    fn normalize(&self) -> Fp {
        let mut f = self.f;
        let mut e = self.e;
        let min = 1 << 63;
        while f < min {
            f <<= 1;
            e -= 1;
        }
        Fp { f: f, e: e }
    }

    fn normalize_to(&self, e: i16) -> Fp {
        let edelta = self.e - e;
        assert!(edelta >= 0);
        let edelta = edelta as uint;
        assert_eq!(self.f << edelta >> edelta, self.f);
        Fp { f: self.f << edelta, e: e }
    }
}

/*
# the following Python code generates this table:
for i in xrange(-308, 333, 8):
    if i >= 0: f = 10**i; e = 0
    else: f = 2**(80-4*i) // 10**-i; e = 4 * i - 80
    l = f.bit_length()
    f = ((f << 64 >> (l-1)) + 1) >> 1; e += l - 64
    print '    (%#018x, %5d, %4d),' % (f, e, i)
*/
const CACHED_POW10: &'static [(u64, i16, i16)] = &[ // (f, e, k)
    (0xe61acf033d1a45df, -1087, -308),
    (0xab70fe17c79ac6ca, -1060, -300),
    (0xff77b1fcbebcdc4f, -1034, -292),
    (0xbe5691ef416bd60c, -1007, -284),
    (0x8dd01fad907ffc3c,  -980, -276),
    (0xd3515c2831559a83,  -954, -268),
    (0x9d71ac8fada6c9b5,  -927, -260),
    (0xea9c227723ee8bcb,  -901, -252),
    (0xaecc49914078536d,  -874, -244),
    (0x823c12795db6ce57,  -847, -236),
    (0xc21094364dfb5637,  -821, -228),
    (0x9096ea6f3848984f,  -794, -220),
    (0xd77485cb25823ac7,  -768, -212),
    (0xa086cfcd97bf97f4,  -741, -204),
    (0xef340a98172aace5,  -715, -196),
    (0xb23867fb2a35b28e,  -688, -188),
    (0x84c8d4dfd2c63f3b,  -661, -180),
    (0xc5dd44271ad3cdba,  -635, -172),
    (0x936b9fcebb25c996,  -608, -164),
    (0xdbac6c247d62a584,  -582, -156),
    (0xa3ab66580d5fdaf6,  -555, -148),
    (0xf3e2f893dec3f126,  -529, -140),
    (0xb5b5ada8aaff80b8,  -502, -132),
    (0x87625f056c7c4a8b,  -475, -124),
    (0xc9bcff6034c13053,  -449, -116),
    (0x964e858c91ba2655,  -422, -108),
    (0xdff9772470297ebd,  -396, -100),
    (0xa6dfbd9fb8e5b88f,  -369,  -92),
    (0xf8a95fcf88747d94,  -343,  -84),
    (0xb94470938fa89bcf,  -316,  -76),
    (0x8a08f0f8bf0f156b,  -289,  -68),
    (0xcdb02555653131b6,  -263,  -60),
    (0x993fe2c6d07b7fac,  -236,  -52),
    (0xe45c10c42a2b3b06,  -210,  -44),
    (0xaa242499697392d3,  -183,  -36),
    (0xfd87b5f28300ca0e,  -157,  -28),
    (0xbce5086492111aeb,  -130,  -20),
    (0x8cbccc096f5088cc,  -103,  -12),
    (0xd1b71758e219652c,   -77,   -4),
    (0x9c40000000000000,   -50,    4),
    (0xe8d4a51000000000,   -24,   12),
    (0xad78ebc5ac620000,     3,   20),
    (0x813f3978f8940984,    30,   28),
    (0xc097ce7bc90715b3,    56,   36),
    (0x8f7e32ce7bea5c70,    83,   44),
    (0xd5d238a4abe98068,   109,   52),
    (0x9f4f2726179a2245,   136,   60),
    (0xed63a231d4c4fb27,   162,   68),
    (0xb0de65388cc8ada8,   189,   76),
    (0x83c7088e1aab65db,   216,   84),
    (0xc45d1df942711d9a,   242,   92),
    (0x924d692ca61be758,   269,  100),
    (0xda01ee641a708dea,   295,  108),
    (0xa26da3999aef774a,   322,  116),
    (0xf209787bb47d6b85,   348,  124),
    (0xb454e4a179dd1877,   375,  132),
    (0x865b86925b9bc5c2,   402,  140),
    (0xc83553c5c8965d3d,   428,  148),
    (0x952ab45cfa97a0b3,   455,  156),
    (0xde469fbd99a05fe3,   481,  164),
    (0xa59bc234db398c25,   508,  172),
    (0xf6c69a72a3989f5c,   534,  180),
    (0xb7dcbf5354e9bece,   561,  188),
    (0x88fcf317f22241e2,   588,  196),
    (0xcc20ce9bd35c78a5,   614,  204),
    (0x98165af37b2153df,   641,  212),
    (0xe2a0b5dc971f303a,   667,  220),
    (0xa8d9d1535ce3b396,   694,  228),
    (0xfb9b7cd9a4a7443c,   720,  236),
    (0xbb764c4ca7a44410,   747,  244),
    (0x8bab8eefb6409c1a,   774,  252),
    (0xd01fef10a657842c,   800,  260),
    (0x9b10a4e5e9913129,   827,  268),
    (0xe7109bfba19c0c9d,   853,  276),
    (0xac2820d9623bf429,   880,  284),
    (0x80444b5e7aa7cf85,   907,  292),
    (0xbf21e44003acdd2d,   933,  300),
    (0x8e679c2f5e44ff8f,   960,  308),
    (0xd433179d9c8cb841,   986,  316),
    (0x9e19db92b4e31ba9,  1013,  324),
    (0xeb96bf6ebadf77d9,  1039,  332),
];

const CACHED_POW10_FIRST_E: i16 = -1087;
const CACHED_POW10_LAST_E: i16 = 1039;

fn cached_power(alpha: i16, gamma: i16) -> (i16, Fp) {
    let offset = CACHED_POW10_FIRST_E as i32;
    let range = (CACHED_POW10.len() as i32) - 1;
    let domain = (CACHED_POW10_LAST_E - CACHED_POW10_FIRST_E) as i32;
    let idx = ((gamma as i32) - offset) * range / domain;
    let (f, e, k) = CACHED_POW10[idx as uint];
    debug_assert!(alpha <= e && e <= gamma);
    (k, Fp { f: f, e: e })
}

#[cfg(test)] #[test]
fn test_cached_power() {
    assert_eq!(CACHED_POW10.first().unwrap().1, CACHED_POW10_FIRST_E);
    assert_eq!(CACHED_POW10.last().unwrap().1, CACHED_POW10_LAST_E);

    let alpha = -60;
    let gamma = -32;
    for e in range(-1137, 961) { // full range for f64
        let low = alpha - e - 64;
        let high = gamma - e - 64;
        let (_k, cached) = cached_power(low, high);
        assert!(low <= cached.e && cached.e <= high,
                "cached_power({}, {}) = {} is incorrect", low, high, cached);
    }
}

// given `x > 0`, `max_pow10_less_than(x) = (k, 10^k)` such that `10^k < x <= 10^(k+1)`.
fn max_pow10_less_than(x: u32) -> (u8, u32) {
    debug_assert!(x > 0);

    const X9: u32 = 10_0000_0000;
    const X8: u32 =  1_0000_0000;
    const X7: u32 =    1000_0000;
    const X6: u32 =     100_0000;
    const X5: u32 =      10_0000;
    const X4: u32 =       1_0000;
    const X3: u32 =         1000;
    const X2: u32 =          100;
    const X1: u32 =           10;

    if x < X4 {
        if x < X2 { if x < X1 {(0,  1)} else {(1, X1)} }
        else      { if x < X3 {(2, X2)} else {(3, X3)} }
    } else {
        if x < X6      { if x < X5 {(4, X4)} else {(5, X5)} }
        else if x < X8 { if x < X7 {(6, X6)} else {(7, X7)} }
        else           { if x < X9 {(8, X8)} else {(9, X9)} }
    }
}

#[cfg(test)] #[test]
fn test_max_pow10_less_than() {
    let mut prevtenk = 1;
    for k in range(1, 10) {
        let tenk = prevtenk * 10;
        assert_eq!(max_pow10_less_than(tenk - 1), (k - 1, prevtenk));
        assert_eq!(max_pow10_less_than(tenk), (k, tenk));
        prevtenk = tenk;
    }
}

pub fn format_shortest_opt(d: &Decoded, buf: &mut [u8]) -> Option<(/*#digits*/ uint, /*exp*/ i16)> {
    assert!(d.mant > 0);
    assert!(d.minus > 0);
    assert!(d.plus > 0);
    assert!(d.mant.checked_add(d.plus).is_some());
    assert!(d.mant.checked_sub(d.minus).is_some());
    assert!(buf.len() >= MAX_SIG_DIGITS);
    assert!(d.mant + d.plus < (1 << 61)); // we need at least three bits of additional precision

    // start with the normalized values with the shared exponent
    let plus = Fp { f: d.mant + d.plus, e: d.exp }.normalize();
    let minus = Fp { f: d.mant - d.minus, e: d.exp }.normalize_to(plus.e);
    let v = Fp { f: d.mant, e: d.exp }.normalize_to(plus.e);

    // find any `cached = 10^minusk` such that `alpha <= minusk + plus.e + 64 <= gamma`.
    // since `plus` is normalized, this means `2^(62 + alpha) <= plus * cached < 2^(64 + gamma)`;
    // given our choices of `alpha` and `gamma`, this puts `plus * cached` into `[4, 2^32)`.
    //
    // it is obviously desirable to maximize `gamma - alpha`,
    // so that we don't need many cached powers of 10, but there are some considerations:
    //
    // 1. we want to keep `floor(plus * cached)` within `u32` since it needs a costly division.
    //    (this is not really avoidable, remainder is required for accuracy estimation.)
    // 2. the remainder of `floor(plus * cached)` repeatedly gets multiplied by 10,
    //    and it should not overflow.
    // 
    // the first gives `64 + gamma <= 32`, while the second gives `10 * 2^-alpha <= 2^64`;
    // -60 and -32 is the maximal range with this constraint, and V8 also uses them.
    let alpha = -60;
    let gamma = -32;
    let (minusk, cached) = cached_power(alpha - plus.e - 64, gamma - plus.e - 64);

    // scale fps.
    let plus = plus.mul(&cached);
    let minus = minus.mul(&cached);
    let v = v.mul(&cached);
    debug_assert_eq!(plus.e, minus.e);
    debug_assert_eq!(plus.e, v.e);

    //         +- actual range of minus
    //   | <---|---------------------- unsafe region --------------------------> |
    //   |     |                                                                 |
    //   |  |<--->|  | <--------------- safe region ---------------> |           |
    //   |  |     |  |                                               |           |
    //   |1 ulp|1 ulp|                 |1 ulp|1 ulp|                 |1 ulp|1 ulp|
    //   |<--->|<--->|                 |<--->|<--->|                 |<--->|<--->|
    //   |-----|-----|-------...-------|-----|-----|-------...-------|-----|-----|
    //   |   minus   |                 |     v     |                 |   plus    |
    // minus1     minus0           v - 1 ulp   v + 1 ulp           plus0       plus1
    //
    // above `minus`, `v` and `plus` are *quantized* approximations (error <= 0.5 ulp).
    // as we don't know the error is positive or negative, we use two approximations spaced equally
    // and have the maximal error of 1.5 ulps; the combined error will be exactly 2 ulps.
    //
    // the "unsafe region" is a liberal interval which we initially generate.
    // the "safe region" is a conservative interval which we only accept.
    // we start with the correct repr within the unsafe region, and try to find the closest repr
    // to `v` which is also within the safe region. if we can't, we give up.
    let plus1 = plus.f + 1;
//  let plus0 = plus.f - 1; // only for explanation
//  let minus0 = minus.f + 1; // only for explanation
    let minus1 = minus.f - 1;
    let e = -plus.e as uint; // shared exponent

    // divide `plus1` into integral and fractional parts.
    // integral parts are guaranteed to fit in u32, since cached power guarantees `plus < 2^32`
    // and normalized `plus.f` is always less than `2^64 - 2^4` due to the precision requirement.
    let plus1int = (plus1 >> e) as u32;
    let plus1frac = plus1 & ((1 << e) - 1);

    // calculate the largest `10^max_kappa` less than `plus1` (thus `plus1 <= 10^(max_kappa+1)`).
    // this is an upper bound of `kappa` below.
    let (max_kappa, max_ten_kappa) = max_pow10_less_than(plus1int);

    let mut i = 0;
    let exp = max_kappa as i16 - minusk + 1;

    // Theorem 6.2: if `k` is the greatest integer s.t. `0 <= y mod 10^k <= y - x`,
    //              then `V = floor(y / 10^k) * 10^k` is in `[x, y]` and one of the shortest
    //              representations (with the minimal number of significant digits) in that range.
    //
    // find the digit length `kappa` between `(minus1, plus1)` as per Theorem 6.2.
    // Theorem 6.2 can be adopted to exclude `x` by requiring `y mod 10^k < y - x` instead.
    // (e.g. `x` = 32000, `y` = 32777; `kappa` = 2 since `y mod 10^3 = 777 < y - x = 777`.)
    // the algorithm relies on the later verification phase to exclude `y`.
    let delta1 = plus1 - minus1;
//  let delta1int = (delta1 >> e) as uint; // only for explanation
    let delta1frac = delta1 & ((1 << e) - 1);

    // render integral parts, while checking for the accuracy at each step.
    let mut kappa = max_kappa as i16;
    let mut ten_kappa = max_ten_kappa; // 10^kappa
    let mut remainder = plus1int; // digits yet to be rendered
    loop { // we always have at least one digit to render, as `plus1 >= 10^kappa`
        // invariants:
        // - `delta1int <= remainder < 10^(kappa+1)`
        // - `plus1int = d[0..n-1] * 10^(kappa+1) + remainder`
        //   (it follows that `remainder = plus1int % 10^(kappa+1)`)

        // divide `remainder` by `10^kappa`. both are scaled by `2^-e`.
        let q = remainder / ten_kappa;
        let r = remainder % ten_kappa;
        debug_assert!(q < 10);
        buf[i] = b'0' + q as u8;
        i += 1;

        let plus1rem = ((r as u64) << e) + plus1frac; // == plus1 % (10^kappa * 2^e)
        if plus1rem < delta1 {
            // `plus1 % 10^kappa < delta1 = plus1 - minus1`; we've found the correct `kappa`.
            let ten_kappa = (ten_kappa as u64) << e; // scale 10^kappa back to the shared exponent
            return round_and_weed(buf.slice_to_or_fail_mut(&i), exp, plus1rem, delta1,
                                  plus1 - v.f, ten_kappa, 1);
        }

        // break the loop when we have rendered all integral digits.
        // the exact number of digits is `max_kappa + 1` as `plus1 < 10^(max_kappa+1)`.
        if i > max_kappa as uint {
            debug_assert_eq!(ten_kappa, 1);
            debug_assert_eq!(kappa, 0);
            break;
        }

        // restore invariants
        kappa -= 1;
        ten_kappa /= 10;
        remainder = r;
    }

    // render fractional parts, while checking for the accuracy at each step.
    // this time we rely on repeated multiplications, as division will lose the precision.
    let mut remainder = plus1frac;
    let mut threshold = delta1frac;
    let mut ulp = 1;
    loop { // the next digit should be significant as we've tested that before breaking out
        // invariants, where `m = max_kappa + 1` (# of digits in the integral part):
        // - `remainder < 2^e`
        // - `plus1frac * 10^(n-m) = d[m..n-1] * 2^e + remainder`

        remainder *= 10; // won't overflow, `2^e * 10 < 2^64`
        threshold *= 10; 
        ulp *= 10;

        // divide `remainder` by `10^kappa`.
        // both are scaled by `2^e / 10^kappa`, so the latter is implicit here.
        let q = remainder >> e;
        let r = remainder & ((1 << e) - 1);
        debug_assert!(q < 10);
        buf[i] = b'0' + q as u8;
        i += 1;

        if r < threshold {
            let ten_kappa = 1 << e; // implicit divisor
            return round_and_weed(buf.slice_to_or_fail_mut(&i), exp, r, threshold,
                                  (plus1 - v.f) * ulp, ten_kappa, ulp);
        }

        // restore invariants
        kappa -= 1;
        remainder = r;
    }

    // we've generated all significant digits of `plus1`, but not sure if it's the optimal one.
    // for example, if `minus1` is 3.14153... and `plus1` is 3.14158..., there are 5 different
    // shortest representation from 3.14154 to 3.14158 but we only have the greatest one.
    // we have to successively decrease the last digit and check if this is the optimal repr.
    // there are at most 9 candidates (..1 to ..9), so this is fairly quick. ("rounding" phase)
    //
    // the function checks if this "optimal" repr is actually within the ulp ranges,
    // and also, it is possible that the "second-to-optimal" repr can actually be optimal
    // due to the rounding error. in either cases this returns `None`. ("weeding" phase)
    //
    // all arguments here are scaled by the common (but implicit) value `k`, so that:
    // - `remainder = (plus1 % 10^kappa) * k`
    // - `threshold = (plus1 - minus1) * k` (and also, `remainder < threshold`)
    // - `plus1v = (plus1 - v) * k` (and also, `threshold > plus1v` from prior invariants)
    // - `ten_kappa = 10^kappa * k`
    // - `ulp = 2^-e * k`
    fn round_and_weed(buf: &mut [u8], exp: i16, remainder: u64, threshold: u64, plus1v: u64,
                      ten_kappa: u64, ulp: u64) -> Option<(uint, i16)> {
        assert!(!buf.is_empty());

        // produce two approximations to `v` (actually `plus1 - v`) within 1.5 ulps.
        // the resulting representation should be the closest representation to both.
        //
        // here `plus1 - v` is used since calculations are done with respect to `plus1`
        // in order to avoid overflow/underflow (hence the seemingly swapped names).
        let plus1v_down = plus1v + ulp; // plus1 - (v - 1 ulp)
        let plus1v_up = plus1v - ulp; // plus1 - (v + 1 ulp)

        // decrease the last digit and stop at the closest representation to `v + 1 ulp`.
        let mut plus1w = remainder; // plus1w(n) = plus1 - w(n)
        {
            let last = buf.last_mut().unwrap();

            // we work with the approximated digits `w(n)`, which is initially equal to `plus1 -
            // plus1 % 10^kappa`. after running the loop body `n` times, `w(n) = plus1 -
            // plus1 % 10^kappa - n * 10^kappa`. we set `plus1w(n) = plus1 - w(n) =
            // plus1 % 10^kappa + n * 10^kappa` (thus `remainder = plus1w(0)`) to simplify checks.
            // note that `plus1w(n)` is always increasing.
            //
            // we have three conditions to terminate. any of them will make the loop unable to
            // proceed, but we then have at least one valid representation known to be closest to
            // `v + 1 ulp` anyway. we will denote them as TC1 through TC3 for brevity.
            //
            // TC1: `w(n) <= v + 1 ulp`, i.e. this is the last repr that can be the closest one.
            // this is equivalent to `plus1 - w(n) = plus1w(n) >= plus1 - (v + 1 ulp) = plus1v_up`.
            // combined with TC2 (which checks if `w(n+1)` is valid), this prevents the possible
            // overflow on the calculation of `plus1w(n)`.
            //
            // TC2: `w(n+1) < minus1`, i.e. the next repr definitely does not round to `v`.
            // this is equivalent to `plus1 - w(n) + 10^kappa = plus1w(n) + 10^kappa >
            // plus1 - minus1 = threshold`. the left hand side can overflow, but we know
            // `threshold > plus1v`, so if TC1 is false, `threshold - plus1w(n) >
            // threshold - (plus1v - 1 ulp) > 1 ulp` and we can safely test if
            // `threshold - plus1w(n) < 10^kappa` instead.
            //
            // TC3: `abs(w(n) - (v + 1 ulp)) <= abs(w(n+1) - (v + 1 ulp))`, i.e. the next repr is
            // no closer to `v + 1 ulp` than the current repr. given `z(n) = plus1v_up - plus1w(n)`,
            // this becomes `abs(z(n)) <= abs(z(n+1))`. again assuming that TC1 is false, we have
            // `z(n) > 0`. we have two cases to consider:
            //
            // - when `z(n+1) >= 0`: TC3 becomes `z(n) <= z(n+1)`. as `plus1w(n)` is increasing,
            //   `z(n)` should be decreasing and this is clearly false.
            // - when `z(n+1) < 0`:
            //   - TC3a: the precondition is `plus1v_up < plus1w(n) + 10^kappa`. assuming TC2 is
            //     false, `threshold >= plus1w(n) + 10^kappa` so it cannot overflow.
            //   - TC3b: TC3 becomes `z(n) <= -z(n+1)`, i.e. `plus1v_up - plus1w(n) >=
            //     plus1w(n+1) - plus1v_up = plus1w(n) + 10^kappa - plus1v_up`. the negated TC1
            //     gives `plus1v_up > plus1w(n)`, so it cannot overflow or underflow when
            //     combined with TC3a.
            //
            // consequently, we should stop when `TC1 || TC2 || (TC3a && TC3b)`. the following is
            // equal to its inverse, `!TC1 && !TC2 && (!TC3a || !TC3b)`.
            while plus1w < plus1v_up &&
                  threshold - plus1w >= ten_kappa &&
                  (plus1w + ten_kappa < plus1v_up ||
                   plus1v_up - plus1w >= plus1w + ten_kappa - plus1v_up) {
                *last -= 1;
                debug_assert!(*last > b'0'); // the shortest repr cannot end with `0`
                plus1w += ten_kappa;
            }
        }

        // check if this representation is also the closest representation to `v - 1 ulp`.
        //
        // this is simply same to the terminating conditions for `v + 1 ulp`, with all `plus1v_up`
        // replaced by `plus1v_down` instead. overflow analysis equally holds.
        if plus1w < plus1v_down &&
           threshold - plus1w >= ten_kappa &&
           (plus1w + ten_kappa < plus1v_down ||
            plus1v_down - plus1w >= plus1w + ten_kappa - plus1v_down) {
            return None;
        }

        // now we have the closest representation to `v` between `plus1` and `minus1`.
        // this is too liberal, though, so we reject any `w(n)` not between `plus0` and `minus0`,
        // i.e. `plus1 - plus1w(n) <= minus0` or `plus1 - plus1w(n) >= plus0`. we utilize the facts
        // that `threshold = plus1 - minus1` and `plus1 - plus0 = minus0 - minus1 = 2 ulp`.
        if 2 * ulp <= plus1w && plus1w <= threshold - 4 * ulp {
            Some((buf.len(), exp))
        } else {
            None
        }
    }
}

pub fn format_shortest(d: &Decoded, buf: &mut [u8]) -> (/*#digits*/ uint, /*exp*/ i16) {
    use flt2dec::strategy::dragon::format_shortest as fallback;
    match format_shortest_opt(d, buf) {
        Some(ret) => ret,
        None => fallback(d, buf),
    }
}

#[cfg(test)] #[test]
fn shortest_sanity_test() {
    testing::f64_shortest_sanity_test(format_shortest);
    testing::f32_shortest_sanity_test(format_shortest);
}

#[cfg(test)] #[test] #[ignore] // it is too expensive
fn shortest_f32_equivalence_test() {
    // it is hard to directly test the optimality of the output, but we can at least test if
    // two different algorithms agree to each other.
    //
    // this reports the progress and the number of f32 values returned `None`.
    // with `--nocapture` (and plenty of time and appropriate rustc flags), this should print:
    // `done, ignored=17643160 passed=2121451879 failed=0`.

    use flt2dec::strategy::dragon::format_shortest as fallback;
    testing::f32_equivalence_test(format_shortest_opt, fallback);
}

#[cfg(test)] #[bench]
fn bench_small_shortest(b: &mut test::Bencher) {
    use flt2dec::decode;
    let decoded = decode(3.141592f64);
    b.iter(|| { let mut buf = [0; MAX_SIG_DIGITS]; format_shortest(&decoded, &mut buf) });
}

#[cfg(test)] #[bench]
fn bench_big_shortest(b: &mut test::Bencher) {
    use flt2dec::decode;
    let v: f64 = Float::max_value();
    let decoded = decode(v);
    b.iter(|| { let mut buf = [0; MAX_SIG_DIGITS]; format_shortest(&decoded, &mut buf) });
}


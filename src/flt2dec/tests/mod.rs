use std::prelude::v1::*;
use std::{str, mem, i16, f32, f64, fmt};
use core::num::Float;
use std::num::Float as StdFloat;
use std::slice::bytes;
use std::rand;
use std::rand::distributions::{IndependentSample, Range};

use flt2dec::{decode, FullDecoded, Decoded, MAX_SIG_DIGITS, round_up, Part, Sign};
use flt2dec::{to_shortest_str, to_shortest_exp_str, to_exact_exp_str, to_exact_fixed_str};

pub use test::Bencher;

mod estimator;
mod bignum;
mod strategy {
    mod system;
    mod libc;
    mod dragon;
    mod grisu;
}

pub fn decode_finite<T: Float + 'static>(v: T) -> Decoded {
    match decode(v).1 {
        FullDecoded::Finite(decoded) => decoded,
        full_decoded => panic!("expected finite, got {:?} instead", full_decoded)
    }
}

macro_rules! check_shortest {
    ($f:ident($v:expr) => $buf:expr, $exp:expr) => (
        check_shortest!($f($v) => $buf, $exp;
                        "shortest mismatch for v={v}: actual {actual:?}, expected {expected:?}",
                        v = stringify!($v))
    );

    ($f:ident{$($k:ident: $v:expr),+} => $buf:expr, $exp:expr) => (
        check_shortest!($f{$($k: $v),+} => $buf, $exp;
                        "shortest mismatch for {v:?}: actual {actual:?}, expected {expected:?}",
                        v = Decoded { $($k: $v),+ })
    );

    ($f:ident($v:expr) => $buf:expr, $exp:expr; $fmt:expr, $($key:ident = $val:expr),*) => ({
        let mut buf = [b'_'; MAX_SIG_DIGITS];
        let (len, k) = $f(&decode_finite($v), &mut buf);
        assert!((&buf[..len], k) == ($buf, $exp),
                $fmt, actual = (str::from_utf8(&buf[..len]).unwrap(), k),
                      expected = (str::from_utf8($buf).unwrap(), $exp),
                      $($key = $val),*);
    });

    ($f:ident{$($k:ident: $v:expr),+} => $buf:expr, $exp:expr;
                                         $fmt:expr, $($key:ident = $val:expr),*) => ({
        let mut buf = [b'_'; MAX_SIG_DIGITS];
        let (len, k) = $f(&Decoded { $($k: $v),+ }, &mut buf);
        assert!((&buf[..len], k) == ($buf, $exp),
                $fmt, actual = (str::from_utf8(&buf[..len]).unwrap(), k),
                      expected = (str::from_utf8($buf).unwrap(), $exp),
                      $($key = $val),*);
    })
}

macro_rules! try_exact {
    ($f:ident($decoded:expr) => $buf:expr, $expected:expr, $expectedk:expr;
                                $fmt:expr, $($key:ident = $val:expr),*) => ({
        let (len, k) = $f($decoded, &mut $buf[..$expected.len()], i16::MIN);
        assert!((&$buf[..len], k) == ($expected, $expectedk),
                $fmt, actual = (str::from_utf8(&$buf[..len]).unwrap(), k),
                      expected = (str::from_utf8($expected).unwrap(), $expectedk),
                      $($key = $val),*);
    })
}

macro_rules! try_fixed {
    ($f:ident($decoded:expr) => $buf:expr, $request:expr, $expected:expr, $expectedk:expr;
                                $fmt:expr, $($key:ident = $val:expr),*) => ({
        let (len, k) = $f($decoded, &mut $buf[..], $request);
        assert!((&$buf[..len], k) == ($expected, $expectedk),
                $fmt, actual = (str::from_utf8(&$buf[..len]).unwrap(), k),
                      expected = (str::from_utf8($expected).unwrap(), $expectedk),
                      $($key = $val),*);
    })
}

fn check_exact<F, T>(mut f: F, v: T, vstr: &str, expected: &[u8], expectedk: i16)
        where T: Float + 'static,
              F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    // use a large enough buffer
    let mut buf = [b'_'; 1024];
    let mut expected_ = [b'_'; 1024];

    let decoded = decode_finite(v);
    let cut = expected.iter().position(|&c| c == b' ');

    // check significant digits
    for i in 1..cut.unwrap_or(expected.len() - 1) {
        bytes::copy_memory(&mut expected_, &expected[..i]);
        let mut expectedk_ = expectedk;
        if expected[i] >= b'5' {
            // if this returns true, expected_[..i] is all `9`s and being rounded up.
            // we should always return `100..00` (`i` digits) instead, since that's
            // what we can came up with `i` digits anyway. `round_up` assumes that
            // the adjustment to the length is done by caller, which we simply ignore.
            if let Some(_) = round_up(&mut expected_, i) { expectedk_ += 1; }
        }

        try_exact!(f(&decoded) => &mut buf, &expected_[..i], expectedk_;
                   "exact sigdigit mismatch for v={v}, i={i}: \
                    actual {actual:?}, expected {expected:?}",
                   v = vstr, i = i);
        try_fixed!(f(&decoded) => &mut buf, expectedk_ - i as i16, &expected_[..i], expectedk_;
                   "fixed sigdigit mismatch for v={v}, i={i}: \
                    actual {actual:?}, expected {expected:?}",
                   v = vstr, i = i);
    }

    // check exact rounding for zero- and negative-width cases
    let start;
    if expected[0] >= b'5' {
        try_fixed!(f(&decoded) => &mut buf, expectedk, b"1", expectedk + 1;
                   "zero-width rounding-up mismatch for v={v}: \
                    actual {actual:?}, expected {expected:?}",
                   v = vstr);
        start = 1;
    } else {
        start = 0;
    }
    for i in start..-10 {
        try_fixed!(f(&decoded) => &mut buf, expectedk - i, b"", expectedk;
                   "rounding-down mismatch for v={v}, i={i}: \
                    actual {actual:?}, expected {expected:?}",
                   v = vstr, i = -i);
    }

    // check infinite zero digits
    if let Some(cut) = cut {
        for i in cut..expected.len()-1 {
            bytes::copy_memory(&mut expected_, &expected[..cut]);
            for c in &mut expected_[cut..i] { *c = b'0'; }

            try_exact!(f(&decoded) => &mut buf, &expected_[..i], expectedk;
                       "exact infzero mismatch for v={v}, i={i}: \
                        actual {actual:?}, expected {expected:?}",
                       v = vstr, i = i);
            try_fixed!(f(&decoded) => &mut buf, expectedk - i as i16, &expected_[..i], expectedk;
                       "fixed infzero mismatch for v={v}, i={i}: \
                        actual {actual:?}, expected {expected:?}",
                       v = vstr, i = i);
        }
    }
}

fn check_exact_one<F, T>(mut f: F, x: T, e: isize, tstr: &str, expected: &[u8], expectedk: i16)
        where T: Float + StdFloat + fmt::Display + 'static,
              F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    // use a large enough buffer
    let mut buf = [b'_'; 1024];
    let v: T = StdFloat::ldexp(x, e);
    let decoded = decode_finite(v);

    try_exact!(f(&decoded) => &mut buf, &expected, expectedk;
               "exact mismatch for v={x}p{e}{t}: actual {actual:?}, expected {expected:?}",
               x = x, e = e, t = tstr);
    try_fixed!(f(&decoded) => &mut buf, expectedk - expected.len() as i16, &expected, expectedk;
               "fixed mismatch for v={x}p{e}{t}: actual {actual:?}, expected {expected:?}",
               x = x, e = e, t = tstr);
}

macro_rules! check_exact {
    ($f:ident($v:expr) => $buf:expr, $exp:expr) => (
        check_exact(|d,b,k| $f(d,b,k), $v, stringify!($v), $buf, $exp)
    )
}

macro_rules! check_exact_one {
    ($f:ident($x:expr, $e:expr; $t:ty) => $buf:expr, $exp:expr) => (
        check_exact_one::<_, $t>(|d,b,k| $f(d,b,k), $x, $e, stringify!($t), $buf, $exp)
    )
}

// in the following comments, three numbers are spaced by 1 ulp apart,
// and the second one is being formatted.
//
// some tests are derived from [1].
//
// [1] Vern Paxson, A Program for Testing IEEE Decimal-Binary Conversion
//     ftp://ftp.ee.lbl.gov/testbase-report.ps.Z

pub fn f32_shortest_sanity_test<F>(mut f: F) where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    // 0.0999999940395355224609375
    // 0.100000001490116119384765625
    // 0.10000000894069671630859375
    check_shortest!(f(0.1f32) => b"1", 0);

    // 0.333333313465118408203125
    // 0.3333333432674407958984375 (1/3 in the default rounding)
    // 0.33333337306976318359375
    check_shortest!(f(1.0f32/3.0) => b"33333334", 0);

    // 10^1 * 0.31415917873382568359375
    // 10^1 * 0.31415920257568359375
    // 10^1 * 0.31415922641754150390625
    check_shortest!(f(3.141592f32) => b"3141592", 1);

    // 10^18 * 0.31415916243714048
    // 10^18 * 0.314159196796878848
    // 10^18 * 0.314159231156617216
    check_shortest!(f(3.141592e17f32) => b"3141592", 18);

    // 10^39 * 0.340282326356119256160033759537265639424
    // 10^39 * 0.34028234663852885981170418348451692544
    // 10^39 * 0.340282366920938463463374607431768211456
    check_shortest!(f(f32::MAX) => b"34028235", 39);

    // 10^-37 * 0.1175494210692441075487029444849287348827...
    // 10^-37 * 0.1175494350822287507968736537222245677818...
    // 10^-37 * 0.1175494490952133940450443629595204006810...
    check_shortest!(f(f32::MIN_POSITIVE) => b"11754944", -37);

    // 10^-44 * 0
    // 10^-44 * 0.1401298464324817070923729583289916131280...
    // 10^-44 * 0.2802596928649634141847459166579832262560...
    let minf32: f32 = StdFloat::ldexp(1.0, -149);
    check_shortest!(f(minf32) => b"1", -44);
}

pub fn f32_exact_sanity_test<F>(mut f: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    let minf32: f32 = StdFloat::ldexp(1.0, -149);

    check_exact!(f(0.1f32)            => b"100000001490116119384765625             ", 0);
    check_exact!(f(1.0f32/3.0)        => b"3333333432674407958984375               ", 0);
    check_exact!(f(3.141592f32)       => b"31415920257568359375                    ", 1);
    check_exact!(f(3.141592e17f32)    => b"314159196796878848                      ", 18);
    check_exact!(f(f32::MAX)          => b"34028234663852885981170418348451692544  ", 39);
    check_exact!(f(f32::MIN_POSITIVE) => b"1175494350822287507968736537222245677818", -37);
    check_exact!(f(minf32)            => b"1401298464324817070923729583289916131280", -44);

    // [1], Table 16: Stress Inputs for Converting 24-bit Binary to Decimal, < 1/2 ULP
    check_exact_one!(f(12676506.0, -102; f32) => b"2",            -23);
    check_exact_one!(f(12676506.0, -103; f32) => b"12",           -23);
    check_exact_one!(f(15445013.0,   86; f32) => b"119",           34);
    check_exact_one!(f(13734123.0, -138; f32) => b"3941",         -34);
    check_exact_one!(f(12428269.0, -130; f32) => b"91308",        -32);
    check_exact_one!(f(15334037.0, -146; f32) => b"171900",       -36);
    check_exact_one!(f(11518287.0,  -41; f32) => b"5237910",       -5);
    check_exact_one!(f(12584953.0, -145; f32) => b"28216440",     -36);
    check_exact_one!(f(15961084.0, -125; f32) => b"375243281",    -30);
    check_exact_one!(f(14915817.0, -146; f32) => b"1672120916",   -36);
    check_exact_one!(f(10845484.0, -102; f32) => b"21388945814",  -23);
    check_exact_one!(f(16431059.0,  -61; f32) => b"712583594561", -11);

    // [1], Table 17: Stress Inputs for Converting 24-bit Binary to Decimal, > 1/2 ULP
    check_exact_one!(f(16093626.0,   69; f32) => b"1",             29);
    check_exact_one!(f( 9983778.0,   25; f32) => b"34",            15);
    check_exact_one!(f(12745034.0,  104; f32) => b"259",           39);
    check_exact_one!(f(12706553.0,   72; f32) => b"6001",          29);
    check_exact_one!(f(11005028.0,   45; f32) => b"38721",         21);
    check_exact_one!(f(15059547.0,   71; f32) => b"355584",        29);
    check_exact_one!(f(16015691.0,  -99; f32) => b"2526831",      -22);
    check_exact_one!(f( 8667859.0,   56; f32) => b"62458507",      24);
    check_exact_one!(f(14855922.0,  -82; f32) => b"307213267",    -17);
    check_exact_one!(f(14855922.0,  -83; f32) => b"1536066333",   -17);
    check_exact_one!(f(10144164.0, -110; f32) => b"78147796834",  -26);
    check_exact_one!(f(13248074.0,   95; f32) => b"524810279937",  36);
}

pub fn f64_shortest_sanity_test<F>(mut f: F) where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    // 0.0999999999999999777955395074968691915273...
    // 0.1000000000000000055511151231257827021181...
    // 0.1000000000000000333066907387546962127089...
    check_shortest!(f(0.1f64) => b"1", 0);

    // this example is explicitly mentioned in the paper.
    // 10^3 * 0.0999999999999999857891452847979962825775...
    // 10^3 * 0.1 (exact)
    // 10^3 * 0.1000000000000000142108547152020037174224...
    check_shortest!(f(100.0f64) => b"1", 3);

    // 0.3333333333333332593184650249895639717578...
    // 0.3333333333333333148296162562473909929394... (1/3 in the default rounding)
    // 0.3333333333333333703407674875052180141210...
    check_shortest!(f(1.0f64/3.0) => b"3333333333333333", 0);

    // explicit test case for equally closest representations.
    // Dragon has its own tie-breaking rule; Grisu should fall back.
    // 10^1 * 0.1000007629394531027955395074968691915273...
    // 10^1 * 0.100000762939453125 (exact)
    // 10^1 * 0.1000007629394531472044604925031308084726...
    check_shortest!(f(1.00000762939453125f64) => b"10000076293945313", 1);

    // 10^1 * 0.3141591999999999718085064159822650253772...
    // 10^1 * 0.3141592000000000162174274009885266423225...
    // 10^1 * 0.3141592000000000606263483859947882592678...
    check_shortest!(f(3.141592f64) => b"3141592", 1);

    // 10^18 * 0.314159199999999936
    // 10^18 * 0.3141592 (exact)
    // 10^18 * 0.314159200000000064
    check_shortest!(f(3.141592e17f64) => b"3141592", 18);

    // pathological case: high = 10^23 (exact). tie breaking should always prefer that.
    // 10^24 * 0.099999999999999974834176
    // 10^24 * 0.099999999999999991611392
    // 10^24 * 0.100000000000000008388608
    check_shortest!(f(1.0e23f64) => b"1", 24);

    // 10^309 * 0.1797693134862315508561243283845062402343...
    // 10^309 * 0.1797693134862315708145274237317043567980...
    // 10^309 * 0.1797693134862315907729305190789024733617...
    check_shortest!(f(f64::MAX) => b"17976931348623157", 309);

    // 10^-307 * 0.2225073858507200889024586876085859887650...
    // 10^-307 * 0.2225073858507201383090232717332404064219...
    // 10^-307 * 0.2225073858507201877155878558578948240788...
    check_shortest!(f(f64::MIN_POSITIVE) => b"22250738585072014", -307);

    // 10^-323 * 0
    // 10^-323 * 0.4940656458412465441765687928682213723650...
    // 10^-323 * 0.9881312916824930883531375857364427447301...
    let minf64: f64 = StdFloat::ldexp(1.0, -1074);
    check_shortest!(f(minf64) => b"5", -323);
}

pub fn f64_exact_sanity_test<F>(mut f: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    let minf64: f64 = StdFloat::ldexp(1.0, -1074);

    check_exact!(f(0.1f64)            => b"1000000000000000055511151231257827021181", 0);
    check_exact!(f(0.45f64)           => b"4500000000000000111022302462515654042363", 0);
    check_exact!(f(0.95f64)           => b"9499999999999999555910790149937383830547", 0);
    check_exact!(f(100.0f64)          => b"1                                       ", 3);
    check_exact!(f(999.5f64)          => b"9995000000000000000000000000000000000000", 3);
    check_exact!(f(1.0f64/3.0)        => b"3333333333333333148296162562473909929394", 0);
    check_exact!(f(3.141592f64)       => b"3141592000000000162174274009885266423225", 1);
    check_exact!(f(3.141592e17f64)    => b"3141592                                 ", 18);
    check_exact!(f(1.0e23f64)         => b"99999999999999991611392                 ", 23);
    check_exact!(f(f64::MAX)          => b"1797693134862315708145274237317043567980", 309);
    check_exact!(f(f64::MIN_POSITIVE) => b"2225073858507201383090232717332404064219", -307);
    check_exact!(f(minf64)            => b"4940656458412465441765687928682213723650\
                                           5980261432476442558568250067550727020875\
                                           1865299836361635992379796564695445717730\
                                           9266567103559397963987747960107818781263\
                                           0071319031140452784581716784898210368871\
                                           8636056998730723050006387409153564984387\
                                           3124733972731696151400317153853980741262\
                                           3856559117102665855668676818703956031062\
                                           4931945271591492455329305456544401127480\
                                           1297099995419319894090804165633245247571\
                                           4786901472678015935523861155013480352649\
                                           3472019379026810710749170333222684475333\
                                           5720832431936092382893458368060106011506\
                                           1698097530783422773183292479049825247307\
                                           7637592724787465608477820373446969953364\
                                           7017972677717585125660551199131504891101\
                                           4510378627381672509558373897335989936648\
                                           0994116420570263709027924276754456522908\
                                           7538682506419718265533447265625         ", -323);

    // [1], Table 3: Stress Inputs for Converting 53-bit Binary to Decimal, < 1/2 ULP
    check_exact_one!(f(8511030020275656.0,  -342; f64) => b"9",                       -87);
    check_exact_one!(f(5201988407066741.0,  -824; f64) => b"46",                     -232);
    check_exact_one!(f(6406892948269899.0,   237; f64) => b"141",                      88);
    check_exact_one!(f(8431154198732492.0,    72; f64) => b"3981",                     38);
    check_exact_one!(f(6475049196144587.0,    99; f64) => b"41040",                    46);
    check_exact_one!(f(8274307542972842.0,   726; f64) => b"292084",                  235);
    check_exact_one!(f(5381065484265332.0,  -456; f64) => b"2891946",                -121);
    check_exact_one!(f(6761728585499734.0, -1057; f64) => b"43787718",               -302);
    check_exact_one!(f(7976538478610756.0,   376; f64) => b"122770163",               130);
    check_exact_one!(f(5982403858958067.0,   377; f64) => b"1841552452",              130);
    check_exact_one!(f(5536995190630837.0,    93; f64) => b"54835744350",              44);
    check_exact_one!(f(7225450889282194.0,   710; f64) => b"389190181146",            230);
    check_exact_one!(f(7225450889282194.0,   709; f64) => b"1945950905732",           230);
    check_exact_one!(f(8703372741147379.0,   117; f64) => b"14460958381605",           52);
    check_exact_one!(f(8944262675275217.0, -1001; f64) => b"417367747458531",        -285);
    check_exact_one!(f(7459803696087692.0,  -707; f64) => b"1107950772878888",       -196);
    check_exact_one!(f(6080469016670379.0,  -381; f64) => b"12345501366327440",       -98);
    check_exact_one!(f(8385515147034757.0,   721; f64) => b"925031711960365024",      233);
    check_exact_one!(f(7514216811389786.0,  -828; f64) => b"4198047150284889840",    -233);
    check_exact_one!(f(8397297803260511.0,  -345; f64) => b"11716315319786511046",    -87);
    check_exact_one!(f(6733459239310543.0,   202; f64) => b"432810072844612493629",    77);
    check_exact_one!(f(8091450587292794.0,  -473; f64) => b"3317710118160031081518", -126);

    // [1], Table 4: Stress Inputs for Converting 53-bit Binary to Decimal, > 1/2 ULP
    check_exact_one!(f(6567258882077402.0,   952; f64) => b"3",                       303);
    check_exact_one!(f(6712731423444934.0,   535; f64) => b"76",                      177);
    check_exact_one!(f(6712731423444934.0,   534; f64) => b"378",                     177);
    check_exact_one!(f(5298405411573037.0,  -957; f64) => b"4350",                   -272);
    check_exact_one!(f(5137311167659507.0,  -144; f64) => b"23037",                   -27);
    check_exact_one!(f(6722280709661868.0,   363; f64) => b"126301",                  126);
    check_exact_one!(f(5344436398034927.0,  -169; f64) => b"7142211",                 -35);
    check_exact_one!(f(8369123604277281.0,  -853; f64) => b"13934574",               -240);
    check_exact_one!(f(8995822108487663.0,  -780; f64) => b"141463449",              -218);
    check_exact_one!(f(8942832835564782.0,  -383; f64) => b"4539277920",              -99);
    check_exact_one!(f(8942832835564782.0,  -384; f64) => b"22696389598",             -99);
    check_exact_one!(f(8942832835564782.0,  -385; f64) => b"113481947988",            -99);
    check_exact_one!(f(6965949469487146.0,  -249; f64) => b"7700366561890",           -59);
    check_exact_one!(f(6965949469487146.0,  -250; f64) => b"38501832809448",          -59);
    check_exact_one!(f(6965949469487146.0,  -251; f64) => b"192509164047238",         -59);
    check_exact_one!(f(7487252720986826.0,   548; f64) => b"6898586531774201",        181);
    check_exact_one!(f(5592117679628511.0,   164; f64) => b"13076622631878654",        66);
    check_exact_one!(f(8887055249355788.0,   665; f64) => b"136052020756121240",      217);
    check_exact_one!(f(6994187472632449.0,   690; f64) => b"3592810217475959676",     224);
    check_exact_one!(f(8797576579012143.0,   588; f64) => b"89125197712484551899",    193);
    check_exact_one!(f(7363326733505337.0,   272; f64) => b"558769757362301140950",    98);
    check_exact_one!(f(8549497411294502.0,  -448; f64) => b"1176257830728540379990", -118);
}

pub fn more_shortest_sanity_test<F>(mut f: F) where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    check_shortest!(f{mant: 99_999_999_999_999_999, minus: 1, plus: 1,
                      exp: 0, inclusive: true} => b"1", 18);
    check_shortest!(f{mant: 99_999_999_999_999_999, minus: 1, plus: 1,
                      exp: 0, inclusive: false} => b"99999999999999999", 17);
}

fn iterate<F, G, V>(func: &str, k: usize, n: usize, mut f: F, mut g: G, mut v: V) -> (usize, usize)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16),
              V: FnMut(usize) -> Decoded {
    assert!(k <= 1024);

    let mut npassed = 0; // f(x) = Some(g(x))
    let mut nignored = 0; // f(x) = None

    for i in 0..n {
        if (i & 0xfffff) == 0 {
            println!("in progress, {:x}/{:x} (ignored={} passed={} failed={})",
                     i, n, nignored, npassed, i - nignored - npassed);
        }

        let decoded = v(i);
        let mut buf1 = [0; 1024];
        if let Some((len1, e1)) = f(&decoded, &mut buf1[..k]) {
            let mut buf2 = [0; 1024];
            let (len2, e2) = g(&decoded, &mut buf2[..k]);
            if e1 == e2 && &buf1[..len1] == &buf2[..len2] {
                npassed += 1;
            } else {
                println!("equivalence test failed, {:x}/{:x}: {:?} f(i)={}e{} g(i)={}e{}",
                         i, n, decoded, str::from_utf8(&buf1[..len1]).unwrap(), e1,
                                        str::from_utf8(&buf2[..len2]).unwrap(), e2);
            }
        } else {
            nignored += 1;
        }
    }
    println!("{}({}): done, ignored={} passed={} failed={}",
             func, k, nignored, npassed, n - nignored - npassed);
    assert!(nignored + npassed == n,
            "{}({}): {} out of {} values returns an incorrect value!",
            func, k, n - nignored - npassed, n);
    (npassed, nignored)
}

pub fn f32_random_equivalence_test<F, G>(f: F, g: G, k: usize, n: usize)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    let mut rng = rand::weak_rng();
    let f32_range = Range::new(0x0000_0001u32, 0x7f80_0000);
    iterate("f32_random_equivalence_test", k, n, f, g, |_| {
        let i: u32 = f32_range.ind_sample(&mut rng);
        let x: f32 = unsafe {mem::transmute(i)};
        decode_finite(x)
    });
}

pub fn f64_random_equivalence_test<F, G>(f: F, g: G, k: usize, n: usize)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    let mut rng = rand::weak_rng();
    let f64_range = Range::new(0x0000_0000_0000_0001u64, 0x7ff0_0000_0000_0000);
    iterate("f64_random_equivalence_test", k, n, f, g, |_| {
        let i: u64 = f64_range.ind_sample(&mut rng);
        let x: f64 = unsafe {mem::transmute(i)};
        decode_finite(x)
    });
}

pub fn f32_exhaustive_equivalence_test<F, G>(f: F, g: G, k: usize)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    // we have only 2^23 * (2^8 - 1) - 1 = 2,139,095,039 positive finite f32 values,
    // so why not simply testing all of them?
    //
    // this is of course very stressful (and thus should be behind an `#[ignore]` attribute),
    // but with `-O3 -C lto` this only takes about two hours or so.

    // iterate from 0x0000_0001 to 0x7f7f_ffff, i.e. all finite ranges
    let (npassed, nignored) = iterate("f32_exhaustive_equivalence_test",
                                      k, 0x7f7f_ffff, f, g, |i: usize| {
        let x: f32 = unsafe {mem::transmute(i as u32 + 1)};
        decode_finite(x)
    });
    assert_eq!((npassed, nignored), (2121451879, 17643160));
}

fn to_string_with_parts<F>(mut f: F) -> String
        where F: for<'a, 'b> FnMut(&'a mut [u8], &'b mut [Part<'a>]) -> usize {
    use std::{str, iter};

    let mut buf = [0; 1024];
    let mut parts = [Part::Zero(0); 16];
    let nparts = f(&mut buf, &mut parts);
    let mut ret = String::new();
    for part in &parts[..nparts] {
        match *part {
            Part::Copy(buf) => ret.push_str(str::from_utf8(buf).unwrap()),
            Part::Num(v) => ret.push_str(&format!("{}", v)),
            Part::Zero(nzeroes) => ret.extend(iter::repeat('0').take(nzeroes)),
        }
    }
    ret
}

pub fn to_shortest_str_test<F>(mut f_: F)
        where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    use super::Sign::*;

    fn to_string<T, F>(f: &mut F, v: T, sign: Sign, frac_digits: usize, upper: bool) -> String
            where T: Float + StdFloat + 'static,
                  F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
        to_string_with_parts(|buf, parts| to_shortest_str(|d,b| f(d,b), v, sign,
                                                          frac_digits, upper, buf, parts))
    }

    let f = &mut f_;

    assert_eq!(to_string(f,  0.0, Minus,        0, false), "0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    0, false), "+0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 0, false), "+0");
    assert_eq!(to_string(f, -0.0, Minus,        0, false), "0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    0, false), "+0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 0, false), "-0");
    assert_eq!(to_string(f,  0.0, Minus,        1,  true), "0.0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    1,  true), "+0.0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 1,  true), "+0.0");
    assert_eq!(to_string(f, -0.0, Minus,        8,  true), "0.00000000");
    assert_eq!(to_string(f, -0.0, MinusPlus,    8,  true), "+0.00000000");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 8,  true), "-0.00000000");

    assert_eq!(to_string(f,  1.0/0.0, Minus,         0, false), "inf");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlus,     0,  true), "+INF");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlusRaw,  0, false), "+inf");
    assert_eq!(to_string(f,  0.0/0.0, Minus,         1,  true), "NAN");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlus,     8, false), "nan");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlusRaw, 64,  true), "NAN");
    assert_eq!(to_string(f, -1.0/0.0, Minus,         1, false), "-inf");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlus,     8,  true), "-INF");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlusRaw, 64, false), "-inf");

    assert_eq!(to_string(f,  3.14, Minus,        0, false), "3.14");
    assert_eq!(to_string(f,  3.14, MinusPlus,    0, false), "+3.14");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 0, false), "+3.14");
    assert_eq!(to_string(f, -3.14, Minus,        0, false), "-3.14");
    assert_eq!(to_string(f, -3.14, MinusPlus,    0, false), "-3.14");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 0, false), "-3.14");
    assert_eq!(to_string(f,  3.14, Minus,        1,  true), "3.14");
    assert_eq!(to_string(f,  3.14, MinusPlus,    2,  true), "+3.14");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 3,  true), "+3.140");
    assert_eq!(to_string(f, -3.14, Minus,        8,  true), "-3.14000000");
    assert_eq!(to_string(f, -3.14, MinusPlus,    8,  true), "-3.14000000");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 8,  true), "-3.14000000");

    assert_eq!(to_string(f, 7.5e-11, Minus,  0, false), "0.000000000075");
    assert_eq!(to_string(f, 7.5e-11, Minus,  3, false), "0.000000000075");
    assert_eq!(to_string(f, 7.5e-11, Minus, 12, false), "0.000000000075");
    assert_eq!(to_string(f, 7.5e-11, Minus, 13, false), "0.0000000000750");

    assert_eq!(to_string(f, 1.9971e20, Minus, 0, false), "199710000000000000000");
    assert_eq!(to_string(f, 1.9971e20, Minus, 1, false), "199710000000000000000.0");
    assert_eq!(to_string(f, 1.9971e20, Minus, 8, false), "199710000000000000000.00000000");

    assert_eq!(to_string(f, f32::MAX, Minus, 0, false), format!("34028235{:0>31}", ""));
    assert_eq!(to_string(f, f32::MAX, Minus, 1, false), format!("34028235{:0>31}.0", ""));
    assert_eq!(to_string(f, f32::MAX, Minus, 8, false), format!("34028235{:0>31}.00000000", ""));

    let minf32: f32 = StdFloat::ldexp(1.0, -149);
    assert_eq!(to_string(f, minf32, Minus,  0, false), format!("0.{:0>44}1", ""));
    assert_eq!(to_string(f, minf32, Minus, 45, false), format!("0.{:0>44}1", ""));
    assert_eq!(to_string(f, minf32, Minus, 46, false), format!("0.{:0>44}10", ""));

    assert_eq!(to_string(f, f64::MAX, Minus, 0, false),
               format!("17976931348623157{:0>292}", ""));
    assert_eq!(to_string(f, f64::MAX, Minus, 1, false),
               format!("17976931348623157{:0>292}.0", ""));
    assert_eq!(to_string(f, f64::MAX, Minus, 8, false),
               format!("17976931348623157{:0>292}.00000000", ""));

    let minf64: f64 = StdFloat::ldexp(1.0, -1074);
    assert_eq!(to_string(f, minf64, Minus,   0, false), format!("0.{:0>323}5", ""));
    assert_eq!(to_string(f, minf64, Minus, 324, false), format!("0.{:0>323}5", ""));
    assert_eq!(to_string(f, minf64, Minus, 325, false), format!("0.{:0>323}50", ""));

    // very large output
    assert_eq!(to_string(f, 1.1, Minus, 80000, false), format!("1.1{:0>79999}", ""));
}

pub fn to_shortest_exp_str_test<F>(mut f_: F)
        where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    use super::Sign::*;

    fn to_string<T, F>(f: &mut F, v: T, sign: Sign, exp_bounds: (i16, i16), upper: bool) -> String
            where T: Float + StdFloat + 'static,
                  F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
        to_string_with_parts(|buf, parts| to_shortest_exp_str(|d,b| f(d,b), v, sign,
                                                              exp_bounds, upper, buf, parts))
    }

    let f = &mut f_;

    assert_eq!(to_string(f,  0.0, Minus,        (-4, 16), false), "0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    (-4, 16), false), "+0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, (-4, 16), false), "+0");
    assert_eq!(to_string(f, -0.0, Minus,        (-4, 16), false), "0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    (-4, 16), false), "+0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, (-4, 16), false), "-0");
    assert_eq!(to_string(f,  0.0, Minus,        ( 0,  0),  true), "0E0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    (-9, -5), false), "+0e0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, ( 5,  9),  true), "+0E0");
    assert_eq!(to_string(f, -0.0, Minus,        ( 0,  0), false), "0e0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    (-9, -5),  true), "+0E0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, ( 5,  9), false), "-0e0");

    assert_eq!(to_string(f,  1.0/0.0, Minus,        (-4, 16), false), "inf");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlus,    (-4, 16),  true), "+INF");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlusRaw, (-4, 16), false), "+inf");
    assert_eq!(to_string(f,  0.0/0.0, Minus,        ( 0,  0),  true), "NAN");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlus,    (-9, -5), false), "nan");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlusRaw, ( 5,  9),  true), "NAN");
    assert_eq!(to_string(f, -1.0/0.0, Minus,        ( 0,  0), false), "-inf");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlus,    (-9, -5),  true), "-INF");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlusRaw, ( 5,  9), false), "-inf");

    assert_eq!(to_string(f,  3.14, Minus,        (-4, 16), false), "3.14");
    assert_eq!(to_string(f,  3.14, MinusPlus,    (-4, 16), false), "+3.14");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, (-4, 16), false), "+3.14");
    assert_eq!(to_string(f, -3.14, Minus,        (-4, 16), false), "-3.14");
    assert_eq!(to_string(f, -3.14, MinusPlus,    (-4, 16), false), "-3.14");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, (-4, 16), false), "-3.14");
    assert_eq!(to_string(f,  3.14, Minus,        ( 0,  0),  true), "3.14E0");
    assert_eq!(to_string(f,  3.14, MinusPlus,    (-9, -5), false), "+3.14e0");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, ( 5,  9),  true), "+3.14E0");
    assert_eq!(to_string(f, -3.14, Minus,        ( 0,  0), false), "-3.14e0");
    assert_eq!(to_string(f, -3.14, MinusPlus,    (-9, -5),  true), "-3.14E0");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, ( 5,  9), false), "-3.14e0");

    assert_eq!(to_string(f,  0.1, Minus,        (-4, 16), false), "0.1");
    assert_eq!(to_string(f,  0.1, MinusPlus,    (-4, 16), false), "+0.1");
    assert_eq!(to_string(f,  0.1, MinusPlusRaw, (-4, 16), false), "+0.1");
    assert_eq!(to_string(f, -0.1, Minus,        (-4, 16), false), "-0.1");
    assert_eq!(to_string(f, -0.1, MinusPlus,    (-4, 16), false), "-0.1");
    assert_eq!(to_string(f, -0.1, MinusPlusRaw, (-4, 16), false), "-0.1");
    assert_eq!(to_string(f,  0.1, Minus,        ( 0,  0),  true), "1E-1");
    assert_eq!(to_string(f,  0.1, MinusPlus,    (-9, -5), false), "+1e-1");
    assert_eq!(to_string(f,  0.1, MinusPlusRaw, ( 5,  9),  true), "+1E-1");
    assert_eq!(to_string(f, -0.1, Minus,        ( 0,  0), false), "-1e-1");
    assert_eq!(to_string(f, -0.1, MinusPlus,    (-9, -5),  true), "-1E-1");
    assert_eq!(to_string(f, -0.1, MinusPlusRaw, ( 5,  9), false), "-1e-1");

    assert_eq!(to_string(f, 7.5e-11, Minus, ( -4, 16), false), "7.5e-11");
    assert_eq!(to_string(f, 7.5e-11, Minus, (-11, 10), false), "0.000000000075");
    assert_eq!(to_string(f, 7.5e-11, Minus, (-10, 11), false), "7.5e-11");

    assert_eq!(to_string(f, 1.9971e20, Minus, ( -4, 16), false), "1.9971e20");
    assert_eq!(to_string(f, 1.9971e20, Minus, (-20, 21), false), "199710000000000000000");
    assert_eq!(to_string(f, 1.9971e20, Minus, (-21, 20), false), "1.9971e20");

    // the true value of 1.0e23f64 is less than 10^23, but that shouldn't matter here
    assert_eq!(to_string(f, 1.0e23, Minus, (22, 23), false), "1e23");
    assert_eq!(to_string(f, 1.0e23, Minus, (23, 24), false), "100000000000000000000000");
    assert_eq!(to_string(f, 1.0e23, Minus, (24, 25), false), "1e23");

    assert_eq!(to_string(f, f32::MAX, Minus, ( -4, 16), false), "3.4028235e38");
    assert_eq!(to_string(f, f32::MAX, Minus, (-39, 38), false), "3.4028235e38");
    assert_eq!(to_string(f, f32::MAX, Minus, (-38, 39), false), format!("34028235{:0>31}", ""));

    let minf32: f32 = StdFloat::ldexp(1.0, -149);
    assert_eq!(to_string(f, minf32, Minus, ( -4, 16), false), "1e-45");
    assert_eq!(to_string(f, minf32, Minus, (-44, 45), false), "1e-45");
    assert_eq!(to_string(f, minf32, Minus, (-45, 44), false), format!("0.{:0>44}1", ""));

    assert_eq!(to_string(f, f64::MAX, Minus, (  -4,  16), false),
               "1.7976931348623157e308");
    assert_eq!(to_string(f, f64::MAX, Minus, (-308, 309), false),
               format!("17976931348623157{:0>292}", ""));
    assert_eq!(to_string(f, f64::MAX, Minus, (-309, 308), false),
               "1.7976931348623157e308");

    let minf64: f64 = StdFloat::ldexp(1.0, -1074);
    assert_eq!(to_string(f, minf64, Minus, (  -4,  16), false), "5e-324");
    assert_eq!(to_string(f, minf64, Minus, (-324, 323), false), format!("0.{:0>323}5", ""));
    assert_eq!(to_string(f, minf64, Minus, (-323, 324), false), "5e-324");

    assert_eq!(to_string(f, 1.1, Minus, (i16::MIN, i16::MAX), false), "1.1");
}

pub fn to_exact_exp_str_test<F>(mut f_: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    use super::Sign::*;

    fn to_string<T, F>(f: &mut F, v: T, sign: Sign, ndigits: usize, upper: bool) -> String
            where T: Float + StdFloat + 'static,
                  F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
        to_string_with_parts(|buf, parts| to_exact_exp_str(|d,b,l| f(d,b,l), v, sign,
                                                           ndigits, upper, buf, parts))
    }

    let f = &mut f_;

    assert_eq!(to_string(f,  0.0, Minus,        1,  true), "0E0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    1, false), "+0e0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 1,  true), "+0E0");
    assert_eq!(to_string(f, -0.0, Minus,        1, false), "0e0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    1,  true), "+0E0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 1, false), "-0e0");
    assert_eq!(to_string(f,  0.0, Minus,        2,  true), "0.0E0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    2, false), "+0.0e0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 2,  true), "+0.0E0");
    assert_eq!(to_string(f, -0.0, Minus,        8, false), "0.0000000e0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    8,  true), "+0.0000000E0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 8, false), "-0.0000000e0");

    assert_eq!(to_string(f,  1.0/0.0, Minus,         1, false), "inf");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlus,     1,  true), "+INF");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlusRaw,  1, false), "+inf");
    assert_eq!(to_string(f,  0.0/0.0, Minus,         8,  true), "NAN");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlus,     8, false), "nan");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlusRaw,  8,  true), "NAN");
    assert_eq!(to_string(f, -1.0/0.0, Minus,        64, false), "-inf");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlus,    64,  true), "-INF");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlusRaw, 64, false), "-inf");

    assert_eq!(to_string(f,  3.14, Minus,        1,  true), "3E0");
    assert_eq!(to_string(f,  3.14, MinusPlus,    1, false), "+3e0");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 1,  true), "+3E0");
    assert_eq!(to_string(f, -3.14, Minus,        2, false), "-3.1e0");
    assert_eq!(to_string(f, -3.14, MinusPlus,    2,  true), "-3.1E0");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 2, false), "-3.1e0");
    assert_eq!(to_string(f,  3.14, Minus,        3,  true), "3.14E0");
    assert_eq!(to_string(f,  3.14, MinusPlus,    3, false), "+3.14e0");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 3,  true), "+3.14E0");
    assert_eq!(to_string(f, -3.14, Minus,        4, false), "-3.140e0");
    assert_eq!(to_string(f, -3.14, MinusPlus,    4,  true), "-3.140E0");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 4, false), "-3.140e0");

    assert_eq!(to_string(f,  0.195, Minus,        1,  true), "2E-1");
    assert_eq!(to_string(f,  0.195, MinusPlus,    1, false), "+2e-1");
    assert_eq!(to_string(f,  0.195, MinusPlusRaw, 1,  true), "+2E-1");
    assert_eq!(to_string(f, -0.195, Minus,        2, false), "-2.0e-1");
    assert_eq!(to_string(f, -0.195, MinusPlus,    2,  true), "-2.0E-1");
    assert_eq!(to_string(f, -0.195, MinusPlusRaw, 2, false), "-2.0e-1");
    assert_eq!(to_string(f,  0.195, Minus,        3,  true), "1.95E-1");
    assert_eq!(to_string(f,  0.195, MinusPlus,    3, false), "+1.95e-1");
    assert_eq!(to_string(f,  0.195, MinusPlusRaw, 3,  true), "+1.95E-1");
    assert_eq!(to_string(f, -0.195, Minus,        4, false), "-1.950e-1");
    assert_eq!(to_string(f, -0.195, MinusPlus,    4,  true), "-1.950E-1");
    assert_eq!(to_string(f, -0.195, MinusPlusRaw, 4, false), "-1.950e-1");

    assert_eq!(to_string(f, 9.5, Minus,  1, false), "1e1");
    assert_eq!(to_string(f, 9.5, Minus,  2, false), "9.5e0");
    assert_eq!(to_string(f, 9.5, Minus,  3, false), "9.50e0");
    assert_eq!(to_string(f, 9.5, Minus, 30, false), "9.50000000000000000000000000000e0");

    assert_eq!(to_string(f, 1.0e25, Minus,  1, false), "1e25");
    assert_eq!(to_string(f, 1.0e25, Minus,  2, false), "1.0e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 15, false), "1.00000000000000e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 16, false), "1.000000000000000e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 17, false), "1.0000000000000001e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 18, false), "1.00000000000000009e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 19, false), "1.000000000000000091e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 20, false), "1.0000000000000000906e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 21, false), "1.00000000000000009060e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 22, false), "1.000000000000000090597e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 23, false), "1.0000000000000000905970e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 24, false), "1.00000000000000009059697e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 25, false), "1.000000000000000090596966e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 26, false), "1.0000000000000000905969664e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 27, false), "1.00000000000000009059696640e25");
    assert_eq!(to_string(f, 1.0e25, Minus, 30, false), "1.00000000000000009059696640000e25");

    assert_eq!(to_string(f, 1.0e-6, Minus,  1, false), "1e-6");
    assert_eq!(to_string(f, 1.0e-6, Minus,  2, false), "1.0e-6");
    assert_eq!(to_string(f, 1.0e-6, Minus, 16, false), "1.000000000000000e-6");
    assert_eq!(to_string(f, 1.0e-6, Minus, 17, false), "9.9999999999999995e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 18, false), "9.99999999999999955e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 19, false), "9.999999999999999547e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 20, false), "9.9999999999999995475e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 30, false), "9.99999999999999954748111825886e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 40, false),
               "9.999999999999999547481118258862586856139e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 50, false),
               "9.9999999999999995474811182588625868561393872369081e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 60, false),
               "9.99999999999999954748111825886258685613938723690807819366455e-7");
    assert_eq!(to_string(f, 1.0e-6, Minus, 70, false),
               "9.999999999999999547481118258862586856139387236908078193664550781250000e-7");

    assert_eq!(to_string(f, f32::MAX, Minus,  1, false), "3e38");
    assert_eq!(to_string(f, f32::MAX, Minus,  2, false), "3.4e38");
    assert_eq!(to_string(f, f32::MAX, Minus,  4, false), "3.403e38");
    assert_eq!(to_string(f, f32::MAX, Minus,  8, false), "3.4028235e38");
    assert_eq!(to_string(f, f32::MAX, Minus, 16, false), "3.402823466385289e38");
    assert_eq!(to_string(f, f32::MAX, Minus, 32, false), "3.4028234663852885981170418348452e38");
    assert_eq!(to_string(f, f32::MAX, Minus, 64, false),
               "3.402823466385288598117041834845169254400000000000000000000000000e38");

    let minf32: f32 = StdFloat::ldexp(1.0, -149);
    assert_eq!(to_string(f, minf32, Minus,   1, false), "1e-45");
    assert_eq!(to_string(f, minf32, Minus,   2, false), "1.4e-45");
    assert_eq!(to_string(f, minf32, Minus,   4, false), "1.401e-45");
    assert_eq!(to_string(f, minf32, Minus,   8, false), "1.4012985e-45");
    assert_eq!(to_string(f, minf32, Minus,  16, false), "1.401298464324817e-45");
    assert_eq!(to_string(f, minf32, Minus,  32, false), "1.4012984643248170709237295832899e-45");
    assert_eq!(to_string(f, minf32, Minus,  64, false),
               "1.401298464324817070923729583289916131280261941876515771757068284e-45");
    assert_eq!(to_string(f, minf32, Minus, 128, false),
               "1.401298464324817070923729583289916131280261941876515771757068283\
                 8897910826858606014866381883621215820312500000000000000000000000e-45");

    assert_eq!(to_string(f, f64::MAX, Minus,   1, false), "2e308");
    assert_eq!(to_string(f, f64::MAX, Minus,   2, false), "1.8e308");
    assert_eq!(to_string(f, f64::MAX, Minus,   4, false), "1.798e308");
    assert_eq!(to_string(f, f64::MAX, Minus,   8, false), "1.7976931e308");
    assert_eq!(to_string(f, f64::MAX, Minus,  16, false), "1.797693134862316e308");
    assert_eq!(to_string(f, f64::MAX, Minus,  32, false), "1.7976931348623157081452742373170e308");
    assert_eq!(to_string(f, f64::MAX, Minus,  64, false),
               "1.797693134862315708145274237317043567980705675258449965989174768e308");
    assert_eq!(to_string(f, f64::MAX, Minus, 128, false),
               "1.797693134862315708145274237317043567980705675258449965989174768\
                 0315726078002853876058955863276687817154045895351438246423432133e308");
    assert_eq!(to_string(f, f64::MAX, Minus, 256, false),
               "1.797693134862315708145274237317043567980705675258449965989174768\
                 0315726078002853876058955863276687817154045895351438246423432132\
                 6889464182768467546703537516986049910576551282076245490090389328\
                 9440758685084551339423045832369032229481658085593321233482747978e308");
    assert_eq!(to_string(f, f64::MAX, Minus, 512, false),
               "1.797693134862315708145274237317043567980705675258449965989174768\
                 0315726078002853876058955863276687817154045895351438246423432132\
                 6889464182768467546703537516986049910576551282076245490090389328\
                 9440758685084551339423045832369032229481658085593321233482747978\
                 2620414472316873817718091929988125040402618412485836800000000000\
                 0000000000000000000000000000000000000000000000000000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000e308");

    // okay, this is becoming tough. fortunately for us, this is almost the worst case.
    let minf64: f64 = StdFloat::ldexp(1.0, -1074);
    assert_eq!(to_string(f, minf64, Minus,    1, false), "5e-324");
    assert_eq!(to_string(f, minf64, Minus,    2, false), "4.9e-324");
    assert_eq!(to_string(f, minf64, Minus,    4, false), "4.941e-324");
    assert_eq!(to_string(f, minf64, Minus,    8, false), "4.9406565e-324");
    assert_eq!(to_string(f, minf64, Minus,   16, false), "4.940656458412465e-324");
    assert_eq!(to_string(f, minf64, Minus,   32, false), "4.9406564584124654417656879286822e-324");
    assert_eq!(to_string(f, minf64, Minus,   64, false),
               "4.940656458412465441765687928682213723650598026143247644255856825e-324");
    assert_eq!(to_string(f, minf64, Minus,  128, false),
               "4.940656458412465441765687928682213723650598026143247644255856825\
                 0067550727020875186529983636163599237979656469544571773092665671e-324");
    assert_eq!(to_string(f, minf64, Minus,  256, false),
               "4.940656458412465441765687928682213723650598026143247644255856825\
                 0067550727020875186529983636163599237979656469544571773092665671\
                 0355939796398774796010781878126300713190311404527845817167848982\
                 1036887186360569987307230500063874091535649843873124733972731696e-324");
    assert_eq!(to_string(f, minf64, Minus,  512, false),
               "4.940656458412465441765687928682213723650598026143247644255856825\
                 0067550727020875186529983636163599237979656469544571773092665671\
                 0355939796398774796010781878126300713190311404527845817167848982\
                 1036887186360569987307230500063874091535649843873124733972731696\
                 1514003171538539807412623856559117102665855668676818703956031062\
                 4931945271591492455329305456544401127480129709999541931989409080\
                 4165633245247571478690147267801593552386115501348035264934720193\
                 7902681071074917033322268447533357208324319360923828934583680601e-324");
    assert_eq!(to_string(f, minf64, Minus, 1024, false),
               "4.940656458412465441765687928682213723650598026143247644255856825\
                 0067550727020875186529983636163599237979656469544571773092665671\
                 0355939796398774796010781878126300713190311404527845817167848982\
                 1036887186360569987307230500063874091535649843873124733972731696\
                 1514003171538539807412623856559117102665855668676818703956031062\
                 4931945271591492455329305456544401127480129709999541931989409080\
                 4165633245247571478690147267801593552386115501348035264934720193\
                 7902681071074917033322268447533357208324319360923828934583680601\
                 0601150616980975307834227731832924790498252473077637592724787465\
                 6084778203734469699533647017972677717585125660551199131504891101\
                 4510378627381672509558373897335989936648099411642057026370902792\
                 4276754456522908753868250641971826553344726562500000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000\
                 0000000000000000000000000000000000000000000000000000000000000000e-324");

    // very large output
    assert_eq!(to_string(f, 0.0,     Minus, 80000, false), format!("0.{:0>79999}e0", ""));
    assert_eq!(to_string(f, 1.0e1,   Minus, 80000, false), format!("1.{:0>79999}e1", ""));
    assert_eq!(to_string(f, 1.0e0,   Minus, 80000, false), format!("1.{:0>79999}e0", ""));
    assert_eq!(to_string(f, 1.0e-1,  Minus, 80000, false),
               format!("1.000000000000000055511151231257827021181583404541015625{:0>79945}\
                        e-1", ""));
    assert_eq!(to_string(f, 1.0e-20, Minus, 80000, false),
               format!("9.999999999999999451532714542095716517295037027873924471077157760\
                         66783064379706047475337982177734375{:0>79901}e-21", ""));
}

pub fn to_exact_fixed_str_test<F>(mut f_: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    use super::Sign::*;

    fn to_string<T, F>(f: &mut F, v: T, sign: Sign, frac_digits: usize, upper: bool) -> String
            where T: Float + StdFloat + 'static,
                  F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
        to_string_with_parts(|buf, parts| to_exact_fixed_str(|d,b,l| f(d,b,l), v, sign,
                                                             frac_digits, upper, buf, parts))
    }

    let f = &mut f_;

    assert_eq!(to_string(f,  0.0, Minus,        0, false), "0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    0, false), "+0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 0, false), "+0");
    assert_eq!(to_string(f, -0.0, Minus,        0, false), "0");
    assert_eq!(to_string(f, -0.0, MinusPlus,    0, false), "+0");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 0, false), "-0");
    assert_eq!(to_string(f,  0.0, Minus,        1,  true), "0.0");
    assert_eq!(to_string(f,  0.0, MinusPlus,    1,  true), "+0.0");
    assert_eq!(to_string(f,  0.0, MinusPlusRaw, 1,  true), "+0.0");
    assert_eq!(to_string(f, -0.0, Minus,        8,  true), "0.00000000");
    assert_eq!(to_string(f, -0.0, MinusPlus,    8,  true), "+0.00000000");
    assert_eq!(to_string(f, -0.0, MinusPlusRaw, 8,  true), "-0.00000000");

    assert_eq!(to_string(f,  1.0/0.0, Minus,         0, false), "inf");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlus,     0,  true), "+INF");
    assert_eq!(to_string(f,  1.0/0.0, MinusPlusRaw,  0, false), "+inf");
    assert_eq!(to_string(f,  0.0/0.0, Minus,         1,  true), "NAN");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlus,     8, false), "nan");
    assert_eq!(to_string(f,  0.0/0.0, MinusPlusRaw, 64,  true), "NAN");
    assert_eq!(to_string(f, -1.0/0.0, Minus,         1, false), "-inf");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlus,     8,  true), "-INF");
    assert_eq!(to_string(f, -1.0/0.0, MinusPlusRaw, 64, false), "-inf");

    assert_eq!(to_string(f,  3.14, Minus,        0, false), "3");
    assert_eq!(to_string(f,  3.14, MinusPlus,    0, false), "+3");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 0, false), "+3");
    assert_eq!(to_string(f, -3.14, Minus,        0, false), "-3");
    assert_eq!(to_string(f, -3.14, MinusPlus,    0, false), "-3");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 0, false), "-3");
    assert_eq!(to_string(f,  3.14, Minus,        1,  true), "3.1");
    assert_eq!(to_string(f,  3.14, MinusPlus,    2,  true), "+3.14");
    assert_eq!(to_string(f,  3.14, MinusPlusRaw, 3,  true), "+3.140");
    assert_eq!(to_string(f, -3.14, Minus,        8,  true), "-3.14000000");
    assert_eq!(to_string(f, -3.14, MinusPlus,    8,  true), "-3.14000000");
    assert_eq!(to_string(f, -3.14, MinusPlusRaw, 8,  true), "-3.14000000");

    assert_eq!(to_string(f,  0.195, Minus,        0, false), "0");
    assert_eq!(to_string(f,  0.195, MinusPlus,    0, false), "+0");
    assert_eq!(to_string(f,  0.195, MinusPlusRaw, 0, false), "+0");
    assert_eq!(to_string(f, -0.195, Minus,        0, false), "-0");
    assert_eq!(to_string(f, -0.195, MinusPlus,    0, false), "-0");
    assert_eq!(to_string(f, -0.195, MinusPlusRaw, 0, false), "-0");
    assert_eq!(to_string(f,  0.195, Minus,        1,  true), "0.2");
    assert_eq!(to_string(f,  0.195, MinusPlus,    2,  true), "+0.20");
    assert_eq!(to_string(f,  0.195, MinusPlusRaw, 3,  true), "+0.195");
    assert_eq!(to_string(f, -0.195, Minus,        4,  true), "-0.1950");
    assert_eq!(to_string(f, -0.195, MinusPlus,    5,  true), "-0.19500");
    assert_eq!(to_string(f, -0.195, MinusPlusRaw, 6,  true), "-0.195000");

    assert_eq!(to_string(f, 999.5, Minus,  0, false), "1000");
    assert_eq!(to_string(f, 999.5, Minus,  1, false), "999.5");
    assert_eq!(to_string(f, 999.5, Minus,  2, false), "999.50");
    assert_eq!(to_string(f, 999.5, Minus,  3, false), "999.500");
    assert_eq!(to_string(f, 999.5, Minus, 30, false), "999.500000000000000000000000000000");

    assert_eq!(to_string(f, 0.95, Minus,  0, false), "1");
    assert_eq!(to_string(f, 0.95, Minus,  1, false), "0.9"); // because it really is less than 0.95
    assert_eq!(to_string(f, 0.95, Minus,  2, false), "0.95");
    assert_eq!(to_string(f, 0.95, Minus,  3, false), "0.950");
    assert_eq!(to_string(f, 0.95, Minus, 10, false), "0.9500000000");
    assert_eq!(to_string(f, 0.95, Minus, 30, false), "0.949999999999999955591079014994");

    assert_eq!(to_string(f, 0.095, Minus,  0, false), "0");
    assert_eq!(to_string(f, 0.095, Minus,  1, false), "0.1");
    assert_eq!(to_string(f, 0.095, Minus,  2, false), "0.10");
    assert_eq!(to_string(f, 0.095, Minus,  3, false), "0.095");
    assert_eq!(to_string(f, 0.095, Minus,  4, false), "0.0950");
    assert_eq!(to_string(f, 0.095, Minus, 10, false), "0.0950000000");
    assert_eq!(to_string(f, 0.095, Minus, 30, false), "0.095000000000000001110223024625");

    assert_eq!(to_string(f, 0.0095, Minus,  0, false), "0");
    assert_eq!(to_string(f, 0.0095, Minus,  1, false), "0.0");
    assert_eq!(to_string(f, 0.0095, Minus,  2, false), "0.01");
    assert_eq!(to_string(f, 0.0095, Minus,  3, false), "0.009"); // really is less than 0.0095
    assert_eq!(to_string(f, 0.0095, Minus,  4, false), "0.0095");
    assert_eq!(to_string(f, 0.0095, Minus,  5, false), "0.00950");
    assert_eq!(to_string(f, 0.0095, Minus, 10, false), "0.0095000000");
    assert_eq!(to_string(f, 0.0095, Minus, 30, false), "0.009499999999999999764077607267");

    assert_eq!(to_string(f, 7.5e-11, Minus,  0, false), "0");
    assert_eq!(to_string(f, 7.5e-11, Minus,  3, false), "0.000");
    assert_eq!(to_string(f, 7.5e-11, Minus, 10, false), "0.0000000001");
    assert_eq!(to_string(f, 7.5e-11, Minus, 11, false), "0.00000000007"); // ditto
    assert_eq!(to_string(f, 7.5e-11, Minus, 12, false), "0.000000000075");
    assert_eq!(to_string(f, 7.5e-11, Minus, 13, false), "0.0000000000750");
    assert_eq!(to_string(f, 7.5e-11, Minus, 20, false), "0.00000000007500000000");
    assert_eq!(to_string(f, 7.5e-11, Minus, 30, false), "0.000000000074999999999999999501");

    assert_eq!(to_string(f, 1.0e25, Minus, 0, false), "10000000000000000905969664");
    assert_eq!(to_string(f, 1.0e25, Minus, 1, false), "10000000000000000905969664.0");
    assert_eq!(to_string(f, 1.0e25, Minus, 3, false), "10000000000000000905969664.000");

    assert_eq!(to_string(f, 1.0e-6, Minus,  0, false), "0");
    assert_eq!(to_string(f, 1.0e-6, Minus,  3, false), "0.000");
    assert_eq!(to_string(f, 1.0e-6, Minus,  6, false), "0.000001");
    assert_eq!(to_string(f, 1.0e-6, Minus,  9, false), "0.000001000");
    assert_eq!(to_string(f, 1.0e-6, Minus, 12, false), "0.000001000000");
    assert_eq!(to_string(f, 1.0e-6, Minus, 22, false), "0.0000010000000000000000");
    assert_eq!(to_string(f, 1.0e-6, Minus, 23, false), "0.00000099999999999999995");
    assert_eq!(to_string(f, 1.0e-6, Minus, 24, false), "0.000000999999999999999955");
    assert_eq!(to_string(f, 1.0e-6, Minus, 25, false), "0.0000009999999999999999547");
    assert_eq!(to_string(f, 1.0e-6, Minus, 35, false), "0.00000099999999999999995474811182589");
    assert_eq!(to_string(f, 1.0e-6, Minus, 45, false),
               "0.000000999999999999999954748111825886258685614");
    assert_eq!(to_string(f, 1.0e-6, Minus, 55, false),
               "0.0000009999999999999999547481118258862586856139387236908");
    assert_eq!(to_string(f, 1.0e-6, Minus, 65, false),
               "0.00000099999999999999995474811182588625868561393872369080781936646");
    assert_eq!(to_string(f, 1.0e-6, Minus, 75, false),
               "0.000000999999999999999954748111825886258685613938723690807819366455078125000");

    assert_eq!(to_string(f, f32::MAX, Minus, 0, false),
               "340282346638528859811704183484516925440");
    assert_eq!(to_string(f, f32::MAX, Minus, 1, false),
               "340282346638528859811704183484516925440.0");
    assert_eq!(to_string(f, f32::MAX, Minus, 2, false),
               "340282346638528859811704183484516925440.00");

    let minf32: f32 = StdFloat::ldexp(1.0, -149);
    assert_eq!(to_string(f, minf32, Minus,   0, false), "0");
    assert_eq!(to_string(f, minf32, Minus,   1, false), "0.0");
    assert_eq!(to_string(f, minf32, Minus,   2, false), "0.00");
    assert_eq!(to_string(f, minf32, Minus,   4, false), "0.0000");
    assert_eq!(to_string(f, minf32, Minus,   8, false), "0.00000000");
    assert_eq!(to_string(f, minf32, Minus,  16, false), "0.0000000000000000");
    assert_eq!(to_string(f, minf32, Minus,  32, false), "0.00000000000000000000000000000000");
    assert_eq!(to_string(f, minf32, Minus,  64, false),
               "0.0000000000000000000000000000000000000000000014012984643248170709");
    assert_eq!(to_string(f, minf32, Minus, 128, false),
               "0.0000000000000000000000000000000000000000000014012984643248170709\
                  2372958328991613128026194187651577175706828388979108268586060149");
    assert_eq!(to_string(f, minf32, Minus, 256, false),
               "0.0000000000000000000000000000000000000000000014012984643248170709\
                  2372958328991613128026194187651577175706828388979108268586060148\
                  6638188362121582031250000000000000000000000000000000000000000000\
                  0000000000000000000000000000000000000000000000000000000000000000");

    assert_eq!(to_string(f, f64::MAX, Minus, 0, false),
               "1797693134862315708145274237317043567980705675258449965989174768\
                0315726078002853876058955863276687817154045895351438246423432132\
                6889464182768467546703537516986049910576551282076245490090389328\
                9440758685084551339423045832369032229481658085593321233482747978\
                26204144723168738177180919299881250404026184124858368");
    assert_eq!(to_string(f, f64::MAX, Minus, 10, false),
               "1797693134862315708145274237317043567980705675258449965989174768\
                0315726078002853876058955863276687817154045895351438246423432132\
                6889464182768467546703537516986049910576551282076245490090389328\
                9440758685084551339423045832369032229481658085593321233482747978\
                26204144723168738177180919299881250404026184124858368.0000000000");

    let minf64: f64 = StdFloat::ldexp(1.0, -1074);
    assert_eq!(to_string(f, minf64, Minus, 0, false), "0");
    assert_eq!(to_string(f, minf64, Minus, 1, false), "0.0");
    assert_eq!(to_string(f, minf64, Minus, 10, false), "0.0000000000");
    assert_eq!(to_string(f, minf64, Minus, 100, false),
               "0.0000000000000000000000000000000000000000000000000000000000000000\
                  000000000000000000000000000000000000");
    assert_eq!(to_string(f, minf64, Minus, 1000, false),
               "0.0000000000000000000000000000000000000000000000000000000000000000\
                  0000000000000000000000000000000000000000000000000000000000000000\
                  0000000000000000000000000000000000000000000000000000000000000000\
                  0000000000000000000000000000000000000000000000000000000000000000\
                  0000000000000000000000000000000000000000000000000000000000000000\
                  0004940656458412465441765687928682213723650598026143247644255856\
                  8250067550727020875186529983636163599237979656469544571773092665\
                  6710355939796398774796010781878126300713190311404527845817167848\
                  9821036887186360569987307230500063874091535649843873124733972731\
                  6961514003171538539807412623856559117102665855668676818703956031\
                  0624931945271591492455329305456544401127480129709999541931989409\
                  0804165633245247571478690147267801593552386115501348035264934720\
                  1937902681071074917033322268447533357208324319360923828934583680\
                  6010601150616980975307834227731832924790498252473077637592724787\
                  4656084778203734469699533647017972677717585125660551199131504891\
                  1014510378627381672509558373897335989937");

    // very large output
    assert_eq!(to_string(f, 0.0,     Minus, 80000, false), format!("0.{:0>80000}", ""));
    assert_eq!(to_string(f, 1.0e1,   Minus, 80000, false), format!("10.{:0>80000}", ""));
    assert_eq!(to_string(f, 1.0e0,   Minus, 80000, false), format!("1.{:0>80000}", ""));
    assert_eq!(to_string(f, 1.0e-1,  Minus, 80000, false),
               format!("0.1000000000000000055511151231257827021181583404541015625{:0>79945}", ""));
    assert_eq!(to_string(f, 1.0e-20, Minus, 80000, false),
               format!("0.0000000000000000000099999999999999994515327145420957165172950370\
                          2787392447107715776066783064379706047475337982177734375{:0>79881}", ""));
}


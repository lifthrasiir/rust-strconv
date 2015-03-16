use std::{str, mem, i16};
use std::num::Float;
use std::slice::bytes;
use rand;
use rand::distributions::{IndependentSample, Range};
use flt2dec::{decode, Decoded, MAX_SIG_DIGITS, round_up};

pub use test::Bencher;

macro_rules! check_shortest {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        let mut buf = [b'_'; MAX_SIG_DIGITS];
        let (len, k) = $fmt(&decode($v), &mut buf);
        assert_eq!((str::from_utf8(&buf[..len]).unwrap(), k),
                   (str::from_utf8($buf).unwrap(), $exp));
    });

    ($fmt:ident{$($k:ident: $v:expr),+} => $buf:expr, $exp:expr) => ({
        let mut buf = [b'_'; MAX_SIG_DIGITS];
        let (len, k) = $fmt(&Decoded { $($k: $v),+ }, &mut buf);
        assert_eq!((str::from_utf8(&buf[..len]).unwrap(), k),
                   (str::from_utf8($buf).unwrap(), $exp));
    })
}

macro_rules! try_exact {
    ($fmt:ident($decoded:expr) => $buf:expr, $expected:expr, $expectedk:expr) => ({
        let (len, k) = $fmt($decoded, &mut $buf[..$expected.len()], i16::MIN);
        assert_eq!((str::from_utf8(&$buf[..len]).unwrap(), k),
                   (str::from_utf8(&$expected).unwrap(), $expectedk));
    })
}

macro_rules! try_fixed {
    ($fmt:ident($decoded:expr) => $buf:expr, $expected:expr, $expectedk:expr) => ({
        let (len, k) = $fmt($decoded, &mut $buf[..], $expectedk - $expected.len() as i16);
        assert_eq!((str::from_utf8(&$buf[..len]).unwrap(), k),
                   (str::from_utf8(&$expected).unwrap(), $expectedk));
    })
}

macro_rules! check_exact {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        let expected = $buf;
        let expectedk = $exp;

        // use a large enough buffer
        let mut buf = [b'_'; 1024];
        let mut expected_ = [b'_'; 1024];

        let decoded = decode($v);
        let cut = expected.iter().position(|&c| c == b' ');

        // check significant digits
        for i in range(1, cut.unwrap_or(expected.len() - 1)) {
            bytes::copy_memory(&mut expected_, &expected[..i]);
            let mut expectedk = expectedk;
            if expected[i] >= b'5' {
                // if this returns true, expected_[..i] is all `9`s and being rounded up.
                // we should always return `100..00` (`i` digits) instead, since that's
                // what we can came up with `i` digits anyway. `round_up` assumes that
                // the adjustment to the length is done by caller, which we simply ignore.
                if round_up(&mut expected_, i) { expectedk += 1; }
            }

            try_exact!($fmt(&decoded) => &mut buf, &expected_[..i], expectedk);
            try_fixed!($fmt(&decoded) => &mut buf, &expected_[..i], expectedk);
        }

        // check infinite zero digits
        if let Some(cut) = cut {
            for i in range(cut, expected.len() - 1) {
                bytes::copy_memory(&mut expected_, &expected[..cut]);
                for c in &mut expected_[cut..i] { *c = b'0'; }

                try_exact!($fmt(&decoded) => &mut buf, &expected_[..i], expectedk);
                try_fixed!($fmt(&decoded) => &mut buf, &expected_[..i], expectedk);
            }
        }
    })
}

macro_rules! check_exact_one {
    ($fmt:ident($x:expr, $e:expr; $t:ty) => $buf:expr, $exp:expr) => ({
        let expected = $buf;
        let expectedk = $exp;

        // use a large enough buffer
        let mut buf = [b'_'; 1024];
        let v: $t = Float::ldexp($x, $e);
        let decoded = decode(v);

        try_exact!($fmt(&decoded) => &mut buf, &expected, expectedk);
        try_fixed!($fmt(&decoded) => &mut buf, &expected, expectedk);
    })
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
    let maxf32: f32 = Float::max_value();
    check_shortest!(f(maxf32) => b"34028235", 39);

    // 10^-37 * 0.1175494210692441075487029444849287348827...
    // 10^-37 * 0.1175494350822287507968736537222245677818...
    // 10^-37 * 0.1175494490952133940450443629595204006810...
    let minnormf32: f32 = Float::min_pos_value(None);
    check_shortest!(f(minnormf32) => b"11754944", -37);

    // 10^-44 * 0
    // 10^-44 * 0.1401298464324817070923729583289916131280...
    // 10^-44 * 0.2802596928649634141847459166579832262560...
    let minf32: f32 = Float::ldexp(1.0, -149);
    check_shortest!(f(minf32) => b"1", -44);
}

pub fn f32_exact_sanity_test<F>(mut f: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    let maxf32: f32 = Float::max_value();
    let minnormf32: f32 = Float::min_pos_value(None);
    let minf32: f32 = Float::ldexp(1.0, -149);

    check_exact!(f(0.1f32)         => b"100000001490116119384765625             ", 0);
    check_exact!(f(1.0f32/3.0)     => b"3333333432674407958984375               ", 0);
    check_exact!(f(3.141592f32)    => b"31415920257568359375                    ", 1);
    check_exact!(f(3.141592e17f32) => b"314159196796878848                      ", 18);
    check_exact!(f(maxf32)         => b"34028234663852885981170418348451692544  ", 39);
    check_exact!(f(minnormf32)     => b"1175494350822287507968736537222245677818", -37);
    check_exact!(f(minf32)         => b"1401298464324817070923729583289916131280", -44);

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
    let maxf64: f64 = Float::max_value();
    check_shortest!(f(maxf64) => b"17976931348623157", 309);

    // 10^-307 * 0.2225073858507200889024586876085859887650...
    // 10^-307 * 0.2225073858507201383090232717332404064219...
    // 10^-307 * 0.2225073858507201877155878558578948240788...
    let minnormf64: f64 = Float::min_pos_value(None);
    check_shortest!(f(minnormf64) => b"22250738585072014", -307);

    // 10^-323 * 0
    // 10^-323 * 0.4940656458412465441765687928682213723650...
    // 10^-323 * 0.9881312916824930883531375857364427447301...
    let minf64: f64 = Float::ldexp(1.0, -1074);
    check_shortest!(f(minf64) => b"5", -323);
}

pub fn f64_exact_sanity_test<F>(mut f: F)
        where F: FnMut(&Decoded, &mut [u8], i16) -> (usize, i16) {
    let maxf64: f64 = Float::max_value();
    let minnormf64: f64 = Float::min_pos_value(None);
    let minf64: f64 = Float::ldexp(1.0, -1074);

    check_exact!(f(0.1f64)         => b"1000000000000000055511151231257827021181", 0);
    check_exact!(f(100.0f64)       => b"1                                       ", 3);
    check_exact!(f(1.0f64/3.0)     => b"3333333333333333148296162562473909929394", 0);
    check_exact!(f(3.141592f64)    => b"3141592000000000162174274009885266423225", 1);
    check_exact!(f(3.141592e17f64) => b"3141592                                 ", 18);
    check_exact!(f(1.0e23f64)      => b"99999999999999991611392                 ", 23);
    check_exact!(f(maxf64)         => b"1797693134862315708145274237317043567980", 309);
    check_exact!(f(minnormf64)     => b"2225073858507201383090232717332404064219", -307);
    check_exact!(f(minf64)         => b"4940656458412465441765687928682213723650\
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
                      exp: 0, sign: 1, inclusive: true} => b"1", 18);
    check_shortest!(f{mant: 99_999_999_999_999_999, minus: 1, plus: 1,
                      exp: 0, sign: 1, inclusive: false} => b"99999999999999999", 17);
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
    let mut rng = rand::thread_rng();
    let f32_range = Range::new(0x0000_0001u32, 0x7f80_0000);
    iterate("f32_random_equivalence_test", k, n, f, g, |_| {
        let i: u32 = f32_range.ind_sample(&mut rng);
        let x: f32 = unsafe {mem::transmute(i)};
        decode(x)
    });
}

pub fn f64_random_equivalence_test<F, G>(f: F, g: G, k: usize, n: usize)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    let mut rng = rand::thread_rng();
    let f64_range = Range::new(0x0000_0000_0000_0001u64, 0x7ff0_0000_0000_0000);
    iterate("f64_random_equivalence_test", k, n, f, g, |_| {
        let i: u64 = f64_range.ind_sample(&mut rng);
        let x: f64 = unsafe {mem::transmute(i)};
        decode(x)
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
        decode(x)
    });
    assert_eq!((npassed, nignored), (2121451879, 17643160));
}


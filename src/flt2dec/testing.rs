use std::{str, mem};
use std::num::Float;
use std::slice::bytes;
use flt2dec::{decode, Decoded, MAX_SIG_DIGITS, round_up};

pub use test::Bencher;

macro_rules! check_shortest {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        let mut buf = [0; MAX_SIG_DIGITS];
        let (len, k) = $fmt(&decode($v), &mut buf);
        assert_eq!((str::from_utf8(&buf[..len]).unwrap(), k),
                   (str::from_utf8($buf).unwrap(), $exp));
    })
}

macro_rules! check_exact {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        let expected = $buf;
        let expectedk = $exp;

        // use a large enough buffer
        let mut buf = [0; 1024];
        let mut expected_ = [0; 1024];

        let decoded = decode($v);
        let cut = expected.iter().position(|&c| c == b' ');

        // check significant digits
        for i in range(1, cut.unwrap_or(expected.len() - 1)) {
            let (len, k) = $fmt(&decoded, buf.slice_to_mut(i));
            assert_eq!(len, i);

            bytes::copy_memory(&mut expected_, expected);
            let mut expectedk = expectedk;
            if expected[i] >= b'5' {
                // if this returns true, expected_[..i] is all `9`s and being rounded up.
                // we should always return `100..00` (`i` digits) instead, since that's
                // what we can came up with `i` digits anyway. `round_up` assumes that
                // the adjustment to the length is done by caller, which we simply ignore.
                if round_up(&mut expected_, i) { expectedk += 1; }
            }

            assert_eq!((str::from_utf8(&buf[..len]).unwrap(), k),
                       (str::from_utf8(&expected_[..len]).unwrap(), expectedk));
        }

        // check infinite zero digits
        if let Some(cut) = cut {
            for i in range(cut, expected.len() - 1) {
                let (len, k) = $fmt(&decoded, buf.slice_to_mut(i));
                assert_eq!(len, cut);
                assert_eq!((str::from_utf8(&buf[..len]).unwrap(), k),
                           (str::from_utf8(&expected[..len]).unwrap(), expectedk));
            }
        }
    })
}

// in the following comments, three numbers are spaced by 1 ulp apart,
// and the second one is being formatted.

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
    let minf32: f32 = 2.0.powf(-149.0);
    check_shortest!(f(minf32) => b"1", -44);
}

pub fn f32_exact_sanity_test<F>(mut f: F) where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    let maxf32: f32 = Float::max_value();
    let minnormf32: f32 = Float::min_pos_value(None);
    let minf32: f32 = 2.0.powf(-149.0);

    check_exact!(f(0.1f32)         => b"100000001490116119384765625             ", 0);
    check_exact!(f(1.0f32/3.0)     => b"3333333432674407958984375               ", 0);
    check_exact!(f(3.141592f32)    => b"31415920257568359375                    ", 1);
    check_exact!(f(3.141592e17f32) => b"314159196796878848                      ", 18);
    check_exact!(f(maxf32)         => b"34028234663852885981170418348451692544  ", 39);
    check_exact!(f(minnormf32)     => b"1175494350822287507968736537222245677818", -37);
    check_exact!(f(minf32)         => b"1401298464324817070923729583289916131280", -44);
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
    let minf64: f64 = 2.0.powf(-1074.0);
    check_shortest!(f(minf64) => b"5", -323);
}

pub fn f64_exact_sanity_test<F>(mut f: F) where F: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    let maxf64: f64 = Float::max_value();
    let minnormf64: f64 = Float::min_pos_value(None);
    let minf64: f64 = 2.0.powf(-1074.0);

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
}

pub fn f32_equivalence_test<F, G>(mut f: F, mut g: G)
        where F: FnMut(&Decoded, &mut [u8]) -> Option<(usize, i16)>,
              G: FnMut(&Decoded, &mut [u8]) -> (usize, i16) {
    // we have only 2^23 * (2^8 - 1) - 1 = 2,139,095,039 positive finite f32 values,
    // so why not simply testing all of them?
    //
    // this is of course very stressful (and thus should be behind an `#[ignore]` attribute),
    // but with `-O3 -C lto` this only takes about two hours or so.

    let mut ntested = 0;
    let mut npassed = 0; // f(x) = Some(g(x))
    let mut nignored = 0; // f(x) = None

    for i in 0x00000001u32..0x7f800000 {
        if (i & 0xfffff) == 0 {
            println!("in progress, {:x}/{:x} (ignored={} passed={} failed={})",
                     i, 0x7f800000u32, nignored, npassed, ntested - nignored - npassed);
        }

        let x: f32 = unsafe {mem::transmute(i)};
        let decoded = decode(x);
        let mut buf1 = [0; MAX_SIG_DIGITS];
        if let Some((len1, e1)) = f(&decoded, &mut buf1) {
            let mut buf2 = [0; MAX_SIG_DIGITS];
            let (len2, e2) = g(&decoded, &mut buf2);
            if e1 == e2 && &buf1[..len1] == &buf2[..len2] {
                npassed += 1;
            } else {
                println!("equivalent test failed, i={:x} f(i)={}e{} g(i)={}e{}",
                         i, str::from_utf8(&buf1[..len1]).unwrap(), e1,
                            str::from_utf8(&buf2[..len2]).unwrap(), e2);
            }
        } else {
            nignored += 1;
        }
        ntested += 1;
    }
    println!("done, ignored={} passed={} failed={}",
             nignored, npassed, ntested - nignored - npassed);
    assert!(nignored + npassed == ntested,
            "{} out of {} f32 values returns an incorrect value!",
            ntested - nignored - npassed, 0x7f7fffffu32);
}


use std::num::Float;
use std::str;
use flt2dec::{Decoded, MAX_SIG_DIGITS};

//use test;

pub use test::Bencher;

macro_rules! check_shortest {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        use flt2dec::decode;

        let mut buf = [0; MAX_SIG_DIGITS];
        let (len, k) = $fmt(&decode($v), &mut buf);
        assert_eq!((str::from_utf8(buf[..len]).unwrap(), k),
                   (str::from_utf8($buf).unwrap(), $exp));
    })
}

macro_rules! check_exact {
    ($fmt:ident($v:expr) => $buf:expr, $exp:expr) => ({
        use flt2dec::{decode, round_up};
        use std::slice::bytes;

        let expected = $buf;
        let expectedk = $exp;

        // use a large enough buffer
        let mut buf = [0; 128];
        let mut expected_ = [0; 128];

        let decoded = decode($v);
        for i in range(1, expected.len() - 1) {
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

            assert_eq!((str::from_utf8(buf[..len]).unwrap(), k),
                       (str::from_utf8(expected_[..len]).unwrap(), expectedk));
        }
    })
}

// in the following comments, three numbers are spaced by 1 ulp apart,
// and the second one is being formatted.

pub fn f32_shortest_sanity_test(f: |&Decoded, &mut [u8]| -> (uint, i16)) {
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

pub fn f32_exact_sanity_test(f: |&Decoded, &mut [u8]| -> (uint, i16)) {
    let maxf32: f32 = Float::max_value();
    let minnormf32: f32 = Float::min_pos_value(None);
    let minf32: f32 = 2.0.powf(-149.0);

    check_exact!(f(0.1f32)         => b"1000000014901161193847656250000000000000", 0);
    check_exact!(f(1.0f32/3.0)     => b"3333333432674407958984375000000000000000", 0);
    check_exact!(f(3.141592f32)    => b"3141592025756835937500000000000000000000", 1);
    check_exact!(f(3.141592e17f32) => b"3141591967968788480000000000000000000000", 18);
    check_exact!(f(maxf32)         => b"3402823466385288598117041834845169254400", 39);
    check_exact!(f(minnormf32)     => b"1175494350822287507968736537222245677818", -37);
    check_exact!(f(minf32)         => b"1401298464324817070923729583289916131280", -44);
}

pub fn f64_shortest_sanity_test(f: |&Decoded, &mut [u8]| -> (uint, i16)) {
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

pub fn f64_exact_sanity_test(f: |&Decoded, &mut [u8]| -> (uint, i16)) {
    let maxf64: f64 = Float::max_value();
    let minnormf64: f64 = Float::min_pos_value(None);
    let minf64: f64 = 2.0.powf(-1074.0);

    check_exact!(f(0.1f64)         => b"1000000000000000055511151231257827021181", 0);
    check_exact!(f(100.0f64)       => b"1000000000000000000000000000000000000000", 3);
    check_exact!(f(1.0f64/3.0)     => b"3333333333333333148296162562473909929394", 0);
    check_exact!(f(3.141592f64)    => b"3141592000000000162174274009885266423225", 1);
    check_exact!(f(3.141592e17f64) => b"3141592000000000000000000000000000000000", 18);
    check_exact!(f(1.0e23f64)      => b"9999999999999999161139200000000000000000", 23);
    check_exact!(f(maxf64)         => b"1797693134862315708145274237317043567980", 309);
    check_exact!(f(minnormf64)     => b"2225073858507201383090232717332404064219", -307);
    check_exact!(f(minf64)         => b"4940656458412465441765687928682213723650", -323);
}


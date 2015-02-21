use std::num::Float;

#[derive(Copy, Debug)]
pub struct Decoded {
    // the scaled mantissa. `original value = mant * 2^exp`.
    pub mant: u64,
    // the lower and upper bounds of ulps.
    // any number between `(mant - minus) * 2^exp` and `(mant + plus) * 2^exp`
    // should have rounded to `mant`. (bounds included only when `inclusive` is true)
    pub minus: u64,
    pub plus: u64,
    // shared exponent in base 2.
    pub exp: i16,
    // sign. either -1 or 1.
    pub sign: i8,
    // are the ulp bounds inclusive?
    pub inclusive: bool, // in IEEE 754, this is true when the original mantissa was even
}

// Float::integer_decode always preserves the exponent, so the mantissa is scaled for subnormals
pub fn decode<T: Float>(v: T) -> Decoded {
    let zero: T = Float::zero();
    let minnorm: T = Float::min_pos_value(None);

    let (mant, exp, sign) = v.integer_decode();
    let (_, zeroexp, _) = zero.integer_decode();
    let (minnormmant, _, _) = minnorm.integer_decode();

    let even = (mant & 1) == 0;

    if exp == zeroexp { // subnormal
        // (mant - 2, exp) -- (mant, exp) -- (mant + 2, exp)
        Decoded { mant: mant, minus: 1, plus: 1, exp: exp, sign: sign, inclusive: even }
    } else if mant == minnormmant {
        // (maxmant, exp - 1) -- (minnormmant, exp) -- (minnormmant + 1, exp)
        // where maxmant = minnormmant * 2 - 1
        Decoded { mant: mant << 1, minus: 1, plus: 2, exp: exp - 1, sign: sign, inclusive: even }
    } else {
        // (mant - 1, exp) -- (mant, exp) -- (mant + 1, exp)
        Decoded { mant: mant << 1, minus: 1, plus: 1, exp: exp - 1, sign: sign, inclusive: even }
    }
}


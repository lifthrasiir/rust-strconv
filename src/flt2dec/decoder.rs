use core::{f32, f64};
use core::num::{Float, FpCategory};
use core::any::TypeId;

#[derive(Copy, Debug, PartialEq)]
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
    // are the ulp bounds inclusive?
    pub inclusive: bool, // in IEEE 754, this is true when the original mantissa was even
}

#[derive(Copy, Debug, PartialEq)]
pub enum FullDecoded {
    Nan,
    Infinite,
    Zero,
    Finite(Decoded),
}

// Float::integer_decode always preserves the exponent, so the mantissa is scaled for subnormals
pub fn decode<T: Float + 'static>(v: T) -> (/*negative?*/ bool, FullDecoded) {
    let (mant, exp, sign) = v.integer_decode();
    let even = (mant & 1) == 0;
    let decoded = match v.classify() {
        FpCategory::Nan => FullDecoded::Nan,
        FpCategory::Infinite => FullDecoded::Infinite,
        FpCategory::Zero => FullDecoded::Zero,
        FpCategory::Subnormal => {
            // (mant - 2, exp) -- (mant, exp) -- (mant + 2, exp)
            FullDecoded::Finite(Decoded { mant: mant, minus: 1, plus: 1,
                                          exp: exp, inclusive: even })
        }
        FpCategory::Normal => {
            // XXX unfortunately `core::num::Float` does not provide a good means
            // to get the minimum normalized value...
            let minnorm = if TypeId::of::<T>() == TypeId::of::<f32>() {
                f32::MIN_POSITIVE.integer_decode()
            } else if TypeId::of::<T>() == TypeId::of::<f64>() {
                f64::MIN_POSITIVE.integer_decode()
            } else {
                unreachable!()
            };

            if mant == minnorm.0 && exp == minnorm.1 {
                // (maxmant, exp - 1) -- (minnormmant, exp) -- (minnormmant + 1, exp)
                // where maxmant = minnormmant * 2 - 1
                FullDecoded::Finite(Decoded { mant: mant << 1, minus: 1, plus: 2,
                                              exp: exp - 1, inclusive: even })
            } else {
                // (mant - 1, exp) -- (mant, exp) -- (mant + 1, exp)
                FullDecoded::Finite(Decoded { mant: mant << 1, minus: 1, plus: 1,
                                              exp: exp - 1, inclusive: even })
            }
        }
    };
    (sign < 0, decoded)
}


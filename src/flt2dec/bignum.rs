use core::mem;
use core::intrinsics;

pub trait FullOps {
    fn full_add(self, other: Self, carry: bool) -> (bool /*carry*/, Self);
    fn full_mul(self, other: Self, carry: Self) -> (Self /*carry*/, Self);
    fn full_mul_add(self, other: Self, other2: Self, carry: Self) -> (Self /*carry*/, Self);
    fn full_div_rem(self, other: Self, borrow: Self) -> (Self /*quotient*/, Self /*remainder*/);
}

macro_rules! impl_full_ops {
    ($($ty:ty: add($addfn:path), mul/div($bigty:ident);)*) => (
        $(
            impl FullOps for $ty {
                // carry' || v' <- self + other + carry
                fn full_add(self, other: $ty, carry: bool) -> (bool, $ty) {
                    // this cannot overflow, the output is between 0 and 2*2^nbits - 1
                    // XXX will LLVM optimize this into ADC or similar???
                    let (v, carry1) = unsafe { $addfn(self, other) };
                    let (v, carry2) = unsafe { $addfn(v, if carry {1} else {0}) };
                    (carry1 || carry2, v)
                }

                // carry' || v' <- self * other + carry
                fn full_mul(self, other: $ty, carry: $ty) -> ($ty, $ty) {
                    // this cannot overflow, the output is between 0 and 2^nbits * (2^nbits - 1)
                    let nbits = mem::size_of::<$ty>() * 8;
                    let v = (self as $bigty) * (other as $bigty) + (carry as $bigty);
                    ((v >> nbits) as $ty, v as $ty)
                }

                // carry' || v' <- self * other + other2 + carry
                fn full_mul_add(self, other: $ty, other2: $ty, carry: $ty) -> ($ty, $ty) {
                    // this cannot overflow, the output is between 0 and 2^(2*nbits) - 1
                    let nbits = mem::size_of::<$ty>() * 8;
                    let v = (self as $bigty) * (other as $bigty) + (other2 as $bigty) +
                            (carry as $bigty);
                    ((v >> nbits) as $ty, v as $ty)
                }

                // (quo, rem) <- (borrow || self) /% other
                fn full_div_rem(self, other: $ty, borrow: $ty) -> ($ty, $ty) {
                    debug_assert!(borrow < other);
                    // this cannot overflow, the dividend is between 0 and other * 2^nbits - 1
                    let nbits = mem::size_of::<$ty>() * 8;
                    let lhs = ((borrow as $bigty) << nbits) | (self as $bigty);
                    let rhs = other as $bigty;
                    ((lhs / rhs) as $ty, (lhs % rhs) as $ty)
                }
            }
        )*
    )
}

impl_full_ops! {
    u8:  add(intrinsics::u8_add_with_overflow),  mul/div(u16);
    u16: add(intrinsics::u16_add_with_overflow), mul/div(u32);
    u32: add(intrinsics::u32_add_with_overflow), mul/div(u64);
//  u64: add(intrinsics::u64_add_with_overflow), mul/div(u128); // damn!
}

macro_rules! define_bignum {
    ($name:ident: type=$ty:ty, n=$n:expr) => (
        #[derive(Copy)]
        pub struct $name {
            size: usize, // base[size..] is known to be zero
            base: [$ty; $n] // [a, b, c, ...] represents a + b*n + c*n^2 + ...
        }

        impl $name {
            pub fn from_small(v: $ty) -> $name {
                let mut base = [0; $n];
                base[0] = v;
                $name { size: 1, base: base }
            }

            pub fn from_u64(mut v: u64) -> $name {
                use std::mem;

                let mut base = [0; $n];
                let mut sz = 0;
                while v > 0 {
                    base[sz] = v as $ty;
                    v >>= mem::size_of::<$ty>() * 8;
                    sz += 1;
                }
                $name { size: sz, base: base }
            }

            pub fn is_zero(&self) -> bool {
                self.base[..self.size].iter().all(|&v| v == 0)
            }

            pub fn add(mut self, other: &$name) -> $name {
                use std::cmp;
                use flt2dec::bignum::FullOps;

                let mut sz = cmp::max(self.size, other.size);
                let mut carry = false;
                for (a, b) in self.base[..sz].iter_mut().zip(other.base[..sz].iter()) {
                    let (c, v) = (*a).full_add(*b, carry);
                    *a = v;
                    carry = c;
                }
                if carry {
                    self.base[sz] = 1;
                    sz += 1;
                }
                self.size = sz;
                self
            }

            pub fn sub(mut self, other: &$name) -> $name {
                use std::cmp;
                use flt2dec::bignum::FullOps;

                let sz = cmp::max(self.size, other.size);
                let mut noborrow = true;
                for (a, b) in self.base[..sz].iter_mut().zip(other.base[..sz].iter()) {
                    let (c, v) = (*a).full_add(!*b, noborrow);
                    *a = v;
                    noborrow = c;
                }
                debug_assert!(noborrow);
                self.size = sz;
                self
            }

            pub fn mul_small(mut self, other: $ty) -> $name {
                use flt2dec::bignum::FullOps;

                let mut sz = self.size;
                let mut carry = 0;
                for a in self.base[..sz].iter_mut() {
                    let (c, v) = (*a).full_mul(other, carry);
                    *a = v;
                    carry = c;
                }
                if carry > 0 {
                    self.base[sz] = carry;
                    sz += 1;
                }
                self.size = sz;
                self
            }

            pub fn mul_pow2(mut self, bits: usize) -> $name {
                use std::mem;

                let digitbits = mem::size_of::<$ty>() * 8;
                let digits = bits / digitbits;
                let bits = bits % digitbits;

                assert!(digits < $n);
                debug_assert!(self.base[$n-digits..].iter().all(|&v| v == 0));
                debug_assert!(bits == 0 || (self.base[$n-digits-1] >> (digitbits - bits)) == 0);

                // shift by `digits * digitbits` bits
                for i in (0..self.size).rev() {
                    self.base[i+digits] = self.base[i];
                }
                for i in 0..digits {
                    self.base[i] = 0;
                }

                // shift by `nbits` bits
                let mut sz = self.size + digits;
                if bits > 0 {
                    let last = sz;
                    let overflow = self.base[last-1] >> (digitbits - bits);
                    if overflow > 0 {
                        self.base[last] = overflow;
                        sz += 1;
                    }
                    for i in (digits+1..last).rev() {
                        self.base[i] = (self.base[i] << bits) |
                                       (self.base[i-1] >> (digitbits - bits));
                    }
                    self.base[digits] <<= bits;
                    // self.base[..digits] is zero, no need to shift
                }

                self.size = sz;
                self
            }

            pub fn mul_digits(&self, other: &[$ty]) -> $name {
                // the internal routine. works best when aa.len() <= bb.len().
                fn mul_inner(aa: &[$ty], bb: &[$ty]) -> ([$ty; $n], usize) {
                    use flt2dec::bignum::FullOps;

                    let mut ret = [0; $n];
                    let mut retsz = 0;
                    for (i, &a) in aa.iter().enumerate() {
                        if a == 0 { continue; }
                        let mut sz = bb.len();
                        let mut carry = 0;
                        for (j, &b) in bb.iter().enumerate() {
                            let (c, v) = a.full_mul_add(b, ret[i + j], carry);
                            ret[i + j] = v;
                            carry = c;
                        }
                        if carry > 0 {
                            ret[i + sz] = carry;
                            sz += 1;
                        }
                        if retsz < i + sz {
                            retsz = i + sz;
                        }
                    }

                    (ret, retsz)
                }

                let (ret, retsz) = if self.size < other.len() {
                    mul_inner(&self.base[..self.size], other)
                } else {
                    mul_inner(other, &self.base[..self.size])
                };
                $name { size: retsz, base: ret }
            }

            pub fn div_rem_small(mut self, other: $ty) -> ($name, $ty) {
                use flt2dec::bignum::FullOps;

                assert!(other > 0);

                let sz = self.size;
                let mut borrow = 0;
                for a in self.base[..sz].iter_mut().rev() {
                    let (q, r) = (*a).full_div_rem(other, borrow);
                    *a = q;
                    borrow = r;
                }
                (self, borrow)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &$name) -> bool { self.base[..] == other.base[..] }
        }

        impl Eq for $name {
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &$name) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> ::std::cmp::Ordering {
                use std::cmp::max;
                use std::iter::order;

                let sz = max(self.size, other.size);
                let lhs = self.base[..sz].iter().cloned().rev();
                let rhs = other.base[..sz].iter().cloned().rev();
                order::cmp(lhs, rhs)
            }
        }

        impl Clone for $name {
            fn clone(&self) -> $name {
                $name { size: self.size, base: self.base }
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                use std::mem;

                let sz = if self.size < 1 {1} else {self.size};
                let digitlen = mem::size_of::<$ty>() * 2;

                try!(write!(f, "{:#x}", self.base[sz-1]));
                for &v in self.base[..sz-1].iter().rev() {
                    try!(write!(f, "_{:01$x}", v, digitlen));
                }
                Ok(())
            }
        }
    )
}

pub type Digit32 = u32;
define_bignum!(Big32x36: type=Digit32, n=36);

#[cfg(test)]
mod tests {
    define_bignum!(Big: type=u8, n=3);

    #[test]
    #[should_panic]
    fn test_from_u64_overflow() {
        Big::from_u64(0x1000000);
    }

    #[test]
    fn test_add() {
        assert_eq!(Big::from_small(3).add(&Big::from_small(4)), Big::from_small(7));
        assert_eq!(Big::from_small(3).add(&Big::from_small(0)), Big::from_small(3));
        assert_eq!(Big::from_small(0).add(&Big::from_small(3)), Big::from_small(3));
        assert_eq!(Big::from_small(3).add(&Big::from_u64(0xfffe)), Big::from_u64(0x10001)); 
        assert_eq!(Big::from_u64(0xfedc).add(&Big::from_u64(0x789)), Big::from_u64(0x10665)); 
        assert_eq!(Big::from_u64(0x789).add(&Big::from_u64(0xfedc)), Big::from_u64(0x10665)); 
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_1() {
        Big::from_small(1).add(&Big::from_u64(0xffffff));
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_2() {
        Big::from_u64(0xffffff).add(&Big::from_small(1));
    }

    #[test]
    fn test_sub() {
        assert_eq!(Big::from_small(7).sub(&Big::from_small(4)), Big::from_small(3));
        assert_eq!(Big::from_u64(0x10665).sub(&Big::from_u64(0x789)), Big::from_u64(0xfedc)); 
        assert_eq!(Big::from_u64(0x10665).sub(&Big::from_u64(0xfedc)), Big::from_u64(0x789)); 
        assert_eq!(Big::from_u64(0x10665).sub(&Big::from_u64(0x10664)), Big::from_small(1)); 
        assert_eq!(Big::from_u64(0x10665).sub(&Big::from_u64(0x10665)), Big::from_small(0)); 
    }

    #[test]
    #[should_panic]
    fn test_sub_underflow_1() {
        Big::from_u64(0x10665).sub(&Big::from_u64(0x10666));
    }

    #[test]
    #[should_panic]
    fn test_sub_underflow_2() {
        Big::from_small(0).sub(&Big::from_u64(0x123456));
    }

    #[test]
    fn test_mul_small() {
        assert_eq!(Big::from_small(7).mul_small(5), Big::from_small(35));
        assert_eq!(Big::from_small(0xff).mul_small(0xff), Big::from_u64(0xfe01));
        assert_eq!(Big::from_u64(0xffffff/13).mul_small(13), Big::from_u64(0xffffff));
    }

    #[test]
    #[should_panic]
    fn test_mul_small_overflow() {
        Big::from_u64(0x800000).mul_small(2);
    }

    #[test]
    fn test_mul_pow2() {
        assert_eq!(Big::from_small(0x7).mul_pow2(4), Big::from_small(0x70));
        assert_eq!(Big::from_small(0xff).mul_pow2(1), Big::from_u64(0x1fe));
        assert_eq!(Big::from_small(0xff).mul_pow2(12), Big::from_u64(0xff000));
        assert_eq!(Big::from_small(0x1).mul_pow2(23), Big::from_u64(0x800000));
        assert_eq!(Big::from_u64(0x123).mul_pow2(0), Big::from_u64(0x123));
        assert_eq!(Big::from_u64(0x123).mul_pow2(7), Big::from_u64(0x9180));
        assert_eq!(Big::from_u64(0x123).mul_pow2(15), Big::from_u64(0x918000));
        assert_eq!(Big::from_small(0).mul_pow2(23), Big::from_small(0));
    }

    #[test]
    #[should_panic]
    fn test_mul_pow2_overflow_1() {
        Big::from_u64(0x1).mul_pow2(24);
    }

    #[test]
    #[should_panic]
    fn test_mul_pow2_overflow_2() {
        Big::from_u64(0x123).mul_pow2(16);
    }

    #[test]
    fn test_mul_digits() {
        assert_eq!(Big::from_small(3).mul_digits(&[5]), Big::from_small(15));
        assert_eq!(Big::from_small(0xff).mul_digits(&[0xff]), Big::from_u64(0xfe01));
        assert_eq!(Big::from_u64(0x123).mul_digits(&[0x56, 0x4]), Big::from_u64(0x4edc2));
        assert_eq!(Big::from_u64(0x12345).mul_digits(&[0x67]), Big::from_u64(0x7530c3));
        assert_eq!(Big::from_small(0x12).mul_digits(&[0x67, 0x45, 0x3]), Big::from_u64(0x3ae13e));
        assert_eq!(Big::from_u64(0xffffff/13).mul_digits(&[13]), Big::from_u64(0xffffff));
        assert_eq!(Big::from_small(13).mul_digits(&[0x3b, 0xb1, 0x13]), Big::from_u64(0xffffff));
    }

    #[test]
    #[should_panic]
    fn test_mul_digits_overflow_1() {
        Big::from_u64(0x800000).mul_digits(&[2]);
    }

    #[test]
    #[should_panic]
    fn test_mul_digits_overflow_2() {
        Big::from_u64(0x1000).mul_digits(&[0, 0x10]);
    }

    #[test]
    fn test_div_rem_small() {
        assert_eq!(Big::from_small(0xff).div_rem_small(15), (Big::from_small(17), 0));
        assert_eq!(Big::from_small(0xff).div_rem_small(16), (Big::from_small(15), 15));
        assert_eq!(Big::from_small(3).div_rem_small(40), (Big::from_small(0), 3));
        assert_eq!(Big::from_u64(0xffffff).div_rem_small(123),
                   (Big::from_u64(0xffffff / 123), (0xffffffu64 % 123) as u8));
        assert_eq!(Big::from_u64(0x10000).div_rem_small(123),
                   (Big::from_u64(0x10000 / 123), (0x10000u64 % 123) as u8));
    }

    #[test]
    fn test_is_zero() {
        assert!(Big::from_small(0).is_zero());
        assert!(!Big::from_small(3).is_zero());
        assert!(!Big::from_u64(0x123).is_zero());
        assert!(!Big::from_u64(0xffffff).sub(&Big::from_u64(0xfffffe)).is_zero());
        assert!(Big::from_u64(0xffffff).sub(&Big::from_u64(0xffffff)).is_zero());
    }

    #[test]
    fn test_ord() {
        assert!(Big::from_u64(0) < Big::from_u64(0xffffff));
        assert!(Big::from_u64(0x102) < Big::from_u64(0x201));
    }

    #[test]
    fn test_fmt() {
        assert_eq!(format!("{:?}", Big::from_u64(0)), "0x0");
        assert_eq!(format!("{:?}", Big::from_u64(0x1)), "0x1");
        assert_eq!(format!("{:?}", Big::from_u64(0x12)), "0x12");
        assert_eq!(format!("{:?}", Big::from_u64(0x123)), "0x1_23");
        assert_eq!(format!("{:?}", Big::from_u64(0x1234)), "0x12_34");
        assert_eq!(format!("{:?}", Big::from_u64(0x12345)), "0x1_23_45");
        assert_eq!(format!("{:?}", Big::from_u64(0x123456)), "0x12_34_56");
    }
}


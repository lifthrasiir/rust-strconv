#![macro_use]

use core::prelude::*;
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
                use core::mem;

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
                use core::cmp;
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
                use core::cmp;
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
                use core::mem;

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

        impl ::core::cmp::PartialEq for $name {
            fn eq(&self, other: &$name) -> bool { self.base[..] == other.base[..] }
        }

        impl ::core::cmp::Eq for $name {
        }

        impl ::core::cmp::PartialOrd for $name {
            fn partial_cmp(&self, other: &$name) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::option::Option::Some(self.cmp(other))
            }
        }

        impl ::core::cmp::Ord for $name {
            fn cmp(&self, other: &$name) -> ::core::cmp::Ordering {
                use core::cmp::max;
                use core::iter::order;

                let sz = max(self.size, other.size);
                let lhs = self.base[..sz].iter().cloned().rev();
                let rhs = other.base[..sz].iter().cloned().rev();
                order::cmp(lhs, rhs)
            }
        }

        impl ::core::clone::Clone for $name {
            fn clone(&self) -> $name {
                $name { size: self.size, base: self.base }
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                use core::mem;

                let sz = if self.size < 1 {1} else {self.size};
                let digitlen = mem::size_of::<$ty>() * 2;

                try!(write!(f, "{:#x}", self.base[sz-1]));
                for &v in self.base[..sz-1].iter().rev() {
                    try!(write!(f, "_{:01$x}", v, digitlen));
                }
                ::core::result::Result::Ok(())
            }
        }
    )
}

pub type Digit32 = u32;
define_bignum!(Big32x36: type=Digit32, n=36);


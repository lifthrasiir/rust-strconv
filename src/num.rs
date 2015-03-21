use core::num::Int;

pub fn div_rem<T: Int>(x: T, y: T) -> (T, T) {
    (x / y, x % y)
}

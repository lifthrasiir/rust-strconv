use core::marker::Copy;
use core::ops::{Div, Rem};

pub fn div_rem<T: Copy + Div<T,Output=T> + Rem<T,Output=T>>(x: T, y: T) -> (T, T) {
    (x / y, x % y)
}

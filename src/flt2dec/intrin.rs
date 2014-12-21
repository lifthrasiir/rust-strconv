// safe intrinsics

use std::intrinsics;

pub fn u8_add_with_overflow(x: u8, y: u8) -> (u8, bool) {
    unsafe {intrinsics::u8_add_with_overflow(x, y)}
}

pub fn u16_add_with_overflow(x: u16, y: u16) -> (u16, bool) {
    unsafe {intrinsics::u16_add_with_overflow(x, y)}
}

pub fn u32_add_with_overflow(x: u32, y: u32) -> (u32, bool) {
    unsafe {intrinsics::u32_add_with_overflow(x, y)}
}

pub fn ctlz64(x: u64) -> u64 {
    unsafe {intrinsics::ctlz64(x)}
}


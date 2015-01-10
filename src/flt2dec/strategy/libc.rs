#![cfg(test)]

extern crate libc;

use std::num::Float;
use test;

extern {
    fn snprintf(buf: *mut libc::c_char, len: libc::size_t,
                fmt: *const libc::c_char, ...) -> libc::c_int;
}

fn f64_to_buf(buf: &mut [u8], fmt: &str, v: f64) -> usize {
    unsafe {
        snprintf(buf.as_mut_ptr() as *mut _, buf.len() as libc::size_t,
                 fmt.as_ptr() as *const _, v) as usize
    }
}

#[bench]
fn bench_small_exact_3(b: &mut test::Bencher) {
    let mut buf = [0; 32];
    b.iter(|| f64_to_buf(&mut buf, "%.2e\0", 3.141592f64))
}

#[bench]
fn bench_big_exact_3(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    let mut buf = [0; 32];
    b.iter(|| f64_to_buf(&mut buf, "%.2e\0", v))
}


#![cfg(test)]

use std::num::Float;
use test;

#[bench]
fn bench_small_exact_3(b: &mut test::Bencher) {
    b.iter(|| format!("{:.2e}", 3.141592f64));
}

#[bench]
fn bench_big_exact_3(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| format!("{:.2e}", v));
}

#[bench]
fn bench_small_exact_inf(b: &mut test::Bencher) {
    b.iter(|| 3.141592f64.to_string());
}

#[bench]
fn bench_big_exact_inf(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| v.to_string());
}


#![cfg(test)]

use std::num::Float;
use test;

#[bench]
fn bench_small_external(b: &mut test::Bencher) {
    b.iter(|| 3.141592f64.to_string());
}

#[bench]
fn bench_big_external(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| v.to_string());
}


#![cfg(test)]

use std::prelude::v1::*;
use std::f64;
use test;

#[bench]
fn bench_small_exact_3(b: &mut test::Bencher) {
    b.iter(|| format!("{:.2e}", 3.141592f64));
}

#[bench]
fn bench_big_exact_3(b: &mut test::Bencher) {
    b.iter(|| format!("{:.2e}", f64::MAX));
}

#[bench]
fn bench_small_exact_inf(b: &mut test::Bencher) {
    b.iter(|| 3.141592f64.to_string());
}

#[bench]
fn bench_big_exact_inf(b: &mut test::Bencher) {
    b.iter(|| f64::MAX.to_string());
}


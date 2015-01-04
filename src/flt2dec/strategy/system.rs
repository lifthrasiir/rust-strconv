#[cfg(test)] use std::num::Float;
#[cfg(test)] use test;

#[cfg(test)] #[bench]
fn bench_small_exact(b: &mut test::Bencher) {
    b.iter(|| 3.141592f64.to_string());
}

#[cfg(test)] #[bench]
fn bench_big_exact(b: &mut test::Bencher) {
    let v: f64 = Float::max_value();
    b.iter(|| v.to_string());
}


#[cfg(test)] use int2dec::testing;

#[cfg(target_arch = "x86")] pub use super::bcd_earlyexit::u64_to_digits;
#[cfg(target_arch = "x86")] pub use super::div100_earlyexit::u32_to_digits;
#[cfg(target_arch = "x86")] pub use super::div100::u16_to_digits;
#[cfg(target_arch = "x86")] pub use super::div100_earlyexit::u8_to_digits;

#[cfg(not(target_arch = "x86"))] pub use super::div100_u32_earlyexit::u64_to_digits;
#[cfg(not(target_arch = "x86"))] pub use super::div100::u32_to_digits;
#[cfg(not(target_arch = "x86"))] pub use super::div100::u16_to_digits;
#[cfg(not(target_arch = "x86"))] pub use super::naive::u8_to_digits;

#[cfg(test)] #[bench]
fn bench_u64(b: &mut testing::Bencher) {
    testing::rotating_bench(u64_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u32(b: &mut testing::Bencher) {
    testing::rotating_bench(u32_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u16(b: &mut testing::Bencher) {
    testing::rotating_bench(u16_to_digits, b);
}

#[cfg(test)] #[bench]
fn bench_u8(b: &mut testing::Bencher) {
    testing::rotating_bench(u8_to_digits, b);
}


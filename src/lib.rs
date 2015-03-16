#![feature(core, std_misc)] // lib stability features as per RFC #507
#![cfg_attr(test, feature(libc, test))] // ditto

#[cfg(test)] extern crate test;
#[cfg(test)] extern crate rand;

mod num;

pub mod int2dec;
pub mod flt2dec;


/* Rust-strconv -- Experimental string-to-number and number-to-string
 * conversion libraries and benchmarks for Rust.
 * Written by Kang Seonghoon <http://mearie.org/>.
 *
 * The author disclaims copyright to this source code.  In place of
 * a legal notice, here is a blessing:
 *
 *    May you do good and not evil.
 *    May you find forgiveness for yourself and forgive others.
 *    May you share freely, never taking more than you give.
 *
 * See LICENSE.txt for the exact and precise legal mumbo-jumbo.
 * This legal notice and blessing is shamelessly adopted from
 * the SQLite library.
 */

#![feature(no_std, core)] // lib stability features as per RFC #507
#![cfg_attr(test, feature(std_misc, libc, zero_one, test))] // ditto
#![no_std]

#[macro_use] extern crate core;

// tests only
#[cfg(test)] #[macro_use] extern crate std;
#[cfg(test)] extern crate test;
#[cfg(test)] extern crate rand;
#[cfg(test)] extern crate libc;

mod num;

pub mod int2dec;
pub mod flt2dec;


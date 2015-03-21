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
 * Alternatively, for jurisdictions where authors cannot disclaim
 * their copyright, this source code is distributed under the terms of
 * CC0 1.0 Universal license as published by Creative Commons
 * <https://creativecommons.org/publicdomain/zero/1.0/>.
 *
 * This legal notice and blessing is shamelessly adopted from
 * the SQLite library.
 */

#![feature(core, std_misc)] // lib stability features as per RFC #507
#![cfg_attr(test, feature(libc, test))] // ditto

#[cfg(test)] extern crate test;
#[cfg(test)] extern crate rand;

mod num;

pub mod int2dec;
pub mod flt2dec;


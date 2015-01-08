pub use self::decoder::{decode, Decoded};

mod intrin;
#[macro_use] mod bignum;
mod decoder;

#[cfg(test)] mod testing;
pub mod strategy {
    pub mod system;
    pub mod libc;
    pub mod dragon;
    pub mod grisu;
}

// it is a bit non-trivial to derive, but this is one plus the maximal number of
// significant decimal digits from formatting algorithms with the shortest result.
// the exact formula for this is: ceil(# bits in mantissa * log_10 2 + 1).
pub const MAX_SIG_DIGITS: uint = 17;

// when d[..n] contains decimal digits, increase the last digit and propagate carry.
// returns true when it causes the length change.
fn round_up(d: &mut [u8], n: uint) -> bool {
    match d[..n].iter().rposition(|&c| c != b'9') {
        Some(i) => { // d[i+1..n] is all nines
            d[i] += 1;
            for j in range(i+1, n) { d[j] = b'0'; }
            false
        }
        None => { // 999..999 rounds to 1000..000 with an increased exponent
            d[0] = b'1';
            for j in range(1, n+1) { d[j] = b'0'; }
            true
        }
    }
}


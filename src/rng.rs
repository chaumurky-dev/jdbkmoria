// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Random number generator

// This alg uses a prime modulus multiplicative congruential generator
//
// (PMMLCG), also known as a Lehmer Grammer, which satisfies the following
// properties
//
//   (i)   modulus: m - a large prime integer
//   (ii)  multiplier: a - an integer in the range 2, 3, ..., m - 1
//   (iii) z[n+1] = f(z[n]), for n = 1, 2, ...
//   (iv)  f(z) = az mod m
//   (v)   u[n] = z[n] / m, for n = 1, 2, ...
//
// The sequence of z's must be initialized by choosing an initial seed z[1]
// from the range 1, 2, ..., m - 1.  The sequence of z's is a pseudo-random
// sequence drawn without replacement from the set 1, 2, ..., m - 1.
// The u's form a pseudo-random sequence of real numbers between (but not
// including) 0 and 1.
//
// Schrage's method is used to compute the sequence of z's.
//
// a good random number generator, correct on any machine with 32 bit
// integers, this algorithm is from:
//
// Stephen K. Park and Keith W. Miller, "Random Number Generators:
//       Good ones are hard to find", Communications of the ACM, October 1988,
//       vol 31, number 10, pp. 1192-1201.
//
//  If this algorithm is implemented correctly,
//  then if z[1] = 1,
//  then z[10001] will equal 1043618065
//
//  Has a full period of 2^31 - 1.
//  Returns integers in the range 1 to 2^31-1.

use crate::globals::RacyCell;

const RNG_M: i32 = i32::MAX; // m = 2^31 - 1
const RNG_A: i32 = 16807;
const RNG_Q: i32 = RNG_M / RNG_A; // m div a 127773
const RNG_R: i32 = RNG_M % RNG_A; // m mod a 2836

// 32 bit seed
static RND_SEED: RacyCell<u32> = RacyCell::new(0);

pub fn get_random_seed() -> u32 {
    *RND_SEED.get()
}

pub fn set_random_seed(seed: u32) {
    // set seed to value between 1 and m-1
    *RND_SEED.get() = (seed % (RNG_M as u32 - 1)) + 1;
}

// returns a pseudo-random number from set 1, 2, ..., RNG_M - 1
pub fn rnd() -> i32 {
    let seed = RND_SEED.get();

    let high = (*seed / RNG_Q as u32) as i32;
    let low = (*seed % RNG_Q as u32) as i32;
    let test = RNG_A.wrapping_mul(low).wrapping_sub(RNG_R.wrapping_mul(high));

    if test > 0 {
        *seed = test as u32;
    } else {
        *seed = (test + RNG_M) as u32;
    }

    *seed as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_miller_reference_value() {
        set_random_seed(0);
        for _ in 1..10000 {
            rnd();
        }
        assert_eq!(rnd(), 1043618065);
    }
}

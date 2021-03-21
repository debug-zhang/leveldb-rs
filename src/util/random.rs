// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use std::cell::Cell;

/// A very simple random number generator.
/// Not especially good at generating truly random bits,
/// but good enough for our needs in this package.
pub struct Random {
    seed: Cell<u32>,
}

impl Random {
    /// Return a random number generator.
    pub fn new(s: u32) -> Self {
        let mut seed = s & 0x7fffffffu32;
        // Avoid bad seeds.
        if seed == 0 || seed == 2147483647 {
            seed = 1;
        }
        Self {
            seed: Cell::new(seed),
        }
    }

    /// Return the next random number in this generator.
    pub fn next(&self) -> u32 {
        // M = 2^31-1
        const M: u32 = 2147483647;
        // A = 0b0100_0001_1010_0111
        const A: u64 = 16807;

        // We are computing
        //       seed = (seed * A) % M,    where M = 2^31-1
        //
        // seed must not be zero or M, or else all subsequent computed values
        // will be zero or M respectively. For all other values, seed will end
        // up cycling through every number in [1,M-1].
        let product: u64 = (self.seed.get() as u64) * A;

        // To avoid the 64-bit division, compute (product % M) using the fact:
        //       ((x << 31) % M) == x.
        self.seed.set(((product >> 31) as u32) + ((product as u32) & M));

        // The first reduction may overflow by 1 bit, so we may need to repeat.
        // mod == M is not possible; using > allows the faster sign-bit-based test.
        if self.seed.get() > M {
            self.seed.set(self.seed.get() - M);
        }

        self.seed.get()
    }

    /// Returns a uniformly distributed value in the range [0..n-1]
    /// REQUIRES: n > 0
    #[inline]
    pub fn uniform(&self, n: u32) -> u32 {
        self.next() % n
    }

    /// Randomly returns true ~"1/n" of the time, and false otherwise.
    /// REQUIRES: n > 0
    #[inline]
    pub fn one_in(&self, n: u32) -> bool {
        self.next() % n == 0
    }

    /// Skewed: pick "base" uniformly from range [0,max_log] and then
    /// return "base" random bits. The effect is to pick a number in the
    /// range [0,2^max_log-1] with exponential bias towards smaller numbers.
    #[inline]
    pub fn skewed(&self, max_log: u32) -> u32 {
        self.uniform(1 << self.uniform(max_log + 1))
    }
}

#[cfg(test)]
mod tests {
    use crate::util::random::Random;

    #[test]
    fn random() {
        let mut r = Random::new(0);
        assert_eq!(r.seed.get(), 1);

        r = Random::new(2147483647);
        assert_eq!(r.seed.get(), 1);


        r = Random::new(7);
        assert_eq!(r.next(), 117649);
        assert_eq!(r.uniform(11), 7);
        assert_eq!(r.one_in(5), false);
        assert_eq!(r.skewed(3), 1);
    }
}
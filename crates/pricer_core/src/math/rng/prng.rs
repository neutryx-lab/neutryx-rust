//! Pseudo-random number generator wrapper for Monte Carlo simulations.
//!
//! This module provides [`PricerRng`], a seeded PRNG wrapper that offers
//! reproducible random number generation with efficient batch operations.

use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::{Distribution, StandardNormal};

/// Seeded PRNG for reproducible Monte Carlo simulations.
///
/// Uses static dispatch only (no `Box<dyn Trait>`) for Enzyme AD compatibility.
pub struct PricerRng {
    /// The underlying PRNG instance.
    inner: StdRng,
    /// The seed used for initialisation (stored for reproducibility tracking).
    seed: u64,
}

impl PricerRng {
    /// Creates a new RNG instance initialised with the given seed.
    #[inline]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    /// Returns the seed used for initialisation.
    #[inline]
    pub fn seed(&self) -> u64 { self.seed }

    /// Generates a single uniform random value in [0, 1).
    #[inline]
    pub fn gen_uniform(&mut self) -> f64 { self.inner.gen() }

    /// Generates a single standard normal variate (mean=0, std=1).
    #[inline]
    pub fn gen_normal(&mut self) -> f64 { StandardNormal.sample(&mut self.inner) }

    /// Fills the buffer with uniform random values in [0, 1).
    #[inline]
    pub fn fill_uniform(&mut self, buffer: &mut [f64]) {
        for value in buffer.iter_mut() {
            *value = self.inner.gen();
        }
    }

    /// Fills the buffer with standard normal (mean=0, std=1) variates.
    #[inline]
    pub fn fill_normal(&mut self, buffer: &mut [f64]) {
        for value in buffer.iter_mut() {
            *value = StandardNormal.sample(&mut self.inner);
        }
    }
}

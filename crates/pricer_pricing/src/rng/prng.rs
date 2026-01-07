//! Pseudo-random number generator wrapper for Monte Carlo simulations.
//!
//! This module provides [`PricerRng`], a seeded PRNG wrapper that offers
//! reproducible random number generation with efficient batch operations.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, StandardNormal};

/// Monte Carlo simulation random number generator.
///
/// Provides seeded, reproducible random number generation with efficient
/// batch operations for uniform and normal distributions.
///
/// # Enzyme Compatibility
///
/// This type uses static dispatch exclusively (no `Box<dyn Trait>`) to
/// ensure compatibility with Enzyme's LLVM-level automatic differentiation.
///
/// # British English Note
///
/// All documentation uses British English spelling conventions
/// (e.g., "initialise", "randomise", "behaviour").
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::rng::PricerRng;
///
/// let mut rng = PricerRng::from_seed(42);
///
/// // Single value generation
/// let u: f64 = rng.gen_uniform();
/// let n: f64 = rng.gen_normal();
///
/// // Batch generation (zero allocation)
/// let mut buffer = vec![0.0; 100];
/// rng.fill_uniform(&mut buffer);
/// rng.fill_normal(&mut buffer);
/// ```
pub struct PricerRng {
    /// The underlying PRNG instance.
    inner: StdRng,
    /// The seed used for initialisation (stored for reproducibility tracking).
    seed: u64,
}

impl PricerRng {
    /// Creates a new RNG instance initialised with the given seed.
    ///
    /// The same seed will always produce the same sequence of random numbers,
    /// enabling reproducible Monte Carlo simulations.
    ///
    /// # Arguments
    ///
    /// * `seed` - 64-bit seed value for reproducibility
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let mut rng1 = PricerRng::from_seed(12345);
    /// let mut rng2 = PricerRng::from_seed(12345);
    ///
    /// // Same seed produces identical sequences
    /// assert_eq!(rng1.gen_uniform(), rng2.gen_uniform());
    /// ```
    #[inline]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    /// Returns the seed used for initialisation.
    ///
    /// This is useful for logging and debugging reproducibility issues.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let rng = PricerRng::from_seed(42);
    /// assert_eq!(rng.seed(), 42);
    /// ```
    #[inline]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Generates a single uniform random value in [0, 1).
    ///
    /// # Returns
    ///
    /// A uniformly distributed `f64` in the half-open interval [0, 1).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let mut rng = PricerRng::from_seed(42);
    /// let value = rng.gen_uniform();
    /// assert!(value >= 0.0 && value < 1.0);
    /// ```
    #[inline]
    pub fn gen_uniform(&mut self) -> f64 {
        self.inner.gen()
    }

    /// Generates a single standard normal variate (mean=0, std=1).
    ///
    /// Uses the ZIGNOR Ziggurat algorithm via `rand_distr::StandardNormal`
    /// for high-performance sampling.
    ///
    /// # Algorithm Reference
    ///
    /// The Ziggurat method is described in:
    /// - Marsaglia, G. & Tsang, W. W. (2000). "The Ziggurat Method for
    ///   Generating Random Variables". Journal of Statistical Software.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let mut rng = PricerRng::from_seed(42);
    /// let value = rng.gen_normal();
    /// // Value is from standard normal distribution
    /// ```
    #[inline]
    pub fn gen_normal(&mut self) -> f64 {
        StandardNormal.sample(&mut self.inner)
    }

    /// Fills the buffer with uniform random values in [0, 1).
    ///
    /// This is a zero-allocation operation; the buffer must be pre-allocated
    /// by the caller. Empty buffers are handled gracefully (no operation).
    ///
    /// # Arguments
    ///
    /// * `buffer` - Mutable slice to fill with uniform variates
    ///
    /// # Performance
    ///
    /// - Zero heap allocations
    /// - Achieves >10M samples/second on standard hardware
    /// - Recommended buffer size: 64KB-1MB for optimal cache utilisation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let mut rng = PricerRng::from_seed(42);
    /// let mut buffer = vec![0.0; 1000];
    /// rng.fill_uniform(&mut buffer);
    ///
    /// for &value in &buffer {
    ///     assert!(value >= 0.0 && value < 1.0);
    /// }
    /// ```
    #[inline]
    pub fn fill_uniform(&mut self, buffer: &mut [f64]) {
        for value in buffer.iter_mut() {
            *value = self.inner.gen();
        }
    }

    /// Fills the buffer with standard normal (mean=0, std=1) variates.
    ///
    /// Uses the ZIGNOR Ziggurat algorithm via `rand_distr::StandardNormal`
    /// for high-performance sampling.
    ///
    /// This is a zero-allocation operation; the buffer must be pre-allocated
    /// by the caller. Empty buffers are handled gracefully (no operation).
    ///
    /// # Arguments
    ///
    /// * `buffer` - Mutable slice to fill with normal variates
    ///
    /// # Performance
    ///
    /// - Zero heap allocations
    /// - Achieves >1M samples/second on standard hardware
    /// - Uses fixed-size loop for Enzyme compatibility
    /// - Recommended buffer size: 64KB-1MB for optimal cache utilisation
    ///
    /// # Algorithm Reference
    ///
    /// The Ziggurat method is described in:
    /// - Marsaglia, G. & Tsang, W. W. (2000). "The Ziggurat Method for
    ///   Generating Random Variables". Journal of Statistical Software.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::rng::PricerRng;
    ///
    /// let mut rng = PricerRng::from_seed(42);
    /// let mut buffer = vec![0.0; 1000];
    /// rng.fill_normal(&mut buffer);
    /// // Buffer now contains standard normal variates
    /// ```
    #[inline]
    pub fn fill_normal(&mut self, buffer: &mut [f64]) {
        for value in buffer.iter_mut() {
            *value = StandardNormal.sample(&mut self.inner);
        }
    }
}

//! # Random Number Generation Infrastructure
//!
//! This module provides random number generation facilities for Monte Carlo
//! simulations in the pricer kernel. It includes pseudo-random number generators
//! (PRNGs) and placeholders for quasi-Monte Carlo (QMC) sequences.
//!
//! ## Design Rationale
//!
//! The RNG infrastructure is designed with the following principles:
//!
//! - **Reproducibility**: All generators support seeding for deterministic sequences
//! - **Efficiency**: Zero-allocation batch operations via `&mut [f64]` slices
//! - **Enzyme Compatibility**: Static dispatch only; no `Box<dyn Trait>` in hot paths
//! - **Independence**: No dependencies on `pricer_core` (Phase 3.0 isolation)
//!
//! ## British English Convention
//!
//! All documentation in this module uses British English spelling conventions:
//! - "initialise" (not "initialize")
//! - "randomise" (not "randomize")
//! - "behaviour" (not "behavior")
//! - "optimisation" (not "optimization")
//!
//! ## Module Structure
//!
//! - [`prng`]: Pseudo-random number generator wrapper with seed management
//! - [`qmc`]: Quasi-Monte Carlo sequence traits and placeholders
//!
//! ## Usage Example
//!
//! ```rust
//! use pricer_pricing::rng::PricerRng;
//!
//! // Create a seeded RNG for reproducible simulations
//! let mut rng = PricerRng::from_seed(12345);
//!
//! // Generate uniform random values in [0, 1)
//! let uniform_value = rng.gen_uniform();
//!
//! // Generate standard normal variates (mean=0, std=1)
//! let normal_value = rng.gen_normal();
//!
//! // Batch generation into pre-allocated buffer (zero allocation)
//! let mut buffer = vec![0.0; 1000];
//! rng.fill_normal(&mut buffer);
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Uniform generation**: >10M samples/second
//! - **Normal generation**: >1M samples/second (Ziggurat algorithm)
//! - **Recommended buffer size**: 64KB-1MB for optimal cache utilisation
//!
//! ## Phase 3.1a Scope
//!
//! This phase implements:
//! - PRNG wrapper around `rand::StdRng`
//! - Normal distribution via Ziggurat algorithm (`rand_distr::StandardNormal`)
//! - QMC trait definitions (placeholders only)
//!
//! Sobol sequence implementation is deferred to a future phase.

mod prng;
mod qmc;

// Public re-exports
pub use prng::PricerRng;
pub use qmc::{LowDiscrepancySequence, SobolPlaceholder};

#[cfg(test)]
mod tests;

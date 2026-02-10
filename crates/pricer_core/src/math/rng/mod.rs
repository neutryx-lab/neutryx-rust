//! Random number generation for Monte Carlo simulations (PRNG and QMC).

mod prng;
mod qmc;

// Public re-exports
pub use prng::PricerRng;
pub use qmc::{LowDiscrepancySequence, SobolPlaceholder};

#[cfg(test)]
mod tests;

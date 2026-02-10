//! Quasi-Monte Carlo sequence traits and placeholders.
//!
//! This module defines the interface for low-discrepancy sequences used in
//! quasi-Monte Carlo (QMC) methods. Currently, only placeholder implementations
//! are provided; full Sobol sequence support is planned for a future phase.
//!
//! ## Future Integration
//!
//! The following implementations are planned:
//! - Sobol sequences via the `sobol` crate (21,201 dimensions supported)
//! - Halton sequences for lower-dimensional problems
//! - Joe-Kuo D6 direction numbers for improved uniformity

/// Trait for low-discrepancy sequences used in quasi-Monte Carlo methods.
pub trait LowDiscrepancySequence {
    /// Returns the dimensionality of the sequence.
    fn dimension(&self) -> usize;

    /// Advances the sequence and returns the next point.
    fn next_point(&mut self) -> &[f64];

    /// Resets the sequence to its initial state.
    fn reset(&mut self);

    /// Skips ahead by `n` points in the sequence.
    fn skip(&mut self, n: usize);
}

/// Placeholder for Sobol sequence generator (not yet implemented).
pub struct SobolPlaceholder {
    /// The dimensionality of the sequence (stored but not used).
    #[allow(dead_code)]
    dimension: usize,
    /// Placeholder for sequence state.
    #[allow(dead_code)]
    buffer: Vec<f64>,
}

impl SobolPlaceholder {
    /// Creates a placeholder Sobol sequence generator.
    ///
    /// # Panics
    ///
    /// Always panics — Sobol sequences are not yet implemented.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(dimension: usize) -> Self {
        unimplemented!(
            "Sobol sequence not implemented in the current phase. \
             Dimension {} was requested. \
             See research.md for future integration plans with `sobol` crate.",
            dimension
        );
    }
}

impl LowDiscrepancySequence for SobolPlaceholder {
    fn dimension(&self) -> usize {
        unimplemented!("Sobol sequence not implemented in the current phase")
    }

    fn next_point(&mut self) -> &[f64] {
        unimplemented!("Sobol sequence not implemented in the current phase")
    }

    fn reset(&mut self) { unimplemented!("Sobol sequence not implemented in the current phase") }

    fn skip(&mut self, _n: usize) {
        unimplemented!("Sobol sequence not implemented in the current phase")
    }
}

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
///
/// Low-discrepancy sequences provide more uniform coverage of the unit hypercube
/// compared to pseudo-random sequences, often leading to faster convergence in
/// numerical integration problems.
///
/// # Future Implementation
///
/// This trait will be implemented by Sobol, Halton, and other low-discrepancy
/// sequence generators in future phases.
///
/// # Warning
///
/// No implementations are available in Phase 3.1a. Attempting to use
/// placeholder types will result in panics with clear error messages.
///
/// # British English Note
///
/// All documentation uses British English spelling conventions.
pub trait LowDiscrepancySequence {
    /// Returns the dimensionality of the sequence.
    ///
    /// # Returns
    ///
    /// The number of dimensions in each generated point.
    fn dimension(&self) -> usize;

    /// Advances the sequence and returns the next point.
    ///
    /// # Returns
    ///
    /// A slice of `dimension()` values, each in the interval [0, 1).
    fn next_point(&mut self) -> &[f64];

    /// Resets the sequence to its initial state.
    ///
    /// After calling this method, the sequence will produce the same
    /// points as a newly initialised instance.
    fn reset(&mut self);

    /// Skips ahead by `n` points in the sequence.
    ///
    /// This is useful for parallel computation where different workers
    /// need to process non-overlapping portions of the sequence.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of points to skip
    fn skip(&mut self, n: usize);
}

/// Placeholder for Sobol sequence generator.
///
/// # Not Implemented
///
/// This type exists to define the future interface for Sobol sequences.
/// All methods will panic with `unimplemented!()` if called.
///
/// # Future Plans
///
/// - Integration with `sobol` crate (21,201 dimensions supported)
/// - Joe-Kuo D6 parameters for high-dimensional sequences
/// - Antonov-Saleev grey code optimisation
///
/// # Example (will panic)
///
/// ```rust,should_panic
/// use pricer_pricing::rng::SobolPlaceholder;
///
/// let mut sobol = SobolPlaceholder::new(10);  // panics!
/// ```
///
/// # British English Note
///
/// All documentation uses British English spelling conventions.
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
    /// Always panics with a message indicating that Sobol sequences
    /// are not implemented in Phase 3.1a.
    ///
    /// # Arguments
    ///
    /// * `dimension` - The desired dimensionality of the sequence
    #[allow(clippy::new_ret_no_self)]
    pub fn new(dimension: usize) -> Self {
        unimplemented!(
            "Sobol sequence not implemented in Phase 3.1a. \
             Dimension {} was requested. \
             See research.md for future integration plans with `sobol` crate.",
            dimension
        );
    }
}

impl LowDiscrepancySequence for SobolPlaceholder {
    fn dimension(&self) -> usize {
        unimplemented!("Sobol sequence not implemented in Phase 3.1a")
    }

    fn next_point(&mut self) -> &[f64] {
        unimplemented!("Sobol sequence not implemented in Phase 3.1a")
    }

    fn reset(&mut self) {
        unimplemented!("Sobol sequence not implemented in Phase 3.1a")
    }

    fn skip(&mut self, _n: usize) {
        unimplemented!("Sobol sequence not implemented in Phase 3.1a")
    }
}

//! Checkpoint strategy for memory-efficient automatic differentiation.
//!
//! This module provides different strategies for determining when to save
//! simulation state during forward pass, enabling memory-efficient reverse
//! pass computation.

/// Checkpoint interval strategy.
///
/// Determines when to save simulation state during Monte Carlo forward pass.
/// Different strategies offer trade-offs between memory usage and recomputation cost.
///
/// # Enzyme AD Integration
///
/// During reverse-mode AD, checkpoints allow the engine to recompute forward
/// values from saved states rather than storing all intermediate values,
/// reducing memory usage from O(n) to O(√n) in the optimal case.
///
/// # Examples
///
/// ```
/// use pricer_pricing::checkpoint::CheckpointStrategy;
///
/// // Uniform interval: checkpoint every 10 steps
/// let strategy = CheckpointStrategy::Uniform { interval: 10 };
/// assert!(strategy.should_checkpoint(0, 100));
/// assert!(!strategy.should_checkpoint(5, 100));
/// assert!(strategy.should_checkpoint(10, 100));
///
/// // No checkpointing (store all intermediate values)
/// let none = CheckpointStrategy::None;
/// assert!(!none.should_checkpoint(50, 100));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CheckpointStrategy {
    /// Uniform interval checkpointing.
    ///
    /// Saves state every `interval` steps. Simple and predictable.
    /// Good default for most use cases.
    Uniform {
        /// Number of steps between checkpoints
        interval: usize,
    },

    /// Logarithmic interval checkpointing (Griewank-style).
    ///
    /// Based on Griewank's binomial checkpointing theory, this strategy
    /// places checkpoints at logarithmically increasing intervals.
    /// Optimal for reverse-mode AD with O(log n) checkpoints.
    Logarithmic {
        /// Base interval for the first checkpoint
        base_interval: usize,
    },

    /// Adaptive checkpointing based on memory pressure.
    ///
    /// Adjusts checkpoint interval dynamically to stay within
    /// a target memory budget.
    Adaptive {
        /// Target memory usage in megabytes
        target_memory_mb: usize,
    },

    /// No checkpointing.
    ///
    /// All intermediate values are stored. Maximum memory usage
    /// but no recomputation overhead.
    None,

    /// Binomial checkpointing (Griewank-Walther algorithm).
    ///
    /// Optimal strategy for fixed memory budget, achieving O(√n) memory
    /// usage. The `memory_slots` parameter specifies the maximum number
    /// of checkpoint states that can be stored simultaneously.
    ///
    /// This is the recommended strategy when using [`MinimalState`] for
    /// memory-efficient reverse-mode AD.
    ///
    /// # Reference
    ///
    /// Griewank, A., & Walther, A. (2000). Algorithm 799: revolve: an
    /// implementation of checkpointing for the reverse or adjoint mode
    /// of computational differentiation.
    Binomial {
        /// Maximum number of checkpoints to store simultaneously.
        /// For n steps, optimal is approximately √n checkpoints.
        memory_slots: usize,
    },
}

impl CheckpointStrategy {
    /// Determines if a checkpoint should be saved at the given step.
    ///
    /// # Arguments
    ///
    /// * `step` - Current simulation step (0-indexed)
    /// * `total_steps` - Total number of steps in the simulation
    ///
    /// # Returns
    ///
    /// `true` if state should be saved at this step, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_pricing::checkpoint::CheckpointStrategy;
    ///
    /// let uniform = CheckpointStrategy::Uniform { interval: 5 };
    /// assert!(uniform.should_checkpoint(0, 100));
    /// assert!(!uniform.should_checkpoint(3, 100));
    /// assert!(uniform.should_checkpoint(5, 100));
    /// assert!(uniform.should_checkpoint(10, 100));
    /// ```
    #[inline]
    #[allow(unknown_lints)]
    #[allow(clippy::manual_is_multiple_of)]
    pub fn should_checkpoint(&self, step: usize, total_steps: usize) -> bool {
        match self {
            CheckpointStrategy::Uniform { interval } => {
                if *interval == 0 {
                    return false;
                }
                step % interval == 0
            }
            CheckpointStrategy::Logarithmic { base_interval } => {
                if *base_interval == 0 || total_steps == 0 {
                    return false;
                }
                // Logarithmic checkpointing: checkpoint at steps
                // base_interval, 2*base_interval, 4*base_interval, etc.
                if step == 0 {
                    return true;
                }
                if step < *base_interval {
                    return false;
                }
                // Check if step is exactly a power of 2 multiple of base_interval
                // Step must be divisible by base_interval AND quotient must be power of 2
                if step % base_interval != 0 {
                    return false;
                }
                let ratio = step / base_interval;
                ratio > 0 && (ratio & (ratio - 1)) == 0
            }
            CheckpointStrategy::Adaptive {
                target_memory_mb: _,
            } => {
                // Adaptive strategy requires runtime memory information
                // For now, fall back to uniform with interval based on total_steps
                if total_steps == 0 {
                    return false;
                }
                // Default: ~10 checkpoints
                let interval = (total_steps / 10).max(1);
                step % interval == 0
            }
            CheckpointStrategy::None => false,
            CheckpointStrategy::Binomial { memory_slots } => {
                if *memory_slots == 0 || total_steps == 0 {
                    return false;
                }
                // Binomial checkpointing: checkpoint at intervals of √n
                // This achieves O(√n) memory with O(√n) recomputation
                let interval = ((total_steps as f64).sqrt().ceil() as usize).max(1);
                step % interval == 0 && step / interval < *memory_slots
            }
        }
    }

    /// Returns the estimated number of checkpoints for the given total steps.
    ///
    /// # Arguments
    ///
    /// * `total_steps` - Total number of steps in the simulation
    ///
    /// # Returns
    ///
    /// Estimated number of checkpoints that will be created.
    #[inline]
    pub fn estimated_checkpoints(&self, total_steps: usize) -> usize {
        match self {
            CheckpointStrategy::Uniform { interval } => {
                if *interval == 0 {
                    0
                } else {
                    (total_steps / interval) + 1
                }
            }
            CheckpointStrategy::Logarithmic { base_interval } => {
                if *base_interval == 0 || total_steps == 0 {
                    0
                } else {
                    // Number of power-of-2 multiples of base_interval that fit
                    let max_ratio = total_steps / base_interval;
                    if max_ratio == 0 {
                        1 // At least step 0
                    } else {
                        // Count powers of 2 up to max_ratio, plus 1 for step 0
                        (max_ratio as f64).log2().floor() as usize + 2
                    }
                }
            }
            CheckpointStrategy::Adaptive { .. } => {
                // Approximately 10 checkpoints
                11
            }
            CheckpointStrategy::None => 0,
            CheckpointStrategy::Binomial { memory_slots } => {
                // Number of checkpoints is min(memory_slots, √n)
                let sqrt_n = (total_steps as f64).sqrt().ceil() as usize;
                (*memory_slots).min(sqrt_n)
            }
        }
    }

    /// Creates a Binomial strategy with optimal memory slots for the given step count.
    ///
    /// This is a convenience constructor that calculates the optimal number of
    /// memory slots (approximately √n) for the given total steps.
    ///
    /// # Arguments
    ///
    /// * `total_steps` - Expected total number of simulation steps
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_pricing::checkpoint::CheckpointStrategy;
    ///
    /// // For 10000 steps, optimal is ~100 checkpoints
    /// let strategy = CheckpointStrategy::binomial_optimal(10000);
    /// assert_eq!(strategy.estimated_checkpoints(10000), 100);
    /// ```
    pub fn binomial_optimal(total_steps: usize) -> Self {
        let memory_slots = (total_steps as f64).sqrt().ceil() as usize;
        CheckpointStrategy::Binomial {
            memory_slots: memory_slots.max(1),
        }
    }
}

impl Default for CheckpointStrategy {
    /// Returns the default checkpoint strategy.
    ///
    /// Default is `Uniform { interval: 100 }`, providing a good balance
    /// between memory usage and recomputation overhead for typical
    /// Monte Carlo simulations.
    fn default() -> Self {
        CheckpointStrategy::Uniform { interval: 100 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // CheckpointStrategy::Uniform Tests
    // ========================================================================

    #[test]
    fn test_uniform_should_checkpoint_at_zero() {
        let strategy = CheckpointStrategy::Uniform { interval: 10 };
        assert!(strategy.should_checkpoint(0, 100));
    }

    #[test]
    fn test_uniform_should_checkpoint_at_interval() {
        let strategy = CheckpointStrategy::Uniform { interval: 10 };
        assert!(strategy.should_checkpoint(10, 100));
        assert!(strategy.should_checkpoint(20, 100));
        assert!(strategy.should_checkpoint(100, 100));
    }

    #[test]
    fn test_uniform_should_not_checkpoint_between_intervals() {
        let strategy = CheckpointStrategy::Uniform { interval: 10 };
        assert!(!strategy.should_checkpoint(1, 100));
        assert!(!strategy.should_checkpoint(5, 100));
        assert!(!strategy.should_checkpoint(9, 100));
        assert!(!strategy.should_checkpoint(15, 100));
    }

    #[test]
    fn test_uniform_interval_one_checkpoints_every_step() {
        let strategy = CheckpointStrategy::Uniform { interval: 1 };
        for step in 0..10 {
            assert!(strategy.should_checkpoint(step, 10));
        }
    }

    #[test]
    fn test_uniform_interval_zero_never_checkpoints() {
        let strategy = CheckpointStrategy::Uniform { interval: 0 };
        for step in 0..10 {
            assert!(!strategy.should_checkpoint(step, 10));
        }
    }

    #[test]
    fn test_uniform_estimated_checkpoints() {
        let strategy = CheckpointStrategy::Uniform { interval: 10 };
        // 0, 10, 20, ..., 100 = 11 checkpoints
        assert_eq!(strategy.estimated_checkpoints(100), 11);
    }

    // ========================================================================
    // CheckpointStrategy::Logarithmic Tests
    // ========================================================================

    #[test]
    fn test_logarithmic_should_checkpoint_at_zero() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        assert!(strategy.should_checkpoint(0, 100));
    }

    #[test]
    fn test_logarithmic_should_checkpoint_at_powers_of_two() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        // Should checkpoint at: 0, 10, 20, 40, 80, ...
        assert!(strategy.should_checkpoint(10, 100)); // 1 * 10
        assert!(strategy.should_checkpoint(20, 100)); // 2 * 10
        assert!(strategy.should_checkpoint(40, 100)); // 4 * 10
        assert!(strategy.should_checkpoint(80, 100)); // 8 * 10
    }

    #[test]
    fn test_logarithmic_should_not_checkpoint_non_powers() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        assert!(!strategy.should_checkpoint(5, 100));
        assert!(!strategy.should_checkpoint(15, 100));
        assert!(!strategy.should_checkpoint(30, 100)); // 3 * 10, not power of 2
        assert!(!strategy.should_checkpoint(50, 100)); // 5 * 10, not power of 2
    }

    #[test]
    fn test_logarithmic_base_interval_zero() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 0 };
        assert!(!strategy.should_checkpoint(0, 100));
        assert!(!strategy.should_checkpoint(10, 100));
    }

    // ========================================================================
    // CheckpointStrategy::Adaptive Tests
    // ========================================================================

    #[test]
    fn test_adaptive_checkpoints_approximately_ten_times() {
        let strategy = CheckpointStrategy::Adaptive {
            target_memory_mb: 100,
        };
        // With 100 steps, should checkpoint roughly every 10 steps
        let checkpoint_count: usize = (0..100)
            .filter(|&step| strategy.should_checkpoint(step, 100))
            .count();
        // Should be around 10 checkpoints (±2)
        assert!(checkpoint_count >= 8 && checkpoint_count <= 12);
    }

    #[test]
    fn test_adaptive_handles_zero_total_steps() {
        let strategy = CheckpointStrategy::Adaptive {
            target_memory_mb: 100,
        };
        assert!(!strategy.should_checkpoint(0, 0));
    }

    // ========================================================================
    // CheckpointStrategy::None Tests
    // ========================================================================

    #[test]
    fn test_none_never_checkpoints() {
        let strategy = CheckpointStrategy::None;
        for step in 0..100 {
            assert!(!strategy.should_checkpoint(step, 100));
        }
    }

    #[test]
    fn test_none_estimated_checkpoints_is_zero() {
        let strategy = CheckpointStrategy::None;
        assert_eq!(strategy.estimated_checkpoints(100), 0);
    }

    // ========================================================================
    // Default Tests
    // ========================================================================

    #[test]
    fn test_default_is_uniform_100() {
        let strategy = CheckpointStrategy::default();
        assert_eq!(strategy, CheckpointStrategy::Uniform { interval: 100 });
    }

    // ========================================================================
    // Clone and Debug Tests
    // ========================================================================

    #[test]
    fn test_strategy_clone() {
        let strategy = CheckpointStrategy::Uniform { interval: 50 };
        let cloned = strategy;
        assert_eq!(strategy, cloned);
    }

    #[test]
    fn test_strategy_debug() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("Logarithmic"));
        assert!(debug_str.contains("10"));
    }
}

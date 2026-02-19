//! Checkpoint strategy for memory-efficient automatic differentiation.

/// Checkpoint interval strategy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CheckpointStrategy {
    /// Uniform interval checkpointing.
    Uniform {
        /// Step interval between checkpoints.
        interval: usize,
    },

    /// Logarithmic interval checkpointing (Griewank-style).
    Logarithmic {
        /// Base interval for logarithmic spacing.
        base_interval: usize,
    },

    /// Adaptive checkpointing based on memory pressure.
    Adaptive {
        /// Target memory budget in megabytes.
        target_memory_mb: usize,
    },

    /// No checkpointing.
    None,

    /// Binomial checkpointing (Griewank-Walther algorithm).
    Binomial {
        /// Number of available memory slots.
        memory_slots: usize,
    },
}

impl CheckpointStrategy {
    /// Determines if a checkpoint should be saved at the given step.
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
                if step == 0 {
                    return true;
                }
                if step < *base_interval {
                    return false;
                }
                if step % base_interval != 0 {
                    return false;
                }
                let ratio = step / base_interval;
                ratio > 0 && ratio.is_power_of_two()
            }
            CheckpointStrategy::Adaptive {
                target_memory_mb: _,
            } => {
                if total_steps == 0 {
                    return false;
                }
                let interval = (total_steps / 10).max(1);
                step % interval == 0
            }
            CheckpointStrategy::None => false,
            CheckpointStrategy::Binomial { memory_slots } => {
                if *memory_slots == 0 || total_steps == 0 {
                    return false;
                }
                let interval = ((total_steps as f64).sqrt().ceil() as usize).max(1);
                step % interval == 0 && step / interval < *memory_slots
            }
        }
    }

    /// Returns the estimated number of checkpoints for the given total steps.
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
                    let max_ratio = total_steps / base_interval;
                    if max_ratio == 0 {
                        1
                    } else {
                        (max_ratio as f64).log2().floor() as usize + 2
                    }
                }
            }
            CheckpointStrategy::Adaptive { .. } => 11,
            CheckpointStrategy::None => 0,
            CheckpointStrategy::Binomial { memory_slots } => {
                let sqrt_n = (total_steps as f64).sqrt().ceil() as usize;
                (*memory_slots).min(sqrt_n)
            }
        }
    }

    /// Creates a Binomial strategy with optimal memory slots for the given step
    /// count.
    pub fn binomial_optimal(total_steps: usize) -> Self {
        let memory_slots = (total_steps as f64).sqrt().ceil() as usize;
        CheckpointStrategy::Binomial {
            memory_slots: memory_slots.max(1),
        }
    }
}

impl Default for CheckpointStrategy {
    fn default() -> Self { CheckpointStrategy::Uniform { interval: 100 } }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(strategy.estimated_checkpoints(100), 11);
    }

    #[test]
    fn test_logarithmic_should_checkpoint_at_zero() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        assert!(strategy.should_checkpoint(0, 100));
    }

    #[test]
    fn test_logarithmic_should_checkpoint_at_powers_of_two() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        assert!(strategy.should_checkpoint(10, 100));
        assert!(strategy.should_checkpoint(20, 100));
        assert!(strategy.should_checkpoint(40, 100));
        assert!(strategy.should_checkpoint(80, 100));
    }

    #[test]
    fn test_logarithmic_should_not_checkpoint_non_powers() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 10 };
        assert!(!strategy.should_checkpoint(5, 100));
        assert!(!strategy.should_checkpoint(15, 100));
        assert!(!strategy.should_checkpoint(30, 100));
        assert!(!strategy.should_checkpoint(50, 100));
    }

    #[test]
    fn test_logarithmic_base_interval_zero() {
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 0 };
        assert!(!strategy.should_checkpoint(0, 100));
        assert!(!strategy.should_checkpoint(10, 100));
    }

    #[test]
    fn test_adaptive_checkpoints_approximately_ten_times() {
        let strategy = CheckpointStrategy::Adaptive {
            target_memory_mb: 100,
        };
        let checkpoint_count: usize = (0..100)
            .filter(|&step| strategy.should_checkpoint(step, 100))
            .count();
        assert!(checkpoint_count >= 8 && checkpoint_count <= 12);
    }

    #[test]
    fn test_adaptive_handles_zero_total_steps() {
        let strategy = CheckpointStrategy::Adaptive {
            target_memory_mb: 100,
        };
        assert!(!strategy.should_checkpoint(0, 0));
    }

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

    #[test]
    fn test_default_is_uniform_100() {
        let strategy = CheckpointStrategy::default();
        assert_eq!(strategy, CheckpointStrategy::Uniform { interval: 100 });
    }

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

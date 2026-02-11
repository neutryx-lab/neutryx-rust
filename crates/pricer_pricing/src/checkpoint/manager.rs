//! Checkpoint manager for orchestrating checkpoint operations.

use num_traits::Float;
use thiserror::Error;

use super::{
    budget::MemoryBudget,
    state::{CheckpointStorage, SimulationState},
    strategy::CheckpointStrategy,
};

/// Errors that can occur during checkpoint operations.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Checkpoint not found for the given step.
    #[error("Checkpoint not found for step {step}")]
    NotFound {
        /// The step that was not found.
        step: usize,
    },

    /// Checkpoint storage is full.
    #[error("Checkpoint storage is full (capacity: {capacity})")]
    StorageFull {
        /// Maximum storage capacity.
        capacity: usize,
    },

    /// Invalid simulation state.
    #[error("Invalid simulation state: {message}")]
    InvalidState {
        /// Description of the invalid state.
        message: String,
    },

    /// Memory budget exceeded.
    #[error("Memory budget exceeded: {current} bytes > {max} bytes")]
    MemoryExceeded {
        /// Current memory usage in bytes.
        current: usize,
        /// Maximum allowed memory in bytes.
        max: usize,
    },
}

/// Result type for checkpoint operations.
pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// Manages checkpoint creation and retrieval for memory-efficient AD.
#[derive(Clone, Debug)]
pub struct CheckpointManager<T: Float> {
    strategy: CheckpointStrategy,
    storage: CheckpointStorage<T>,
    total_steps: usize,
    memory_budget: Option<MemoryBudget>,
}

impl<T: Float> CheckpointManager<T> {
    /// Creates a new checkpoint manager with the given strategy.
    pub fn new(strategy: CheckpointStrategy) -> Self {
        let estimated = strategy.estimated_checkpoints(1000);
        let capacity = estimated.clamp(10, 1000);

        Self {
            strategy,
            storage: CheckpointStorage::new(capacity),
            total_steps: 0,
            memory_budget: None,
        }
    }

    /// Creates a checkpoint manager with custom storage capacity.
    pub fn with_capacity(strategy: CheckpointStrategy, max_checkpoints: usize) -> Self {
        Self {
            strategy,
            storage: CheckpointStorage::new(max_checkpoints),
            total_steps: 0,
            memory_budget: None,
        }
    }

    /// Sets a memory budget for automatic memory management.
    pub fn with_memory_budget(mut self, budget: MemoryBudget) -> Self {
        self.memory_budget = Some(budget);
        self
    }

    /// Returns the memory budget if set.
    pub fn memory_budget(&self) -> Option<&MemoryBudget> { self.memory_budget.as_ref() }

    /// Checks if current memory usage is within the budget.
    pub fn is_within_budget(&self) -> bool {
        match &self.memory_budget {
            Some(budget) => budget.is_within_budget(self.memory_usage()),
            None => true,
        }
    }

    /// Checks if current memory usage exceeds the warning threshold.
    pub fn is_memory_warning(&self) -> bool {
        match &self.memory_budget {
            Some(budget) => budget.is_warning(self.memory_usage()),
            None => false,
        }
    }

    /// Returns the recommended checkpoint interval based on memory budget.
    pub fn recommended_interval(&self, n_paths: usize, state_size_per_path: usize) -> usize {
        match &self.memory_budget {
            Some(budget) => {
                budget.recommended_interval(n_paths, self.total_steps, state_size_per_path)
            }
            None => match self.strategy {
                CheckpointStrategy::Uniform { interval } => interval,
                CheckpointStrategy::Logarithmic { base_interval } => base_interval,
                CheckpointStrategy::Adaptive { .. } => (self.total_steps / 10).max(1),
                CheckpointStrategy::None => self.total_steps,
                CheckpointStrategy::Binomial { memory_slots } => {
                    let interval = ((self.total_steps as f64).sqrt().ceil() as usize).max(1);
                    self.total_steps
                        .checked_div(memory_slots)
                        .map(|v| v.max(1))
                        .unwrap_or(interval)
                }
            },
        }
    }

    /// Sets the total number of simulation steps.
    pub fn set_total_steps(&mut self, total_steps: usize) { self.total_steps = total_steps; }

    /// Sets the total number of simulation steps (builder pattern).
    pub fn with_total_steps(mut self, total_steps: usize) -> Self {
        self.total_steps = total_steps;
        self
    }

    /// Returns the current checkpoint strategy.
    pub fn strategy(&self) -> &CheckpointStrategy { &self.strategy }

    /// Determines if a checkpoint should be saved at the given step.
    #[inline]
    pub fn should_checkpoint(&self, step: usize) -> bool {
        self.strategy.should_checkpoint(step, self.total_steps)
    }

    /// Saves a simulation state as a checkpoint.
    pub fn save_state(&mut self, step: usize, state: SimulationState<T>) -> CheckpointResult<()> {
        if state.step != step {
            return Err(CheckpointError::InvalidState {
                message: format!(
                    "State step {} doesn't match provided step {}",
                    state.step, step
                ),
            });
        }

        self.storage.save(step, state);
        Ok(())
    }

    /// Restores a simulation state from a checkpoint.
    pub fn restore_state(&self, step: usize) -> CheckpointResult<SimulationState<T>> {
        self.storage
            .get(step)
            .cloned()
            .ok_or(CheckpointError::NotFound { step })
    }

    /// Finds the nearest checkpoint at or before the given step.
    pub fn nearest_checkpoint(&self, step: usize) -> Option<usize> {
        self.storage.nearest_before(step)
    }

    /// Returns the number of stored checkpoints.
    pub fn checkpoint_count(&self) -> usize { self.storage.len() }

    /// Returns true if no checkpoints are stored.
    pub fn is_empty(&self) -> bool { self.storage.is_empty() }

    /// Clears all stored checkpoints.
    pub fn clear(&mut self) { self.storage.clear(); }

    /// Returns the total memory usage of stored checkpoints in bytes.
    pub fn memory_usage(&self) -> usize { self.storage.memory_usage() }

    /// Returns the storage capacity.
    pub fn capacity(&self) -> usize { self.storage.capacity() }
}

impl<T: Float> Default for CheckpointManager<T> {
    fn default() -> Self { Self::new(CheckpointStrategy::default()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::path_dependent::PathObserverState;

    fn create_test_state(step: usize, n_paths: usize) -> SimulationState<f64> {
        SimulationState::new(
            step,
            42,
            step * 100,
            PathObserverState::default(),
            vec![100.0; n_paths],
        )
    }

    #[test]
    fn test_manager_new() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        assert!(manager.is_empty());
        assert_eq!(manager.checkpoint_count(), 0);
    }

    #[test]
    fn test_manager_with_capacity() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::with_capacity(CheckpointStrategy::None, 50);

        assert_eq!(manager.capacity(), 50);
    }

    #[test]
    fn test_manager_default() {
        let manager: CheckpointManager<f64> = Default::default();
        assert_eq!(
            *manager.strategy(),
            CheckpointStrategy::Uniform { interval: 100 }
        );
    }

    #[test]
    fn test_should_checkpoint_delegates_to_strategy() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });
        manager.set_total_steps(100);

        assert!(manager.should_checkpoint(0));
        assert!(!manager.should_checkpoint(5));
        assert!(manager.should_checkpoint(10));
        assert!(!manager.should_checkpoint(15));
        assert!(manager.should_checkpoint(20));
    }

    #[test]
    fn test_should_checkpoint_with_none_strategy() {
        let mut manager: CheckpointManager<f64> = CheckpointManager::new(CheckpointStrategy::None);
        manager.set_total_steps(100);

        for step in 0..100 {
            assert!(!manager.should_checkpoint(step));
        }
    }

    #[test]
    fn test_save_and_restore_state() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        let state = create_test_state(100, 50);
        manager.save_state(100, state).unwrap();

        let restored = manager.restore_state(100).unwrap();
        assert_eq!(restored.step, 100);
        assert_eq!(restored.current_prices.len(), 50);
    }

    #[test]
    fn test_save_state_step_mismatch_error() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        let state = create_test_state(50, 10);
        let result = manager.save_state(100, state);

        assert!(result.is_err());
        match result {
            Err(CheckpointError::InvalidState { message }) => {
                assert!(message.contains("50"));
                assert!(message.contains("100"));
            }
            _ => panic!("Expected InvalidState error"),
        }
    }

    #[test]
    fn test_restore_nonexistent_state_error() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        let result = manager.restore_state(100);
        assert!(result.is_err());
        match result {
            Err(CheckpointError::NotFound { step }) => {
                assert_eq!(step, 100);
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_nearest_checkpoint() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        manager.save_state(0, create_test_state(0, 10)).unwrap();
        manager.save_state(100, create_test_state(100, 10)).unwrap();
        manager.save_state(200, create_test_state(200, 10)).unwrap();

        assert_eq!(manager.nearest_checkpoint(100), Some(100));
        assert_eq!(manager.nearest_checkpoint(150), Some(100));
        assert_eq!(manager.nearest_checkpoint(250), Some(200));
    }

    #[test]
    fn test_nearest_checkpoint_empty() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        assert_eq!(manager.nearest_checkpoint(100), None);
    }

    #[test]
    fn test_clear() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        manager.save_state(0, create_test_state(0, 10)).unwrap();
        manager.save_state(100, create_test_state(100, 10)).unwrap();

        assert_eq!(manager.checkpoint_count(), 2);

        manager.clear();

        assert!(manager.is_empty());
        assert_eq!(manager.checkpoint_count(), 0);
    }

    #[test]
    fn test_memory_usage() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        manager.save_state(0, create_test_state(0, 1000)).unwrap();
        manager
            .save_state(100, create_test_state(100, 1000))
            .unwrap();

        let usage = manager.memory_usage();
        assert!(usage > 2 * 1000 * std::mem::size_of::<f64>());
    }

    #[test]
    fn test_full_forward_pass_checkpointing() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 25 });
        manager.set_total_steps(100);

        for step in 0..=100 {
            if manager.should_checkpoint(step) {
                let state = create_test_state(step, 10);
                manager.save_state(step, state).unwrap();
            }
        }

        assert_eq!(manager.checkpoint_count(), 5);

        assert!(manager.restore_state(0).is_ok());
        assert!(manager.restore_state(25).is_ok());
        assert!(manager.restore_state(50).is_ok());
        assert!(manager.restore_state(75).is_ok());
        assert!(manager.restore_state(100).is_ok());
    }

    #[test]
    fn test_with_memory_budget() {
        let budget = MemoryBudget::from_mb(100);
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 })
                .with_memory_budget(budget);

        assert!(manager.memory_budget().is_some());
        assert_eq!(
            manager.memory_budget().unwrap().max_bytes(),
            100 * 1024 * 1024
        );
    }

    #[test]
    fn test_is_within_budget_no_budget() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        assert!(manager.is_within_budget());
    }

    #[test]
    fn test_is_within_budget_with_budget() {
        let budget = MemoryBudget::from_mb(1);
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 })
                .with_memory_budget(budget);

        assert!(manager.is_within_budget());

        manager.save_state(0, create_test_state(0, 100)).unwrap();
        assert!(manager.is_within_budget());
    }

    #[test]
    fn test_is_memory_warning_no_budget() {
        let manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 });

        assert!(!manager.is_memory_warning());
    }

    #[test]
    fn test_is_memory_warning_with_budget() {
        let budget = MemoryBudget::new(1000).with_warning_threshold(0.5);
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 })
                .with_memory_budget(budget);

        assert!(!manager.is_memory_warning());

        manager.save_state(0, create_test_state(0, 10)).unwrap();
    }

    #[test]
    fn test_recommended_interval_with_budget() {
        let budget = MemoryBudget::from_mb(10);
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 10 })
                .with_memory_budget(budget);
        manager.set_total_steps(1000);

        let interval = manager.recommended_interval(10_000, 8);
        assert!(interval > 0);
    }

    #[test]
    fn test_recommended_interval_without_budget() {
        let mut manager: CheckpointManager<f64> =
            CheckpointManager::new(CheckpointStrategy::Uniform { interval: 50 });
        manager.set_total_steps(1000);

        let interval = manager.recommended_interval(10_000, 8);
        assert_eq!(interval, 50);
    }
}

//! Simulation state capture for checkpointing.
//!
//! This module provides types for capturing and storing simulation state
//! at checkpoint intervals, enabling memory-efficient reverse-mode AD.
//!
//! # State Types
//!
//! Two state types are provided for different use cases:
//!
//! - [`MinimalState`]: Lightweight state for O(√n) memory checkpointing
//! - [`SimulationState`]: Full state including current prices for all paths

use crate::path_dependent::PathObserverState;
use num_traits::Float;

/// Minimal simulation state for memory-efficient checkpointing.
///
/// This struct captures only the essential information needed to resume
/// simulation: step number, RNG state, and observer statistics. Unlike
/// [`SimulationState`], it does not store current prices, achieving
/// significantly lower memory usage.
///
/// # Memory Efficiency
///
/// When combined with Binomial checkpointing strategy, MinimalState enables
/// O(√n) memory usage for reverse-mode AD instead of O(n).
///
/// # RNG State Recovery
///
/// The `rng_state` field stores the RNG seed, not the full state. To resume
/// simulation, create a new RNG with this seed and skip `rng_calls` values.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::checkpoint::MinimalState;
/// use pricer_pricing::path_dependent::PathObserverState;
///
/// let state = MinimalState {
///     step: 100,
///     rng_state: 42,
///     rng_calls: 10000,
///     observer_snapshot: PathObserverState::default(),
/// };
///
/// assert_eq!(state.step, 100);
/// assert!(state.memory_size() < 200); // Very compact
/// ```
#[derive(Clone, Debug)]
pub struct MinimalState {
    /// Current simulation step (0-indexed).
    pub step: usize,

    /// Original RNG seed for reproducibility.
    ///
    /// To restore RNG state: create new RNG with this seed,
    /// then advance by `rng_calls` values.
    pub rng_state: u64,

    /// Number of RNG calls made up to this checkpoint.
    ///
    /// Used to fast-forward RNG to correct state when restoring.
    pub rng_calls: usize,

    /// Path observer state (running statistics).
    ///
    /// Contains accumulated min/max/sum for path-dependent payoffs.
    pub observer_snapshot: PathObserverState<f64>,
}

impl MinimalState {
    /// Creates a new minimal state.
    ///
    /// # Arguments
    ///
    /// * `step` - Current step number
    /// * `rng_state` - Original RNG seed
    /// * `rng_calls` - Number of RNG calls made
    /// * `observer_snapshot` - Path observer state snapshot
    pub fn new(
        step: usize,
        rng_state: u64,
        rng_calls: usize,
        observer_snapshot: PathObserverState<f64>,
    ) -> Self {
        Self {
            step,
            rng_state,
            rng_calls,
            observer_snapshot,
        }
    }

    /// Returns the memory size of this state in bytes (approximate).
    ///
    /// MinimalState is designed to be very compact, typically under 200 bytes.
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    /// Validates that this state can be used to resume simulation.
    ///
    /// Returns `true` if the state is valid and consistent.
    pub fn is_valid(&self) -> bool {
        // RNG seed 0 is technically valid but unusual
        // Step can be any value
        // Observer state is always valid by construction
        true
    }
}

impl Default for MinimalState {
    fn default() -> Self {
        Self {
            step: 0,
            rng_state: 0,
            rng_calls: 0,
            observer_snapshot: PathObserverState::default(),
        }
    }
}

/// Complete simulation state at a checkpoint.
///
/// Contains all information needed to resume forward simulation from
/// a saved checkpoint. Used for memory-efficient reverse-mode AD where
/// intermediate values are recomputed from checkpoints rather than stored.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```rust,ignore
/// use pricer_pricing::checkpoint::SimulationState;
///
/// let state = SimulationState {
///     step: 100,
///     rng_seed: 42,
///     rng_calls: 1000,
///     observer_state: observer.snapshot(),
///     current_prices: prices.clone(),
/// };
/// ```
#[derive(Clone, Debug)]
pub struct SimulationState<T: Float> {
    /// Current simulation step (0-indexed)
    pub step: usize,

    /// Original RNG seed for reproducibility
    pub rng_seed: u64,

    /// Number of RNG calls made up to this checkpoint.
    /// Used to fast-forward RNG to correct state when restoring.
    pub rng_calls: usize,

    /// Path observer state (running statistics)
    pub observer_state: PathObserverState<T>,

    /// Current asset prices for each path
    pub current_prices: Vec<T>,
}

impl<T: Float> SimulationState<T> {
    /// Creates a new simulation state.
    ///
    /// # Arguments
    ///
    /// * `step` - Current step number
    /// * `rng_seed` - Original RNG seed
    /// * `rng_calls` - Number of RNG calls made
    /// * `observer_state` - Path observer state snapshot
    /// * `current_prices` - Current prices for all paths
    pub fn new(
        step: usize,
        rng_seed: u64,
        rng_calls: usize,
        observer_state: PathObserverState<T>,
        current_prices: Vec<T>,
    ) -> Self {
        Self {
            step,
            rng_seed,
            rng_calls,
            observer_state,
            current_prices,
        }
    }

    /// Returns the memory size of this state in bytes (approximate).
    ///
    /// Used for memory budget calculations.
    pub fn memory_size(&self) -> usize {
        // Base struct size + vector capacity
        std::mem::size_of::<Self>() + self.current_prices.capacity() * std::mem::size_of::<T>()
    }
}

/// Storage for multiple checkpoint states.
///
/// Manages a collection of simulation states, providing efficient
/// lookup and automatic eviction when capacity is reached.
///
/// # Type Parameters
///
/// * `T` - Floating-point type
///
/// # Example
///
/// ```rust,ignore
/// use pricer_pricing::checkpoint::CheckpointStorage;
///
/// let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);
///
/// // Save state at step 100
/// storage.save(100, state);
///
/// // Retrieve state
/// if let Some(state) = storage.get(100) {
///     println!("Found state at step {}", state.step);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CheckpointStorage<T: Float> {
    /// Stored checkpoints: (step, state) pairs
    checkpoints: Vec<(usize, SimulationState<T>)>,

    /// Maximum number of checkpoints to store
    max_checkpoints: usize,
}

impl<T: Float> CheckpointStorage<T> {
    /// Creates a new checkpoint storage with the given capacity.
    ///
    /// # Arguments
    ///
    /// * `max_checkpoints` - Maximum number of checkpoints to store
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: Vec::with_capacity(max_checkpoints),
            max_checkpoints,
        }
    }

    /// Saves a simulation state at the given step.
    ///
    /// If storage is at capacity, the oldest checkpoint is removed.
    /// If a checkpoint already exists for this step, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `step` - Step number for this checkpoint
    /// * `state` - Simulation state to save
    pub fn save(&mut self, step: usize, state: SimulationState<T>) {
        // Check if we already have a checkpoint at this step
        if let Some(pos) = self.checkpoints.iter().position(|(s, _)| *s == step) {
            self.checkpoints[pos] = (step, state);
            return;
        }

        // If at capacity, remove oldest checkpoint
        if self.checkpoints.len() >= self.max_checkpoints && self.max_checkpoints > 0 {
            // Find and remove the oldest (smallest step number)
            if let Some((min_idx, _)) = self
                .checkpoints
                .iter()
                .enumerate()
                .min_by_key(|(_, (s, _))| *s)
            {
                self.checkpoints.remove(min_idx);
            }
        }

        self.checkpoints.push((step, state));
    }

    /// Retrieves the state at the given step.
    ///
    /// # Arguments
    ///
    /// * `step` - Step number to look up
    ///
    /// # Returns
    ///
    /// Reference to the state if found, `None` otherwise.
    pub fn get(&self, step: usize) -> Option<&SimulationState<T>> {
        self.checkpoints
            .iter()
            .find(|(s, _)| *s == step)
            .map(|(_, state)| state)
    }

    /// Finds the nearest checkpoint at or before the given step.
    ///
    /// Useful for reverse-mode AD where we need to recompute forward
    /// from the nearest saved checkpoint.
    ///
    /// # Arguments
    ///
    /// * `step` - Target step number
    ///
    /// # Returns
    ///
    /// Step number of nearest checkpoint, or `None` if no checkpoints exist.
    pub fn nearest_before(&self, step: usize) -> Option<usize> {
        self.checkpoints
            .iter()
            .filter(|(s, _)| *s <= step)
            .max_by_key(|(s, _)| *s)
            .map(|(s, _)| *s)
    }

    /// Returns the number of stored checkpoints.
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// Returns true if no checkpoints are stored.
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// Clears all stored checkpoints.
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Returns an iterator over all checkpoints.
    pub fn iter(&self) -> impl Iterator<Item = &(usize, SimulationState<T>)> {
        self.checkpoints.iter()
    }

    /// Returns the total memory usage of stored checkpoints in bytes.
    pub fn memory_usage(&self) -> usize {
        self.checkpoints
            .iter()
            .map(|(_, state)| state.memory_size())
            .sum()
    }

    /// Returns the maximum capacity.
    pub fn capacity(&self) -> usize {
        self.max_checkpoints
    }
}

impl<T: Float> Default for CheckpointStorage<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test state
    fn create_test_state(step: usize, n_paths: usize) -> SimulationState<f64> {
        SimulationState {
            step,
            rng_seed: 42,
            rng_calls: step * 100,
            observer_state: PathObserverState::default(),
            current_prices: vec![100.0; n_paths],
        }
    }

    // ========================================================================
    // SimulationState Tests
    // ========================================================================

    #[test]
    fn test_simulation_state_new() {
        let state = SimulationState::new(
            10,
            42,
            1000,
            PathObserverState::default(),
            vec![100.0, 101.0, 102.0],
        );

        assert_eq!(state.step, 10);
        assert_eq!(state.rng_seed, 42);
        assert_eq!(state.rng_calls, 1000);
        assert_eq!(state.current_prices.len(), 3);
    }

    #[test]
    fn test_simulation_state_clone() {
        let state1 = create_test_state(50, 100);
        let state2 = state1.clone();

        assert_eq!(state1.step, state2.step);
        assert_eq!(state1.current_prices.len(), state2.current_prices.len());
    }

    #[test]
    fn test_simulation_state_memory_size() {
        let state = create_test_state(10, 1000);
        let size = state.memory_size();

        // Should include base struct + 1000 f64 values
        assert!(size > 1000 * std::mem::size_of::<f64>());
    }

    // ========================================================================
    // CheckpointStorage Tests
    // ========================================================================

    #[test]
    fn test_storage_new() {
        let storage: CheckpointStorage<f64> = CheckpointStorage::new(10);
        assert_eq!(storage.capacity(), 10);
        assert!(storage.is_empty());
    }

    #[test]
    fn test_storage_save_and_get() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);
        let state = create_test_state(100, 50);

        storage.save(100, state);

        assert!(!storage.is_empty());
        assert_eq!(storage.len(), 1);

        let retrieved = storage.get(100);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().step, 100);
    }

    #[test]
    fn test_storage_get_nonexistent() {
        let storage: CheckpointStorage<f64> = CheckpointStorage::new(10);
        assert!(storage.get(100).is_none());
    }

    #[test]
    fn test_storage_save_multiple() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        storage.save(0, create_test_state(0, 10));
        storage.save(100, create_test_state(100, 10));
        storage.save(200, create_test_state(200, 10));

        assert_eq!(storage.len(), 3);
        assert!(storage.get(0).is_some());
        assert!(storage.get(100).is_some());
        assert!(storage.get(200).is_some());
    }

    #[test]
    fn test_storage_replace_existing() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        let state1 = SimulationState::new(100, 42, 1000, PathObserverState::default(), vec![100.0]);
        let state2 = SimulationState::new(
            100,
            42,
            2000, // Different rng_calls
            PathObserverState::default(),
            vec![200.0],
        );

        storage.save(100, state1);
        storage.save(100, state2);

        assert_eq!(storage.len(), 1);
        let retrieved = storage.get(100).unwrap();
        assert_eq!(retrieved.rng_calls, 2000);
    }

    #[test]
    fn test_storage_eviction_at_capacity() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(3);

        // Fill storage
        storage.save(10, create_test_state(10, 5));
        storage.save(20, create_test_state(20, 5));
        storage.save(30, create_test_state(30, 5));
        assert_eq!(storage.len(), 3);

        // Add one more, should evict oldest (step 10)
        storage.save(40, create_test_state(40, 5));
        assert_eq!(storage.len(), 3);

        // Step 10 should be gone, others should exist
        assert!(storage.get(10).is_none());
        assert!(storage.get(20).is_some());
        assert!(storage.get(30).is_some());
        assert!(storage.get(40).is_some());
    }

    #[test]
    fn test_storage_nearest_before() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        storage.save(0, create_test_state(0, 5));
        storage.save(100, create_test_state(100, 5));
        storage.save(200, create_test_state(200, 5));

        // Exact match
        assert_eq!(storage.nearest_before(100), Some(100));

        // Between checkpoints
        assert_eq!(storage.nearest_before(150), Some(100));

        // Before first checkpoint
        assert_eq!(storage.nearest_before(0), Some(0));

        // After all checkpoints
        assert_eq!(storage.nearest_before(250), Some(200));
    }

    #[test]
    fn test_storage_nearest_before_empty() {
        let storage: CheckpointStorage<f64> = CheckpointStorage::new(10);
        assert_eq!(storage.nearest_before(100), None);
    }

    #[test]
    fn test_storage_clear() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        storage.save(0, create_test_state(0, 5));
        storage.save(100, create_test_state(100, 5));

        assert_eq!(storage.len(), 2);

        storage.clear();

        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_storage_iter() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        storage.save(0, create_test_state(0, 5));
        storage.save(100, create_test_state(100, 5));
        storage.save(200, create_test_state(200, 5));

        let steps: Vec<usize> = storage.iter().map(|(step, _)| *step).collect();
        assert_eq!(steps.len(), 3);
        assert!(steps.contains(&0));
        assert!(steps.contains(&100));
        assert!(steps.contains(&200));
    }

    #[test]
    fn test_storage_memory_usage() {
        let mut storage: CheckpointStorage<f64> = CheckpointStorage::new(10);

        storage.save(0, create_test_state(0, 100));
        storage.save(100, create_test_state(100, 100));

        let usage = storage.memory_usage();
        // Should be substantial due to 2 states with 100 prices each
        assert!(usage > 0);
    }

    #[test]
    fn test_storage_default() {
        let storage: CheckpointStorage<f64> = Default::default();
        assert_eq!(storage.capacity(), 100);
        assert!(storage.is_empty());
    }
}

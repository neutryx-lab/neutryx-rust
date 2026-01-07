//! Memory budget management for checkpointing.
//!
//! This module provides tools for managing memory usage during checkpoint
//! operations, including automatic interval calculation based on available
//! memory.

/// Memory budget for checkpoint storage.
///
/// Used to calculate recommended checkpoint intervals and monitor
/// memory usage during simulation.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::checkpoint::MemoryBudget;
///
/// // Create a 100 MB budget
/// let budget = MemoryBudget::from_mb(100);
///
/// // Calculate recommended checkpoint interval
/// let n_paths = 10_000;
/// let n_steps = 500;
/// let state_size = n_paths * std::mem::size_of::<f64>() + 100; // Approximate
///
/// let interval = budget.recommended_interval(n_paths, n_steps, state_size);
/// println!("Recommended interval: {} steps", interval);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemoryBudget {
    /// Maximum memory usage in bytes
    max_bytes: usize,

    /// Warning threshold as a fraction (0.0 to 1.0)
    warning_threshold: f64,
}

impl MemoryBudget {
    /// Creates a new memory budget with the specified maximum in bytes.
    ///
    /// # Arguments
    ///
    /// * `max_bytes` - Maximum memory usage in bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::checkpoint::MemoryBudget;
    ///
    /// // 50 MB budget
    /// let budget = MemoryBudget::new(50 * 1024 * 1024);
    /// assert_eq!(budget.max_bytes(), 50 * 1024 * 1024);
    /// ```
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            warning_threshold: 0.8,
        }
    }

    /// Creates a memory budget from megabytes.
    ///
    /// # Arguments
    ///
    /// * `mb` - Maximum memory usage in megabytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::checkpoint::MemoryBudget;
    ///
    /// let budget = MemoryBudget::from_mb(100);
    /// assert_eq!(budget.max_bytes(), 100 * 1024 * 1024);
    /// ```
    #[inline]
    pub fn from_mb(mb: usize) -> Self {
        Self::new(mb * 1024 * 1024)
    }

    /// Creates a memory budget from gigabytes.
    ///
    /// # Arguments
    ///
    /// * `gb` - Maximum memory usage in gigabytes
    #[inline]
    pub fn from_gb(gb: usize) -> Self {
        Self::new(gb * 1024 * 1024 * 1024)
    }

    /// Sets the warning threshold as a fraction of maximum.
    ///
    /// When memory usage exceeds this fraction of the maximum,
    /// `is_warning` returns true.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Fraction between 0.0 and 1.0
    ///
    /// # Panics
    ///
    /// Panics if threshold is not in [0.0, 1.0].
    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&threshold),
            "Warning threshold must be between 0.0 and 1.0"
        );
        self.warning_threshold = threshold;
        self
    }

    /// Returns the maximum memory in bytes.
    #[inline]
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Returns the warning threshold.
    #[inline]
    pub fn warning_threshold(&self) -> f64 {
        self.warning_threshold
    }

    /// Checks if the current usage is within budget.
    ///
    /// # Arguments
    ///
    /// * `current_usage` - Current memory usage in bytes
    ///
    /// # Returns
    ///
    /// `true` if usage is within budget.
    #[inline]
    pub fn is_within_budget(&self, current_usage: usize) -> bool {
        current_usage <= self.max_bytes
    }

    /// Checks if the current usage exceeds the warning threshold.
    ///
    /// # Arguments
    ///
    /// * `current_usage` - Current memory usage in bytes
    ///
    /// # Returns
    ///
    /// `true` if usage exceeds the warning threshold.
    #[inline]
    pub fn is_warning(&self, current_usage: usize) -> bool {
        let threshold_bytes = (self.max_bytes as f64 * self.warning_threshold) as usize;
        current_usage > threshold_bytes
    }

    /// Calculates the recommended checkpoint interval.
    ///
    /// Uses the memory budget, number of paths, and state size to determine
    /// how many checkpoints can be stored, then calculates the interval.
    ///
    /// # Arguments
    ///
    /// * `n_paths` - Number of simulation paths
    /// * `n_steps` - Total number of simulation steps
    /// * `state_size_per_path` - Approximate memory per path in bytes
    ///
    /// # Returns
    ///
    /// Recommended number of steps between checkpoints.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::checkpoint::MemoryBudget;
    ///
    /// let budget = MemoryBudget::from_mb(100);
    ///
    /// // 10,000 paths, 500 steps, ~8 bytes per path (f64)
    /// let interval = budget.recommended_interval(10_000, 500, 8);
    /// assert!(interval > 0);
    /// ```
    pub fn recommended_interval(
        &self,
        n_paths: usize,
        n_steps: usize,
        state_size_per_path: usize,
    ) -> usize {
        if n_steps == 0 || n_paths == 0 {
            return 1;
        }

        // Calculate total size of one checkpoint state
        // Overhead for SimulationState struct (step, rng_seed, rng_calls, observer_state)
        let overhead = 128; // Approximate overhead in bytes
        let checkpoint_size = n_paths * state_size_per_path + overhead;

        if checkpoint_size == 0 {
            return 1;
        }

        // How many checkpoints can we fit in the budget?
        let max_checkpoints = self.max_bytes / checkpoint_size;

        if max_checkpoints == 0 {
            // Can't fit even one checkpoint, use maximum interval
            return n_steps;
        }

        // Interval = total_steps / max_checkpoints
        // Add 1 to ensure we don't exceed budget due to rounding
        (n_steps / max_checkpoints).max(1)
    }

    /// Returns the remaining budget after accounting for current usage.
    ///
    /// # Arguments
    ///
    /// * `current_usage` - Current memory usage in bytes
    ///
    /// # Returns
    ///
    /// Remaining bytes available, or 0 if over budget.
    #[inline]
    pub fn remaining(&self, current_usage: usize) -> usize {
        self.max_bytes.saturating_sub(current_usage)
    }

    /// Returns the usage as a percentage of the budget.
    ///
    /// # Arguments
    ///
    /// * `current_usage` - Current memory usage in bytes
    ///
    /// # Returns
    ///
    /// Usage percentage (0.0 to 100.0+).
    #[inline]
    pub fn usage_percentage(&self, current_usage: usize) -> f64 {
        if self.max_bytes == 0 {
            return 100.0;
        }
        (current_usage as f64 / self.max_bytes as f64) * 100.0
    }
}

impl Default for MemoryBudget {
    /// Creates a default memory budget of 1 GB.
    fn default() -> Self {
        Self::from_gb(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Construction Tests
    // ========================================================================

    #[test]
    fn test_new() {
        let budget = MemoryBudget::new(1000);
        assert_eq!(budget.max_bytes(), 1000);
    }

    #[test]
    fn test_from_mb() {
        let budget = MemoryBudget::from_mb(50);
        assert_eq!(budget.max_bytes(), 50 * 1024 * 1024);
    }

    #[test]
    fn test_from_gb() {
        let budget = MemoryBudget::from_gb(2);
        assert_eq!(budget.max_bytes(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_default() {
        let budget = MemoryBudget::default();
        assert_eq!(budget.max_bytes(), 1024 * 1024 * 1024); // 1 GB
    }

    #[test]
    fn test_warning_threshold() {
        let budget = MemoryBudget::new(1000).with_warning_threshold(0.9);
        assert_eq!(budget.warning_threshold(), 0.9);
    }

    #[test]
    #[should_panic(expected = "between 0.0 and 1.0")]
    fn test_invalid_warning_threshold() {
        MemoryBudget::new(1000).with_warning_threshold(1.5);
    }

    // ========================================================================
    // Budget Checking Tests
    // ========================================================================

    #[test]
    fn test_is_within_budget() {
        let budget = MemoryBudget::new(1000);

        assert!(budget.is_within_budget(0));
        assert!(budget.is_within_budget(500));
        assert!(budget.is_within_budget(1000));
        assert!(!budget.is_within_budget(1001));
    }

    #[test]
    fn test_is_warning() {
        let budget = MemoryBudget::new(1000).with_warning_threshold(0.8);

        assert!(!budget.is_warning(799)); // Below 80%
        assert!(!budget.is_warning(800)); // At 80%
        assert!(budget.is_warning(801)); // Above 80%
        assert!(budget.is_warning(1000)); // At 100%
    }

    #[test]
    fn test_remaining() {
        let budget = MemoryBudget::new(1000);

        assert_eq!(budget.remaining(0), 1000);
        assert_eq!(budget.remaining(400), 600);
        assert_eq!(budget.remaining(1000), 0);
        assert_eq!(budget.remaining(1500), 0); // Saturating
    }

    #[test]
    fn test_usage_percentage() {
        let budget = MemoryBudget::new(1000);

        assert_eq!(budget.usage_percentage(0), 0.0);
        assert_eq!(budget.usage_percentage(500), 50.0);
        assert_eq!(budget.usage_percentage(1000), 100.0);
        assert_eq!(budget.usage_percentage(1500), 150.0);
    }

    #[test]
    fn test_usage_percentage_zero_budget() {
        let budget = MemoryBudget::new(0);
        assert_eq!(budget.usage_percentage(100), 100.0);
    }

    // ========================================================================
    // Interval Calculation Tests
    // ========================================================================

    #[test]
    fn test_recommended_interval_basic() {
        let budget = MemoryBudget::from_mb(100);

        // 10,000 paths, 500 steps, 8 bytes per path (f64)
        let interval = budget.recommended_interval(10_000, 500, 8);

        // Checkpoint size ≈ 10,000 * 8 + 128 ≈ 80,128 bytes
        // 100 MB / 80,128 ≈ 1,308 checkpoints
        // Interval = 500 / 1,308 = 0 -> max(1) = 1
        assert!(interval >= 1);
    }

    #[test]
    fn test_recommended_interval_large_states() {
        // Smaller budget, larger state size
        let budget = MemoryBudget::from_mb(10);

        // 100,000 paths, 1000 steps, 8 bytes per path
        // Checkpoint size ≈ 100,000 * 8 + 128 ≈ 800,128 bytes
        // 10 MB = 10,485,760 bytes
        // Max checkpoints ≈ 13
        // Interval ≈ 1000 / 13 ≈ 77
        let interval = budget.recommended_interval(100_000, 1000, 8);

        assert!(interval > 1);
        assert!(interval < 1000);
    }

    #[test]
    fn test_recommended_interval_zero_steps() {
        let budget = MemoryBudget::from_mb(100);
        assert_eq!(budget.recommended_interval(1000, 0, 8), 1);
    }

    #[test]
    fn test_recommended_interval_zero_paths() {
        let budget = MemoryBudget::from_mb(100);
        assert_eq!(budget.recommended_interval(0, 500, 8), 1);
    }

    #[test]
    fn test_recommended_interval_very_small_budget() {
        // Budget smaller than one checkpoint
        let budget = MemoryBudget::new(100); // 100 bytes

        // 1000 paths * 8 bytes = 8000 bytes per checkpoint
        let interval = budget.recommended_interval(1000, 500, 8);

        // Should return n_steps since we can't fit any checkpoints
        assert_eq!(interval, 500);
    }

    // ========================================================================
    // Clone and Copy Tests
    // ========================================================================

    #[test]
    fn test_clone() {
        let budget1 = MemoryBudget::from_mb(100).with_warning_threshold(0.75);
        let budget2 = budget1;

        assert_eq!(budget1.max_bytes(), budget2.max_bytes());
        assert_eq!(budget1.warning_threshold(), budget2.warning_threshold());
    }

    #[test]
    fn test_debug() {
        let budget = MemoryBudget::from_mb(100);
        let debug_str = format!("{:?}", budget);
        assert!(debug_str.contains("MemoryBudget"));
    }
}

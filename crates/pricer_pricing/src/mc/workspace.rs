//! Pre-allocated workspace buffers for Monte Carlo simulation.
//!
//! This module provides [`PathWorkspace`], which manages pre-allocated buffers
//! for allocation-free simulation loops. Buffer hoisting is critical for Enzyme
//! optimisation.
//!
//! # Memory Layout
//!
//! All buffers use row-major contiguous layout for cache efficiency:
//! - `randoms`: n_paths × n_steps (random normal samples)
//! - `paths`: n_paths × (n_steps + 1) (price paths including initial spot)
//! - `payoffs`: n_paths (terminal payoff values)
//!
//! # Enzyme Compatibility
//!
//! Buffer hoisting (allocating outside the simulation loop) enables Enzyme
//! to better analyse and optimise the inner loop for gradient computation.

/// Pre-allocated workspace for Monte Carlo simulation.
///
/// Manages memory buffers for random samples, price paths, and payoff values.
/// Designed for reuse across multiple pricing calls without reallocation.
///
/// # Buffer Hoisting
///
/// All allocations occur in [`ensure_capacity`](Self::ensure_capacity).
/// The simulation loop operates on slices without any heap allocation,
/// which is essential for Enzyme LLVM-level optimisation.
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::PathWorkspace;
///
/// let mut workspace = PathWorkspace::new(1000, 252);
///
/// // Fill random buffer externally
/// let randoms = workspace.randoms_mut();
/// // ... fill with random normals
///
/// // Access paths after generation
/// let paths = workspace.paths();
/// ```
pub struct PathWorkspace {
    /// Random normal samples (n_paths × n_steps).
    randoms: Vec<f64>,
    /// Price paths (n_paths × (n_steps + 1)).
    paths: Vec<f64>,
    /// Payoff values per path (n_paths).
    payoffs: Vec<f64>,
    /// Current capacity for paths dimension.
    capacity_paths: usize,
    /// Current capacity for steps dimension.
    capacity_steps: usize,
    /// Logical size for paths dimension.
    size_paths: usize,
    /// Logical size for steps dimension.
    size_steps: usize,
}

impl PathWorkspace {
    /// Creates a new workspace with the specified initial capacity.
    ///
    /// # Arguments
    ///
    /// * `n_paths` - Initial capacity for number of paths
    /// * `n_steps` - Initial capacity for number of steps
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::PathWorkspace;
    ///
    /// let workspace = PathWorkspace::new(10_000, 252);
    /// ```
    pub fn new(n_paths: usize, n_steps: usize) -> Self {
        let randoms_size = n_paths * n_steps;
        let paths_size = n_paths * (n_steps + 1);

        Self {
            randoms: vec![0.0; randoms_size],
            paths: vec![0.0; paths_size],
            payoffs: vec![0.0; n_paths],
            capacity_paths: n_paths,
            capacity_steps: n_steps,
            size_paths: n_paths,
            size_steps: n_steps,
        }
    }

    /// Ensures workspace has sufficient capacity for the given dimensions.
    ///
    /// Grows buffers if necessary using a doubling strategy. Never shrinks
    /// to avoid repeated allocations for varying simulation sizes.
    ///
    /// # Arguments
    ///
    /// * `n_paths` - Required number of paths
    /// * `n_steps` - Required number of steps
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::PathWorkspace;
    ///
    /// let mut workspace = PathWorkspace::new(100, 10);
    /// workspace.ensure_capacity(1000, 100);
    /// // Buffers now have capacity for 1000 paths × 100 steps
    /// ```
    pub fn ensure_capacity(&mut self, n_paths: usize, n_steps: usize) {
        let needs_growth = n_paths > self.capacity_paths || n_steps > self.capacity_steps;

        if needs_growth {
            // Use doubling strategy for amortised O(1) growth
            let new_capacity_paths = n_paths.max(self.capacity_paths * 2);
            let new_capacity_steps = n_steps.max(self.capacity_steps * 2);

            let randoms_size = new_capacity_paths * new_capacity_steps;
            let paths_size = new_capacity_paths * (new_capacity_steps + 1);

            self.randoms.resize(randoms_size, 0.0);
            self.paths.resize(paths_size, 0.0);
            self.payoffs.resize(new_capacity_paths, 0.0);

            self.capacity_paths = new_capacity_paths;
            self.capacity_steps = new_capacity_steps;
        }

        self.size_paths = n_paths;
        self.size_steps = n_steps;
    }

    /// Resets workspace state without deallocating buffers.
    ///
    /// Clears logical size but retains capacity for efficient reuse.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::PathWorkspace;
    ///
    /// let mut workspace = PathWorkspace::new(1000, 100);
    /// workspace.reset();
    /// // Capacity retained, ready for reuse
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.size_paths = 0;
        self.size_steps = 0;
    }

    /// Fast reset that preserves both capacity and logical size.
    ///
    /// Unlike [`reset`](Self::reset), this method does not modify sizes.
    /// It only ensures the workspace is ready for immediate reuse by
    /// avoiding any initialization overhead.
    ///
    /// This is the fastest reset option when dimensions remain constant
    /// across simulation runs, achieving true zero-allocation behavior.
    ///
    /// # Performance
    ///
    /// - `reset_fast`: O(1) - No memory operations
    /// - `reset`: O(1) - Clears size tracking
    /// - `ensure_capacity`: O(n) when growing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::mc::PathWorkspace;
    ///
    /// let mut workspace = PathWorkspace::new(1000, 100);
    ///
    /// // Run simulation loop
    /// for _ in 0..1000 {
    ///     workspace.reset_fast();
    ///     // ... simulation work using same dimensions ...
    /// }
    /// ```
    #[inline]
    pub fn reset_fast(&mut self) {
        // Intentionally empty - capacity and size are preserved
        // This serves as documentation and a clear API entry point
        // for zero-allocation simulation loops
    }

    /// Returns total memory used by all buffers in bytes.
    ///
    /// Useful for monitoring memory consumption and pool statistics.
    #[inline]
    pub fn memory_usage(&self) -> usize {
        (self.randoms.capacity() + self.paths.capacity() + self.payoffs.capacity())
            * std::mem::size_of::<f64>()
    }

    /// Returns current path capacity.
    #[inline]
    pub fn capacity_paths(&self) -> usize {
        self.capacity_paths
    }

    /// Returns current step capacity.
    #[inline]
    pub fn capacity_steps(&self) -> usize {
        self.capacity_steps
    }

    /// Returns logical path size.
    #[inline]
    pub fn size_paths(&self) -> usize {
        self.size_paths
    }

    /// Returns logical step size.
    #[inline]
    pub fn size_steps(&self) -> usize {
        self.size_steps
    }

    /// Returns mutable slice of random buffer for filling.
    ///
    /// Returns slice of size `n_paths × n_steps` based on current logical size.
    ///
    /// # Panics
    ///
    /// Panics if logical size exceeds capacity (programming error).
    #[inline]
    pub fn randoms_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * self.size_steps;
        debug_assert!(len <= self.randoms.len());
        &mut self.randoms[..len]
    }

    /// Returns slice of random buffer.
    #[inline]
    pub fn randoms(&self) -> &[f64] {
        let len = self.size_paths * self.size_steps;
        &self.randoms[..len]
    }

    /// Returns slice of price paths.
    ///
    /// Returns slice of size `n_paths × (n_steps + 1)` based on current logical size.
    #[inline]
    pub fn paths(&self) -> &[f64] {
        let len = self.size_paths * (self.size_steps + 1);
        &self.paths[..len]
    }

    /// Returns mutable slice of price paths.
    #[inline]
    pub fn paths_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * (self.size_steps + 1);
        &mut self.paths[..len]
    }

    /// Returns slice of payoff values.
    #[inline]
    pub fn payoffs(&self) -> &[f64] {
        &self.payoffs[..self.size_paths]
    }

    /// Returns mutable slice of payoff values.
    #[inline]
    pub fn payoffs_mut(&mut self) -> &mut [f64] {
        &mut self.payoffs[..self.size_paths]
    }

    /// Returns immutable slice of paths and mutable slice of payoffs.
    #[inline]
    pub fn paths_and_payoffs_mut(&mut self) -> (&[f64], &mut [f64]) {
        let paths_len = self.size_paths * (self.size_steps + 1);
        (
            &self.paths[..paths_len],
            &mut self.payoffs[..self.size_paths],
        )
    }

    /// Returns mutable slice of paths and immutable slice of randoms.
    #[inline]
    pub fn paths_mut_and_randoms(&mut self) -> (&mut [f64], &[f64]) {
        let randoms_len = self.size_paths * self.size_steps;
        let paths_len = self.size_paths * (self.size_steps + 1);
        (&mut self.paths[..paths_len], &self.randoms[..randoms_len])
    }

    /// Returns the index into the paths buffer for a specific path and step.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index (0-based)
    /// * `step_idx` - Step index (0-based, where 0 is initial spot)
    ///
    /// # Returns
    ///
    /// Linear index into the paths buffer.
    #[inline]
    pub fn path_index(&self, path_idx: usize, step_idx: usize) -> usize {
        path_idx * (self.size_steps + 1) + step_idx
    }

    /// Returns the index into the randoms buffer for a specific path and step.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index (0-based)
    /// * `step_idx` - Step index (0-based)
    ///
    /// # Returns
    ///
    /// Linear index into the randoms buffer.
    #[inline]
    pub fn random_index(&self, path_idx: usize, step_idx: usize) -> usize {
        path_idx * self.size_steps + step_idx
    }
}

impl Default for PathWorkspace {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let ws = PathWorkspace::new(100, 10);
        assert_eq!(ws.capacity_paths(), 100);
        assert_eq!(ws.capacity_steps(), 10);
        assert_eq!(ws.size_paths(), 100);
        assert_eq!(ws.size_steps(), 10);
    }

    #[test]
    fn test_workspace_buffer_sizes() {
        let ws = PathWorkspace::new(100, 10);
        assert_eq!(ws.randoms().len(), 100 * 10);
        assert_eq!(ws.paths().len(), 100 * 11); // n_steps + 1
        assert_eq!(ws.payoffs().len(), 100);
    }

    #[test]
    fn test_workspace_ensure_capacity_growth() {
        let mut ws = PathWorkspace::new(100, 10);
        ws.ensure_capacity(200, 20);

        assert!(ws.capacity_paths() >= 200);
        assert!(ws.capacity_steps() >= 20);
        assert_eq!(ws.size_paths(), 200);
        assert_eq!(ws.size_steps(), 20);
    }

    #[test]
    fn test_workspace_ensure_capacity_no_shrink() {
        let mut ws = PathWorkspace::new(200, 20);
        let cap_paths = ws.capacity_paths();
        let cap_steps = ws.capacity_steps();

        ws.ensure_capacity(100, 10);

        assert_eq!(ws.capacity_paths(), cap_paths);
        assert_eq!(ws.capacity_steps(), cap_steps);
        assert_eq!(ws.size_paths(), 100);
        assert_eq!(ws.size_steps(), 10);
    }

    #[test]
    fn test_workspace_reset() {
        let mut ws = PathWorkspace::new(100, 10);
        let cap_paths = ws.capacity_paths();
        let cap_steps = ws.capacity_steps();

        ws.reset();

        assert_eq!(ws.capacity_paths(), cap_paths);
        assert_eq!(ws.capacity_steps(), cap_steps);
        assert_eq!(ws.size_paths(), 0);
        assert_eq!(ws.size_steps(), 0);
    }

    #[test]
    fn test_workspace_indexing() {
        let ws = PathWorkspace::new(10, 5);

        // Path 0, Step 0
        assert_eq!(ws.path_index(0, 0), 0);
        // Path 0, Step 5 (terminal)
        assert_eq!(ws.path_index(0, 5), 5);
        // Path 1, Step 0
        assert_eq!(ws.path_index(1, 0), 6);

        // Random indexing (no +1 for steps)
        assert_eq!(ws.random_index(0, 0), 0);
        assert_eq!(ws.random_index(0, 4), 4);
        assert_eq!(ws.random_index(1, 0), 5);
    }

    #[test]
    fn test_workspace_mutable_access() {
        let mut ws = PathWorkspace::new(10, 5);

        // Modify randoms
        ws.randoms_mut()[0] = 1.0;
        assert_eq!(ws.randoms()[0], 1.0);

        // Modify paths
        ws.paths_mut()[0] = 100.0;
        assert_eq!(ws.paths()[0], 100.0);

        // Modify payoffs
        ws.payoffs_mut()[0] = 10.0;
        assert_eq!(ws.payoffs()[0], 10.0);
    }

    #[test]
    fn test_workspace_default() {
        let ws = PathWorkspace::default();
        assert_eq!(ws.capacity_paths(), 0);
        assert_eq!(ws.capacity_steps(), 0);
    }

    #[test]
    fn test_workspace_reset_fast() {
        let mut ws = PathWorkspace::new(100, 10);
        let size_paths = ws.size_paths();
        let size_steps = ws.size_steps();
        let cap_paths = ws.capacity_paths();
        let cap_steps = ws.capacity_steps();

        // Modify buffers
        ws.randoms_mut()[0] = 42.0;
        ws.paths_mut()[0] = 100.0;

        // reset_fast should preserve everything
        ws.reset_fast();

        assert_eq!(ws.size_paths(), size_paths);
        assert_eq!(ws.size_steps(), size_steps);
        assert_eq!(ws.capacity_paths(), cap_paths);
        assert_eq!(ws.capacity_steps(), cap_steps);

        // Data should be preserved (not zeroed)
        assert_eq!(ws.randoms()[0], 42.0);
        assert_eq!(ws.paths()[0], 100.0);
    }

    #[test]
    fn test_workspace_memory_usage() {
        let ws = PathWorkspace::new(100, 10);
        let mem = ws.memory_usage();

        // Expected: (100*10 + 100*11 + 100) * 8 bytes
        // = (1000 + 1100 + 100) * 8 = 2200 * 8 = 17600
        let expected = (100 * 10 + 100 * 11 + 100) * std::mem::size_of::<f64>();
        assert_eq!(mem, expected);
    }

    #[test]
    fn test_workspace_zero_allocation_loop() {
        let mut ws = PathWorkspace::new(100, 10);
        let initial_ptr = ws.randoms().as_ptr();

        // Simulate zero-allocation loop
        for i in 0..1000 {
            ws.reset_fast();
            // Write some data
            ws.randoms_mut()[0] = i as f64;
        }

        // Pointer should be the same (no reallocation)
        assert_eq!(ws.randoms().as_ptr(), initial_ptr);
    }
}

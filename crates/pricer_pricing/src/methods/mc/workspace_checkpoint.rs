//! Extended workspace with checkpoint and path-dependent option support.

use num_traits::Float;
use pricer_models::payoff::{PathObserver, PathObserverState};

use super::capacity::calculate_growth_capacity;

/// Extended workspace with checkpoint and path-dependent support.
#[derive(Clone)]
pub struct CheckpointWorkspace<T: Float> {
    /// Random normal samples (n_paths × n_steps).
    randoms: Vec<T>,
    /// Price paths (n_paths × (n_steps + 1)).
    paths: Vec<T>,
    /// Payoff values per path (n_paths).
    payoffs: Vec<T>,
    /// Path observers for streaming statistics (n_paths).
    observers: Vec<PathObserver<T>>,
    /// Current capacity for paths dimension.
    capacity_paths: usize,
    /// Current capacity for steps dimension.
    capacity_steps: usize,
    /// Logical size for paths dimension.
    size_paths: usize,
    /// Logical size for steps dimension.
    size_steps: usize,
}

impl<T: Float> CheckpointWorkspace<T> {
    /// Creates a new checkpoint-enabled workspace with the specified initial
    pub fn new(n_paths: usize, n_steps: usize) -> Self {
        let randoms_size = n_paths * n_steps;
        let paths_size = n_paths * (n_steps + 1);

        Self {
            randoms: vec![T::zero(); randoms_size],
            paths: vec![T::zero(); paths_size],
            payoffs: vec![T::zero(); n_paths],
            observers: (0..n_paths).map(|_| PathObserver::new()).collect(),
            capacity_paths: n_paths,
            capacity_steps: n_steps,
            size_paths: n_paths,
            size_steps: n_steps,
        }
    }

    /// Ensures workspace has sufficient capacity for the given dimensions.
    pub fn ensure_capacity(&mut self, n_paths: usize, n_steps: usize) {
        let needs_growth = n_paths > self.capacity_paths || n_steps > self.capacity_steps;

        if needs_growth {
            let new_capacity_paths = calculate_growth_capacity(self.capacity_paths, n_paths);
            let new_capacity_steps = calculate_growth_capacity(self.capacity_steps, n_steps);

            let randoms_size = new_capacity_paths * new_capacity_steps;
            let paths_size = new_capacity_paths * (new_capacity_steps + 1);

            self.randoms.resize(randoms_size, T::zero());
            self.paths.resize(paths_size, T::zero());
            self.payoffs.resize(new_capacity_paths, T::zero());

            while self.observers.len() < new_capacity_paths {
                self.observers.push(PathObserver::new());
            }

            self.capacity_paths = new_capacity_paths;
            self.capacity_steps = new_capacity_steps;
        }

        self.size_paths = n_paths;
        self.size_steps = n_steps;
    }

    /// Resets logical size without deallocating buffers.
    #[inline]
    pub fn reset(&mut self) {
        self.size_paths = 0;
        self.size_steps = 0;
    }

    /// Resets all observers to their initial state.
    #[inline]
    pub fn reset_observers(&mut self) {
        for observer in &mut self.observers[..self.size_paths] {
            observer.reset();
        }
    }

    /// Clears all state for safe reuse.
    pub fn clear_all(&mut self) {
        let randoms_len = self.size_paths * self.size_steps;
        for i in 0..randoms_len {
            self.randoms[i] = T::zero();
        }

        let paths_len = self.size_paths * (self.size_steps + 1);
        for i in 0..paths_len {
            self.paths[i] = T::zero();
        }

        for i in 0..self.size_paths {
            self.payoffs[i] = T::zero();
        }

        self.reset_observers();
    }

    /// Returns current path capacity.
    #[inline]
    pub fn capacity_paths(&self) -> usize { self.capacity_paths }

    /// Returns current step capacity.
    #[inline]
    pub fn capacity_steps(&self) -> usize { self.capacity_steps }

    /// Returns logical path size.
    #[inline]
    pub fn size_paths(&self) -> usize { self.size_paths }

    /// Returns logical step size.
    #[inline]
    pub fn size_steps(&self) -> usize { self.size_steps }

    /// Returns mutable slice of random buffer for filling.
    #[inline]
    pub fn randoms_mut(&mut self) -> &mut [T] {
        let len = self.size_paths * self.size_steps;
        &mut self.randoms[..len]
    }

    /// Returns slice of random buffer.
    #[inline]
    pub fn randoms(&self) -> &[T] {
        let len = self.size_paths * self.size_steps;
        &self.randoms[..len]
    }

    /// Returns slice of price paths.
    #[inline]
    pub fn paths(&self) -> &[T] {
        let len = self.size_paths * (self.size_steps + 1);
        &self.paths[..len]
    }

    /// Returns mutable slice of price paths.
    #[inline]
    pub fn paths_mut(&mut self) -> &mut [T] {
        let len = self.size_paths * (self.size_steps + 1);
        &mut self.paths[..len]
    }

    /// Returns slice of payoff values.
    #[inline]
    pub fn payoffs(&self) -> &[T] { &self.payoffs[..self.size_paths] }

    /// Returns mutable slice of payoff values.
    #[inline]
    pub fn payoffs_mut(&mut self) -> &mut [T] { &mut self.payoffs[..self.size_paths] }

    /// Returns a reference to the observer for a specific path.
    #[inline]
    pub fn observer(&self, path_idx: usize) -> &PathObserver<T> {
        debug_assert!(path_idx < self.size_paths, "path_idx out of bounds");
        &self.observers[path_idx]
    }

    /// Returns a mutable reference to the observer for a specific path.
    #[inline]
    pub fn observer_mut(&mut self, path_idx: usize) -> &mut PathObserver<T> {
        debug_assert!(path_idx < self.size_paths, "path_idx out of bounds");
        &mut self.observers[path_idx]
    }

    /// Returns a slice of all observers.
    #[inline]
    pub fn observers(&self) -> &[PathObserver<T>] { &self.observers[..self.size_paths] }

    /// Returns a mutable slice of all observers.
    #[inline]
    pub fn observers_mut(&mut self) -> &mut [PathObserver<T>] {
        &mut self.observers[..self.size_paths]
    }

    /// Creates a snapshot of all observer states for checkpointing.
    pub fn snapshot_observers(&self) -> Vec<PathObserverState<T>> {
        self.observers[..self.size_paths]
            .iter()
            .map(|o| o.snapshot())
            .collect()
    }

    /// Restores all observers from a checkpointed state.
    pub fn restore_observers(&mut self, states: &[PathObserverState<T>]) {
        assert_eq!(
            states.len(),
            self.size_paths,
            "Observer state count mismatch: expected {}, got {}",
            self.size_paths,
            states.len()
        );

        for (observer, state) in self.observers[..self.size_paths].iter_mut().zip(states) {
            observer.restore(state);
        }
    }

    /// Returns the approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let randoms_bytes = self.randoms.len() * std::mem::size_of::<T>();
        let paths_bytes = self.paths.len() * std::mem::size_of::<T>();
        let payoffs_bytes = self.payoffs.len() * std::mem::size_of::<T>();
        let observers_bytes = self.observers.len() * std::mem::size_of::<PathObserver<T>>();

        randoms_bytes + paths_bytes + payoffs_bytes + observers_bytes
    }

    /// Returns the estimated state size per path for checkpoint calculations.
    pub fn state_size_per_path(&self) -> usize {
        std::mem::size_of::<T>() + std::mem::size_of::<PathObserverState<T>>()
    }

    /// Returns the index into the paths buffer for a specific path and step.
    #[inline]
    pub fn path_index(&self, path_idx: usize, step_idx: usize) -> usize {
        path_idx * (self.size_steps + 1) + step_idx
    }

    /// Returns the index into the randoms buffer for a specific path and step.
    #[inline]
    pub fn random_index(&self, path_idx: usize, step_idx: usize) -> usize {
        path_idx * self.size_steps + step_idx
    }
}

impl<T: Float> Default for CheckpointWorkspace<T> {
    fn default() -> Self { Self::new(0, 0) }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_workspace_new() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        assert_eq!(ws.capacity_paths(), 100);
        assert_eq!(ws.capacity_steps(), 10);
        assert_eq!(ws.size_paths(), 100);
        assert_eq!(ws.size_steps(), 10);
    }

    #[test]
    fn test_workspace_default() {
        let ws: CheckpointWorkspace<f64> = Default::default();
        assert_eq!(ws.capacity_paths(), 0);
        assert_eq!(ws.capacity_steps(), 0);
    }

    #[test]
    fn test_workspace_observers_initialised() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        assert_eq!(ws.observers().len(), 100);

        for observer in ws.observers() {
            assert_eq!(observer.count(), 0);
        }
    }

    #[test]
    fn test_workspace_buffer_sizes() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        assert_eq!(ws.randoms().len(), 100 * 10);
        assert_eq!(ws.paths().len(), 100 * 11);
        assert_eq!(ws.payoffs().len(), 100);
    }

    #[test]
    fn test_workspace_ensure_capacity_growth() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        ws.ensure_capacity(200, 20);

        assert!(ws.capacity_paths() >= 200);
        assert!(ws.capacity_steps() >= 20);
        assert_eq!(ws.size_paths(), 200);
        assert_eq!(ws.size_steps(), 20);

        assert!(ws.observers.len() >= 200);
    }

    #[test]
    fn test_workspace_ensure_capacity_no_shrink() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(200, 20);
        let cap_paths = ws.capacity_paths();
        let cap_steps = ws.capacity_steps();

        ws.ensure_capacity(100, 10);

        assert_eq!(ws.capacity_paths(), cap_paths);
        assert_eq!(ws.capacity_steps(), cap_steps);
        assert_eq!(ws.size_paths(), 100);
        assert_eq!(ws.size_steps(), 10);
    }

    #[test]
    fn test_workspace_observer_access() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(10, 5);

        ws.observer_mut(0).observe(100.0);
        ws.observer_mut(0).observe(110.0);

        assert_eq!(ws.observer(0).count(), 2);
        assert_relative_eq!(ws.observer(0).arithmetic_average(), 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_workspace_reset_observers() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(10, 5);

        ws.observer_mut(0).observe(100.0);
        ws.observer_mut(1).observe(200.0);
        ws.observer_mut(2).observe(300.0);

        assert_eq!(ws.observer(0).count(), 1);
        assert_eq!(ws.observer(1).count(), 1);

        ws.reset_observers();

        assert_eq!(ws.observer(0).count(), 0);
        assert_eq!(ws.observer(1).count(), 0);
        assert_eq!(ws.observer(2).count(), 0);
    }

    #[test]
    fn test_workspace_clear_all() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(10, 5);

        ws.randoms_mut()[0] = 1.0;
        ws.paths_mut()[0] = 100.0;
        ws.payoffs_mut()[0] = 10.0;
        ws.observer_mut(0).observe(100.0);

        ws.clear_all();

        assert_eq!(ws.randoms()[0], 0.0);
        assert_eq!(ws.paths()[0], 0.0);
        assert_eq!(ws.payoffs()[0], 0.0);
        assert_eq!(ws.observer(0).count(), 0);
    }

    #[test]
    fn test_workspace_clear_all_preserves_capacity() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        let cap_paths = ws.capacity_paths();
        let cap_steps = ws.capacity_steps();

        ws.clear_all();

        assert_eq!(ws.capacity_paths(), cap_paths);
        assert_eq!(ws.capacity_steps(), cap_steps);
    }

    #[test]
    fn test_workspace_snapshot_observers() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(3, 5);

        ws.observer_mut(0).observe(100.0);
        ws.observer_mut(0).observe(110.0);
        ws.observer_mut(1).observe(200.0);
        ws.observer_mut(2).observe(300.0);
        ws.observer_mut(2).observe(310.0);
        ws.observer_mut(2).observe(320.0);

        let states = ws.snapshot_observers();

        assert_eq!(states.len(), 3);
        assert_eq!(states[0].count, 2);
        assert_eq!(states[1].count, 1);
        assert_eq!(states[2].count, 3);
    }

    #[test]
    fn test_workspace_restore_observers() {
        let mut ws1: CheckpointWorkspace<f64> = CheckpointWorkspace::new(3, 5);
        let mut ws2: CheckpointWorkspace<f64> = CheckpointWorkspace::new(3, 5);

        ws1.observer_mut(0).observe(100.0);
        ws1.observer_mut(0).observe(110.0);
        ws1.observer_mut(1).observe(200.0);
        ws1.observer_mut(2).observe(300.0);

        let states = ws1.snapshot_observers();

        ws2.restore_observers(&states);

        assert_eq!(ws2.observer(0).count(), 2);
        assert_relative_eq!(ws2.observer(0).arithmetic_average(), 105.0, epsilon = 1e-10);
        assert_eq!(ws2.observer(1).count(), 1);
        assert_eq!(ws2.observer(2).count(), 1);
    }

    #[test]
    #[should_panic(expected = "Observer state count mismatch")]
    fn test_workspace_restore_observers_size_mismatch() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(5, 3);
        let states = vec![PathObserverState::<f64>::default(); 3];

        ws.restore_observers(&states);
    }

    #[test]
    fn test_workspace_memory_usage() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        let usage = ws.memory_usage();

        assert!(usage > 0);

        let expected_min = 100 * 10 * 8 + 100 * 11 * 8 + 100 * 8;
        assert!(usage >= expected_min);
    }

    #[test]
    fn test_workspace_state_size_per_path() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(100, 10);
        let size = ws.state_size_per_path();

        assert!(size >= std::mem::size_of::<f64>());
    }

    #[test]
    fn test_workspace_indexing() {
        let ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(10, 5);

        assert_eq!(ws.path_index(0, 0), 0);
        assert_eq!(ws.path_index(0, 5), 5);
        assert_eq!(ws.path_index(1, 0), 6);

        assert_eq!(ws.random_index(0, 0), 0);
        assert_eq!(ws.random_index(0, 4), 4);
        assert_eq!(ws.random_index(1, 0), 5);
    }

    #[test]
    fn test_workspace_mutable_access() {
        let mut ws: CheckpointWorkspace<f64> = CheckpointWorkspace::new(10, 5);

        ws.randoms_mut()[0] = 1.0;
        assert_eq!(ws.randoms()[0], 1.0);

        ws.paths_mut()[0] = 100.0;
        assert_eq!(ws.paths()[0], 100.0);

        ws.payoffs_mut()[0] = 10.0;
        assert_eq!(ws.payoffs()[0], 10.0);
    }

    #[test]
    fn test_workspace_clone() {
        let mut ws1: CheckpointWorkspace<f64> = CheckpointWorkspace::new(5, 3);
        ws1.observer_mut(0).observe(100.0);
        ws1.paths_mut()[0] = 100.0;

        let ws2 = ws1.clone();

        assert_eq!(ws2.size_paths(), ws1.size_paths());
        assert_eq!(ws2.observer(0).count(), 1);
        assert_eq!(ws2.paths()[0], 100.0);
    }

    #[test]
    fn test_workspace_f32() {
        let mut ws: CheckpointWorkspace<f32> = CheckpointWorkspace::new(10, 5);

        ws.randoms_mut()[0] = 1.0_f32;
        ws.observer_mut(0).observe(100.0_f32);

        assert!((ws.randoms()[0] - 1.0_f32).abs() < 1e-5);
        assert_eq!(ws.observer(0).count(), 1);
    }
}

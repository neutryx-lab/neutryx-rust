//! Time Step First workspace for cache-efficient Monte Carlo simulation.

use super::{
    aligned_buffer::AlignedPathBuffer, layout_config::PathLayout,
    workspace_trait::PathWorkspaceTrait,
};

/// Workspace with time-step-first memory layout.
pub struct TimeStepFirstWorkspace {
    /// Path values buffer: [num_steps + 1][num_paths]
    paths: AlignedPathBuffer<f64>,

    /// Random normal samples: [num_steps][num_paths]
    randoms: AlignedPathBuffer<f64>,

    /// Payoff values: [num_paths]
    payoffs: AlignedPathBuffer<f64>,

    /// Number of simulation paths.
    num_paths: usize,

    /// Number of time steps.
    num_steps: usize,

    /// Alignment in bytes.
    alignment: usize,
}

impl TimeStepFirstWorkspace {
    /// Default alignment for AVX-512 cache lines.
    pub const DEFAULT_ALIGNMENT: usize = 64;

    /// Creates a new workspace with the specified dimensions.
    pub fn new(num_paths: usize, num_steps: usize) -> Self {
        Self::with_alignment(num_paths, num_steps, Self::DEFAULT_ALIGNMENT)
    }

    /// Creates a new workspace with custom alignment.
    pub fn with_alignment(num_paths: usize, num_steps: usize, alignment: usize) -> Self {
        let paths_size = (num_steps + 1) * num_paths;
        let paths = AlignedPathBuffer::with_alignment(paths_size, alignment);

        let randoms_size = num_steps * num_paths;
        let randoms = AlignedPathBuffer::with_alignment(randoms_size, alignment);

        let payoffs = AlignedPathBuffer::with_alignment(num_paths, alignment);

        Self {
            paths,
            randoms,
            payoffs,
            num_paths,
            num_steps,
            alignment,
        }
    }

    /// Returns the alignment in bytes.
    #[inline]
    pub fn alignment(&self) -> usize { self.alignment }

    /// Calculates the buffer index for a given path and step.
    #[inline]
    fn path_index(&self, step_idx: usize, path_idx: usize) -> usize {
        step_idx * self.num_paths + path_idx
    }

    /// Calculates the buffer index for randoms.
    #[inline]
    fn random_index(&self, step_idx: usize, path_idx: usize) -> usize {
        step_idx * self.num_paths + path_idx
    }

    /// Returns a contiguous slice for all paths at a given step.
    #[inline]
    pub fn get_aligned_step_slice(&self, step_idx: usize) -> &[f64] {
        let start = self.path_index(step_idx, 0);
        let end = start + self.num_paths;
        &self.paths.as_slice()[start..end]
    }

    /// Returns a mutable contiguous slice for all paths at a given step.
    #[inline]
    pub fn get_aligned_step_slice_mut(&mut self, step_idx: usize) -> &mut [f64] {
        let start = self.path_index(step_idx, 0);
        let end = start + self.num_paths;
        &mut self.paths.as_mut_slice()[start..end]
    }

    /// Returns a contiguous slice for all randoms at a given step.
    #[inline]
    pub fn get_randoms_step_slice(&self, step_idx: usize) -> &[f64] {
        let start = self.random_index(step_idx, 0);
        let end = start + self.num_paths;
        &self.randoms.as_slice()[start..end]
    }

    /// Returns a mutable contiguous slice for all randoms at a given step.
    #[inline]
    pub fn get_randoms_step_slice_mut(&mut self, step_idx: usize) -> &mut [f64] {
        let start = self.random_index(step_idx, 0);
        let end = start + self.num_paths;
        &mut self.randoms.as_mut_slice()[start..end]
    }

    /// Ensures the workspace has sufficient capacity.
    pub fn ensure_capacity(&mut self, num_paths: usize, num_steps: usize) {
        let needs_resize = num_paths > self.num_paths || num_steps > self.num_steps;

        if needs_resize {
            let new_paths = num_paths.max(self.num_paths);
            let new_steps = num_steps.max(self.num_steps);

            self.paths.resize((new_steps + 1) * new_paths);
            self.randoms.resize(new_steps * new_paths);
            self.payoffs.resize(new_paths);

            self.num_paths = new_paths;
            self.num_steps = new_steps;
        }
    }

    /// Resets the workspace for reuse.
    pub fn reset(&mut self) {
        self.paths.clear();
        self.randoms.clear();
        self.payoffs.clear();
    }

    /// Fast reset that preserves data.
    #[inline]
    pub fn reset_fast(&mut self) {
    }
}

impl PathWorkspaceTrait for TimeStepFirstWorkspace {
    #[inline]
    fn num_paths(&self) -> usize { self.num_paths }

    #[inline]
    fn num_steps(&self) -> usize { self.num_steps }

    #[inline]
    fn layout(&self) -> PathLayout { PathLayout::TimeStepFirst }

    #[inline]
    fn get_path_value(&self, path_idx: usize, step_idx: usize) -> f64 {
        let idx = self.path_index(step_idx, path_idx);
        self.paths.as_slice()[idx]
    }

    #[inline]
    fn set_path_value(&mut self, path_idx: usize, step_idx: usize, value: f64) {
        let idx = self.path_index(step_idx, path_idx);
        self.paths.as_mut_slice()[idx] = value;
    }

    #[inline]
    fn get_step_slice(&self, step_idx: usize) -> Option<&[f64]> {
        Some(self.get_aligned_step_slice(step_idx))
    }

    #[inline]
    fn get_step_slice_mut(&mut self, step_idx: usize) -> Option<&mut [f64]> {
        Some(self.get_aligned_step_slice_mut(step_idx))
    }

    #[inline]
    fn get_path_slice(&self, _path_idx: usize) -> Option<&[f64]> {
        None
    }

    #[inline]
    fn get_path_slice_mut(&mut self, _path_idx: usize) -> Option<&mut [f64]> {
        None
    }

    fn clear(&mut self) {
        self.paths.clear();
        self.randoms.clear();
        self.payoffs.clear();
    }

    #[inline]
    fn memory_usage(&self) -> usize {
        self.paths.memory_usage() + self.randoms.memory_usage() + self.payoffs.memory_usage()
    }

    #[inline]
    fn randoms(&self) -> &[f64] { self.randoms.as_slice() }

    #[inline]
    fn randoms_mut(&mut self) -> &mut [f64] { self.randoms.as_mut_slice() }

    #[inline]
    fn payoffs(&self) -> &[f64] { self.payoffs.as_slice() }

    #[inline]
    fn payoffs_mut(&mut self) -> &mut [f64] { self.payoffs.as_mut_slice() }

    #[inline]
    fn paths(&self) -> &[f64] { self.paths.as_slice() }

    #[inline]
    fn paths_mut(&mut self) -> &mut [f64] { self.paths.as_mut_slice() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestep_first_workspace_new() {
        let ws = TimeStepFirstWorkspace::new(100, 10);
        assert_eq!(ws.num_paths(), 100);
        assert_eq!(ws.num_steps(), 10);
        assert_eq!(ws.alignment(), TimeStepFirstWorkspace::DEFAULT_ALIGNMENT);
    }

    #[test]
    fn test_timestep_first_workspace_with_alignment() {
        let ws = TimeStepFirstWorkspace::with_alignment(100, 10, 128);
        assert_eq!(ws.alignment(), 128);
    }

    #[test]
    fn test_timestep_first_workspace_layout() {
        let ws = TimeStepFirstWorkspace::new(100, 10);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
    }

    #[test]
    fn test_timestep_first_workspace_get_set_value() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(0, 5, 150.0);
        ws.set_path_value(9, 0, 200.0);
        ws.set_path_value(9, 5, 250.0);

        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(0, 5), 150.0);
        assert_eq!(ws.get_path_value(9, 0), 200.0);
        assert_eq!(ws.get_path_value(9, 5), 250.0);
    }

    #[test]
    fn test_timestep_first_workspace_step_slice() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        if let Some(step0) = ws.get_step_slice_mut(0) {
            for (i, val) in step0.iter_mut().enumerate() {
                *val = 100.0 + i as f64;
            }
        }

        for i in 0..10 {
            assert_eq!(ws.get_path_value(i, 0), 100.0 + i as f64);
        }

        let step0 = ws.get_step_slice(0).unwrap();
        assert_eq!(step0.len(), 10);
        assert_eq!(step0[0], 100.0);
        assert_eq!(step0[9], 109.0);
    }

    #[test]
    fn test_timestep_first_workspace_path_slice_returns_none() {
        let ws = TimeStepFirstWorkspace::new(10, 5);
        assert!(ws.get_path_slice(0).is_none());
    }

    #[test]
    fn test_timestep_first_workspace_index_calculation() {
        let ws = TimeStepFirstWorkspace::new(10, 5);

        assert_eq!(ws.path_index(0, 0), 0);
        assert_eq!(ws.path_index(0, 9), 9);
        assert_eq!(ws.path_index(1, 0), 10);
        assert_eq!(ws.path_index(5, 5), 55);
    }

    #[test]
    fn test_timestep_first_workspace_memory_layout() {
        let mut ws = TimeStepFirstWorkspace::new(4, 3);

        ws.set_path_value(0, 0, 0.0);
        ws.set_path_value(1, 0, 1.0);
        ws.set_path_value(2, 0, 2.0);
        ws.set_path_value(3, 0, 3.0);
        ws.set_path_value(0, 1, 10.0);
        ws.set_path_value(1, 1, 11.0);

        let step0 = ws.get_step_slice(0).unwrap();
        assert_eq!(step0, &[0.0, 1.0, 2.0, 3.0]);

        let step1 = ws.get_step_slice(1).unwrap();
        assert_eq!(step1, &[10.0, 11.0, 0.0, 0.0]);
    }

    #[test]
    fn test_timestep_first_workspace_randoms() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        let randoms = ws.randoms_mut();
        for (i, val) in randoms.iter_mut().enumerate() {
            *val = i as f64;
        }

        assert_eq!(ws.randoms().len(), 50);
        assert_eq!(ws.randoms()[0], 0.0);
        assert_eq!(ws.randoms()[49], 49.0);
    }

    #[test]
    fn test_timestep_first_workspace_randoms_step_slice() {
        let mut ws = TimeStepFirstWorkspace::new(4, 3);

        for step in 0..3 {
            let step_randoms = ws.get_randoms_step_slice_mut(step);
            for (path, val) in step_randoms.iter_mut().enumerate() {
                *val = (step * 10 + path) as f64;
            }
        }

        let step0_randoms = ws.get_randoms_step_slice(0);
        assert_eq!(step0_randoms, &[0.0, 1.0, 2.0, 3.0]);

        let step2_randoms = ws.get_randoms_step_slice(2);
        assert_eq!(step2_randoms, &[20.0, 21.0, 22.0, 23.0]);
    }

    #[test]
    fn test_timestep_first_workspace_payoffs() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        let payoffs = ws.payoffs_mut();
        for (i, val) in payoffs.iter_mut().enumerate() {
            *val = i as f64 * 10.0;
        }

        assert_eq!(ws.payoffs().len(), 10);
        assert_eq!(ws.payoffs()[0], 0.0);
        assert_eq!(ws.payoffs()[9], 90.0);
    }

    #[test]
    fn test_timestep_first_workspace_clear() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        ws.set_path_value(0, 0, 100.0);
        ws.randoms_mut()[0] = 1.0;
        ws.payoffs_mut()[0] = 42.0;

        ws.clear();

        assert_eq!(ws.get_path_value(0, 0), 0.0);
        assert_eq!(ws.randoms()[0], 0.0);
        assert_eq!(ws.payoffs()[0], 0.0);
    }

    #[test]
    fn test_timestep_first_workspace_memory_usage() {
        let ws = TimeStepFirstWorkspace::new(100, 10);
        let mem = ws.memory_usage();

        let expected = (100 * 11 + 100 * 10 + 100) * std::mem::size_of::<f64>();
        assert_eq!(mem, expected);
    }

    #[test]
    fn test_timestep_first_workspace_ensure_capacity() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);

        ws.set_path_value(0, 0, 100.0);

        ws.ensure_capacity(20, 10);

        assert_eq!(ws.num_paths(), 20);
        assert_eq!(ws.num_steps(), 10);
    }

    #[test]
    fn test_timestep_first_workspace_large() {
        let ws = TimeStepFirstWorkspace::new(100_000, 252);

        assert_eq!(ws.num_paths(), 100_000);
        assert_eq!(ws.num_steps(), 252);

        let mem_mb = ws.memory_usage() as f64 / (1024.0 * 1024.0);
        println!("Memory usage for 100k paths x 252 steps: {:.2} MB", mem_mb);
    }
}

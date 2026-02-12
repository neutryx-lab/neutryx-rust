//! Pre-allocated workspace buffers for Monte Carlo simulation.

use std::marker::PhantomData;

use super::{
    capacity::calculate_growth_capacity, layout_config::PathLayout,
    workspace_trait::PathWorkspaceTrait,
};

/// Index calculation strategy for workspace memory layout.
pub trait LayoutStrategy: Send + Sync + Clone + 'static {
    /// Computes the linear index into the paths buffer.
    fn path_index(
        path_idx: usize,
        step_idx: usize,
        num_paths: usize,
        num_steps_plus_1: usize,
    ) -> usize;

    /// Computes the linear index into the randoms buffer.
    fn random_index(path_idx: usize, step_idx: usize, num_paths: usize, num_steps: usize) -> usize;

    /// Returns the [`PathLayout`] tag for this strategy.
    fn layout() -> PathLayout;
}

/// Path-first layout: `paths[path_idx * (n_steps+1) + step_idx]`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PathFirst;

impl LayoutStrategy for PathFirst {
    #[inline]
    fn path_index(
        path_idx: usize,
        step_idx: usize,
        _num_paths: usize,
        num_steps_plus_1: usize,
    ) -> usize {
        path_idx * num_steps_plus_1 + step_idx
    }

    #[inline]
    fn random_index(
        path_idx: usize,
        step_idx: usize,
        _num_paths: usize,
        num_steps: usize,
    ) -> usize {
        path_idx * num_steps + step_idx
    }

    #[inline]
    fn layout() -> PathLayout { PathLayout::PathFirst }
}

/// Time-step-first layout: `paths[step_idx * num_paths + path_idx]`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeStepFirst;

impl LayoutStrategy for TimeStepFirst {
    #[inline]
    fn path_index(
        path_idx: usize,
        step_idx: usize,
        num_paths: usize,
        _num_steps_plus_1: usize,
    ) -> usize {
        step_idx * num_paths + path_idx
    }

    #[inline]
    fn random_index(
        path_idx: usize,
        step_idx: usize,
        num_paths: usize,
        _num_steps: usize,
    ) -> usize {
        step_idx * num_paths + path_idx
    }

    #[inline]
    fn layout() -> PathLayout { PathLayout::TimeStepFirst }
}

/// Pre-allocated workspace for Monte Carlo simulation.
pub struct Workspace<S: LayoutStrategy = PathFirst> {
    randoms: Vec<f64>,
    paths: Vec<f64>,
    payoffs: Vec<f64>,
    capacity_paths: usize,
    capacity_steps: usize,
    size_paths: usize,
    size_steps: usize,
    _strategy: PhantomData<S>,
}

/// Path-first workspace (default).
pub type PathWorkspace = Workspace<PathFirst>;

/// Time-step-first workspace for SIMD-friendly access.
pub type TimeStepFirstWorkspace = Workspace<TimeStepFirst>;

impl<S: LayoutStrategy> Workspace<S> {
    /// Creates a new workspace with the specified initial capacity.
    pub fn new(n_paths: usize, n_steps: usize) -> Self {
        Self {
            randoms: vec![0.0; n_paths * n_steps],
            paths: vec![0.0; n_paths * (n_steps + 1)],
            payoffs: vec![0.0; n_paths],
            capacity_paths: n_paths,
            capacity_steps: n_steps,
            size_paths: n_paths,
            size_steps: n_steps,
            _strategy: PhantomData,
        }
    }

    /// Ensures workspace has sufficient capacity (doubling strategy, never
    pub fn ensure_capacity(&mut self, n_paths: usize, n_steps: usize) {
        if n_paths > self.capacity_paths || n_steps > self.capacity_steps {
            let new_cap_paths = calculate_growth_capacity(self.capacity_paths, n_paths);
            let new_cap_steps = calculate_growth_capacity(self.capacity_steps, n_steps);

            self.randoms.resize(new_cap_paths * new_cap_steps, 0.0);
            self.paths.resize(new_cap_paths * (new_cap_steps + 1), 0.0);
            self.payoffs.resize(new_cap_paths, 0.0);

            self.capacity_paths = new_cap_paths;
            self.capacity_steps = new_cap_steps;
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

    /// O(1) reset preserving both capacity and logical size.
    #[inline]
    pub fn reset_fast(&mut self) {}

    /// Returns the memory layout.
    #[inline]
    pub fn layout(&self) -> PathLayout { S::layout() }

    /// Returns total memory used by all buffers in bytes.
    #[inline]
    pub fn memory_usage(&self) -> usize {
        (self.randoms.capacity() + self.paths.capacity() + self.payoffs.capacity())
            * std::mem::size_of::<f64>()
    }

    /// Returns path capacity.
    #[inline]
    pub fn capacity_paths(&self) -> usize { self.capacity_paths }
    /// Returns step capacity.
    #[inline]
    pub fn capacity_steps(&self) -> usize { self.capacity_steps }
    /// Returns current path count.
    #[inline]
    pub fn size_paths(&self) -> usize { self.size_paths }
    /// Returns current step count.
    #[inline]
    pub fn size_steps(&self) -> usize { self.size_steps }
    /// Returns the number of simulation paths.
    #[inline]
    pub fn num_paths(&self) -> usize { self.size_paths }
    /// Returns the number of time steps.
    #[inline]
    pub fn num_steps(&self) -> usize { self.size_steps }

    /// Returns the randoms buffer as a slice.
    #[inline]
    pub fn randoms(&self) -> &[f64] { &self.randoms[..self.size_paths * self.size_steps] }
    /// Returns the randoms buffer as a mutable slice.
    #[inline]
    pub fn randoms_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * self.size_steps;
        &mut self.randoms[..len]
    }
    /// Returns the paths buffer as a slice.
    #[inline]
    pub fn paths(&self) -> &[f64] { &self.paths[..self.size_paths * (self.size_steps + 1)] }
    /// Returns the paths buffer as a mutable slice.
    #[inline]
    pub fn paths_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * (self.size_steps + 1);
        &mut self.paths[..len]
    }
    /// Returns the payoffs buffer as a slice.
    #[inline]
    pub fn payoffs(&self) -> &[f64] { &self.payoffs[..self.size_paths] }
    /// Returns the payoffs buffer as a mutable slice.
    #[inline]
    pub fn payoffs_mut(&mut self) -> &mut [f64] { &mut self.payoffs[..self.size_paths] }

    /// Returns the linear index into the paths buffer.
    #[inline]
    pub fn path_index(&self, path_idx: usize, step_idx: usize) -> usize {
        S::path_index(path_idx, step_idx, self.size_paths, self.size_steps + 1)
    }

    /// Returns the linear index into the randoms buffer.
    #[inline]
    pub fn random_index(&self, path_idx: usize, step_idx: usize) -> usize {
        S::random_index(path_idx, step_idx, self.size_paths, self.size_steps)
    }

    /// Gets a path value at the given position.
    #[inline]
    pub fn get_path_value(&self, path_idx: usize, step_idx: usize) -> f64 {
        self.paths[self.path_index(path_idx, step_idx)]
    }

    /// Sets a path value at the given position.
    #[inline]
    pub fn set_path_value(&mut self, path_idx: usize, step_idx: usize, value: f64) {
        let idx = self.path_index(path_idx, step_idx);
        self.paths[idx] = value;
    }

    /// Zeroes all buffer contents.
    pub fn clear(&mut self) {
        let r_len = self.size_paths * self.size_steps;
        self.randoms[..r_len].fill(0.0);
        let p_len = self.size_paths * (self.size_steps + 1);
        self.paths[..p_len].fill(0.0);
        self.payoffs[..self.size_paths].fill(0.0);
    }

    /// Returns a contiguous slice for all paths at a given step,
    #[inline]
    pub fn get_step_slice(&self, step_idx: usize) -> Option<&[f64]> {
        if S::layout() == PathLayout::TimeStepFirst {
            let start = step_idx * self.size_paths;
            Some(&self.paths[start..start + self.size_paths])
        } else {
            None
        }
    }

    /// Mutable variant of [`get_step_slice`](Self::get_step_slice).
    #[inline]
    pub fn get_step_slice_mut(&mut self, step_idx: usize) -> Option<&mut [f64]> {
        if S::layout() == PathLayout::TimeStepFirst {
            let start = step_idx * self.size_paths;
            Some(&mut self.paths[start..start + self.size_paths])
        } else {
            None
        }
    }

    /// Returns a contiguous slice for a single path,
    #[inline]
    pub fn get_path_slice(&self, path_idx: usize) -> Option<&[f64]> {
        if S::layout() == PathLayout::PathFirst {
            let start = path_idx * (self.size_steps + 1);
            Some(&self.paths[start..start + self.size_steps + 1])
        } else {
            None
        }
    }
}

impl<S: LayoutStrategy> PathWorkspaceTrait for Workspace<S> {
    #[inline]
    fn num_paths(&self) -> usize { self.size_paths }
    #[inline]
    fn num_steps(&self) -> usize { self.size_steps }
    #[inline]
    fn layout(&self) -> PathLayout { S::layout() }

    #[inline]
    fn get_path_value(&self, path_idx: usize, step_idx: usize) -> f64 {
        self.paths[self.path_index(path_idx, step_idx)]
    }

    #[inline]
    fn set_path_value(&mut self, path_idx: usize, step_idx: usize, value: f64) {
        let idx = self.path_index(path_idx, step_idx);
        self.paths[idx] = value;
    }

    #[inline]
    fn get_step_slice(&self, step_idx: usize) -> Option<&[f64]> {
        if S::layout() == PathLayout::TimeStepFirst {
            let start = step_idx * self.size_paths;
            Some(&self.paths[start..start + self.size_paths])
        } else {
            None
        }
    }

    #[inline]
    fn get_step_slice_mut(&mut self, step_idx: usize) -> Option<&mut [f64]> {
        if S::layout() == PathLayout::TimeStepFirst {
            let start = step_idx * self.size_paths;
            Some(&mut self.paths[start..start + self.size_paths])
        } else {
            None
        }
    }

    #[inline]
    fn get_path_slice(&self, path_idx: usize) -> Option<&[f64]> {
        if S::layout() == PathLayout::PathFirst {
            let start = path_idx * (self.size_steps + 1);
            Some(&self.paths[start..start + self.size_steps + 1])
        } else {
            None
        }
    }

    #[inline]
    fn get_path_slice_mut(&mut self, path_idx: usize) -> Option<&mut [f64]> {
        if S::layout() == PathLayout::PathFirst {
            let start = path_idx * (self.size_steps + 1);
            Some(&mut self.paths[start..start + self.size_steps + 1])
        } else {
            None
        }
    }

    fn clear(&mut self) {
        let r_len = self.size_paths * self.size_steps;
        self.randoms[..r_len].fill(0.0);
        let p_len = self.size_paths * (self.size_steps + 1);
        self.paths[..p_len].fill(0.0);
        self.payoffs[..self.size_paths].fill(0.0);
    }

    #[inline]
    fn memory_usage(&self) -> usize {
        (self.randoms.capacity() + self.paths.capacity() + self.payoffs.capacity())
            * std::mem::size_of::<f64>()
    }

    #[inline]
    fn randoms(&self) -> &[f64] { &self.randoms[..self.size_paths * self.size_steps] }
    #[inline]
    fn randoms_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * self.size_steps;
        &mut self.randoms[..len]
    }
    #[inline]
    fn payoffs(&self) -> &[f64] { &self.payoffs[..self.size_paths] }
    #[inline]
    fn payoffs_mut(&mut self) -> &mut [f64] { &mut self.payoffs[..self.size_paths] }
    #[inline]
    fn paths(&self) -> &[f64] { &self.paths[..self.size_paths * (self.size_steps + 1)] }
    #[inline]
    fn paths_mut(&mut self) -> &mut [f64] {
        let len = self.size_paths * (self.size_steps + 1);
        &mut self.paths[..len]
    }
}

impl Workspace<PathFirst> {
    /// Returns immutable paths and mutable payoffs (split borrow).
    #[inline]
    pub fn paths_and_payoffs_mut(&mut self) -> (&[f64], &mut [f64]) {
        let paths_len = self.size_paths * (self.size_steps + 1);
        (
            &self.paths[..paths_len],
            &mut self.payoffs[..self.size_paths],
        )
    }

    /// Returns mutable paths and immutable randoms (split borrow).
    #[inline]
    pub fn paths_mut_and_randoms(&mut self) -> (&mut [f64], &[f64]) {
        let randoms_len = self.size_paths * self.size_steps;
        let paths_len = self.size_paths * (self.size_steps + 1);
        (&mut self.paths[..paths_len], &self.randoms[..randoms_len])
    }
}

impl<S: LayoutStrategy> Default for Workspace<S> {
    fn default() -> Self { Self::new(0, 0) }
}

/// Runtime-selected workspace layout.
#[allow(missing_docs)]
pub enum WorkspaceEnum {
    /// Path-first layout.
    PathFirst(PathWorkspace),
    /// Time-step-first layout.
    TimeStepFirst(TimeStepFirstWorkspace),
}

#[allow(missing_docs)]
impl WorkspaceEnum {
    /// Creates a workspace with the specified layout.
    pub fn new(layout: PathLayout, num_paths: usize, num_steps: usize) -> Self {
        match layout {
            PathLayout::PathFirst => Self::PathFirst(PathWorkspace::new(num_paths, num_steps)),
            PathLayout::TimeStepFirst => {
                Self::TimeStepFirst(TimeStepFirstWorkspace::new(num_paths, num_steps))
            }
        }
    }

    #[inline]
    pub fn path_first(num_paths: usize, num_steps: usize) -> Self {
        Self::PathFirst(PathWorkspace::new(num_paths, num_steps))
    }
    #[inline]
    pub fn timestep_first(num_paths: usize, num_steps: usize) -> Self {
        Self::TimeStepFirst(TimeStepFirstWorkspace::new(num_paths, num_steps))
    }

    #[inline]
    pub fn as_path_first(&self) -> Option<&PathWorkspace> {
        match self {
            Self::PathFirst(ws) => Some(ws),
            _ => None,
        }
    }
    #[inline]
    pub fn as_path_first_mut(&mut self) -> Option<&mut PathWorkspace> {
        match self {
            Self::PathFirst(ws) => Some(ws),
            _ => None,
        }
    }
    #[inline]
    pub fn as_timestep_first(&self) -> Option<&TimeStepFirstWorkspace> {
        match self {
            Self::TimeStepFirst(ws) => Some(ws),
            _ => None,
        }
    }

    #[inline]
    pub fn num_paths(&self) -> usize {
        match self {
            Self::PathFirst(w) => w.num_paths(),
            Self::TimeStepFirst(w) => w.num_paths(),
        }
    }
    #[inline]
    pub fn num_steps(&self) -> usize {
        match self {
            Self::PathFirst(w) => w.num_steps(),
            Self::TimeStepFirst(w) => w.num_steps(),
        }
    }
    #[inline]
    pub fn layout(&self) -> PathLayout {
        match self {
            Self::PathFirst(w) => w.layout(),
            Self::TimeStepFirst(w) => w.layout(),
        }
    }
    pub fn ensure_capacity(&mut self, np: usize, ns: usize) {
        match self {
            Self::PathFirst(w) => w.ensure_capacity(np, ns),
            Self::TimeStepFirst(w) => w.ensure_capacity(np, ns),
        }
    }
    pub fn reset(&mut self) {
        match self {
            Self::PathFirst(w) => w.reset(),
            Self::TimeStepFirst(w) => w.reset(),
        }
    }
    #[inline]
    pub fn memory_usage(&self) -> usize {
        match self {
            Self::PathFirst(w) => w.memory_usage(),
            Self::TimeStepFirst(w) => w.memory_usage(),
        }
    }
    #[inline]
    pub fn randoms(&self) -> &[f64] {
        match self {
            Self::PathFirst(w) => w.randoms(),
            Self::TimeStepFirst(w) => w.randoms(),
        }
    }
    #[inline]
    pub fn randoms_mut(&mut self) -> &mut [f64] {
        match self {
            Self::PathFirst(w) => w.randoms_mut(),
            Self::TimeStepFirst(w) => w.randoms_mut(),
        }
    }
    #[inline]
    pub fn paths(&self) -> &[f64] {
        match self {
            Self::PathFirst(w) => w.paths(),
            Self::TimeStepFirst(w) => w.paths(),
        }
    }
    #[inline]
    pub fn paths_mut(&mut self) -> &mut [f64] {
        match self {
            Self::PathFirst(w) => w.paths_mut(),
            Self::TimeStepFirst(w) => w.paths_mut(),
        }
    }
    #[inline]
    pub fn payoffs(&self) -> &[f64] {
        match self {
            Self::PathFirst(w) => w.payoffs(),
            Self::TimeStepFirst(w) => w.payoffs(),
        }
    }
    #[inline]
    pub fn payoffs_mut(&mut self) -> &mut [f64] {
        match self {
            Self::PathFirst(w) => w.payoffs_mut(),
            Self::TimeStepFirst(w) => w.payoffs_mut(),
        }
    }
    #[inline]
    pub fn get_path_value(&self, path_idx: usize, step_idx: usize) -> f64 {
        match self {
            Self::PathFirst(w) => w.get_path_value(path_idx, step_idx),
            Self::TimeStepFirst(w) => w.get_path_value(path_idx, step_idx),
        }
    }
    #[inline]
    pub fn set_path_value(&mut self, path_idx: usize, step_idx: usize, value: f64) {
        match self {
            Self::PathFirst(w) => w.set_path_value(path_idx, step_idx, value),
            Self::TimeStepFirst(w) => w.set_path_value(path_idx, step_idx, value),
        }
    }
    #[inline]
    pub fn get_step_slice(&self, step_idx: usize) -> Option<&[f64]> {
        match self {
            Self::PathFirst(w) => w.get_step_slice(step_idx),
            Self::TimeStepFirst(w) => w.get_step_slice(step_idx),
        }
    }
    #[inline]
    pub fn get_step_slice_mut(&mut self, step_idx: usize) -> Option<&mut [f64]> {
        match self {
            Self::PathFirst(w) => w.get_step_slice_mut(step_idx),
            Self::TimeStepFirst(w) => w.get_step_slice_mut(step_idx),
        }
    }
    #[inline]
    pub fn get_path_slice(&self, path_idx: usize) -> Option<&[f64]> {
        match self {
            Self::PathFirst(w) => w.get_path_slice(path_idx),
            Self::TimeStepFirst(w) => w.get_path_slice(path_idx),
        }
    }
    pub fn clear(&mut self) {
        match self {
            Self::PathFirst(w) => w.clear(),
            Self::TimeStepFirst(w) => w.clear(),
        }
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
        assert_eq!(ws.randoms().len(), 1000);
        assert_eq!(ws.paths().len(), 1100);
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
        let cap_p = ws.capacity_paths();
        let cap_s = ws.capacity_steps();
        ws.ensure_capacity(100, 10);
        assert_eq!(ws.capacity_paths(), cap_p);
        assert_eq!(ws.capacity_steps(), cap_s);
    }

    #[test]
    fn test_workspace_reset() {
        let mut ws = PathWorkspace::new(100, 10);
        ws.reset();
        assert_eq!(ws.size_paths(), 0);
        assert_eq!(ws.size_steps(), 0);
    }

    #[test]
    fn test_workspace_indexing_path_first() {
        let ws = PathWorkspace::new(10, 5);
        assert_eq!(ws.path_index(0, 0), 0);
        assert_eq!(ws.path_index(0, 5), 5);
        assert_eq!(ws.path_index(1, 0), 6);
        assert_eq!(ws.random_index(0, 0), 0);
        assert_eq!(ws.random_index(0, 4), 4);
        assert_eq!(ws.random_index(1, 0), 5);
    }

    #[test]
    fn test_workspace_mutable_access() {
        let mut ws = PathWorkspace::new(10, 5);
        ws.randoms_mut()[0] = 1.0;
        assert_eq!(ws.randoms()[0], 1.0);
        ws.paths_mut()[0] = 100.0;
        assert_eq!(ws.paths()[0], 100.0);
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
        ws.randoms_mut()[0] = 42.0;
        ws.reset_fast();
        assert_eq!(ws.size_paths(), 100);
        assert_eq!(ws.randoms()[0], 42.0);
    }

    #[test]
    fn test_workspace_memory_usage() {
        let ws = PathWorkspace::new(100, 10);
        let expected = (100 * 10 + 100 * 11 + 100) * std::mem::size_of::<f64>();
        assert_eq!(ws.memory_usage(), expected);
    }

    #[test]
    fn test_workspace_zero_allocation_loop() {
        let mut ws = PathWorkspace::new(100, 10);
        let ptr = ws.randoms().as_ptr();
        for i in 0..1000 {
            ws.reset_fast();
            ws.randoms_mut()[0] = i as f64;
        }
        assert_eq!(ws.randoms().as_ptr(), ptr);
    }

    #[test]
    fn test_get_set_path_value() {
        let mut ws = PathWorkspace::new(10, 5);
        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(0, 5, 150.0);
        ws.set_path_value(9, 0, 200.0);
        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(0, 5), 150.0);
        assert_eq!(ws.get_path_value(9, 0), 200.0);
    }

    #[test]
    fn test_path_first_step_slice_returns_none() {
        let ws = PathWorkspace::new(10, 5);
        assert!(ws.get_step_slice(0).is_none());
    }

    #[test]
    fn test_path_first_path_slice() {
        let mut ws = PathWorkspace::new(10, 5);
        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(0, 5, 150.0);
        let slice = ws.get_path_slice(0).unwrap();
        assert_eq!(slice.len(), 6);
        assert_eq!(slice[0], 100.0);
        assert_eq!(slice[5], 150.0);
    }

    #[test]
    fn test_timestep_first_layout() {
        let ws = TimeStepFirstWorkspace::new(100, 10);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
        assert_eq!(ws.num_paths(), 100);
        assert_eq!(ws.num_steps(), 10);
    }

    #[test]
    fn test_timestep_first_indexing() {
        let ws = TimeStepFirstWorkspace::new(10, 5);
        assert_eq!(ws.path_index(0, 0), 0);
        assert_eq!(ws.path_index(9, 0), 9);
        assert_eq!(ws.path_index(0, 1), 10);
        assert_eq!(ws.path_index(5, 5), 55);
    }

    #[test]
    fn test_timestep_first_step_slice() {
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
    }

    #[test]
    fn test_timestep_first_path_slice_returns_none() {
        let ws = TimeStepFirstWorkspace::new(10, 5);
        assert!(ws.get_path_slice(0).is_none());
    }

    #[test]
    fn test_timestep_first_get_set() {
        let mut ws = TimeStepFirstWorkspace::new(10, 5);
        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(9, 5, 250.0);
        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(9, 5), 250.0);
    }

    #[test]
    fn test_workspace_enum_path_first() {
        let ws = WorkspaceEnum::new(PathLayout::PathFirst, 100, 10);
        assert_eq!(ws.num_paths(), 100);
        assert_eq!(ws.layout(), PathLayout::PathFirst);
        assert!(ws.as_path_first().is_some());
    }

    #[test]
    fn test_workspace_enum_timestep_first() {
        let ws = WorkspaceEnum::new(PathLayout::TimeStepFirst, 100, 10);
        assert_eq!(ws.layout(), PathLayout::TimeStepFirst);
        assert!(ws.as_timestep_first().is_some());
    }

    #[test]
    fn test_workspace_enum_get_set() {
        let mut ws = WorkspaceEnum::timestep_first(10, 5);
        ws.set_path_value(0, 0, 100.0);
        ws.set_path_value(9, 5, 200.0);
        assert_eq!(ws.get_path_value(0, 0), 100.0);
        assert_eq!(ws.get_path_value(9, 5), 200.0);
    }

    #[test]
    fn test_workspace_enum_consistent_across_layouts() {
        let mut ws_pf = WorkspaceEnum::path_first(4, 3);
        let mut ws_tsf = WorkspaceEnum::timestep_first(4, 3);
        for path in 0..4 {
            for step in 0..4 {
                let val = (path * 10 + step) as f64;
                ws_pf.set_path_value(path, step, val);
                ws_tsf.set_path_value(path, step, val);
            }
        }
        for path in 0..4 {
            for step in 0..4 {
                let expected = (path * 10 + step) as f64;
                assert_eq!(ws_pf.get_path_value(path, step), expected);
                assert_eq!(ws_tsf.get_path_value(path, step), expected);
            }
        }
    }
}

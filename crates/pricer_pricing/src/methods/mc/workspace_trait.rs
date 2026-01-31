//! Workspace abstraction trait for Monte Carlo simulation.
//!
//! This module provides [`PathWorkspaceTrait`], an abstraction over different
//! memory layout implementations for path storage.
//!
//! # Implementations
//!
//! - [`PathWorkspace`]: Traditional `[path][step]` layout
//! - [`TimeStepFirstWorkspace`]: Optimised `[step][path]` layout
//!
//! # Design Rationale
//!
//! The trait uses associated types rather than generic parameters to enable
//! static dispatch via [`WorkspaceEnum`], avoiding trait object overhead
//! in hot paths while maintaining Enzyme AD compatibility.
//!
//! The `#[enum_dispatch]` attribute is used to auto-generate the trait
//! implementation for `WorkspaceEnum`, eliminating boilerplate match statements.

use enum_dispatch::enum_dispatch;

use super::{
    layout_config::PathLayout, workspace::PathWorkspace, workspace_enum::WorkspaceEnum,
    workspace_timestep_first::TimeStepFirstWorkspace,
};

/// Trait for workspace implementations providing path storage.
///
/// This trait abstracts over different memory layout strategies for storing
/// Monte Carlo simulation paths.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support parallel path generation.
///
/// # Invariants
///
/// - `num_paths()` and `num_steps()` return consistent values
/// - Index operations must be within bounds: `path_idx < num_paths()`,
///   `step_idx <= num_steps()` (step 0 is initial spot)
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::{PathWorkspace, PathWorkspaceTrait, PathLayout};
///
/// let mut workspace = PathWorkspace::new(1000, 100);
///
/// assert_eq!(workspace.num_paths(), 1000);
/// assert_eq!(workspace.num_steps(), 100);
/// assert_eq!(workspace.layout(), PathLayout::PathFirst);
///
/// // Set initial spot prices
/// for i in 0..workspace.num_paths() {
///     workspace.set_path_value(i, 0, 100.0);
/// }
/// ```
#[enum_dispatch]
pub trait PathWorkspaceTrait: Send + Sync {
    /// Returns the number of simulation paths.
    fn num_paths(&self) -> usize;

    /// Returns the number of time steps per path.
    ///
    /// Note: The paths buffer has `num_steps + 1` values per path,
    /// where index 0 is the initial spot price.
    fn num_steps(&self) -> usize;

    /// Returns the memory layout mode.
    fn layout(&self) -> PathLayout;

    /// Returns the path value at the given position.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index in [0, num_paths())
    /// * `step_idx` - Step index in [0, num_steps()] (0 is initial spot)
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds.
    fn get_path_value(&self, path_idx: usize, step_idx: usize) -> f64;

    /// Sets the path value at the given position.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index in [0, num_paths())
    /// * `step_idx` - Step index in [0, num_steps()]
    /// * `value` - Value to set
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds.
    fn set_path_value(&mut self, path_idx: usize, step_idx: usize, value: f64);

    /// Returns a slice of all path values at the given step.
    ///
    /// # Arguments
    ///
    /// * `step_idx` - Step index in [0, num_steps()]
    ///
    /// # Returns
    ///
    /// - `Some(&[f64])` for TimeStepFirst layout (contiguous step data)
    /// - `None` for PathFirst layout (non-contiguous step data)
    ///
    /// # Note
    ///
    /// This method is only efficient for TimeStepFirst layout where
    /// all path values at a step are contiguous in memory.
    fn get_step_slice(&self, step_idx: usize) -> Option<&[f64]>;

    /// Returns a mutable slice of all path values at the given step.
    ///
    /// # Arguments
    ///
    /// * `step_idx` - Step index in [0, num_steps()]
    ///
    /// # Returns
    ///
    /// - `Some(&mut [f64])` for TimeStepFirst layout
    /// - `None` for PathFirst layout
    fn get_step_slice_mut(&mut self, step_idx: usize) -> Option<&mut [f64]>;

    /// Returns a slice of all values for a single path.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index in [0, num_paths())
    ///
    /// # Returns
    ///
    /// - `Some(&[f64])` for PathFirst layout (contiguous path data)
    /// - `None` for TimeStepFirst layout (non-contiguous path data)
    fn get_path_slice(&self, path_idx: usize) -> Option<&[f64]>;

    /// Returns a mutable slice of all values for a single path.
    ///
    /// # Arguments
    ///
    /// * `path_idx` - Path index in [0, num_paths())
    ///
    /// # Returns
    ///
    /// - `Some(&mut [f64])` for PathFirst layout
    /// - `None` for TimeStepFirst layout
    fn get_path_slice_mut(&mut self, path_idx: usize) -> Option<&mut [f64]>;

    /// Clears all path data for reuse.
    ///
    /// Resets all values to zero without deallocating memory.
    fn clear(&mut self);

    /// Returns total memory usage in bytes.
    fn memory_usage(&self) -> usize;

    /// Returns the randoms buffer as a slice.
    fn randoms(&self) -> &[f64];

    /// Returns the randoms buffer as a mutable slice.
    fn randoms_mut(&mut self) -> &mut [f64];

    /// Returns the payoffs buffer as a slice.
    fn payoffs(&self) -> &[f64];

    /// Returns the payoffs buffer as a mutable slice.
    fn payoffs_mut(&mut self) -> &mut [f64];

    /// Returns the full paths buffer as a slice.
    fn paths(&self) -> &[f64];

    /// Returns the full paths buffer as a mutable slice.
    fn paths_mut(&mut self) -> &mut [f64];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::mc::PathWorkspace;

    #[test]
    fn test_path_workspace_trait_num_paths() {
        let workspace = PathWorkspace::new(1000, 100);
        assert_eq!(workspace.num_paths(), 1000);
    }

    #[test]
    fn test_path_workspace_trait_num_steps() {
        let workspace = PathWorkspace::new(1000, 100);
        assert_eq!(workspace.num_steps(), 100);
    }

    #[test]
    fn test_path_workspace_trait_layout() {
        let workspace = PathWorkspace::new(1000, 100);
        assert_eq!(workspace.layout(), PathLayout::PathFirst);
    }

    #[test]
    fn test_path_workspace_trait_get_set_value() {
        let mut workspace = PathWorkspace::new(10, 5);
        workspace.set_path_value(0, 0, 100.0);
        workspace.set_path_value(0, 5, 150.0);
        workspace.set_path_value(9, 0, 200.0);

        assert_eq!(workspace.get_path_value(0, 0), 100.0);
        assert_eq!(workspace.get_path_value(0, 5), 150.0);
        assert_eq!(workspace.get_path_value(9, 0), 200.0);
    }

    #[test]
    fn test_path_workspace_trait_get_step_slice_returns_none() {
        let workspace = PathWorkspace::new(10, 5);
        // PathFirst layout doesn't support step slices
        assert!(workspace.get_step_slice(0).is_none());
    }

    #[test]
    fn test_path_workspace_trait_get_path_slice() {
        let mut workspace = PathWorkspace::new(10, 5);
        workspace.set_path_value(0, 0, 100.0);
        workspace.set_path_value(0, 5, 150.0);

        let path_slice = workspace.get_path_slice(0);
        assert!(path_slice.is_some());
        let slice = path_slice.unwrap();
        assert_eq!(slice.len(), 6); // num_steps + 1
        assert_eq!(slice[0], 100.0);
        assert_eq!(slice[5], 150.0);
    }

    #[test]
    fn test_path_workspace_trait_clear() {
        let mut workspace = PathWorkspace::new(10, 5);
        workspace.set_path_value(0, 0, 100.0);
        workspace.set_path_value(5, 3, 200.0);

        workspace.clear();

        assert_eq!(workspace.get_path_value(0, 0), 0.0);
        assert_eq!(workspace.get_path_value(5, 3), 0.0);
    }

    #[test]
    fn test_path_workspace_trait_memory_usage() {
        let workspace = PathWorkspace::new(100, 10);
        let mem = workspace.memory_usage();
        // At least (100 * 10 + 100 * 11 + 100) * 8 bytes
        assert!(mem > 0);
    }

    #[test]
    fn test_path_workspace_trait_randoms() {
        let mut workspace = PathWorkspace::new(10, 5);
        let randoms = workspace.randoms_mut();
        randoms[0] = 1.0;
        assert_eq!(workspace.randoms()[0], 1.0);
    }

    #[test]
    fn test_path_workspace_trait_payoffs() {
        let mut workspace = PathWorkspace::new(10, 5);
        let payoffs = workspace.payoffs_mut();
        payoffs[0] = 42.0;
        assert_eq!(workspace.payoffs()[0], 42.0);
    }
}

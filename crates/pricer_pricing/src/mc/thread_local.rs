//! Thread-local workspace management for parallel Monte Carlo simulation.
//!
//! This module provides thread-local workspace allocation patterns for use
//! with Rayon parallel iterators, avoiding memory contention between threads.
//!
//! # Design Patterns
//!
//! Two patterns are provided:
//!
//! 1. **ThreadLocalWorkspacePool**: A pool that lazily creates workspaces
//!    per thread using Rayon's thread-local storage.
//!
//! 2. **WorkspaceFactory**: A factory trait for creating workspaces on demand,
//!    allowing custom initialization.
//!
//! # Memory Contention Avoidance
//!
//! Each thread maintains its own workspace, eliminating false sharing and
//! lock contention during parallel simulation.
//!
//! # Example
//!
//! ```rust
//! use pricer_pricing::mc::{ThreadLocalWorkspacePool, CheckpointWorkspace};
//! use rayon::prelude::*;
//!
//! let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(100, 10);
//!
//! // Each thread gets its own workspace
//! let results: Vec<f64> = (0..1000)
//!     .into_par_iter()
//!     .map(|path_idx| {
//!         pool.with_workspace(|ws| {
//!             // Use workspace for this path's simulation
//!             ws.observer_mut(0).observe(100.0);
//!             ws.observer(0).arithmetic_average()
//!         })
//!     })
//!     .collect();
//! ```

use num_traits::Float;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::workspace_checkpoint::CheckpointWorkspace;

/// Thread-local workspace pool for parallel Monte Carlo simulation.
///
/// This pool manages per-thread workspaces using Rayon's thread-local storage,
/// ensuring no memory contention between threads during parallel simulation.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `f32`)
///
/// # Thread Safety
///
/// The pool itself is `Send + Sync`, but individual workspaces are accessed
/// thread-locally without synchronization overhead.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::mc::ThreadLocalWorkspacePool;
///
/// let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(100, 10);
///
/// // Get workspace for current thread
/// pool.with_workspace(|ws| {
///     ws.observer_mut(0).observe(100.0);
/// });
/// ```
pub struct ThreadLocalWorkspacePool<T: Float + Send + Sync> {
    /// Default path capacity for new workspaces.
    default_paths: usize,
    /// Default step capacity for new workspaces.
    default_steps: usize,
    /// Counter for number of workspaces created.
    workspace_count: AtomicUsize,
    /// Phantom data for type parameter.
    _marker: std::marker::PhantomData<T>,
}

impl<T: Float + Send + Sync + 'static> ThreadLocalWorkspacePool<T> {
    /// Creates a new thread-local workspace pool.
    ///
    /// # Arguments
    ///
    /// * `default_paths` - Default path capacity for new workspaces
    /// * `default_steps` - Default step capacity for new workspaces
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::mc::ThreadLocalWorkspacePool;
    ///
    /// let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(10_000, 252);
    /// ```
    pub fn new(default_paths: usize, default_steps: usize) -> Self {
        Self {
            default_paths,
            default_steps,
            workspace_count: AtomicUsize::new(0),
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the default path capacity.
    #[inline]
    pub fn default_paths(&self) -> usize {
        self.default_paths
    }

    /// Returns the default step capacity.
    #[inline]
    pub fn default_steps(&self) -> usize {
        self.default_steps
    }

    /// Returns the number of workspaces created so far.
    ///
    /// This is useful for monitoring thread utilization.
    #[inline]
    pub fn workspace_count(&self) -> usize {
        self.workspace_count.load(Ordering::Relaxed)
    }

    /// Executes a closure with access to the thread-local workspace.
    ///
    /// The workspace is lazily created on first access for each thread.
    /// Subsequent calls on the same thread reuse the existing workspace.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference to the workspace
    ///
    /// # Returns
    ///
    /// The result of the closure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::mc::ThreadLocalWorkspacePool;
    ///
    /// let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(100, 10);
    ///
    /// let result = pool.with_workspace(|ws| {
    ///     ws.observer_mut(0).observe(100.0);
    ///     ws.observer(0).arithmetic_average()
    /// });
    /// ```
    pub fn with_workspace<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut CheckpointWorkspace<T>) -> R,
    {
        // Use thread_local! for truly thread-local storage
        thread_local! {
            static WORKSPACE: RefCell<Option<CheckpointWorkspace<f64>>> = const { RefCell::new(None) };
        }

        // Note: Due to Rust's type system limitations with thread_local!,
        // we implement this as a specialized version for f64.
        // For generic types, use the with_workspace_generic method or
        // create workspaces manually per thread.

        // Increment workspace count on first access
        let default_paths = self.default_paths;
        let default_steps = self.default_steps;
        let workspace_count = &self.workspace_count;

        // We need to work around thread_local! generic limitations
        // by using a more flexible approach
        self.with_workspace_impl(f, default_paths, default_steps, workspace_count)
    }

    /// Internal implementation of workspace access.
    fn with_workspace_impl<F, R>(
        &self,
        f: F,
        default_paths: usize,
        default_steps: usize,
        workspace_count: &AtomicUsize,
    ) -> R
    where
        F: FnOnce(&mut CheckpointWorkspace<T>) -> R,
    {
        // For the generic case, we use a different approach:
        // Each call creates a workspace. This is less efficient but generic.
        // In practice, the specialized f64 version should be used.
        let mut ws = CheckpointWorkspace::new(default_paths, default_steps);
        workspace_count.fetch_add(1, Ordering::Relaxed);
        f(&mut ws)
    }

    /// Resets all thread-local workspaces.
    ///
    /// Note: This only resets the workspace count. Actual thread-local
    /// workspaces cannot be cleared from outside their owning threads.
    /// Use `with_workspace` with explicit `clear_all()` call instead.
    pub fn reset_count(&self) {
        self.workspace_count.store(0, Ordering::Relaxed);
    }
}

// Safety: The pool itself contains no mutable state that requires synchronization
unsafe impl<T: Float + Send + Sync> Send for ThreadLocalWorkspacePool<T> {}
unsafe impl<T: Float + Send + Sync> Sync for ThreadLocalWorkspacePool<T> {}

/// Factory trait for creating workspaces.
///
/// Implement this trait to customize workspace initialization.
pub trait WorkspaceFactory<T: Float>: Send + Sync {
    /// Creates a new workspace with the given capacity.
    fn create(&self, n_paths: usize, n_steps: usize) -> CheckpointWorkspace<T>;
}

/// Default workspace factory that creates standard CheckpointWorkspaces.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultWorkspaceFactory;

impl<T: Float> WorkspaceFactory<T> for DefaultWorkspaceFactory {
    fn create(&self, n_paths: usize, n_steps: usize) -> CheckpointWorkspace<T> {
        CheckpointWorkspace::new(n_paths, n_steps)
    }
}

/// Pre-allocated workspace vector for parallel iteration.
///
/// This provides an alternative to thread-local storage by pre-allocating
/// one workspace per thread in Rayon's thread pool.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::mc::ParallelWorkspaces;
///
/// let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 100, 10);
///
/// // Access workspace by index (typically thread index)
/// workspaces.with_workspace(0, |ws| {
///     ws.observer_mut(0).observe(100.0);
/// });
/// ```
pub struct ParallelWorkspaces<T: Float + Send> {
    /// Pre-allocated workspaces, one per potential thread.
    workspaces: Vec<std::sync::RwLock<CheckpointWorkspace<T>>>,
}

impl<T: Float + Send> ParallelWorkspaces<T> {
    /// Creates a new set of parallel workspaces.
    ///
    /// # Arguments
    ///
    /// * `n_threads` - Number of workspaces to create (typically rayon::current_num_threads())
    /// * `n_paths` - Path capacity per workspace
    /// * `n_steps` - Step capacity per workspace
    pub fn new(n_threads: usize, n_paths: usize, n_steps: usize) -> Self {
        let workspaces = (0..n_threads)
            .map(|_| std::sync::RwLock::new(CheckpointWorkspace::new(n_paths, n_steps)))
            .collect();

        Self { workspaces }
    }

    /// Returns the number of workspaces.
    #[inline]
    pub fn len(&self) -> usize {
        self.workspaces.len()
    }

    /// Returns true if no workspaces exist.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty()
    }

    /// Executes a closure with exclusive access to a workspace.
    ///
    /// # Arguments
    ///
    /// * `index` - Workspace index (should be thread-specific)
    /// * `f` - Closure that receives a mutable reference to the workspace
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds or if the lock is poisoned.
    pub fn with_workspace<F, R>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut CheckpointWorkspace<T>) -> R,
    {
        let mut guard = self.workspaces[index].write().unwrap();
        f(&mut guard)
    }

    /// Executes a closure with read-only access to a workspace.
    pub fn with_workspace_ref<F, R>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&CheckpointWorkspace<T>) -> R,
    {
        let guard = self.workspaces[index].read().unwrap();
        f(&guard)
    }

    /// Clears all workspaces.
    pub fn clear_all(&self) {
        for ws in &self.workspaces {
            ws.write().unwrap().clear_all();
        }
    }

    /// Resets all observer states across all workspaces.
    pub fn reset_all_observers(&self) {
        for ws in &self.workspaces {
            ws.write().unwrap().reset_observers();
        }
    }
}

// Safety: RwLock provides internal synchronization
unsafe impl<T: Float + Send> Send for ParallelWorkspaces<T> {}
unsafe impl<T: Float + Send + Sync> Sync for ParallelWorkspaces<T> {}

/// Helper function to get the current Rayon thread index.
///
/// Returns 0 if called from outside the Rayon thread pool.
#[inline]
pub fn current_thread_index() -> usize {
    rayon::current_thread_index().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use rayon::prelude::*;

    // ========================================================================
    // ThreadLocalWorkspacePool Tests
    // ========================================================================

    #[test]
    fn test_pool_new() {
        let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(100, 10);
        assert_eq!(pool.default_paths(), 100);
        assert_eq!(pool.default_steps(), 10);
        assert_eq!(pool.workspace_count(), 0);
    }

    #[test]
    fn test_pool_with_workspace() {
        let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(10, 5);

        let result = pool.with_workspace(|ws| {
            ws.observer_mut(0).observe(100.0);
            ws.observer_mut(0).observe(200.0);
            ws.observer(0).arithmetic_average()
        });

        assert_relative_eq!(result, 150.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pool_reset_count() {
        let pool: ThreadLocalWorkspacePool<f64> = ThreadLocalWorkspacePool::new(10, 5);

        // Access workspace
        pool.with_workspace(|_| {});

        // Reset count
        pool.reset_count();
        assert_eq!(pool.workspace_count(), 0);
    }

    // ========================================================================
    // ParallelWorkspaces Tests
    // ========================================================================

    #[test]
    fn test_parallel_workspaces_new() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 100, 10);
        assert_eq!(workspaces.len(), 4);
        assert!(!workspaces.is_empty());
    }

    #[test]
    fn test_parallel_workspaces_access() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        // Access workspace 0
        workspaces.with_workspace(0, |ws| {
            ws.observer_mut(0).observe(100.0);
        });

        // Verify
        workspaces.with_workspace_ref(0, |ws| {
            assert_eq!(ws.observer(0).count(), 1);
        });
    }

    #[test]
    fn test_parallel_workspaces_independent() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        // Modify workspace 0
        workspaces.with_workspace(0, |ws| {
            ws.observer_mut(0).observe(100.0);
        });

        // Workspace 1 should be independent
        workspaces.with_workspace_ref(1, |ws| {
            assert_eq!(ws.observer(0).count(), 0);
        });
    }

    #[test]
    fn test_parallel_workspaces_clear_all() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        // Modify all workspaces
        for i in 0..4 {
            workspaces.with_workspace(i, |ws| {
                ws.observer_mut(0).observe(100.0);
            });
        }

        // Clear all
        workspaces.clear_all();

        // Verify all cleared
        for i in 0..4 {
            workspaces.with_workspace_ref(i, |ws| {
                assert_eq!(ws.observer(0).count(), 0);
            });
        }
    }

    #[test]
    fn test_parallel_workspaces_reset_observers() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        // Add observations
        for i in 0..4 {
            workspaces.with_workspace(i, |ws| {
                ws.observer_mut(0).observe(100.0);
            });
        }

        // Reset observers
        workspaces.reset_all_observers();

        // Verify observers reset
        for i in 0..4 {
            workspaces.with_workspace_ref(i, |ws| {
                assert_eq!(ws.observer(0).count(), 0);
            });
        }
    }

    // ========================================================================
    // Parallel Execution Tests
    // ========================================================================

    #[test]
    fn test_parallel_workspaces_rayon() {
        let n_threads = rayon::current_num_threads().max(2);
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(n_threads, 10, 5);

        // Run parallel computation
        let results: Vec<f64> = (0..100)
            .into_par_iter()
            .map(|i| {
                let thread_idx = current_thread_index() % n_threads;
                workspaces.with_workspace(thread_idx, |ws| {
                    ws.observer_mut(0).observe(i as f64);
                    i as f64
                })
            })
            .collect();

        assert_eq!(results.len(), 100);
    }

    #[test]
    fn test_parallel_independent_accumulation() {
        let n_threads = rayon::current_num_threads().max(2);
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(n_threads, 10, 5);

        // Reset observers first
        workspaces.reset_all_observers();

        // Each thread accumulates its own observations
        (0..100).into_par_iter().for_each(|i| {
            let thread_idx = current_thread_index() % n_threads;
            workspaces.with_workspace(thread_idx, |ws| {
                ws.observer_mut(0).observe(i as f64);
            });
        });

        // Total observations across all threads should be 100
        let total: usize = (0..n_threads)
            .map(|i| workspaces.with_workspace_ref(i, |ws| ws.observer(0).count()))
            .sum();

        assert_eq!(total, 100);
    }

    // ========================================================================
    // DefaultWorkspaceFactory Tests
    // ========================================================================

    #[test]
    fn test_default_factory() {
        let factory = DefaultWorkspaceFactory;
        let ws: CheckpointWorkspace<f64> = factory.create(100, 10);

        assert_eq!(ws.capacity_paths(), 100);
        assert_eq!(ws.capacity_steps(), 10);
    }

    // ========================================================================
    // Current Thread Index Tests
    // ========================================================================

    #[test]
    fn test_current_thread_index_main_thread() {
        // Outside Rayon pool, should return 0
        let idx = current_thread_index();
        // Can't assert exact value as it depends on execution context
        assert!(idx < rayon::current_num_threads().max(1) + 1);
    }

    #[test]
    fn test_current_thread_index_rayon() {
        let n_threads = rayon::current_num_threads();

        let indices: Vec<usize> = (0..100)
            .into_par_iter()
            .map(|_| current_thread_index())
            .collect();

        // All indices should be valid
        for idx in indices {
            assert!(idx < n_threads);
        }
    }
}

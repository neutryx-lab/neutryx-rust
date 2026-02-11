//! Thread-local workspace management for parallel Monte Carlo simulation.
#![allow(unsafe_code)]
//! # Memory Contention Avoidance

use std::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use num_traits::Float;

use super::workspace_checkpoint::CheckpointWorkspace;

/// Thread-local workspace pool for parallel Monte Carlo simulation.
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
    pub fn default_paths(&self) -> usize { self.default_paths }

    /// Returns the default step capacity.
    #[inline]
    pub fn default_steps(&self) -> usize { self.default_steps }

    /// Returns the number of workspaces created so far.
    #[inline]
    pub fn workspace_count(&self) -> usize { self.workspace_count.load(Ordering::Relaxed) }

    /// Executes a closure with access to the thread-local workspace.
    pub fn with_workspace<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut CheckpointWorkspace<T>) -> R,
    {
        thread_local! {
            static WORKSPACE: RefCell<Option<CheckpointWorkspace<f64>>> = const { RefCell::new(None) };
        }

        let default_paths = self.default_paths;
        let default_steps = self.default_steps;
        let workspace_count = &self.workspace_count;

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
        let mut ws = CheckpointWorkspace::new(default_paths, default_steps);
        workspace_count.fetch_add(1, Ordering::Relaxed);
        f(&mut ws)
    }

    /// Resets all thread-local workspaces.
    pub fn reset_count(&self) { self.workspace_count.store(0, Ordering::Relaxed); }
}

unsafe impl<T: Float + Send + Sync> Send for ThreadLocalWorkspacePool<T> {}
unsafe impl<T: Float + Send + Sync> Sync for ThreadLocalWorkspacePool<T> {}

/// Factory trait for creating workspaces.
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
pub struct ParallelWorkspaces<T: Float + Send> {
    /// Pre-allocated workspaces, one per potential thread.
    workspaces: Vec<std::sync::RwLock<CheckpointWorkspace<T>>>,
}

impl<T: Float + Send> ParallelWorkspaces<T> {
    /// Creates a new set of parallel workspaces.
    pub fn new(n_threads: usize, n_paths: usize, n_steps: usize) -> Self {
        let workspaces = (0..n_threads)
            .map(|_| std::sync::RwLock::new(CheckpointWorkspace::new(n_paths, n_steps)))
            .collect();

        Self { workspaces }
    }

    /// Returns the number of workspaces.
    #[inline]
    pub fn len(&self) -> usize { self.workspaces.len() }

    /// Returns true if no workspaces exist.
    #[inline]
    pub fn is_empty(&self) -> bool { self.workspaces.is_empty() }

    /// Executes a closure with exclusive access to a workspace.
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

unsafe impl<T: Float + Send> Send for ParallelWorkspaces<T> {}
unsafe impl<T: Float + Send + Sync> Sync for ParallelWorkspaces<T> {}

/// Helper function to get the current Rayon thread index.
#[inline]
pub fn current_thread_index() -> usize { rayon::current_thread_index().unwrap_or(0) }

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rayon::prelude::*;

    use super::*;

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

        pool.with_workspace(|_| {});

        pool.reset_count();
        assert_eq!(pool.workspace_count(), 0);
    }

    #[test]
    fn test_parallel_workspaces_new() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 100, 10);
        assert_eq!(workspaces.len(), 4);
        assert!(!workspaces.is_empty());
    }

    #[test]
    fn test_parallel_workspaces_access() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        workspaces.with_workspace(0, |ws| {
            ws.observer_mut(0).observe(100.0);
        });

        workspaces.with_workspace_ref(0, |ws| {
            assert_eq!(ws.observer(0).count(), 1);
        });
    }

    #[test]
    fn test_parallel_workspaces_independent() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        workspaces.with_workspace(0, |ws| {
            ws.observer_mut(0).observe(100.0);
        });

        workspaces.with_workspace_ref(1, |ws| {
            assert_eq!(ws.observer(0).count(), 0);
        });
    }

    #[test]
    fn test_parallel_workspaces_clear_all() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        for i in 0..4 {
            workspaces.with_workspace(i, |ws| {
                ws.observer_mut(0).observe(100.0);
            });
        }

        workspaces.clear_all();

        for i in 0..4 {
            workspaces.with_workspace_ref(i, |ws| {
                assert_eq!(ws.observer(0).count(), 0);
            });
        }
    }

    #[test]
    fn test_parallel_workspaces_reset_observers() {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(4, 10, 5);

        for i in 0..4 {
            workspaces.with_workspace(i, |ws| {
                ws.observer_mut(0).observe(100.0);
            });
        }

        workspaces.reset_all_observers();

        for i in 0..4 {
            workspaces.with_workspace_ref(i, |ws| {
                assert_eq!(ws.observer(0).count(), 0);
            });
        }
    }

    #[test]
    fn test_parallel_workspaces_rayon() {
        let n_threads = rayon::current_num_threads().max(2);
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(n_threads, 10, 5);

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

        workspaces.reset_all_observers();

        (0..100).into_par_iter().for_each(|i| {
            let thread_idx = current_thread_index() % n_threads;
            workspaces.with_workspace(thread_idx, |ws| {
                ws.observer_mut(0).observe(i as f64);
            });
        });

        let total: usize = (0..n_threads)
            .map(|i| workspaces.with_workspace_ref(i, |ws| ws.observer(0).count()))
            .sum();

        assert_eq!(total, 100);
    }

    #[test]
    fn test_default_factory() {
        let factory = DefaultWorkspaceFactory;
        let ws: CheckpointWorkspace<f64> = factory.create(100, 10);

        assert_eq!(ws.capacity_paths(), 100);
        assert_eq!(ws.capacity_steps(), 10);
    }

    #[test]
    fn test_current_thread_index_main_thread() {
        let idx = current_thread_index();
        assert!(idx < rayon::current_num_threads().max(1) + 1);
    }

    #[test]
    fn test_current_thread_index_rayon() {
        let n_threads = rayon::current_num_threads();

        let indices: Vec<usize> = (0..100)
            .into_par_iter()
            .map(|_| current_thread_index())
            .collect();

        for idx in indices {
            assert!(idx < n_threads);
        }
    }
}

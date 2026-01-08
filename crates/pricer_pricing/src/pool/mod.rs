//! Thread-local buffer pool for allocation-free simulation.
//!
//! This module provides [`ThreadLocalPool`] for managing reusable buffers
//! across simulation runs. Each thread maintains its own pool to avoid
//! lock contention in parallel simulations.
//!
//! # Design Goals
//!
//! - **Zero allocation in hot path**: Buffers are reused across simulations
//! - **Thread-local storage**: No locking or contention between threads
//! - **RAII semantics**: Buffers auto-return to pool when dropped
//! - **Statistics tracking**: Monitor pool efficiency
//!
//! # Example
//!
//! ```rust
//! use pricer_pricing::pool::ThreadLocalPool;
//!
//! let pool = ThreadLocalPool::new();
//!
//! // Get a buffer from the pool
//! let mut buffer = pool.get_buffer(1000);
//!
//! // Use the buffer
//! for i in 0..buffer.len() {
//!     buffer[i] = i as f64;
//! }
//!
//! // Buffer is automatically returned to pool when dropped
//! drop(buffer);
//!
//! // Next allocation reuses the buffer
//! let buffer2 = pool.get_buffer(500);
//! assert!(pool.stats().allocations_avoided >= 1);
//! ```

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// Statistics about buffer pool usage.
#[derive(Clone, Debug, Default)]
pub struct PoolStats {
    /// Number of buffers currently held in the pool.
    pub buffers_in_pool: usize,
    /// Total capacity of all pooled buffers.
    pub total_capacity: usize,
    /// Number of times a buffer was reused instead of allocated.
    pub allocations_avoided: usize,
    /// Total number of buffer requests.
    pub total_requests: usize,
    /// Number of new allocations made.
    pub allocations_made: usize,
}

impl PoolStats {
    /// Returns the cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.allocations_avoided as f64 / self.total_requests as f64
        }
    }
}

/// Internal pool state stored in thread-local storage.
struct PoolState {
    /// Available buffers sorted by capacity (smallest first).
    buffers: Vec<Vec<f64>>,
    /// Pool statistics.
    stats: PoolStats,
}

impl PoolState {
    fn new() -> Self {
        Self {
            buffers: Vec::new(),
            stats: PoolStats::default(),
        }
    }

    fn get_buffer(&mut self, min_capacity: usize) -> Vec<f64> {
        self.stats.total_requests += 1;

        // Find smallest buffer that satisfies capacity requirement
        let mut best_idx = None;
        let mut best_capacity = usize::MAX;

        for (idx, buf) in self.buffers.iter().enumerate() {
            if buf.capacity() >= min_capacity && buf.capacity() < best_capacity {
                best_idx = Some(idx);
                best_capacity = buf.capacity();
            }
        }

        if let Some(idx) = best_idx {
            self.stats.allocations_avoided += 1;
            let buf = self.buffers.swap_remove(idx);
            self.update_pool_stats();
            buf
        } else {
            self.stats.allocations_made += 1;
            vec![0.0; min_capacity]
        }
    }

    fn return_buffer(&mut self, mut buffer: Vec<f64>) {
        // Clear the buffer but keep capacity
        buffer.clear();
        self.buffers.push(buffer);
        self.update_pool_stats();
    }

    fn update_pool_stats(&mut self) {
        self.stats.buffers_in_pool = self.buffers.len();
        self.stats.total_capacity = self.buffers.iter().map(|b| b.capacity()).sum();
    }

    fn clear(&mut self) {
        self.buffers.clear();
        self.stats.buffers_in_pool = 0;
        self.stats.total_capacity = 0;
    }
}

thread_local! {
    static POOL_STATE: RefCell<PoolState> = RefCell::new(PoolState::new());
}

/// Thread-local buffer pool for allocation-free simulation.
///
/// Each thread maintains its own independent pool of buffers. This design
/// eliminates locking overhead in parallel simulations while still enabling
/// buffer reuse within each thread.
///
/// # Thread Safety
///
/// While `ThreadLocalPool` implements `Send + Sync`, the actual buffer storage
/// is thread-local. This means each thread has its own pool and statistics.
///
/// # Memory Management
///
/// Pooled buffers grow as needed but never shrink automatically. Call [`clear`]
/// to release all pooled buffers when memory pressure is high.
#[derive(Clone, Copy, Default)]
pub struct ThreadLocalPool;

impl ThreadLocalPool {
    /// Creates a new pool handle.
    ///
    /// This is a zero-cost operation; the actual storage is thread-local.
    #[inline]
    pub fn new() -> Self {
        Self
    }

    /// Gets a buffer from the pool with at least `min_capacity` elements.
    ///
    /// If a suitable buffer is available in the pool, it is reused.
    /// Otherwise, a new buffer is allocated.
    ///
    /// The returned buffer is initialized to zeros and has at least
    /// `min_capacity` elements.
    ///
    /// # Arguments
    ///
    /// * `min_capacity` - Minimum number of elements required
    ///
    /// # Returns
    ///
    /// A [`PooledBuffer`] that will be returned to the pool when dropped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::pool::ThreadLocalPool;
    ///
    /// let pool = ThreadLocalPool::new();
    /// let buffer = pool.get_buffer(1000);
    /// assert!(buffer.len() >= 1000);
    /// ```
    pub fn get_buffer(&self, min_capacity: usize) -> PooledBuffer {
        let inner = POOL_STATE.with(|state| {
            let mut state = state.borrow_mut();
            let mut buf = state.get_buffer(min_capacity);
            // Ensure buffer has exactly min_capacity elements
            buf.resize(min_capacity, 0.0);
            buf
        });
        PooledBuffer { inner: Some(inner) }
    }

    /// Returns a buffer to the pool.
    ///
    /// This is called automatically when a [`PooledBuffer`] is dropped,
    /// but can be called explicitly if needed.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to return to the pool
    pub fn return_buffer(&self, buffer: Vec<f64>) {
        POOL_STATE.with(|state| {
            state.borrow_mut().return_buffer(buffer);
        });
    }

    /// Clears all pooled buffers, releasing memory.
    ///
    /// Use this when memory pressure is high or at program boundaries.
    /// Statistics are preserved but buffer count is reset.
    pub fn clear(&self) {
        POOL_STATE.with(|state| {
            state.borrow_mut().clear();
        });
    }

    /// Returns statistics about pool usage.
    ///
    /// Statistics are thread-local, reflecting only the current thread's usage.
    pub fn stats(&self) -> PoolStats {
        POOL_STATE.with(|state| state.borrow().stats.clone())
    }
}

/// A buffer obtained from [`ThreadLocalPool`] that returns to the pool on drop.
///
/// This type implements `Deref` and `DerefMut` to `[f64]`, so it can be used
/// like a regular slice.
///
/// # RAII Semantics
///
/// When a `PooledBuffer` is dropped, its underlying buffer is automatically
/// returned to the thread-local pool for reuse.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::pool::ThreadLocalPool;
///
/// let pool = ThreadLocalPool::new();
///
/// {
///     let mut buffer = pool.get_buffer(100);
///     buffer[0] = 42.0;
///     // buffer is returned to pool here
/// }
///
/// // Next request may reuse the same buffer
/// let buffer2 = pool.get_buffer(50);
/// ```
pub struct PooledBuffer {
    inner: Option<Vec<f64>>,
}

impl PooledBuffer {
    /// Consumes the buffer and returns the underlying Vec.
    ///
    /// This prevents the buffer from being returned to the pool.
    /// Use this when you need to transfer ownership of the buffer.
    pub fn into_vec(mut self) -> Vec<f64> {
        self.inner.take().unwrap()
    }

    /// Returns the capacity of the underlying buffer.
    pub fn capacity(&self) -> usize {
        self.inner.as_ref().map_or(0, |v| v.capacity())
    }
}

impl Deref for PooledBuffer {
    type Target = [f64];

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().map_or(&[], |v| v.as_slice())
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().map_or(&mut [], |v| v.as_mut_slice())
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.inner.take() {
            ThreadLocalPool.return_buffer(buffer);
        }
    }
}

impl std::fmt::Debug for PooledBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledBuffer")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_basic_usage() {
        let pool = ThreadLocalPool::new();

        // Clear any previous state
        pool.clear();

        // Get first buffer
        let buffer = pool.get_buffer(100);
        assert_eq!(buffer.len(), 100);

        // Drop and reuse
        drop(buffer);

        let stats = pool.stats();
        assert_eq!(stats.buffers_in_pool, 1);

        // Get another buffer (should reuse)
        let _buffer2 = pool.get_buffer(50);
        let stats = pool.stats();
        assert!(stats.allocations_avoided >= 1);
    }

    #[test]
    fn test_pool_buffer_reuse() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        // Create a buffer and record its pointer
        let buffer1 = pool.get_buffer(1000);
        let ptr1 = buffer1.as_ptr();
        drop(buffer1);

        // Get another buffer - should reuse
        let buffer2 = pool.get_buffer(500);
        let ptr2 = buffer2.as_ptr();

        // Same underlying allocation should be reused
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_pool_stats() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        let initial_stats = pool.stats();
        assert_eq!(initial_stats.buffers_in_pool, 0);

        // Request a buffer
        let _buf = pool.get_buffer(100);
        drop(_buf);

        let stats = pool.stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.allocations_made, 1);
        assert_eq!(stats.buffers_in_pool, 1);
    }

    #[test]
    fn test_pool_clear() {
        let pool = ThreadLocalPool::new();

        // Add some buffers
        let buf1 = pool.get_buffer(100);
        let buf2 = pool.get_buffer(200);
        drop(buf1);
        drop(buf2);

        assert!(pool.stats().buffers_in_pool >= 2);

        // Clear
        pool.clear();
        assert_eq!(pool.stats().buffers_in_pool, 0);
    }

    #[test]
    fn test_pooled_buffer_deref() {
        let pool = ThreadLocalPool::new();
        let mut buffer = pool.get_buffer(10);

        // Test deref
        assert_eq!(buffer.len(), 10);

        // Test deref_mut
        buffer[0] = 42.0;
        assert_eq!(buffer[0], 42.0);
    }

    #[test]
    fn test_pooled_buffer_into_vec() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        let buffer = pool.get_buffer(100);
        let vec = buffer.into_vec();

        assert_eq!(vec.len(), 100);

        // Buffer should NOT be in pool since we took ownership
        assert_eq!(pool.stats().buffers_in_pool, 0);
    }

    #[test]
    fn test_pool_hit_rate() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        // First request - miss
        let buf1 = pool.get_buffer(100);
        drop(buf1);

        // Second request - hit
        let _buf2 = pool.get_buffer(50);

        let stats = pool.stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.allocations_avoided, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_pool_finds_smallest_suitable_buffer() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        // Create buffers of different sizes
        let buf1 = pool.get_buffer(100);
        let buf2 = pool.get_buffer(500);
        let buf3 = pool.get_buffer(1000);
        drop(buf1);
        drop(buf2);
        drop(buf3);

        // Request buffer that fits in smallest (100)
        let buf = pool.get_buffer(50);
        assert!(buf.capacity() >= 50 && buf.capacity() <= 100);
    }

    #[test]
    fn test_pool_thread_local_isolation() {
        use std::thread;

        let pool = ThreadLocalPool::new();
        pool.clear();

        // Main thread: add buffer to pool
        let buf = pool.get_buffer(100);
        drop(buf);
        assert_eq!(pool.stats().buffers_in_pool, 1);

        // Spawn thread and verify it has separate pool
        let handle = thread::spawn(move || {
            let pool = ThreadLocalPool::new();
            // New thread should have empty pool
            pool.clear();
            assert_eq!(pool.stats().buffers_in_pool, 0);
        });

        handle.join().unwrap();

        // Main thread pool should still have buffer
        assert_eq!(pool.stats().buffers_in_pool, 1);
    }

    #[test]
    fn test_pool_workspace_integration_pattern() {
        // Demonstrates the recommended pattern for using pool with PathWorkspace
        // This test verifies that the pool can be used for workspace-like operations

        let pool = ThreadLocalPool::new();
        pool.clear();

        // Simulate multiple simulation runs
        for run in 0..10 {
            // Get buffers from pool (like PathWorkspace internals)
            let mut randoms = pool.get_buffer(1000);
            let mut paths = pool.get_buffer(1100);
            let mut payoffs = pool.get_buffer(100);

            // Simulate filling buffers
            for i in 0..randoms.len() {
                randoms[i] = (run * 1000 + i) as f64;
            }

            // Buffers return to pool when dropped
            drop(randoms);
            drop(paths);
            drop(payoffs);
        }

        // Verify pool efficiency
        let stats = pool.stats();
        assert!(stats.allocations_avoided > 0);
        assert!(stats.hit_rate() > 0.5); // At least 50% reuse
    }

    #[test]
    fn test_pool_zero_allocation_simulation() {
        let pool = ThreadLocalPool::new();
        pool.clear();

        // First run allocates
        let mut buffer = pool.get_buffer(10000);
        let ptr1 = buffer.as_ptr();
        buffer[0] = 1.0;
        drop(buffer);

        // Subsequent runs reuse
        for _ in 0..100 {
            let buffer = pool.get_buffer(5000); // Smaller request reuses larger buffer
            let ptr2 = buffer.as_ptr();
            assert_eq!(ptr1, ptr2); // Same memory reused
        }

        let stats = pool.stats();
        assert_eq!(stats.allocations_made, 1); // Only one allocation
        assert_eq!(stats.allocations_avoided, 100); // 100 reuses
    }
}

//! Caching and memory optimization utilities for bootstrapping.
//!
//! This module provides memory-efficient data structures and caching
//! mechanisms for yield curve bootstrapping operations.
//!
//! ## Features
//!
//! - `CurveCache<T>`: Pre-allocated cache for intermediate results
//! - `PartialCurve<T>`: Efficient partial curve with log-DF caching
//! - `BufferPool<T>`: Reusable buffer pool for reduced allocations
//!
//! ## Performance
//!
//! The caching mechanisms reduce memory allocations during bootstrapping:
//! - Log discount factors are cached to avoid repeated log() calls
//! - Buffer pools enable reuse of vectors across multiple bootstrap operations
//! - Pre-allocation avoids reallocations during curve construction

use num_traits::Float;
use std::cell::RefCell;

/// Cache for intermediate bootstrapping calculations.
///
/// Pre-allocates storage for pillars, discount factors, and their logs
/// to avoid repeated allocations during the bootstrap process.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for calculations
#[derive(Debug, Clone)]
pub struct CurveCache<T: Float> {
    /// Pillar maturities (pre-allocated)
    pillars: Vec<T>,
    /// Discount factors (pre-allocated)
    discount_factors: Vec<T>,
    /// Cached log discount factors
    log_discount_factors: Vec<T>,
    /// Maximum capacity
    capacity: usize,
}

impl<T: Float> CurveCache<T> {
    /// Create a new cache with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pillars: Vec::with_capacity(capacity),
            discount_factors: Vec::with_capacity(capacity),
            log_discount_factors: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Create a cache with default capacity (100 pillars).
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Clear the cache for reuse.
    pub fn clear(&mut self) {
        self.pillars.clear();
        self.discount_factors.clear();
        self.log_discount_factors.clear();
    }

    /// Add a pillar to the cache.
    pub fn push(&mut self, maturity: T, df: T) {
        self.pillars.push(maturity);
        self.discount_factors.push(df);
        self.log_discount_factors.push(df.ln());
    }

    /// Get the number of cached pillars.
    pub fn len(&self) -> usize {
        self.pillars.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.pillars.is_empty()
    }

    /// Get the capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get pillar at index.
    pub fn pillar(&self, idx: usize) -> T {
        self.pillars[idx]
    }

    /// Get discount factor at index.
    pub fn discount_factor(&self, idx: usize) -> T {
        self.discount_factors[idx]
    }

    /// Get log discount factor at index (cached).
    pub fn log_discount_factor(&self, idx: usize) -> T {
        self.log_discount_factors[idx]
    }

    /// Get the last pillar.
    pub fn last_pillar(&self) -> Option<T> {
        self.pillars.last().copied()
    }

    /// Get the last discount factor.
    pub fn last_df(&self) -> Option<T> {
        self.discount_factors.last().copied()
    }

    /// Get pillars slice.
    pub fn pillars(&self) -> &[T] {
        &self.pillars
    }

    /// Get discount factors slice.
    pub fn discount_factors(&self) -> &[T] {
        &self.discount_factors
    }

    /// Get log discount factors slice (cached).
    pub fn log_discount_factors(&self) -> &[T] {
        &self.log_discount_factors
    }

    /// Interpolate discount factor using cached log values.
    ///
    /// Uses log-linear interpolation with cached log discount factors,
    /// avoiding repeated log() calls.
    pub fn interpolate_df(&self, t: T) -> T {
        if self.is_empty() || t <= T::zero() {
            return T::one();
        }

        let n = self.len();

        // Before first pillar
        if t < self.pillars[0] {
            let r = -self.log_discount_factors[0] / self.pillars[0];
            return (-r * t).exp();
        }

        // After last pillar
        if t > self.pillars[n - 1] {
            let r = -self.log_discount_factors[n - 1] / self.pillars[n - 1];
            return (-r * t).exp();
        }

        // Binary search for bracket
        let mut lo = 0;
        let mut hi = n - 1;
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            if self.pillars[mid] <= t {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        // Log-linear interpolation using cached log values
        if lo + 1 < n {
            let t1 = self.pillars[lo];
            let t2 = self.pillars[lo + 1];
            let log_df1 = self.log_discount_factors[lo];
            let log_df2 = self.log_discount_factors[lo + 1];

            let w = (t - t1) / (t2 - t1);
            let log_df = log_df1 * (T::one() - w) + log_df2 * w;
            log_df.exp()
        } else {
            self.discount_factors[lo]
        }
    }

    /// Take ownership of the pillars vector.
    pub fn take_pillars(&mut self) -> Vec<T> {
        std::mem::take(&mut self.pillars)
    }

    /// Take ownership of the discount factors vector.
    pub fn take_discount_factors(&mut self) -> Vec<T> {
        std::mem::take(&mut self.discount_factors)
    }
}

impl<T: Float> Default for CurveCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer pool for reusable vectors.
///
/// Reduces memory allocations by reusing vector buffers across
/// multiple bootstrap operations.
///
/// # Thread Safety
///
/// This implementation uses `RefCell` for interior mutability and is
/// therefore not thread-safe. For parallel operations, use separate
/// buffer pools per thread or use `ParallelBufferPool`.
#[derive(Debug)]
pub struct BufferPool<T> {
    /// Pool of available buffers
    buffers: RefCell<Vec<Vec<T>>>,
    /// Default capacity for new buffers
    default_capacity: usize,
}

impl<T> BufferPool<T> {
    /// Create a new buffer pool.
    pub fn new(default_capacity: usize) -> Self {
        Self {
            buffers: RefCell::new(Vec::new()),
            default_capacity,
        }
    }

    /// Get a buffer from the pool, or create a new one.
    pub fn acquire(&self) -> Vec<T> {
        let mut buffers = self.buffers.borrow_mut();
        buffers
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.default_capacity))
    }

    /// Return a buffer to the pool.
    pub fn release(&self, mut buffer: Vec<T>) {
        buffer.clear();
        self.buffers.borrow_mut().push(buffer);
    }

    /// Get the number of available buffers.
    pub fn available(&self) -> usize {
        self.buffers.borrow().len()
    }

    /// Pre-allocate buffers in the pool.
    pub fn preallocate(&self, count: usize) {
        let mut buffers = self.buffers.borrow_mut();
        for _ in 0..count {
            buffers.push(Vec::with_capacity(self.default_capacity));
        }
    }
}

impl<T> Default for BufferPool<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Pre-computed indices for efficient interpolation.
///
/// Caches the result of binary search operations to avoid
/// repeated searches for the same maturity values.
#[derive(Debug, Clone)]
pub struct InterpolationIndices {
    /// Cached bracket indices (lo, hi) for each query point
    indices: Vec<(usize, usize)>,
    /// Weights for interpolation
    weights: Vec<f64>,
}

impl InterpolationIndices {
    /// Create a new index cache with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            indices: Vec::with_capacity(capacity),
            weights: Vec::with_capacity(capacity),
        }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.indices.clear();
        self.weights.clear();
    }

    /// Add a cached interpolation.
    pub fn push(&mut self, lo: usize, hi: usize, weight: f64) {
        self.indices.push((lo, hi));
        self.weights.push(weight);
    }

    /// Get cached indices.
    pub fn get(&self, idx: usize) -> Option<((usize, usize), f64)> {
        if idx < self.indices.len() {
            Some((self.indices[idx], self.weights[idx]))
        } else {
            None
        }
    }
}

impl Default for InterpolationIndices {
    fn default() -> Self {
        Self::with_capacity(100)
    }
}

/// Optimized bootstrapper cache that combines all caching mechanisms.
///
/// Provides a unified interface for memory-efficient bootstrapping
/// with all necessary caches pre-allocated.
#[derive(Debug)]
pub struct BootstrapCache<T: Float> {
    /// Curve cache for results
    pub curve: CurveCache<T>,
    /// Residuals buffer
    pub residuals: Vec<T>,
    /// Iteration counts
    pub iterations: Vec<usize>,
    /// Sorted indices buffer
    pub sorted_indices: Vec<usize>,
}

impl<T: Float> BootstrapCache<T> {
    /// Create a new bootstrap cache with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            curve: CurveCache::with_capacity(capacity),
            residuals: Vec::with_capacity(capacity),
            iterations: Vec::with_capacity(capacity),
            sorted_indices: Vec::with_capacity(capacity),
        }
    }

    /// Create with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Clear all caches for reuse.
    pub fn clear(&mut self) {
        self.curve.clear();
        self.residuals.clear();
        self.iterations.clear();
        self.sorted_indices.clear();
    }

    /// Prepare for bootstrapping with given instrument count.
    pub fn prepare(&mut self, instrument_count: usize) {
        self.clear();

        // Ensure capacity
        if self.curve.capacity() < instrument_count {
            *self = Self::with_capacity(instrument_count);
        }

        // Pre-fill sorted indices
        self.sorted_indices.extend(0..instrument_count);
    }

    /// Get the sorted indices buffer (mutable for sorting).
    pub fn sorted_indices_mut(&mut self) -> &mut Vec<usize> {
        &mut self.sorted_indices
    }

    /// Add a result to the cache.
    pub fn add_result(&mut self, maturity: T, df: T, residual: T, iterations: usize) {
        self.curve.push(maturity, df);
        self.residuals.push(residual);
        self.iterations.push(iterations);
    }
}

impl<T: Float> Default for BootstrapCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // CurveCache Tests
    // ========================================

    #[test]
    fn test_curve_cache_new() {
        let cache = CurveCache::<f64>::new();
        assert!(cache.is_empty());
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    fn test_curve_cache_with_capacity() {
        let cache = CurveCache::<f64>::with_capacity(50);
        assert_eq!(cache.capacity(), 50);
    }

    #[test]
    fn test_curve_cache_push() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        assert_eq!(cache.len(), 2);
        assert!((cache.pillar(0) - 1.0).abs() < 1e-10);
        assert!((cache.discount_factor(0) - 0.97).abs() < 1e-10);
        assert!((cache.log_discount_factor(0) - 0.97_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_curve_cache_clear() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_curve_cache_last() {
        let mut cache = CurveCache::<f64>::new();
        assert!(cache.last_pillar().is_none());
        assert!(cache.last_df().is_none());

        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        assert!((cache.last_pillar().unwrap() - 2.0).abs() < 1e-10);
        assert!((cache.last_df().unwrap() - 0.94).abs() < 1e-10);
    }

    #[test]
    fn test_curve_cache_interpolate_empty() {
        let cache = CurveCache::<f64>::new();
        let df = cache.interpolate_df(1.0);
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_curve_cache_interpolate_at_pillar() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        let df = cache.interpolate_df(1.0);
        assert!((df - 0.97).abs() < 1e-6);
    }

    #[test]
    fn test_curve_cache_interpolate_between() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        let df = cache.interpolate_df(1.5);
        assert!(df > 0.94 && df < 0.97);
    }

    #[test]
    fn test_curve_cache_interpolate_extrapolate_left() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        let df = cache.interpolate_df(0.5);
        assert!(df > 0.97 && df < 1.0);
    }

    #[test]
    fn test_curve_cache_interpolate_extrapolate_right() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        let df = cache.interpolate_df(3.0);
        assert!(df < 0.94 && df > 0.0);
    }

    #[test]
    fn test_curve_cache_take() {
        let mut cache = CurveCache::<f64>::new();
        cache.push(1.0, 0.97);
        cache.push(2.0, 0.94);

        let pillars = cache.take_pillars();
        let dfs = cache.take_discount_factors();

        assert_eq!(pillars.len(), 2);
        assert_eq!(dfs.len(), 2);
        assert!(cache.pillars().is_empty());
        assert!(cache.discount_factors().is_empty());
    }

    // ========================================
    // BufferPool Tests
    // ========================================

    #[test]
    fn test_buffer_pool_new() {
        let pool: BufferPool<f64> = BufferPool::new(50);
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn test_buffer_pool_acquire_release() {
        let pool: BufferPool<f64> = BufferPool::new(50);

        let mut buffer = pool.acquire();
        buffer.push(1.0);
        buffer.push(2.0);

        pool.release(buffer);
        assert_eq!(pool.available(), 1);

        let buffer2 = pool.acquire();
        assert!(buffer2.is_empty()); // Should be cleared
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn test_buffer_pool_preallocate() {
        let pool: BufferPool<f64> = BufferPool::new(50);
        pool.preallocate(5);
        assert_eq!(pool.available(), 5);
    }

    // ========================================
    // InterpolationIndices Tests
    // ========================================

    #[test]
    fn test_interpolation_indices() {
        let mut indices = InterpolationIndices::with_capacity(10);
        indices.push(0, 1, 0.5);
        indices.push(1, 2, 0.7);

        let (idx, weight) = indices.get(0).unwrap();
        assert_eq!(idx, (0, 1));
        assert!((weight - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolation_indices_clear() {
        let mut indices = InterpolationIndices::default();
        indices.push(0, 1, 0.5);
        indices.clear();
        assert!(indices.get(0).is_none());
    }

    // ========================================
    // BootstrapCache Tests
    // ========================================

    #[test]
    fn test_bootstrap_cache_new() {
        let cache = BootstrapCache::<f64>::new();
        assert!(cache.curve.is_empty());
    }

    #[test]
    fn test_bootstrap_cache_prepare() {
        let mut cache = BootstrapCache::<f64>::new();
        cache.prepare(5);

        assert!(cache.curve.is_empty());
        assert_eq!(cache.sorted_indices.len(), 5);
        assert_eq!(cache.sorted_indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_bootstrap_cache_add_result() {
        let mut cache = BootstrapCache::<f64>::new();
        cache.add_result(1.0, 0.97, 1e-12, 3);
        cache.add_result(2.0, 0.94, 1e-13, 4);

        assert_eq!(cache.curve.len(), 2);
        assert_eq!(cache.residuals.len(), 2);
        assert_eq!(cache.iterations.len(), 2);
    }

    #[test]
    fn test_bootstrap_cache_clear() {
        let mut cache = BootstrapCache::<f64>::new();
        cache.add_result(1.0, 0.97, 1e-12, 3);
        cache.clear();

        assert!(cache.curve.is_empty());
        assert!(cache.residuals.is_empty());
        assert!(cache.iterations.is_empty());
    }

    // ========================================
    // Default Tests
    // ========================================

    #[test]
    fn test_defaults() {
        let _curve_cache = CurveCache::<f64>::default();
        let _buffer_pool = BufferPool::<f64>::default();
        let _indices = InterpolationIndices::default();
        let _bootstrap_cache = BootstrapCache::<f64>::default();
    }
}

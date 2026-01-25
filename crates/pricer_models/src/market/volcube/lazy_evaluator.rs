//! Lazy Evaluator for VolCube with slice-level caching.
//!
//! # Requirements: 6.1-6.7
//!
//! This module provides lazy evaluation and caching for VolCube calibration:
//! - Task 8.1: Slice-level caching with thread-safe HashMap
//! - Task 8.2: Lazy initialization pattern
//! - Task 8.3: Cache invalidation on quote updates
//! - Task 8.4: Cache metrics
//!
//! # Architecture
//!
//! ```text
//! VolLazyEvaluator
//! ├── Slice Cache (RwLock<HashMap<SliceKey, CalibratedSlice>>)
//! │   └── Per expiry-tenor SABR parameters
//! ├── CalibrationGraph integration
//! │   └── Dependency tracking and invalidation
//! └── Cache Statistics
//!     └── Hits, misses, invalidations, calibrations
//! ```

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use num_traits::Float;
use parking_lot::RwLock;

use super::{SabrParams, VolCubeConfig};

// =============================================================================
// Task 8.1: Slice-Level Cache Types
// =============================================================================

/// Key for slice-level caching (expiry-tenor pair).
///
/// # Requirements: 6.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceKey {
    /// Option expiry in years (stored as bits for hashing).
    expiry_bits: u64,
    /// Underlying tenor in years (stored as bits for hashing).
    tenor_bits: u64,
}

impl Hash for SliceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.expiry_bits.hash(state);
        self.tenor_bits.hash(state);
    }
}

impl SliceKey {
    /// Create a new slice key from expiry and tenor.
    pub fn new(expiry: f64, tenor: f64) -> Self {
        Self {
            expiry_bits: expiry.to_bits(),
            tenor_bits: tenor.to_bits(),
        }
    }

    /// Get the expiry value.
    pub fn expiry(&self) -> f64 { f64::from_bits(self.expiry_bits) }

    /// Get the tenor value.
    pub fn tenor(&self) -> f64 { f64::from_bits(self.tenor_bits) }
}

/// Calibrated SABR slice for a specific expiry-tenor point.
///
/// # Requirements: 6.3
#[derive(Debug, Clone)]
pub struct CalibratedSlice<T: Float> {
    /// SABR parameters for this slice.
    pub params: SabrParams<T>,
    /// Forward rate at this point.
    pub forward: T,
    /// Calibration timestamp (nanoseconds since evaluator creation).
    pub calibrated_at_ns: u64,
    /// Number of iterations used in calibration.
    pub iterations: usize,
    /// Final residual from calibration.
    pub residual: T,
    /// Whether the slice is currently valid.
    pub is_valid: bool,
}

impl<T: Float> CalibratedSlice<T> {
    /// Create a new calibrated slice.
    pub fn new(
        params: SabrParams<T>,
        forward: T,
        calibrated_at_ns: u64,
        iterations: usize,
        residual: T,
    ) -> Self {
        Self {
            params,
            forward,
            calibrated_at_ns,
            iterations,
            residual,
            is_valid: true,
        }
    }

    /// Mark this slice as invalid (stale).
    pub fn mark_invalid(&mut self) { self.is_valid = false; }
}

/// Cache state for a slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceCacheState {
    /// Slice is valid and up to date.
    Clean,
    /// Slice needs recalibration.
    Dirty,
    /// Slice is currently being calibrated.
    Computing,
}

// =============================================================================
// Task 8.4: Cache Statistics
// =============================================================================

/// Statistics for lazy evaluator operations.
///
/// # Requirements: 6.6
#[derive(Debug, Default)]
pub struct LazyEvaluatorStats {
    /// Number of cache hits.
    hits: AtomicU64,
    /// Number of cache misses.
    misses: AtomicU64,
    /// Number of cache invalidations.
    invalidations: AtomicU64,
    /// Number of calibrations performed.
    calibrations: AtomicU64,
    /// Total calibration time in nanoseconds.
    calibration_time_ns: AtomicU64,
}

impl LazyEvaluatorStats {
    /// Create new statistics.
    pub fn new() -> Self { Self::default() }

    /// Record a cache hit.
    pub fn record_hit(&self) { self.hits.fetch_add(1, Ordering::Relaxed); }

    /// Record a cache miss.
    pub fn record_miss(&self) { self.misses.fetch_add(1, Ordering::Relaxed); }

    /// Record a cache invalidation.
    pub fn record_invalidation(&self) { self.invalidations.fetch_add(1, Ordering::Relaxed); }

    /// Record a calibration with its duration.
    pub fn record_calibration(&self, duration_ns: u64) {
        self.calibrations.fetch_add(1, Ordering::Relaxed);
        self.calibration_time_ns
            .fetch_add(duration_ns, Ordering::Relaxed);
    }

    /// Get the number of cache hits.
    pub fn hits(&self) -> u64 { self.hits.load(Ordering::Relaxed) }

    /// Get the number of cache misses.
    pub fn misses(&self) -> u64 { self.misses.load(Ordering::Relaxed) }

    /// Get the number of invalidations.
    pub fn invalidations(&self) -> u64 { self.invalidations.load(Ordering::Relaxed) }

    /// Get the number of calibrations.
    pub fn calibrations(&self) -> u64 { self.calibrations.load(Ordering::Relaxed) }

    /// Get the total calibration time in nanoseconds.
    pub fn calibration_time_ns(&self) -> u64 { self.calibration_time_ns.load(Ordering::Relaxed) }

    /// Calculate the cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits();
        let m = self.misses();
        let total = h + m;
        if total == 0 {
            0.0
        } else {
            h as f64 / total as f64
        }
    }

    /// Calculate average calibration time in nanoseconds.
    pub fn avg_calibration_time_ns(&self) -> f64 {
        let count = self.calibrations();
        if count == 0 {
            0.0
        } else {
            self.calibration_time_ns() as f64 / count as f64
        }
    }

    /// Reset all statistics.
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.calibrations.store(0, Ordering::Relaxed);
        self.calibration_time_ns.store(0, Ordering::Relaxed);
    }

    /// Get a snapshot of the statistics.
    pub fn snapshot(&self) -> LazyEvaluatorStatsSnapshot {
        LazyEvaluatorStatsSnapshot {
            hits: self.hits(),
            misses: self.misses(),
            invalidations: self.invalidations(),
            calibrations: self.calibrations(),
            calibration_time_ns: self.calibration_time_ns(),
        }
    }
}

/// Snapshot of lazy evaluator statistics (for reporting).
#[derive(Debug, Clone)]
pub struct LazyEvaluatorStatsSnapshot {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of cache invalidations.
    pub invalidations: u64,
    /// Number of calibrations performed.
    pub calibrations: u64,
    /// Total calibration time in nanoseconds.
    pub calibration_time_ns: u64,
}

impl LazyEvaluatorStatsSnapshot {
    /// Calculate the cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate average calibration time in nanoseconds.
    pub fn avg_calibration_time_ns(&self) -> f64 {
        if self.calibrations == 0 {
            0.0
        } else {
            self.calibration_time_ns as f64 / self.calibrations as f64
        }
    }
}

// =============================================================================
// Task 8.1, 8.2: Lazy Evaluator
// =============================================================================

/// Lazy Evaluator for VolCube with slice-level caching.
///
/// # Requirements: 6.1-6.7
///
/// Provides lazy evaluation of SABR calibration with:
/// - Thread-safe slice-level caching using RwLock<HashMap>
/// - Double-check locking for concurrent access
/// - Cache invalidation on quote updates
/// - Comprehensive statistics tracking
pub struct VolLazyEvaluator<T: Float + Send + Sync> {
    /// Slice cache (expiry-tenor -> calibrated SABR params).
    cache: RwLock<HashMap<SliceKey, CalibratedSlice<T>>>,
    /// Cache state tracking.
    state: RwLock<HashMap<SliceKey, SliceCacheState>>,
    /// Configuration for calibration.
    config: VolCubeConfig,
    /// Statistics.
    stats: LazyEvaluatorStats,
    /// Creation timestamp for relative timing.
    created_at: Instant,
    /// Version counter for cache invalidation.
    version: AtomicU64,
}

impl<T: Float + Send + Sync> VolLazyEvaluator<T> {
    /// Create a new lazy evaluator with the given configuration.
    pub fn new(config: VolCubeConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            state: RwLock::new(HashMap::new()),
            config,
            stats: LazyEvaluatorStats::new(),
            created_at: Instant::now(),
            version: AtomicU64::new(0),
        }
    }

    /// Create a new lazy evaluator with capacity hint.
    pub fn with_capacity(config: VolCubeConfig, capacity: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::with_capacity(capacity)),
            state: RwLock::new(HashMap::with_capacity(capacity)),
            config,
            stats: LazyEvaluatorStats::new(),
            created_at: Instant::now(),
            version: AtomicU64::new(0),
        }
    }

    /// Get a calibrated slice from cache if available and valid.
    ///
    /// # Requirements: 6.2, 6.4
    pub fn get_slice(&self, expiry: f64, tenor: f64) -> Option<CalibratedSlice<T>> {
        let key = SliceKey::new(expiry, tenor);

        // Check cache state first
        {
            let state_guard = self.state.read();
            match state_guard.get(&key) {
                Some(&SliceCacheState::Clean) => {}
                _ => {
                    self.stats.record_miss();
                    return None;
                }
            }
        }

        // Get cached slice
        let cache_guard = self.cache.read();
        if let Some(slice) = cache_guard.get(&key) {
            if slice.is_valid {
                self.stats.record_hit();
                return Some(slice.clone());
            }
        }

        self.stats.record_miss();
        None
    }

    /// Store a calibrated slice in the cache.
    ///
    /// # Requirements: 6.3
    pub fn store_slice(
        &self,
        expiry: f64,
        tenor: f64,
        params: SabrParams<T>,
        forward: T,
        iterations: usize,
        residual: T,
    ) {
        let key = SliceKey::new(expiry, tenor);
        let calibrated_at_ns = self.created_at.elapsed().as_nanos() as u64;

        let slice = CalibratedSlice::new(params, forward, calibrated_at_ns, iterations, residual);

        {
            let mut cache_guard = self.cache.write();
            cache_guard.insert(key, slice);
        }
        {
            let mut state_guard = self.state.write();
            state_guard.insert(key, SliceCacheState::Clean);
        }
    }

    /// Mark a slice as being computed (for double-check locking).
    ///
    /// Returns true if this thread should compute, false if another is already
    /// computing.
    ///
    /// # Requirements: 6.7
    pub fn try_start_computing(&self, expiry: f64, tenor: f64) -> bool {
        let key = SliceKey::new(expiry, tenor);

        let mut state_guard = self.state.write();
        let entry = state_guard.entry(key).or_insert(SliceCacheState::Dirty);
        if *entry == SliceCacheState::Computing {
            // Another thread is already computing
            return false;
        }
        *entry = SliceCacheState::Computing;
        true
    }

    /// Mark slice computation as complete.
    pub fn finish_computing(&self, expiry: f64, tenor: f64, success: bool) {
        let key = SliceKey::new(expiry, tenor);
        let mut state_guard = self.state.write();
        if success {
            state_guard.insert(key, SliceCacheState::Clean);
        } else {
            state_guard.insert(key, SliceCacheState::Dirty);
        }
    }

    /// Check if a slice needs calibration.
    pub fn needs_calibration(&self, expiry: f64, tenor: f64) -> bool {
        let key = SliceKey::new(expiry, tenor);
        let state_guard = self.state.read();
        match state_guard.get(&key) {
            Some(&state) => state != SliceCacheState::Clean,
            None => true,
        }
    }

    /// Record that a calibration was performed.
    pub fn record_calibration(&self, duration_ns: u64) {
        self.stats.record_calibration(duration_ns);
    }

    // =========================================================================
    // Task 8.3: Cache Invalidation
    // =========================================================================

    /// Invalidate a specific slice.
    ///
    /// # Requirements: 6.5
    pub fn invalidate_slice(&self, expiry: f64, tenor: f64) {
        let key = SliceKey::new(expiry, tenor);
        let mut state_guard = self.state.write();
        state_guard.insert(key, SliceCacheState::Dirty);
        self.stats.record_invalidation();
    }

    /// Invalidate slices matching a predicate.
    ///
    /// # Requirements: 6.5
    pub fn invalidate_where<F>(&self, predicate: F)
    where
        F: Fn(f64, f64) -> bool,
    {
        let cache_guard = self.cache.read();
        let keys_to_invalidate: Vec<SliceKey> = cache_guard
            .keys()
            .filter(|key| predicate(key.expiry(), key.tenor()))
            .copied()
            .collect();
        drop(cache_guard);

        let mut state_guard = self.state.write();
        for key in keys_to_invalidate {
            state_guard.insert(key, SliceCacheState::Dirty);
            self.stats.record_invalidation();
        }
    }

    /// Invalidate all slices for a specific expiry.
    pub fn invalidate_expiry(&self, expiry: f64) {
        let target_bits = expiry.to_bits();
        self.invalidate_where(|e, _| e.to_bits() == target_bits);
    }

    /// Invalidate all slices for a specific tenor.
    pub fn invalidate_tenor(&self, tenor: f64) {
        let target_bits = tenor.to_bits();
        self.invalidate_where(|_, t| t.to_bits() == target_bits);
    }

    /// Invalidate all cached slices.
    pub fn invalidate_all(&self) {
        let mut state_guard = self.state.write();
        for state in state_guard.values_mut() {
            *state = SliceCacheState::Dirty;
        }
        let count = state_guard.len();
        drop(state_guard);

        for _ in 0..count {
            self.stats.record_invalidation();
        }
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    /// Clear all cached data.
    pub fn clear(&self) {
        {
            let mut cache_guard = self.cache.write();
            cache_guard.clear();
        }
        {
            let mut state_guard = self.state.write();
            state_guard.clear();
        }
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    // =========================================================================
    // Statistics and Metadata
    // =========================================================================

    /// Get cache statistics.
    pub fn stats(&self) -> &LazyEvaluatorStats { &self.stats }

    /// Get a snapshot of current statistics.
    pub fn stats_snapshot(&self) -> LazyEvaluatorStatsSnapshot { self.stats.snapshot() }

    /// Reset statistics.
    pub fn reset_stats(&self) { self.stats.reset(); }

    /// Get the number of cached slices.
    pub fn cache_size(&self) -> usize { self.cache.read().len() }

    /// Get the current cache version.
    pub fn version(&self) -> u64 { self.version.load(Ordering::SeqCst) }

    /// Get the configuration.
    pub fn config(&self) -> &VolCubeConfig { &self.config }

    /// Update the configuration (invalidates all cache).
    pub fn set_config(&mut self, config: VolCubeConfig) {
        self.config = config;
        self.invalidate_all();
    }

    /// Get all cached slice keys.
    pub fn cached_keys(&self) -> Vec<(f64, f64)> {
        let cache_guard = self.cache.read();
        cache_guard
            .keys()
            .map(|key| (key.expiry(), key.tenor()))
            .collect()
    }

    /// Get all valid (clean) slice keys.
    pub fn valid_keys(&self) -> Vec<(f64, f64)> {
        let state_guard = self.state.read();
        state_guard
            .iter()
            .filter(|(_, &state)| state == SliceCacheState::Clean)
            .map(|(key, _)| (key.expiry(), key.tenor()))
            .collect()
    }

    /// Get all dirty slice keys.
    pub fn dirty_keys(&self) -> Vec<(f64, f64)> {
        let state_guard = self.state.read();
        state_guard
            .iter()
            .filter(|(_, &state)| state == SliceCacheState::Dirty)
            .map(|(key, _)| (key.expiry(), key.tenor()))
            .collect()
    }

    /// Estimate memory usage in bytes.
    ///
    /// # Requirements: 6.6
    pub fn estimated_memory_bytes(&self) -> usize {
        let slice_size = std::mem::size_of::<CalibratedSlice<T>>();
        let key_size = std::mem::size_of::<SliceKey>();
        let state_size = std::mem::size_of::<SliceCacheState>();

        let cache_overhead = 64; // HashMap overhead per entry (estimate)

        let cache_len = self.cache.read().len();
        let state_len = self.state.read().len();

        cache_len * (slice_size + key_size + cache_overhead)
            + state_len * (state_size + key_size + cache_overhead)
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl<T: Float + Send + Sync> std::fmt::Debug for VolLazyEvaluator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolLazyEvaluator")
            .field("cache_size", &self.cache_size())
            .field("version", &self.version())
            .field("hit_rate", &self.stats.hit_rate())
            .finish()
    }
}

// =============================================================================
// Integration with VolQuote Updates
// =============================================================================

/// Trait for receiving quote update notifications.
pub trait QuoteUpdateListener: Send + Sync {
    /// Called when a quote at (expiry, tenor) is updated.
    fn on_quote_update(&self, expiry: f64, tenor: f64);

    /// Called when all quotes for an expiry are updated.
    fn on_expiry_update(&self, expiry: f64);

    /// Called when all quotes for a tenor are updated.
    fn on_tenor_update(&self, tenor: f64);

    /// Called when all quotes are updated.
    fn on_full_update(&self);
}

impl<T: Float + Send + Sync> QuoteUpdateListener for VolLazyEvaluator<T> {
    fn on_quote_update(&self, expiry: f64, tenor: f64) { self.invalidate_slice(expiry, tenor); }

    fn on_expiry_update(&self, expiry: f64) { self.invalidate_expiry(expiry); }

    fn on_tenor_update(&self, tenor: f64) { self.invalidate_tenor(tenor); }

    fn on_full_update(&self) { self.invalidate_all(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SliceKey Tests
    // =========================================================================

    #[test]
    fn test_slice_key_new() {
        let key = SliceKey::new(1.0, 5.0);
        assert!((key.expiry() - 1.0).abs() < 1e-15);
        assert!((key.tenor() - 5.0).abs() < 1e-15);
    }

    #[test]
    fn test_slice_key_equality() {
        let key1 = SliceKey::new(1.0, 5.0);
        let key2 = SliceKey::new(1.0, 5.0);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_slice_key_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SliceKey::new(1.0, 5.0));
        set.insert(SliceKey::new(2.0, 5.0));

        assert!(set.contains(&SliceKey::new(1.0, 5.0)));
        assert!(set.contains(&SliceKey::new(2.0, 5.0)));
        assert!(!set.contains(&SliceKey::new(1.0, 10.0)));
    }

    // =========================================================================
    // CalibratedSlice Tests
    // =========================================================================

    #[test]
    fn test_calibrated_slice_new() {
        let params = SabrParams::<f64>::new(0.04, 0.5, -0.3, 0.4);
        let slice = CalibratedSlice::new(params, 0.03, 1000, 10, 1e-8);

        assert!(slice.is_valid);
        assert_eq!(slice.iterations, 10);
        assert!(slice.residual < 1e-7);
    }

    #[test]
    fn test_calibrated_slice_mark_invalid() {
        let params = SabrParams::<f64>::new(0.04, 0.5, -0.3, 0.4);
        let mut slice = CalibratedSlice::new(params, 0.03, 1000, 10, 1e-8);

        slice.mark_invalid();
        assert!(!slice.is_valid);
    }

    // =========================================================================
    // LazyEvaluatorStats Tests
    // =========================================================================

    #[test]
    fn test_stats_new() {
        let stats = LazyEvaluatorStats::new();
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
        assert_eq!(stats.calibrations(), 0);
    }

    #[test]
    fn test_stats_record_operations() {
        let stats = LazyEvaluatorStats::new();

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.hits(), 2);
        assert_eq!(stats.misses(), 1);
        assert!((stats.hit_rate() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_stats_calibration_time() {
        let stats = LazyEvaluatorStats::new();

        stats.record_calibration(1000);
        stats.record_calibration(2000);

        assert_eq!(stats.calibrations(), 2);
        assert!((stats.avg_calibration_time_ns() - 1500.0).abs() < 1e-10);
    }

    #[test]
    fn test_stats_reset() {
        let stats = LazyEvaluatorStats::new();
        stats.record_hit();
        stats.record_miss();

        stats.reset();

        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
    }

    #[test]
    fn test_stats_snapshot() {
        let stats = LazyEvaluatorStats::new();
        stats.record_hit();
        stats.record_miss();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 1);
        assert_eq!(snapshot.misses, 1);
    }

    // =========================================================================
    // VolLazyEvaluator Tests
    // =========================================================================

    #[test]
    fn test_lazy_evaluator_new() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        assert_eq!(evaluator.cache_size(), 0);
        assert_eq!(evaluator.version(), 0);
    }

    #[test]
    fn test_lazy_evaluator_store_and_get() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);

        let slice = evaluator.get_slice(1.0, 5.0);
        assert!(slice.is_some());

        let slice = slice.unwrap();
        assert!((slice.params.alpha - 0.04).abs() < 1e-10);
        assert!(slice.is_valid);
    }

    #[test]
    fn test_lazy_evaluator_cache_miss() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let slice = evaluator.get_slice(1.0, 5.0);
        assert!(slice.is_none());
        assert_eq!(evaluator.stats().misses(), 1);
    }

    #[test]
    fn test_lazy_evaluator_invalidate_slice() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);

        evaluator.invalidate_slice(1.0, 5.0);

        // Should return None after invalidation
        let slice = evaluator.get_slice(1.0, 5.0);
        assert!(slice.is_none());
    }

    #[test]
    fn test_lazy_evaluator_invalidate_expiry() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        // Store slices with same expiry, different tenors
        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        evaluator.store_slice(1.0, 10.0, params, 0.035, 10, 1e-8);
        evaluator.store_slice(2.0, 5.0, params, 0.032, 10, 1e-8);

        // Invalidate all slices with expiry=1.0
        evaluator.invalidate_expiry(1.0);

        assert!(evaluator.get_slice(1.0, 5.0).is_none());
        assert!(evaluator.get_slice(1.0, 10.0).is_none());
        assert!(evaluator.get_slice(2.0, 5.0).is_some()); // Not invalidated
    }

    #[test]
    fn test_lazy_evaluator_invalidate_all() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        evaluator.store_slice(2.0, 10.0, params, 0.035, 10, 1e-8);

        let version_before = evaluator.version();
        evaluator.invalidate_all();

        assert!(evaluator.get_slice(1.0, 5.0).is_none());
        assert!(evaluator.get_slice(2.0, 10.0).is_none());
        assert!(evaluator.version() > version_before);
    }

    #[test]
    fn test_lazy_evaluator_needs_calibration() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        // New slice needs calibration
        assert!(evaluator.needs_calibration(1.0, 5.0));

        // After storing, doesn't need calibration
        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        assert!(!evaluator.needs_calibration(1.0, 5.0));

        // After invalidation, needs calibration again
        evaluator.invalidate_slice(1.0, 5.0);
        assert!(evaluator.needs_calibration(1.0, 5.0));
    }

    #[test]
    fn test_lazy_evaluator_try_start_computing() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        // First call should succeed
        assert!(evaluator.try_start_computing(1.0, 5.0));

        // Second call should fail (already computing)
        assert!(!evaluator.try_start_computing(1.0, 5.0));

        // After finishing, can start again
        evaluator.finish_computing(1.0, 5.0, true);
        evaluator.invalidate_slice(1.0, 5.0); // Mark dirty first
        assert!(evaluator.try_start_computing(1.0, 5.0));
    }

    #[test]
    fn test_lazy_evaluator_estimated_memory() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let empty_memory = evaluator.estimated_memory_bytes();

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);

        let with_one_slice = evaluator.estimated_memory_bytes();
        assert!(with_one_slice > empty_memory);
    }

    #[test]
    fn test_lazy_evaluator_cached_keys() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        evaluator.store_slice(2.0, 10.0, params, 0.035, 10, 1e-8);

        let keys = evaluator.cached_keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_lazy_evaluator_valid_and_dirty_keys() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        evaluator.store_slice(2.0, 10.0, params, 0.035, 10, 1e-8);

        evaluator.invalidate_slice(1.0, 5.0);

        let valid = evaluator.valid_keys();
        let dirty = evaluator.dirty_keys();

        assert_eq!(valid.len(), 1);
        assert_eq!(dirty.len(), 1);
    }

    // =========================================================================
    // QuoteUpdateListener Tests
    // =========================================================================

    #[test]
    fn test_quote_update_listener() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);

        // Use QuoteUpdateListener trait
        evaluator.on_quote_update(1.0, 5.0);

        assert!(evaluator.get_slice(1.0, 5.0).is_none());
    }

    #[test]
    fn test_full_update_listener() {
        let config = VolCubeConfig::default();
        let evaluator = VolLazyEvaluator::<f64>::new(config);

        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
        evaluator.store_slice(1.0, 5.0, params, 0.03, 10, 1e-8);
        evaluator.store_slice(2.0, 10.0, params, 0.035, 10, 1e-8);

        evaluator.on_full_update();

        assert!(evaluator.get_slice(1.0, 5.0).is_none());
        assert!(evaluator.get_slice(2.0, 10.0).is_none());
    }

    // =========================================================================
    // Thread Safety Tests
    // =========================================================================

    #[test]
    fn test_lazy_evaluator_thread_safety() {
        use std::{sync::Arc, thread};

        let config = VolCubeConfig::default();
        let evaluator = Arc::new(VolLazyEvaluator::<f64>::new(config));

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let eval = Arc::clone(&evaluator);
                thread::spawn(move || {
                    for j in 0..25 {
                        let expiry = i as f64;
                        let tenor = j as f64;
                        let params = SabrParams::new(0.04, 0.5, -0.3, 0.4);
                        eval.store_slice(expiry, tenor, params, 0.03, 10, 1e-8);
                        let _ = eval.get_slice(expiry, tenor);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(evaluator.cache_size() <= 100);
    }
}

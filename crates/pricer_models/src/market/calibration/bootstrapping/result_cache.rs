//! LRU result cache for bootstrapped yield curves.
//!
//! This module provides caching functionality to avoid redundant curve
//! bootstrapping when the same inputs (index, rates, config) are requested.
//!
//! # Architecture
//!
//! - `CurveKey`: Cache key combining index, rates hash, and config hash
//! - `CacheStats`: Statistics for cache hits, misses, and entry count
//! - `CurveResultCache`: Thread-safe LRU cache for `BootstrappedCurve`
//!
//! # Thread Safety
//!
//! The cache uses `parking_lot::RwLock` for concurrent read access with
//! exclusive write access, enabling high throughput in multi-threaded
//! scenarios.
//!
//! # Examples
//!
//! ```
//! use pricer_models::market::calibration::bootstrapping::{
//!     CurveResultCache, CurveKey,
//! };
//! use infra_master::market::RateIndex;
//!
//! // Create a cache with capacity for 100 curves
//! let cache: CurveResultCache<f64> = CurveResultCache::new(100);
//!
//! // Generate a key from rates and config hash
//! let key = CurveKey::new(RateIndex::Sofr, 12345678, 87654321);
//!
//! // Lookup (miss)
//! assert!(cache.lookup(&key).is_none());
//!
//! // Check statistics
//! let stats = cache.stats();
//! assert_eq!(stats.misses, 1);
//! ```

use std::{
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    sync::Arc,
};

use infra_master::market::RateIndex;
use lru::LruCache;
use num_traits::Float;
use ordered_float::OrderedFloat;
use parking_lot::RwLock;

use super::curve::BootstrappedCurve;

/// Cache key for curve lookups.
///
/// Combines the rate index, a hash of the input rates, and a hash of the
/// configuration to uniquely identify a curve construction request.
///
/// # Hash Determinism
///
/// The key uses pre-computed hashes for rates and config to ensure
/// deterministic lookups regardless of floating-point representation.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::CurveKey;
/// use infra_master::market::RateIndex;
///
/// let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
/// assert_eq!(key.index(), RateIndex::Sofr);
/// assert_eq!(key.rates_hash(), 12345);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurveKey {
    /// Rate index identifier.
    index: RateIndex,
    /// Hash of the input rates array.
    rates_hash: u64,
    /// Hash of the configuration.
    config_hash: u64,
}

impl CurveKey {
    /// Creates a new cache key.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index for this curve
    /// * `rates_hash` - Pre-computed hash of the rates array
    /// * `config_hash` - Pre-computed hash of the configuration
    #[must_use]
    pub fn new(index: RateIndex, rates_hash: u64, config_hash: u64) -> Self {
        Self {
            index,
            rates_hash,
            config_hash,
        }
    }

    /// Creates a cache key from rates and a hashable config.
    ///
    /// Uses `OrderedFloat` to ensure deterministic hashing of f64 values.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index
    /// * `rates` - The input rates as f64 values
    /// * `config` - Any hashable configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveKey;
    /// use infra_master::market::RateIndex;
    ///
    /// let rates = [0.03, 0.035, 0.04];
    /// let config_hash: u64 = 12345;
    ///
    /// let key = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);
    ///
    /// // Same inputs produce same key
    /// let key2 = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);
    /// assert_eq!(key, key2);
    ///
    /// // Different rates produce different key
    /// let rates_diff = [0.03, 0.035, 0.041];
    /// let key3 = CurveKey::from_rates(RateIndex::Sofr, &rates_diff, config_hash);
    /// assert_ne!(key, key3);
    /// ```
    #[must_use]
    pub fn from_rates<T: Float>(index: RateIndex, rates: &[T], config_hash: u64) -> Self {
        let rates_hash = Self::hash_rates(rates);
        Self::new(index, rates_hash, config_hash)
    }

    /// Computes a deterministic hash of the rates array.
    ///
    /// Uses `OrderedFloat` to ensure consistent hashing of floating-point
    /// values.
    #[must_use]
    pub fn hash_rates<T: Float>(rates: &[T]) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the length first
        rates.len().hash(&mut hasher);

        // Hash each rate as OrderedFloat
        for rate in rates {
            if let Some(f) = rate.to_f64() {
                OrderedFloat(f).hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Computes a hash for a hashable configuration.
    #[must_use]
    pub fn hash_config<C: Hash>(config: &C) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        config.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the rate index.
    #[must_use]
    pub fn index(&self) -> RateIndex { self.index }

    /// Returns the rates hash.
    #[must_use]
    pub fn rates_hash(&self) -> u64 { self.rates_hash }

    /// Returns the config hash.
    #[must_use]
    pub fn config_hash(&self) -> u64 { self.config_hash }
}

/// Statistics for cache performance monitoring.
///
/// Tracks cache hits, misses, and current entry count to enable
/// performance analysis and tuning.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::CacheStats;
///
/// let stats = CacheStats::new(10, 5, 8);
/// assert!((stats.hit_rate() - 0.666666).abs() < 0.001);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Current number of cached entries.
    pub entries: usize,
}

impl CacheStats {
    /// Creates a new statistics instance.
    #[must_use]
    pub fn new(hits: u64, misses: u64, entries: usize) -> Self {
        Self {
            hits,
            misses,
            entries,
        }
    }

    /// Calculates the cache hit rate.
    ///
    /// Returns 0.0 if there are no lookups, otherwise returns
    /// hits / (hits + misses).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CacheStats;
    ///
    /// let stats = CacheStats::new(80, 20, 50);
    /// assert!((stats.hit_rate() - 0.80).abs() < 1e-10);
    ///
    /// let empty = CacheStats::default();
    /// assert!((empty.hit_rate() - 0.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Returns the total number of lookups.
    #[must_use]
    pub fn total_lookups(&self) -> u64 { self.hits + self.misses }
}

/// Thread-safe LRU cache for bootstrapped yield curves.
///
/// Provides caching to avoid redundant curve construction when the same
/// inputs are requested multiple times. Uses `parking_lot::RwLock` for
/// efficient concurrent access.
///
/// # Thread Safety
///
/// - Multiple threads can perform lookups concurrently (read lock)
/// - Insertions and clears acquire an exclusive write lock
/// - Statistics updates are atomic within the lock scope
///
/// # Capacity
///
/// The cache uses LRU (Least Recently Used) eviction. When capacity is
/// reached, the least recently accessed entry is removed.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::{
///     CurveResultCache, CurveKey, BootstrappedCurve,
/// };
/// use infra_master::market::RateIndex;
///
/// let cache: CurveResultCache<f64> = CurveResultCache::new(10);
///
/// // Create a curve (simplified)
/// let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
///
/// // Lookup miss
/// assert!(cache.lookup(&key).is_none());
/// assert_eq!(cache.stats().misses, 1);
/// ```
pub struct CurveResultCache<T: Float> {
    /// The LRU cache wrapped in a read-write lock.
    cache: Arc<RwLock<LruCache<CurveKey, BootstrappedCurve<T>>>>,
    /// Statistics tracking wrapped in a read-write lock.
    stats: Arc<RwLock<CacheStatsInternal>>,
}

/// Internal mutable statistics.
#[derive(Debug, Default)]
struct CacheStatsInternal {
    hits: u64,
    misses: u64,
}

impl<T: Float> CurveResultCache<T> {
    /// Creates a new cache with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of curves to cache (must be > 0)
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveResultCache;
    ///
    /// let cache: CurveResultCache<f64> = CurveResultCache::new(100);
    /// ```
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).expect("Cache capacity must be > 0");
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            stats: Arc::new(RwLock::new(CacheStatsInternal::default())),
        }
    }

    /// Looks up a curve in the cache.
    ///
    /// Returns a clone of the cached curve if found, or `None` if not present.
    /// Updates hit/miss statistics accordingly.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    ///
    /// `Some(curve)` if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::{
    ///     CurveResultCache, CurveKey,
    /// };
    /// use infra_master::market::RateIndex;
    ///
    /// let cache: CurveResultCache<f64> = CurveResultCache::new(10);
    /// let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
    ///
    /// // Miss
    /// assert!(cache.lookup(&key).is_none());
    /// assert_eq!(cache.stats().misses, 1);
    /// ```
    pub fn lookup(&self, key: &CurveKey) -> Option<BootstrappedCurve<T>> {
        // First try read lock for lookup
        {
            let cache = self.cache.read();
            if let Some(curve) = cache.peek(key) {
                // Found - update stats and return clone
                let mut stats = self.stats.write();
                stats.hits += 1;
                return Some(curve.clone());
            }
        }

        // Not found - record miss
        {
            let mut stats = self.stats.write();
            stats.misses += 1;
        }

        None
    }

    /// Inserts a curve into the cache.
    ///
    /// If the cache is at capacity, the least recently used entry is evicted.
    /// If a curve with the same key exists, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `curve` - The bootstrapped curve to cache
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pricer_models::market::calibration::bootstrapping::{
    ///     CurveResultCache, CurveKey, BootstrappedCurve,
    /// };
    /// use infra_master::market::RateIndex;
    ///
    /// let cache: CurveResultCache<f64> = CurveResultCache::new(10);
    /// let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
    ///
    /// // Insert a curve
    /// let curve = /* create curve */;
    /// cache.insert(key.clone(), curve);
    ///
    /// // Now lookup returns the curve
    /// assert!(cache.lookup(&key).is_some());
    /// ```
    pub fn insert(&self, key: CurveKey, curve: BootstrappedCurve<T>) {
        let mut cache = self.cache.write();
        cache.put(key, curve);
    }

    /// Clears all entries from the cache.
    ///
    /// Does not reset statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveResultCache;
    ///
    /// let cache: CurveResultCache<f64> = CurveResultCache::new(10);
    /// // ... insert some entries ...
    /// cache.clear();
    /// assert_eq!(cache.len(), 0);
    /// ```
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Returns current cache statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveResultCache;
    ///
    /// let cache: CurveResultCache<f64> = CurveResultCache::new(10);
    /// let stats = cache.stats();
    /// assert_eq!(stats.hits, 0);
    /// assert_eq!(stats.misses, 0);
    /// ```
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let stats = self.stats.read();
        let cache = self.cache.read();
        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            entries: cache.len(),
        }
    }

    /// Resets the statistics counters.
    ///
    /// Does not clear the cache entries.
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        stats.hits = 0;
        stats.misses = 0;
    }

    /// Returns the number of entries currently in the cache.
    #[must_use]
    pub fn len(&self) -> usize { self.cache.read().len() }

    /// Returns true if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.cache.read().is_empty() }

    /// Returns the cache capacity.
    #[must_use]
    pub fn capacity(&self) -> usize { self.cache.read().cap().get() }

    /// Checks if a key exists in the cache without updating LRU order.
    #[must_use]
    pub fn contains(&self, key: &CurveKey) -> bool { self.cache.read().peek(key).is_some() }
}

impl<T: Float> Clone for CurveResultCache<T> {
    fn clone(&self) -> Self {
        // Create a new cache with the same capacity
        let capacity = self.capacity();
        let new_cache = Self::new(capacity);

        // Copy statistics
        {
            let old_stats = self.stats.read();
            let mut new_stats = new_cache.stats.write();
            new_stats.hits = old_stats.hits;
            new_stats.misses = old_stats.misses;
        }

        // Note: We don't copy cache entries as BootstrappedCurve may not
        // implement Clone efficiently for all T. The new cache starts empty.

        new_cache
    }
}

impl<T: Float> Default for CurveResultCache<T> {
    /// Creates a cache with default capacity of 100 entries.
    fn default() -> Self { Self::new(100) }
}

impl<T: Float> std::fmt::Debug for CurveResultCache<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("CurveResultCache")
            .field("capacity", &self.capacity())
            .field("entries", &stats.entries)
            .field("hits", &stats.hits)
            .field("misses", &stats.misses)
            .field("hit_rate", &stats.hit_rate())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // CurveKey Tests
    // ========================================

    #[test]
    fn test_curve_key_new() {
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        assert_eq!(key.index(), RateIndex::Sofr);
        assert_eq!(key.rates_hash(), 12345);
        assert_eq!(key.config_hash(), 67890);
    }

    #[test]
    fn test_curve_key_equality() {
        let key1 = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let key2 = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let key3 = CurveKey::new(RateIndex::Sofr, 12345, 99999);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_curve_key_hash_deterministic() {
        let rates = [0.03_f64, 0.035, 0.04, 0.045, 0.05];

        let hash1 = CurveKey::hash_rates(&rates);
        let hash2 = CurveKey::hash_rates(&rates);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_curve_key_hash_different_rates() {
        let rates1 = [0.03_f64, 0.035, 0.04];
        let rates2 = [0.03_f64, 0.035, 0.041];

        let hash1 = CurveKey::hash_rates(&rates1);
        let hash2 = CurveKey::hash_rates(&rates2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_curve_key_hash_different_length() {
        let rates1 = [0.03_f64, 0.035];
        let rates2 = [0.03_f64, 0.035, 0.04];

        let hash1 = CurveKey::hash_rates(&rates1);
        let hash2 = CurveKey::hash_rates(&rates2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_curve_key_from_rates() {
        let rates = [0.03_f64, 0.035, 0.04];
        let config_hash = 12345_u64;

        let key1 = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);
        let key2 = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_curve_key_hash_config() {
        // Use types that implement Hash (f64 doesn't implement Hash)
        let config1 = ("interpolation", "log_linear", 100_i32);
        let config2 = ("interpolation", "log_linear", 100_i32);
        let config3 = ("interpolation", "linear", 100_i32);

        let hash1 = CurveKey::hash_config(&config1);
        let hash2 = CurveKey::hash_config(&config2);
        let hash3 = CurveKey::hash_config(&config3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_curve_key_clone() {
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let cloned = key.clone();
        assert_eq!(key, cloned);
    }

    #[test]
    fn test_curve_key_debug() {
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("CurveKey"));
        assert!(debug_str.contains("12345"));
    }

    #[test]
    fn test_curve_key_as_hash_map_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);

        map.insert(key.clone(), "curve1");
        assert_eq!(map.get(&key), Some(&"curve1"));
    }

    // ========================================
    // CacheStats Tests
    // ========================================

    #[test]
    fn test_cache_stats_new() {
        let stats = CacheStats::new(10, 5, 8);
        assert_eq!(stats.hits, 10);
        assert_eq!(stats.misses, 5);
        assert_eq!(stats.entries, 8);
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats::new(80, 20, 50);
        assert!((stats.hit_rate() - 0.80).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_hit_rate_zero_lookups() {
        let stats = CacheStats::default();
        assert!((stats.hit_rate() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_hit_rate_all_hits() {
        let stats = CacheStats::new(100, 0, 10);
        assert!((stats.hit_rate() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_hit_rate_all_misses() {
        let stats = CacheStats::new(0, 100, 10);
        assert!((stats.hit_rate() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_cache_stats_total_lookups() {
        let stats = CacheStats::new(80, 20, 50);
        assert_eq!(stats.total_lookups(), 100);
    }

    #[test]
    fn test_cache_stats_clone() {
        let stats = CacheStats::new(10, 5, 8);
        let cloned = stats.clone();
        assert_eq!(stats.hits, cloned.hits);
        assert_eq!(stats.misses, cloned.misses);
        assert_eq!(stats.entries, cloned.entries);
    }

    #[test]
    fn test_cache_stats_debug() {
        let stats = CacheStats::new(10, 5, 8);
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("CacheStats"));
        assert!(debug_str.contains("10"));
    }

    // ========================================
    // CurveResultCache Tests
    // ========================================

    #[test]
    fn test_cache_new() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(100);
        assert_eq!(cache.capacity(), 100);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_default() {
        let cache: CurveResultCache<f64> = CurveResultCache::default();
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_cache_zero_capacity_panics() {
        let _cache: CurveResultCache<f64> = CurveResultCache::new(0);
    }

    #[test]
    fn test_cache_lookup_miss() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);

        let result = cache.lookup(&key);
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_multiple_misses() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);

        for i in 0..5 {
            let key = CurveKey::new(RateIndex::Sofr, i, 0);
            assert!(cache.lookup(&key).is_none());
        }

        let stats = cache.stats();
        assert_eq!(stats.misses, 5);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_cache_clear() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);

        // Do some lookups to generate stats
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let _ = cache.lookup(&key);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        // Stats are preserved after clear
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_reset_stats() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);

        // Generate some stats
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let _ = cache.lookup(&key);
        let _ = cache.lookup(&key);

        cache.reset_stats();

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_contains() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);

        assert!(!cache.contains(&key));

        // Note: We can't easily test contains after insert without
        // constructing a BootstrappedCurve, which is complex
    }

    #[test]
    fn test_cache_clone() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(50);

        // Generate stats
        let key = CurveKey::new(RateIndex::Sofr, 12345, 67890);
        let _ = cache.lookup(&key);

        let cloned = cache.clone();

        assert_eq!(cloned.capacity(), 50);
        // Stats are copied
        let stats = cloned.stats();
        assert_eq!(stats.misses, 1);
        // But cache entries are not (starts empty)
        assert!(cloned.is_empty());
    }

    #[test]
    fn test_cache_debug() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);
        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("CurveResultCache"));
        assert!(debug_str.contains("capacity"));
    }

    #[test]
    fn test_cache_stats_entries() {
        let cache: CurveResultCache<f64> = CurveResultCache::new(10);
        let stats = cache.stats();
        assert_eq!(stats.entries, 0);
    }

    // ========================================
    // Thread Safety Tests
    // ========================================

    #[test]
    fn test_cache_concurrent_lookups() {
        use std::thread;

        let cache: CurveResultCache<f64> = CurveResultCache::new(100);
        let cache = Arc::new(cache);

        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let key = CurveKey::new(RateIndex::Sofr, i * 1000 + j, 0);
                    let _ = cache_clone.lookup(&key);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = cache.stats();
        assert_eq!(stats.misses, 1000); // 10 threads * 100 lookups
    }

    #[test]
    fn test_cache_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CurveResultCache<f64>>();
    }

    // ========================================
    // OrderedFloat Hash Tests
    // ========================================

    #[test]
    fn test_ordered_float_nan_handling() {
        // NaN values should hash consistently
        let rates_with_nan = [0.03_f64, f64::NAN, 0.04];
        let hash1 = CurveKey::hash_rates(&rates_with_nan);
        let hash2 = CurveKey::hash_rates(&rates_with_nan);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_ordered_float_infinity_handling() {
        let rates_with_inf = [0.03_f64, f64::INFINITY, 0.04];
        let hash1 = CurveKey::hash_rates(&rates_with_inf);
        let hash2 = CurveKey::hash_rates(&rates_with_inf);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_ordered_float_negative_zero() {
        // -0.0 and 0.0 should produce same hash with OrderedFloat
        let rates1 = [0.0_f64];
        let rates2 = [-0.0_f64];

        let hash1 = CurveKey::hash_rates(&rates1);
        let hash2 = CurveKey::hash_rates(&rates2);

        // OrderedFloat treats -0.0 and 0.0 as equal
        assert_eq!(hash1, hash2);
    }
}

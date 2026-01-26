//! Lazy Evaluator for IRS pricing with dependency tracking and caching.
//!
//! This module implements:
//! - Task 3.1: Dependency graph construction
//! - Task 3.2: Cache management
//! - Task 3.3: Curve update notification and selective recalculation
//! - Task 3.4: AAD tape reuse for efficient recomputation
//!
//! # Architecture
//!
//! The `IrsLazyEvaluator` manages:
//! - **Dependency Graph**: Tracks which curve tenor points affect each IRS
//! - **Result Cache**: Caches computation results keyed by swap+curve+date
//! - **Change Propagation**: Invalidates only affected cache entries on curve
//!   updates
//! - **AAD Tape Cache**: Caches AAD tapes for efficient recomputation when only
//!   values change
//!
//! # Requirements Coverage
//!
//! - Requirement 3.1: Curve change -> re-execute only dependent calculations
//! - Requirement 3.2: Cache results for repeated queries with same market data
//! - Requirement 3.3: Dependency graph construction and change propagation
//! - Requirement 3.4: Auto-recalculate on cache invalidation
//! - Requirement 3.5: AAD tape reuse capability

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use num_traits::Float;

// =============================================================================
// Task 3.1: Dependency Graph Types
// =============================================================================

/// Unique identifier for a swap in the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwapId(String);

impl SwapId {
    /// Create a new SwapId from a string.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Get the string representation.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for SwapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

/// A curve tenor point identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TenorPoint {
    /// Curve identifier (numeric for non-l1l2 mode)
    pub curve_id: u32,
    /// Tenor in years
    pub tenor: f64,
}

impl Eq for TenorPoint {}

impl Hash for TenorPoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.curve_id.hash(state);
        self.tenor.to_bits().hash(state);
    }
}

impl TenorPoint {
    /// Create a new tenor point.
    pub fn new(curve_id: u32, tenor: f64) -> Self { Self { curve_id, tenor } }
}

/// Dependency graph tracking curve tenor -> swap relationships.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Maps curve tenor points to the set of swaps that depend on them.
    tenor_to_swaps: HashMap<TenorPoint, HashSet<SwapId>>,

    /// Maps swap IDs to the set of tenor points they depend on.
    swap_to_tenors: HashMap<SwapId, HashSet<TenorPoint>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self { Self::default() }

    /// Register a dependency between a swap and a tenor point.
    pub fn add_dependency(&mut self, swap_id: SwapId, tenor_point: TenorPoint) {
        self.tenor_to_swaps
            .entry(tenor_point)
            .or_default()
            .insert(swap_id.clone());

        self.swap_to_tenors
            .entry(swap_id)
            .or_default()
            .insert(tenor_point);
    }

    /// Register multiple dependencies for a swap.
    pub fn add_dependencies(
        &mut self,
        swap_id: SwapId,
        tenor_points: impl IntoIterator<Item = TenorPoint>,
    ) {
        for point in tenor_points {
            self.add_dependency(swap_id.clone(), point);
        }
    }

    /// Remove all dependencies for a swap.
    pub fn remove_swap(&mut self, swap_id: &SwapId) {
        if let Some(tenors) = self.swap_to_tenors.remove(swap_id) {
            for tenor in tenors {
                if let Some(swaps) = self.tenor_to_swaps.get_mut(&tenor) {
                    swaps.remove(swap_id);
                    if swaps.is_empty() {
                        self.tenor_to_swaps.remove(&tenor);
                    }
                }
            }
        }
    }

    /// Get all swaps that depend on a specific tenor point.
    pub fn get_affected_swaps(&self, tenor_point: &TenorPoint) -> impl Iterator<Item = &SwapId> {
        self.tenor_to_swaps
            .get(tenor_point)
            .map(|set| set.iter())
            .into_iter()
            .flatten()
    }

    /// Get all tenor points that a swap depends on.
    pub fn get_swap_dependencies(&self, swap_id: &SwapId) -> impl Iterator<Item = &TenorPoint> {
        self.swap_to_tenors
            .get(swap_id)
            .map(|set| set.iter())
            .into_iter()
            .flatten()
    }

    /// Check if a swap has any dependencies registered.
    pub fn has_dependencies(&self, swap_id: &SwapId) -> bool {
        self.swap_to_tenors
            .get(swap_id)
            .is_some_and(|set| !set.is_empty())
    }

    /// Get the total number of swaps tracked.
    pub fn swap_count(&self) -> usize { self.swap_to_tenors.len() }

    /// Get the total number of unique tenor points tracked.
    pub fn tenor_count(&self) -> usize { self.tenor_to_swaps.len() }

    /// Clear all dependencies.
    pub fn clear(&mut self) {
        self.tenor_to_swaps.clear();
        self.swap_to_tenors.clear();
    }
}

// =============================================================================
// Task 3.2: Cache Types
// =============================================================================

/// Cache key for IRS computation results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Hash of the swap parameters
    swap_hash: u64,
    /// Version number of the curve set
    curve_version: u64,
    /// Valuation date (as days since epoch)
    valuation_date_days: i64,
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(swap_hash: u64, curve_version: u64, valuation_date_days: i64) -> Self {
        Self {
            swap_hash,
            curve_version,
            valuation_date_days,
        }
    }
}

/// Cached computation result.
#[derive(Debug, Clone)]
pub struct CachedResult<T: Float> {
    /// The cached NPV value
    pub npv: T,
    /// DV01 if computed
    pub dv01: Option<T>,
    /// Tenor deltas if computed
    pub deltas: Option<Vec<T>>,
    /// Timestamp when cached (nanoseconds since epoch)
    pub cached_at_ns: u64,
}

impl<T: Float> CachedResult<T> {
    /// Create a new cached result with NPV only.
    pub fn new(npv: T, cached_at_ns: u64) -> Self {
        Self {
            npv,
            dv01: None,
            deltas: None,
            cached_at_ns,
        }
    }

    /// Add DV01 to the cached result.
    pub fn with_dv01(mut self, dv01: T) -> Self {
        self.dv01 = Some(dv01);
        self
    }

    /// Add deltas to the cached result.
    pub fn with_deltas(mut self, deltas: Vec<T>) -> Self {
        self.deltas = Some(deltas);
        self
    }
}

/// Cache statistics for monitoring and debugging.
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache invalidations
    pub invalidations: u64,
    /// Number of AAD tape reuses
    pub tape_reuses: u64,
}

impl CacheStats {
    /// Create new cache statistics.
    pub fn new() -> Self { Self::default() }

    /// Calculate the cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) { self.hits += 1; }

    /// Record a cache miss.
    pub fn record_miss(&mut self) { self.misses += 1; }

    /// Record a cache invalidation.
    pub fn record_invalidation(&mut self) { self.invalidations += 1; }

    /// Record a tape reuse.
    pub fn record_tape_reuse(&mut self) { self.tape_reuses += 1; }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.invalidations = 0;
        self.tape_reuses = 0;
    }
}

// =============================================================================
// Task 3.3: Lazy Evaluator
// =============================================================================

/// State of a cache entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    /// Cache entry is valid and up to date
    Clean,
    /// Cache entry has been invalidated and needs recomputation
    Dirty,
}

/// Lazy Evaluator for IRS pricing with dependency tracking and caching.
#[derive(Debug)]
pub struct IrsLazyEvaluator<T: Float> {
    /// Result cache
    cache: HashMap<CacheKey, CachedResult<T>>,
    /// Cache state tracking (which entries are dirty)
    cache_state: HashMap<CacheKey, CacheState>,
    /// Dependency graph
    dependency_graph: DependencyGraph,
    /// Cache statistics
    stats: CacheStats,
    /// Current curve version (incremented on updates)
    curve_version: u64,
    /// Mapping from SwapId to CacheKey for invalidation
    swap_to_cache_key: HashMap<SwapId, CacheKey>,
    /// AAD tape cache for efficient recomputation
    tape_cache: AadTapeCache,
    /// Mapping from SwapId to structure hash for tape lookups
    swap_to_structure_hash: HashMap<SwapId, u64>,
}

impl<T: Float> Default for IrsLazyEvaluator<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float> IrsLazyEvaluator<T> {
    /// Create a new Lazy Evaluator.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_state: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            stats: CacheStats::new(),
            curve_version: 0,
            swap_to_cache_key: HashMap::new(),
            tape_cache: AadTapeCache::new(),
            swap_to_structure_hash: HashMap::new(),
        }
    }

    /// Create a new Lazy Evaluator with specified tape cache capacity.
    pub fn with_tape_capacity(tape_cache_capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            cache_state: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            stats: CacheStats::new(),
            curve_version: 0,
            swap_to_cache_key: HashMap::new(),
            tape_cache: AadTapeCache::with_capacity(tape_cache_capacity),
            swap_to_structure_hash: HashMap::new(),
        }
    }

    /// Get cached result if available and valid.
    pub fn get_cached(&mut self, key: &CacheKey) -> Option<&CachedResult<T>> {
        if let Some(&CacheState::Clean) = self.cache_state.get(key) {
            if let Some(result) = self.cache.get(key) {
                self.stats.record_hit();
                return Some(result);
            }
        }
        self.stats.record_miss();
        None
    }

    /// Store a computation result in the cache.
    pub fn store(&mut self, key: CacheKey, result: CachedResult<T>, swap_id: SwapId) {
        self.cache.insert(key.clone(), result);
        self.cache_state.insert(key.clone(), CacheState::Clean);
        self.swap_to_cache_key.insert(swap_id, key);
    }

    /// Register dependencies for a swap.
    pub fn register_dependencies(
        &mut self,
        swap_id: SwapId,
        tenor_points: impl IntoIterator<Item = TenorPoint>,
    ) {
        self.dependency_graph
            .add_dependencies(swap_id, tenor_points);
    }

    /// Notify of a curve update and invalidate affected cache entries.
    pub fn notify_curve_update(&mut self, curve_id: u32, tenor: f64) {
        let tenor_point = TenorPoint::new(curve_id, tenor);

        let affected_swaps: Vec<SwapId> = self
            .dependency_graph
            .get_affected_swaps(&tenor_point)
            .cloned()
            .collect();

        for swap_id in affected_swaps {
            if let Some(cache_key) = self.swap_to_cache_key.get(&swap_id) {
                if self.cache_state.contains_key(cache_key) {
                    self.cache_state
                        .insert(cache_key.clone(), CacheState::Dirty);
                    self.stats.record_invalidation();
                }
            }
        }

        self.curve_version += 1;
    }

    /// Notify of a full curve update (all tenor points changed).
    pub fn notify_full_curve_update(&mut self, curve_id: u32) {
        let tenor_points: Vec<TenorPoint> = self
            .dependency_graph
            .tenor_to_swaps
            .keys()
            .filter(|tp| tp.curve_id == curve_id)
            .copied()
            .collect();

        for point in tenor_points {
            self.notify_curve_update(curve_id, point.tenor);
        }
    }

    /// Invalidate all cache entries.
    pub fn invalidate_all(&mut self) {
        for state in self.cache_state.values_mut() {
            *state = CacheState::Dirty;
            self.stats.record_invalidation();
        }
        self.curve_version += 1;
    }

    /// Check if a cache entry needs recomputation.
    pub fn needs_recompute(&self, key: &CacheKey) -> bool {
        match self.cache_state.get(key) {
            Some(CacheState::Clean) => false,
            Some(CacheState::Dirty) | None => true,
        }
    }

    /// Check if AAD tape can be reused for a swap.
    pub fn can_reuse_tape(&self, swap_id: &SwapId) -> bool {
        if let Some(structure_hash) = self.swap_to_structure_hash.get(swap_id) {
            self.tape_cache.has_tape(*structure_hash)
        } else {
            false
        }
    }

    /// Check if AAD tape can be reused for a given structure hash.
    pub fn can_reuse_tape_for_structure(&self, structure_hash: u64) -> bool {
        self.tape_cache.has_tape(structure_hash)
    }

    /// Mark that AAD tape was reused (for statistics).
    pub fn record_tape_reuse(&mut self) { self.stats.record_tape_reuse(); }

    /// Register an AAD tape for a swap.
    pub fn register_tape(
        &mut self,
        swap_id: SwapId,
        structure_hash: u64,
        tenor_count: usize,
    ) -> u64 {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let tape_id = self
            .tape_cache
            .store_tape(structure_hash, tenor_count, timestamp);

        self.swap_to_structure_hash.insert(swap_id, structure_hash);

        tape_id
    }

    /// Try to get a reusable tape for a swap.
    pub fn try_reuse_tape(&mut self, swap_id: &SwapId) -> Option<&CachedTape> {
        if let Some(structure_hash) = self.swap_to_structure_hash.get(swap_id).copied() {
            if let Some(tape) = self.tape_cache.get_tape_mut(structure_hash) {
                tape.record_reuse();
                self.stats.record_tape_reuse();
                return self.tape_cache.tapes.get(&structure_hash);
            }
        }
        None
    }

    /// Try to get a reusable tape by structure hash.
    pub fn try_reuse_tape_by_hash(&mut self, structure_hash: u64) -> Option<&CachedTape> {
        if let Some(tape) = self.tape_cache.get_tape_mut(structure_hash) {
            tape.record_reuse();
            self.stats.record_tape_reuse();
            return self.tape_cache.tapes.get(&structure_hash);
        }
        None
    }

    /// Invalidate the tape for a specific swap.
    pub fn invalidate_tape(&mut self, swap_id: &SwapId) -> bool {
        if let Some(structure_hash) = self.swap_to_structure_hash.remove(swap_id) {
            self.tape_cache.invalidate_tape(structure_hash)
        } else {
            false
        }
    }

    /// Invalidate all cached tapes.
    pub fn invalidate_all_tapes(&mut self) {
        self.tape_cache.invalidate_all();
        self.swap_to_structure_hash.clear();
    }

    /// Get the tape cache.
    pub fn tape_cache(&self) -> &AadTapeCache { &self.tape_cache }

    /// Get mutable reference to the tape cache.
    pub fn tape_cache_mut(&mut self) -> &mut AadTapeCache { &mut self.tape_cache }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> &CacheStats { &self.stats }

    /// Get mutable reference to cache statistics.
    pub fn cache_stats_mut(&mut self) -> &mut CacheStats { &mut self.stats }

    /// Get the current curve version.
    pub fn curve_version(&self) -> u64 { self.curve_version }

    /// Get the dependency graph.
    pub fn dependency_graph(&self) -> &DependencyGraph { &self.dependency_graph }

    /// Get mutable reference to the dependency graph.
    pub fn dependency_graph_mut(&mut self) -> &mut DependencyGraph { &mut self.dependency_graph }

    /// Get the number of cached entries.
    pub fn cache_size(&self) -> usize { self.cache.len() }

    /// Clear all cache entries and reset statistics.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.cache_state.clear();
        self.swap_to_cache_key.clear();
        self.dependency_graph.clear();
        self.stats.reset();
        self.curve_version = 0;
        self.tape_cache.invalidate_all();
        self.tape_cache.reset_stats();
        self.swap_to_structure_hash.clear();
    }

    /// Remove a specific swap from the evaluator.
    pub fn remove_swap(&mut self, swap_id: &SwapId) {
        self.dependency_graph.remove_swap(swap_id);
        if let Some(cache_key) = self.swap_to_cache_key.remove(swap_id) {
            self.cache.remove(&cache_key);
            self.cache_state.remove(&cache_key);
        }
        if let Some(structure_hash) = self.swap_to_structure_hash.remove(swap_id) {
            self.tape_cache.invalidate_tape(structure_hash);
        }
    }
}

// =============================================================================
// Task 3.4: AAD Tape Cache Types
// =============================================================================

/// Cached AAD tape information.
#[derive(Debug, Clone)]
pub struct CachedTape {
    /// Unique identifier for the tape
    pub tape_id: u64,
    /// Hash of the swap structure (parameters that affect tape structure)
    pub structure_hash: u64,
    /// Number of tenor points the tape was computed for
    pub tenor_count: usize,
    /// Timestamp when the tape was created (nanoseconds since epoch)
    pub created_at_ns: u64,
    /// Number of times this tape has been reused
    pub reuse_count: u64,
}

impl CachedTape {
    /// Create a new cached tape entry.
    pub fn new(tape_id: u64, structure_hash: u64, tenor_count: usize, created_at_ns: u64) -> Self {
        Self {
            tape_id,
            structure_hash,
            tenor_count,
            created_at_ns,
            reuse_count: 0,
        }
    }

    /// Record a reuse of this tape.
    pub fn record_reuse(&mut self) { self.reuse_count += 1; }
}

/// AAD Tape Cache for managing reusable computation tapes.
#[derive(Debug, Default)]
pub struct AadTapeCache {
    /// Map from swap structure hash to cached tape
    pub(crate) tapes: HashMap<u64, CachedTape>,
    /// Next tape ID to assign
    next_tape_id: u64,
    /// Maximum number of tapes to cache
    max_tapes: usize,
    /// Statistics about tape operations
    stats: TapeCacheStats,
}

/// Statistics for tape cache operations.
#[derive(Debug, Clone, Default)]
pub struct TapeCacheStats {
    /// Number of tape cache hits
    pub hits: u64,
    /// Number of tape cache misses
    pub misses: u64,
    /// Number of tapes created
    pub tapes_created: u64,
    /// Number of tapes evicted due to capacity
    pub tapes_evicted: u64,
    /// Total number of tape reuses
    pub total_reuses: u64,
}

impl TapeCacheStats {
    /// Calculate the tape cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.tapes_created = 0;
        self.tapes_evicted = 0;
        self.total_reuses = 0;
    }
}

impl AadTapeCache {
    /// Create a new tape cache with default capacity.
    pub fn new() -> Self { Self::with_capacity(100) }

    /// Create a new tape cache with specified capacity.
    pub fn with_capacity(max_tapes: usize) -> Self {
        Self {
            tapes: HashMap::new(),
            next_tape_id: 1,
            max_tapes,
            stats: TapeCacheStats::default(),
        }
    }

    /// Check if a tape exists for the given structure hash.
    pub fn has_tape(&self, structure_hash: u64) -> bool { self.tapes.contains_key(&structure_hash) }

    /// Get a cached tape if available.
    pub fn get_tape(&mut self, structure_hash: u64) -> Option<&CachedTape> {
        if self.tapes.contains_key(&structure_hash) {
            self.stats.hits += 1;
            self.tapes.get(&structure_hash)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Get a mutable reference to a cached tape.
    pub fn get_tape_mut(&mut self, structure_hash: u64) -> Option<&mut CachedTape> {
        if self.tapes.contains_key(&structure_hash) {
            self.stats.hits += 1;
            self.stats.total_reuses += 1;
            self.tapes.get_mut(&structure_hash)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Store a new tape in the cache.
    pub fn store_tape(
        &mut self,
        structure_hash: u64,
        tenor_count: usize,
        created_at_ns: u64,
    ) -> u64 {
        if self.tapes.len() >= self.max_tapes && !self.tapes.contains_key(&structure_hash) {
            self.evict_lru();
        }

        let tape_id = self.next_tape_id;
        self.next_tape_id += 1;

        let tape = CachedTape::new(tape_id, structure_hash, tenor_count, created_at_ns);
        self.tapes.insert(structure_hash, tape);
        self.stats.tapes_created += 1;

        tape_id
    }

    /// Evict the least recently used tape.
    fn evict_lru(&mut self) {
        if self.tapes.is_empty() {
            return;
        }

        let lru_hash = self
            .tapes
            .iter()
            .min_by(|a, b| {
                let cmp = a.1.reuse_count.cmp(&b.1.reuse_count);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
                a.1.created_at_ns.cmp(&b.1.created_at_ns)
            })
            .map(|(hash, _)| *hash);

        if let Some(hash) = lru_hash {
            self.tapes.remove(&hash);
            self.stats.tapes_evicted += 1;
        }
    }

    /// Invalidate a specific tape.
    pub fn invalidate_tape(&mut self, structure_hash: u64) -> bool {
        self.tapes.remove(&structure_hash).is_some()
    }

    /// Invalidate all cached tapes.
    pub fn invalidate_all(&mut self) { self.tapes.clear(); }

    /// Get the number of cached tapes.
    pub fn tape_count(&self) -> usize { self.tapes.len() }

    /// Get tape cache statistics.
    pub fn stats(&self) -> &TapeCacheStats { &self.stats }

    /// Reset statistics.
    pub fn reset_stats(&mut self) { self.stats.reset(); }

    /// Get the maximum tape capacity.
    pub fn capacity(&self) -> usize { self.max_tapes }
}

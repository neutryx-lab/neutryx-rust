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
//! - **Change Propagation**: Invalidates only affected cache entries on curve updates
//! - **AAD Tape Cache**: Caches AAD tapes for efficient recomputation when only values change
//!
//! # Requirements Coverage
//!
//! - Requirement 3.1: Curve change -> re-execute only dependent calculations
//! - Requirement 3.2: Cache results for repeated queries with same market data
//! - Requirement 3.3: Dependency graph construction and change propagation
//! - Requirement 3.4: Auto-recalculate on cache invalidation
//! - Requirement 3.5: AAD tape reuse capability
//!
//! # AAD Tape Reuse (Task 3.4)
//!
//! The `AadTapeCache` enables efficient AAD calculations by caching the
//! computational graph (tape) and reusing it when only input values change.
//!
//! Key concepts:
//! - **Structure Hash**: Identifies swap structure (tenor count, schedule, etc.)
//! - **Tape Caching**: Stores tape metadata for future reuse
//! - **LRU Eviction**: Removes least-used tapes when cache is full
//!
//! ```rust,ignore
//! let mut evaluator = IrsLazyEvaluator::<f64>::new();
//!
//! // Register tape after AAD computation
//! let structure_hash = compute_structure_hash(&swap);
//! evaluator.register_tape(swap_id, structure_hash, tenor_count);
//!
//! // Check if tape can be reused
//! if evaluator.can_reuse_tape(&swap_id) {
//!     let tape = evaluator.try_reuse_tape(&swap_id);
//!     // Use tape for efficient recomputation
//! }
//! ```

use num_traits::Float;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

// =============================================================================
// Task 3.1: Dependency Graph Types
// =============================================================================

/// Unique identifier for a swap in the dependency graph.
///
/// Generated from swap parameters to enable lookup without storing
/// the full swap object.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwapId(String);

impl SwapId {
    /// Create a new SwapId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SwapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A curve tenor point identifier.
///
/// Combines curve name and tenor (in years) to identify a specific
/// point on a yield curve.
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
        // Hash tenor as bits to avoid floating point issues
        self.tenor.to_bits().hash(state);
    }
}

impl TenorPoint {
    /// Create a new tenor point.
    pub fn new(curve_id: u32, tenor: f64) -> Self {
        Self { curve_id, tenor }
    }
}

/// Dependency graph tracking curve tenor -> swap relationships.
///
/// # Design
///
/// The graph uses a sparse representation where only actual dependencies
/// are stored. This is efficient for typical IRS portfolios where each
/// swap depends on a subset of curve tenor points.
///
/// # Requirements Coverage
///
/// - Requirement 3.3: Dependency graph construction
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Maps curve tenor points to the set of swaps that depend on them.
    /// Key: TenorPoint, Value: Set of SwapIds
    tenor_to_swaps: HashMap<TenorPoint, HashSet<SwapId>>,

    /// Maps swap IDs to the set of tenor points they depend on.
    /// This reverse index enables efficient cache invalidation.
    swap_to_tenors: HashMap<SwapId, HashSet<TenorPoint>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a dependency between a swap and a tenor point.
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    /// * `tenor_point` - The curve tenor point the swap depends on
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
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    /// * `tenor_points` - Iterator of tenor points the swap depends on
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
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier to remove
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
    ///
    /// # Arguments
    ///
    /// * `tenor_point` - The tenor point to query
    ///
    /// # Returns
    ///
    /// Iterator over SwapIds that depend on this tenor point.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.3: Query for affected IRS on curve change
    pub fn get_affected_swaps(&self, tenor_point: &TenorPoint) -> impl Iterator<Item = &SwapId> {
        self.tenor_to_swaps
            .get(tenor_point)
            .map(|set| set.iter())
            .into_iter()
            .flatten()
    }

    /// Get all tenor points that a swap depends on.
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier to query
    ///
    /// # Returns
    ///
    /// Iterator over TenorPoints this swap depends on.
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
    pub fn swap_count(&self) -> usize {
        self.swap_to_tenors.len()
    }

    /// Get the total number of unique tenor points tracked.
    pub fn tenor_count(&self) -> usize {
        self.tenor_to_swaps.len()
    }

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
///
/// Uniquely identifies a computation by combining:
/// - Swap identity (via hash)
/// - Curve set version
/// - Valuation date
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
///
/// Stores the computed value along with metadata about when it was computed.
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
///
/// # Requirements Coverage
///
/// - Requirement 3.2: Track cache statistics (hits, misses, invalidations)
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the cache hit rate.
    ///
    /// Returns 0.0 if no lookups have occurred.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Record a cache invalidation.
    pub fn record_invalidation(&mut self) {
        self.invalidations += 1;
    }

    /// Record a tape reuse.
    pub fn record_tape_reuse(&mut self) {
        self.tape_reuses += 1;
    }

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
///
/// # Architecture
///
/// The evaluator maintains:
/// - A dependency graph tracking curve tenor -> swap relationships
/// - A result cache for computed values
/// - State tracking for cache validity
/// - An AAD tape cache for efficient recomputation
///
/// # Requirements Coverage
///
/// - Requirement 3.1: Re-execute only dependent calculations on curve change
/// - Requirement 3.2: Return cached results for repeated queries
/// - Requirement 3.3: Dependency graph for change propagation
/// - Requirement 3.4: Auto-recalculate on next evaluation after invalidation
/// - Requirement 3.5: AAD tape reuse capability
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
    /// Requirement 3.5: AAD tape reuse capability
    tape_cache: AadTapeCache,
    /// Mapping from SwapId to structure hash for tape lookups
    swap_to_structure_hash: HashMap<SwapId, u64>,
}

impl<T: Float> Default for IrsLazyEvaluator<T> {
    fn default() -> Self {
        Self::new()
    }
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
    ///
    /// # Arguments
    ///
    /// * `tape_cache_capacity` - Maximum number of tapes to cache
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
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to lookup
    ///
    /// # Returns
    ///
    /// `Some(&CachedResult)` if found and clean, `None` otherwise.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.2: Return cached results for repeated queries
    pub fn get_cached(&mut self, key: &CacheKey) -> Option<&CachedResult<T>> {
        // Check if entry exists and is clean
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
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `result` - The computation result to cache
    /// * `swap_id` - The swap identifier for dependency tracking
    pub fn store(&mut self, key: CacheKey, result: CachedResult<T>, swap_id: SwapId) {
        self.cache.insert(key.clone(), result);
        self.cache_state.insert(key.clone(), CacheState::Clean);
        self.swap_to_cache_key.insert(swap_id, key);
    }

    /// Register dependencies for a swap.
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    /// * `tenor_points` - The tenor points this swap depends on
    pub fn register_dependencies(
        &mut self,
        swap_id: SwapId,
        tenor_points: impl IntoIterator<Item = TenorPoint>,
    ) {
        self.dependency_graph
            .add_dependencies(swap_id, tenor_points);
    }

    /// Notify of a curve update and invalidate affected cache entries.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - The curve identifier that was updated
    /// * `tenor` - The tenor point that was updated
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.1: Invalidate only dependent calculations
    /// - Requirement 3.4: Mark for recalculation
    pub fn notify_curve_update(&mut self, curve_id: u32, tenor: f64) {
        let tenor_point = TenorPoint::new(curve_id, tenor);

        // Get all affected swaps
        let affected_swaps: Vec<SwapId> = self
            .dependency_graph
            .get_affected_swaps(&tenor_point)
            .cloned()
            .collect();

        // Invalidate cache entries for affected swaps
        for swap_id in affected_swaps {
            if let Some(cache_key) = self.swap_to_cache_key.get(&swap_id) {
                if self.cache_state.contains_key(cache_key) {
                    self.cache_state
                        .insert(cache_key.clone(), CacheState::Dirty);
                    self.stats.record_invalidation();
                }
            }
        }

        // Increment curve version
        self.curve_version += 1;
    }

    /// Notify of a full curve update (all tenor points changed).
    ///
    /// # Arguments
    ///
    /// * `curve_id` - The curve identifier that was updated
    pub fn notify_full_curve_update(&mut self, curve_id: u32) {
        // Find all tenor points for this curve and invalidate
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
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.4: Support full invalidation
    pub fn invalidate_all(&mut self) {
        for state in self.cache_state.values_mut() {
            *state = CacheState::Dirty;
            self.stats.record_invalidation();
        }
        self.curve_version += 1;
    }

    /// Check if a cache entry needs recomputation.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to check
    ///
    /// # Returns
    ///
    /// `true` if the entry is dirty or missing, `false` if clean.
    pub fn needs_recompute(&self, key: &CacheKey) -> bool {
        match self.cache_state.get(key) {
            Some(CacheState::Clean) => false,
            Some(CacheState::Dirty) | None => true,
        }
    }

    /// Check if AAD tape can be reused for a swap.
    ///
    /// Tape can be reused if:
    /// - A tape exists for the swap's structure hash
    /// - The swap structure hasn't changed (same tenor count, etc.)
    ///
    /// This is separate from result caching - tapes can be reused
    /// even when results need recalculation (e.g., after curve updates).
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier to check
    ///
    /// # Returns
    ///
    /// `true` if tape can be reused.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.5: AAD tape reuse capability
    pub fn can_reuse_tape(&self, swap_id: &SwapId) -> bool {
        if let Some(structure_hash) = self.swap_to_structure_hash.get(swap_id) {
            self.tape_cache.has_tape(*structure_hash)
        } else {
            false
        }
    }

    /// Check if AAD tape can be reused for a given structure hash.
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - The structure hash to check
    ///
    /// # Returns
    ///
    /// `true` if tape can be reused.
    pub fn can_reuse_tape_for_structure(&self, structure_hash: u64) -> bool {
        self.tape_cache.has_tape(structure_hash)
    }

    /// Mark that AAD tape was reused (for statistics).
    pub fn record_tape_reuse(&mut self) {
        self.stats.record_tape_reuse();
    }

    /// Register an AAD tape for a swap.
    ///
    /// Call this after computing AAD derivatives to cache the tape
    /// for potential reuse.
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    /// * `structure_hash` - Hash of the swap structure
    /// * `tenor_count` - Number of tenor points in the computation
    ///
    /// # Returns
    ///
    /// The tape ID assigned to this tape.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.5: AAD tape reuse capability
    pub fn register_tape(
        &mut self,
        swap_id: SwapId,
        structure_hash: u64,
        tenor_count: usize,
    ) -> u64 {
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // Store the tape
        let tape_id = self
            .tape_cache
            .store_tape(structure_hash, tenor_count, timestamp);

        // Map swap to structure hash
        self.swap_to_structure_hash.insert(swap_id, structure_hash);

        tape_id
    }

    /// Try to get a reusable tape for a swap.
    ///
    /// If a tape exists and can be reused, returns the cached tape
    /// and records the reuse in statistics.
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    ///
    /// # Returns
    ///
    /// Reference to the cached tape if available and reusable.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 3.5: AAD tape reuse capability
    pub fn try_reuse_tape(&mut self, swap_id: &SwapId) -> Option<&CachedTape> {
        if let Some(structure_hash) = self.swap_to_structure_hash.get(swap_id).copied() {
            if let Some(tape) = self.tape_cache.get_tape_mut(structure_hash) {
                tape.record_reuse();
                self.stats.record_tape_reuse();
                // Return immutable reference after mutation
                return self.tape_cache.tapes.get(&structure_hash);
            }
        }
        None
    }

    /// Try to get a reusable tape by structure hash.
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - The structure hash to lookup
    ///
    /// # Returns
    ///
    /// Reference to the cached tape if available.
    pub fn try_reuse_tape_by_hash(&mut self, structure_hash: u64) -> Option<&CachedTape> {
        if let Some(tape) = self.tape_cache.get_tape_mut(structure_hash) {
            tape.record_reuse();
            self.stats.record_tape_reuse();
            return self.tape_cache.tapes.get(&structure_hash);
        }
        None
    }

    /// Invalidate the tape for a specific swap.
    ///
    /// Call this when the swap structure changes (not just curve values).
    ///
    /// # Arguments
    ///
    /// * `swap_id` - The swap identifier
    ///
    /// # Returns
    ///
    /// `true` if a tape was invalidated.
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
    pub fn tape_cache(&self) -> &AadTapeCache {
        &self.tape_cache
    }

    /// Get mutable reference to the tape cache.
    pub fn tape_cache_mut(&mut self) -> &mut AadTapeCache {
        &mut self.tape_cache
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get mutable reference to cache statistics.
    pub fn cache_stats_mut(&mut self) -> &mut CacheStats {
        &mut self.stats
    }

    /// Get the current curve version.
    pub fn curve_version(&self) -> u64 {
        self.curve_version
    }

    /// Get the dependency graph.
    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependency_graph
    }

    /// Get mutable reference to the dependency graph.
    pub fn dependency_graph_mut(&mut self) -> &mut DependencyGraph {
        &mut self.dependency_graph
    }

    /// Get the number of cached entries.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

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
    ///
    /// This removes the swap from:
    /// - The dependency graph
    /// - The cache
    /// - The swap-to-cache mapping
    /// - The tape cache (if applicable)
    pub fn remove_swap(&mut self, swap_id: &SwapId) {
        self.dependency_graph.remove_swap(swap_id);
        if let Some(cache_key) = self.swap_to_cache_key.remove(swap_id) {
            self.cache.remove(&cache_key);
            self.cache_state.remove(&cache_key);
        }
        // Also remove from tape cache
        if let Some(structure_hash) = self.swap_to_structure_hash.remove(swap_id) {
            self.tape_cache.invalidate_tape(structure_hash);
        }
    }
}

// =============================================================================
// Task 3.4: AAD Tape Cache Types
// =============================================================================

/// Cached AAD tape information.
///
/// Stores metadata about a cached tape that can be reused for efficient
/// AAD calculations when only curve parameters (not structure) change.
///
/// # Requirements Coverage
///
/// - Requirement 3.5: AAD tape reuse capability
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
    pub fn record_reuse(&mut self) {
        self.reuse_count += 1;
    }
}

/// AAD Tape Cache for managing reusable computation tapes.
///
/// The tape cache enables efficient AAD calculations by reusing the
/// computational graph (tape) when only input values change but the
/// structure remains the same.
///
/// # Architecture
///
/// When an IRS is first priced with AAD:
/// 1. A tape is generated recording the computation graph
/// 2. The tape is cached with a structure hash
/// 3. On subsequent pricing with same structure, the tape is reused
///
/// # Requirements Coverage
///
/// - Requirement 3.5: AAD tape reuse capability
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
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

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
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - Hash of the swap structure
    ///
    /// # Returns
    ///
    /// `true` if a cached tape exists for this structure.
    pub fn has_tape(&self, structure_hash: u64) -> bool {
        self.tapes.contains_key(&structure_hash)
    }

    /// Get a cached tape if available.
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - Hash of the swap structure
    ///
    /// # Returns
    ///
    /// Reference to the cached tape if found.
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
    ///
    /// Use this to update reuse count when a tape is actually reused.
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
    ///
    /// If the cache is at capacity, the least recently used tape
    /// is evicted (based on reuse count, then creation time).
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - Hash of the swap structure
    /// * `tenor_count` - Number of tenor points
    /// * `created_at_ns` - Creation timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// The ID of the cached tape.
    pub fn store_tape(
        &mut self,
        structure_hash: u64,
        tenor_count: usize,
        created_at_ns: u64,
    ) -> u64 {
        // Evict if at capacity
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
    ///
    /// LRU is determined by:
    /// 1. Lowest reuse count
    /// 2. Oldest creation time (ties broken)
    fn evict_lru(&mut self) {
        if self.tapes.is_empty() {
            return;
        }

        // Find LRU tape
        let lru_hash = self
            .tapes
            .iter()
            .min_by(|a, b| {
                // Primary: reuse count (lower = more evictable)
                let cmp = a.1.reuse_count.cmp(&b.1.reuse_count);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
                // Secondary: creation time (older = more evictable)
                a.1.created_at_ns.cmp(&b.1.created_at_ns)
            })
            .map(|(hash, _)| *hash);

        if let Some(hash) = lru_hash {
            self.tapes.remove(&hash);
            self.stats.tapes_evicted += 1;
        }
    }

    /// Invalidate a specific tape.
    ///
    /// # Arguments
    ///
    /// * `structure_hash` - Hash of the swap structure to invalidate
    ///
    /// # Returns
    ///
    /// `true` if a tape was invalidated.
    pub fn invalidate_tape(&mut self, structure_hash: u64) -> bool {
        self.tapes.remove(&structure_hash).is_some()
    }

    /// Invalidate all cached tapes.
    pub fn invalidate_all(&mut self) {
        self.tapes.clear();
    }

    /// Get the number of cached tapes.
    pub fn tape_count(&self) -> usize {
        self.tapes.len()
    }

    /// Get tape cache statistics.
    pub fn stats(&self) -> &TapeCacheStats {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Get the maximum tape capacity.
    pub fn capacity(&self) -> usize {
        self.max_tapes
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 3.4: AAD Tape Cache Tests (TDD: RED -> GREEN -> REFACTOR)
    // =========================================================================

    mod tape_cache_tests {
        use super::*;

        #[test]
        fn test_tape_cache_creation() {
            let cache = AadTapeCache::new();
            assert_eq!(cache.tape_count(), 0);
            assert_eq!(cache.capacity(), 100);
        }

        #[test]
        fn test_tape_cache_with_capacity() {
            let cache = AadTapeCache::with_capacity(50);
            assert_eq!(cache.tape_count(), 0);
            assert_eq!(cache.capacity(), 50);
        }

        #[test]
        fn test_store_and_retrieve_tape() {
            let mut cache = AadTapeCache::new();

            let structure_hash = 12345u64;
            let tape_id = cache.store_tape(structure_hash, 5, 1000);

            assert!(cache.has_tape(structure_hash));
            assert_eq!(cache.tape_count(), 1);

            let tape = cache.get_tape(structure_hash).unwrap();
            assert_eq!(tape.tape_id, tape_id);
            assert_eq!(tape.structure_hash, structure_hash);
            assert_eq!(tape.tenor_count, 5);
            assert_eq!(tape.created_at_ns, 1000);
            assert_eq!(tape.reuse_count, 0);
        }

        #[test]
        fn test_tape_not_found() {
            let mut cache = AadTapeCache::new();

            assert!(!cache.has_tape(99999));
            assert!(cache.get_tape(99999).is_none());
        }

        #[test]
        fn test_tape_reuse_tracking() {
            let mut cache = AadTapeCache::new();

            let structure_hash = 12345u64;
            cache.store_tape(structure_hash, 5, 1000);

            // Simulate multiple reuses
            {
                let tape = cache.get_tape_mut(structure_hash).unwrap();
                tape.record_reuse();
            }
            {
                let tape = cache.get_tape_mut(structure_hash).unwrap();
                tape.record_reuse();
            }
            {
                let tape = cache.get_tape_mut(structure_hash).unwrap();
                tape.record_reuse();
            }

            let tape = cache.get_tape(structure_hash).unwrap();
            assert_eq!(tape.reuse_count, 3);
        }

        #[test]
        fn test_tape_invalidation() {
            let mut cache = AadTapeCache::new();

            let structure_hash = 12345u64;
            cache.store_tape(structure_hash, 5, 1000);

            assert!(cache.has_tape(structure_hash));

            let invalidated = cache.invalidate_tape(structure_hash);
            assert!(invalidated);
            assert!(!cache.has_tape(structure_hash));
            assert_eq!(cache.tape_count(), 0);
        }

        #[test]
        fn test_tape_invalidate_all() {
            let mut cache = AadTapeCache::new();

            cache.store_tape(111, 5, 1000);
            cache.store_tape(222, 10, 2000);
            cache.store_tape(333, 15, 3000);

            assert_eq!(cache.tape_count(), 3);

            cache.invalidate_all();

            assert_eq!(cache.tape_count(), 0);
        }

        #[test]
        fn test_tape_cache_eviction_at_capacity() {
            let mut cache = AadTapeCache::with_capacity(3);

            // Fill to capacity
            cache.store_tape(111, 5, 1000);
            cache.store_tape(222, 5, 2000);
            cache.store_tape(333, 5, 3000);

            assert_eq!(cache.tape_count(), 3);

            // Add one more - should evict oldest (111)
            cache.store_tape(444, 5, 4000);

            assert_eq!(cache.tape_count(), 3);
            assert!(!cache.has_tape(111)); // Evicted (oldest, no reuses)
            assert!(cache.has_tape(222));
            assert!(cache.has_tape(333));
            assert!(cache.has_tape(444));
        }

        #[test]
        fn test_tape_cache_eviction_prefers_less_reused() {
            let mut cache = AadTapeCache::with_capacity(3);

            // Fill to capacity
            cache.store_tape(111, 5, 1000);
            cache.store_tape(222, 5, 2000);
            cache.store_tape(333, 5, 3000);

            // Reuse 111 multiple times
            {
                let tape = cache.get_tape_mut(111).unwrap();
                tape.record_reuse();
                tape.record_reuse();
            }

            // Add one more - should evict 222 (fewer reuses than 111, older than 333)
            cache.store_tape(444, 5, 4000);

            assert_eq!(cache.tape_count(), 3);
            assert!(cache.has_tape(111)); // Kept (most reuses)
            assert!(!cache.has_tape(222)); // Evicted (no reuses, oldest of unreused)
            assert!(cache.has_tape(333));
            assert!(cache.has_tape(444));
        }

        #[test]
        fn test_tape_cache_stats_hits_misses() {
            let mut cache = AadTapeCache::new();

            cache.store_tape(111, 5, 1000);

            // Hit
            cache.get_tape(111);
            // Miss
            cache.get_tape(999);
            // Hit
            cache.get_tape(111);

            let stats = cache.stats();
            assert_eq!(stats.hits, 2);
            assert_eq!(stats.misses, 1);
            assert!((stats.hit_rate() - 2.0 / 3.0).abs() < 1e-10);
        }

        #[test]
        fn test_tape_cache_stats_creation_eviction() {
            let mut cache = AadTapeCache::with_capacity(2);

            cache.store_tape(111, 5, 1000);
            cache.store_tape(222, 5, 2000);
            cache.store_tape(333, 5, 3000); // Triggers eviction

            let stats = cache.stats();
            assert_eq!(stats.tapes_created, 3);
            assert_eq!(stats.tapes_evicted, 1);
        }

        #[test]
        fn test_tape_cache_stats_total_reuses() {
            let mut cache = AadTapeCache::new();

            cache.store_tape(111, 5, 1000);

            // Multiple reuses via get_tape_mut
            cache.get_tape_mut(111);
            cache.get_tape_mut(111);
            cache.get_tape_mut(111);

            let stats = cache.stats();
            assert_eq!(stats.total_reuses, 3);
        }

        #[test]
        fn test_tape_cache_stats_reset() {
            let mut cache = AadTapeCache::new();

            cache.store_tape(111, 5, 1000);
            cache.get_tape(111);
            cache.get_tape(999);

            cache.reset_stats();

            let stats = cache.stats();
            assert_eq!(stats.hits, 0);
            assert_eq!(stats.misses, 0);
            assert_eq!(stats.tapes_created, 0);
        }

        #[test]
        fn test_cached_tape_record_reuse() {
            let mut tape = CachedTape::new(1, 12345, 5, 1000);

            assert_eq!(tape.reuse_count, 0);

            tape.record_reuse();
            assert_eq!(tape.reuse_count, 1);

            tape.record_reuse();
            tape.record_reuse();
            assert_eq!(tape.reuse_count, 3);
        }

        #[test]
        fn test_tape_ids_are_unique() {
            let mut cache = AadTapeCache::new();

            let id1 = cache.store_tape(111, 5, 1000);
            let id2 = cache.store_tape(222, 5, 2000);
            let id3 = cache.store_tape(333, 5, 3000);

            assert_ne!(id1, id2);
            assert_ne!(id2, id3);
            assert_ne!(id1, id3);
        }

        #[test]
        fn test_update_existing_tape() {
            let mut cache = AadTapeCache::new();

            let structure_hash = 12345u64;
            let id1 = cache.store_tape(structure_hash, 5, 1000);
            let id2 = cache.store_tape(structure_hash, 10, 2000); // Update same hash

            assert_eq!(cache.tape_count(), 1);
            assert_ne!(id1, id2); // New tape ID assigned

            let tape = cache.get_tape(structure_hash).unwrap();
            assert_eq!(tape.tenor_count, 10); // Updated
            assert_eq!(tape.created_at_ns, 2000); // Updated
        }
    }

    // =========================================================================
    // Task 3.1: Dependency Graph Tests
    // =========================================================================

    mod dependency_graph_tests {
        use super::*;

        #[test]
        fn test_empty_graph() {
            let graph = DependencyGraph::new();
            assert_eq!(graph.swap_count(), 0);
            assert_eq!(graph.tenor_count(), 0);
        }

        #[test]
        fn test_add_single_dependency() {
            let mut graph = DependencyGraph::new();
            let swap_id = SwapId::new("SWAP001");
            let tenor_point = TenorPoint::new(1, 1.0);

            graph.add_dependency(swap_id.clone(), tenor_point);

            assert_eq!(graph.swap_count(), 1);
            assert_eq!(graph.tenor_count(), 1);
            assert!(graph.has_dependencies(&swap_id));
        }

        #[test]
        fn test_add_multiple_dependencies_same_swap() {
            let mut graph = DependencyGraph::new();
            let swap_id = SwapId::new("SWAP001");
            let tenors = vec![
                TenorPoint::new(1, 0.5),
                TenorPoint::new(1, 1.0),
                TenorPoint::new(1, 2.0),
            ];

            graph.add_dependencies(swap_id.clone(), tenors);

            assert_eq!(graph.swap_count(), 1);
            assert_eq!(graph.tenor_count(), 3);

            let deps: Vec<_> = graph.get_swap_dependencies(&swap_id).collect();
            assert_eq!(deps.len(), 3);
        }

        #[test]
        fn test_add_multiple_swaps_same_tenor() {
            let mut graph = DependencyGraph::new();
            let tenor_point = TenorPoint::new(1, 1.0);

            graph.add_dependency(SwapId::new("SWAP001"), tenor_point);
            graph.add_dependency(SwapId::new("SWAP002"), tenor_point);
            graph.add_dependency(SwapId::new("SWAP003"), tenor_point);

            assert_eq!(graph.swap_count(), 3);
            assert_eq!(graph.tenor_count(), 1);

            let affected: Vec<_> = graph.get_affected_swaps(&tenor_point).collect();
            assert_eq!(affected.len(), 3);
        }

        #[test]
        fn test_get_affected_swaps_empty() {
            let graph = DependencyGraph::new();
            let tenor_point = TenorPoint::new(1, 1.0);

            let affected: Vec<_> = graph.get_affected_swaps(&tenor_point).collect();
            assert!(affected.is_empty());
        }

        #[test]
        fn test_get_affected_swaps_specific_tenor() {
            let mut graph = DependencyGraph::new();

            // SWAP001 depends on tenor 1.0
            graph.add_dependency(SwapId::new("SWAP001"), TenorPoint::new(1, 1.0));

            // SWAP002 depends on tenor 2.0
            graph.add_dependency(SwapId::new("SWAP002"), TenorPoint::new(1, 2.0));

            // SWAP003 depends on both
            graph.add_dependency(SwapId::new("SWAP003"), TenorPoint::new(1, 1.0));
            graph.add_dependency(SwapId::new("SWAP003"), TenorPoint::new(1, 2.0));

            // Query tenor 1.0 -> should return SWAP001 and SWAP003
            let affected_1y: Vec<_> = graph.get_affected_swaps(&TenorPoint::new(1, 1.0)).collect();
            assert_eq!(affected_1y.len(), 2);

            // Query tenor 2.0 -> should return SWAP002 and SWAP003
            let affected_2y: Vec<_> = graph.get_affected_swaps(&TenorPoint::new(1, 2.0)).collect();
            assert_eq!(affected_2y.len(), 2);
        }

        #[test]
        fn test_remove_swap() {
            let mut graph = DependencyGraph::new();
            let swap_id = SwapId::new("SWAP001");
            let tenor_point = TenorPoint::new(1, 1.0);

            graph.add_dependency(swap_id.clone(), tenor_point);
            assert!(graph.has_dependencies(&swap_id));

            graph.remove_swap(&swap_id);

            assert!(!graph.has_dependencies(&swap_id));
            assert_eq!(graph.swap_count(), 0);
            assert_eq!(graph.tenor_count(), 0);
        }

        #[test]
        fn test_remove_swap_cleans_tenor_mapping() {
            let mut graph = DependencyGraph::new();
            let tenor_point = TenorPoint::new(1, 1.0);

            // Two swaps depend on same tenor
            graph.add_dependency(SwapId::new("SWAP001"), tenor_point);
            graph.add_dependency(SwapId::new("SWAP002"), tenor_point);

            // Remove one swap
            graph.remove_swap(&SwapId::new("SWAP001"));

            // Tenor should still exist with one dependent
            let affected: Vec<_> = graph.get_affected_swaps(&tenor_point).collect();
            assert_eq!(affected.len(), 1);
            assert_eq!(affected[0].as_str(), "SWAP002");
        }

        #[test]
        fn test_clear_graph() {
            let mut graph = DependencyGraph::new();
            graph.add_dependency(SwapId::new("SWAP001"), TenorPoint::new(1, 1.0));
            graph.add_dependency(SwapId::new("SWAP002"), TenorPoint::new(1, 2.0));

            graph.clear();

            assert_eq!(graph.swap_count(), 0);
            assert_eq!(graph.tenor_count(), 0);
        }

        #[test]
        fn test_tenor_point_equality() {
            let tp1 = TenorPoint::new(1, 1.0);
            let tp2 = TenorPoint::new(1, 1.0);
            let tp3 = TenorPoint::new(1, 2.0);
            let tp4 = TenorPoint::new(2, 1.0);

            assert_eq!(tp1, tp2);
            assert_ne!(tp1, tp3);
            assert_ne!(tp1, tp4);
        }

        #[test]
        fn test_swap_id_display() {
            let swap_id = SwapId::new("SWAP001");
            assert_eq!(format!("{}", swap_id), "SWAP001");
            assert_eq!(swap_id.as_str(), "SWAP001");
        }
    }

    // =========================================================================
    // Task 3.2: Cache Management Tests
    // =========================================================================

    mod cache_tests {
        use super::*;

        #[test]
        fn test_cache_key_equality() {
            let key1 = CacheKey::new(123, 1, 19000);
            let key2 = CacheKey::new(123, 1, 19000);
            let key3 = CacheKey::new(456, 1, 19000);

            assert_eq!(key1, key2);
            assert_ne!(key1, key3);
        }

        #[test]
        fn test_cached_result_creation() {
            let result = CachedResult::new(1000.0_f64, 12345);

            assert!((result.npv - 1000.0).abs() < 1e-10);
            assert!(result.dv01.is_none());
            assert!(result.deltas.is_none());
            assert_eq!(result.cached_at_ns, 12345);
        }

        #[test]
        fn test_cached_result_with_dv01() {
            let result = CachedResult::new(1000.0_f64, 12345).with_dv01(50.0);

            assert!((result.npv - 1000.0).abs() < 1e-10);
            assert!((result.dv01.unwrap() - 50.0).abs() < 1e-10);
        }

        #[test]
        fn test_cached_result_with_deltas() {
            let deltas = vec![10.0, 20.0, 30.0];
            let result = CachedResult::new(1000.0_f64, 12345).with_deltas(deltas.clone());

            assert!(result.deltas.is_some());
            assert_eq!(result.deltas.unwrap().len(), 3);
        }

        #[test]
        fn test_cache_stats_default() {
            let stats = CacheStats::new();

            assert_eq!(stats.hits, 0);
            assert_eq!(stats.misses, 0);
            assert_eq!(stats.invalidations, 0);
            assert_eq!(stats.tape_reuses, 0);
            assert!((stats.hit_rate() - 0.0).abs() < 1e-10);
        }

        #[test]
        fn test_cache_stats_recording() {
            let mut stats = CacheStats::new();

            stats.record_hit();
            stats.record_hit();
            stats.record_miss();
            stats.record_invalidation();
            stats.record_tape_reuse();

            assert_eq!(stats.hits, 2);
            assert_eq!(stats.misses, 1);
            assert_eq!(stats.invalidations, 1);
            assert_eq!(stats.tape_reuses, 1);
        }

        #[test]
        fn test_cache_stats_hit_rate() {
            let mut stats = CacheStats::new();

            stats.record_hit();
            stats.record_hit();
            stats.record_hit();
            stats.record_miss();

            // 3 hits / 4 total = 0.75
            assert!((stats.hit_rate() - 0.75).abs() < 1e-10);
        }

        #[test]
        fn test_cache_stats_reset() {
            let mut stats = CacheStats::new();

            stats.record_hit();
            stats.record_miss();
            stats.reset();

            assert_eq!(stats.hits, 0);
            assert_eq!(stats.misses, 0);
        }
    }

    // =========================================================================
    // Task 3.3: Lazy Evaluator Tests
    // =========================================================================

    mod lazy_evaluator_tests {
        use super::*;

        #[test]
        fn test_evaluator_creation() {
            let evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            assert_eq!(evaluator.cache_size(), 0);
            assert_eq!(evaluator.curve_version(), 0);
        }

        #[test]
        fn test_store_and_retrieve() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);
            let swap_id = SwapId::new("SWAP001");
            let result = CachedResult::new(1000.0, 12345);

            evaluator.store(key.clone(), result, swap_id);

            let cached = evaluator.get_cached(&key);
            assert!(cached.is_some());
            assert!((cached.unwrap().npv - 1000.0).abs() < 1e-10);
        }

        #[test]
        fn test_cache_miss_on_nonexistent() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);

            let cached = evaluator.get_cached(&key);
            assert!(cached.is_none());
            assert_eq!(evaluator.cache_stats().misses, 1);
        }

        #[test]
        fn test_needs_recompute_missing() {
            let evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);

            assert!(evaluator.needs_recompute(&key));
        }

        #[test]
        fn test_needs_recompute_clean() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);
            let swap_id = SwapId::new("SWAP001");
            let result = CachedResult::new(1000.0, 12345);

            evaluator.store(key.clone(), result, swap_id);

            assert!(!evaluator.needs_recompute(&key));
        }

        #[test]
        fn test_curve_update_invalidates_cache() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);
            let swap_id = SwapId::new("SWAP001");
            let result = CachedResult::new(1000.0, 12345);

            // Store result and register dependency
            evaluator.store(key.clone(), result, swap_id.clone());
            evaluator.register_dependencies(swap_id, vec![TenorPoint::new(1, 1.0)]);

            // Verify clean before update
            assert!(!evaluator.needs_recompute(&key));

            // Notify curve update
            evaluator.notify_curve_update(1, 1.0);

            // Should now need recompute
            assert!(evaluator.needs_recompute(&key));
            assert_eq!(evaluator.cache_stats().invalidations, 1);
        }

        #[test]
        fn test_curve_update_only_affects_dependents() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            // Swap 1 depends on tenor 1.0
            let key1 = CacheKey::new(123, 1, 19000);
            let swap_id1 = SwapId::new("SWAP001");
            evaluator.store(
                key1.clone(),
                CachedResult::new(1000.0, 12345),
                swap_id1.clone(),
            );
            evaluator.register_dependencies(swap_id1, vec![TenorPoint::new(1, 1.0)]);

            // Swap 2 depends on tenor 2.0
            let key2 = CacheKey::new(456, 1, 19000);
            let swap_id2 = SwapId::new("SWAP002");
            evaluator.store(
                key2.clone(),
                CachedResult::new(2000.0, 12345),
                swap_id2.clone(),
            );
            evaluator.register_dependencies(swap_id2, vec![TenorPoint::new(1, 2.0)]);

            // Update tenor 1.0 -> should only invalidate SWAP001
            evaluator.notify_curve_update(1, 1.0);

            assert!(evaluator.needs_recompute(&key1));
            assert!(!evaluator.needs_recompute(&key2));
            assert_eq!(evaluator.cache_stats().invalidations, 1);
        }

        #[test]
        fn test_invalidate_all() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            // Store multiple entries
            evaluator.store(
                CacheKey::new(123, 1, 19000),
                CachedResult::new(1000.0, 12345),
                SwapId::new("SWAP001"),
            );
            evaluator.store(
                CacheKey::new(456, 1, 19000),
                CachedResult::new(2000.0, 12345),
                SwapId::new("SWAP002"),
            );

            evaluator.invalidate_all();

            assert!(evaluator.needs_recompute(&CacheKey::new(123, 1, 19000)));
            assert!(evaluator.needs_recompute(&CacheKey::new(456, 1, 19000)));
        }

        // =====================================================================
        // Task 3.4: AAD Tape Reuse Integration Tests
        // =====================================================================

        #[test]
        fn test_can_reuse_tape_after_registration() {
            // Requirement 3.5: AAD tape reuse capability
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let structure_hash = 12345u64;

            // Initially no tape registered
            assert!(!evaluator.can_reuse_tape(&swap_id));

            // Register tape
            evaluator.register_tape(swap_id.clone(), structure_hash, 5);

            // Now tape can be reused
            assert!(evaluator.can_reuse_tape(&swap_id));
        }

        #[test]
        fn test_tape_reuse_after_curve_update() {
            // Requirement 3.5: Tape can be reused even after curve update
            // (only parameters change, not structure)
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let key = CacheKey::new(123, 1, 19000);
            let structure_hash = 12345u64;

            // Store result and register tape
            evaluator.store(
                key.clone(),
                CachedResult::new(1000.0, 12345),
                swap_id.clone(),
            );
            evaluator.register_dependencies(swap_id.clone(), vec![TenorPoint::new(1, 1.0)]);
            evaluator.register_tape(swap_id.clone(), structure_hash, 5);

            // Notify curve update (invalidates cache but NOT tape)
            evaluator.notify_curve_update(1, 1.0);

            // Cache should need recompute
            assert!(evaluator.needs_recompute(&key));

            // But tape should still be reusable!
            assert!(evaluator.can_reuse_tape(&swap_id));
        }

        #[test]
        fn test_tape_reuse_nonexistent() {
            let evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");

            assert!(!evaluator.can_reuse_tape(&swap_id));
        }

        #[test]
        fn test_record_tape_reuse() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            evaluator.record_tape_reuse();
            evaluator.record_tape_reuse();

            assert_eq!(evaluator.cache_stats().tape_reuses, 2);
        }

        #[test]
        fn test_register_and_try_reuse_tape() {
            // Requirement 3.5: AAD tape reuse capability
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let structure_hash = 12345u64;

            // Register tape
            let tape_id = evaluator.register_tape(swap_id.clone(), structure_hash, 5);
            assert!(tape_id > 0);

            // Try to reuse - should record stats
            let tape = evaluator.try_reuse_tape(&swap_id);
            assert!(tape.is_some());
            assert_eq!(tape.unwrap().tenor_count, 5);

            // Check stats updated
            assert_eq!(evaluator.cache_stats().tape_reuses, 1);
        }

        #[test]
        fn test_tape_invalidation_by_swap() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let structure_hash = 12345u64;

            // Register tape
            evaluator.register_tape(swap_id.clone(), structure_hash, 5);
            assert!(evaluator.can_reuse_tape(&swap_id));

            // Invalidate tape
            let invalidated = evaluator.invalidate_tape(&swap_id);
            assert!(invalidated);
            assert!(!evaluator.can_reuse_tape(&swap_id));
        }

        #[test]
        fn test_invalidate_all_tapes() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            // Register multiple tapes
            evaluator.register_tape(SwapId::new("SWAP001"), 111, 5);
            evaluator.register_tape(SwapId::new("SWAP002"), 222, 10);
            evaluator.register_tape(SwapId::new("SWAP003"), 333, 15);

            assert_eq!(evaluator.tape_cache().tape_count(), 3);

            // Invalidate all
            evaluator.invalidate_all_tapes();

            assert_eq!(evaluator.tape_cache().tape_count(), 0);
            assert!(!evaluator.can_reuse_tape(&SwapId::new("SWAP001")));
            assert!(!evaluator.can_reuse_tape(&SwapId::new("SWAP002")));
            assert!(!evaluator.can_reuse_tape(&SwapId::new("SWAP003")));
        }

        #[test]
        fn test_evaluator_with_tape_capacity() {
            let evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::with_tape_capacity(50);

            assert_eq!(evaluator.tape_cache().capacity(), 50);
        }

        #[test]
        fn test_tape_reuse_by_structure_hash() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let structure_hash = 12345u64;

            // Register tape for swap
            evaluator.register_tape(SwapId::new("SWAP001"), structure_hash, 5);

            // Can check by structure hash directly
            assert!(evaluator.can_reuse_tape_for_structure(structure_hash));

            // Can reuse by hash
            let tape = evaluator.try_reuse_tape_by_hash(structure_hash);
            assert!(tape.is_some());
        }

        #[test]
        fn test_remove_swap_also_removes_tape() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let key = CacheKey::new(123, 1, 19000);
            let structure_hash = 12345u64;

            // Store result and register tape
            evaluator.store(
                key.clone(),
                CachedResult::new(1000.0, 12345),
                swap_id.clone(),
            );
            evaluator.register_tape(swap_id.clone(), structure_hash, 5);

            assert!(evaluator.can_reuse_tape(&swap_id));
            assert_eq!(evaluator.tape_cache().tape_count(), 1);

            // Remove swap
            evaluator.remove_swap(&swap_id);

            // Both cache and tape should be removed
            assert_eq!(evaluator.cache_size(), 0);
            assert!(!evaluator.can_reuse_tape(&swap_id));
            assert_eq!(evaluator.tape_cache().tape_count(), 0);
        }

        #[test]
        fn test_clear_evaluator_clears_tapes() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            // Store result and register tape
            evaluator.store(
                CacheKey::new(123, 1, 19000),
                CachedResult::new(1000.0, 12345),
                SwapId::new("SWAP001"),
            );
            evaluator.register_tape(SwapId::new("SWAP001"), 12345, 5);

            evaluator.clear();

            assert_eq!(evaluator.tape_cache().tape_count(), 0);
            assert!(!evaluator.can_reuse_tape(&SwapId::new("SWAP001")));
        }

        #[test]
        fn test_multiple_swaps_same_structure_hash() {
            // Different swaps with same structure can share tape
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let structure_hash = 12345u64; // Same structure

            // Register two swaps with same structure
            evaluator.register_tape(SwapId::new("SWAP001"), structure_hash, 5);
            evaluator.register_tape(SwapId::new("SWAP002"), structure_hash, 5);

            // Only one tape in cache (same hash)
            assert_eq!(evaluator.tape_cache().tape_count(), 1);

            // Both swaps can reuse
            assert!(evaluator.can_reuse_tape(&SwapId::new("SWAP001")));
            assert!(evaluator.can_reuse_tape(&SwapId::new("SWAP002")));
        }

        #[test]
        fn test_tape_reuse_count_tracking() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let structure_hash = 12345u64;

            // Register tape
            evaluator.register_tape(swap_id.clone(), structure_hash, 5);

            // Reuse multiple times
            evaluator.try_reuse_tape(&swap_id);
            evaluator.try_reuse_tape(&swap_id);
            evaluator.try_reuse_tape(&swap_id);

            // Check tape reuse count
            let tape = evaluator.tape_cache().tapes.get(&structure_hash).unwrap();
            assert_eq!(tape.reuse_count, 3);

            // Check stats
            assert_eq!(evaluator.cache_stats().tape_reuses, 3);
        }

        #[test]
        fn test_curve_version_increments() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            assert_eq!(evaluator.curve_version(), 0);

            evaluator.notify_curve_update(1, 1.0);
            assert_eq!(evaluator.curve_version(), 1);

            evaluator.invalidate_all();
            assert_eq!(evaluator.curve_version(), 2);
        }

        #[test]
        fn test_clear_evaluator() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();

            evaluator.store(
                CacheKey::new(123, 1, 19000),
                CachedResult::new(1000.0, 12345),
                SwapId::new("SWAP001"),
            );
            evaluator.register_dependencies(SwapId::new("SWAP001"), vec![TenorPoint::new(1, 1.0)]);
            evaluator.cache_stats_mut().record_hit();

            evaluator.clear();

            assert_eq!(evaluator.cache_size(), 0);
            assert_eq!(evaluator.curve_version(), 0);
            assert_eq!(evaluator.cache_stats().hits, 0);
            assert_eq!(evaluator.dependency_graph().swap_count(), 0);
        }

        #[test]
        fn test_remove_swap() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let swap_id = SwapId::new("SWAP001");
            let key = CacheKey::new(123, 1, 19000);

            evaluator.store(
                key.clone(),
                CachedResult::new(1000.0, 12345),
                swap_id.clone(),
            );
            evaluator.register_dependencies(swap_id.clone(), vec![TenorPoint::new(1, 1.0)]);

            evaluator.remove_swap(&swap_id);

            assert_eq!(evaluator.cache_size(), 0);
            assert!(!evaluator.can_reuse_tape(&swap_id));
            assert!(!evaluator.dependency_graph().has_dependencies(&swap_id));
        }

        #[test]
        fn test_get_cached_records_hit() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);
            let swap_id = SwapId::new("SWAP001");

            evaluator.store(key.clone(), CachedResult::new(1000.0, 12345), swap_id);

            let _ = evaluator.get_cached(&key);
            let _ = evaluator.get_cached(&key);

            assert_eq!(evaluator.cache_stats().hits, 2);
        }

        #[test]
        fn test_get_cached_returns_none_for_dirty() {
            let mut evaluator: IrsLazyEvaluator<f64> = IrsLazyEvaluator::new();
            let key = CacheKey::new(123, 1, 19000);
            let swap_id = SwapId::new("SWAP001");

            evaluator.store(
                key.clone(),
                CachedResult::new(1000.0, 12345),
                swap_id.clone(),
            );
            evaluator.register_dependencies(swap_id, vec![TenorPoint::new(1, 1.0)]);
            evaluator.notify_curve_update(1, 1.0);

            let cached = evaluator.get_cached(&key);
            assert!(cached.is_none());
        }
    }
}

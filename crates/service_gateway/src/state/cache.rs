//! Generic LRU cache and domain-specific entry types
//!
//! `TypedCache<T>` provides a single, thread-safe, UUID-keyed LRU cache
//! implementation that replaces the previous per-domain cache structs.

use std::num::NonZeroUsize;

#[cfg(any(feature = "models", feature = "risk", feature = "volatility"))]
use chrono::{DateTime, Utc};
use lru::LruCache;
use parking_lot::RwLock;
use pricer_models::market::curves::BootstrappedCurve;
use uuid::Uuid;

// ============================================================================
// Generic cache
// ============================================================================

/// Thread-safe LRU cache keyed by [`Uuid`].
///
/// Entries must be `Clone` so reads can return owned values without
/// holding the lock beyond the call.
pub struct TypedCache<T: Clone> {
    inner: RwLock<LruCache<Uuid, T>>,
}

impl<T: Clone> TypedCache<T> {
    /// Create a cache with the given *capacity*.
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(cap)),
        }
    }

    /// Insert an entry and return its generated UUID.
    pub fn add(&self, entry: T) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().put(id, entry);
        id
    }

    /// Retrieve a clone of the entry (promotes to MRU).
    pub fn get(&self, id: &Uuid) -> Option<T> {
        self.inner.write().get(id).cloned()
    }

    /// Replace an existing entry. Returns `true` if the key was present.
    pub fn update(&self, id: &Uuid, entry: T) -> bool {
        let mut cache = self.inner.write();
        if cache.contains(id) {
            cache.put(*id, entry);
            true
        } else {
            false
        }
    }

    /// Remove and return an entry.
    pub fn remove(&self, id: &Uuid) -> Option<T> { self.inner.write().pop(id) }

    /// Check whether the key exists.
    pub fn contains(&self, id: &Uuid) -> bool { self.inner.read().contains(id) }

    /// Number of cached entries.
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Remove all entries.
    pub fn clear(&self) { self.inner.write().clear(); }

    /// List all cached UUIDs.
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.inner.read().iter().map(|(id, _)| *id).collect()
    }
}

impl<T: Clone> Default for TypedCache<T> {
    fn default() -> Self { Self::new(100) }
}

// ============================================================================
// Domain entry types
// ============================================================================

/// Curve cache entry
#[derive(Debug, Clone)]
pub struct CurveEntry {
    pub curve: BootstrappedCurve<f64>,
    pub instruments: Vec<InstrumentInput>,
}

/// Simplified instrument input for caching
#[derive(Debug, Clone)]
pub struct InstrumentInput {
    pub instrument_type: String,
    pub tenor: String,
    pub rate: f64,
}

/// FX volatility surface cache entry
#[derive(Debug, Clone)]
pub struct FxVolEntry {
    pub currency_pair: String,
    pub surface_type: String,
    pub calibrated_params: Vec<SabrParams>,
}

/// SABR parameters for a single expiry slice
#[derive(Debug, Clone)]
pub struct SabrParams {
    pub expiry: f64,
    pub alpha: f64,
    pub beta: f64,
    pub rho: f64,
    pub nu: f64,
}

/// Portfolio cache entry
#[cfg(feature = "risk")]
#[derive(Debug, Clone)]
pub struct PortfolioEntry {
    pub name: Option<String>,
    pub trade_count: usize,
    pub trade_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Model type enumeration
#[cfg(feature = "models")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    Gbm,
    Heston,
    HullWhite,
    Cir,
    Sabr,
}

/// Model cache entry
#[cfg(feature = "models")]
#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub model_type: ModelType,
    pub params_json: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Vol surface type enumeration
#[cfg(feature = "volatility")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolSurfaceType {
    FxSurface,
    IrCube,
    EquitySurface,
}

/// Vol surface cache entry
#[cfg(feature = "volatility")]
#[derive(Debug, Clone)]
pub struct VolSurfaceEntry {
    pub surface_type: VolSurfaceType,
    pub underlying: String,
    pub sabr_params: Vec<SabrParams>,
    pub expiry_count: usize,
    pub residual_ss: Option<f64>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_cache_basic_operations() {
        let cache: TypedCache<String> = TypedCache::new(10);
        assert!(cache.is_empty());

        let id = cache.add("hello".to_string());
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&id));

        let val = cache.get(&id).unwrap();
        assert_eq!(val, "hello");

        assert!(cache.update(&id, "world".to_string()));
        assert_eq!(cache.get(&id).unwrap(), "world");

        let removed = cache.remove(&id);
        assert_eq!(removed.unwrap(), "world");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_typed_cache_list_ids() {
        let cache: TypedCache<i32> = TypedCache::new(5);
        let id1 = cache.add(1);
        let id2 = cache.add(2);

        let ids = cache.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_fxvol_entry() {
        let cache: TypedCache<FxVolEntry> = TypedCache::new(5);
        let entry = FxVolEntry {
            currency_pair: "USDJPY".to_string(),
            surface_type: "SABR".to_string(),
            calibrated_params: vec![SabrParams {
                expiry: 0.25,
                alpha: 0.2,
                beta: 0.5,
                rho: -0.1,
                nu: 0.3,
            }],
        };

        let id = cache.add(entry);
        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.currency_pair, "USDJPY");
    }

    #[cfg(feature = "risk")]
    #[test]
    fn test_portfolio_entry() {
        let cache: TypedCache<PortfolioEntry> = TypedCache::new(10);
        let entry = PortfolioEntry {
            name: Some("Test Portfolio".to_string()),
            trade_count: 5,
            trade_ids: vec!["T1".to_string(), "T2".to_string()],
            created_at: Utc::now(),
        };

        let id = cache.add(entry);
        assert!(cache.contains(&id));
        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.name, Some("Test Portfolio".to_string()));
        assert_eq!(retrieved.trade_count, 5);

        let updated = PortfolioEntry {
            name: Some("Updated".to_string()),
            trade_count: 10,
            trade_ids: vec!["T1".to_string(), "T2".to_string(), "T3".to_string()],
            created_at: Utc::now(),
        };
        assert!(cache.update(&id, updated));
        assert_eq!(cache.get(&id).unwrap().trade_count, 10);

        assert!(cache.remove(&id).is_some());
        assert!(cache.is_empty());
    }

    #[cfg(feature = "models")]
    #[test]
    fn test_model_entry() {
        let cache: TypedCache<ModelEntry> = TypedCache::new(10);
        let entry = ModelEntry {
            model_type: ModelType::Heston,
            params_json: r#"{"kappa": 2.0}"#.to_string(),
            name: Some("Test".to_string()),
            created_at: Utc::now(),
        };

        let id = cache.add(entry);
        assert!(cache.contains(&id));
        assert_eq!(cache.get(&id).unwrap().model_type, ModelType::Heston);
        assert!(cache.remove(&id).is_some());
        assert!(cache.is_empty());
    }

    #[cfg(feature = "volatility")]
    #[test]
    fn test_vol_surface_entry() {
        let cache: TypedCache<VolSurfaceEntry> = TypedCache::new(10);
        let entry = VolSurfaceEntry {
            surface_type: VolSurfaceType::FxSurface,
            underlying: "USDJPY".to_string(),
            sabr_params: vec![SabrParams {
                expiry: 0.25,
                alpha: 0.2,
                beta: 0.5,
                rho: -0.1,
                nu: 0.3,
            }],
            expiry_count: 1,
            residual_ss: Some(1e-6),
            created_at: Utc::now(),
        };

        let id = cache.add(entry);
        assert!(cache.contains(&id));
        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.surface_type, VolSurfaceType::FxSurface);
        assert_eq!(retrieved.underlying, "USDJPY");
        assert!(cache.remove(&id).is_some());
        assert!(cache.is_empty());
    }
}

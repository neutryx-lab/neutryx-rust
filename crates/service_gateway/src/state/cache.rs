//! Cache implementations for service_gateway
//!
//! Provides LRU caches for bootstrapped curves and volatility surfaces.

use std::num::NonZeroUsize;

use lru::LruCache;
use parking_lot::RwLock;
use pricer_models::market::curves::BootstrappedCurve;
use uuid::Uuid;

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CurveEntry {
    /// The bootstrapped curve
    pub curve: BootstrappedCurve<f64>,
    /// Original instrument inputs (for risk calculations)
    pub instruments: Vec<InstrumentInput>,
}

/// Simplified instrument input for caching
#[derive(Debug, Clone)]
pub struct InstrumentInput {
    pub instrument_type: String,
    pub tenor: String,
    pub rate: f64,
}

/// LRU cache for bootstrapped curves
pub struct CurveCache {
    inner: RwLock<LruCache<Uuid, CurveEntry>>,
}

impl CurveCache {
    /// Create a new curve cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(capacity)),
        }
    }

    /// Add a curve to the cache and return its ID
    pub fn add(&self, curve: BootstrappedCurve<f64>, instruments: Vec<InstrumentInput>) -> Uuid {
        let id = Uuid::new_v4();
        let entry = CurveEntry { curve, instruments };
        self.inner.write().put(id, entry);
        id
    }

    /// Get a curve by ID
    pub fn get(&self, id: &Uuid) -> Option<CurveEntry> {
        self.inner.write().get(id).cloned()
    }

    /// Check if a curve exists
    pub fn contains(&self, id: &Uuid) -> bool {
        self.inner.read().contains(id)
    }

    /// Remove a curve from the cache
    pub fn remove(&self, id: &Uuid) -> Option<CurveEntry> {
        self.inner.write().pop(id)
    }

    /// Get the number of cached curves
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Clear all cached curves
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

impl Default for CurveCache {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Cache for FX volatility surfaces
pub struct FxVolCache {
    inner: RwLock<LruCache<Uuid, FxVolEntry>>,
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

impl FxVolCache {
    /// Create a new FX vol cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(20).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(capacity)),
        }
    }

    /// Add an FX vol surface to the cache
    pub fn add(&self, entry: FxVolEntry) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().put(id, entry);
        id
    }

    /// Get an FX vol surface by ID
    pub fn get(&self, id: &Uuid) -> Option<FxVolEntry> {
        self.inner.write().get(id).cloned()
    }

    /// Get the number of cached surfaces
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Clear all cached surfaces
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

impl Default for FxVolCache {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_cache_basic_operations() {
        let cache = CurveCache::new(10);
        assert!(cache.is_empty());

        // We can't easily create a BootstrappedCurve without the builder,
        // so we just test the cache structure
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_fxvol_cache_basic_operations() {
        let cache = FxVolCache::new(5);
        assert!(cache.is_empty());

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

        let id = cache.add(entry.clone());
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.currency_pair, "USDJPY");
    }
}

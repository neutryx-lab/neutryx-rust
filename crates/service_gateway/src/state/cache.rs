//! Cache implementations for `service_gateway`
//!
//! Provides LRU caches for bootstrapped curves, volatility surfaces,
//! portfolios, models, and vol surfaces/cubes.

use std::num::NonZeroUsize;

#[cfg(any(feature = "models", feature = "risk", feature = "volatility"))]
use chrono::{DateTime, Utc};
use lru::LruCache;
use parking_lot::RwLock;
use pricer_models::market::curves::BootstrappedCurve;
use uuid::Uuid;

// ============================================================================
// Curve Cache
// ============================================================================

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
    pub fn get(&self, id: &Uuid) -> Option<CurveEntry> { self.inner.write().get(id).cloned() }

    /// Check if a curve exists
    pub fn contains(&self, id: &Uuid) -> bool { self.inner.read().contains(id) }

    /// Remove a curve from the cache
    pub fn remove(&self, id: &Uuid) -> Option<CurveEntry> { self.inner.write().pop(id) }

    /// Get the number of cached curves
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Clear all cached curves
    pub fn clear(&self) { self.inner.write().clear(); }

    /// List all curve IDs in the cache
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.inner.read().iter().map(|(id, _)| *id).collect()
    }
}

impl Default for CurveCache {
    fn default() -> Self { Self::new(100) }
}

// ============================================================================
// FX Volatility Cache
// ============================================================================

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
    pub fn get(&self, id: &Uuid) -> Option<FxVolEntry> { self.inner.write().get(id).cloned() }

    /// Get the number of cached surfaces
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Clear all cached surfaces
    pub fn clear(&self) { self.inner.write().clear(); }
}

impl Default for FxVolCache {
    fn default() -> Self { Self::new(20) }
}

// ============================================================================
// Portfolio Cache (feature = "risk")
// ============================================================================

/// Portfolio cache entry with metadata
#[cfg(feature = "risk")]
#[derive(Debug, Clone)]
pub struct PortfolioEntry {
    /// Portfolio name
    pub name: Option<String>,
    /// Number of trades in the portfolio
    pub trade_count: usize,
    /// Trade IDs in this portfolio
    pub trade_ids: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// LRU cache for portfolios
#[cfg(feature = "risk")]
pub struct PortfolioCache {
    inner: RwLock<LruCache<Uuid, PortfolioEntry>>,
}

#[cfg(feature = "risk")]
impl PortfolioCache {
    /// Create a new portfolio cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(50).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(capacity)),
        }
    }

    /// Add a portfolio to the cache and return its ID
    pub fn add(&self, entry: PortfolioEntry) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().put(id, entry);
        id
    }

    /// Get a portfolio by ID
    pub fn get(&self, id: &Uuid) -> Option<PortfolioEntry> { self.inner.write().get(id).cloned() }

    /// Update a portfolio in the cache
    pub fn update(&self, id: &Uuid, entry: PortfolioEntry) -> bool {
        let mut cache = self.inner.write();
        if cache.contains(id) {
            cache.put(*id, entry);
            true
        } else {
            false
        }
    }

    /// Remove a portfolio from the cache
    pub fn remove(&self, id: &Uuid) -> Option<PortfolioEntry> { self.inner.write().pop(id) }

    /// Check if a portfolio exists
    pub fn contains(&self, id: &Uuid) -> bool { self.inner.read().contains(id) }

    /// Get the number of cached portfolios
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Clear all cached portfolios
    pub fn clear(&self) { self.inner.write().clear(); }
}

#[cfg(feature = "risk")]
impl Default for PortfolioCache {
    fn default() -> Self { Self::new(50) }
}

// ============================================================================
// Model Cache (feature = "models")
// ============================================================================

/// Model type enumeration
#[cfg(feature = "models")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Geometric Brownian Motion
    Gbm,
    /// Heston stochastic volatility
    Heston,
    /// Hull-White interest rate model
    HullWhite,
    /// Cox-Ingersoll-Ross model
    Cir,
    /// SABR model
    Sabr,
}

/// Model cache entry with metadata
#[cfg(feature = "models")]
#[derive(Debug, Clone)]
pub struct ModelEntry {
    /// Type of stochastic model
    pub model_type: ModelType,
    /// Model parameters (serialised)
    pub params_json: String,
    /// Model name/description
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// LRU cache for stochastic models
#[cfg(feature = "models")]
pub struct ModelCache {
    inner: RwLock<LruCache<Uuid, ModelEntry>>,
}

#[cfg(feature = "models")]
impl ModelCache {
    /// Create a new model cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(20).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(capacity)),
        }
    }

    /// Add a model to the cache and return its ID
    pub fn add(&self, entry: ModelEntry) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().put(id, entry);
        id
    }

    /// Get a model by ID
    pub fn get(&self, id: &Uuid) -> Option<ModelEntry> { self.inner.write().get(id).cloned() }

    /// Remove a model from the cache
    pub fn remove(&self, id: &Uuid) -> Option<ModelEntry> { self.inner.write().pop(id) }

    /// Check if a model exists
    pub fn contains(&self, id: &Uuid) -> bool { self.inner.read().contains(id) }

    /// Get the number of cached models
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Clear all cached models
    pub fn clear(&self) { self.inner.write().clear(); }
}

#[cfg(feature = "models")]
impl Default for ModelCache {
    fn default() -> Self { Self::new(20) }
}

// ============================================================================
// Vol Surface Cache (feature = "volatility")
// ============================================================================

/// Vol surface type enumeration
#[cfg(feature = "volatility")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolSurfaceType {
    /// FX volatility surface (2D: expiry x strike)
    FxSurface,
    /// IR volatility cube (3D: expiry x tenor x strike)
    IrCube,
    /// Equity volatility surface
    EquitySurface,
}

/// Vol surface cache entry with metadata
#[cfg(feature = "volatility")]
#[derive(Debug, Clone)]
pub struct VolSurfaceEntry {
    /// Type of volatility surface
    pub surface_type: VolSurfaceType,
    /// Underlying identifier (e.g., currency pair, index)
    pub underlying: String,
    /// Calibrated SABR parameters per expiry slice
    pub sabr_params: Vec<SabrParams>,
    /// Number of expiry slices
    pub expiry_count: usize,
    /// Calibration residual (sum of squared errors)
    pub residual_ss: Option<f64>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// LRU cache for volatility surfaces/cubes
#[cfg(feature = "volatility")]
pub struct VolSurfaceCache {
    inner: RwLock<LruCache<Uuid, VolSurfaceEntry>>,
}

#[cfg(feature = "volatility")]
impl VolSurfaceCache {
    /// Create a new vol surface cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(20).unwrap());
        Self {
            inner: RwLock::new(LruCache::new(capacity)),
        }
    }

    /// Add a vol surface to the cache and return its ID
    pub fn add(&self, entry: VolSurfaceEntry) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.write().put(id, entry);
        id
    }

    /// Get a vol surface by ID
    pub fn get(&self, id: &Uuid) -> Option<VolSurfaceEntry> { self.inner.write().get(id).cloned() }

    /// Remove a vol surface from the cache
    pub fn remove(&self, id: &Uuid) -> Option<VolSurfaceEntry> { self.inner.write().pop(id) }

    /// Check if a vol surface exists
    pub fn contains(&self, id: &Uuid) -> bool { self.inner.read().contains(id) }

    /// Get the number of cached surfaces
    pub fn len(&self) -> usize { self.inner.read().len() }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool { self.inner.read().is_empty() }

    /// Clear all cached surfaces
    pub fn clear(&self) { self.inner.write().clear(); }
}

#[cfg(feature = "volatility")]
impl Default for VolSurfaceCache {
    fn default() -> Self { Self::new(20) }
}

// ============================================================================
// Tests
// ============================================================================

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

    #[cfg(feature = "risk")]
    #[test]
    fn test_portfolio_cache_operations() {
        let cache = PortfolioCache::new(10);
        assert!(cache.is_empty());

        let entry = PortfolioEntry {
            name: Some("Test Portfolio".to_string()),
            trade_count: 5,
            trade_ids: vec!["T1".to_string(), "T2".to_string()],
            created_at: Utc::now(),
        };

        let id = cache.add(entry);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&id));

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.name, Some("Test Portfolio".to_string()));
        assert_eq!(retrieved.trade_count, 5);

        // Test update
        let updated_entry = PortfolioEntry {
            name: Some("Updated Portfolio".to_string()),
            trade_count: 10,
            trade_ids: vec!["T1".to_string(), "T2".to_string(), "T3".to_string()],
            created_at: Utc::now(),
        };
        assert!(cache.update(&id, updated_entry));

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.name, Some("Updated Portfolio".to_string()));
        assert_eq!(retrieved.trade_count, 10);

        // Test remove
        let removed = cache.remove(&id);
        assert!(removed.is_some());
        assert!(cache.is_empty());
    }

    #[cfg(feature = "models")]
    #[test]
    fn test_model_cache_operations() {
        let cache = ModelCache::new(10);
        assert!(cache.is_empty());

        let entry = ModelEntry {
            model_type: ModelType::Heston,
            params_json: r#"{"kappa": 2.0, "theta": 0.04}"#.to_string(),
            name: Some("Test Heston".to_string()),
            created_at: Utc::now(),
        };

        let id = cache.add(entry);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&id));

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.model_type, ModelType::Heston);

        // Test remove
        let removed = cache.remove(&id);
        assert!(removed.is_some());
        assert!(cache.is_empty());
    }

    #[cfg(feature = "volatility")]
    #[test]
    fn test_vol_surface_cache_operations() {
        let cache = VolSurfaceCache::new(10);
        assert!(cache.is_empty());

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
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&id));

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.surface_type, VolSurfaceType::FxSurface);
        assert_eq!(retrieved.underlying, "USDJPY");

        // Test remove
        let removed = cache.remove(&id);
        assert!(removed.is_some());
        assert!(cache.is_empty());
    }
}

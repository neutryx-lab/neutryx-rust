//! Market data provider with lazy evaluation and Arc caching.
//!
//! This module provides `MarketProvider` - a thread-safe cache for market data
//! objects that implements lazy evaluation (on-demand construction) and
//! Arc-based sharing for zero-copy access across threads.
//!
//! # Architecture Role
//!
//! `MarketProvider` is the "Pull" mechanism in the Pull-then-Push execution
//! pattern:
//! - **Pull Phase**: Dependencies are resolved lazily via `get_curve()` /
//!   `get_vol()`
//! - **Push Phase**: Resolved references are passed to pricing kernels
//!
//! # Caching Strategy
//!
//! - Uses double-check locking pattern to prevent duplicate construction
//! - First access triggers construction with log output
//! - Subsequent accesses return cached `Arc` without logging
//!
//! # Example
//!
//! ```rust
//! use infra_master::Currency;
//! use pricer_models::market::MarketProvider;
//!
//! let provider = MarketProvider::new();
//!
//! // First call bootstraps and caches the curve
//! let curve1 = provider.get_curve(Currency::USD);
//!
//! // Second call returns cached Arc (no bootstrap)
//! let curve2 = provider.get_curve(Currency::USD);
//!
//! // Both point to the same object
//! assert!(std::sync::Arc::ptr_eq(&curve1, &curve2));
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use infra_master::Currency;

use crate::market::{
    curves::{CurveEnum, FlatCurve},
    surfaces::VolSurfaceEnum,
};

/// Thread-safe market data provider with lazy evaluation and Arc caching.
///
/// Maintains separate caches for yield curves and volatility surfaces,
/// constructing objects on first access and sharing via `Arc` thereafter.
pub struct MarketProvider {
    /// Cache for yield curves, keyed by currency.
    curve_cache: RwLock<HashMap<Currency, Arc<CurveEnum<f64>>>>,
    /// Cache for volatility surfaces, keyed by currency.
    vol_cache: RwLock<HashMap<Currency, Arc<VolSurfaceEnum<f64>>>>,
}

impl MarketProvider {
    /// Creates a new `MarketProvider` with empty caches.
    ///
    /// # Returns
    ///
    /// A new `MarketProvider` instance ready for lazy population.
    pub fn new() -> Self {
        Self {
            curve_cache: RwLock::new(HashMap::new()),
            vol_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieves or constructs the yield curve for the given currency.
    ///
    /// Implements double-check locking pattern:
    /// 1. Acquire read lock, check cache
    /// 2. If miss, acquire write lock, check again (another thread may have
    ///    populated)
    /// 3. If still miss, bootstrap curve, log, and cache
    ///
    /// # Arguments
    ///
    /// * `ccy` - The currency for which to retrieve the curve.
    ///
    /// # Returns
    ///
    /// `Arc<CurveEnum<f64>>` - shared reference to the yield curve.
    ///
    /// # Logging
    ///
    /// On cache miss, prints: `[MarketData] Bootstrapping Yield Curve for
    /// {currency}...`
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)] // RwLock unwrap is safe: poisoned lock indicates unrecoverable prior panic
    pub fn get_curve(&self, ccy: Currency) -> Arc<CurveEnum<f64>> {
        // Fast path: read lock check
        {
            let cache = self.curve_cache.read().unwrap();
            if let Some(curve) = cache.get(&ccy) {
                return Arc::clone(curve);
            }
        }

        // Slow path: write lock with double-check
        let mut cache = self.curve_cache.write().unwrap();

        // Double-check: another thread may have populated while we waited
        if let Some(curve) = cache.get(&ccy) {
            return Arc::clone(curve);
        }

        // Bootstrap the curve
        println!("[MarketData] Bootstrapping Yield Curve for {}...", ccy);

        // Create a flat curve with currency-specific rate (demo purposes)
        let rate = match ccy {
            Currency::USD => 0.05,
            Currency::EUR => 0.03,
            Currency::GBP => 0.04,
            Currency::JPY => 0.01,
            Currency::CHF => 0.02,
            _ => 0.03, // Default rate for unknown currencies
        };

        let curve = Arc::new(CurveEnum::Flat(FlatCurve::new(rate)));
        cache.insert(ccy, Arc::clone(&curve));
        curve
    }

    /// Retrieves or constructs the volatility surface for the given currency.
    ///
    /// Implements double-check locking pattern similar to `get_curve()`.
    ///
    /// # Arguments
    ///
    /// * `ccy` - The currency for which to retrieve the volatility surface.
    ///
    /// # Returns
    ///
    /// `Arc<VolSurfaceEnum<f64>>` - shared reference to the volatility surface.
    ///
    /// # Logging
    ///
    /// On cache miss, prints: `[MarketData] Calibrating Vol Surface for
    /// {currency}...`
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)] // RwLock unwrap is safe: poisoned lock indicates unrecoverable prior panic
    pub fn get_vol(&self, ccy: Currency) -> Arc<VolSurfaceEnum<f64>> {
        // Fast path: read lock check
        {
            let cache = self.vol_cache.read().unwrap();
            if let Some(vol) = cache.get(&ccy) {
                return Arc::clone(vol);
            }
        }

        // Slow path: write lock with double-check
        let mut cache = self.vol_cache.write().unwrap();

        // Double-check: another thread may have populated while we waited
        if let Some(vol) = cache.get(&ccy) {
            return Arc::clone(vol);
        }

        // Calibrate the volatility surface
        println!("[MarketData] Calibrating Vol Surface for {}...", ccy);

        // Create a flat vol surface with currency-specific vol (demo purposes)
        let sigma = match ccy {
            Currency::USD => 0.20,
            Currency::EUR => 0.18,
            Currency::GBP => 0.19,
            Currency::JPY => 0.15,
            Currency::CHF => 0.16,
            _ => 0.20, // Default vol for unknown currencies
        };

        let vol = Arc::new(VolSurfaceEnum::flat(sigma));
        cache.insert(ccy, Arc::clone(&vol));
        vol
    }
}

impl Default for MarketProvider {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::market::{curves::YieldCurve, surfaces::VolatilitySurface};

    // -------------------------------------------------------------------------
    // MarketProvider Structure Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_market_provider_new() {
        let provider = MarketProvider::new();
        // Verify caches are empty by checking no curves exist yet
        let cache = provider.curve_cache.read().unwrap();
        assert!(cache.is_empty());
        let vol_cache = provider.vol_cache.read().unwrap();
        assert!(vol_cache.is_empty());
    }

    #[test]
    fn test_market_provider_default() {
        let provider = MarketProvider::default();
        let cache = provider.curve_cache.read().unwrap();
        assert!(cache.is_empty());
    }

    // -------------------------------------------------------------------------
    // get_curve() Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_curve_returns_arc() {
        let provider = MarketProvider::new();
        let curve = provider.get_curve(Currency::USD);
        // Verify we got an Arc<CurveEnum>
        match curve.as_ref() {
            CurveEnum::Flat(flat) => {
                // USD rate is 0.05, check DF at 1Y
                let df = flat.discount_factor(1.0).unwrap();
                let expected = (-0.05_f64).exp();
                assert!((df - expected).abs() < 1e-10);
            }
            _ => panic!("Expected Flat curve"),
        }
    }

    #[test]
    fn test_get_curve_caches_result() {
        let provider = MarketProvider::new();

        let curve1 = provider.get_curve(Currency::USD);
        let curve2 = provider.get_curve(Currency::USD);

        // Both should point to the same Arc
        assert!(Arc::ptr_eq(&curve1, &curve2), "Should return cached Arc");
    }

    #[test]
    fn test_get_curve_different_currencies() {
        let provider = MarketProvider::new();

        let usd = provider.get_curve(Currency::USD);
        let jpy = provider.get_curve(Currency::JPY);

        // Should be different objects
        assert!(
            !Arc::ptr_eq(&usd, &jpy),
            "Different currencies should have different curves"
        );
    }

    #[test]
    fn test_get_curve_discount_factor() {
        let provider = MarketProvider::new();
        let curve = provider.get_curve(Currency::USD);

        // USD rate is 0.05, so DF at 1Y should be exp(-0.05)
        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // get_vol() Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_vol_returns_arc() {
        let provider = MarketProvider::new();
        let vol = provider.get_vol(Currency::USD);
        // Verify we got an Arc<VolSurfaceEnum>
        let sigma = vol.volatility(100.0, 1.0).unwrap();
        assert!((sigma - 0.20).abs() < 1e-10, "USD vol should be 0.20");
    }

    #[test]
    fn test_get_vol_caches_result() {
        let provider = MarketProvider::new();

        let vol1 = provider.get_vol(Currency::USD);
        let vol2 = provider.get_vol(Currency::USD);

        // Both should point to the same Arc
        assert!(Arc::ptr_eq(&vol1, &vol2), "Should return cached Arc");
    }

    #[test]
    fn test_get_vol_different_currencies() {
        let provider = MarketProvider::new();

        let usd = provider.get_vol(Currency::USD);
        let eur = provider.get_vol(Currency::EUR);

        // Should be different objects
        assert!(
            !Arc::ptr_eq(&usd, &eur),
            "Different currencies should have different vol surfaces"
        );

        // Verify different vols
        let usd_vol = usd.volatility(100.0, 1.0).unwrap();
        let eur_vol = eur.volatility(100.0, 1.0).unwrap();
        assert!((usd_vol - 0.20).abs() < 1e-10);
        assert!((eur_vol - 0.18).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // Double-Check Locking Verification
    // -------------------------------------------------------------------------

    #[test]
    fn test_curve_cache_population() {
        let provider = MarketProvider::new();

        // Cache should be empty initially
        {
            let cache = provider.curve_cache.read().unwrap();
            assert!(!cache.contains_key(&Currency::USD));
        }

        // Get curve populates cache
        let _curve = provider.get_curve(Currency::USD);

        // Cache should now contain USD
        {
            let cache = provider.curve_cache.read().unwrap();
            assert!(cache.contains_key(&Currency::USD));
            assert_eq!(cache.len(), 1);
        }
    }

    #[test]
    fn test_vol_cache_population() {
        let provider = MarketProvider::new();

        // Cache should be empty initially
        {
            let cache = provider.vol_cache.read().unwrap();
            assert!(!cache.contains_key(&Currency::USD));
        }

        // Get vol populates cache
        let _vol = provider.get_vol(Currency::USD);

        // Cache should now contain USD
        {
            let cache = provider.vol_cache.read().unwrap();
            assert!(cache.contains_key(&Currency::USD));
            assert_eq!(cache.len(), 1);
        }
    }

    #[test]
    fn test_independent_caches() {
        let provider = MarketProvider::new();

        // Getting curve should not affect vol cache
        let _curve = provider.get_curve(Currency::USD);

        {
            let curve_cache = provider.curve_cache.read().unwrap();
            let vol_cache = provider.vol_cache.read().unwrap();
            assert_eq!(curve_cache.len(), 1);
            assert_eq!(vol_cache.len(), 0);
        }

        // Getting vol should not affect curve cache
        let _vol = provider.get_vol(Currency::EUR);

        {
            let curve_cache = provider.curve_cache.read().unwrap();
            let vol_cache = provider.vol_cache.read().unwrap();
            assert_eq!(curve_cache.len(), 1);
            assert_eq!(vol_cache.len(), 1);
        }
    }

    // -------------------------------------------------------------------------
    // Thread Safety Tests (Basic)
    // -------------------------------------------------------------------------

    #[test]
    fn test_concurrent_access_same_currency() {
        use std::thread;

        let provider = Arc::new(MarketProvider::new());
        let mut handles = vec![];

        // Spawn multiple threads accessing the same currency
        for _ in 0..4 {
            let provider_clone = Arc::clone(&provider);
            handles.push(thread::spawn(move || {
                provider_clone.get_curve(Currency::USD)
            }));
        }

        // Collect results
        let curves: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should point to the same Arc
        for curve in curves.iter().skip(1) {
            assert!(
                Arc::ptr_eq(&curves[0], curve),
                "All threads should get same Arc"
            );
        }
    }
}

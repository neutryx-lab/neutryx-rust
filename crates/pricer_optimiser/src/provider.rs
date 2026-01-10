//! Market data provider with lazy evaluation and Arc caching.
//!
//! This module provides `MarketProvider` - a thread-safe cache for market data
//! objects that implements lazy evaluation (on-demand construction) and Arc-based
//! sharing for zero-copy access across threads.
//!
//! # Architecture Role
//!
//! `MarketProvider` is the "Pull" mechanism in the Pull-then-Push execution pattern:
//! - **Pull Phase**: Dependencies are resolved lazily via `get_curve()` / `get_vol()`
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
//! use pricer_optimiser::provider::MarketProvider;
//! use pricer_core::types::Currency;
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

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use pricer_core::types::Currency;
use pricer_models::demo::{CurveEnum, FlatCurve, SabrVolSurface, VolSurfaceEnum};

/// Thread-safe market data provider with lazy evaluation and Arc caching.
///
/// Maintains separate caches for yield curves and volatility surfaces,
/// constructing objects on first access and sharing via `Arc` thereafter.
pub struct MarketProvider {
    /// Cache for yield curves, keyed by currency.
    curve_cache: RwLock<HashMap<Currency, Arc<CurveEnum>>>,
    /// Cache for volatility surfaces, keyed by currency.
    vol_cache: RwLock<HashMap<Currency, Arc<VolSurfaceEnum>>>,
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
    /// 2. If miss, acquire write lock, check again (another thread may have populated)
    /// 3. If still miss, bootstrap curve, log, and cache
    ///
    /// # Arguments
    ///
    /// * `ccy` - The currency for which to retrieve the curve.
    ///
    /// # Returns
    ///
    /// `Arc<CurveEnum>` - shared reference to the yield curve.
    ///
    /// # Logging
    ///
    /// On cache miss, prints: `[Optimiser] Bootstrapping Yield Curve for {currency}...`
    pub fn get_curve(&self, ccy: Currency) -> Arc<CurveEnum> {
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
        println!("[Optimiser] Bootstrapping Yield Curve for {}...", ccy);

        // Create a flat curve with currency-specific rate (demo purposes)
        let rate = match ccy {
            Currency::USD => 0.05,
            Currency::EUR => 0.03,
            Currency::GBP => 0.04,
            Currency::JPY => 0.01,
            Currency::CHF => 0.02,
            _ => 0.03, // Default rate for unknown currencies
        };

        let curve = Arc::new(CurveEnum::Flat(FlatCurve { rate }));
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
    /// `Arc<VolSurfaceEnum>` - shared reference to the volatility surface.
    ///
    /// # Logging
    ///
    /// On cache miss, prints: `[Optimiser] Calibrating SABR Surface for {currency}...`
    pub fn get_vol(&self, ccy: Currency) -> Arc<VolSurfaceEnum> {
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
        println!("[Optimiser] Calibrating SABR Surface for {}...", ccy);

        // Create a SABR surface with currency-specific alpha (demo purposes)
        let alpha = match ccy {
            Currency::USD => 0.3,
            Currency::EUR => 0.25,
            Currency::GBP => 0.28,
            Currency::JPY => 0.2,
            Currency::CHF => 0.22,
            _ => 0.25, // Default alpha for unknown currencies
        };

        let vol = Arc::new(VolSurfaceEnum::Sabr(SabrVolSurface { alpha }));
        cache.insert(ccy, Arc::clone(&vol));
        vol
    }
}

impl Default for MarketProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Task 2.1: MarketProvider Structure Tests
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
    // Task 2.2: get_curve() Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_curve_returns_arc() {
        let provider = MarketProvider::new();
        let curve = provider.get_curve(Currency::USD);
        // Verify we got an Arc<CurveEnum>
        match curve.as_ref() {
            CurveEnum::Flat(flat) => {
                assert!((flat.rate - 0.05).abs() < 1e-10, "USD rate should be 0.05");
            }
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
        assert!(!Arc::ptr_eq(&usd, &jpy), "Different currencies should have different curves");

        // Verify different rates
        match (usd.as_ref(), jpy.as_ref()) {
            (CurveEnum::Flat(usd_flat), CurveEnum::Flat(jpy_flat)) => {
                assert!((usd_flat.rate - 0.05).abs() < 1e-10);
                assert!((jpy_flat.rate - 0.01).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_get_curve_discount_factor() {
        let provider = MarketProvider::new();
        let curve = provider.get_curve(Currency::USD);

        // USD rate is 0.05, so DF at 1Y should be exp(-0.05)
        let df = curve.get_df(1.0);
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // Task 2.3: get_vol() Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_vol_returns_arc() {
        let provider = MarketProvider::new();
        let vol = provider.get_vol(Currency::USD);
        // Verify we got an Arc<VolSurfaceEnum>
        match vol.as_ref() {
            VolSurfaceEnum::Sabr(sabr) => {
                assert!((sabr.alpha - 0.3).abs() < 1e-10, "USD alpha should be 0.3");
            }
        }
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
        assert!(!Arc::ptr_eq(&usd, &eur), "Different currencies should have different vol surfaces");

        // Verify different alphas
        match (usd.as_ref(), eur.as_ref()) {
            (VolSurfaceEnum::Sabr(usd_sabr), VolSurfaceEnum::Sabr(eur_sabr)) => {
                assert!((usd_sabr.alpha - 0.3).abs() < 1e-10);
                assert!((eur_sabr.alpha - 0.25).abs() < 1e-10);
            }
        }
    }

    // -------------------------------------------------------------------------
    // Task 2.7: Double-Check Locking Verification
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
            handles.push(thread::spawn(move || provider_clone.get_curve(Currency::USD)));
        }

        // Collect results
        let curves: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should point to the same Arc
        for curve in curves.iter().skip(1) {
            assert!(Arc::ptr_eq(&curves[0], curve), "All threads should get same Arc");
        }
    }
}

//! Lazy FX Volatility Surface with deferred calibration.
//!
//! Note: The implementations have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::{CacheStats, LazyFxVolSurface};

// ============================================================================
// Tests (verify re-exports work correctly)
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use infra_master::{trade::instrument_def::CurrencyPair, Currency};

    use super::*;
    use crate::market::{
        curves::{FlatCurve, FxCurve, SimpleFxCurve},
        surfaces::FxVolSurfaceBuilder,
    };

    fn make_test_fx_curve() -> Arc<dyn FxCurve<f64> + Send + Sync> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        Arc::new(SimpleFxCurve::new(pair, 1.10, domestic, foreign))
    }

    fn make_test_builder() -> FxVolSurfaceBuilder<f64> {
        let curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let expiry_1m = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();

        FxVolSurfaceBuilder::new(CurrencyPair::new(Currency::EUR, Currency::USD))
            .with_reference_date(ref_date)
            .with_fx_curve(curve)
            .add_atm_quote(expiry_1m, 0.08)
    }

    #[test]
    fn test_lazy_surface_creation() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        assert!(!lazy_surface.is_calibrated());
        assert!(!lazy_surface.has_failed());
    }

    #[test]
    fn test_lazy_calibration_on_first_vol() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        assert!(!lazy_surface.is_calibrated());

        let strike = 1.10;
        let vol = lazy_surface.volatility(strike, 0.0833);
        assert!(vol.is_ok());

        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);

        let stats = CacheStats { hits: 8, misses: 2, invalidations: 0 };
        assert!((stats.hit_rate() - 0.8).abs() < 0.001);
    }
}

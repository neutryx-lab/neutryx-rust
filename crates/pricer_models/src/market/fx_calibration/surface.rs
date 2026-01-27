//! Calibrated FX Volatility Surface implementation.
//!
//! Note: The implementations have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::{
    CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSmile, VolSurfaceError,
};

// ============================================================================
// Tests (verify re-exports work correctly)
// ============================================================================

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use chrono::NaiveDate;
    use infra_master::{trade::instrument_def::CurrencyPair, Currency};

    use super::*;
    use crate::market::{
        curves::{FlatCurve, FxCurve, SimpleFxCurve},
        surfaces::{FxVolSurfaceConfig, VolatilitySurface},
    };

    fn make_test_fx_curve() -> Arc<dyn FxCurve<f64> + Send + Sync> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        Arc::new(SimpleFxCurve::new(pair, 1.10, domestic, foreign))
    }

    fn make_test_surface() -> CalibratedFxVolSurface<f64> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let fx_curve = make_test_fx_curve();
        let config = FxVolSurfaceConfig::default();

        let mut smiles = BTreeMap::new();
        let expiry_1m = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        smiles.insert(expiry_1m, CalibratedSmile::flat(expiry_1m, 1.0 / 12.0, 0.10, 1.10));
        let expiry_1y = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        smiles.insert(expiry_1y, CalibratedSmile::flat(expiry_1y, 1.0, 0.12, 1.11));

        CalibratedFxVolSurface::new(pair, ref_date, smiles, fx_curve, config)
    }

    #[test]
    fn test_vol_surface_error_display() {
        let err = VolSurfaceError::invalid_expiry("negative");
        assert!(err.to_string().contains("negative"));
    }

    #[test]
    fn test_sabr_parameters_valid() {
        let params = SabrParameters::new(0.2, 0.5, -0.2, 0.4, 1.10, 1.0);
        assert!(params.is_valid());
    }

    #[test]
    fn test_calibrated_smile_flat() {
        let expiry = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let smile: CalibratedSmile<f64> = CalibratedSmile::flat(expiry, 0.5, 0.15, 1.10);
        let vol = smile.vol_at_strike(1.10).unwrap();
        assert!((vol - 0.15_f64).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_surface_creation() {
        let surface = make_test_surface();
        assert_eq!(surface.num_expiries(), 2);
        assert_eq!(surface.currency_pair().base, Currency::EUR);
    }

    #[test]
    fn test_calibrated_surface_vol() {
        let surface = make_test_surface();
        let vol = surface.volatility(1.10, 1.0).unwrap();
        assert!((vol - 0.12).abs() < 1e-10);
    }
}

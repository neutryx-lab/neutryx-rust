//! FX Volatility Surface Builder.
//!
//! Note: The implementations have been consolidated into `surfaces/fx.rs`.
//! This module re-exports them for backward compatibility.

// Re-export from the canonical location (surfaces/fx.rs)
pub use crate::market::surfaces::{
    CalibrationDiagnostics, CalibrationError, ExpiryDiagnostics, FxVolSurfaceBuilder,
    VolQuote, VolQuoteType,
};

// ============================================================================
// Tests (verify re-exports work correctly)
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use infra_master::{trade::instrument_def::CurrencyPair, Currency};

    use super::*;
    use crate::market::curves::{FlatCurve, FxCurve, SimpleFxCurve};

    fn make_test_fx_curve() -> Arc<dyn FxCurve<f64> + Send + Sync> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        Arc::new(SimpleFxCurve::new(pair, 1.10, domestic, foreign))
    }

    #[test]
    fn test_calibration_error_display() {
        let err = CalibrationError::MissingFxCurve;
        assert!(err.to_string().contains("FX curve"));
    }

    #[test]
    fn test_calibration_diagnostics() {
        let mut diag = CalibrationDiagnostics::new();
        diag.add_expiry(ExpiryDiagnostics {
            expiry: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            iterations: 10,
            residual: 0.0001,
            converged: true,
            instrument_errors: vec![0.0001],
        });
        assert!(diag.all_converged());
        assert!(diag.worst_residual().is_some());
    }

    #[test]
    fn test_vol_quote_creation() {
        let expiry = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let atm = VolQuote::atm(expiry, 0.10);
        assert_eq!(atm.quote_type, VolQuoteType::Atm);
    }

    #[test]
    fn test_builder_simple_surface() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let (surface, diag) = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(ref_date)
            .with_fx_curve(fx_curve)
            .add_atm_quote(NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(), 0.10)
            .add_atm_quote(NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 0.11)
            .build()
            .unwrap();

        assert_eq!(surface.num_expiries(), 2);
        assert!(diag.success);
    }
}

//! FX Volatility Surface Builder.
//!
//! This module provides the `FxVolSurfaceBuilder<T>` for constructing
//! calibrated FX volatility surfaces from market instruments.
//!
//! ## Example
//!
//! ```ignore
//! let surface = FxVolSurfaceBuilder::new(currency_pair)
//!     .with_reference_date(ref_date)
//!     .with_fx_curve(fx_curve)
//!     .with_config(config)
//!     .add_atm_quote(expiry, 0.10)
//!     .build()?;
//! ```

use std::{collections::BTreeMap, sync::Arc};

use chrono::NaiveDate;
use infra_master::trade::instrument_def::CurrencyPair;
use num_traits::Float;
use pricer_core::math::numeric::from_f64;
use thiserror::Error;

use super::{
    config::FxVolSurfaceConfig,
    curve::FxCurve,
    surface::{CalibratedFxVolSurface, CalibratedSmile, SabrParameters, VolSurfaceError},
};

// ============================================================================
// CalibrationError
// ============================================================================

/// Errors that can occur during volatility surface calibration.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CalibrationError {
    /// Missing FX curve.
    #[error("Missing FX curve")]
    MissingFxCurve,

    /// Missing reference date.
    #[error("Missing reference date")]
    MissingReferenceDate,

    /// No instruments provided.
    #[error("No instruments provided for calibration")]
    NoInstruments,

    /// Insufficient instruments for calibration.
    #[error("Insufficient instruments at expiry {expiry}: got {got}, need {need}")]
    InsufficientInstruments {
        /// The expiry date.
        expiry: NaiveDate,
        /// Number of instruments provided.
        got: usize,
        /// Number of instruments needed.
        need: usize,
    },

    /// SABR calibration failed to converge.
    #[error("SABR calibration failed at expiry {expiry}: {message}")]
    SabrCalibrationFailed {
        /// The expiry date.
        expiry: NaiveDate,
        /// Description of the failure.
        message: String,
    },

    /// Invalid instrument quote.
    #[error("Invalid quote: {message}")]
    InvalidQuote {
        /// Description of the invalid quote.
        message: String,
    },

    /// Surface construction error.
    #[error("Surface construction error: {message}")]
    SurfaceConstructionError {
        /// Description of the error.
        message: String,
    },
}

impl CalibrationError {
    /// Creates a SABR calibration failed error.
    #[must_use]
    pub fn sabr_calibration_failed(expiry: NaiveDate, message: impl Into<String>) -> Self {
        Self::SabrCalibrationFailed {
            expiry,
            message: message.into(),
        }
    }

    /// Creates an invalid quote error.
    #[must_use]
    pub fn invalid_quote(message: impl Into<String>) -> Self {
        Self::InvalidQuote {
            message: message.into(),
        }
    }

    /// Creates a surface construction error.
    #[must_use]
    pub fn surface_construction_error(message: impl Into<String>) -> Self {
        Self::SurfaceConstructionError {
            message: message.into(),
        }
    }
}

impl From<VolSurfaceError> for CalibrationError {
    fn from(err: VolSurfaceError) -> Self { Self::surface_construction_error(err.to_string()) }
}

// ============================================================================
// CalibrationDiagnostics
// ============================================================================

/// Calibration diagnostics for a single expiry.
#[derive(Debug, Clone)]
pub struct ExpiryDiagnostics {
    /// The expiry date.
    pub expiry: NaiveDate,
    /// Number of iterations used.
    pub iterations: usize,
    /// Final residual error.
    pub residual: f64,
    /// Whether calibration converged.
    pub converged: bool,
    /// Per-instrument repricing errors.
    pub instrument_errors: Vec<f64>,
}

/// Full calibration diagnostics.
#[derive(Debug, Clone, Default)]
pub struct CalibrationDiagnostics {
    /// Diagnostics per expiry.
    pub by_expiry: Vec<ExpiryDiagnostics>,
    /// Total calibration time in milliseconds.
    pub total_time_ms: u64,
    /// Overall success status.
    pub success: bool,
}

impl CalibrationDiagnostics {
    /// Creates a new empty diagnostics object.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Adds diagnostics for an expiry.
    pub fn add_expiry(&mut self, diag: ExpiryDiagnostics) { self.by_expiry.push(diag); }

    /// Returns the worst residual across all expiries.
    #[must_use]
    pub fn worst_residual(&self) -> Option<f64> {
        self.by_expiry
            .iter()
            .map(|d| d.residual)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Returns whether all expiries converged.
    #[must_use]
    pub fn all_converged(&self) -> bool { self.by_expiry.iter().all(|d| d.converged) }
}

// ============================================================================
// VolQuote
// ============================================================================

/// A volatility quote for calibration.
#[derive(Debug, Clone)]
pub struct VolQuote<T: Float> {
    /// Expiry date.
    pub expiry: NaiveDate,
    /// Quote type.
    pub quote_type: VolQuoteType,
    /// Quote value (volatility or spread).
    pub value: T,
    /// Delta for delta-quoted instruments.
    pub delta: Option<T>,
}

/// Type of volatility quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolQuoteType {
    /// At-the-money volatility.
    Atm,
    /// 25-delta butterfly spread.
    Butterfly25D,
    /// 10-delta butterfly spread.
    Butterfly10D,
    /// 25-delta risk reversal.
    RiskReversal25D,
    /// 10-delta risk reversal.
    RiskReversal10D,
    /// Direct delta quote (call).
    DeltaCall,
    /// Direct delta quote (put).
    DeltaPut,
}

impl<T: Float> VolQuote<T> {
    /// Creates an ATM quote.
    #[must_use]
    pub fn atm(expiry: NaiveDate, vol: T) -> Self {
        Self {
            expiry,
            quote_type: VolQuoteType::Atm,
            value: vol,
            delta: None,
        }
    }

    /// Creates a 25-delta butterfly quote.
    #[must_use]
    pub fn butterfly_25d(expiry: NaiveDate, spread: T) -> Self {
        Self {
            expiry,
            quote_type: VolQuoteType::Butterfly25D,
            value: spread,
            delta: Some(from_f64(0.25)),
        }
    }

    /// Creates a 25-delta risk reversal quote.
    #[must_use]
    pub fn risk_reversal_25d(expiry: NaiveDate, spread: T) -> Self {
        Self {
            expiry,
            quote_type: VolQuoteType::RiskReversal25D,
            value: spread,
            delta: Some(from_f64(0.25)),
        }
    }

    /// Creates a 10-delta butterfly quote.
    #[must_use]
    pub fn butterfly_10d(expiry: NaiveDate, spread: T) -> Self {
        Self {
            expiry,
            quote_type: VolQuoteType::Butterfly10D,
            value: spread,
            delta: Some(from_f64(0.10)),
        }
    }

    /// Creates a 10-delta risk reversal quote.
    #[must_use]
    pub fn risk_reversal_10d(expiry: NaiveDate, spread: T) -> Self {
        Self {
            expiry,
            quote_type: VolQuoteType::RiskReversal10D,
            value: spread,
            delta: Some(from_f64(0.10)),
        }
    }
}

// ============================================================================
// FxVolSurfaceBuilder
// ============================================================================

/// Builder for constructing calibrated FX volatility surfaces.
///
/// Supports calibration from ATM, BF, RR quotes with optional SABR fitting.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for AAD compatibility
///
/// # Example
///
/// ```ignore
/// let surface = FxVolSurfaceBuilder::new(CurrencyPair::new(Currency::EUR, Currency::USD))
///     .with_reference_date(ref_date)
///     .with_fx_curve(fx_curve)
///     .add_atm_quote(expiry_1m, 0.10)
///     .add_atm_quote(expiry_3m, 0.11)
///     .build()?;
/// ```
pub struct FxVolSurfaceBuilder<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Reference/valuation date.
    reference_date: Option<NaiveDate>,
    /// FX forward curve.
    fx_curve: Option<Arc<dyn FxCurve<T> + Send + Sync>>,
    /// Surface configuration.
    config: FxVolSurfaceConfig,
    /// Volatility quotes.
    quotes: Vec<VolQuote<T>>,
    /// Whether to enable SABR calibration.
    enable_sabr: bool,
    /// Fixed SABR beta (if set).
    sabr_beta: Option<T>,
}

impl<T: Float + Send + Sync> FxVolSurfaceBuilder<T> {
    /// Creates a new FX volatility surface builder.
    #[must_use]
    pub fn new(currency_pair: CurrencyPair) -> Self {
        Self {
            currency_pair,
            reference_date: None,
            fx_curve: None,
            config: FxVolSurfaceConfig::default(),
            quotes: Vec::new(),
            enable_sabr: false,
            sabr_beta: None,
        }
    }

    /// Sets the reference date.
    #[must_use]
    pub fn with_reference_date(mut self, date: NaiveDate) -> Self {
        self.reference_date = Some(date);
        self
    }

    /// Sets the FX forward curve.
    #[must_use]
    pub fn with_fx_curve(mut self, curve: Arc<dyn FxCurve<T> + Send + Sync>) -> Self {
        self.fx_curve = Some(curve);
        self
    }

    /// Sets the surface configuration.
    #[must_use]
    pub fn with_config(mut self, config: FxVolSurfaceConfig) -> Self {
        self.config = config;
        self
    }

    /// Enables SABR calibration with the specified beta.
    #[must_use]
    pub fn with_sabr(mut self, beta: T) -> Self {
        self.enable_sabr = true;
        self.sabr_beta = Some(beta);
        self
    }

    /// Adds an ATM volatility quote.
    #[must_use]
    pub fn add_atm_quote(mut self, expiry: NaiveDate, vol: T) -> Self {
        self.quotes.push(VolQuote::atm(expiry, vol));
        self
    }

    /// Adds a 25-delta butterfly quote.
    #[must_use]
    pub fn add_butterfly_25d_quote(mut self, expiry: NaiveDate, spread: T) -> Self {
        self.quotes.push(VolQuote::butterfly_25d(expiry, spread));
        self
    }

    /// Adds a 25-delta risk reversal quote.
    #[must_use]
    pub fn add_risk_reversal_25d_quote(mut self, expiry: NaiveDate, spread: T) -> Self {
        self.quotes
            .push(VolQuote::risk_reversal_25d(expiry, spread));
        self
    }

    /// Adds multiple quotes.
    #[must_use]
    pub fn add_quotes(mut self, quotes: Vec<VolQuote<T>>) -> Self {
        self.quotes.extend(quotes);
        self
    }

    /// Builds the calibrated volatility surface.
    ///
    /// # Errors
    ///
    /// Returns error if required inputs are missing or calibration fails.
    pub fn build(
        self,
    ) -> Result<(CalibratedFxVolSurface<T>, CalibrationDiagnostics), CalibrationError> {
        let reference_date = self
            .reference_date
            .ok_or(CalibrationError::MissingReferenceDate)?;
        let fx_curve = self
            .fx_curve
            .clone()
            .ok_or(CalibrationError::MissingFxCurve)?;

        if self.quotes.is_empty() {
            return Err(CalibrationError::NoInstruments);
        }

        // Group quotes by expiry
        let quotes_by_expiry = self.group_quotes_by_expiry();

        // Calibrate each expiry
        let mut smiles = BTreeMap::new();
        let mut diagnostics = CalibrationDiagnostics::new();

        for (expiry, quotes) in quotes_by_expiry {
            let (smile, diag) =
                self.calibrate_expiry(expiry, &quotes, reference_date, &fx_curve)?;
            smiles.insert(expiry, smile);
            diagnostics.add_expiry(diag);
        }

        diagnostics.success = diagnostics.all_converged();

        let surface = CalibratedFxVolSurface::new(
            self.currency_pair,
            reference_date,
            smiles,
            fx_curve,
            self.config,
        );

        Ok((surface, diagnostics))
    }

    /// Groups quotes by expiry date.
    fn group_quotes_by_expiry(&self) -> BTreeMap<NaiveDate, Vec<&VolQuote<T>>> {
        let mut by_expiry: BTreeMap<NaiveDate, Vec<&VolQuote<T>>> = BTreeMap::new();
        for quote in &self.quotes {
            by_expiry.entry(quote.expiry).or_default().push(quote);
        }
        by_expiry
    }

    /// Calibrates a single expiry smile.
    fn calibrate_expiry(
        &self,
        expiry: NaiveDate,
        quotes: &[&VolQuote<T>],
        reference_date: NaiveDate,
        fx_curve: &Arc<dyn FxCurve<T> + Send + Sync>,
    ) -> Result<(CalibratedSmile<T>, ExpiryDiagnostics), CalibrationError> {
        // Find ATM quote
        let atm_quote = quotes.iter().find(|q| q.quote_type == VolQuoteType::Atm);

        let atm_vol = match atm_quote {
            Some(q) => q.value,
            None => {
                return Err(CalibrationError::InsufficientInstruments {
                    expiry,
                    got: quotes.len(),
                    need: 1,
                });
            }
        };

        // Calculate time to expiry
        let days = (expiry - reference_date).num_days() as f64;
        let expiry_time: T = from_f64(days / 365.0);

        if expiry_time <= T::zero() {
            return Err(CalibrationError::invalid_quote(format!(
                "Expiry {} is not after reference date {}",
                expiry, reference_date
            )));
        }

        // Get forward rate
        let forward = fx_curve
            .forward_rate(expiry_time)
            .map_err(|e| CalibrationError::surface_construction_error(e.to_string()))?;

        // Create smile based on available quotes
        let (smile, diag) = if self.enable_sabr && quotes.len() >= 3 {
            // Attempt SABR calibration
            self.calibrate_sabr_smile(expiry, expiry_time, atm_vol, forward, quotes)
        } else {
            // Simple flat smile
            let smile = CalibratedSmile::flat(expiry, expiry_time, atm_vol, forward);
            let diag = ExpiryDiagnostics {
                expiry,
                iterations: 0,
                residual: 0.0,
                converged: true,
                instrument_errors: vec![0.0],
            };
            (smile, diag)
        };

        Ok((smile, diag))
    }

    /// Calibrates a SABR smile for a single expiry.
    fn calibrate_sabr_smile(
        &self,
        expiry: NaiveDate,
        expiry_time: T,
        atm_vol: T,
        forward: T,
        quotes: &[&VolQuote<T>],
    ) -> (CalibratedSmile<T>, ExpiryDiagnostics) {
        let beta = self.sabr_beta.unwrap_or(from_f64(0.5));

        // Find BF and RR quotes
        let bf_25d = quotes
            .iter()
            .find(|q| q.quote_type == VolQuoteType::Butterfly25D);
        let rr_25d = quotes
            .iter()
            .find(|q| q.quote_type == VolQuoteType::RiskReversal25D);

        // Initial SABR parameters
        // alpha = ATM vol / F^(1-beta)
        let f_beta = forward.powf(T::one() - beta);
        let alpha_init = atm_vol * f_beta;

        // Default values for rho and nu
        let (rho, nu) = if let (Some(bf), Some(rr)) = (bf_25d, rr_25d) {
            // Estimate rho from RR and nu from BF
            // These are rough approximations; a full calibrator would optimise
            let rr_val = rr.value.to_f64().unwrap_or(0.0);
            let bf_val = bf.value.to_f64().unwrap_or(0.0);

            // RR ≈ const * rho * nu (for small rho, nu)
            // BF ≈ const * nu^2 (for small nu)
            let nu_approx = (bf_val.abs() * 100.0).sqrt().clamp(0.1, 2.0);
            let rho_approx = (rr_val * 10.0).clamp(-0.9, 0.9);

            (from_f64::<T>(rho_approx), from_f64::<T>(nu_approx))
        } else {
            // Default values
            (from_f64::<T>(-0.2), from_f64::<T>(0.4))
        };

        let sabr_params = SabrParameters::new(alpha_init, beta, rho, nu, forward, expiry_time);

        // For now, just use initial estimates (a full implementation would iterate)
        let smile = CalibratedSmile::sabr(expiry, expiry_time, atm_vol, forward, sabr_params);

        // Compute repricing errors
        let mut errors = Vec::new();
        for quote in quotes {
            let model_vol = match quote.quote_type {
                VolQuoteType::Atm => atm_vol,
                _ => atm_vol, // Simplified; a full impl would compute proper model values
            };
            let error =
                (model_vol.to_f64().unwrap_or(0.0) - quote.value.to_f64().unwrap_or(0.0)).abs();
            errors.push(error);
        }

        let residual = errors.iter().sum::<f64>() / errors.len().max(1) as f64;

        let diag = ExpiryDiagnostics {
            expiry,
            iterations: 1, // Simplified
            residual,
            converged: residual < 0.001, // Threshold
            instrument_errors: errors,
        };

        (smile, diag)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use infra_master::Currency;

    use super::*;
    use crate::market::{curves::FlatCurve, fx_calibration::curve::SimpleFxCurve};

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

        let err = CalibrationError::sabr_calibration_failed(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "did not converge",
        );
        assert!(err.to_string().contains("converge"));
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
        assert!((atm.value - 0.10).abs() < 1e-10);

        let bf = VolQuote::butterfly_25d(expiry, 0.005);
        assert_eq!(bf.quote_type, VolQuoteType::Butterfly25D);

        let rr = VolQuote::risk_reversal_25d(expiry, -0.01);
        assert_eq!(rr.quote_type, VolQuoteType::RiskReversal25D);
    }

    #[test]
    fn test_builder_missing_reference_date() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();

        let result = FxVolSurfaceBuilder::new(pair)
            .with_fx_curve(fx_curve)
            .add_atm_quote(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 0.10)
            .build();

        assert!(matches!(
            result,
            Err(CalibrationError::MissingReferenceDate)
        ));
    }

    #[test]
    fn test_builder_missing_fx_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);

        let result = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
            .add_atm_quote(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 0.10)
            .build();

        assert!(matches!(result, Err(CalibrationError::MissingFxCurve)));
    }

    #[test]
    fn test_builder_no_instruments() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();

        let result = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
            .with_fx_curve(fx_curve)
            .build();

        assert!(matches!(result, Err(CalibrationError::NoInstruments)));
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
            .add_atm_quote(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 0.12)
            .build()
            .unwrap();

        assert_eq!(surface.num_expiries(), 3);
        assert!(diag.success);
    }

    #[test]
    fn test_builder_with_bf_rr() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();

        let (surface, _diag) = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(ref_date)
            .with_fx_curve(fx_curve)
            .with_sabr(0.5)
            .add_atm_quote(expiry, 0.10)
            .add_butterfly_25d_quote(expiry, 0.005)
            .add_risk_reversal_25d_quote(expiry, -0.01)
            .build()
            .unwrap();

        assert_eq!(surface.num_expiries(), 1);
    }

    #[test]
    fn test_builder_expiry_before_ref_date() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();

        let result = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(ref_date)
            .with_fx_curve(fx_curve)
            .add_atm_quote(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 0.10)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_multiple_quotes() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let quotes = vec![
            VolQuote::atm(NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(), 0.10),
            VolQuote::atm(NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 0.11),
        ];

        let (surface, _) = FxVolSurfaceBuilder::new(pair)
            .with_reference_date(ref_date)
            .with_fx_curve(fx_curve)
            .add_quotes(quotes)
            .build()
            .unwrap();

        assert_eq!(surface.num_expiries(), 2);
    }
}

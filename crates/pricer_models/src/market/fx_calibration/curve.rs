//! FX Forward Curve trait and implementations.
//!
//! This module provides:
//! - [`FxCurve`]: Generic trait for FX forward curve operations
//! - [`CalibratedFxCurve`]: Calibrated FX curve implementation
//! - [`SimpleFxCurve`]: Simple FX curve using interest rate parity
//! - [`FxCurveError`]: Error types for FX curve operations

use std::sync::Arc;

use infra_master::trade::instrument_def::CurrencyPair;
use num_traits::Float;
use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
use thiserror::Error;

use crate::market::curves::YieldCurve;

// ============================================================================
// FxCurveError
// ============================================================================

/// Errors that can occur during FX curve operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FxCurveError {
    /// Missing domestic discount curve.
    #[error("Missing domestic discount curve")]
    MissingDomesticCurve,

    /// Missing foreign discount curve.
    #[error("Missing foreign discount curve")]
    MissingForeignCurve,

    /// Missing spot rate.
    #[error("Missing spot rate")]
    MissingSpotRate,

    /// Invalid expiry (negative or otherwise invalid).
    #[error("Invalid expiry: {expiry}")]
    InvalidExpiry {
        /// The invalid expiry value.
        expiry: f64,
    },

    /// Extrapolation beyond curve bounds.
    #[error("Extrapolation not allowed: {t} is beyond curve bounds [{min}, {max}]")]
    ExtrapolationNotAllowed {
        /// The requested time.
        t: f64,
        /// Minimum curve time.
        min: f64,
        /// Maximum curve time.
        max: f64,
    },

    /// Interpolation failed.
    #[error("Interpolation failed: {message}")]
    InterpolationFailed {
        /// Description of the interpolation failure.
        message: String,
    },

    /// Discount curve error.
    #[error("Discount curve error: {message}")]
    DiscountCurveError {
        /// Description of the underlying error.
        message: String,
    },

    /// Insufficient market data for curve construction.
    #[error("Insufficient market data: expected at least {expected} points, got {got}")]
    InsufficientData {
        /// Expected minimum number of data points.
        expected: usize,
        /// Actual number of data points received.
        got: usize,
    },

    /// Bootstrap failed.
    #[error("Bootstrap failed: {message}")]
    BootstrapFailed {
        /// Description of the bootstrap failure.
        message: String,
    },
}

impl FxCurveError {
    /// Creates an invalid expiry error.
    #[must_use]
    pub fn invalid_expiry(expiry: f64) -> Self { Self::InvalidExpiry { expiry } }

    /// Creates an extrapolation not allowed error.
    #[must_use]
    pub fn extrapolation_not_allowed(t: f64, min: f64, max: f64) -> Self {
        Self::ExtrapolationNotAllowed { t, min, max }
    }

    /// Creates an interpolation failed error.
    #[must_use]
    pub fn interpolation_failed(message: impl Into<String>) -> Self {
        Self::InterpolationFailed {
            message: message.into(),
        }
    }

    /// Creates a discount curve error.
    #[must_use]
    pub fn discount_curve_error(message: impl Into<String>) -> Self {
        Self::DiscountCurveError {
            message: message.into(),
        }
    }

    /// Creates an insufficient data error.
    #[must_use]
    pub fn insufficient_data(expected: usize, got: usize) -> Self {
        Self::InsufficientData { expected, got }
    }

    /// Creates a bootstrap failed error.
    #[must_use]
    pub fn bootstrap_failed(message: impl Into<String>) -> Self {
        Self::BootstrapFailed {
            message: message.into(),
        }
    }
}

// ============================================================================
// ExtrapolationPolicy
// ============================================================================

/// Extrapolation policy for FX curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExtrapolationPolicy {
    /// Flat extrapolation (use boundary value).
    #[default]
    Flat,
    /// Linear extrapolation based on slope at boundary.
    Linear,
    /// Return error if extrapolation is requested.
    Error,
}

impl ExtrapolationPolicy {
    /// Returns a description of the extrapolation policy.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Flat => "Flat extrapolation using boundary value",
            Self::Linear => "Linear extrapolation based on boundary slope",
            Self::Error => "Return error for out-of-bounds queries",
        }
    }
}

// ============================================================================
// FxCurve Trait
// ============================================================================

/// FX Forward Curve trait.
///
/// Provides a unified interface for FX forward rate calculations, supporting
/// both calibrated curves and analytical curves.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
pub trait FxCurve<T: Float>: Send + Sync {
    /// Returns the forward rate at the given expiry.
    fn forward_rate(&self, expiry: T) -> Result<T, FxCurveError>;
    /// Returns the forward points at the given expiry.
    fn forward_points(&self, expiry: T) -> Result<T, FxCurveError>;
    /// Returns the spot rate.
    fn spot_rate(&self) -> T;
    /// Returns the domestic discount factor at the given time.
    fn discount_factor_domestic(&self, t: T) -> Result<T, FxCurveError>;
    /// Returns the foreign discount factor at the given time.
    fn discount_factor_foreign(&self, t: T) -> Result<T, FxCurveError>;
    /// Returns the currency pair for this curve.
    fn currency_pair(&self) -> CurrencyPair;
    /// Returns the maximum maturity supported by this curve.
    fn max_maturity(&self) -> Option<T> { None }

    /// Calculates the forward rate using interest rate parity.
    fn forward_rate_from_irp(&self, expiry: T) -> Result<T, FxCurveError> {
        let df_d = self.discount_factor_domestic(expiry)?;
        let df_f = self.discount_factor_foreign(expiry)?;
        Ok(self.spot_rate() * df_f / df_d)
    }
}

// ============================================================================
// CalibratedFxCurve
// ============================================================================

/// Calibrated FX Forward Curve implementation.
///
/// This curve is constructed from market instruments (FX swaps, XCCY basis swaps)
/// and stores interpolated forward points.
#[derive(Clone)]
pub struct CalibratedFxCurve<T: Float> {
    /// Currency pair (domestic/foreign).
    currency_pair: CurrencyPair,
    /// Spot exchange rate.
    spot_rate: T,
    /// Pillar times for forward points.
    pillar_times: Vec<T>,
    /// Forward points at each pillar.
    pillar_forward_points: Vec<T>,
    /// Domestic discount curve.
    domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    /// Foreign discount curve.
    foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    /// Extrapolation policy.
    extrapolation: ExtrapolationPolicy,
}

impl<T: Float + std::fmt::Debug> std::fmt::Debug for CalibratedFxCurve<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalibratedFxCurve")
            .field("currency_pair", &self.currency_pair)
            .field("spot_rate", &self.spot_rate)
            .field("pillar_times", &self.pillar_times)
            .field("pillar_forward_points", &self.pillar_forward_points)
            .field("extrapolation", &self.extrapolation)
            .finish()
    }
}

impl<T: Float> CalibratedFxCurve<T> {
    /// Creates a new calibrated FX curve from pillar data.
    pub fn new(
        currency_pair: CurrencyPair,
        spot_rate: T,
        pillar_times: Vec<T>,
        pillar_forward_points: Vec<T>,
        domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
        extrapolation: ExtrapolationPolicy,
    ) -> Result<Self, FxCurveError> {
        if pillar_times.len() < 2 {
            return Err(FxCurveError::insufficient_data(2, pillar_times.len()));
        }
        if pillar_times.len() != pillar_forward_points.len() {
            return Err(FxCurveError::insufficient_data(
                pillar_times.len(),
                pillar_forward_points.len(),
            ));
        }

        Ok(Self {
            currency_pair,
            spot_rate,
            pillar_times,
            pillar_forward_points,
            domestic_curve,
            foreign_curve,
            extrapolation,
        })
    }

    /// Creates a curve directly from discount curves using interest rate parity.
    pub fn from_discount_curves(
        currency_pair: CurrencyPair,
        spot_rate: T,
        pillar_times: &[T],
        domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
        extrapolation: ExtrapolationPolicy,
    ) -> Result<Self, FxCurveError> {
        if pillar_times.len() < 2 {
            return Err(FxCurveError::insufficient_data(2, pillar_times.len()));
        }

        let mut forward_points = Vec::with_capacity(pillar_times.len());
        for &t in pillar_times {
            let df_d = domestic_curve
                .discount_factor(t)
                .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
            let df_f = foreign_curve
                .discount_factor(t)
                .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

            let fwd_points = spot_rate * (df_f / df_d - T::one());
            forward_points.push(fwd_points);
        }

        Self::new(
            currency_pair,
            spot_rate,
            pillar_times.to_vec(),
            forward_points,
            domestic_curve,
            foreign_curve,
            extrapolation,
        )
    }

    /// Returns the extrapolation policy.
    #[inline]
    #[must_use]
    pub fn extrapolation_policy(&self) -> ExtrapolationPolicy { self.extrapolation }

    /// Returns the pillar times for the forward points curve.
    #[inline]
    #[must_use]
    pub fn pillar_times(&self) -> &[T] { &self.pillar_times }

    /// Returns the forward points values at pillars.
    #[inline]
    #[must_use]
    pub fn pillar_forward_points(&self) -> &[T] { &self.pillar_forward_points }

    #[inline]
    fn domain(&self) -> (T, T) {
        (self.pillar_times[0], self.pillar_times[self.pillar_times.len() - 1])
    }

    fn interpolate_forward_points(&self, t: T) -> Result<T, FxCurveError> {
        let t_f64 = t.to_f64().unwrap_or(0.0);
        let (t_min, t_max) = self.domain();
        let t_min_f64 = t_min.to_f64().unwrap_or(0.0);
        let t_max_f64 = t_max.to_f64().unwrap_or(0.0);

        let in_bounds = t >= t_min && t <= t_max;

        if in_bounds {
            let interp = LinearInterpolator::new(&self.pillar_times, &self.pillar_forward_points)
                .map_err(|e| FxCurveError::interpolation_failed(format!("{:?}", e)))?;
            interp
                .interpolate(t)
                .map_err(|e| FxCurveError::interpolation_failed(format!("{:?}", e)))
        } else {
            match self.extrapolation {
                ExtrapolationPolicy::Error => {
                    Err(FxCurveError::extrapolation_not_allowed(t_f64, t_min_f64, t_max_f64))
                }
                ExtrapolationPolicy::Flat => {
                    if t < t_min {
                        Ok(self.pillar_forward_points[0])
                    } else {
                        Ok(self.pillar_forward_points[self.pillar_forward_points.len() - 1])
                    }
                }
                ExtrapolationPolicy::Linear => self.forward_points_from_irp(t),
            }
        }
    }

    fn forward_points_from_irp(&self, t: T) -> Result<T, FxCurveError> {
        let df_d = self
            .domestic_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
        let df_f = self
            .foreign_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

        Ok(self.spot_rate * (df_f / df_d - T::one()))
    }
}

impl<T: Float + Send + Sync> FxCurve<T> for CalibratedFxCurve<T> {
    fn forward_rate(&self, expiry: T) -> Result<T, FxCurveError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(expiry_f64));
        }
        if expiry == T::zero() {
            return Ok(self.spot_rate);
        }
        let fp = self.forward_points(expiry)?;
        Ok(self.spot_rate + fp)
    }

    fn forward_points(&self, expiry: T) -> Result<T, FxCurveError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(expiry_f64));
        }
        if expiry == T::zero() {
            return Ok(T::zero());
        }
        self.interpolate_forward_points(expiry)
    }

    fn spot_rate(&self) -> T { self.spot_rate }

    fn discount_factor_domestic(&self, t: T) -> Result<T, FxCurveError> {
        let t_f64 = t.to_f64().unwrap_or(0.0);
        if t_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(t_f64));
        }
        self.domestic_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))
    }

    fn discount_factor_foreign(&self, t: T) -> Result<T, FxCurveError> {
        let t_f64 = t.to_f64().unwrap_or(0.0);
        if t_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(t_f64));
        }
        self.foreign_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))
    }

    fn currency_pair(&self) -> CurrencyPair { self.currency_pair }

    fn max_maturity(&self) -> Option<T> { self.pillar_times.last().copied() }
}

// ============================================================================
// SimpleFxCurve
// ============================================================================

/// Simple FX Forward Curve using Interest Rate Parity.
///
/// Calculates forward rates directly from discount curves using covered
/// interest rate parity: `F(T) = S * DF_f(T) / DF_d(T)`.
///
/// This is useful for quick calculations when calibrated forward points
/// are not available or when cross-currency basis can be ignored.
#[derive(Clone)]
pub struct SimpleFxCurve<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Spot exchange rate.
    spot_rate: T,
    /// Domestic discount curve.
    domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    /// Foreign discount curve.
    foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
}

impl<T: Float + std::fmt::Debug> std::fmt::Debug for SimpleFxCurve<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleFxCurve")
            .field("currency_pair", &self.currency_pair)
            .field("spot_rate", &self.spot_rate)
            .finish()
    }
}

impl<T: Float> SimpleFxCurve<T> {
    /// Creates a new simple FX curve.
    ///
    /// # Arguments
    ///
    /// * `currency_pair` - The currency pair (e.g., EUR/USD)
    /// * `spot_rate` - Current spot FX rate
    /// * `domestic_curve` - Domestic currency yield curve
    /// * `foreign_curve` - Foreign currency yield curve
    #[must_use]
    pub fn new(
        currency_pair: CurrencyPair,
        spot_rate: T,
        domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Self {
        Self { currency_pair, spot_rate, domestic_curve, foreign_curve }
    }
}

impl<T: Float + Send + Sync> FxCurve<T> for SimpleFxCurve<T> {
    fn forward_rate(&self, expiry: T) -> Result<T, FxCurveError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(expiry_f64));
        }
        if expiry == T::zero() {
            return Ok(self.spot_rate);
        }
        self.forward_rate_from_irp(expiry)
    }

    fn forward_points(&self, expiry: T) -> Result<T, FxCurveError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(expiry_f64));
        }
        if expiry == T::zero() {
            return Ok(T::zero());
        }
        let fwd = self.forward_rate(expiry)?;
        Ok(fwd - self.spot_rate)
    }

    fn spot_rate(&self) -> T { self.spot_rate }

    fn discount_factor_domestic(&self, t: T) -> Result<T, FxCurveError> {
        let t_f64 = t.to_f64().unwrap_or(0.0);
        if t_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(t_f64));
        }
        self.domestic_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))
    }

    fn discount_factor_foreign(&self, t: T) -> Result<T, FxCurveError> {
        let t_f64 = t.to_f64().unwrap_or(0.0);
        if t_f64 < 0.0 {
            return Err(FxCurveError::invalid_expiry(t_f64));
        }
        self.foreign_curve
            .discount_factor(t)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))
    }

    fn currency_pair(&self) -> CurrencyPair { self.currency_pair }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::curves::FlatCurve;
    use infra_master::Currency;

    fn make_test_curve() -> SimpleFxCurve<f64> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        SimpleFxCurve::new(pair, 1.10, domestic, foreign)
    }

    #[test]
    fn test_fx_curve_error_display() {
        let err = FxCurveError::MissingDomesticCurve;
        assert!(err.to_string().contains("domestic"));
        let err = FxCurveError::invalid_expiry(-1.0);
        assert!(err.to_string().contains("-1"));
        let err = FxCurveError::extrapolation_not_allowed(15.0, 0.0, 10.0);
        assert!(err.to_string().contains("15"));
        assert!(err.to_string().contains("10"));
    }

    #[test]
    fn test_extrapolation_policy_default() {
        let policy = ExtrapolationPolicy::default();
        assert_eq!(policy, ExtrapolationPolicy::Flat);
    }

    #[test]
    fn test_extrapolation_policy_description() {
        assert!(ExtrapolationPolicy::Flat.description().contains("boundary"));
        assert!(ExtrapolationPolicy::Linear.description().contains("slope"));
        assert!(ExtrapolationPolicy::Error.description().contains("error"));
    }

    #[test]
    fn test_simple_fx_curve_spot() {
        let curve = make_test_curve();
        assert!((curve.spot_rate() - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_simple_fx_curve_forward_at_zero() {
        let curve = make_test_curve();
        let fwd = curve.forward_rate(0.0).unwrap();
        assert!((fwd - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_simple_fx_curve_forward_at_one_year() {
        let curve = make_test_curve();
        let fwd = curve.forward_rate(1.0).unwrap();
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd - expected).abs() < 1e-6);
    }

    #[test]
    fn test_simple_fx_curve_forward_points() {
        let curve = make_test_curve();
        let fp = curve.forward_points(1.0).unwrap();
        let fwd = curve.forward_rate(1.0).unwrap();
        assert!((fp - (fwd - 1.10)).abs() < 1e-10);
    }

    #[test]
    fn test_simple_fx_curve_forward_points_at_zero() {
        let curve = make_test_curve();
        let fp = curve.forward_points(0.0).unwrap();
        assert!((fp - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simple_fx_curve_invalid_expiry() {
        let curve = make_test_curve();
        let result = curve.forward_rate(-1.0);
        assert!(matches!(result, Err(FxCurveError::InvalidExpiry { .. })));
    }

    #[test]
    fn test_simple_fx_curve_discount_factors() {
        let curve = make_test_curve();
        let df_d = curve.discount_factor_domestic(1.0).unwrap();
        let df_f = curve.discount_factor_foreign(1.0).unwrap();
        assert!((df_d - (-0.05_f64).exp()).abs() < 1e-10);
        assert!((df_f - (-0.03_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_simple_fx_curve_currency_pair() {
        let curve = make_test_curve();
        let pair = curve.currency_pair();
        assert_eq!(pair.base, Currency::EUR);
        assert_eq!(pair.quote, Currency::USD);
    }

    #[test]
    fn test_calibrated_fx_curve_from_discount_curves() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Flat,
        ).unwrap();
        assert!((curve.spot_rate() - 1.10).abs() < 1e-10);
        let fwd = curve.forward_rate(1.0).unwrap();
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd - expected).abs() < 1e-6);
    }

    #[test]
    fn test_calibrated_fx_curve_interpolation() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Flat,
        ).unwrap();
        let fwd_075 = curve.forward_rate(0.75).unwrap();
        let fwd_05 = curve.forward_rate(0.5).unwrap();
        let fwd_10 = curve.forward_rate(1.0).unwrap();
        assert!(fwd_075 > fwd_05);
        assert!(fwd_10 > fwd_075);
    }

    #[test]
    fn test_calibrated_fx_curve_flat_extrapolation() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Flat,
        ).unwrap();
        let fp_5y = curve.forward_points(5.0).unwrap();
        let fp_2y = curve.forward_points(2.0).unwrap();
        assert!((fp_5y - fp_2y).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_fx_curve_error_extrapolation() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Error,
        ).unwrap();
        assert!(curve.forward_rate(1.0).is_ok());
        let result = curve.forward_rate(5.0);
        assert!(matches!(result, Err(FxCurveError::ExtrapolationNotAllowed { .. })));
    }

    #[test]
    fn test_calibrated_fx_curve_max_maturity() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0, 5.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Flat,
        ).unwrap();
        let max_t = curve.max_maturity();
        assert!(max_t.is_some());
        assert!((max_t.unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_calibrated_fx_curve_pillar_access() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair, 1.10, &pillar_times, domestic, foreign, ExtrapolationPolicy::Flat,
        ).unwrap();
        let pillars = curve.pillar_times();
        assert_eq!(pillars.len(), 3);
        assert!((pillars[0] - 0.5).abs() < 1e-10);
        assert!((pillars[2] - 2.0).abs() < 1e-10);
        let fp_values = curve.pillar_forward_points();
        assert_eq!(fp_values.len(), 3);
    }

    #[test]
    fn test_forward_rate_from_irp_consistency() {
        let curve = make_test_curve();
        let fwd_1y = curve.forward_rate(1.0).unwrap();
        let fwd_irp = curve.forward_rate_from_irp(1.0).unwrap();
        assert!((fwd_1y - fwd_irp).abs() < 1e-10);
    }

    #[test]
    fn test_covered_interest_rate_parity() {
        let curve = make_test_curve();
        let t = 2.0;
        let fwd = curve.forward_rate(t).unwrap();
        let spot = curve.spot_rate();
        let df_d = curve.discount_factor_domestic(t).unwrap();
        let df_f = curve.discount_factor_foreign(t).unwrap();
        let lhs = fwd / spot;
        let rhs = df_f / df_d;
        assert!((lhs - rhs).abs() < 1e-10);
    }
}

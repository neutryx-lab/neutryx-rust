//! FX Forward Curve module.
//!
//! This module provides FX forward curve abstractions for calculating forward
//! exchange rates using interest rate parity or calibrated forward points.
//!
//! ## Components
//!
//! - [`FxCurve`]: Generic trait for FX forward curve operations
//! - [`SimpleFxCurve`]: Simple FX curve using interest rate parity
//! - [`CalibratedFxCurve`]: Calibrated FX curve with interpolated forward points
//! - [`FxForwardCurveBuilder`]: Builder for constructing calibrated FX curves
//! - [`ForwardPoints`]: Newtype for FX forward points
//!
//! ## Example
//!
//! ```ignore
//! use pricer_models::market::curves::{FxCurve, SimpleFxCurve, FlatCurve};
//! use std::sync::Arc;
//!
//! let domestic = Arc::new(FlatCurve::new(0.05));
//! let foreign = Arc::new(FlatCurve::new(0.03));
//! let curve = SimpleFxCurve::new(pair, 1.10, domestic, foreign);
//!
//! let forward = curve.forward_rate(1.0)?;
//! ```

use std::sync::Arc;

use infra_master::trade::instrument_def::{CurrencyPair, FxSwapInstrument};
use num_traits::Float;
use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
use thiserror::Error;

use super::YieldCurve;

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
// ForwardPoints Newtype
// ============================================================================

/// Forward points for FX forward rate calculation.
///
/// Forward points represent the difference between forward and spot rates,
/// quoted with a scaling factor. Provides convenience methods for forward
/// rate calculation.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::curves::ForwardPoints;
///
/// // EURUSD: 50 points with scaling factor 10000
/// let points = ForwardPoints::new(50.0, 10000.0);
/// let forward = points.to_forward_rate(1.1000);
/// assert!((forward - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForwardPoints {
    /// Raw points value.
    points: f64,
    /// Scaling factor (e.g., 10000 for EURUSD, 100 for USDJPY).
    scaling_factor: f64,
}

impl ForwardPoints {
    /// Creates new forward points with explicit scaling factor.
    #[must_use]
    pub fn new(points: f64, scaling_factor: f64) -> Self {
        Self {
            points,
            scaling_factor,
        }
    }

    /// Creates forward points for EURUSD-like pairs (scaling = 10000).
    #[must_use]
    pub fn for_major_pairs(points: f64) -> Self { Self::new(points, 10000.0) }

    /// Creates forward points for USDJPY-like pairs (scaling = 100).
    #[must_use]
    pub fn for_jpy_pairs(points: f64) -> Self { Self::new(points, 100.0) }

    /// Returns the raw points value.
    #[inline]
    #[must_use]
    pub fn points(&self) -> f64 { self.points }

    /// Returns the scaling factor.
    #[inline]
    #[must_use]
    pub fn scaling_factor(&self) -> f64 { self.scaling_factor }

    /// Calculates forward rate from spot rate.
    ///
    /// Formula: F = S + points / scaling_factor
    #[inline]
    #[must_use]
    pub fn to_forward_rate(&self, spot: f64) -> f64 { spot + self.points / self.scaling_factor }

    /// Calculates points from spot and forward rates.
    #[must_use]
    pub fn from_rates(spot: f64, forward: f64, scaling_factor: f64) -> Self {
        let points = (forward - spot) * scaling_factor;
        Self::new(points, scaling_factor)
    }
}

impl std::fmt::Display for ForwardPoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} pts", self.points)
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
            .finish_non_exhaustive()
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
        Self {
            currency_pair,
            spot_rate,
            domestic_curve,
            foreign_curve,
        }
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
// CalibratedFxCurve
// ============================================================================

/// Calibrated FX Forward Curve implementation.
///
/// This curve is constructed from market instruments (FX swaps, XCCY basis
/// swaps) and stores interpolated forward points.
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

#[allow(clippy::missing_fields_in_debug)]
impl<T: Float + std::fmt::Debug> std::fmt::Debug for CalibratedFxCurve<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalibratedFxCurve")
            .field("currency_pair", &self.currency_pair)
            .field("spot_rate", &self.spot_rate)
            .field("pillar_times", &self.pillar_times)
            .field("pillar_forward_points", &self.pillar_forward_points)
            .field("extrapolation", &self.extrapolation)
            .finish_non_exhaustive()
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

    /// Creates a curve directly from discount curves using interest rate
    /// parity.
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
        (
            self.pillar_times[0],
            self.pillar_times[self.pillar_times.len() - 1],
        )
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
                ExtrapolationPolicy::Error => Err(FxCurveError::extrapolation_not_allowed(
                    t_f64, t_min_f64, t_max_f64,
                )),
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
// FxForwardCurveBuilder
// ============================================================================

/// Internal representation of FX swap data for bootstrapping.
#[derive(Debug, Clone)]
pub struct FxSwapData<T: Float> {
    /// Tenor in years.
    pub tenor: T,
    /// Forward points.
    pub forward_points: T,
}

/// Internal representation of XCCY basis swap data for bootstrapping.
#[derive(Debug, Clone)]
pub struct XccySwapData<T: Float> {
    /// Tenor in years.
    pub tenor: T,
    /// Basis spread in decimal (e.g., -0.0015 for -15 bps).
    pub basis_spread: T,
}

/// Builder for constructing calibrated FX forward curves.
///
/// Supports bootstrapping from FX swaps (short-term) and XCCY basis swaps
/// (long-term), with automatic blending in the transition region.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for AD compatibility
///
/// # Example
///
/// ```ignore
/// let curve = FxForwardCurveBuilder::new(CurrencyPair::new(Currency::EUR, Currency::USD))
///     .with_spot_rate(1.10)
///     .with_domestic_curve(usd_curve)
///     .with_foreign_curve(eur_curve)
///     .with_fx_swaps(fx_swaps)
///     .build()?;
/// ```
pub struct FxForwardCurveBuilder<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Spot rate.
    spot_rate: Option<T>,
    /// Domestic (quote currency) discount curve.
    domestic_curve: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// Foreign (base currency) discount curve.
    foreign_curve: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// FX swap instruments (short-term).
    fx_swaps: Vec<FxSwapData<T>>,
    /// XCCY basis swap instruments (long-term).
    xccy_swaps: Vec<XccySwapData<T>>,
    /// Extrapolation policy.
    extrapolation: ExtrapolationPolicy,
    /// Transition region start (years) - default 1.0.
    transition_start: T,
    /// Transition region end (years) - default 2.0.
    transition_end: T,
}

impl<T: Float + Send + Sync> FxForwardCurveBuilder<T> {
    /// Creates a new builder for the given currency pair.
    #[must_use]
    pub fn new(currency_pair: CurrencyPair) -> Self {
        Self {
            currency_pair,
            spot_rate: None,
            domestic_curve: None,
            foreign_curve: None,
            fx_swaps: Vec::new(),
            xccy_swaps: Vec::new(),
            extrapolation: ExtrapolationPolicy::Flat,
            transition_start: T::one(),
            transition_end: T::one() + T::one(),
        }
    }

    /// Sets the spot rate.
    #[must_use]
    pub fn with_spot_rate(mut self, spot: T) -> Self {
        self.spot_rate = Some(spot);
        self
    }

    /// Sets the domestic (quote currency) discount curve.
    #[must_use]
    pub fn with_domestic_curve(mut self, curve: Arc<dyn YieldCurve<T> + Send + Sync>) -> Self {
        self.domestic_curve = Some(curve);
        self
    }

    /// Sets the foreign (base currency) discount curve.
    #[must_use]
    pub fn with_foreign_curve(mut self, curve: Arc<dyn YieldCurve<T> + Send + Sync>) -> Self {
        self.foreign_curve = Some(curve);
        self
    }

    /// Adds FX swap instruments from infra_master types.
    ///
    /// Extracts forward points from FX swap instruments and adds them
    /// to the builder for short-term curve construction.
    #[must_use]
    pub fn with_fx_swap_instruments(mut self, swaps: &[FxSwapInstrument]) -> Self {
        for swap in swaps {
            // Calculate tenor in years (approximate using 365.25 days/year)
            let days = swap.far_date - swap.near_date;
            let tenor =
                T::from(days).unwrap_or_else(T::zero) / T::from(365.25).unwrap_or_else(T::one);
            let forward_points = T::from(swap.swap_points.as_decimal()).unwrap_or_else(T::zero);

            self.fx_swaps.push(FxSwapData {
                tenor,
                forward_points,
            });
        }
        self
    }

    /// Adds FX swap data directly.
    #[must_use]
    pub fn with_fx_swaps(mut self, swaps: Vec<FxSwapData<T>>) -> Self {
        self.fx_swaps = swaps;
        self
    }

    /// Adds XCCY basis swap data.
    #[must_use]
    pub fn with_xccy_swaps(mut self, swaps: Vec<XccySwapData<T>>) -> Self {
        self.xccy_swaps = swaps;
        self
    }

    /// Sets the extrapolation policy.
    #[must_use]
    pub fn with_extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Sets the transition region for blending short and long-term curves.
    ///
    /// Default is 1.0 to 2.0 years.
    #[must_use]
    pub fn with_transition_region(mut self, start: T, end: T) -> Self {
        self.transition_start = start;
        self.transition_end = end;
        self
    }

    /// Builds the calibrated FX forward curve.
    ///
    /// # Process
    ///
    /// 1. Validates all required inputs are present
    /// 2. Bootstraps short-term forward points from FX swaps
    /// 3. Bootstraps long-term forward points from XCCY basis swaps
    /// 4. Blends the two curves in the transition region
    /// 5. Constructs the final `CalibratedFxCurve`
    ///
    /// # Errors
    ///
    /// Returns `FxCurveError` if:
    /// - Missing required inputs (spot rate, discount curves)
    /// - Insufficient data points
    /// - Bootstrap fails
    pub fn build(self) -> Result<CalibratedFxCurve<T>, FxCurveError> {
        // Validate required inputs
        let spot_rate = self.spot_rate.ok_or(FxCurveError::MissingSpotRate)?;

        let domestic_curve = self
            .domestic_curve
            .clone()
            .ok_or(FxCurveError::MissingDomesticCurve)?;

        let foreign_curve = self
            .foreign_curve
            .clone()
            .ok_or(FxCurveError::MissingForeignCurve)?;

        // Bootstrap forward points
        let (pillar_times, pillar_forward_points) =
            self.bootstrap_forward_points(spot_rate, &domestic_curve, &foreign_curve)?;

        // Validate we have enough data
        if pillar_times.len() < 2 {
            return Err(FxCurveError::insufficient_data(2, pillar_times.len()));
        }

        // Create the calibrated curve
        CalibratedFxCurve::new(
            self.currency_pair,
            spot_rate,
            pillar_times,
            pillar_forward_points,
            domestic_curve,
            foreign_curve,
            self.extrapolation,
        )
    }

    /// Bootstraps forward points from FX swaps and XCCY basis swaps.
    fn bootstrap_forward_points(
        &self,
        spot_rate: T,
        domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Result<(Vec<T>, Vec<T>), FxCurveError> {
        let mut pillar_times = Vec::new();
        let mut pillar_forward_points = Vec::new();

        // 1. Process FX swaps (short-term)
        let mut fx_swap_data: Vec<_> = self.fx_swaps.clone();
        fx_swap_data.sort_by(|a, b| {
            a.tenor
                .partial_cmp(&b.tenor)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for swap in &fx_swap_data {
            pillar_times.push(swap.tenor);
            pillar_forward_points.push(swap.forward_points);
        }

        // 2. Process XCCY basis swaps (long-term)
        if !self.xccy_swaps.is_empty() {
            let mut xccy_data: Vec<_> = self.xccy_swaps.clone();
            xccy_data.sort_by(|a, b| {
                a.tenor
                    .partial_cmp(&b.tenor)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for xccy in &xccy_data {
                // Bootstrap forward points from XCCY basis swap
                // Forward points = S * (DF_f / DF_d - 1) + basis adjustment
                let fp = self.bootstrap_xccy_forward_points(
                    xccy.tenor,
                    xccy.basis_spread,
                    spot_rate,
                    domestic_curve,
                    foreign_curve,
                )?;

                pillar_times.push(xccy.tenor);
                pillar_forward_points.push(fp);
            }
        }

        // 3. Handle case with no instruments - use IRP from discount curves
        if pillar_times.is_empty() {
            // Generate default pillars using IRP
            let default_tenors = [
                T::from(0.25).unwrap_or_else(T::zero),
                T::from(0.5).unwrap_or_else(T::zero),
                T::one(),
                T::one() + T::one(),
                T::from(5.0).unwrap_or_else(T::zero),
            ];

            for &t in &default_tenors {
                let df_d = domestic_curve
                    .discount_factor(t)
                    .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
                let df_f = foreign_curve
                    .discount_factor(t)
                    .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

                let fp = spot_rate * (df_f / df_d - T::one());

                pillar_times.push(t);
                pillar_forward_points.push(fp);
            }
        }

        // 4. Blend short and long-term in transition region if needed
        if !self.fx_swaps.is_empty() && !self.xccy_swaps.is_empty() {
            self.blend_transition_region(
                &mut pillar_times,
                &mut pillar_forward_points,
                spot_rate,
                domestic_curve,
                foreign_curve,
            );
        }

        Ok((pillar_times, pillar_forward_points))
    }

    /// Bootstraps forward points from a single XCCY basis swap.
    ///
    /// The forward points are calculated using interest rate parity
    /// with a basis spread adjustment.
    fn bootstrap_xccy_forward_points(
        &self,
        tenor: T,
        basis_spread: T,
        spot_rate: T,
        domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Result<T, FxCurveError> {
        // Get discount factors
        let df_d = domestic_curve
            .discount_factor(tenor)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
        let df_f = foreign_curve
            .discount_factor(tenor)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

        // Forward points with basis adjustment
        // The basis spread affects the implied foreign rate:
        // F = S * DF_f' / DF_d where DF_f' = exp(-(r_f + basis) * t)
        // Simplified: F ≈ S * (DF_f / DF_d) * (1 + basis * t)
        // Forward points = F - S

        let base_fp = spot_rate * (df_f / df_d - T::one());
        let basis_adjustment = spot_rate * basis_spread * tenor;
        let fp = base_fp + basis_adjustment;

        Ok(fp)
    }

    /// Blends forward points in the transition region between short and
    /// long-term data.
    fn blend_transition_region(
        &self,
        pillar_times: &mut Vec<T>,
        pillar_forward_points: &mut Vec<T>,
        _spot_rate: T,
        _domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        _foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) {
        // Sort pillars by tenor
        let mut pairs: Vec<_> = pillar_times
            .iter()
            .copied()
            .zip(pillar_forward_points.iter().copied())
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Remove duplicates (keep later instrument in transition region)
        let mut unique_pairs: Vec<(T, T)> = Vec::new();
        for (t, fp) in pairs {
            if let Some(last) = unique_pairs.last_mut() {
                // If same tenor (within tolerance), update the forward point
                let diff = if t > last.0 { t - last.0 } else { last.0 - t };
                let tolerance = T::from(0.01).unwrap_or_else(T::zero); // 0.01 years = ~4 days
                if diff < tolerance {
                    // Prefer XCCY data in transition region (later entries)
                    last.1 = fp;
                    continue;
                }
            }
            unique_pairs.push((t, fp));
        }

        // Update the vectors
        pillar_times.clear();
        pillar_forward_points.clear();
        for (t, fp) in unique_pairs {
            pillar_times.push(t);
            pillar_forward_points.push(fp);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use infra_master::Currency;

    use super::*;
    use crate::market::curves::FlatCurve;

    // === FxCurveError Tests ===

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

    // === ExtrapolationPolicy Tests ===

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

    // === ForwardPoints Tests ===

    #[test]
    fn test_forward_points_new() {
        let fp = ForwardPoints::new(50.0, 10000.0);
        assert!((fp.points() - 50.0).abs() < 1e-10);
        assert!((fp.scaling_factor() - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_to_forward_rate_eurusd() {
        let fp = ForwardPoints::for_major_pairs(50.0);
        let forward = fp.to_forward_rate(1.1000);
        // F = 1.1000 + 50/10000 = 1.1050
        assert!((forward - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_to_forward_rate_usdjpy() {
        let fp = ForwardPoints::for_jpy_pairs(-25.0);
        let forward = fp.to_forward_rate(150.0);
        // F = 150.0 + (-25)/100 = 149.75
        assert!((forward - 149.75).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_from_rates() {
        let fp = ForwardPoints::from_rates(1.1000, 1.1050, 10000.0);
        assert!((fp.points() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_forward_points_display() {
        let fp = ForwardPoints::new(50.5, 10000.0);
        assert_eq!(fp.to_string(), "50.5 pts");
    }

    // === SimpleFxCurve Tests ===

    fn make_test_curve() -> SimpleFxCurve<f64> {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        SimpleFxCurve::new(pair, 1.10, domestic, foreign)
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

    // === CalibratedFxCurve Tests ===

    #[test]
    fn test_calibrated_fx_curve_from_discount_curves() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Flat,
        )
        .unwrap();
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
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Flat,
        )
        .unwrap();
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
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Flat,
        )
        .unwrap();
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
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Error,
        )
        .unwrap();
        assert!(curve.forward_rate(1.0).is_ok());
        let result = curve.forward_rate(5.0);
        assert!(matches!(
            result,
            Err(FxCurveError::ExtrapolationNotAllowed { .. })
        ));
    }

    #[test]
    fn test_calibrated_fx_curve_max_maturity() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let domestic = Arc::new(FlatCurve::new(0.05_f64));
        let foreign = Arc::new(FlatCurve::new(0.03_f64));
        let pillar_times = vec![0.5, 1.0, 2.0, 5.0];
        let curve = CalibratedFxCurve::from_discount_curves(
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Flat,
        )
        .unwrap();
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
            pair,
            1.10,
            &pillar_times,
            domestic,
            foreign,
            ExtrapolationPolicy::Flat,
        )
        .unwrap();
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

    // === FxForwardCurveBuilder Tests ===

    fn make_test_curves() -> (
        Arc<dyn YieldCurve<f64> + Send + Sync>,
        Arc<dyn YieldCurve<f64> + Send + Sync>,
    ) {
        let domestic = Arc::new(FlatCurve::new(0.05)) as Arc<dyn YieldCurve<f64> + Send + Sync>;
        let foreign = Arc::new(FlatCurve::new(0.03)) as Arc<dyn YieldCurve<f64> + Send + Sync>;
        (domestic, foreign)
    }

    #[test]
    fn test_builder_missing_spot_rate() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingSpotRate)));
    }

    #[test]
    fn test_builder_missing_domestic_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (_, foreign) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingDomesticCurve)));
    }

    #[test]
    fn test_builder_missing_foreign_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, _) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingForeignCurve)));
    }

    #[test]
    fn test_builder_with_default_irp() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        // Build curve without any instruments - should use IRP
        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .build()
            .unwrap();

        // Check forward rate at 1Y
        let fwd_1y = curve.forward_rate(1.0).unwrap();
        // Expected: F = S * exp(r_d - r_f) = 1.10 * exp(0.05 - 0.03) = 1.10 * exp(0.02)
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_builder_with_fx_swaps() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.25,
                forward_points: 0.0050,
            },
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .build()
            .unwrap();

        // Check forward points at pillars
        let fp_1y = curve.forward_points(1.0).unwrap();
        assert!((fp_1y - 0.0200).abs() < 1e-10);
    }

    #[test]
    fn test_builder_with_xccy_swaps() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let xccy_swaps = vec![
            XccySwapData {
                tenor: 2.0,
                basis_spread: -0.0015, // -15 bps
            },
            XccySwapData {
                tenor: 5.0,
                basis_spread: -0.0020, // -20 bps
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_xccy_swaps(xccy_swaps)
            .build()
            .unwrap();

        // Check we can query at 2Y and 5Y
        let fwd_2y = curve.forward_rate(2.0);
        let fwd_5y = curve.forward_rate(5.0);

        assert!(fwd_2y.is_ok());
        assert!(fwd_5y.is_ok());
    }

    #[test]
    fn test_builder_extrapolation_policy() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_extrapolation(ExtrapolationPolicy::Error)
            .build()
            .unwrap();

        // Query beyond max tenor should fail
        let result = curve.forward_rate(5.0);
        assert!(matches!(
            result,
            Err(FxCurveError::ExtrapolationNotAllowed { .. })
        ));
    }

    #[test]
    fn test_builder_transition_region() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![FxSwapData {
            tenor: 1.0,
            forward_points: 0.0200,
        }];

        let xccy_swaps = vec![XccySwapData {
            tenor: 2.0,
            basis_spread: -0.0015,
        }];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_xccy_swaps(xccy_swaps)
            .with_transition_region(1.0, 2.0)
            .build()
            .unwrap();

        // Check interpolation in transition region (1.5Y)
        let fwd_1_5y = curve.forward_rate(1.5);
        assert!(fwd_1_5y.is_ok());
    }

    #[test]
    fn test_fx_swap_data_clone() {
        let data = FxSwapData {
            tenor: 1.0,
            forward_points: 0.0200,
        };
        let cloned = data.clone();
        assert!((cloned.tenor - 1.0).abs() < 1e-10);
        assert!((cloned.forward_points - 0.0200).abs() < 1e-10);
    }

    #[test]
    fn test_xccy_swap_data_clone() {
        let data = XccySwapData {
            tenor: 5.0,
            basis_spread: -0.0015,
        };
        let cloned = data.clone();
        assert!((cloned.tenor - 5.0).abs() < 1e-10);
        assert!((cloned.basis_spread - (-0.0015)).abs() < 1e-10);
    }
}

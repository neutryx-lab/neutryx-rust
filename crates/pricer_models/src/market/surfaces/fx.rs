//! FX volatility surface abstractions for FX option pricing.
//!
//! This module provides the complete FX volatility surface stack:
//!
//! ## Core Types
//! - [`FxVolatilitySurface`]: Delta-expiry based volatility surface for FX options
//! - [`FxDeltaPoint`]: Standard delta points used in FX markets
//! - [`Strike`]: Newtype for FX option strikes
//! - [`Vol`]: Newtype for implied volatility
//!
//! ## Configuration
//! - [`ExpiryInterpolation`]: Expiry dimension interpolation methods
//! - [`FxVolSurfaceConfig`]: Configuration for FX vol surface calibration
//!
//! ## Calibrated Surfaces
//! - [`CalibratedFxVolSurface`]: SABR-calibrated FX volatility surface
//! - [`CalibratedSmile`]: Per-expiry calibrated smile parameters
//! - [`SabrParameters`]: SABR model parameters
//! - [`VolSmile`]: Extracted smile data for visualisation
//!
//! ## Builders
//! - [`FxVolSurfaceBuilder`]: Builder for constructing calibrated surfaces
//! - [`VolQuote`]: Volatility quote for calibration
//! - [`CalibrationDiagnostics`]: Calibration result diagnostics
//!
//! ## Lazy Evaluation
//! - [`LazyFxVolSurface`]: Lazy wrapper with deferred calibration
//! - [`CacheStats`]: Cache usage statistics
//!
//! ## Probability Density
//! - [`FxDensityCalculator`]: Risk-neutral density calculation
//! - [`DeltaType`]: Delta convention types
//! - [`DensityStatistics`]: Distribution statistics
//!
//! ## Errors
//! - [`VolSurfaceError`]: Volatility surface operation errors
//! - [`CalibrationError`]: Calibration-specific errors

use std::{collections::BTreeMap, sync::Arc};

use chrono::NaiveDate;
use infra_master::trade::instrument_def::CurrencyPair;
use num_traits::Float;
use pricer_core::math::{
    distributions::norm_cdf, interpolators::BilinearInterpolator, numeric::from_f64,
};
use std::sync::RwLock;
use thiserror::Error;

use super::VolatilitySurface;
use crate::market::curves::FxCurve;
use crate::market::error::MarketDataError;
use crate::market::volcube::{ExtrapolationMethod, InterpolationMethod, StrikeAxisType};

// ============================================================================
// Strike Newtype (consolidated from fx_calibration/types.rs)
// ============================================================================

/// Strike price newtype for FX options.
///
/// Stores the strike as an absolute FX rate (quote currency per base currency).
/// Provides type safety and prevents confusion with other f64 values.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::surfaces::Strike;
///
/// let strike = Strike::new(1.1050);
/// assert!((strike.value() - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Strike(f64);

impl Strike {
    /// Creates a new Strike value.
    ///
    /// # Arguments
    ///
    /// * `value` - The strike price (should be positive)
    #[must_use]
    pub fn new(value: f64) -> Self { Self(value) }

    /// Returns the strike value.
    #[inline]
    #[must_use]
    pub fn value(&self) -> f64 { self.0 }

    /// Converts strike to log-moneyness given forward rate.
    ///
    /// Log-moneyness = ln(K/F)
    #[inline]
    #[must_use]
    pub fn log_moneyness(&self, forward: f64) -> f64 { (self.0 / forward).ln() }

    /// Converts strike to moneyness given forward rate.
    ///
    /// Moneyness = K/F
    #[inline]
    #[must_use]
    pub fn moneyness(&self, forward: f64) -> f64 { self.0 / forward }
}

impl From<f64> for Strike {
    fn from(value: f64) -> Self { Self(value) }
}

impl std::fmt::Display for Strike {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.6}", self.0)
    }
}

// ============================================================================
// Vol Newtype (consolidated from fx_calibration/types.rs)
// ============================================================================

/// Implied volatility newtype.
///
/// Stores volatility as an annualised decimal (e.g., 0.10 for 10%).
/// Ensures type safety and clear intent in function signatures.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::surfaces::Vol;
///
/// let vol = Vol::from_decimal(0.10);
/// assert!((vol.as_percent() - 10.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vol(f64);

impl Vol {
    /// Creates volatility from decimal value.
    ///
    /// # Arguments
    ///
    /// * `decimal` - Volatility as decimal (e.g., 0.10 for 10%)
    #[must_use]
    pub fn from_decimal(decimal: f64) -> Self { Self(decimal) }

    /// Creates volatility from percentage value.
    ///
    /// # Arguments
    ///
    /// * `percent` - Volatility as percentage (e.g., 10.0 for 10%)
    #[must_use]
    pub fn from_percent(percent: f64) -> Self { Self(percent / 100.0) }

    /// Creates volatility from basis points.
    ///
    /// # Arguments
    ///
    /// * `bps` - Volatility in basis points (e.g., 1000 for 10%)
    #[must_use]
    pub fn from_bps(bps: f64) -> Self { Self(bps / 10000.0) }

    /// Returns the volatility as a decimal.
    #[inline]
    #[must_use]
    pub fn as_decimal(&self) -> f64 { self.0 }

    /// Returns the volatility as a percentage.
    #[inline]
    #[must_use]
    pub fn as_percent(&self) -> f64 { self.0 * 100.0 }

    /// Returns the volatility in basis points.
    #[inline]
    #[must_use]
    pub fn as_bps(&self) -> f64 { self.0 * 10000.0 }

    /// Validates that the volatility is positive.
    #[must_use]
    pub fn is_valid(&self) -> bool { self.0 > 0.0 }
}

impl From<f64> for Vol {
    fn from(value: f64) -> Self { Self(value) }
}

impl std::fmt::Display for Vol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}%", self.0 * 100.0)
    }
}

// ============================================================================
// ExpiryInterpolation Enum (consolidated from fx_calibration/types.rs)
// ============================================================================

/// Expiry (time) dimension interpolation method.
///
/// Controls how volatility is interpolated between expiry pillar dates.
/// Different methods have different smoothness and arbitrage properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpiryInterpolation {
    /// Linear interpolation in time.
    ///
    /// Simple and fast, but may produce non-smooth term structure.
    #[default]
    Linear,

    /// Flat forward volatility interpolation.
    ///
    /// Assumes constant forward volatility between pillars.
    /// Produces more stable hedging behaviour.
    FlatForward,

    /// Cubic spline interpolation in time.
    ///
    /// Smooth C2 continuous term structure.
    /// May require monotonicity constraints.
    CubicSpline,

    /// Linear variance interpolation.
    ///
    /// Interpolates total variance (sigma^2 * T) linearly.
    /// Ensures no calendar arbitrage for vanilla options.
    LinearVariance,
}

impl ExpiryInterpolation {
    /// Returns a description of the interpolation method.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Linear => "Linear interpolation in time",
            Self::FlatForward => "Flat forward volatility",
            Self::CubicSpline => "Cubic spline (C2 continuous)",
            Self::LinearVariance => "Linear total variance",
        }
    }
}

impl std::fmt::Display for ExpiryInterpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Linear => "Linear",
            Self::FlatForward => "FlatForward",
            Self::CubicSpline => "CubicSpline",
            Self::LinearVariance => "LinearVariance",
        };
        write!(f, "{}", name)
    }
}

// ============================================================================
// FxVolSurfaceConfig (consolidated from fx_calibration/config.rs)
// ============================================================================

/// FX Volatility Surface Configuration.
///
/// Comprehensive configuration for FX vol surface calibration and
/// interpolation. Uses the builder pattern for ergonomic configuration.
///
/// # Example
///
/// ```rust
/// use pricer_models::market::surfaces::FxVolSurfaceConfig;
/// use pricer_models::market::volcube::InterpolationMethod;
///
/// let config = FxVolSurfaceConfig::default()
///     .with_smile_interpolation(InterpolationMethod::Sabr)
///     .with_sabr_beta(0.5)
///     .with_allow_extrapolation(true);
///
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FxVolSurfaceConfig {
    // === Smile (Strike) Dimension ===
    /// Smile interpolation method (SABR, SVI, etc.).
    pub smile_interpolation: InterpolationMethod,

    /// Strike axis type (delta, moneyness, etc.).
    pub strike_axis: StrikeAxisType,

    /// Extrapolation method for strikes outside the quoted range.
    pub strike_extrapolation: ExtrapolationMethod,

    // === Expiry (Time) Dimension ===
    /// Expiry interpolation method.
    pub expiry_interpolation: ExpiryInterpolation,

    /// Whether to allow extrapolation beyond the expiry range.
    pub allow_extrapolation: bool,

    // === SABR Parameters ===
    /// SABR beta (fixed). None = calibrate beta.
    pub sabr_beta: Option<f64>,

    /// SABR shift for negative rates (shifted SABR).
    pub sabr_shift: f64,

    // === Calibration Settings ===
    /// Maximum iterations for calibration optimiser.
    pub max_iterations: usize,

    /// Convergence tolerance.
    pub tolerance: f64,

    /// Enable arbitrage-free constraint checking.
    pub check_arbitrage_free: bool,

    // === Forward Curve ===
    /// Use forward points for forward curve (vs discount curve ratio).
    pub use_forward_points: bool,
}

impl Default for FxVolSurfaceConfig {
    fn default() -> Self {
        Self {
            smile_interpolation: InterpolationMethod::Sabr,
            strike_axis: StrikeAxisType::Delta,
            strike_extrapolation: ExtrapolationMethod::Flat,
            expiry_interpolation: ExpiryInterpolation::Linear,
            allow_extrapolation: true,
            sabr_beta: Some(0.5),
            sabr_shift: 0.0,
            max_iterations: 100,
            tolerance: 1e-8,
            check_arbitrage_free: false,
            use_forward_points: true,
        }
    }
}

impl FxVolSurfaceConfig {
    /// Creates a new config with default settings.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the smile interpolation method.
    #[must_use]
    pub fn with_smile_interpolation(mut self, method: InterpolationMethod) -> Self {
        self.smile_interpolation = method;
        self
    }

    /// Sets the strike axis type.
    #[must_use]
    pub fn with_strike_axis(mut self, axis: StrikeAxisType) -> Self {
        self.strike_axis = axis;
        self
    }

    /// Sets the strike extrapolation method.
    #[must_use]
    pub fn with_strike_extrapolation(mut self, method: ExtrapolationMethod) -> Self {
        self.strike_extrapolation = method;
        self
    }

    /// Sets the expiry interpolation method.
    #[must_use]
    pub fn with_expiry_interpolation(mut self, method: ExpiryInterpolation) -> Self {
        self.expiry_interpolation = method;
        self
    }

    /// Sets whether extrapolation is allowed.
    #[must_use]
    pub fn with_allow_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Sets the SABR beta parameter.
    #[must_use]
    pub fn with_sabr_beta(mut self, beta: f64) -> Self {
        self.sabr_beta = Some(beta);
        self
    }

    /// Sets the SABR shift for negative rates.
    #[must_use]
    pub fn with_sabr_shift(mut self, shift: f64) -> Self {
        self.sabr_shift = shift;
        self
    }

    /// Sets the maximum iterations for calibration.
    #[must_use]
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// Sets the convergence tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Enables or disables arbitrage-free checking.
    #[must_use]
    pub fn with_check_arbitrage_free(mut self, check: bool) -> Self {
        self.check_arbitrage_free = check;
        self
    }

    /// Sets whether to use forward points for forward curve.
    #[must_use]
    pub fn with_use_forward_points(mut self, use_points: bool) -> Self {
        self.use_forward_points = use_points;
        self
    }

    /// Creates a config preset for EURUSD-like pairs.
    ///
    /// - SABR interpolation
    /// - Spot delta convention
    /// - Beta = 0.5
    #[must_use]
    pub fn eurusd_preset() -> Self {
        Self::default()
            .with_smile_interpolation(InterpolationMethod::Sabr)
            .with_strike_axis(StrikeAxisType::Delta)
            .with_sabr_beta(0.5)
    }

    /// Creates a config preset for USDJPY-like pairs.
    ///
    /// - SABR interpolation
    /// - Premium-adjusted delta
    /// - Beta = 0.5
    #[must_use]
    pub fn usdjpy_preset() -> Self {
        Self::default()
            .with_smile_interpolation(InterpolationMethod::Sabr)
            .with_strike_axis(StrikeAxisType::Delta)
            .with_sabr_beta(0.5)
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Validate SABR beta
        if let Some(beta) = self.sabr_beta {
            if !(0.0..=1.0).contains(&beta) {
                return Err(format!("SABR beta must be in [0, 1], got: {}", beta));
            }
        }

        // Validate tolerance
        if self.tolerance <= 0.0 {
            return Err(format!(
                "Tolerance must be positive, got: {}",
                self.tolerance
            ));
        }

        // Validate max iterations
        if self.max_iterations == 0 {
            return Err("Max iterations must be at least 1".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// FxDeltaPoint
// ============================================================================

/// Standard delta points used in FX volatility quoting.
///
/// In FX markets, volatility is typically quoted for specific delta points
/// rather than for absolute strikes. Common convention includes:
/// - 10D Put (10 delta put)
/// - 25D Put (25 delta put)
/// - ATM (at-the-money, typically 50 delta straddle)
/// - 25D Call (25 delta call)
/// - 10D Call (10 delta call)
///
/// # Example
///
/// ```
/// use pricer_models::market::surfaces::FxDeltaPoint;
///
/// let atm = FxDeltaPoint::Atm;
/// assert!((atm.as_delta() - 0.5).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxDeltaPoint {
    /// 10 delta put
    Put10D,
    /// 25 delta put
    Put25D,
    /// At-the-money (50 delta)
    Atm,
    /// 25 delta call
    Call25D,
    /// 10 delta call
    Call10D,
}

impl FxDeltaPoint {
    /// Return the delta value for this point.
    ///
    /// # Returns
    ///
    /// The delta value in the range [-1, 1], where:
    /// - Puts have negative delta
    /// - ATM is approximately 0.5
    /// - Calls have positive delta
    ///
    /// Note: Returns absolute delta values for simplicity (0 to 1 scale).
    #[inline]
    pub fn as_delta(&self) -> f64 {
        match self {
            FxDeltaPoint::Put10D => 0.1,
            FxDeltaPoint::Put25D => 0.25,
            FxDeltaPoint::Atm => 0.5,
            FxDeltaPoint::Call25D => 0.75,
            FxDeltaPoint::Call10D => 0.9,
        }
    }

    /// Return all standard delta points in order.
    #[inline]
    pub fn all() -> [FxDeltaPoint; 5] {
        [
            FxDeltaPoint::Put10D,
            FxDeltaPoint::Put25D,
            FxDeltaPoint::Atm,
            FxDeltaPoint::Call25D,
            FxDeltaPoint::Call10D,
        ]
    }
}

/// FX volatility surface using delta × expiry grid.
///
/// This surface stores implied volatilities for FX options organized by
/// delta (moneyness) and expiry. This is the standard market convention
/// for quoting FX volatilities.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Grid Structure
///
/// - X-axis: Delta points (0.1, 0.25, 0.5, 0.75, 0.9)
/// - Y-axis: Expiry tenors in years
/// - Values: Implied volatilities
///
/// # Example
///
/// ```
/// use pricer_models::market::surfaces::FxVolatilitySurface;
///
/// // Create a 5x3 surface (5 deltas × 3 expiries)
/// let deltas = [0.1_f64, 0.25, 0.5, 0.75, 0.9];
/// let expiries = [0.25, 1.0, 2.0];
/// let vols = [
///     // 10D Put, 25D Put, ATM, 25D Call, 10D Call for each expiry
///     [0.12, 0.11, 0.10, 0.11, 0.12],  // 3M
///     [0.13, 0.12, 0.11, 0.12, 0.13],  // 1Y
///     [0.14, 0.13, 0.12, 0.13, 0.14],  // 2Y
/// ];
///
/// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
///
/// // Get ATM vol at 1 year
/// let atm_vol = surface.atm_volatility(1.0).unwrap();
/// assert!((atm_vol - 0.11).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct FxVolatilitySurface<T: Float> {
    /// Delta points (X-axis)
    deltas: Vec<T>,
    /// Expiry tenors in years (Y-axis)
    expiries: Vec<T>,
    /// Volatility grid (expiry × delta)
    volatilities: Vec<Vec<T>>,
    /// Whether to allow extrapolation
    allow_extrapolation: bool,
}

impl<T: Float> FxVolatilitySurface<T> {
    /// Construct an FX volatility surface from a delta × expiry grid.
    ///
    /// # Arguments
    ///
    /// * `deltas` - Delta points (must be sorted, at least 2 points)
    /// * `expiries` - Expiry tenors in years (must be sorted, at least 2
    ///   points)
    /// * `volatilities` - Grid of volatilities \[expiry\]\[delta\]
    /// * `allow_extrapolation` - Whether to allow flat extrapolation
    ///
    /// # Returns
    ///
    /// * `Ok(FxVolatilitySurface)` - Successfully constructed surface
    /// * `Err(MarketDataError)` - If input validation fails
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::market::surfaces::FxVolatilitySurface;
    ///
    /// let deltas = [0.25_f64, 0.5, 0.75];
    /// let expiries = [0.5, 1.0];
    /// let vols = [
    ///     [0.11, 0.10, 0.11],
    ///     [0.12, 0.11, 0.12],
    /// ];
    ///
    /// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
    /// ```
    pub fn new(
        deltas: &[T],
        expiries: &[T],
        volatilities: &[impl AsRef<[T]>],
        allow_extrapolation: bool,
    ) -> Result<Self, MarketDataError> {
        // Validate delta points
        if deltas.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: deltas.len(),
                need: 2,
            });
        }

        // Validate expiries
        if expiries.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: expiries.len(),
                need: 2,
            });
        }

        // Validate deltas are sorted and in valid range
        for i in 0..deltas.len() {
            if deltas[i] <= T::zero() || deltas[i] >= T::one() {
                return Err(MarketDataError::InvalidStrike {
                    strike: deltas[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && deltas[i] <= deltas[i - 1] {
                return Err(MarketDataError::InvalidStrike {
                    strike: deltas[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate expiries are sorted and positive
        for i in 0..expiries.len() {
            if expiries[i] <= T::zero() {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && expiries[i] <= expiries[i - 1] {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate grid dimensions
        if volatilities.len() != expiries.len() {
            return Err(MarketDataError::InsufficientData {
                got: volatilities.len(),
                need: expiries.len(),
            });
        }

        let mut vol_grid = Vec::with_capacity(expiries.len());
        for row in volatilities {
            let row_ref = row.as_ref();
            if row_ref.len() != deltas.len() {
                return Err(MarketDataError::InsufficientData {
                    got: row_ref.len(),
                    need: deltas.len(),
                });
            }

            // Validate volatilities are positive
            for &vol in row_ref {
                if vol <= T::zero() {
                    return Err(MarketDataError::InterpolationFailed {
                        reason: format!(
                            "Volatility must be positive, got {}",
                            vol.to_f64().unwrap_or(0.0)
                        ),
                    });
                }
            }

            vol_grid.push(row_ref.to_vec());
        }

        Ok(Self {
            deltas: deltas.to_vec(),
            expiries: expiries.to_vec(),
            volatilities: vol_grid,
            allow_extrapolation,
        })
    }

    /// Return the delta domain.
    #[inline]
    pub fn delta_domain(&self) -> (T, T) { (self.deltas[0], self.deltas[self.deltas.len() - 1]) }

    /// Return whether extrapolation is allowed.
    #[inline]
    pub fn allow_extrapolation(&self) -> bool { self.allow_extrapolation }

    /// Get the ATM (at-the-money) volatility for a given expiry.
    ///
    /// This method returns the volatility at delta = 0.5 (ATM point).
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - ATM volatility
    /// * `Err(MarketDataError)` - If expiry is invalid or out of bounds
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_models::market::surfaces::FxVolatilitySurface;
    ///
    /// let deltas = [0.25_f64, 0.5, 0.75];
    /// let expiries = [0.5, 1.0, 2.0];
    /// let vols = [
    ///     [0.11, 0.10, 0.11],
    ///     [0.12, 0.11, 0.12],
    ///     [0.13, 0.12, 0.13],
    /// ];
    ///
    /// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
    /// let atm_1y = surface.atm_volatility(1.0).unwrap();
    /// assert!((atm_1y - 0.11).abs() < 1e-10);
    /// ```
    pub fn atm_volatility(&self, expiry: T) -> Result<T, MarketDataError> {
        let atm_delta: T = from_f64(0.5);
        self.volatility_by_delta(atm_delta, expiry)
    }

    /// Get volatility by delta and expiry.
    ///
    /// # Arguments
    ///
    /// * `delta` - Delta value (0 < delta < 1)
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Interpolated volatility
    /// * `Err(MarketDataError)` - If parameters are invalid or out of bounds
    pub fn volatility_by_delta(&self, delta: T, expiry: T) -> Result<T, MarketDataError> {
        if delta <= T::zero() || delta >= T::one() {
            return Err(MarketDataError::InvalidStrike {
                strike: delta.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }

        let (d_min, d_max) = self.delta_domain();
        let (t_min, t_max) = self.expiry_domain();

        // Handle extrapolation
        if !self.allow_extrapolation {
            if delta < d_min || delta > d_max {
                return Err(MarketDataError::OutOfBounds {
                    x: delta.to_f64().unwrap_or(0.0),
                    min: d_min.to_f64().unwrap_or(0.0),
                    max: d_max.to_f64().unwrap_or(0.0),
                });
            }
            if expiry < t_min || expiry > t_max {
                return Err(MarketDataError::OutOfBounds {
                    x: expiry.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Use bilinear interpolation
        // Convert Vec<Vec<T>> to &[&[T]] for BilinearInterpolator
        let vol_slices: Vec<&[T]> = self.volatilities.iter().map(|v| v.as_slice()).collect();
        // Grid is stored as volatilities[expiry_idx][delta_idx]
        // BilinearInterpolator expects zs[x_idx][y_idx] = z(xs[x_idx], ys[y_idx])
        // So we pass expiries as x-axis and deltas as y-axis
        let interp =
            BilinearInterpolator::new(&self.expiries, &self.deltas, vol_slices.as_slice())?;

        // Clamp for extrapolation
        let clamped_delta = if delta < d_min {
            d_min
        } else if delta > d_max {
            d_max
        } else {
            delta
        };
        let clamped_expiry = if expiry < t_min {
            t_min
        } else if expiry > t_max {
            t_max
        } else {
            expiry
        };

        // Note: BilinearInterpolator was constructed with (expiries, deltas, ...)
        // so we call interpolate(expiry, delta)
        interp
            .interpolate(clamped_expiry, clamped_delta)
            .map_err(MarketDataError::from)
    }

    /// Get the 25-delta risk reversal for a given expiry.
    ///
    /// Risk reversal = σ(25D Call) - σ(25D Put)
    ///
    /// This measures the skew of the volatility smile.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - 25-delta risk reversal
    /// * `Err(MarketDataError)` - If calculation fails
    pub fn risk_reversal_25d(&self, expiry: T) -> Result<T, MarketDataError> {
        let call_25d: T = from_f64(0.75);
        let put_25d: T = from_f64(0.25);

        let vol_call = self.volatility_by_delta(call_25d, expiry)?;
        let vol_put = self.volatility_by_delta(put_25d, expiry)?;

        Ok(vol_call - vol_put)
    }

    /// Get the 25-delta butterfly for a given expiry.
    ///
    /// Butterfly = (σ(25D Call) + σ(25D Put)) / 2 - σ(ATM)
    ///
    /// This measures the curvature of the volatility smile.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - 25-delta butterfly
    /// * `Err(MarketDataError)` - If calculation fails
    pub fn butterfly_25d(&self, expiry: T) -> Result<T, MarketDataError> {
        let call_25d: T = from_f64(0.75);
        let put_25d: T = from_f64(0.25);
        let atm: T = from_f64(0.5);

        let vol_call = self.volatility_by_delta(call_25d, expiry)?;
        let vol_put = self.volatility_by_delta(put_25d, expiry)?;
        let vol_atm = self.volatility_by_delta(atm, expiry)?;

        let two: T = from_f64(2.0);
        Ok((vol_call + vol_put) / two - vol_atm)
    }
}

impl<T: Float> VolatilitySurface<T> for FxVolatilitySurface<T> {
    /// Return the implied volatility for given strike (interpreted as delta)
    /// and expiry.
    ///
    /// Note: For FxVolatilitySurface, the `strike` parameter is interpreted as
    /// delta. For strike-based lookups, use a separate conversion method.
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        // Interpret strike as delta for consistency with FX convention
        self.volatility_by_delta(strike, expiry)
    }

    fn strike_domain(&self) -> (T, T) { self.delta_domain() }

    fn expiry_domain(&self) -> (T, T) { (self.expiries[0], self.expiries[self.expiries.len() - 1]) }
}

// ============================================================================
// VolSurfaceError (consolidated from fx_calibration/surface.rs)
// ============================================================================

/// Errors that can occur during volatility surface operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum VolSurfaceError {
    /// Missing FX curve for delta-strike conversion.
    #[error("Missing FX curve")]
    MissingFxCurve,

    /// Invalid expiry date or time.
    #[error("Invalid expiry: {message}")]
    InvalidExpiry {
        /// Description of the invalid expiry.
        message: String,
    },

    /// Invalid strike or delta value.
    #[error("Invalid strike/delta: {message}")]
    InvalidStrike {
        /// Description of the invalid strike.
        message: String,
    },

    /// Expiry not found in calibrated surface.
    #[error("Expiry not found: {expiry}")]
    ExpiryNotFound {
        /// The missing expiry in year fraction.
        expiry: f64,
    },

    /// Interpolation error.
    #[error("Interpolation error: {message}")]
    InterpolationError {
        /// Description of the interpolation failure.
        message: String,
    },

    /// Calibration error.
    #[error("Calibration error: {message}")]
    CalibrationError {
        /// Description of the calibration failure.
        message: String,
    },

    /// Extrapolation not allowed.
    #[error("Extrapolation not allowed: {t} is outside [{min}, {max}]")]
    ExtrapolationNotAllowed {
        /// The requested point.
        t: f64,
        /// Minimum valid point.
        min: f64,
        /// Maximum valid point.
        max: f64,
    },
}

impl VolSurfaceError {
    /// Creates an invalid expiry error.
    #[must_use]
    pub fn invalid_expiry(message: impl Into<String>) -> Self {
        Self::InvalidExpiry {
            message: message.into(),
        }
    }

    /// Creates an invalid strike error.
    #[must_use]
    pub fn invalid_strike(message: impl Into<String>) -> Self {
        Self::InvalidStrike {
            message: message.into(),
        }
    }

    /// Creates an expiry not found error.
    #[must_use]
    pub fn expiry_not_found(expiry: f64) -> Self { Self::ExpiryNotFound { expiry } }

    /// Creates an interpolation error.
    #[must_use]
    pub fn interpolation_error(message: impl Into<String>) -> Self {
        Self::InterpolationError {
            message: message.into(),
        }
    }

    /// Creates a calibration error.
    #[must_use]
    pub fn calibration_error(message: impl Into<String>) -> Self {
        Self::CalibrationError {
            message: message.into(),
        }
    }

    /// Creates an extrapolation not allowed error.
    #[must_use]
    pub fn extrapolation_not_allowed(t: f64, min: f64, max: f64) -> Self {
        Self::ExtrapolationNotAllowed { t, min, max }
    }
}

// ============================================================================
// SabrParameters (consolidated from fx_calibration/surface.rs)
// ============================================================================

/// SABR model parameters for a single expiry smile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrParameters<T: Float> {
    /// Initial volatility level (alpha).
    pub alpha: T,
    /// Backbone/elasticity parameter (beta, typically fixed).
    pub beta: T,
    /// Correlation between forward and vol (rho).
    pub rho: T,
    /// Vol of vol (nu).
    pub nu: T,
    /// Forward rate at calibration time.
    pub forward: T,
    /// Time to expiry.
    pub expiry: T,
}

impl<T: Float> SabrParameters<T> {
    /// Creates new SABR parameters.
    #[must_use]
    pub fn new(alpha: T, beta: T, rho: T, nu: T, forward: T, expiry: T) -> Self {
        Self {
            alpha,
            beta,
            rho,
            nu,
            forward,
            expiry,
        }
    }

    /// Calculates implied volatility for a given strike using Hagan approximation.
    pub fn implied_vol(&self, strike: T) -> T {
        let f = self.forward;
        let k = strike;
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;
        let t = self.expiry;

        let one = T::one();
        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);
        let four = from_f64::<T>(4.0);
        let twenty_four = from_f64::<T>(24.0);

        // Handle ATM case
        if (f - k).abs() < from_f64::<T>(1e-10) {
            let fk_mid = f.powf(one - beta);
            let term1 = ((one - beta).powi(2) / twenty_four) * alpha.powi(2) / fk_mid.powi(2);
            let term2 = (rho * beta * nu * alpha) / (four * fk_mid);
            let term3 = ((two - three * rho.powi(2)) / twenty_four) * nu.powi(2);
            return (alpha / fk_mid) * (one + (term1 + term2 + term3) * t);
        }

        // Non-ATM case using Hagan approximation
        let fk = (f * k).powf((one - beta) / two);
        let log_fk = (f / k).ln();
        let log_fk_sq = log_fk * log_fk;

        let z = (nu / alpha) * fk * log_fk;
        let sqrt_z = (one - two * rho * z + z * z).sqrt();
        let x_z = ((sqrt_z + z - rho) / (one - rho)).ln();

        let denom_factor = one - beta;
        let denom = fk
            * (one
                + denom_factor.powi(2) * log_fk_sq / twenty_four
                + denom_factor.powi(4) * log_fk_sq * log_fk_sq / from_f64::<T>(1920.0));

        let fk_beta = fk.powi(2);
        let term1 = denom_factor.powi(2) * alpha.powi(2) / (twenty_four * fk_beta);
        let term2 = (rho * beta * nu * alpha) / (four * fk);
        let term3 = ((two - three * rho.powi(2)) / twenty_four) * nu.powi(2);

        let vol = (alpha / denom) * (z / x_z) * (one + (term1 + term2 + term3) * t);

        if vol <= T::zero() {
            alpha / fk * (one + (term1 + term2 + term3) * t)
        } else {
            vol
        }
    }

    /// Validates SABR parameters are within acceptable bounds.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let neg_one = -T::one();
        self.alpha > T::zero()
            && self.beta >= T::zero()
            && self.beta <= T::one()
            && self.rho > neg_one
            && self.rho < T::one()
            && self.nu >= T::zero()
            && self.forward > T::zero()
            && self.expiry > T::zero()
    }
}

// ============================================================================
// CalibratedSmile (consolidated from fx_calibration/surface.rs)
// ============================================================================

/// Per-expiry calibrated volatility smile.
#[derive(Debug, Clone)]
pub struct CalibratedSmile<T: Float> {
    /// Expiry date.
    pub expiry_date: NaiveDate,
    /// Time to expiry in years.
    pub expiry_time: T,
    /// ATM volatility.
    pub atm_vol: T,
    /// Forward rate at this expiry.
    pub forward: T,
    /// SABR parameters if SABR interpolation is used.
    pub sabr_params: Option<SabrParameters<T>>,
    /// Interpolation method used.
    pub interpolation_method: InterpolationMethod,
}

impl<T: Float> CalibratedSmile<T> {
    /// Creates a new calibrated smile with flat vol (no smile).
    #[must_use]
    pub fn flat(expiry_date: NaiveDate, expiry_time: T, atm_vol: T, forward: T) -> Self {
        Self {
            expiry_date,
            expiry_time,
            atm_vol,
            forward,
            sabr_params: None,
            interpolation_method: InterpolationMethod::FlatVol,
        }
    }

    /// Creates a new calibrated smile with SABR parameters.
    #[must_use]
    pub fn sabr(
        expiry_date: NaiveDate,
        expiry_time: T,
        atm_vol: T,
        forward: T,
        sabr_params: SabrParameters<T>,
    ) -> Self {
        Self {
            expiry_date,
            expiry_time,
            atm_vol,
            forward,
            sabr_params: Some(sabr_params),
            interpolation_method: InterpolationMethod::Sabr,
        }
    }

    /// Returns volatility at a given strike.
    pub fn vol_at_strike(&self, strike: T) -> Result<T, VolSurfaceError> {
        if strike <= T::zero() {
            return Err(VolSurfaceError::invalid_strike("Strike must be positive"));
        }
        match &self.sabr_params {
            Some(params) => Ok(params.implied_vol(strike)),
            None => Ok(self.atm_vol),
        }
    }

    /// Returns volatility at a given delta.
    pub fn vol_at_delta(&self, delta: T) -> Result<T, VolSurfaceError> {
        if delta <= T::zero() || delta >= T::one() {
            return Err(VolSurfaceError::invalid_strike("Delta must be in (0, 1)"));
        }
        if self.sabr_params.is_none() {
            return Ok(self.atm_vol);
        }
        let strike = self.delta_to_strike(delta);
        self.vol_at_strike(strike)
    }

    /// Converts delta to strike using Newton-Raphson iteration.
    fn delta_to_strike(&self, delta: T) -> T {
        let f = self.forward;
        let t = self.expiry_time;
        let atm = self.atm_vol;
        let sqrt_t = t.sqrt();
        let delta_f64 = delta.to_f64().unwrap_or(0.5);
        let n_inv = approximate_norm_inv(delta_f64);
        let n_inv_t = from_f64::<T>(n_inv);
        f * (-(atm * sqrt_t * n_inv_t)).exp()
    }
}

/// Approximate inverse normal CDF for delta-to-strike conversion.
fn approximate_norm_inv(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    let t = if p < 0.5 {
        (-2.0 * p.ln()).sqrt()
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;
    let result = t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);
    if p < 0.5 { -result } else { result }
}

// ============================================================================
// VolSmile (consolidated from fx_calibration/surface.rs)
// ============================================================================

/// Extracted volatility smile for a single expiry.
#[derive(Debug, Clone)]
pub struct VolSmile<T: Float> {
    /// Time to expiry in years.
    pub expiry: T,
    /// Forward rate.
    pub forward: T,
    /// Delta values (typically 0.1, 0.25, 0.5, 0.75, 0.9).
    pub deltas: Vec<T>,
    /// Corresponding volatilities.
    pub vols: Vec<T>,
    /// ATM volatility.
    pub atm_vol: T,
}

impl<T: Float> VolSmile<T> {
    /// Creates a new volatility smile.
    #[must_use]
    pub fn new(expiry: T, forward: T, deltas: Vec<T>, vols: Vec<T>, atm_vol: T) -> Self {
        Self { expiry, forward, deltas, vols, atm_vol }
    }

    /// Calculates the 25-delta risk reversal (RR).
    pub fn risk_reversal_25d(&self) -> Option<T> {
        let delta_25p = from_f64::<T>(0.25);
        let delta_75c = from_f64::<T>(0.75);
        let vol_25p = self.vol_at_delta(delta_25p)?;
        let vol_25c = self.vol_at_delta(delta_75c)?;
        Some(vol_25c - vol_25p)
    }

    /// Calculates the 25-delta butterfly (BF).
    pub fn butterfly_25d(&self) -> Option<T> {
        let delta_25p = from_f64::<T>(0.25);
        let delta_75c = from_f64::<T>(0.75);
        let vol_25p = self.vol_at_delta(delta_25p)?;
        let vol_25c = self.vol_at_delta(delta_75c)?;
        let two = from_f64::<T>(2.0);
        Some((vol_25c + vol_25p) / two - self.atm_vol)
    }

    fn vol_at_delta(&self, delta: T) -> Option<T> {
        for i in 0..self.deltas.len().saturating_sub(1) {
            if self.deltas[i] <= delta && delta <= self.deltas[i + 1] {
                let t = (delta - self.deltas[i]) / (self.deltas[i + 1] - self.deltas[i]);
                return Some(self.vols[i] + t * (self.vols[i + 1] - self.vols[i]));
            }
        }
        None
    }
}

// ============================================================================
// CalibratedFxVolSurface (consolidated from fx_calibration/surface.rs)
// ============================================================================

/// Calibrated FX Volatility Surface.
///
/// Stores calibrated smiles at pillar expiry dates with interpolation
/// between expiries. Supports both strike-based and delta-based vol queries.
#[derive(Clone)]
pub struct CalibratedFxVolSurface<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Reference/valuation date.
    reference_date: NaiveDate,
    /// Calibrated smiles by expiry date.
    smiles: BTreeMap<NaiveDate, CalibratedSmile<T>>,
    /// Smile times in years (sorted).
    smile_times: Vec<T>,
    /// FX forward curve for delta-strike conversion.
    #[allow(dead_code)]
    fx_curve: Arc<dyn FxCurve<T> + Send + Sync>,
    /// Surface configuration.
    config: FxVolSurfaceConfig,
}

#[allow(clippy::missing_fields_in_debug)]
impl<T: Float + std::fmt::Debug> std::fmt::Debug for CalibratedFxVolSurface<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalibratedFxVolSurface")
            .field("currency_pair", &self.currency_pair)
            .field("reference_date", &self.reference_date)
            .field("num_expiries", &self.smiles.len())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<T: Float + Send + Sync> CalibratedFxVolSurface<T> {
    /// Creates a new calibrated FX volatility surface.
    pub fn new(
        currency_pair: CurrencyPair,
        reference_date: NaiveDate,
        smiles: BTreeMap<NaiveDate, CalibratedSmile<T>>,
        fx_curve: Arc<dyn FxCurve<T> + Send + Sync>,
        config: FxVolSurfaceConfig,
    ) -> Self {
        let smile_times: Vec<T> = smiles.values().map(|s| s.expiry_time).collect();
        Self { currency_pair, reference_date, smiles, smile_times, fx_curve, config }
    }

    /// Returns the currency pair.
    #[inline]
    #[must_use]
    pub fn currency_pair(&self) -> CurrencyPair { self.currency_pair }

    /// Returns the reference date.
    #[inline]
    #[must_use]
    pub fn reference_date(&self) -> NaiveDate { self.reference_date }

    /// Returns the number of calibrated expiries.
    #[inline]
    #[must_use]
    pub fn num_expiries(&self) -> usize { self.smiles.len() }

    /// Returns the configuration.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &FxVolSurfaceConfig { &self.config }

    /// Returns the expiry dates.
    #[must_use]
    pub fn expiry_dates(&self) -> Vec<NaiveDate> { self.smiles.keys().copied().collect() }

    /// Returns a reference to the calibrated smiles by expiry date.
    #[must_use]
    pub fn smiles(&self) -> &BTreeMap<NaiveDate, CalibratedSmile<T>> { &self.smiles }

    /// Returns volatility at a given strike and expiry.
    pub fn vol(&self, strike: Strike, expiry: T) -> Result<T, VolSurfaceError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 <= 0.0 {
            return Err(VolSurfaceError::invalid_expiry("Expiry must be positive"));
        }
        let smile = self.get_interpolated_smile(expiry)?;
        let strike_t = T::from(strike.value()).unwrap();
        smile.vol_at_strike(strike_t)
    }

    /// Returns volatility at a given expiry and delta.
    pub fn vol_by_delta(&self, expiry: T, delta: T) -> Result<T, VolSurfaceError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        if expiry_f64 <= 0.0 {
            return Err(VolSurfaceError::invalid_expiry("Expiry must be positive"));
        }
        if delta <= T::zero() || delta >= T::one() {
            return Err(VolSurfaceError::invalid_strike("Delta must be in (0, 1)"));
        }
        let smile = self.get_interpolated_smile(expiry)?;
        smile.vol_at_delta(delta)
    }

    /// Extracts the volatility smile for a specific expiry.
    pub fn smile(&self, expiry: T) -> Result<VolSmile<T>, VolSurfaceError> {
        let smile = self.get_interpolated_smile(expiry)?;
        let deltas: Vec<T> = vec![
            from_f64(0.10), from_f64(0.25), from_f64(0.50), from_f64(0.75), from_f64(0.90),
        ];
        let mut vols = Vec::with_capacity(deltas.len());
        for &d in &deltas {
            vols.push(smile.vol_at_delta(d)?);
        }
        Ok(VolSmile::new(smile.expiry_time, smile.forward, deltas, vols, smile.atm_vol))
    }

    /// Returns the ATM volatility at a given expiry.
    pub fn atm_vol(&self, expiry: T) -> Result<T, VolSurfaceError> {
        let smile = self.get_interpolated_smile(expiry)?;
        Ok(smile.atm_vol)
    }

    /// Gets or interpolates the smile at a given expiry time.
    fn get_interpolated_smile(&self, expiry: T) -> Result<CalibratedSmile<T>, VolSurfaceError> {
        if self.smiles.is_empty() {
            return Err(VolSurfaceError::interpolation_error("No calibrated smiles"));
        }
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        let first_time = self.smile_times.first().map(|t| t.to_f64().unwrap_or(0.0));
        let last_time = self.smile_times.last().map(|t| t.to_f64().unwrap_or(0.0));

        if let (Some(min_t), Some(max_t)) = (first_time, last_time) {
            if expiry_f64 < min_t || expiry_f64 > max_t {
                if !self.config.allow_extrapolation {
                    return Err(VolSurfaceError::extrapolation_not_allowed(expiry_f64, min_t, max_t));
                }
                if expiry_f64 < min_t {
                    return Ok(self.smiles.values().next().unwrap().clone());
                } else {
                    return Ok(self.smiles.values().last().unwrap().clone());
                }
            }
        }

        let smiles_vec: Vec<&CalibratedSmile<T>> = self.smiles.values().collect();
        for smile in &smiles_vec {
            if (smile.expiry_time.to_f64().unwrap_or(0.0) - expiry_f64).abs() < 1e-10 {
                return Ok((*smile).clone());
            }
        }

        for i in 0..smiles_vec.len().saturating_sub(1) {
            let t1 = smiles_vec[i].expiry_time.to_f64().unwrap_or(0.0);
            let t2 = smiles_vec[i + 1].expiry_time.to_f64().unwrap_or(0.0);
            if t1 <= expiry_f64 && expiry_f64 <= t2 {
                let w = (expiry_f64 - t1) / (t2 - t1);
                let w_t = from_f64::<T>(w);
                let atm1 = smiles_vec[i].atm_vol;
                let atm2 = smiles_vec[i + 1].atm_vol;
                let atm_interp = atm1 + w_t * (atm2 - atm1);
                let fwd1 = smiles_vec[i].forward;
                let fwd2 = smiles_vec[i + 1].forward;
                let fwd_interp = fwd1 + w_t * (fwd2 - fwd1);
                let interpolated = CalibratedSmile::flat(
                    smiles_vec[i].expiry_date, expiry, atm_interp, fwd_interp,
                );
                return Ok(interpolated);
            }
        }
        Err(VolSurfaceError::expiry_not_found(expiry_f64))
    }
}

impl<T: Float + Send + Sync> VolatilitySurface<T> for CalibratedFxVolSurface<T> {
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        let expiry_f64 = expiry.to_f64().unwrap_or(0.0);
        let strike_f64 = strike.to_f64().unwrap_or(0.0);
        if expiry_f64 <= 0.0 {
            return Err(MarketDataError::InvalidExpiry { expiry: expiry_f64 });
        }
        if strike_f64 <= 0.0 {
            return Err(MarketDataError::InvalidStrike { strike: strike_f64 });
        }
        let smile = self.get_interpolated_smile(expiry).map_err(|e| {
            MarketDataError::InterpolationFailed { reason: e.to_string() }
        })?;
        smile.vol_at_strike(strike).map_err(|e| {
            MarketDataError::InterpolationFailed { reason: e.to_string() }
        })
    }

    fn strike_domain(&self) -> (T, T) { (from_f64(0.01), from_f64(100.0)) }

    fn expiry_domain(&self) -> (T, T) {
        if self.smile_times.is_empty() {
            return (T::zero(), T::zero());
        }
        (*self.smile_times.first().unwrap(), *self.smile_times.last().unwrap())
    }
}

impl<T: Float + Send + Sync> pricer_core::traits::priceable::Differentiable
    for CalibratedFxVolSurface<T>
{
}

// ============================================================================
// CalibrationError (consolidated from fx_calibration/vol_builder.rs)
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
        Self::SabrCalibrationFailed { expiry, message: message.into() }
    }
    /// Creates an invalid quote error.
    #[must_use]
    pub fn invalid_quote(message: impl Into<String>) -> Self {
        Self::InvalidQuote { message: message.into() }
    }
    /// Creates a surface construction error.
    #[must_use]
    pub fn surface_construction_error(message: impl Into<String>) -> Self {
        Self::SurfaceConstructionError { message: message.into() }
    }
}

impl From<VolSurfaceError> for CalibrationError {
    fn from(err: VolSurfaceError) -> Self { Self::surface_construction_error(err.to_string()) }
}

// ============================================================================
// CalibrationDiagnostics (consolidated from fx_calibration/vol_builder.rs)
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
        self.by_expiry.iter().map(|d| d.residual).max_by(|a, b| a.partial_cmp(b).unwrap())
    }
    /// Returns whether all expiries converged.
    #[must_use]
    pub fn all_converged(&self) -> bool { self.by_expiry.iter().all(|d| d.converged) }
}

// ============================================================================
// VolQuote (consolidated from fx_calibration/vol_builder.rs)
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
        Self { expiry, quote_type: VolQuoteType::Atm, value: vol, delta: None }
    }
    /// Creates a 25-delta butterfly quote.
    #[must_use]
    pub fn butterfly_25d(expiry: NaiveDate, spread: T) -> Self {
        Self { expiry, quote_type: VolQuoteType::Butterfly25D, value: spread, delta: Some(from_f64(0.25)) }
    }
    /// Creates a 25-delta risk reversal quote.
    #[must_use]
    pub fn risk_reversal_25d(expiry: NaiveDate, spread: T) -> Self {
        Self { expiry, quote_type: VolQuoteType::RiskReversal25D, value: spread, delta: Some(from_f64(0.25)) }
    }
    /// Creates a 10-delta butterfly quote.
    #[must_use]
    pub fn butterfly_10d(expiry: NaiveDate, spread: T) -> Self {
        Self { expiry, quote_type: VolQuoteType::Butterfly10D, value: spread, delta: Some(from_f64(0.10)) }
    }
    /// Creates a 10-delta risk reversal quote.
    #[must_use]
    pub fn risk_reversal_10d(expiry: NaiveDate, spread: T) -> Self {
        Self { expiry, quote_type: VolQuoteType::RiskReversal10D, value: spread, delta: Some(from_f64(0.10)) }
    }
}

// ============================================================================
// FxVolSurfaceBuilder (consolidated from fx_calibration/vol_builder.rs)
// ============================================================================

/// Builder for constructing calibrated FX volatility surfaces.
pub struct FxVolSurfaceBuilder<T: Float> {
    currency_pair: CurrencyPair,
    reference_date: Option<NaiveDate>,
    fx_curve: Option<Arc<dyn FxCurve<T> + Send + Sync>>,
    config: FxVolSurfaceConfig,
    quotes: Vec<VolQuote<T>>,
    enable_sabr: bool,
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
        self.quotes.push(VolQuote::risk_reversal_25d(expiry, spread));
        self
    }

    /// Adds multiple quotes.
    #[must_use]
    pub fn add_quotes(mut self, quotes: Vec<VolQuote<T>>) -> Self {
        self.quotes.extend(quotes);
        self
    }

    /// Builds the calibrated volatility surface.
    pub fn build(self) -> Result<(CalibratedFxVolSurface<T>, CalibrationDiagnostics), CalibrationError> {
        let reference_date = self.reference_date.ok_or(CalibrationError::MissingReferenceDate)?;
        let fx_curve = self.fx_curve.clone().ok_or(CalibrationError::MissingFxCurve)?;
        if self.quotes.is_empty() {
            return Err(CalibrationError::NoInstruments);
        }
        let quotes_by_expiry = self.group_quotes_by_expiry();
        let mut smiles = BTreeMap::new();
        let mut diagnostics = CalibrationDiagnostics::new();
        for (expiry, quotes) in quotes_by_expiry {
            let (smile, diag) = self.calibrate_expiry(expiry, &quotes, reference_date, &fx_curve)?;
            smiles.insert(expiry, smile);
            diagnostics.add_expiry(diag);
        }
        diagnostics.success = diagnostics.all_converged();
        let surface = CalibratedFxVolSurface::new(
            self.currency_pair, reference_date, smiles, fx_curve, self.config,
        );
        Ok((surface, diagnostics))
    }

    fn group_quotes_by_expiry(&self) -> BTreeMap<NaiveDate, Vec<&VolQuote<T>>> {
        let mut by_expiry: BTreeMap<NaiveDate, Vec<&VolQuote<T>>> = BTreeMap::new();
        for quote in &self.quotes {
            by_expiry.entry(quote.expiry).or_default().push(quote);
        }
        by_expiry
    }

    fn calibrate_expiry(
        &self,
        expiry: NaiveDate,
        quotes: &[&VolQuote<T>],
        reference_date: NaiveDate,
        fx_curve: &Arc<dyn FxCurve<T> + Send + Sync>,
    ) -> Result<(CalibratedSmile<T>, ExpiryDiagnostics), CalibrationError> {
        let atm_quote = quotes.iter().find(|q| q.quote_type == VolQuoteType::Atm);
        let atm_vol = match atm_quote {
            Some(q) => q.value,
            None => return Err(CalibrationError::InsufficientInstruments { expiry, got: quotes.len(), need: 1 }),
        };
        let days = (expiry - reference_date).num_days() as f64;
        let expiry_time: T = from_f64(days / 365.0);
        if expiry_time <= T::zero() {
            return Err(CalibrationError::invalid_quote(format!(
                "Expiry {} is not after reference date {}", expiry, reference_date
            )));
        }
        let forward = fx_curve.forward_rate(expiry_time)
            .map_err(|e| CalibrationError::surface_construction_error(e.to_string()))?;
        let (smile, diag) = if self.enable_sabr && quotes.len() >= 3 {
            self.calibrate_sabr_smile(expiry, expiry_time, atm_vol, forward, quotes)
        } else {
            let smile = CalibratedSmile::flat(expiry, expiry_time, atm_vol, forward);
            let diag = ExpiryDiagnostics {
                expiry, iterations: 0, residual: 0.0, converged: true, instrument_errors: vec![0.0],
            };
            (smile, diag)
        };
        Ok((smile, diag))
    }

    fn calibrate_sabr_smile(
        &self, expiry: NaiveDate, expiry_time: T, atm_vol: T, forward: T, quotes: &[&VolQuote<T>],
    ) -> (CalibratedSmile<T>, ExpiryDiagnostics) {
        let beta = self.sabr_beta.unwrap_or(from_f64(0.5));
        let bf_25d = quotes.iter().find(|q| q.quote_type == VolQuoteType::Butterfly25D);
        let rr_25d = quotes.iter().find(|q| q.quote_type == VolQuoteType::RiskReversal25D);
        let f_beta = forward.powf(T::one() - beta);
        let alpha_init = atm_vol * f_beta;
        let (rho, nu) = if let (Some(bf), Some(rr)) = (bf_25d, rr_25d) {
            let rr_val = rr.value.to_f64().unwrap_or(0.0);
            let bf_val = bf.value.to_f64().unwrap_or(0.0);
            let nu_approx = (bf_val.abs() * 100.0).sqrt().clamp(0.1, 2.0);
            let rho_approx = (rr_val * 10.0).clamp(-0.9, 0.9);
            (from_f64::<T>(rho_approx), from_f64::<T>(nu_approx))
        } else {
            (from_f64::<T>(-0.2), from_f64::<T>(0.4))
        };
        let sabr_params = SabrParameters::new(alpha_init, beta, rho, nu, forward, expiry_time);
        let smile = CalibratedSmile::sabr(expiry, expiry_time, atm_vol, forward, sabr_params);
        let mut errors = Vec::new();
        for quote in quotes {
            let model_vol = match quote.quote_type {
                VolQuoteType::Atm => atm_vol,
                _ => atm_vol,
            };
            let error = (model_vol.to_f64().unwrap_or(0.0) - quote.value.to_f64().unwrap_or(0.0)).abs();
            errors.push(error);
        }
        let residual = errors.iter().sum::<f64>() / errors.len().max(1) as f64;
        let diag = ExpiryDiagnostics {
            expiry, iterations: 1, residual, converged: residual < 0.001, instrument_errors: errors,
        };
        (smile, diag)
    }
}

// ============================================================================
// CacheStats (consolidated from fx_calibration/lazy_surface.rs)
// ============================================================================

/// Statistics for cache usage.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: usize,
    /// Number of cache misses (including first access).
    pub misses: usize,
    /// Number of explicit invalidations.
    pub invalidations: usize,
}

impl CacheStats {
    /// Creates new empty cache statistics.
    pub fn new() -> Self { Self::default() }
    /// Returns the hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

// ============================================================================
// LazyFxVolSurface (consolidated from fx_calibration/lazy_surface.rs)
// ============================================================================

/// State of the lazy surface.
enum LazyState<T: Float> {
    Pending(FxVolSurfaceBuilder<T>),
    Calibrated { surface: CalibratedFxVolSurface<T>, diagnostics: CalibrationDiagnostics },
    Failed(CalibrationError),
}

struct LazyInner<T: Float> {
    state: LazyState<T>,
    stats: CacheStats,
}

/// Lazy FX volatility surface with deferred calibration.
pub struct LazyFxVolSurface<T: Float> {
    inner: Arc<RwLock<LazyInner<T>>>,
}

impl<T: Float + Send + Sync + 'static> LazyFxVolSurface<T> {
    /// Creates a new lazy surface from a builder.
    pub fn new(builder: FxVolSurfaceBuilder<T>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LazyInner {
                state: LazyState::Pending(builder),
                stats: CacheStats::new(),
            })),
        }
    }

    /// Returns whether the surface has been calibrated.
    pub fn is_calibrated(&self) -> bool {
        let inner = self.inner.read().expect("Lock poisoned");
        matches!(inner.state, LazyState::Calibrated { .. })
    }

    /// Returns whether calibration has failed.
    pub fn has_failed(&self) -> bool {
        let inner = self.inner.read().expect("Lock poisoned");
        matches!(inner.state, LazyState::Failed(_))
    }

    /// Returns a clone of the cache statistics.
    pub fn cache_stats(&self) -> CacheStats {
        let inner = self.inner.read().expect("Lock poisoned");
        inner.stats.clone()
    }

    /// Returns calibration diagnostics if calibration has completed.
    pub fn diagnostics(&self) -> Option<CalibrationDiagnostics> {
        let inner = self.inner.read().expect("Lock poisoned");
        if let LazyState::Calibrated { diagnostics, .. } = &inner.state {
            Some(diagnostics.clone())
        } else {
            None
        }
    }

    /// Forces calibration and returns the result.
    pub fn force_calibrate(&self) -> Result<(), CalibrationError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)?;
        Ok(())
    }

    /// Invalidates the cache and resets to pending state.
    pub fn invalidate(&self, new_builder: Option<FxVolSurfaceBuilder<T>>) {
        let mut inner = self.inner.write().expect("Lock poisoned");
        inner.stats.invalidations += 1;
        if let Some(builder) = new_builder {
            inner.state = LazyState::Pending(builder);
        } else {
            inner.state = LazyState::Failed(CalibrationError::NoInstruments);
        }
    }

    /// Invalidates the cache with a new builder.
    pub fn invalidate_with_builder(&self, builder: FxVolSurfaceBuilder<T>) {
        self.invalidate(Some(builder));
    }

    /// Gets the configuration if available.
    pub fn config(&self) -> Option<FxVolSurfaceConfig> {
        let inner = self.inner.read().expect("Lock poisoned");
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            Some(surface.config().clone())
        } else {
            None
        }
    }

    /// Queries volatility by strike and expiry.
    pub fn volatility(&self, strike: T, expiry: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::calibration_error(e.to_string()))?;
        inner.stats.hits += 1;
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            surface.volatility(strike, expiry)
                .map_err(|e| VolSurfaceError::interpolation_error(e.to_string()))
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Queries volatility by expiry and delta.
    pub fn vol_by_delta(&self, expiry: T, delta: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::calibration_error(e.to_string()))?;
        inner.stats.hits += 1;
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            surface.vol_by_delta(expiry, delta)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Extracts a smile at a given expiry.
    pub fn smile(&self, expiry: T) -> Result<VolSmile<T>, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::calibration_error(e.to_string()))?;
        inner.stats.hits += 1;
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            surface.smile(expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Returns the ATM volatility at a given expiry.
    pub fn atm_vol(&self, expiry: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::calibration_error(e.to_string()))?;
        inner.stats.hits += 1;
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            surface.atm_vol(expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    fn ensure_calibrated(&self, inner: &mut LazyInner<T>) -> Result<(), CalibrationError> {
        match &inner.state {
            LazyState::Calibrated { .. } => Ok(()),
            LazyState::Failed(e) => Err(e.clone()),
            LazyState::Pending(_) => {
                let state = std::mem::replace(&mut inner.state, LazyState::Failed(CalibrationError::NoInstruments));
                if let LazyState::Pending(builder) = state {
                    inner.stats.misses += 1;
                    match builder.build() {
                        Ok((surface, diagnostics)) => {
                            inner.state = LazyState::Calibrated { surface, diagnostics };
                            Ok(())
                        }
                        Err(e) => {
                            inner.state = LazyState::Failed(e.clone());
                            Err(e)
                        }
                    }
                } else {
                    unreachable!("State was Pending")
                }
            }
        }
    }
}

impl<T: Float + Send + Sync + 'static> Clone for LazyFxVolSurface<T> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

// ============================================================================
// DeltaType (consolidated from fx_density.rs)
// ============================================================================

/// Delta convention type for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeltaType {
    /// Spot delta (premium excluded).
    #[default]
    SpotDelta,
    /// Forward delta.
    ForwardDelta,
    /// Premium-adjusted delta.
    PremiumAdjusted,
}

// ============================================================================
// DensityStatistics (consolidated from fx_density.rs)
// ============================================================================

/// Statistics of the probability density function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityStatistics<T: Float> {
    /// Expected value (first moment).
    pub mean: T,
    /// Variance (second central moment).
    pub variance: T,
    /// Skewness (third standardised moment).
    pub skewness: T,
    /// Kurtosis (fourth standardised moment, excess).
    pub kurtosis: T,
}

impl<T: Float> Default for DensityStatistics<T> {
    fn default() -> Self {
        Self { mean: T::zero(), variance: T::zero(), skewness: T::zero(), kurtosis: T::zero() }
    }
}

// ============================================================================
// FxDensityCalculator (consolidated from fx_density.rs)
// ============================================================================

/// FX probability density calculator.
#[derive(Debug, Clone)]
pub struct FxDensityCalculator<'a, T: Float> {
    surface: &'a FxVolatilitySurface<T>,
    spot: T,
    domestic_rate: T,
    foreign_rate: T,
}

impl<'a, T: Float> FxDensityCalculator<'a, T> {
    /// Create a new FX density calculator.
    pub fn new(surface: &'a FxVolatilitySurface<T>, spot: T, domestic_rate: T, foreign_rate: T) -> Self {
        Self { surface, spot, domestic_rate, foreign_rate }
    }

    /// Convert delta to strike using Garman-Kohlhagen inverse calculation.
    pub fn delta_to_strike(
        &self, delta: T, expiry: T, volatility: T, delta_type: DeltaType,
    ) -> Result<T, MarketDataError> {
        let delta_abs = delta.abs();
        if delta_abs <= T::zero() || delta_abs >= T::one() {
            return Err(MarketDataError::InvalidStrike { strike: delta.to_f64().unwrap_or(0.0) });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry { expiry: expiry.to_f64().unwrap_or(0.0) });
        }
        if volatility <= T::zero() {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!("Volatility must be positive, got {}", volatility.to_f64().unwrap_or(0.0)),
            });
        }
        if self.spot <= T::zero() {
            return Err(MarketDataError::InvalidStrike { strike: self.spot.to_f64().unwrap_or(0.0) });
        }
        let is_call = delta > T::zero();
        let forward = self.spot * ((self.domestic_rate - self.foreign_rate) * expiry).exp();
        self.bisection_delta_to_strike(forward, expiry, volatility, delta, delta_type, is_call)
    }

    fn compute_delta(&self, strike: T, expiry: T, volatility: T, delta_type: DeltaType, is_call: bool) -> T {
        let d1 = self.compute_d1(strike, expiry, volatility);
        let discount_foreign = (-self.foreign_rate * expiry).exp();
        let forward = self.spot * ((self.domestic_rate - self.foreign_rate) * expiry).exp();
        match delta_type {
            DeltaType::SpotDelta => {
                if is_call { discount_foreign * norm_cdf(d1) } else { -discount_foreign * norm_cdf(-d1) }
            }
            DeltaType::ForwardDelta => {
                if is_call { norm_cdf(d1) } else { norm_cdf(d1) - T::one() }
            }
            DeltaType::PremiumAdjusted => {
                if is_call { discount_foreign * norm_cdf(d1) * strike / forward }
                else { -discount_foreign * norm_cdf(-d1) * strike / forward }
            }
        }
    }

    fn bisection_delta_to_strike(
        &self, forward: T, expiry: T, volatility: T, target_delta: T,
        delta_type: DeltaType, is_call: bool,
    ) -> Result<T, MarketDataError> {
        let vol_factor = (volatility * expiry.sqrt() * from_f64::<T>(3.0)).exp();
        let mut k_low = forward / vol_factor;
        let mut k_high = forward * vol_factor;
        let f_low = self.compute_delta(k_low, expiry, volatility, delta_type, is_call) - target_delta;
        let f_high = self.compute_delta(k_high, expiry, volatility, delta_type, is_call) - target_delta;
        if f_low * f_high > T::zero() {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!("No valid bracket for delta {}", target_delta.to_f64().unwrap_or(0.0)),
            });
        }
        let tolerance: T = from_f64(1e-10);
        let max_iterations = 100;
        let mut sign_low = if f_low < T::zero() { -1i8 } else { 1i8 };
        for _ in 0..max_iterations {
            let k_mid = (k_low + k_high) / from_f64::<T>(2.0);
            let f_mid = self.compute_delta(k_mid, expiry, volatility, delta_type, is_call) - target_delta;
            if f_mid.abs() < tolerance || (k_high - k_low) / from_f64::<T>(2.0) < tolerance {
                return Ok(k_mid);
            }
            let sign_mid = if f_mid < T::zero() { -1i8 } else { 1i8 };
            if sign_low != sign_mid { k_high = k_mid; } else { k_low = k_mid; sign_low = sign_mid; }
        }
        Ok((k_low + k_high) / from_f64::<T>(2.0))
    }

    fn compute_d1(&self, strike: T, expiry: T, volatility: T) -> T {
        if strike <= T::zero() { return from_f64(100.0); }
        let sqrt_t = expiry.sqrt();
        let half: T = from_f64(0.5);
        let numerator = (self.spot / strike).ln()
            + (self.domestic_rate - self.foreign_rate + half * volatility * volatility) * expiry;
        numerator / (volatility * sqrt_t)
    }

    /// Compute the risk-neutral probability density at a given strike.
    pub fn probability_density(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        if strike <= T::zero() {
            return Err(MarketDataError::InvalidStrike { strike: strike.to_f64().unwrap_or(0.0) });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry { expiry: expiry.to_f64().unwrap_or(0.0) });
        }
        let volatility = self.volatility_at_strike(strike, expiry)?;
        let h = strike * from_f64::<T>(0.001);
        let k_low = strike - h;
        let k_mid = strike;
        let k_high = strike + h;
        let vol_low = self.volatility_at_strike(k_low, expiry)?;
        let vol_mid = volatility;
        let vol_high = self.volatility_at_strike(k_high, expiry)?;
        let c_low = self.call_price(k_low, expiry, vol_low);
        let c_mid = self.call_price(k_mid, expiry, vol_mid);
        let c_high = self.call_price(k_high, expiry, vol_high);
        let d2c_dk2 = (c_high - from_f64::<T>(2.0) * c_mid + c_low) / (h * h);
        let discount = (self.domestic_rate * expiry).exp();
        let density = discount * d2c_dk2;
        Ok(density.max(T::zero()))
    }

    /// Compute statistics of the probability density function.
    pub fn statistics(&self, expiry: T, strike_range: (T, T), num_points: usize) -> Result<DensityStatistics<T>, MarketDataError> {
        let (k_min, k_max) = strike_range;
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry { expiry: expiry.to_f64().unwrap_or(0.0) });
        }
        if k_min <= T::zero() || k_max <= T::zero() {
            return Err(MarketDataError::InvalidStrike { strike: k_min.min(k_max).to_f64().unwrap_or(0.0) });
        }
        if k_min >= k_max {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!("Strike range invalid: min {} >= max {}", k_min.to_f64().unwrap_or(0.0), k_max.to_f64().unwrap_or(0.0)),
            });
        }
        if num_points < 10 {
            return Err(MarketDataError::InsufficientData { got: num_points, need: 10 });
        }
        let n = num_points;
        let dk = (k_max - k_min) / from_f64::<T>(n as f64);
        let mut strikes = Vec::with_capacity(n + 1);
        let mut densities = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let k = k_min + from_f64::<T>(i as f64) * dk;
            strikes.push(k);
            let density = self.probability_density(k, expiry).unwrap_or(T::zero());
            densities.push(density);
        }
        let mut total_weight = T::zero();
        for i in 0..=n {
            let weight = if i == 0 || i == n { from_f64(0.5) } else { T::one() };
            total_weight = total_weight + weight * densities[i] * dk;
        }
        let mut mean = T::zero();
        for i in 0..=n {
            let weight = if i == 0 || i == n { from_f64(0.5) } else { T::one() };
            mean = mean + weight * strikes[i] * densities[i] * dk;
        }
        if total_weight > T::zero() { mean = mean / total_weight; }
        let mut m2 = T::zero();
        let mut m3 = T::zero();
        let mut m4 = T::zero();
        for i in 0..=n {
            let weight = if i == 0 || i == n { from_f64(0.5) } else { T::one() };
            let deviation = strikes[i] - mean;
            let d2 = deviation * deviation;
            let d3 = d2 * deviation;
            let d4 = d3 * deviation;
            m2 = m2 + weight * d2 * densities[i] * dk;
            m3 = m3 + weight * d3 * densities[i] * dk;
            m4 = m4 + weight * d4 * densities[i] * dk;
        }
        if total_weight > T::zero() { m2 = m2 / total_weight; m3 = m3 / total_weight; m4 = m4 / total_weight; }
        let variance = m2;
        let std_dev = variance.sqrt();
        let std_dev3 = std_dev * std_dev * std_dev;
        let std_dev4 = std_dev3 * std_dev;
        let skewness = if std_dev3 > T::zero() { m3 / std_dev3 } else { T::zero() };
        let kurtosis = if std_dev4 > T::zero() { m4 / std_dev4 - from_f64(3.0) } else { T::zero() };
        Ok(DensityStatistics { mean, variance, skewness, kurtosis })
    }

    fn volatility_at_strike(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        let atm_vol = self.surface.atm_volatility(expiry)?;
        let delta = self.strike_to_delta(strike, expiry, atm_vol);
        let delta_clamped = delta.max(from_f64(0.01)).min(from_f64(0.99));
        self.surface.volatility_by_delta(delta_clamped, expiry)
    }

    fn strike_to_delta(&self, strike: T, expiry: T, volatility: T) -> T {
        let d1 = self.compute_d1(strike, expiry, volatility);
        let discount_foreign = (-self.foreign_rate * expiry).exp();
        discount_foreign * norm_cdf(d1)
    }

    fn call_price(&self, strike: T, expiry: T, volatility: T) -> T {
        let sqrt_t = expiry.sqrt();
        let d1 = self.compute_d1(strike, expiry, volatility);
        let d2 = d1 - volatility * sqrt_t;
        let discount_foreign = (-self.foreign_rate * expiry).exp();
        let discount_domestic = (-self.domestic_rate * expiry).exp();
        self.spot * discount_foreign * norm_cdf(d1) - strike * discount_domestic * norm_cdf(d2)
    }

    /// Get the spot rate.
    #[inline]
    pub fn spot(&self) -> T { self.spot }
    /// Get the domestic rate.
    #[inline]
    pub fn domestic_rate(&self) -> T { self.domestic_rate }
    /// Get the foreign rate.
    #[inline]
    pub fn foreign_rate(&self) -> T { self.foreign_rate }
    /// Get reference to the underlying surface.
    #[inline]
    pub fn surface(&self) -> &FxVolatilitySurface<T> { self.surface }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // FxDeltaPoint Tests
    // ========================================

    #[test]
    fn test_delta_point_as_delta() {
        assert!((FxDeltaPoint::Put10D.as_delta() - 0.1).abs() < 1e-10);
        assert!((FxDeltaPoint::Put25D.as_delta() - 0.25).abs() < 1e-10);
        assert!((FxDeltaPoint::Atm.as_delta() - 0.5).abs() < 1e-10);
        assert!((FxDeltaPoint::Call25D.as_delta() - 0.75).abs() < 1e-10);
        assert!((FxDeltaPoint::Call10D.as_delta() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_delta_point_all() {
        let all = FxDeltaPoint::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], FxDeltaPoint::Put10D);
        assert_eq!(all[4], FxDeltaPoint::Call10D);
    }

    // ========================================
    // FxVolatilitySurface Construction Tests
    // ========================================

    #[test]
    fn test_surface_new_valid() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0, 2.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12], [0.13, 0.12, 0.13]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert_eq!(surface.delta_domain(), (0.25, 0.75));
        assert_eq!(surface.expiry_domain(), (0.5, 2.0));
    }

    #[test]
    fn test_surface_new_insufficient_deltas() {
        let deltas = [0.5_f64];
        let expiries = [0.5, 1.0];
        let vols = [[0.10], [0.11]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_insufficient_expiries() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [1.0];
        let vols = [[0.11, 0.10, 0.11]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_unsorted_deltas() {
        let deltas = [0.5_f64, 0.25, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.11, 0.11], [0.11, 0.12, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_invalid_delta() {
        let deltas = [0.0_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.11, 0.11], [0.11, 0.12, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_negative_expiry() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [-0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_negative_volatility() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, -0.10, 0.11], [0.12, 0.11, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_mismatched_grid() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10], [0.12, 0.11]]; // Wrong number of delta columns

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    // ========================================
    // ATM Volatility Tests
    // ========================================

    #[test]
    fn test_atm_volatility_at_pillar() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0, 2.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12], [0.13, 0.12, 0.13]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let atm_1y = surface.atm_volatility(1.0).unwrap();
        assert!((atm_1y - 0.11).abs() < 1e-10);
    }

    #[test]
    fn test_atm_volatility_interpolated() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.10, 0.10], [0.12, 0.12, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // At 0.75 years, ATM vol should be between 0.10 and 0.12
        let atm = surface.atm_volatility(0.75).unwrap();
        assert!(atm > 0.10 && atm < 0.12);
    }

    // ========================================
    // Volatility by Delta Tests
    // ========================================

    #[test]
    fn test_volatility_by_delta_at_pillar() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.09], [0.12, 0.11, 0.10]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // 25D Put at 1Y
        let vol_25d = surface.volatility_by_delta(0.25, 1.0).unwrap();
        assert!((vol_25d - 0.12).abs() < 1e-10);

        // 75D (25D Call) at 1Y
        let vol_75d = surface.volatility_by_delta(0.75, 1.0).unwrap();
        assert!((vol_75d - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_volatility_by_delta_invalid_delta() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert!(surface.volatility_by_delta(0.0, 1.0).is_err());
        assert!(surface.volatility_by_delta(1.0, 1.0).is_err());
        assert!(surface.volatility_by_delta(-0.5, 1.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_invalid_expiry() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert!(surface.volatility_by_delta(0.5, 0.0).is_err());
        assert!(surface.volatility_by_delta(0.5, -1.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_out_of_bounds_no_extrapolation() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, false).unwrap();

        // Delta out of bounds
        assert!(surface.volatility_by_delta(0.1, 0.75).is_err());
        assert!(surface.volatility_by_delta(0.9, 0.75).is_err());

        // Expiry out of bounds
        assert!(surface.volatility_by_delta(0.5, 0.25).is_err());
        assert!(surface.volatility_by_delta(0.5, 2.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_with_extrapolation() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.09], [0.12, 0.11, 0.10]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // Should succeed with extrapolation
        let _ = surface.volatility_by_delta(0.1, 0.75).unwrap();
        let _ = surface.volatility_by_delta(0.9, 0.75).unwrap();
        let _ = surface.volatility_by_delta(0.5, 0.25).unwrap();
        let _ = surface.volatility_by_delta(0.5, 2.0).unwrap();
    }

    // ========================================
    // Risk Reversal Tests
    // ========================================

    #[test]
    fn test_risk_reversal_25d() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [
            [0.12, 0.10, 0.11], // 25D Put = 0.12, 25D Call = 0.11
            [0.13, 0.11, 0.12],
        ];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // RR = σ(25D Call) - σ(25D Put) = 0.11 - 0.12 = -0.01
        let rr = surface.risk_reversal_25d(0.5).unwrap();
        assert!((rr - (-0.01)).abs() < 1e-10);
    }

    // ========================================
    // Butterfly Tests
    // ========================================

    #[test]
    fn test_butterfly_25d() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [
            [0.12, 0.10, 0.12], // Symmetric smile
            [0.13, 0.11, 0.13],
        ];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // BF = (σ(25D Call) + σ(25D Put)) / 2 - σ(ATM) = (0.12 + 0.12) / 2 - 0.10 =
        // 0.02
        let bf = surface.butterfly_25d(0.5).unwrap();
        assert!((bf - 0.02).abs() < 1e-10);
    }

    // ========================================
    // VolatilitySurface Trait Tests
    // ========================================

    #[test]
    fn test_volatility_surface_trait() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // VolatilitySurface trait method (strike = delta for FX)
        let vol = surface.volatility(0.5, 1.0).unwrap();
        assert!((vol - 0.11).abs() < 1e-10);
    }

    #[test]
    fn test_strike_domain() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let (k_min, k_max) = surface.strike_domain();
        assert!((k_min - 0.25).abs() < 1e-10);
        assert!((k_max - 0.75).abs() < 1e-10);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_surface_clone() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
        let cloned = surface.clone();

        let vol1 = surface.atm_volatility(1.0).unwrap();
        let vol2 = cloned.atm_volatility(1.0).unwrap();
        assert!((vol1 - vol2).abs() < 1e-10);
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_surface_with_f32() {
        let deltas = [0.25_f32, 0.5, 0.75];
        let expiries = [0.5_f32, 1.0];
        let vols = [[0.11_f32, 0.10, 0.11], [0.12_f32, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let vol = surface.atm_volatility(1.0_f32).unwrap();
        assert!((vol - 0.11_f32).abs() < 1e-6);
    }
}

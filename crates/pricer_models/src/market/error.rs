//! Market data error types.
//!
//! This module provides structured error handling for market data operations
//! including yield curve and volatility surface lookups.
//!
//! # Architecture
//!
//! The error hierarchy follows a domain-driven design:
//!
//! ```text
//! MarketError (root)
//! ├── Curve(CurveError)      - Yield curve operations
//! ├── Surface(SurfaceError)  - Volatility surface operations
//! └── Context(ContextError)  - Market data context operations
//! ```
//!
//! Legacy error types (`MarketDataError`, `MarketBuildError`) are preserved
//! for backward compatibility but new code should use the hierarchical types.

use pricer_core::types::{InterpolationError, PricingError};
use thiserror::Error;

use super::curves::CurveName;

// ============================================================================
// Unified Hierarchical Error Types (Design Document Section 2)
// ============================================================================

/// Root market error type.
///
/// Provides a unified entry point for all market-related errors.
/// Supports automatic conversion from sub-error types via `#[from]`.
///
/// # Examples
///
/// ```
/// use pricer_models::market::error::{MarketError, CurveError};
///
/// let err: MarketError = CurveError::not_found("SOFR").into();
/// assert!(matches!(err, MarketError::Curve(_)));
/// ```
#[derive(Debug, Error, Clone)]
pub enum MarketError {
    /// Curve-related errors (interpolation, bootstrap, lookup).
    #[error("curve error: {0}")]
    Curve(#[from] CurveError),

    /// Surface-related errors (volatility lookup, calibration).
    #[error("surface error: {0}")]
    Surface(#[from] SurfaceError),

    /// Context-related errors (provider, validation).
    #[error("context error: {0}")]
    Context(#[from] ContextError),
}

/// Curve-related errors.
///
/// Covers yield curve interpolation, bootstrapping, and lookup failures.
///
/// # Examples
///
/// ```
/// use pricer_models::market::error::CurveError;
///
/// let err = CurveError::interpolation(2.5, "extrapolation not allowed");
/// assert!(format!("{}", err).contains("2.5"));
/// ```
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CurveError {
    /// Interpolation failed at a specific time point.
    #[error("interpolation failed at t={time}: {reason}")]
    Interpolation {
        /// The time point where interpolation failed.
        time: f64,
        /// Description of the failure.
        reason: String,
    },

    /// Bootstrapping operation failed.
    #[error("bootstrap failed: {0}")]
    Bootstrap(String),

    /// Curve not found by name.
    #[error("curve not found: {0}")]
    NotFound(String),

    /// Invalid curve configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// FX curve-specific error.
    #[error("fx curve error: {0}")]
    Fx(String),

    /// Wrapped interpolation error from pricer_core.
    #[error("interpolation: {0}")]
    InterpolationCore(#[from] InterpolationError),
}

impl CurveError {
    /// Create an interpolation error.
    #[must_use]
    pub fn interpolation(time: f64, reason: impl Into<String>) -> Self {
        Self::Interpolation {
            time,
            reason: reason.into(),
        }
    }

    /// Create a bootstrap error.
    #[must_use]
    pub fn bootstrap(message: impl Into<String>) -> Self { Self::Bootstrap(message.into()) }

    /// Create a not found error.
    #[must_use]
    pub fn not_found(name: impl Into<String>) -> Self { Self::NotFound(name.into()) }

    /// Create an invalid config error.
    #[must_use]
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig(message.into())
    }

    /// Create an FX curve error.
    #[must_use]
    pub fn fx(message: impl Into<String>) -> Self { Self::Fx(message.into()) }
}

/// Surface-related errors.
///
/// Covers volatility surface lookup, calibration, and strike validation.
///
/// # Examples
///
/// ```
/// use pricer_models::market::error::SurfaceError;
///
/// let err = SurfaceError::invalid_strike(-0.01);
/// assert!(format!("{}", err).contains("-0.01"));
/// ```
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SurfaceError {
    /// Volatility lookup failed.
    #[error("volatility lookup failed: {0}")]
    VolLookup(String),

    /// Calibration failed.
    #[error("calibration failed: {0}")]
    Calibration(String),

    /// Invalid strike value.
    #[error("invalid strike: {0}")]
    InvalidStrike(String),

    /// Swaption-specific error.
    #[error("swaption vol error: {0}")]
    Swaption(String),

    /// FX volatility-specific error.
    #[error("fx vol error: {0}")]
    FxVol(String),
}

impl SurfaceError {
    /// Create a vol lookup error.
    #[must_use]
    pub fn vol_lookup(message: impl Into<String>) -> Self { Self::VolLookup(message.into()) }

    /// Create a calibration error.
    #[must_use]
    pub fn calibration(message: impl Into<String>) -> Self { Self::Calibration(message.into()) }

    /// Create an invalid strike error with the strike value.
    #[must_use]
    pub fn invalid_strike(strike: f64) -> Self { Self::InvalidStrike(format!("{strike}")) }

    /// Create a swaption error.
    #[must_use]
    pub fn swaption(message: impl Into<String>) -> Self { Self::Swaption(message.into()) }

    /// Create an FX vol error.
    #[must_use]
    pub fn fx_vol(message: impl Into<String>) -> Self { Self::FxVol(message.into()) }
}

/// Context-related errors.
///
/// Covers market data provider, validation, and missing data errors.
///
/// # Examples
///
/// ```
/// use pricer_models::market::error::ContextError;
///
/// let err = ContextError::not_found("SOFR curve");
/// assert!(format!("{}", err).contains("SOFR"));
/// ```
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ContextError {
    /// Market data not found.
    #[error("market data not found: {0}")]
    NotFound(String),

    /// Validation failed.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Provider error.
    #[error("provider error: {0}")]
    Provider(String),
}

impl ContextError {
    /// Create a not found error.
    #[must_use]
    pub fn not_found(description: impl Into<String>) -> Self { Self::NotFound(description.into()) }

    /// Create a validation error.
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self { Self::Validation(message.into()) }

    /// Create a provider error.
    #[must_use]
    pub fn provider(message: impl Into<String>) -> Self { Self::Provider(message.into()) }
}

// ============================================================================
// Conversions to PricingError
// ============================================================================

impl From<MarketError> for PricingError {
    fn from(err: MarketError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

impl From<CurveError> for PricingError {
    fn from(err: CurveError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

impl From<SurfaceError> for PricingError {
    fn from(err: SurfaceError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

impl From<ContextError> for PricingError {
    fn from(err: ContextError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

// ============================================================================
// Legacy Error Types (Preserved for Backward Compatibility)
// ============================================================================

/// Market data operation errors.
///
/// Provides structured error handling for yield curve and volatility surface
/// operations with descriptive context for each failure mode.
///
/// # Variants
///
/// - `InvalidMaturity`: Negative time to maturity
/// - `InvalidStrike`: Non-positive strike price
/// - `InvalidExpiry`: Non-positive time to expiry
/// - `OutOfBounds`: Query outside valid domain
/// - `Interpolation`: Wrapped interpolation error
/// - `InsufficientData`: Not enough data points for construction
/// - `CurveNotFound`: Requested curve does not exist in CurveSet
/// - `InterpolationFailed`: Interpolation operation failed
/// - `MissingData`: Required market data is missing
///
/// # Examples
///
/// ```
/// use pricer_models::market::MarketDataError;
///
/// let err = MarketDataError::InvalidMaturity { t: -1.0 };
/// assert!(format!("{}", err).contains("-1"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketDataError {
    /// Invalid maturity (negative time).
    #[error("Invalid maturity: t = {t}")]
    InvalidMaturity {
        /// The invalid maturity value
        t: f64,
    },

    /// Invalid strike price (non-positive).
    #[error("Invalid strike: K = {strike}")]
    InvalidStrike {
        /// The invalid strike value
        strike: f64,
    },

    /// Invalid expiry (non-positive).
    #[error("Invalid expiry: T = {expiry}")]
    InvalidExpiry {
        /// The invalid expiry value
        expiry: f64,
    },

    /// Query point outside valid domain.
    #[error("Out of bounds: {x} not in [{min}, {max}]")]
    OutOfBounds {
        /// The query point that was out of bounds
        x: f64,
        /// Minimum valid value
        min: f64,
        /// Maximum valid value
        max: f64,
    },

    /// Interpolation error.
    #[error("Interpolation error: {0}")]
    Interpolation(#[from] InterpolationError),

    /// Insufficient data for construction.
    #[error("Insufficient data: got {got}, need {need}")]
    InsufficientData {
        /// Number of points provided
        got: usize,
        /// Minimum number of points required
        need: usize,
    },

    /// Curve not found in CurveSet.
    #[error("Curve not found: {name}")]
    CurveNotFound {
        /// The name of the curve that was not found
        name: CurveName,
    },

    /// Interpolation operation failed.
    #[error("Interpolation failed: {reason}")]
    InterpolationFailed {
        /// Description of why interpolation failed
        reason: String,
    },

    /// Required market data is missing.
    #[error("Missing market data: {description}")]
    MissingData {
        /// Description of what data is missing
        description: String,
    },

    /// Feature not implemented for this curve type.
    #[error("Feature not implemented: {feature}")]
    NotImplemented {
        /// Description of the feature that is not implemented
        feature: String,
    },

    /// Unsupported index type for curve mapping.
    #[error("Unsupported index: {index}")]
    UnsupportedIndex {
        /// The index that is not supported
        index: String,
    },

    // ========================================
    // Index-Keyed Access Errors (Req 1.4, 2.6, 3.5)
    // ========================================
    /// Index not found in the market.
    ///
    /// Returned when attempting to access market data for an index
    /// that has not been registered in the IndexedMarket.
    #[error("Index not found: {index}")]
    IndexNotFound {
        /// String representation of the index that was not found
        index: String,
    },

    /// Curve not built for the specified index.
    ///
    /// Returned when the curve for an index exists in mapping but
    /// has not been constructed yet (e.g., bootstrapping not complete).
    #[error("Curve not built for index: {index}")]
    CurveNotBuilt {
        /// String representation of the index
        index: String,
    },

    /// VolCube not calibrated for the specified index.
    ///
    /// Returned when the volatility cube for an index exists but
    /// calibration has not been completed.
    #[error("VolCube not calibrated for index: {index}")]
    VolCubeNotCalibrated {
        /// String representation of the index
        index: String,
    },
}

/// Market build error types.
///
/// Provides structured error handling for `IndexedMarketBuilder` operations.
/// Used during Market construction to report configuration and validation
/// errors.
///
/// # Variants
///
/// - `DuplicateIndexMapping`: Same index registered more than once
/// - `IndexNotSpecified`: Required index not set before build
/// - `InvalidValuationDate`: Valuation date is invalid or inconsistent
///
/// # Examples
///
/// ```
/// use pricer_models::market::MarketBuildError;
///
/// let err = MarketBuildError::DuplicateIndexMapping {
///     index: "SOFR".to_string(),
/// };
/// assert!(format!("{}", err).contains("Duplicate"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketBuildError {
    /// Duplicate index mapping detected.
    ///
    /// Returned when attempting to register the same index twice
    /// in the `IndexedMarketBuilder`.
    #[error("Duplicate index mapping: {index}")]
    DuplicateIndexMapping {
        /// String representation of the duplicate index
        index: String,
    },

    /// Required index not specified.
    ///
    /// Returned when `build()` is called but a required index
    /// was not registered (e.g., no curves at all).
    #[error("Index not specified: {context}")]
    IndexNotSpecified {
        /// Context describing what was missing
        context: String,
    },

    /// Invalid valuation date.
    ///
    /// Returned when the valuation date is invalid or inconsistent
    /// with other market data.
    #[error("Invalid valuation date: {reason}")]
    InvalidValuationDate {
        /// Description of why the date is invalid
        reason: String,
    },
}

impl From<MarketBuildError> for PricingError {
    fn from(err: MarketBuildError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

impl From<MarketDataError> for PricingError {
    fn from(err: MarketDataError) -> Self { PricingError::InvalidInput(err.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_maturity_display() {
        let err = MarketDataError::InvalidMaturity { t: -1.5 };
        assert_eq!(format!("{}", err), "Invalid maturity: t = -1.5");
    }

    #[test]
    fn test_invalid_strike_display() {
        let err = MarketDataError::InvalidStrike { strike: -100.0 };
        assert_eq!(format!("{}", err), "Invalid strike: K = -100");
    }

    #[test]
    fn test_invalid_expiry_display() {
        let err = MarketDataError::InvalidExpiry { expiry: 0.0 };
        assert_eq!(format!("{}", err), "Invalid expiry: T = 0");
    }

    #[test]
    fn test_out_of_bounds_display() {
        let err = MarketDataError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        assert_eq!(format!("{}", err), "Out of bounds: 5 not in [0, 3]");
    }

    #[test]
    fn test_insufficient_data_display() {
        let err = MarketDataError::InsufficientData { got: 1, need: 2 };
        assert_eq!(format!("{}", err), "Insufficient data: got 1, need 2");
    }

    #[test]
    fn test_from_interpolation_error() {
        let interp_err = InterpolationError::OutOfBounds {
            x: 5.0,
            min: 0.0,
            max: 3.0,
        };
        let mkt_err: MarketDataError = interp_err.into();
        match mkt_err {
            MarketDataError::Interpolation(_) => {}
            _ => panic!("Expected Interpolation variant"),
        }
    }

    #[test]
    fn test_into_pricing_error() {
        let mkt_err = MarketDataError::InvalidMaturity { t: -1.0 };
        let pricing_err: PricingError = mkt_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("-1"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = MarketDataError::InvalidMaturity { t: -1.0 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = MarketDataError::InvalidStrike { strike: 0.0 };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_curve_not_found_display() {
        let err = MarketDataError::CurveNotFound {
            name: CurveName::Sofr,
        };
        let display = format!("{}", err);
        assert!(display.contains("Curve not found"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_curve_not_found_custom() {
        let err = MarketDataError::CurveNotFound {
            name: CurveName::Custom("MY_CURVE"),
        };
        let display = format!("{}", err);
        assert!(display.contains("MY_CURVE"));
    }

    #[test]
    fn test_interpolation_failed_display() {
        let err = MarketDataError::InterpolationFailed {
            reason: "Negative volatility".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Interpolation failed"));
        assert!(display.contains("Negative volatility"));
    }

    #[test]
    fn test_missing_data_display() {
        let err = MarketDataError::MissingData {
            description: "SOFR curve required for floating leg".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Missing market data"));
        assert!(display.contains("SOFR curve"));
    }

    // ========================================
    // Index-Keyed Access Error Tests
    // ========================================

    #[test]
    fn test_index_not_found_display() {
        let err = MarketDataError::IndexNotFound {
            index: "SOFR".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Index not found"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_index_not_found_equality() {
        let err1 = MarketDataError::IndexNotFound {
            index: "SOFR".to_string(),
        };
        let err2 = MarketDataError::IndexNotFound {
            index: "SOFR".to_string(),
        };
        let err3 = MarketDataError::IndexNotFound {
            index: "EURIBOR3M".to_string(),
        };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_curve_not_built_display() {
        let err = MarketDataError::CurveNotBuilt {
            index: "TONAR".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Curve not built"));
        assert!(display.contains("TONAR"));
    }

    #[test]
    fn test_volcube_not_calibrated_display() {
        let err = MarketDataError::VolCubeNotCalibrated {
            index: "SONIA".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("VolCube not calibrated"));
        assert!(display.contains("SONIA"));
    }

    #[test]
    fn test_index_errors_into_pricing_error() {
        let err = MarketDataError::IndexNotFound {
            index: "SOFR".to_string(),
        };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("Index not found"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    // ========================================
    // MarketBuildError Tests
    // ========================================

    #[test]
    fn test_duplicate_index_mapping_display() {
        let err = MarketBuildError::DuplicateIndexMapping {
            index: "SOFR".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Duplicate index mapping"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_index_not_specified_display() {
        let err = MarketBuildError::IndexNotSpecified {
            context: "No curves registered".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Index not specified"));
        assert!(display.contains("No curves"));
    }

    #[test]
    fn test_invalid_valuation_date_display() {
        let err = MarketBuildError::InvalidValuationDate {
            reason: "Date is in the past".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Invalid valuation date"));
        assert!(display.contains("past"));
    }

    #[test]
    fn test_market_build_error_equality() {
        let err1 = MarketBuildError::DuplicateIndexMapping {
            index: "SOFR".to_string(),
        };
        let err2 = MarketBuildError::DuplicateIndexMapping {
            index: "SOFR".to_string(),
        };
        let err3 = MarketBuildError::DuplicateIndexMapping {
            index: "SONIA".to_string(),
        };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_market_build_error_clone() {
        let err1 = MarketBuildError::InvalidValuationDate {
            reason: "test".to_string(),
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_market_build_error_into_pricing_error() {
        let err = MarketBuildError::DuplicateIndexMapping {
            index: "EURIBOR3M".to_string(),
        };
        let pricing_err: PricingError = err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("Duplicate"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    // ========================================
    // Unified Hierarchical Error Tests (Task 1.2)
    // ========================================

    // --- CurveError Tests ---

    #[test]
    fn test_curve_error_interpolation() {
        let err = CurveError::interpolation(2.5, "extrapolation not allowed");
        let display = format!("{}", err);
        assert!(display.contains("2.5"));
        assert!(display.contains("extrapolation not allowed"));
    }

    #[test]
    fn test_curve_error_bootstrap() {
        let err = CurveError::bootstrap("solver failed at maturity 5Y");
        let display = format!("{}", err);
        assert!(display.contains("bootstrap failed"));
        assert!(display.contains("solver failed"));
    }

    #[test]
    fn test_curve_error_not_found() {
        let err = CurveError::not_found("SOFR");
        let display = format!("{}", err);
        assert!(display.contains("curve not found"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_curve_error_invalid_config() {
        let err = CurveError::invalid_config("negative tolerance");
        let display = format!("{}", err);
        assert!(display.contains("invalid configuration"));
        assert!(display.contains("negative tolerance"));
    }

    #[test]
    fn test_curve_error_fx() {
        let err = CurveError::fx("missing forward points");
        let display = format!("{}", err);
        assert!(display.contains("fx curve error"));
        assert!(display.contains("missing forward points"));
    }

    #[test]
    fn test_curve_error_equality() {
        let err1 = CurveError::not_found("SOFR");
        let err2 = CurveError::not_found("SOFR");
        let err3 = CurveError::not_found("SONIA");
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_curve_error_clone() {
        let err1 = CurveError::bootstrap("test");
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // --- SurfaceError Tests ---

    #[test]
    fn test_surface_error_vol_lookup() {
        let err = SurfaceError::vol_lookup("out of strike range");
        let display = format!("{}", err);
        assert!(display.contains("volatility lookup failed"));
        assert!(display.contains("out of strike range"));
    }

    #[test]
    fn test_surface_error_calibration() {
        let err = SurfaceError::calibration("optimiser did not converge");
        let display = format!("{}", err);
        assert!(display.contains("calibration failed"));
        assert!(display.contains("optimiser"));
    }

    #[test]
    fn test_surface_error_invalid_strike() {
        let err = SurfaceError::invalid_strike(-0.01);
        let display = format!("{}", err);
        assert!(display.contains("invalid strike"));
        assert!(display.contains("-0.01"));
    }

    #[test]
    fn test_surface_error_swaption() {
        let err = SurfaceError::swaption("missing ATM quote");
        let display = format!("{}", err);
        assert!(display.contains("swaption vol error"));
        assert!(display.contains("ATM"));
    }

    #[test]
    fn test_surface_error_fx_vol() {
        let err = SurfaceError::fx_vol("SABR calibration failed");
        let display = format!("{}", err);
        assert!(display.contains("fx vol error"));
        assert!(display.contains("SABR"));
    }

    #[test]
    fn test_surface_error_equality() {
        let err1 = SurfaceError::vol_lookup("test");
        let err2 = SurfaceError::vol_lookup("test");
        let err3 = SurfaceError::vol_lookup("other");
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // --- ContextError Tests ---

    #[test]
    fn test_context_error_not_found() {
        let err = ContextError::not_found("SOFR curve");
        let display = format!("{}", err);
        assert!(display.contains("market data not found"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_context_error_validation() {
        let err = ContextError::validation("missing required curves");
        let display = format!("{}", err);
        assert!(display.contains("validation failed"));
        assert!(display.contains("missing"));
    }

    #[test]
    fn test_context_error_provider() {
        let err = ContextError::provider("lazy resolution failed");
        let display = format!("{}", err);
        assert!(display.contains("provider error"));
        assert!(display.contains("lazy resolution"));
    }

    #[test]
    fn test_context_error_equality() {
        let err1 = ContextError::not_found("SOFR");
        let err2 = ContextError::not_found("SOFR");
        let err3 = ContextError::not_found("SONIA");
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // --- MarketError (Root) Tests ---

    #[test]
    fn test_market_error_from_curve_error() {
        let curve_err = CurveError::not_found("SOFR");
        let market_err: MarketError = curve_err.into();
        let display = format!("{}", market_err);
        assert!(display.contains("curve error"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_market_error_from_surface_error() {
        let surface_err = SurfaceError::vol_lookup("out of bounds");
        let market_err: MarketError = surface_err.into();
        let display = format!("{}", market_err);
        assert!(display.contains("surface error"));
        assert!(display.contains("out of bounds"));
    }

    #[test]
    fn test_market_error_from_context_error() {
        let context_err = ContextError::validation("incomplete market");
        let market_err: MarketError = context_err.into();
        let display = format!("{}", market_err);
        assert!(display.contains("context error"));
        assert!(display.contains("incomplete"));
    }

    #[test]
    fn test_market_error_matches_variant() {
        let market_err: MarketError = CurveError::bootstrap("test").into();
        assert!(matches!(market_err, MarketError::Curve(_)));

        let market_err: MarketError = SurfaceError::calibration("test").into();
        assert!(matches!(market_err, MarketError::Surface(_)));

        let market_err: MarketError = ContextError::provider("test").into();
        assert!(matches!(market_err, MarketError::Context(_)));
    }

    #[test]
    fn test_market_error_into_pricing_error() {
        let market_err: MarketError = CurveError::not_found("SOFR").into();
        let pricing_err: PricingError = market_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("curve error"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_curve_error_into_pricing_error() {
        let curve_err = CurveError::bootstrap("failed");
        let pricing_err: PricingError = curve_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("bootstrap"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_surface_error_into_pricing_error() {
        let surface_err = SurfaceError::calibration("failed");
        let pricing_err: PricingError = surface_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("calibration"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_context_error_into_pricing_error() {
        let context_err = ContextError::not_found("curve");
        let pricing_err: PricingError = context_err.into();
        match pricing_err {
            PricingError::InvalidInput(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_market_error_clone() {
        let market_err: MarketError = CurveError::not_found("SOFR").into();
        let cloned = market_err.clone();
        let display1 = format!("{}", market_err);
        let display2 = format!("{}", cloned);
        assert_eq!(display1, display2);
    }

    #[test]
    fn test_error_trait_for_unified_types() {
        let curve_err = CurveError::not_found("test");
        let _: &dyn std::error::Error = &curve_err;

        let surface_err = SurfaceError::vol_lookup("test");
        let _: &dyn std::error::Error = &surface_err;

        let context_err = ContextError::not_found("test");
        let _: &dyn std::error::Error = &context_err;

        let market_err: MarketError = curve_err.into();
        let _: &dyn std::error::Error = &market_err;
    }
}

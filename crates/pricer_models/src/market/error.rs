//! Market data error types.
//!
//! This module provides structured error handling for market data operations
//! including yield curve and volatility surface lookups.

use pricer_core::types::{InterpolationError, PricingError};
use thiserror::Error;

use super::curves::CurveName;

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
/// Used during Market construction to report configuration and validation errors.
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
}

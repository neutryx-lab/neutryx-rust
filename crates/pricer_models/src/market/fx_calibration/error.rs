//! FX Calibration Error Types.
//!
//! This module provides structured error handling for FX volatility surface
//! calibration operations.

use thiserror::Error;

/// Errors that can occur during FX volatility surface calibration.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FxCalibrationError {
    /// Insufficient market data for calibration.
    #[error("Insufficient market data: expected at least {expected} quotes, got {got}")]
    InsufficientData {
        /// Expected minimum number of quotes.
        expected: usize,
        /// Actual number of quotes received.
        got: usize,
    },

    /// Invalid market quote value.
    #[error("Invalid market quote: {message}")]
    InvalidQuote {
        /// Description of the invalid quote.
        message: String,
    },

    /// Missing quote for a required instrument.
    #[error("Missing quote for instrument: {instrument}")]
    MissingQuote {
        /// The instrument identifier.
        instrument: String,
    },

    /// Calibration optimiser failed to converge.
    #[error(
        "Calibration failed to converge after {iterations} iterations (residual: {residual:.2e})"
    )]
    ConvergenceFailed {
        /// Number of iterations attempted.
        iterations: usize,
        /// Final residual value.
        residual: f64,
    },

    /// Invalid configuration parameter.
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the invalid configuration.
        message: String,
    },

    /// Arbitrage-free condition violated.
    #[error("Arbitrage violation detected: {message}")]
    ArbitrageViolation {
        /// Description of the violation.
        message: String,
    },

    /// Forward curve construction failed.
    #[error("Forward curve error: {message}")]
    ForwardCurveError {
        /// Description of the error.
        message: String,
    },

    /// Interpolation error.
    #[error("Interpolation error: {message}")]
    InterpolationError {
        /// Description of the error.
        message: String,
    },

    /// Invalid expiry date or time.
    #[error("Invalid expiry: {message}")]
    InvalidExpiry {
        /// Description of the error.
        message: String,
    },

    /// Delta to strike conversion failed.
    #[error("Delta-strike conversion failed: {message}")]
    DeltaStrikeConversionFailed {
        /// Description of the error.
        message: String,
    },

    /// SABR model error.
    #[error("SABR model error: {message}")]
    SabrError {
        /// Description of the error.
        message: String,
    },

    /// Currency pair not supported.
    #[error("Unsupported currency pair: {pair}")]
    UnsupportedCurrencyPair {
        /// The currency pair.
        pair: String,
    },
}

impl FxCalibrationError {
    /// Creates an insufficient data error.
    #[must_use]
    pub fn insufficient_data(expected: usize, got: usize) -> Self {
        Self::InsufficientData { expected, got }
    }

    /// Creates an invalid quote error.
    #[must_use]
    pub fn invalid_quote(message: impl Into<String>) -> Self {
        Self::InvalidQuote {
            message: message.into(),
        }
    }

    /// Creates a missing quote error.
    #[must_use]
    pub fn missing_quote(instrument: impl Into<String>) -> Self {
        Self::MissingQuote {
            instrument: instrument.into(),
        }
    }

    /// Creates a convergence failure error.
    #[must_use]
    pub fn convergence_failed(iterations: usize, residual: f64) -> Self {
        Self::ConvergenceFailed {
            iterations,
            residual,
        }
    }

    /// Creates an invalid config error.
    #[must_use]
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Creates an arbitrage violation error.
    #[must_use]
    pub fn arbitrage_violation(message: impl Into<String>) -> Self {
        Self::ArbitrageViolation {
            message: message.into(),
        }
    }

    /// Creates a forward curve error.
    #[must_use]
    pub fn forward_curve_error(message: impl Into<String>) -> Self {
        Self::ForwardCurveError {
            message: message.into(),
        }
    }

    /// Creates an interpolation error.
    #[must_use]
    pub fn interpolation_error(message: impl Into<String>) -> Self {
        Self::InterpolationError {
            message: message.into(),
        }
    }

    /// Creates an invalid expiry error.
    #[must_use]
    pub fn invalid_expiry(message: impl Into<String>) -> Self {
        Self::InvalidExpiry {
            message: message.into(),
        }
    }

    /// Creates a delta-strike conversion error.
    #[must_use]
    pub fn delta_strike_conversion_failed(message: impl Into<String>) -> Self {
        Self::DeltaStrikeConversionFailed {
            message: message.into(),
        }
    }

    /// Creates a SABR model error.
    #[must_use]
    pub fn sabr_error(message: impl Into<String>) -> Self {
        Self::SabrError {
            message: message.into(),
        }
    }

    /// Creates an unsupported currency pair error.
    #[must_use]
    pub fn unsupported_currency_pair(pair: impl Into<String>) -> Self {
        Self::UnsupportedCurrencyPair { pair: pair.into() }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_data_display() {
        let err = FxCalibrationError::insufficient_data(5, 3);
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_invalid_quote_display() {
        let err = FxCalibrationError::invalid_quote("Negative volatility");
        let msg = format!("{}", err);
        assert!(msg.contains("Negative volatility"));
    }

    #[test]
    fn test_missing_quote_display() {
        let err = FxCalibrationError::missing_quote("EURUSD 1Y ATM");
        let msg = format!("{}", err);
        assert!(msg.contains("EURUSD 1Y ATM"));
    }

    #[test]
    fn test_convergence_failed_display() {
        let err = FxCalibrationError::convergence_failed(100, 1.5e-6);
        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("1.50e-6") || msg.contains("1.5e-6"));
    }

    #[test]
    fn test_invalid_config_display() {
        let err = FxCalibrationError::invalid_config("Beta out of range");
        let msg = format!("{}", err);
        assert!(msg.contains("Beta out of range"));
    }

    #[test]
    fn test_arbitrage_violation_display() {
        let err = FxCalibrationError::arbitrage_violation("Calendar spread arbitrage");
        let msg = format!("{}", err);
        assert!(msg.contains("Calendar spread"));
    }

    #[test]
    fn test_forward_curve_error_display() {
        let err = FxCalibrationError::forward_curve_error("Missing swap points");
        let msg = format!("{}", err);
        assert!(msg.contains("Missing swap points"));
    }

    #[test]
    fn test_interpolation_error_display() {
        let err = FxCalibrationError::interpolation_error("Out of bounds");
        let msg = format!("{}", err);
        assert!(msg.contains("Out of bounds"));
    }

    #[test]
    fn test_invalid_expiry_display() {
        let err = FxCalibrationError::invalid_expiry("Expiry in the past");
        let msg = format!("{}", err);
        assert!(msg.contains("Expiry in the past"));
    }

    #[test]
    fn test_delta_strike_conversion_display() {
        let err = FxCalibrationError::delta_strike_conversion_failed("Volatility too low");
        let msg = format!("{}", err);
        assert!(msg.contains("Volatility too low"));
    }

    #[test]
    fn test_sabr_error_display() {
        let err = FxCalibrationError::sabr_error("Invalid alpha");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid alpha"));
    }

    #[test]
    fn test_unsupported_currency_pair_display() {
        let err = FxCalibrationError::unsupported_currency_pair("XYZ/ABC");
        let msg = format!("{}", err);
        assert!(msg.contains("XYZ/ABC"));
    }

    #[test]
    fn test_error_equality() {
        let err1 = FxCalibrationError::insufficient_data(5, 3);
        let err2 = FxCalibrationError::insufficient_data(5, 3);
        let err3 = FxCalibrationError::insufficient_data(5, 4);

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_clone() {
        let err = FxCalibrationError::invalid_quote("test");
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }
}

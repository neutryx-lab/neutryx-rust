//! Compilation error types for Trade → IR transformation.
//!
//! This module defines structured error types for the compilation phase,
//! providing detailed context for debugging and error handling.

use thiserror::Error;

/// Error type for Trade → IR compilation.
///
/// `CompileError` captures all possible failure modes during the
/// transformation of hierarchical `Trade` structures into
/// `PricingKernel` IR.
///
/// # Examples
///
/// ```
/// use pricer_core::ir::CompileError;
///
/// let err = CompileError::UnsupportedInstrument("CreditLinkedNote".to_string());
/// assert!(err.to_string().contains("CreditLinkedNote"));
/// ```
#[derive(Debug, Clone, Error)]
pub enum CompileError {
    /// The instrument type is not supported for IR compilation.
    #[error("Unsupported instrument type: {0}")]
    UnsupportedInstrument(String),

    /// The rate index could not be resolved to an index ID.
    #[error("Unknown rate index: {0}")]
    UnknownIndex(String),

    /// The exotic payoff is not supported.
    #[error("Unsupported exotic payoff: {0}")]
    UnsupportedPayoff(String),

    /// The payment schedule is invalid.
    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),

    /// A required calendar is missing.
    #[error("Missing calendar for: {0}")]
    MissingCalendar(String),

    /// Date conversion or calculation error.
    #[error("Date error: {0}")]
    DateError(String),

    /// The trade has no legs or cashflows.
    #[error("Empty trade: {0}")]
    EmptyTrade(String),

    /// Array length mismatch in kernel construction.
    #[error("Array length mismatch: expected {expected}, got {actual}")]
    LengthMismatch {
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },

    /// Currency could not be resolved.
    #[error("Unknown currency: {0}")]
    UnknownCurrency(String),

    /// Discount curve could not be resolved.
    #[error("Unknown discount curve: {0}")]
    UnknownDiscountCurve(String),
}

impl CompileError {
    /// Creates an `UnsupportedInstrument` error.
    #[must_use]
    pub fn unsupported_instrument(name: impl Into<String>) -> Self {
        Self::UnsupportedInstrument(name.into())
    }

    /// Creates an `UnknownIndex` error.
    #[must_use]
    pub fn unknown_index(name: impl Into<String>) -> Self {
        Self::UnknownIndex(name.into())
    }

    /// Creates an `InvalidSchedule` error.
    #[must_use]
    pub fn invalid_schedule(msg: impl Into<String>) -> Self {
        Self::InvalidSchedule(msg.into())
    }

    /// Creates a `LengthMismatch` error.
    #[must_use]
    pub fn length_mismatch(expected: usize, actual: usize) -> Self {
        Self::LengthMismatch { expected, actual }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_instrument_error() {
        let err = CompileError::UnsupportedInstrument("CreditLinkedNote".to_string());
        assert!(err.to_string().contains("Unsupported instrument type"));
        assert!(err.to_string().contains("CreditLinkedNote"));
    }

    #[test]
    fn test_unknown_index_error() {
        let err = CompileError::UnknownIndex("UNKNOWN_LIBOR".to_string());
        assert!(err.to_string().contains("Unknown rate index"));
        assert!(err.to_string().contains("UNKNOWN_LIBOR"));
    }

    #[test]
    fn test_unsupported_payoff_error() {
        let err = CompileError::UnsupportedPayoff("RainbowOption".to_string());
        assert!(err.to_string().contains("Unsupported exotic payoff"));
    }

    #[test]
    fn test_invalid_schedule_error() {
        let err = CompileError::InvalidSchedule("Start date after end date".to_string());
        assert!(err.to_string().contains("Invalid schedule"));
    }

    #[test]
    fn test_missing_calendar_error() {
        let err = CompileError::MissingCalendar("TOKYO".to_string());
        assert!(err.to_string().contains("Missing calendar"));
    }

    #[test]
    fn test_date_error() {
        let err = CompileError::DateError("Invalid date format".to_string());
        assert!(err.to_string().contains("Date error"));
    }

    #[test]
    fn test_empty_trade_error() {
        let err = CompileError::EmptyTrade("TRADE001".to_string());
        assert!(err.to_string().contains("Empty trade"));
    }

    #[test]
    fn test_length_mismatch_error() {
        let err = CompileError::LengthMismatch {
            expected: 10,
            actual: 5,
        };
        assert!(err.to_string().contains("expected 10"));
        assert!(err.to_string().contains("got 5"));
    }

    #[test]
    fn test_error_constructors() {
        let err1 = CompileError::unsupported_instrument("test");
        assert!(matches!(err1, CompileError::UnsupportedInstrument(_)));

        let err2 = CompileError::unknown_index("test");
        assert!(matches!(err2, CompileError::UnknownIndex(_)));

        let err3 = CompileError::invalid_schedule("test");
        assert!(matches!(err3, CompileError::InvalidSchedule(_)));

        let err4 = CompileError::length_mismatch(10, 5);
        assert!(matches!(err4, CompileError::LengthMismatch { .. }));
    }

    #[test]
    fn test_error_clone() {
        let err = CompileError::UnknownIndex("SOFR".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_error_debug() {
        let err = CompileError::UnknownIndex("SOFR".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("UnknownIndex"));
    }
}

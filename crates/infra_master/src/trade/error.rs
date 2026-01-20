//! Trade-specific error types.
//!
//! This module provides structured error handling for trade construction
//! and validation.

use thiserror::Error;

use crate::DateError;

/// Errors that can occur during trade construction and validation.
///
/// Provides structured error handling for trade builders with
/// descriptive context for each failure mode.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TradeError {
    /// Invalid schedule provided (e.g., empty, dates out of order).
    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),

    /// Empty leg: no cashflows generated.
    #[error("Empty leg: no cashflows generated")]
    EmptyLeg,

    /// Invalid notional value (e.g., negative).
    #[error("Invalid notional: {0}")]
    InvalidNotional(f64),

    /// Currency mismatch between expected and actual values.
    #[error("Mismatched currency: expected {expected}, got {actual}")]
    MismatchedCurrency {
        /// Expected currency code.
        expected: String,
        /// Actual currency code.
        actual: String,
    },

    /// Invalid payoff configuration.
    #[error("Invalid payoff configuration")]
    InvalidPayoff,

    /// Incompatible convention for the given instrument type.
    #[error("Incompatible convention for this instrument type")]
    IncompatibleConvention,

    /// Date calculation error.
    #[error("Date calculation error: {0}")]
    DateError(#[from] DateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_schedule_display() {
        let err = TradeError::InvalidSchedule("Empty schedule".into());
        assert_eq!(format!("{}", err), "Invalid schedule: Empty schedule");
    }

    #[test]
    fn test_empty_leg_display() {
        let err = TradeError::EmptyLeg;
        assert_eq!(format!("{}", err), "Empty leg: no cashflows generated");
    }

    #[test]
    fn test_invalid_notional_display() {
        let err = TradeError::InvalidNotional(-1000.0);
        assert_eq!(format!("{}", err), "Invalid notional: -1000");
    }

    #[test]
    fn test_mismatched_currency_display() {
        let err = TradeError::MismatchedCurrency {
            expected: "USD".into(),
            actual: "EUR".into(),
        };
        assert_eq!(
            format!("{}", err),
            "Mismatched currency: expected USD, got EUR"
        );
    }

    #[test]
    fn test_invalid_payoff_display() {
        let err = TradeError::InvalidPayoff;
        assert_eq!(format!("{}", err), "Invalid payoff configuration");
    }

    #[test]
    fn test_incompatible_convention_display() {
        let err = TradeError::IncompatibleConvention;
        assert_eq!(
            format!("{}", err),
            "Incompatible convention for this instrument type"
        );
    }

    #[test]
    fn test_date_error_conversion() {
        let date_err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let trade_err: TradeError = date_err.into();
        assert!(matches!(trade_err, TradeError::DateError(_)));
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = TradeError::InvalidSchedule("test".into());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = TradeError::InvalidNotional(100.0);
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_debug_format() {
        let err = TradeError::EmptyLeg;
        let debug = format!("{:?}", err);
        assert!(debug.contains("EmptyLeg"));
    }
}

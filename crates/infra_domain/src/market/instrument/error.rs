//! Instrument-specific error types.

use thiserror::Error;

use crate::trade::TradeError;

/// Errors that can occur during instrument operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum InstrumentError {
    /// Invalid instrument parameter.
    #[error("Invalid instrument parameter: {0}")]
    InvalidParameter(String),

    /// Missing convention required for CF expansion.
    #[error("Missing convention for {instrument_type}")]
    MissingConvention {
        /// The instrument type requiring the convention.
        instrument_type: String,
    },

    /// Invalid date configuration.
    #[error("Invalid date configuration: {0}")]
    InvalidDate(String),

    /// Validation failed for the instrument.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Cashflow expansion failed.
    #[error("CF expansion failed: {0}")]
    ExpansionFailed(String),

    /// Underlying trade error.
    #[error(transparent)]
    TradeError(#[from] TradeError),
}

impl InstrumentError {
    /// Creates an invalid parameter error.
    #[must_use]
    pub fn invalid_parameter(msg: impl Into<String>) -> Self {
        InstrumentError::InvalidParameter(msg.into())
    }

    /// Creates a missing convention error.
    #[must_use]
    pub fn missing_convention(instrument_type: impl Into<String>) -> Self {
        InstrumentError::MissingConvention {
            instrument_type: instrument_type.into(),
        }
    }

    /// Creates an invalid date error.
    #[must_use]
    pub fn invalid_date(msg: impl Into<String>) -> Self { InstrumentError::InvalidDate(msg.into()) }

    /// Creates a validation error.
    #[must_use]
    pub fn validation_failed(msg: impl Into<String>) -> Self {
        InstrumentError::ValidationFailed(msg.into())
    }

    /// Creates an expansion error.
    #[must_use]
    pub fn expansion_failed(msg: impl Into<String>) -> Self {
        InstrumentError::ExpansionFailed(msg.into())
    }

    /// Checks that a value is strictly positive (> 0).
    pub fn check_positive(val: f64, field: &str) -> Result<(), Self> {
        if val <= 0.0 {
            return Err(Self::invalid_parameter(format!("{field} must be positive")));
        }
        Ok(())
    }

    /// Checks that a value is non-negative (>= 0).
    pub fn check_non_negative(val: f64, field: &str) -> Result<(), Self> {
        if val < 0.0 {
            return Err(Self::invalid_parameter(format!(
                "{field} must be non-negative"
            )));
        }
        Ok(())
    }

    /// Checks that a value falls within the given range.
    pub fn check_range(val: f64, min: f64, max: f64, field: &str) -> Result<(), Self> {
        if val < min || val > max {
            return Err(Self::invalid_parameter(format!(
                "{field} must be between {min} and {max}"
            )));
        }
        Ok(())
    }

    /// Checks that a string is not empty.
    pub fn check_not_empty(val: &str, field: &str) -> Result<(), Self> {
        if val.is_empty() {
            return Err(Self::invalid_parameter(format!(
                "{field} must be specified"
            )));
        }
        Ok(())
    }

    /// Checks that `later` is strictly after `earlier`.
    pub fn check_date_order(
        earlier: crate::time::Date,
        later: crate::time::Date,
        msg: &str,
    ) -> Result<(), Self> {
        if later <= earlier {
            return Err(Self::invalid_date(msg));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_parameter_display() {
        let err = InstrumentError::invalid_parameter("Negative notional");
        assert_eq!(
            format!("{}", err),
            "Invalid instrument parameter: Negative notional"
        );
    }

    #[test]
    fn test_missing_convention_display() {
        let err = InstrumentError::missing_convention("Swaption");
        assert_eq!(format!("{}", err), "Missing convention for Swaption");
    }

    #[test]
    fn test_invalid_date_display() {
        let err = InstrumentError::invalid_date("Expiry before start date");
        assert_eq!(
            format!("{}", err),
            "Invalid date configuration: Expiry before start date"
        );
    }

    #[test]
    fn test_validation_failed_display() {
        let err = InstrumentError::validation_failed("Strike must be positive");
        assert_eq!(
            format!("{}", err),
            "Validation failed: Strike must be positive"
        );
    }

    #[test]
    fn test_expansion_failed_display() {
        let err = InstrumentError::expansion_failed("Cannot generate cashflows");
        assert_eq!(
            format!("{}", err),
            "CF expansion failed: Cannot generate cashflows"
        );
    }

    #[test]
    fn test_from_trade_error() {
        let trade_err = TradeError::InvalidNotional(-100.0);
        let instrument_err: InstrumentError = trade_err.into();
        assert!(matches!(instrument_err, InstrumentError::TradeError(_)));
    }

    #[test]
    fn test_error_clone() {
        let err = InstrumentError::invalid_parameter("test");
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_debug() {
        let err = InstrumentError::invalid_parameter("test");
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidParameter"));
    }
}

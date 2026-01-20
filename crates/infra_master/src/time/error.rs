//! Time-related error types.
//!
//! This module provides unified error handling for time-related operations.
//!
//! # Examples
//!
//! ```
//! use infra_master::time::TimeError;
//!
//! let err = TimeError::InvalidDate { year: 2024, month: 2, day: 30 };
//! assert_eq!(format!("{}", err), "Invalid date: 2024-02-30");
//! ```

use thiserror::Error;

/// Unified error type for time-related operations.
///
/// Provides structured error handling for date construction, parsing,
/// calculations, and calendar operations with descriptive context
/// for each failure mode.
///
/// # Variants
/// - `InvalidDate`: Invalid date components (e.g., February 30th)
/// - `ParseError`: Failed to parse date string
/// - `CalculationError`: Calculation error (e.g., invalid serial number)
/// - `CalendarError`: Calendar-related error
///
/// # Examples
///
/// ```
/// use infra_master::time::TimeError;
///
/// let err = TimeError::InvalidDate { year: 2024, month: 2, day: 30 };
/// assert_eq!(format!("{}", err), "Invalid date: 2024-02-30");
///
/// let err = TimeError::CalculationError("Serial number must be >= 1".to_string());
/// assert!(format!("{}", err).contains("Serial number"));
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// Invalid date components (e.g., February 30th).
    #[error("Invalid date: {year}-{month:02}-{day:02}")]
    InvalidDate {
        /// Year component
        year: i32,
        /// Month component (1-12)
        month: u32,
        /// Day component (1-31)
        day: u32,
    },

    /// Failed to parse date string.
    #[error("Date parse error: {0}")]
    ParseError(String),

    /// Calculation error (e.g., invalid serial number).
    #[error("Calculation error: {0}")]
    CalculationError(String),

    /// Calendar-related error.
    #[error("Calendar error: {0}")]
    CalendarError(String),
}

/// Backward compatibility alias for DateError.
#[deprecated(since = "0.3.0", note = "Use TimeError instead")]
pub type DateError = TimeError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_date_display() {
        let err = TimeError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        assert_eq!(format!("{}", err), "Invalid date: 2024-02-30");
    }

    #[test]
    fn test_parse_error_display() {
        let err = TimeError::ParseError("invalid format".to_string());
        assert_eq!(format!("{}", err), "Date parse error: invalid format");
    }

    #[test]
    fn test_calculation_error_display() {
        let err = TimeError::CalculationError("Serial number must be >= 1".to_string());
        assert_eq!(
            format!("{}", err),
            "Calculation error: Serial number must be >= 1"
        );
    }

    #[test]
    fn test_calendar_error_display() {
        let err = TimeError::CalendarError("Calendar not found".to_string());
        assert_eq!(format!("{}", err), "Calendar error: Calendar not found");
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = TimeError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = TimeError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);

        let err3 = TimeError::ParseError("test".to_string());
        let err4 = err3.clone();
        assert_eq!(err3, err4);
    }

    #[test]
    fn test_debug() {
        let err = TimeError::CalculationError("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("CalculationError"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_alias() {
        let err: DateError = TimeError::InvalidDate {
            year: 2024,
            month: 1,
            day: 1,
        };
        assert_eq!(format!("{}", err), "Invalid date: 2024-01-01");
    }
}

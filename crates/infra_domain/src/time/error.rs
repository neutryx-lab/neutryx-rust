//! Time-related error types.

use thiserror::Error;

use crate::error::DateError;

/// Unified error type for time-related operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// Invalid date components (e.g., February 30th).
    #[error("Invalid date: {year}-{month:02}-{day:02}")]
    InvalidDate {
        /// Year component.
        year: i32,
        /// Month component (1-12).
        month: u32,
        /// Day component (1-31).
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

/// Convert DateError to TimeError.
impl From<DateError> for TimeError {
    fn from(err: DateError) -> Self {
        match err {
            DateError::InvalidDate { year, month, day } => {
                TimeError::InvalidDate { year, month, day }
            }
            DateError::ParseError(msg) => TimeError::ParseError(msg),
        }
    }
}

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
    fn test_from_date_error_invalid_date() {
        let date_err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let time_err: TimeError = date_err.into();
        assert!(matches!(
            time_err,
            TimeError::InvalidDate {
                year: 2024,
                month: 2,
                day: 30
            }
        ));
    }

    #[test]
    fn test_from_date_error_parse_error() {
        let date_err = DateError::ParseError("invalid".to_string());
        let time_err: TimeError = date_err.into();
        assert!(matches!(time_err, TimeError::ParseError(msg) if msg == "invalid"));
    }
}

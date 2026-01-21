//! Master data errors.

use thiserror::Error;

/// Errors that can occur when accessing master data.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MasterDataError {
    /// Calendar not found
    #[error("Calendar not found: {0}")]
    CalendarNotFound(String),

    /// Invalid date
    #[error("Invalid date: {0}")]
    InvalidDate(String),

    /// Invalid ISIN
    #[error("Invalid ISIN: {0}")]
    InvalidIsin(String),

    /// CounterParty module error
    #[error("CounterParty error: {0}")]
    CounterParty(String),
}

/// Date-related errors.
///
/// Provides structured error handling for date construction and parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `InvalidDate`: Invalid date components (e.g., February 30th)
/// - `ParseError`: Failed to parse date string
///
/// # Examples
/// ```
/// use infra_master::DateError;
///
/// let err = DateError::InvalidDate { year: 2024, month: 2, day: 30 };
/// assert_eq!(format!("{}", err), "Invalid date: 2024-02-30");
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DateError {
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
}

/// Currency-related errors.
///
/// Provides structured error handling for currency parsing
/// with descriptive context for each failure mode.
///
/// # Variants
/// - `UnknownCurrency`: Unknown currency code
/// - `ParseError`: Failed to parse currency string
/// - `SameCurrency`: Base and quote currencies are the same
/// - `InvalidSpotRate`: Spot rate is not positive
///
/// # Examples
/// ```
/// use infra_master::CurrencyError;
///
/// let err = CurrencyError::UnknownCurrency("XYZ".to_string());
/// assert_eq!(format!("{}", err), "Unknown currency: XYZ");
///
/// let err = CurrencyError::ParseError("invalid format".to_string());
/// assert_eq!(format!("{}", err), "Currency parse error: invalid format");
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// Unknown currency code.
    #[error("Unknown currency: {0}")]
    UnknownCurrency(String),

    /// Failed to parse currency string.
    #[error("Currency parse error: {0}")]
    ParseError(String),

    /// Base and quote currencies are the same.
    #[error("Base and quote currencies are the same: {0}")]
    SameCurrency(String),

    /// Spot rate is not positive.
    #[error("Invalid spot rate: must be positive")]
    InvalidSpotRate,
}

/// Convert DateError to MasterDataError.
impl From<DateError> for MasterDataError {
    fn from(err: DateError) -> Self {
        MasterDataError::InvalidDate(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_error_invalid_date_display() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        assert_eq!(format!("{}", err), "Invalid date: 2024-02-30");
    }

    #[test]
    fn test_date_error_parse_error_display() {
        let err = DateError::ParseError("invalid format".to_string());
        assert_eq!(format!("{}", err), "Date parse error: invalid format");
    }

    #[test]
    fn test_date_error_trait_implementation() {
        let err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_date_error_clone_and_equality() {
        let err1 = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_currency_error_unknown_currency_display() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        assert_eq!(format!("{}", err), "Unknown currency: XYZ");
    }

    #[test]
    fn test_currency_error_trait_implementation() {
        let err = CurrencyError::UnknownCurrency("XYZ".to_string());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_currency_error_clone_and_equality() {
        let err1 = CurrencyError::UnknownCurrency("XYZ".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_currency_error_parse_error_display() {
        let err = CurrencyError::ParseError("invalid format".to_string());
        assert_eq!(format!("{}", err), "Currency parse error: invalid format");
    }

    #[test]
    fn test_currency_error_same_currency_display() {
        let err = CurrencyError::SameCurrency("USD".to_string());
        assert_eq!(
            format!("{}", err),
            "Base and quote currencies are the same: USD"
        );
    }

    #[test]
    fn test_currency_error_invalid_spot_rate_display() {
        let err = CurrencyError::InvalidSpotRate;
        assert_eq!(format!("{}", err), "Invalid spot rate: must be positive");
    }

    #[test]
    fn test_master_data_error_from_date_error() {
        let date_err = DateError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        let master_err: MasterDataError = date_err.into();
        assert!(matches!(master_err, MasterDataError::InvalidDate(_)));
        assert!(format!("{}", master_err).contains("2024-02-30"));
    }

    #[test]
    fn test_master_data_error_clone_and_equality() {
        let err1 = MasterDataError::CalendarNotFound("TARGET".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}

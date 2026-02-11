//! Master data errors.

use thiserror::Error;

/// Errors that can occur when accessing master data.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MasterDataError {
    /// Calendar not found.
    #[error("Calendar not found: {0}")]
    CalendarNotFound(String),

    /// Invalid date.
    #[error("Invalid date: {0}")]
    InvalidDate(String),

    /// Invalid ISIN.
    #[error("Invalid ISIN: {0}")]
    InvalidIsin(String),

    /// CounterParty module error.
    #[error("CounterParty error: {0}")]
    CounterParty(String),
}

/// Date-related errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DateError {
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
}

/// Currency-related errors.
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
    fn from(err: DateError) -> Self { MasterDataError::InvalidDate(err.to_string()) }
}

/// Errors that can occur in Book operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BookError {
    /// Duplicate BookId in collection.
    #[error("Duplicate BookId: {0}")]
    DuplicateId(String),

    /// Invalid ownership configuration.
    #[error("Invalid ownership: {0}")]
    InvalidOwnership(String),

    /// Invalid book type.
    #[error("Invalid book type: {0}")]
    InvalidType(String),

    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
}

/// Errors that can occur in Portfolio operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    /// Duplicate PortfolioId in collection.
    #[error("Duplicate PortfolioId: {0}")]
    DuplicateId(String),

    /// Circular reference in portfolio hierarchy.
    #[error("Circular portfolio reference detected: {0} -> {1}")]
    CircularReference(String, String),

    /// Invalid book reference (book does not exist).
    #[error("Invalid book reference: {0}")]
    InvalidBookReference(String),

    /// Invalid portfolio scope.
    #[error("Invalid portfolio scope: {0}")]
    InvalidScope(String),
}

/// Errors that can occur in Netting operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NettingError {
    /// Counterparty mismatch in netting set.
    #[error("Counterparty mismatch in netting set: expected {expected}, got {actual}")]
    CounterpartyMismatch {
        /// Expected counterparty ID.
        expected: String,
        /// Actual counterparty ID.
        actual: String,
    },

    /// Netting is not enforceable in jurisdiction.
    #[error("Netting not enforceable in jurisdiction: {0}")]
    NotEnforceable(String),

    /// Invalid netting agreement configuration.
    #[error("Invalid netting agreement: {0}")]
    InvalidAgreement(String),

    /// Cross-book netting violation (books must be explicitly allowed for.
    #[error("Cross-book netting violation: {0}")]
    CrossBookViolation(String),
}

/// Errors that can occur in Exposure calculations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ExposureError {
    /// Missing exposure data for a specific date.
    #[error("Missing exposure data for date: {0}")]
    MissingDate(String),

    /// Currency mismatch between expected and actual.
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        /// Expected currency.
        expected: String,
        /// Actual currency.
        actual: String,
    },

    /// Invalid time grid configuration.
    #[error("Invalid time grid: {0}")]
    InvalidTimeGrid(String),
}

/// Unified validation error type that wraps domain-specific errors.
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    /// Book-related validation error.
    #[error("Book error: {0}")]
    Book(#[from] BookError),

    /// Portfolio-related validation error.
    #[error("Portfolio error: {0}")]
    Portfolio(#[from] PortfolioError),

    /// Netting-related validation error.
    #[error("Netting error: {0}")]
    Netting(#[from] NettingError),

    /// Exposure-related validation error.
    #[error("Exposure error: {0}")]
    Exposure(#[from] ExposureError),

    /// Multiple validation errors collected together.
    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<ValidationError>),
}

/// Result type alias for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

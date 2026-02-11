//! Market quote representation.
//!
//! This module provides the [`MarketQuote`] type for representing a single
//! market quote with metadata.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{
//!     MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_domain::time::Tenor;
//!
//! let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let quote = MarketQuote::new(
//!     quote_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000, // Unix milliseconds
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! assert_eq!(quote.value, 0.05);
//! ```

use super::{error::MarketQuoteError, quote_id::QuoteId, quote_type::QuoteType};
use crate::market::source::DataSource;

/// A single market quote with metadata.
///
/// Represents an immutable market quote containing:
/// - The quote identifier ([`QuoteId`])
/// - Quote type (bid/ask/mid/last)
/// - Quote value
/// - Timestamp (Unix milliseconds)
/// - Data source
///
/// # Construction
///
/// Use [`MarketQuote::new`] to create a new quote. The constructor validates
/// that the quote value is finite (not NaN or Infinite).
///
/// # Examples
///
/// ```
/// use infra_domain::market::{
///     MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_domain::time::Tenor;
///
/// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
///
/// // Create a valid quote
/// let quote = MarketQuote::new(
///     quote_id.clone(),
///     QuoteType::Mid,
///     0.05,
///     1700000000000,
///     DataSource::Bloomberg,
/// ).unwrap();
///
/// // Invalid values are rejected
/// let invalid = MarketQuote::new(
///     quote_id,
///     QuoteType::Mid,
///     f64::NAN,
///     1700000000000,
///     DataSource::Bloomberg,
/// );
/// assert!(invalid.is_err());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketQuote {
    /// Quote identifier.
    pub id: QuoteId,
    /// Quote type (bid/ask/mid/last).
    pub quote_type: QuoteType,
    /// Quote value.
    pub value: f64,
    /// Timestamp in Unix milliseconds.
    pub timestamp: i64,
    /// Data source.
    pub source: DataSource,
}

impl MarketQuote {
    /// Creates a new `MarketQuote`.
    ///
    /// Validates that the value is finite (not NaN or Infinite).
    ///
    /// # Arguments
    ///
    /// * `id` - The quote identifier
    /// * `quote_type` - Type of quote (bid/ask/mid/last)
    /// * `value` - The quote value
    /// * `timestamp` - Unix timestamp in milliseconds
    /// * `source` - Data source
    ///
    /// # Errors
    ///
    /// Returns [`MarketQuoteError::InvalidQuote`] if the value is NaN or
    /// Infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(
    ///     quote_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    /// ```
    pub fn new(
        id: QuoteId,
        quote_type: QuoteType,
        value: f64,
        timestamp: i64,
        source: DataSource,
    ) -> Result<Self, MarketQuoteError> {
        // Validate value
        if value.is_nan() {
            return Err(MarketQuoteError::nan());
        }
        if value.is_infinite() {
            return Err(MarketQuoteError::infinite(value));
        }

        Ok(Self {
            id,
            quote_type,
            value,
            timestamp,
            source,
        })
    }

    /// Returns a new `MarketQuote` with a different timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The new timestamp in Unix milliseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(
    ///     quote_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    ///
    /// let updated = quote.with_timestamp(1700000001000);
    /// assert_eq!(updated.timestamp, 1700000001000);
    /// ```
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Returns a new `MarketQuote` with a different data source.
    ///
    /// # Arguments
    ///
    /// * `source` - The new data source
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{
    ///     MarketQuote, QuoteId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_domain::time::Tenor;
    ///
    /// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let quote = MarketQuote::new(
    ///     quote_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    ///
    /// let updated = quote.with_source(DataSource::Reuters);
    /// assert_eq!(updated.source, DataSource::Reuters);
    /// ```
    #[must_use]
    pub fn with_source(mut self, source: DataSource) -> Self {
        self.source = source;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::{Currency, RateType},
        time::Tenor,
    };

    fn test_quote_id() -> QuoteId {
        QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit)
    }

    #[test]
    fn test_construction_and_validation() {
        let q = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();
        assert!((q.value - 0.05).abs() < f64::EPSILON);
        assert_eq!(q.quote_type, QuoteType::Mid);

        // Zero and negative rates are valid
        assert!(MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.0,
            0,
            DataSource::Bloomberg
        )
        .is_ok());
        assert!(MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            -0.005,
            0,
            DataSource::Bloomberg
        )
        .is_ok());

        // NaN and Infinite are rejected
        assert!(MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::NAN,
            0,
            DataSource::Bloomberg
        )
        .is_err());
        assert!(MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::INFINITY,
            0,
            DataSource::Bloomberg
        )
        .is_err());
        assert!(MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::NEG_INFINITY,
            0,
            DataSource::Bloomberg
        )
        .is_err());
    }

    #[test]
    fn test_builder_methods() {
        let q = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1000,
            DataSource::Bloomberg,
        )
        .unwrap()
        .with_timestamp(2000)
        .with_source(DataSource::Reuters);
        assert_eq!(q.timestamp, 2000);
        assert_eq!(q.source, DataSource::Reuters);
    }
}

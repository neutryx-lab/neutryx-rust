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

use crate::market::source::DataSource;

use super::{error::MarketQuoteError, quote_type::QuoteType, quote_id::QuoteId};

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

/// Type alias for backward compatibility.
#[deprecated(since = "0.2.0", note = "Use MarketQuote instead")]
pub type MarketRate = MarketQuote;

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
    fn test_market_quote_new_valid() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_ok());
        let quote = quote.unwrap();
        assert!((quote.value - 0.05).abs() < f64::EPSILON);
        assert_eq!(quote.quote_type, QuoteType::Mid);
        assert_eq!(quote.timestamp, 1700000000000);
        assert_eq!(quote.source, DataSource::Bloomberg);
    }

    #[test]
    fn test_market_quote_new_nan() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::NAN,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_err());
        match quote {
            Err(MarketQuoteError::InvalidQuote { reason, .. }) => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidQuote error"),
        }
    }

    #[test]
    fn test_market_quote_new_positive_infinity() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::INFINITY,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_err());
        match quote {
            Err(MarketQuoteError::InvalidQuote { value, reason }) => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidQuote error"),
        }
    }

    #[test]
    fn test_market_quote_new_negative_infinity() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            f64::NEG_INFINITY,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_err());
        match quote {
            Err(MarketQuoteError::InvalidQuote { value, reason }) => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidQuote error"),
        }
    }

    #[test]
    fn test_market_quote_new_zero() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.0,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_ok());
        assert!((quote.unwrap().value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_quote_new_negative() {
        // Negative rates are valid (e.g., negative interest rates)
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            -0.005,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(quote.is_ok());
        assert!((quote.unwrap().value - (-0.005)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_quote_with_timestamp() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let updated = quote.with_timestamp(1700000001000);
        assert_eq!(updated.timestamp, 1700000001000);
        // Value should be preserved
        assert!((updated.value - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_quote_with_source() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let updated = quote.with_source(DataSource::Reuters);
        assert_eq!(updated.source, DataSource::Reuters);
        // Value should be preserved
        assert!((updated.value - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_quote_clone() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Reuters,
        )
        .unwrap();

        let cloned = quote.clone();
        assert_eq!(quote, cloned);
    }

    #[test]
    fn test_market_quote_eq() {
        let quote1 = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let quote2 = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let quote3 = MarketQuote::new(
            test_quote_id(),
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        assert_eq!(quote1, quote2);
        assert_ne!(quote1, quote3);
    }

    #[test]
    fn test_market_quote_debug() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let debug_str = format!("{:?}", quote);
        assert!(debug_str.contains("MarketQuote"));
        assert!(debug_str.contains("Mid"));
        assert!(debug_str.contains("Bloomberg"));
    }

    #[test]
    fn test_market_quote_all_quote_types() {
        for quote_type in [
            QuoteType::Bid,
            QuoteType::Ask,
            QuoteType::Mid,
            QuoteType::Last,
        ] {
            let quote = MarketQuote::new(
                test_quote_id(),
                quote_type,
                0.05,
                1700000000000,
                DataSource::Bloomberg,
            );
            assert!(quote.is_ok());
            assert_eq!(quote.unwrap().quote_type, quote_type);
        }
    }

    #[test]
    fn test_market_quote_all_data_sources() {
        for source in [
            DataSource::Bloomberg,
            DataSource::Reuters,
            DataSource::Internal,
            DataSource::Manual,
        ] {
            let quote = MarketQuote::new(test_quote_id(), QuoteType::Mid, 0.05, 1700000000000, source);
            assert!(quote.is_ok());
            assert_eq!(quote.unwrap().source, source);
        }
    }

    #[test]
    fn test_market_quote_builder_chain() {
        let quote = MarketQuote::new(
            test_quote_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
        .with_timestamp(1700000001000)
        .with_source(DataSource::Reuters);

        assert_eq!(quote.timestamp, 1700000001000);
        assert_eq!(quote.source, DataSource::Reuters);
    }
}

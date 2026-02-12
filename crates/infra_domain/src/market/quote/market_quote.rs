//! Market quote representation.

use super::{error::MarketQuoteError, quote_id::QuoteId, quote_type::QuoteType};
use crate::market::source::DataSource;

/// A single market quote with metadata.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn new(
        id: QuoteId,
        quote_type: QuoteType,
        value: f64,
        timestamp: i64,
        source: DataSource,
    ) -> Result<Self, MarketQuoteError> {
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
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Returns a new `MarketQuote` with a different data source.
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
        market::{Currency, QuoteCategory},
        time::Tenor,
    };

    fn test_quote_id() -> QuoteId {
        QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit)
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

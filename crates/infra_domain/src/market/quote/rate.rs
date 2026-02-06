//! Market rate representation.
//!
//! This module provides the [`MarketRate`] type for representing a single
//! market rate quote with metadata.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{
//!     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::Tenor;
//!
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     rate_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000, // Unix milliseconds
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! assert_eq!(rate.value, 0.05);
//! ```

use crate::market::source::DataSource;

use super::{error::MarketRateError, quote_type::QuoteType, rate_id::RateId};

/// A single market rate quote with metadata.
///
/// Represents an immutable market quote containing:
/// - The rate identifier ([`RateId`])
/// - Quote type (bid/ask/mid/last)
/// - Rate value
/// - Timestamp (Unix milliseconds)
/// - Data source
///
/// # Construction
///
/// Use [`MarketRate::new`] to create a new rate. The constructor validates
/// that the rate value is finite (not NaN or Infinite).
///
/// # Examples
///
/// ```
/// use infra_master::market::{
///     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_master::time::Tenor;
///
/// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
///
/// // Create a valid rate
/// let rate = MarketRate::new(
///     rate_id.clone(),
///     QuoteType::Mid,
///     0.05,
///     1700000000000,
///     DataSource::Bloomberg,
/// ).unwrap();
///
/// // Invalid values are rejected
/// let invalid = MarketRate::new(
///     rate_id,
///     QuoteType::Mid,
///     f64::NAN,
///     1700000000000,
///     DataSource::Bloomberg,
/// );
/// assert!(invalid.is_err());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketRate {
    /// Rate identifier.
    pub id: RateId,
    /// Quote type (bid/ask/mid/last).
    pub quote_type: QuoteType,
    /// Rate value.
    pub value: f64,
    /// Timestamp in Unix milliseconds.
    pub timestamp: i64,
    /// Data source.
    pub source: DataSource,
}

impl MarketRate {
    /// Creates a new `MarketRate`.
    ///
    /// Validates that the value is finite (not NaN or Infinite).
    ///
    /// # Arguments
    ///
    /// * `id` - The rate identifier
    /// * `quote_type` - Type of quote (bid/ask/mid/last)
    /// * `value` - The rate value
    /// * `timestamp` - Unix timestamp in milliseconds
    /// * `source` - Data source
    ///
    /// # Errors
    ///
    /// Returns [`MarketRateError::InvalidRate`] if the value is NaN or
    /// Infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(
    ///     rate_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    /// ```
    pub fn new(
        id: RateId,
        quote_type: QuoteType,
        value: f64,
        timestamp: i64,
        source: DataSource,
    ) -> Result<Self, MarketRateError> {
        // Validate value
        if value.is_nan() {
            return Err(MarketRateError::nan());
        }
        if value.is_infinite() {
            return Err(MarketRateError::infinite(value));
        }

        Ok(Self {
            id,
            quote_type,
            value,
            timestamp,
            source,
        })
    }

    /// Returns a new `MarketRate` with a different timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The new timestamp in Unix milliseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(
    ///     rate_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    ///
    /// let updated = rate.with_timestamp(1700000001000);
    /// assert_eq!(updated.timestamp, 1700000001000);
    /// ```
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Returns a new `MarketRate` with a different data source.
    ///
    /// # Arguments
    ///
    /// * `source` - The new data source
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::{
    ///     MarketRate, RateId, RateType, QuoteType, DataSource, Currency
    /// };
    /// use infra_master::time::Tenor;
    ///
    /// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
    /// let rate = MarketRate::new(
    ///     rate_id,
    ///     QuoteType::Mid,
    ///     0.05,
    ///     1700000000000,
    ///     DataSource::Bloomberg,
    /// ).unwrap();
    ///
    /// let updated = rate.with_source(DataSource::Reuters);
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

    fn test_rate_id() -> RateId {
        RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit)
    }

    #[test]
    fn test_market_rate_new_valid() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_ok());
        let rate = rate.unwrap();
        assert!((rate.value - 0.05).abs() < f64::EPSILON);
        assert_eq!(rate.quote_type, QuoteType::Mid);
        assert_eq!(rate.timestamp, 1700000000000);
        assert_eq!(rate.source, DataSource::Bloomberg);
    }

    #[test]
    fn test_market_rate_new_nan() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            f64::NAN,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_err());
        match rate {
            Err(MarketRateError::InvalidRate { reason, .. }) => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_market_rate_new_positive_infinity() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            f64::INFINITY,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_err());
        match rate {
            Err(MarketRateError::InvalidRate { value, reason }) => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_market_rate_new_negative_infinity() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            f64::NEG_INFINITY,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_err());
        match rate {
            Err(MarketRateError::InvalidRate { value, reason }) => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_market_rate_new_zero() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.0,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_ok());
        assert!((rate.unwrap().value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_rate_new_negative() {
        // Negative rates are valid (e.g., negative interest rates)
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            -0.005,
            1700000000000,
            DataSource::Bloomberg,
        );

        assert!(rate.is_ok());
        assert!((rate.unwrap().value - (-0.005)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_rate_with_timestamp() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let updated = rate.with_timestamp(1700000001000);
        assert_eq!(updated.timestamp, 1700000001000);
        // Value should be preserved
        assert!((updated.value - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_rate_with_source() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let updated = rate.with_source(DataSource::Reuters);
        assert_eq!(updated.source, DataSource::Reuters);
        // Value should be preserved
        assert!((updated.value - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_market_rate_clone() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Reuters,
        )
        .unwrap();

        let cloned = rate.clone();
        assert_eq!(rate, cloned);
    }

    #[test]
    fn test_market_rate_eq() {
        let rate1 = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let rate2 = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let rate3 = MarketRate::new(
            test_rate_id(),
            QuoteType::Bid,
            0.049,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        assert_eq!(rate1, rate2);
        assert_ne!(rate1, rate3);
    }

    #[test]
    fn test_market_rate_debug() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap();

        let debug_str = format!("{:?}", rate);
        assert!(debug_str.contains("MarketRate"));
        assert!(debug_str.contains("Mid"));
        assert!(debug_str.contains("Bloomberg"));
    }

    #[test]
    fn test_market_rate_all_quote_types() {
        for quote_type in [
            QuoteType::Bid,
            QuoteType::Ask,
            QuoteType::Mid,
            QuoteType::Last,
        ] {
            let rate = MarketRate::new(
                test_rate_id(),
                quote_type,
                0.05,
                1700000000000,
                DataSource::Bloomberg,
            );
            assert!(rate.is_ok());
            assert_eq!(rate.unwrap().quote_type, quote_type);
        }
    }

    #[test]
    fn test_market_rate_all_data_sources() {
        for source in [
            DataSource::Bloomberg,
            DataSource::Reuters,
            DataSource::Internal,
            DataSource::Manual,
        ] {
            let rate = MarketRate::new(test_rate_id(), QuoteType::Mid, 0.05, 1700000000000, source);
            assert!(rate.is_ok());
            assert_eq!(rate.unwrap().source, source);
        }
    }

    #[test]
    fn test_market_rate_builder_chain() {
        let rate = MarketRate::new(
            test_rate_id(),
            QuoteType::Mid,
            0.05,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
        .with_timestamp(1700000001000)
        .with_source(DataSource::Reuters);

        assert_eq!(rate.timestamp, 1700000001000);
        assert_eq!(rate.source, DataSource::Reuters);
    }
}

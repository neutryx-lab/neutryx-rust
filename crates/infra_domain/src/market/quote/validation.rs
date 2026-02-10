//! Quote validation traits and implementations.
//!
//! This module provides the [`QuoteValidator`] trait and
//! [`StandardQuoteValidator`] implementation for validating market quote
//! values.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{
//!     QuoteValidator, StandardQuoteValidator, MarketQuote,
//!     QuoteId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_domain::time::Tenor;
//!
//! let validator = StandardQuoteValidator::default();
//! let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let quote = MarketQuote::new(
//!     quote_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! assert!(validator.validate(&quote).is_ok());
//! ```

use super::{error::MarketQuoteError, market_quote::MarketQuote};
use crate::market::core::RateType;

/// Trait for validating market quotes.
///
/// Implementations of this trait define custom validation logic
/// for market quote values beyond basic NaN/Infinite checks.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{QuoteValidator, StandardQuoteValidator, MarketQuoteError};
///
/// struct StrictValidator;
///
/// impl QuoteValidator for StrictValidator {
///     fn validate(&self, quote: &infra_domain::market::MarketQuote) -> Result<(), MarketQuoteError> {
///         // Custom validation logic
///         if quote.value < 0.0 {
///             return Err(MarketQuoteError::ValidationFailed(
///                 "Negative quotes not allowed".to_string()
///             ));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait QuoteValidator {
    /// Validates a market quote.
    ///
    /// # Arguments
    ///
    /// * `quote` - The market quote to validate
    ///
    /// # Errors
    ///
    /// Returns [`MarketQuoteError`] if validation fails.
    fn validate(&self, quote: &MarketQuote) -> Result<(), MarketQuoteError>;
}

/// Standard quote validator with reasonable default bounds.
///
/// Validates quotes based on their type:
/// - Interest rates: -10% to 100% (-0.10 to 1.00)
/// - FX rates: 0.0001 to 100,000
/// - Volatility: 0% to 500% (0.0 to 5.0)
///
/// # Examples
///
/// ```
/// use infra_domain::market::{
///     QuoteValidator, StandardQuoteValidator, MarketQuote,
///     QuoteId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_domain::time::Tenor;
///
/// let validator = StandardQuoteValidator::default();
///
/// // Valid interest rate
/// let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
/// let quote = MarketQuote::new(
///     quote_id,
///     QuoteType::Mid,
///     0.05,  // 5% - within bounds
///     1700000000000,
///     DataSource::Bloomberg,
/// ).unwrap();
///
/// assert!(validator.validate(&quote).is_ok());
/// ```
#[derive(Debug, Clone, Default)]
pub struct StandardQuoteValidator;

impl StandardQuoteValidator {
    /// Minimum allowed interest rate (-10%).
    pub const MIN_INTEREST_RATE: f64 = -0.10;

    /// Maximum allowed interest rate (100%).
    pub const MAX_INTEREST_RATE: f64 = 1.00;

    /// Minimum allowed FX rate.
    pub const MIN_FX_RATE: f64 = 0.0001;

    /// Maximum allowed FX rate.
    pub const MAX_FX_RATE: f64 = 100_000.0;

    /// Minimum allowed volatility (0%).
    pub const MIN_VOLATILITY: f64 = 0.0;

    /// Maximum allowed volatility (500%).
    pub const MAX_VOLATILITY: f64 = 5.0;

    /// Creates a new `StandardQuoteValidator`.
    #[must_use]
    pub fn new() -> Self { Self }

    /// Validates the basic properties of a quote value.
    ///
    /// Checks for NaN and Infinite values.
    fn validate_basic(&self, value: f64) -> Result<(), MarketQuoteError> {
        if value.is_nan() {
            return Err(MarketQuoteError::nan());
        }
        if value.is_infinite() {
            return Err(MarketQuoteError::infinite(value));
        }
        Ok(())
    }

    /// Validates an interest rate value.
    fn validate_interest_rate(&self, value: f64) -> Result<(), MarketQuoteError> {
        self.validate_basic(value)?;

        if value < Self::MIN_INTEREST_RATE || value > Self::MAX_INTEREST_RATE {
            return Err(MarketQuoteError::out_of_bounds(
                value,
                Self::MIN_INTEREST_RATE,
                Self::MAX_INTEREST_RATE,
            ));
        }
        Ok(())
    }

    /// Validates an FX rate value.
    fn validate_fx_rate(&self, value: f64) -> Result<(), MarketQuoteError> {
        self.validate_basic(value)?;

        if value < Self::MIN_FX_RATE || value > Self::MAX_FX_RATE {
            return Err(MarketQuoteError::out_of_bounds(
                value,
                Self::MIN_FX_RATE,
                Self::MAX_FX_RATE,
            ));
        }
        Ok(())
    }

    /// Validates a volatility value.
    fn validate_volatility(&self, value: f64) -> Result<(), MarketQuoteError> {
        self.validate_basic(value)?;

        if value < Self::MIN_VOLATILITY || value > Self::MAX_VOLATILITY {
            return Err(MarketQuoteError::out_of_bounds(
                value,
                Self::MIN_VOLATILITY,
                Self::MAX_VOLATILITY,
            ));
        }
        Ok(())
    }
}

impl QuoteValidator for StandardQuoteValidator {
    fn validate(&self, quote: &MarketQuote) -> Result<(), MarketQuoteError> {
        let value = quote.value;
        let rate_type = quote.id.rate_type;

        match rate_type {
            // Interest rate types
            RateType::Deposit
            | RateType::Fra
            | RateType::Futures
            | RateType::Swap
            | RateType::Ois
            | RateType::BasisSwap
            | RateType::Event => self.validate_interest_rate(value),

            // FX types
            RateType::FxSpot | RateType::FxForward => self.validate_fx_rate(value),

            // Volatility
            RateType::Vol => self.validate_volatility(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::{Currency, DataSource, QuoteId, QuoteType},
        time::Tenor,
    };

    fn make_quote(rate_type: RateType, value: f64) -> MarketQuote {
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, rate_type);
        MarketQuote::new(
            id,
            QuoteType::Mid,
            value,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }

    fn make_quote_unchecked(rate_type: RateType, value: f64) -> MarketQuote {
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, rate_type);
        MarketQuote {
            id,
            quote_type: QuoteType::Mid,
            value,
            timestamp: 1700000000000,
            source: DataSource::Bloomberg,
        }
    }

    #[test]
    fn test_interest_rate_bounds() {
        let v = StandardQuoteValidator::default();
        for rt in [
            RateType::Deposit,
            RateType::Fra,
            RateType::Futures,
            RateType::Swap,
            RateType::Ois,
            RateType::BasisSwap,
        ] {
            assert!(
                v.validate(&make_quote(rt, 0.05)).is_ok(),
                "Failed for {:?}",
                rt
            );
        }
        // Boundary values
        assert!(v.validate(&make_quote(RateType::Swap, -0.10)).is_ok());
        assert!(v.validate(&make_quote(RateType::Swap, 1.0)).is_ok());
        assert!(v.validate(&make_quote(RateType::Swap, 0.0)).is_ok());
        assert!(v.validate(&make_quote(RateType::Swap, -0.005)).is_ok());
        // Out of bounds
        assert!(v.validate(&make_quote(RateType::Swap, -0.15)).is_err());
        assert!(v.validate(&make_quote(RateType::Swap, 1.5)).is_err());
    }

    #[test]
    fn test_fx_and_vol_bounds() {
        let v = StandardQuoteValidator::default();
        // FX valid
        assert!(v.validate(&make_quote(RateType::FxSpot, 1.2345)).is_ok());
        assert!(v.validate(&make_quote(RateType::FxForward, 1.2345)).is_ok());
        assert!(v.validate(&make_quote(RateType::FxSpot, 0.0001)).is_ok());
        assert!(v.validate(&make_quote(RateType::FxSpot, 100_000.0)).is_ok());
        // FX out of bounds
        assert!(v.validate(&make_quote(RateType::FxSpot, 0.00001)).is_err());
        assert!(v
            .validate(&make_quote(RateType::FxSpot, 200_000.0))
            .is_err());
        // Vol valid
        assert!(v.validate(&make_quote(RateType::Vol, 0.20)).is_ok());
        assert!(v.validate(&make_quote(RateType::Vol, 0.0)).is_ok());
        assert!(v.validate(&make_quote(RateType::Vol, 5.0)).is_ok());
        // Vol out of bounds
        assert!(v
            .validate(&make_quote_unchecked(RateType::Vol, -0.01))
            .is_err());
        assert!(v.validate(&make_quote(RateType::Vol, 6.0)).is_err());
    }

    #[test]
    fn test_nan_and_infinity() {
        let v = StandardQuoteValidator::default();
        assert!(v
            .validate(&make_quote_unchecked(RateType::Swap, f64::NAN))
            .is_err());
        assert!(v
            .validate(&make_quote_unchecked(RateType::Swap, f64::INFINITY))
            .is_err());
        assert!(v
            .validate(&make_quote_unchecked(RateType::Swap, f64::NEG_INFINITY))
            .is_err());
    }
}

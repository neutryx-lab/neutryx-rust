//! Quote validation traits and implementations.

use super::{error::MarketQuoteError, market_quote::MarketQuote};
use crate::market::core::QuoteCategory;

/// Trait for validating market quotes.
pub trait QuoteValidator {
    /// Validates a market quote.
    fn validate(&self, quote: &MarketQuote) -> Result<(), MarketQuoteError>;
}

/// Standard quote validator with reasonable default bounds.
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
        let quote_category = quote.id.quote_category;

        match quote_category {
            QuoteCategory::Deposit
            | QuoteCategory::Fra
            | QuoteCategory::Futures
            | QuoteCategory::Swap
            | QuoteCategory::Ois
            | QuoteCategory::BasisSwap
            | QuoteCategory::Event => self.validate_interest_rate(value),

            QuoteCategory::FxSpot | QuoteCategory::FxForward => self.validate_fx_rate(value),

            QuoteCategory::Vol => self.validate_volatility(value),

            QuoteCategory::Bond | QuoteCategory::CreditSpread => self.validate_interest_rate(value),
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

    fn make_quote(quote_category: QuoteCategory, value: f64) -> MarketQuote {
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, quote_category);
        MarketQuote::new(
            id,
            QuoteType::Mid,
            value,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }

    fn make_quote_unchecked(quote_category: QuoteCategory, value: f64) -> MarketQuote {
        let id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, quote_category);
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
            QuoteCategory::Deposit,
            QuoteCategory::Fra,
            QuoteCategory::Futures,
            QuoteCategory::Swap,
            QuoteCategory::Ois,
            QuoteCategory::BasisSwap,
        ] {
            assert!(
                v.validate(&make_quote(rt, 0.05)).is_ok(),
                "Failed for {:?}",
                rt
            );
        }
        assert!(v.validate(&make_quote(QuoteCategory::Swap, -0.10)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Swap, 1.0)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Swap, 0.0)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Swap, -0.005)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Swap, -0.15)).is_err());
        assert!(v.validate(&make_quote(QuoteCategory::Swap, 1.5)).is_err());
    }

    #[test]
    fn test_fx_and_vol_bounds() {
        let v = StandardQuoteValidator::default();
        assert!(v
            .validate(&make_quote(QuoteCategory::FxSpot, 1.2345))
            .is_ok());
        assert!(v
            .validate(&make_quote(QuoteCategory::FxForward, 1.2345))
            .is_ok());
        assert!(v
            .validate(&make_quote(QuoteCategory::FxSpot, 0.0001))
            .is_ok());
        assert!(v
            .validate(&make_quote(QuoteCategory::FxSpot, 100_000.0))
            .is_ok());
        assert!(v
            .validate(&make_quote(QuoteCategory::FxSpot, 0.00001))
            .is_err());
        assert!(v
            .validate(&make_quote(QuoteCategory::FxSpot, 200_000.0))
            .is_err());
        assert!(v.validate(&make_quote(QuoteCategory::Vol, 0.20)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Vol, 0.0)).is_ok());
        assert!(v.validate(&make_quote(QuoteCategory::Vol, 5.0)).is_ok());
        assert!(v
            .validate(&make_quote_unchecked(QuoteCategory::Vol, -0.01))
            .is_err());
        assert!(v.validate(&make_quote(QuoteCategory::Vol, 6.0)).is_err());
    }

    #[test]
    fn test_nan_and_infinity() {
        let v = StandardQuoteValidator::default();
        assert!(v
            .validate(&make_quote_unchecked(QuoteCategory::Swap, f64::NAN))
            .is_err());
        assert!(v
            .validate(&make_quote_unchecked(QuoteCategory::Swap, f64::INFINITY))
            .is_err());
        assert!(v
            .validate(&make_quote_unchecked(
                QuoteCategory::Swap,
                f64::NEG_INFINITY
            ))
            .is_err());
    }
}

//! Instrument mapping for market quotes.

use crate::{
    market::{
        core::QuoteCategory,
        quote::{MarketQuote, MarketQuoteError},
    },
    time::{Date, EndOfMonthRule},
    trade::Instrument,
};

/// Trait for mapping market quotes to instruments.
pub trait InstrumentMapper {
    /// Maps a market quote to an instrument.
    fn map_to_instrument(
        &self,
        quote: &MarketQuote,
        valuation_date: Date,
    ) -> Result<Instrument, MarketQuoteError>;
}

/// Standard instrument mapper with default conventions.
#[derive(Debug, Clone)]
pub struct StandardInstrumentMapper {
    /// End of month rule for date calculations.
    eom_rule: EndOfMonthRule,
    /// Settlement lag in business days (default: 2).
    settlement_lag: u32,
}

impl Default for StandardInstrumentMapper {
    fn default() -> Self { Self::new() }
}

impl StandardInstrumentMapper {
    /// Creates a new `StandardInstrumentMapper` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            eom_rule: EndOfMonthRule::Adjust,
            settlement_lag: 2,
        }
    }

    /// Creates a mapper with a custom end of month rule.
    #[must_use]
    pub fn with_eom_rule(eom_rule: EndOfMonthRule) -> Self {
        Self {
            eom_rule,
            settlement_lag: 2,
        }
    }

    /// Creates a mapper with a custom settlement lag.
    #[must_use]
    pub fn with_settlement_lag(mut self, settlement_lag: u32) -> Self {
        self.settlement_lag = settlement_lag;
        self
    }

    /// Calculates the start date (spot date) from valuation date.
    fn spot_date(&self, valuation_date: Date) -> Date {
        use crate::time::Period;
        valuation_date + Period::days(self.settlement_lag as i32)
    }

    /// Maps a deposit rate to a Deposit instrument.
    fn map_deposit(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Deposit {
            currency: quote.id.currency,
            start_date,
            tenor: quote.id.tenor,
            rate: quote.value,
        }
    }

    /// Maps a FRA rate to a Fra instrument.
    fn map_fra(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Fra {
            currency: quote.id.currency,
            start_date,
            tenor: quote.id.tenor,
            rate: quote.value,
        }
    }

    /// Maps a futures rate to a Futures instrument.
    fn map_futures(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let expiry = quote
            .id
            .tenor
            .add_to_date(self.spot_date(valuation_date), self.eom_rule);

        let price = 100.0 - quote.value * 100.0;

        Instrument::Futures {
            currency: quote.id.currency,
            expiry,
            price,
        }
    }

    /// Maps a swap rate to a ParSwap instrument.
    fn map_swap(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::ParSwap {
            currency: quote.id.currency,
            start_date,
            tenor: quote.id.tenor,
            rate: quote.value,
        }
    }

    /// Maps an OIS rate to an Ois instrument.
    fn map_ois(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Ois {
            currency: quote.id.currency,
            start_date,
            tenor: quote.id.tenor,
            rate: quote.value,
        }
    }

    /// Maps a basis swap rate to a BasisSwap instrument.
    fn map_basis_swap(&self, quote: &MarketQuote, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::BasisSwap {
            currency: quote.id.currency,
            start_date,
            tenor: quote.id.tenor,
            spread: quote.value,
        }
    }
}

impl InstrumentMapper for StandardInstrumentMapper {
    fn map_to_instrument(
        &self,
        quote: &MarketQuote,
        valuation_date: Date,
    ) -> Result<Instrument, MarketQuoteError> {
        match quote.id.quote_category {
            QuoteCategory::Deposit => Ok(self.map_deposit(quote, valuation_date)),
            QuoteCategory::Fra => Ok(self.map_fra(quote, valuation_date)),
            QuoteCategory::Futures => Ok(self.map_futures(quote, valuation_date)),
            QuoteCategory::Swap => Ok(self.map_swap(quote, valuation_date)),
            QuoteCategory::Ois => Ok(self.map_ois(quote, valuation_date)),
            QuoteCategory::BasisSwap => Ok(self.map_basis_swap(quote, valuation_date)),
            QuoteCategory::FxSpot | QuoteCategory::FxForward | QuoteCategory::Vol | QuoteCategory::Event => {
                Err(MarketQuoteError::unsupported_quote_category(quote.id.quote_category))
            }
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

    fn q(rt: QuoteCategory, t: Tenor, v: f64) -> MarketQuote {
        MarketQuote::new(
            QuoteId::new(Currency::USD, t, rt),
            QuoteType::Mid,
            v,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }
    fn vd() -> Date { Date::from_ymd(2024, 1, 15).unwrap() }
    fn spot() -> Date { Date::from_ymd(2024, 1, 17).unwrap() }

    #[test]
    fn test_mapper_construction() {
        let m = StandardInstrumentMapper::new();
        assert_eq!(m.settlement_lag, 2);
        assert_eq!(m.eom_rule, EndOfMonthRule::Adjust);
        assert_eq!(StandardInstrumentMapper::default().settlement_lag, 2);
        assert_eq!(
            StandardInstrumentMapper::with_eom_rule(EndOfMonthRule::Preserve).eom_rule,
            EndOfMonthRule::Preserve
        );
        assert_eq!(
            StandardInstrumentMapper::new()
                .with_settlement_lag(1)
                .settlement_lag,
            1
        );
        assert_eq!(m.clone().settlement_lag, 2);
        assert!(format!("{:?}", m).contains("StandardInstrumentMapper"));
    }

    #[test]
    fn test_mapper_instrument_types() {
        let m = StandardInstrumentMapper::new();
        let v = vd();

        match m
            .map_to_instrument(&q(QuoteCategory::Deposit, Tenor::ThreeMonths, 0.05), v)
            .unwrap()
        {
            Instrument::Deposit {
                currency,
                start_date,
                tenor,
                rate,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, spot());
                assert_eq!(tenor, Tenor::ThreeMonths);
                assert!((rate - 0.05).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Deposit"),
        }
        match m
            .map_to_instrument(&q(QuoteCategory::Fra, Tenor::SixMonths, 0.055), v)
            .unwrap()
        {
            Instrument::Fra {
                currency,
                start_date,
                tenor,
                rate,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, spot());
                assert_eq!(tenor, Tenor::SixMonths);
                assert!((rate - 0.055).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Fra"),
        }
        match m
            .map_to_instrument(&q(QuoteCategory::Futures, Tenor::ThreeMonths, 0.045), v)
            .unwrap()
        {
            Instrument::Futures {
                currency,
                expiry,
                price,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(expiry, Date::from_ymd(2024, 4, 17).unwrap());
                assert!((price - 95.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Futures"),
        }
        match m
            .map_to_instrument(&q(QuoteCategory::Swap, Tenor::FiveYears, 0.04), v)
            .unwrap()
        {
            Instrument::ParSwap {
                currency,
                start_date,
                tenor,
                rate,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, spot());
                assert_eq!(tenor, Tenor::FiveYears);
                assert!((rate - 0.04).abs() < f64::EPSILON);
            }
            _ => panic!("Expected ParSwap"),
        }
        match m
            .map_to_instrument(&q(QuoteCategory::Ois, Tenor::OneYear, 0.035), v)
            .unwrap()
        {
            Instrument::Ois {
                currency,
                start_date,
                tenor,
                rate,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, spot());
                assert_eq!(tenor, Tenor::OneYear);
                assert!((rate - 0.035).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Ois"),
        }
        match m
            .map_to_instrument(&q(QuoteCategory::BasisSwap, Tenor::TenYears, 0.0025), v)
            .unwrap()
        {
            Instrument::BasisSwap {
                currency,
                start_date,
                tenor,
                spread,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, spot());
                assert_eq!(tenor, Tenor::TenYears);
                assert!((spread - 0.0025).abs() < f64::EPSILON);
            }
            _ => panic!("Expected BasisSwap"),
        }
        assert!(matches!(
            m.map_to_instrument(&q(QuoteCategory::FxSpot, Tenor::TwoWeeks, 1.1), v),
            Err(MarketQuoteError::MappingFailed {
                quote_category: QuoteCategory::FxSpot,
                ..
            })
        ));
        assert!(m
            .map_to_instrument(&q(QuoteCategory::FxForward, Tenor::ThreeMonths, 1.1), v)
            .is_err());
        assert!(m
            .map_to_instrument(&q(QuoteCategory::Vol, Tenor::OneYear, 0.2), v)
            .is_err());
    }

    #[test]
    fn test_mapper_edge_cases() {
        let v = vd();

        let m1 = StandardInstrumentMapper::new().with_settlement_lag(1);
        match m1
            .map_to_instrument(&q(QuoteCategory::Deposit, Tenor::OneMonth, 0.05), v)
            .unwrap()
        {
            Instrument::Deposit { start_date, .. } => {
                assert_eq!(start_date, Date::from_ymd(2024, 1, 16).unwrap())
            }
            _ => panic!("Expected Deposit"),
        }

        let m = StandardInstrumentMapper::new();
        assert!(m
            .map_to_instrument(&q(QuoteCategory::Deposit, Tenor::ThreeMonths, 0.05), v)
            .is_ok());
        assert!(m
            .map_to_instrument(&q(QuoteCategory::Swap, Tenor::FiveYears, 0.045), v)
            .is_ok());
        assert!(m
            .map_to_instrument(&q(QuoteCategory::Ois, Tenor::OneYear, 0.04), v)
            .is_ok());

        for (rate_val, expected_price) in [(0.0, 100.0), (0.01, 99.0), (0.05, 95.0), (0.10, 90.0)] {
            match m
                .map_to_instrument(&q(QuoteCategory::Futures, Tenor::ThreeMonths, rate_val), v)
                .unwrap()
            {
                Instrument::Futures { price, .. } => {
                    assert!((price - expected_price).abs() < f64::EPSILON)
                }
                _ => panic!("Expected Futures"),
            }
        }

        let eur_q = MarketQuote::new(
            QuoteId::new(Currency::EUR, Tenor::TenYears, QuoteCategory::Swap),
            QuoteType::Mid,
            0.025,
            1700000000000,
            DataSource::Reuters,
        )
        .unwrap();
        match m.map_to_instrument(&eur_q, v).unwrap() {
            Instrument::ParSwap { currency, .. } => assert_eq!(currency, Currency::EUR),
            _ => panic!("Expected ParSwap"),
        }
    }
}

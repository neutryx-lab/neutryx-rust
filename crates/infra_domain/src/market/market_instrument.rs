//! Market instrument type for CF-expandable instruments.

use thiserror::Error;

use super::{convention::MarketConvention, Currency, QuoteCategory, QuoteId};
use crate::{
    time::{Date, Tenor},
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

/// Errors that can occur when creating or expanding market instruments.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MarketInstrumentError {
    /// The rate value is invalid.
    #[error("Invalid rate value: {value} ({reason})")]
    InvalidRate {
        /// The invalid value.
        value: f64,
        /// Reason for invalidity.
        reason: String,
    },

    /// No convention is available for the rate.
    #[error("No convention available for {quote_id}")]
    NoConvention {
        /// The rate ID that has no convention.
        quote_id: String,
    },

    /// Invalid date calculation.
    #[error("Invalid date: {reason}")]
    InvalidDate {
        /// Reason for the date being invalid.
        reason: String,
    },

    /// Expansion to trade failed.
    #[error("Trade expansion failed: {reason}")]
    ExpansionFailed {
        /// Reason for expansion failure.
        reason: String,
    },

    /// Unsupported convention for the operation.
    #[error("Unsupported convention: {convention_type} ({reason})")]
    UnsupportedConvention {
        /// The convention type.
        convention_type: String,
        /// Reason for being unsupported.
        reason: String,
    },
}

impl MarketInstrumentError {
    /// Creates an error for NaN rate value.
    #[must_use]
    pub fn nan() -> Self {
        Self::InvalidRate {
            value: f64::NAN,
            reason: "Value is NaN".to_string(),
        }
    }

    /// Creates an error for infinite rate value.
    #[must_use]
    pub fn infinite(value: f64) -> Self {
        Self::InvalidRate {
            value,
            reason: "Value is infinite".to_string(),
        }
    }

    /// Creates an error for missing convention.
    #[must_use]
    pub fn no_convention(quote_id: &QuoteId) -> Self {
        Self::NoConvention {
            quote_id: quote_id.to_string(),
        }
    }
}

/// A market instrument combining rate data with convention.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketInstrument {
    /// Unique identifier for the rate.
    pub quote_id: QuoteId,
    /// Rate value (e.g., 0.05 for 5%).
    pub rate_value: f64,
    /// Market convention for this instrument.
    pub convention: MarketConvention,
    /// Valuation date.
    pub valuation_date: Date,
    /// Effective date (start of instrument).
    pub effective_date: Date,
    /// Maturity date (end of instrument).
    pub maturity_date: Date,
    /// Notional amount.
    pub notional: f64,
}

impl MarketInstrument {
    /// Creates a new market instrument.
    pub fn new(
        quote_id: QuoteId,
        rate_value: f64,
        convention: MarketConvention,
        valuation_date: Date,
        notional: f64,
    ) -> Result<Self, MarketInstrumentError> {
        if rate_value.is_nan() {
            return Err(MarketInstrumentError::nan());
        }
        if rate_value.is_infinite() {
            return Err(MarketInstrumentError::infinite(rate_value));
        }

        let spot_lag = Self::get_spot_lag(&convention);
        let effective_date = valuation_date + spot_lag as i64;

        let maturity_date = effective_date + quote_id.tenor.to_period();

        Ok(Self {
            quote_id,
            rate_value,
            convention,
            valuation_date,
            effective_date,
            maturity_date,
            notional,
        })
    }

    /// Creates a market instrument with explicit dates.
    #[allow(clippy::too_many_arguments)]
    pub fn with_dates(
        quote_id: QuoteId,
        rate_value: f64,
        convention: MarketConvention,
        valuation_date: Date,
        effective_date: Date,
        maturity_date: Date,
        notional: f64,
    ) -> Result<Self, MarketInstrumentError> {
        if rate_value.is_nan() {
            return Err(MarketInstrumentError::nan());
        }
        if rate_value.is_infinite() {
            return Err(MarketInstrumentError::infinite(rate_value));
        }

        if maturity_date <= effective_date {
            return Err(MarketInstrumentError::InvalidDate {
                reason: format!(
                    "Maturity date ({}) must be after effective date ({})",
                    maturity_date, effective_date
                ),
            });
        }

        Ok(Self {
            quote_id,
            rate_value,
            convention,
            valuation_date,
            effective_date,
            maturity_date,
            notional,
        })
    }

    /// Returns the spot lag for the given convention.
    fn get_spot_lag(convention: &MarketConvention) -> u32 {
        match convention {
            MarketConvention::Deposit(c) => c.spot_lag,
            MarketConvention::Swap(c) | MarketConvention::Ois(c) => c.spot_lag,
            MarketConvention::Fra(_) => 2,
            MarketConvention::Futures(_) => 0,
            MarketConvention::XCcyBasis(c) => c.spot_lag,
            MarketConvention::FxForward(c) => c.spot_days,
            MarketConvention::FxSwap(c) => c.spot_days,
        }
    }

    /// Returns the currency of this instrument.
    #[must_use]
    pub fn currency(&self) -> Currency { self.quote_id.currency }

    /// Returns the tenor of this instrument.
    #[must_use]
    pub fn tenor(&self) -> Tenor { self.quote_id.tenor }

    /// Returns the quote category of this instrument.
    #[must_use]
    pub fn quote_category(&self) -> QuoteCategory { self.quote_id.quote_category }

    /// Returns the instrument type name.
    #[must_use]
    pub fn instrument_type_name(&self) -> &'static str { self.convention.instrument_type_name() }

    /// Returns the year fraction for the instrument period.
    #[must_use]
    pub fn year_fraction(&self) -> f64 {
        let days = (self.maturity_date - self.effective_date) as f64;
        days / 365.0
    }

    /// Returns whether this is a deposit instrument.
    #[must_use]
    pub fn is_deposit(&self) -> bool { self.convention.is_deposit() }

    /// Returns whether this is a swap instrument.
    #[must_use]
    pub fn is_swap(&self) -> bool { self.convention.is_swap() }

    /// Returns whether this is an OIS instrument.
    #[must_use]
    pub fn is_ois(&self) -> bool { self.convention.is_ois() }

    /// Converts this market instrument to a CF-expanded Trade.
    pub fn to_trade(&self) -> Result<Trade, MarketInstrumentError> {
        let trade_id = format!(
            "MI_{}_{}_{:?}",
            self.quote_id.currency.code(),
            self.quote_id.tenor.code(),
            self.quote_id.quote_category
        );

        match &self.convention {
            MarketConvention::Deposit(_) => self.expand_deposit(&trade_id),
            MarketConvention::Swap(_) | MarketConvention::Ois(_) => self.expand_swap(&trade_id),
            MarketConvention::Fra(_) => self.expand_fra(&trade_id),
            MarketConvention::Futures(_) => self.expand_futures(&trade_id),
            MarketConvention::XCcyBasis(_) => Err(MarketInstrumentError::UnsupportedConvention {
                convention_type: "XCcyBasis".to_string(),
                reason: "Cross-currency basis swap expansion not yet implemented".to_string(),
            }),
            MarketConvention::FxForward(_) => Err(MarketInstrumentError::UnsupportedConvention {
                convention_type: "FxForward".to_string(),
                reason: "FX forward expansion not yet implemented".to_string(),
            }),
            MarketConvention::FxSwap(_) => Err(MarketInstrumentError::UnsupportedConvention {
                convention_type: "FxSwap".to_string(),
                reason: "FX swap expansion not yet implemented".to_string(),
            }),
        }
    }

    /// Expands a deposit instrument to a trade.
    #[allow(clippy::unnecessary_wraps)]
    fn expand_deposit(&self, trade_id: &str) -> Result<Trade, MarketInstrumentError> {
        let cashflow = Cashflow::new(
            CashflowType::Coupon,
            self.maturity_date,
            self.effective_date,
            self.maturity_date,
            self.year_fraction(),
            self.notional,
            Payoff::Fixed {
                rate: self.rate_value,
            },
            self.currency(),
        );

        let leg = Leg::new(
            vec![cashflow],
            Direction::Receiver,
            LegType::Fixed,
            self.currency(),
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Deposit))
    }

    /// Expands a swap or OIS instrument to a trade.
    fn expand_swap(&self, trade_id: &str) -> Result<Trade, MarketInstrumentError> {
        let schedule = self.generate_annual_schedule();

        if schedule.len() < 2 {
            return Err(MarketInstrumentError::InvalidDate {
                reason: "Schedule must have at least 2 dates".to_string(),
            });
        }

        let mut fixed_cashflows = Vec::new();
        for window in schedule.windows(2) {
            let (start, end) = (window[0], window[1]);
            let yf = (end - start) as f64 / 365.0;

            fixed_cashflows.push(Cashflow::new(
                CashflowType::Coupon,
                end,
                start,
                end,
                yf,
                self.notional,
                Payoff::Fixed {
                    rate: self.rate_value,
                },
                self.currency(),
            ));
        }

        let fixed_leg = Leg::new(
            fixed_cashflows,
            Direction::Payer,
            LegType::Fixed,
            self.currency(),
        );

        let mut floating_cashflows = Vec::new();
        for window in schedule.windows(2) {
            let (start, end) = (window[0], window[1]);
            let yf = (end - start) as f64 / 365.0;

            let index = crate::trade::IndexType::Rate(crate::market::RateIndex::Sofr);

            floating_cashflows.push(Cashflow::new(
                CashflowType::Coupon,
                end,
                start,
                end,
                yf,
                self.notional,
                Payoff::Linear {
                    index,
                    spread: 0.0,
                    multiplier: 1.0,
                },
                self.currency(),
            ));
        }

        let floating_leg = Leg::new(
            floating_cashflows,
            Direction::Receiver,
            LegType::Floating,
            self.currency(),
        );

        let trade_type = TradeType::Swap;

        Ok(Trade::new(
            trade_id,
            vec![fixed_leg, floating_leg],
            trade_type,
        ))
    }

    /// Expands a FRA instrument to a trade.
    #[allow(clippy::unnecessary_wraps)]
    fn expand_fra(&self, trade_id: &str) -> Result<Trade, MarketInstrumentError> {
        let cashflow = Cashflow::new(
            CashflowType::Settlement,
            self.effective_date,
            self.effective_date,
            self.maturity_date,
            self.year_fraction(),
            self.notional,
            Payoff::Fixed {
                rate: self.rate_value,
            },
            self.currency(),
        );

        let leg = Leg::new(
            vec![cashflow],
            Direction::Receiver,
            LegType::Fixed,
            self.currency(),
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Fra))
    }

    /// Expands a futures instrument to a trade.
    #[allow(clippy::unnecessary_wraps)]
    fn expand_futures(&self, trade_id: &str) -> Result<Trade, MarketInstrumentError> {
        let cashflow = Cashflow::new(
            CashflowType::Settlement,
            self.maturity_date,
            self.effective_date,
            self.maturity_date,
            self.year_fraction(),
            self.notional,
            Payoff::Fixed {
                rate: self.rate_value,
            },
            self.currency(),
        );

        let leg = Leg::new(
            vec![cashflow],
            Direction::Receiver,
            LegType::Fixed,
            self.currency(),
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Futures))
    }

    /// Generates an annual schedule from effective to maturity date.
    fn generate_annual_schedule(&self) -> Vec<Date> {
        let mut schedule = vec![self.effective_date];
        let mut current = self.effective_date;

        while current < self.maturity_date {
            current = current + 365;
            if current <= self.maturity_date {
                schedule.push(current);
            }
        }

        if schedule.last() != Some(&self.maturity_date) {
            schedule.push(self.maturity_date);
        }

        schedule
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::convention::{DepositConvention, FraConvention, SwapConvention},
        trade::{LegType, Payoff, TradeType},
    };

    #[test]
    fn test_market_instrument_new_deposit() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument = MarketInstrument::new(
            quote_id.clone(),
            0.05,
            convention,
            valuation_date,
            1_000_000.0,
        )
        .unwrap();

        assert_eq!(instrument.quote_id, quote_id);
        assert_eq!(instrument.rate_value, 0.05);
        assert_eq!(instrument.notional, 1_000_000.0);
        assert!(instrument.effective_date > valuation_date);
        assert!(instrument.maturity_date > instrument.effective_date);
    }

    #[test]
    fn test_market_instrument_new_swap() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::FiveYears, QuoteCategory::Swap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Swap(SwapConvention::usd_sofr());

        let instrument = MarketInstrument::new(
            quote_id.clone(),
            0.045,
            convention,
            valuation_date,
            10_000_000.0,
        )
        .unwrap();

        assert_eq!(instrument.quote_id, quote_id);
        assert_eq!(instrument.rate_value, 0.045);
        assert!(instrument.is_swap());
        assert!(!instrument.is_deposit());
    }

    #[test]
    fn test_market_instrument_new_ois() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Ois);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Ois(SwapConvention::usd_sofr());

        let instrument = MarketInstrument::new(
            quote_id.clone(),
            0.052,
            convention,
            valuation_date,
            5_000_000.0,
        )
        .unwrap();

        assert!(instrument.is_ois());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_market_instrument_new_fra() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Fra);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Fra(FraConvention::usd_sofr());

        let instrument =
            MarketInstrument::new(quote_id, 0.051, convention, valuation_date, 2_000_000.0)
                .unwrap();

        assert_eq!(instrument.instrument_type_name(), "FRA");
    }

    #[test]
    fn test_market_instrument_invalid_nan() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let result =
            MarketInstrument::new(quote_id, f64::NAN, convention, valuation_date, 1_000_000.0);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketInstrumentError::InvalidRate { reason, .. } => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_market_instrument_invalid_infinite() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let result = MarketInstrument::new(
            quote_id,
            f64::INFINITY,
            convention,
            valuation_date,
            1_000_000.0,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketInstrumentError::InvalidRate { reason, .. } => {
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_market_instrument_with_dates() {
        let quote_id = QuoteId::new(Currency::EUR, Tenor::OneYear, QuoteCategory::Swap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let effective_date = Date::from_ymd(2024, 1, 17).unwrap();
        let maturity_date = Date::from_ymd(2025, 1, 17).unwrap();
        let convention = MarketConvention::Swap(SwapConvention::eur_euribor_6m());

        let instrument = MarketInstrument::with_dates(
            quote_id,
            0.035,
            convention,
            valuation_date,
            effective_date,
            maturity_date,
            5_000_000.0,
        )
        .unwrap();

        assert_eq!(instrument.effective_date, effective_date);
        assert_eq!(instrument.maturity_date, maturity_date);
    }

    #[test]
    fn test_market_instrument_with_dates_invalid_order() {
        let quote_id = QuoteId::new(Currency::EUR, Tenor::OneYear, QuoteCategory::Swap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let effective_date = Date::from_ymd(2025, 1, 17).unwrap();
        let maturity_date = Date::from_ymd(2024, 1, 17).unwrap();
        let convention = MarketConvention::Swap(SwapConvention::eur_euribor_6m());

        let result = MarketInstrument::with_dates(
            quote_id,
            0.035,
            convention,
            valuation_date,
            effective_date,
            maturity_date,
            5_000_000.0,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketInstrumentError::InvalidDate { reason } => {
                assert!(reason.contains("must be after"));
            }
            _ => panic!("Expected InvalidDate error"),
        }
    }

    #[test]
    fn test_market_instrument_currency() {
        let quote_id = QuoteId::new(Currency::GBP, Tenor::TenYears, QuoteCategory::Swap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Swap(SwapConvention::gbp_sonia());

        let instrument =
            MarketInstrument::new(quote_id, 0.04, convention, valuation_date, 10_000_000.0)
                .unwrap();

        assert_eq!(instrument.currency(), Currency::GBP);
    }

    #[test]
    fn test_market_instrument_tenor() {
        let quote_id = QuoteId::new(Currency::JPY, Tenor::TwoYears, QuoteCategory::Ois);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Ois(SwapConvention::jpy_tonar());

        let instrument =
            MarketInstrument::new(quote_id, 0.001, convention, valuation_date, 100_000_000.0)
                .unwrap();

        assert_eq!(instrument.tenor(), Tenor::TwoYears);
    }

    #[test]
    fn test_market_instrument_quote_category() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::SixMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.053, convention, valuation_date, 1_000_000.0)
                .unwrap();

        assert_eq!(instrument.quote_category(), QuoteCategory::Deposit);
    }

    #[test]
    fn test_market_instrument_year_fraction() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.05, convention, valuation_date, 1_000_000.0).unwrap();

        let yf = instrument.year_fraction();
        assert!(
            yf > 0.9 && yf < 1.1,
            "Year fraction should be ~1.0, got {}",
            yf
        );
    }

    #[test]
    fn test_market_instrument_clone() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.05, convention, valuation_date, 1_000_000.0).unwrap();
        let cloned = instrument.clone();

        assert_eq!(instrument, cloned);
    }

    #[test]
    fn test_market_instrument_debug() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.05, convention, valuation_date, 1_000_000.0).unwrap();
        let debug_str = format!("{:?}", instrument);

        assert!(debug_str.contains("MarketInstrument"));
        assert!(debug_str.contains("USD"));
    }

    #[test]
    fn test_market_instrument_error_no_convention() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Vol);
        let error = MarketInstrumentError::no_convention(&quote_id);

        match error {
            MarketInstrumentError::NoConvention { quote_id: id } => {
                assert!(id.contains("USD"));
            }
            _ => panic!("Expected NoConvention error"),
        }
    }

    #[test]
    fn test_to_trade_deposit() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.05, convention, valuation_date, 1_000_000.0).unwrap();

        let trade = instrument.to_trade().unwrap();

        let legs: Vec<_> = trade.legs().collect();
        assert_eq!(legs.len(), 1);
        assert!(matches!(trade.trade_type, TradeType::Deposit));

        let leg = legs[0];
        assert_eq!(leg.cashflows().count(), 1);
        assert!(matches!(leg.leg_type, LegType::Fixed));
    }

    #[test]
    fn test_to_trade_swap() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::FiveYears, QuoteCategory::Swap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Swap(SwapConvention::usd_sofr());

        let instrument =
            MarketInstrument::new(quote_id, 0.045, convention, valuation_date, 10_000_000.0)
                .unwrap();

        let trade = instrument.to_trade().unwrap();

        let legs: Vec<_> = trade.legs().collect();
        assert_eq!(legs.len(), 2);
        assert!(matches!(trade.trade_type, TradeType::Swap));

        let has_fixed = legs.iter().any(|l| matches!(l.leg_type, LegType::Fixed));
        let has_floating = legs.iter().any(|l| matches!(l.leg_type, LegType::Floating));
        assert!(has_fixed, "Should have fixed leg");
        assert!(has_floating, "Should have floating leg");
    }

    #[test]
    fn test_to_trade_ois() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::OneYear, QuoteCategory::Ois);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Ois(SwapConvention::usd_sofr());

        let instrument =
            MarketInstrument::new(quote_id, 0.052, convention, valuation_date, 5_000_000.0)
                .unwrap();

        let trade = instrument.to_trade().unwrap();

        assert_eq!(trade.legs().count(), 2);
    }

    #[test]
    fn test_to_trade_fra() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Fra);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Fra(FraConvention::usd_sofr());

        let instrument =
            MarketInstrument::new(quote_id, 0.051, convention, valuation_date, 2_000_000.0)
                .unwrap();

        let trade = instrument.to_trade().unwrap();

        assert_eq!(trade.legs().count(), 1);
        assert!(matches!(trade.trade_type, TradeType::Fra));
    }

    #[test]
    fn test_to_trade_unsupported_xccy() {
        use crate::market::convention::XCcyBasisConvention;

        let quote_id = QuoteId::new(Currency::USD, Tenor::FiveYears, QuoteCategory::BasisSwap);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy());

        let instrument =
            MarketInstrument::new(quote_id, 0.0025, convention, valuation_date, 100_000_000.0)
                .unwrap();

        let result = instrument.to_trade();
        assert!(result.is_err());

        match result.unwrap_err() {
            MarketInstrumentError::UnsupportedConvention {
                convention_type, ..
            } => {
                assert_eq!(convention_type, "XCcyBasis");
            }
            _ => panic!("Expected UnsupportedConvention error"),
        }
    }

    #[test]
    fn test_to_trade_cashflow_values() {
        let quote_id = QuoteId::new(Currency::USD, Tenor::ThreeMonths, QuoteCategory::Deposit);
        let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
        let convention = MarketConvention::Deposit(DepositConvention::usd());

        let instrument =
            MarketInstrument::new(quote_id, 0.05, convention, valuation_date, 1_000_000.0).unwrap();

        let trade = instrument.to_trade().unwrap();
        let legs: Vec<_> = trade.legs().collect();
        let cashflows: Vec<_> = legs[0].cashflows().collect();
        let cf = cashflows[0];

        assert_eq!(cf.notional, 1_000_000.0);
        assert!(cf.year_fraction > 0.0);
        assert!(matches!(cf.payoff, Payoff::Fixed { rate } if (rate - 0.05).abs() < 1e-10));
    }
}

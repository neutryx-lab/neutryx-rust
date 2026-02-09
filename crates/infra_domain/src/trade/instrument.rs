//! Market instrument definitions.
//!
//! This module provides types for standardised market instruments
//! used in curve calibration and trading.

use crate::{
    market::Currency,
    time::{Date, Tenor},
};

/// A market instrument used for curve calibration.
///
/// Represents standardised financial products with market quotes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Instrument {
    /// Money market deposit.
    Deposit {
        /// Currency of the deposit.
        currency: Currency,
        /// Start date.
        start_date: Date,
        /// Tenor of the deposit.
        tenor: Tenor,
        /// Quoted rate (as decimal).
        rate: f64,
    },

    /// Forward rate agreement.
    Fra {
        /// Currency.
        currency: Currency,
        /// Start date (fixing date).
        start_date: Date,
        /// FRA tenor (e.g., 3M for 3x6).
        tenor: Tenor,
        /// Quoted rate (as decimal).
        rate: f64,
    },

    /// Interest rate future.
    Futures {
        /// Currency.
        currency: Currency,
        /// Expiry date.
        expiry: Date,
        /// Contract price (e.g., 95.5 implies rate 4.5%).
        price: f64,
    },

    /// Par swap (vanilla IRS).
    ParSwap {
        /// Currency.
        currency: Currency,
        /// Effective date.
        start_date: Date,
        /// Swap tenor.
        tenor: Tenor,
        /// Par swap rate (as decimal).
        rate: f64,
    },

    /// Overnight index swap.
    Ois {
        /// Currency.
        currency: Currency,
        /// Effective date.
        start_date: Date,
        /// Swap tenor.
        tenor: Tenor,
        /// Par OIS rate (as decimal).
        rate: f64,
    },

    /// Basis swap (two floating legs).
    BasisSwap {
        /// Currency.
        currency: Currency,
        /// Effective date.
        start_date: Date,
        /// Swap tenor.
        tenor: Tenor,
        /// Basis spread (as decimal).
        spread: f64,
    },

    /// Cross-currency swap.
    CrossCurrencySwap {
        /// Pay currency.
        pay_currency: Currency,
        /// Receive currency.
        receive_currency: Currency,
        /// Effective date.
        start_date: Date,
        /// Swap tenor.
        tenor: Tenor,
        /// Cross-currency basis spread (as decimal).
        spread: f64,
    },
}

impl Instrument {
    /// Returns the market quote for this instrument.
    ///
    /// For futures, returns (100 - price) as the implied rate.
    #[must_use]
    pub fn quote(&self) -> f64 {
        match self {
            Instrument::Deposit { rate, .. } => *rate,
            Instrument::Fra { rate, .. } => *rate,
            Instrument::Futures { price, .. } => (100.0 - price) / 100.0,
            Instrument::ParSwap { rate, .. } => *rate,
            Instrument::Ois { rate, .. } => *rate,
            Instrument::BasisSwap { spread, .. } => *spread,
            Instrument::CrossCurrencySwap { spread, .. } => *spread,
        }
    }

    /// Returns the primary currency of this instrument.
    #[must_use]
    pub fn currency(&self) -> Currency {
        match self {
            Instrument::Deposit { currency, .. } => *currency,
            Instrument::Fra { currency, .. } => *currency,
            Instrument::Futures { currency, .. } => *currency,
            Instrument::ParSwap { currency, .. } => *currency,
            Instrument::Ois { currency, .. } => *currency,
            Instrument::BasisSwap { currency, .. } => *currency,
            Instrument::CrossCurrencySwap { pay_currency, .. } => *pay_currency,
        }
    }

    /// Returns the start date of this instrument.
    #[must_use]
    pub fn start_date(&self) -> Date {
        match self {
            Instrument::Deposit { start_date, .. } => *start_date,
            Instrument::Fra { start_date, .. } => *start_date,
            Instrument::Futures { expiry, .. } => *expiry,
            Instrument::ParSwap { start_date, .. } => *start_date,
            Instrument::Ois { start_date, .. } => *start_date,
            Instrument::BasisSwap { start_date, .. } => *start_date,
            Instrument::CrossCurrencySwap { start_date, .. } => *start_date,
        }
    }

    /// Returns true if this is a deposit instrument.
    #[must_use]
    pub fn is_deposit(&self) -> bool { matches!(self, Instrument::Deposit { .. }) }

    /// Returns true if this is a swap instrument.
    #[must_use]
    pub fn is_swap(&self) -> bool {
        matches!(
            self,
            Instrument::ParSwap { .. } | Instrument::Ois { .. } | Instrument::BasisSwap { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_quote() {
        let deposit = Instrument::Deposit {
            currency: Currency::USD,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::ThreeMonths,
            rate: 0.05,
        };

        assert_eq!(deposit.quote(), 0.05);
        assert_eq!(deposit.currency(), Currency::USD);
    }

    #[test]
    fn test_futures_quote() {
        let futures = Instrument::Futures {
            currency: Currency::USD,
            expiry: Date::from_ymd(2025, 3, 15).unwrap(),
            price: 95.5,
        };

        // 100 - 95.5 = 4.5%, so quote = 0.045
        assert!((futures.quote() - 0.045).abs() < 1e-10);
    }

    #[test]
    fn test_par_swap() {
        let swap = Instrument::ParSwap {
            currency: Currency::EUR,
            start_date: Date::from_ymd(2025, 1, 3).unwrap(),
            tenor: Tenor::FiveYears,
            rate: 0.025,
        };

        assert_eq!(swap.quote(), 0.025);
        assert!(swap.is_swap());
    }

    #[test]
    fn test_ois() {
        let ois = Instrument::Ois {
            currency: Currency::USD,
            start_date: Date::from_ymd(2025, 1, 3).unwrap(),
            tenor: Tenor::OneYear,
            rate: 0.045,
        };

        assert_eq!(ois.quote(), 0.045);
        assert!(ois.is_swap());
    }

    #[test]
    fn test_is_deposit() {
        let deposit = Instrument::Deposit {
            currency: Currency::USD,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::OneMonth,
            rate: 0.05,
        };

        assert!(deposit.is_deposit());
        assert!(!deposit.is_swap());
    }

    #[test]
    fn test_instrument_clone() {
        let deposit = Instrument::Deposit {
            currency: Currency::USD,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::ThreeMonths,
            rate: 0.05,
        };
        let cloned = deposit.clone();
        assert_eq!(deposit, cloned);
    }
}

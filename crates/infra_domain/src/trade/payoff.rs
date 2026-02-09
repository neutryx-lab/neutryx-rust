//! Payoff definitions for cashflow calculations.
//!
//! This module provides types for representing payoff formulas
//! used in financial instruments.

use super::index::IndexType;

/// Option type for vanilla options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum OptionType {
    /// Call option (right to buy).
    #[default]
    Call,
    /// Put option (right to sell).
    Put,
    /// Digital call - pays 1 if S > K, else 0.
    DigitalCall,
    /// Digital put - pays 1 if S < K, else 0.
    DigitalPut,
}

impl OptionType {
    /// Returns 1.0 for Call/DigitalCall, -1.0 for Put/DigitalPut.
    #[must_use]
    pub fn sign(&self) -> f64 {
        match self {
            OptionType::Call | OptionType::DigitalCall => 1.0,
            OptionType::Put | OptionType::DigitalPut => -1.0,
        }
    }

    /// Returns true if this is a call-like payoff.
    #[inline]
    #[must_use]
    pub fn is_call(&self) -> bool { matches!(self, OptionType::Call | OptionType::DigitalCall) }

    /// Returns true if this is a put-like payoff.
    #[inline]
    #[must_use]
    pub fn is_put(&self) -> bool { matches!(self, OptionType::Put | OptionType::DigitalPut) }

    /// Returns true if this is a digital option.
    #[inline]
    #[must_use]
    pub fn is_digital(&self) -> bool {
        matches!(self, OptionType::DigitalCall | OptionType::DigitalPut)
    }
}

/// Payoff formula for a cashflow.
///
/// Defines how the cashflow amount is calculated based on
/// market observations.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Payoff {
    /// Fixed rate payment.
    ///
    /// Amount = notional * rate * year_fraction
    Fixed {
        /// Fixed rate (as decimal, e.g., 0.05 for 5%).
        rate: f64,
    },

    /// Linear (floating) rate payment.
    ///
    /// Amount = notional * (index_rate + spread) * year_fraction
    Linear {
        /// Index to observe.
        index: IndexType,
        /// Spread over the index (as decimal).
        spread: f64,
        /// Multiplier (gearing), typically 1.0.
        multiplier: f64,
    },

    /// Vanilla option (Cap/Floor).
    ///
    /// Amount = notional * max(0, omega * (index_rate - strike)) *
    /// year_fraction where omega = +1 for Call (Cap), -1 for Put (Floor)
    VanillaOption {
        /// Index to observe.
        index: IndexType,
        /// Strike rate (as decimal).
        strike: f64,
        /// Call or Put.
        option_type: OptionType,
    },

    /// Digital option.
    ///
    /// Amount = notional * payout if index_rate > strike (Call) or < strike
    /// (Put)
    Digital {
        /// Index to observe.
        index: IndexType,
        /// Strike level.
        strike: f64,
        /// Call or Put.
        option_type: OptionType,
        /// Fixed payout amount (as a rate, e.g., 0.01 for 1%).
        payout: f64,
    },
}

impl Payoff {
    /// Creates a fixed rate payoff.
    #[must_use]
    pub fn fixed(rate: f64) -> Self { Payoff::Fixed { rate } }

    /// Creates a linear (floating) rate payoff with zero spread.
    #[must_use]
    pub fn floating(index: IndexType) -> Self {
        Payoff::Linear {
            index,
            spread: 0.0,
            multiplier: 1.0,
        }
    }

    /// Creates a linear payoff with a spread.
    #[must_use]
    pub fn floating_with_spread(index: IndexType, spread: f64) -> Self {
        Payoff::Linear {
            index,
            spread,
            multiplier: 1.0,
        }
    }

    /// Creates a cap payoff.
    #[must_use]
    pub fn cap(index: IndexType, strike: f64) -> Self {
        Payoff::VanillaOption {
            index,
            strike,
            option_type: OptionType::Call,
        }
    }

    /// Creates a floor payoff.
    #[must_use]
    pub fn floor(index: IndexType, strike: f64) -> Self {
        Payoff::VanillaOption {
            index,
            strike,
            option_type: OptionType::Put,
        }
    }

    /// Returns the index required for this payoff, if any.
    ///
    /// Returns `None` for fixed payoffs.
    #[must_use]
    pub fn required_index(&self) -> Option<&IndexType> {
        match self {
            Payoff::Fixed { .. } => None,
            Payoff::Linear { index, .. } => Some(index),
            Payoff::VanillaOption { index, .. } => Some(index),
            Payoff::Digital { index, .. } => Some(index),
        }
    }

    /// Returns true if this is a fixed rate payoff.
    #[must_use]
    pub fn is_fixed(&self) -> bool { matches!(self, Payoff::Fixed { .. }) }

    /// Returns true if this is a linear (floating) rate payoff.
    #[must_use]
    pub fn is_linear(&self) -> bool { matches!(self, Payoff::Linear { .. }) }

    /// Returns true if this is an option payoff (cap/floor or digital).
    #[must_use]
    pub fn is_option(&self) -> bool {
        matches!(self, Payoff::VanillaOption { .. } | Payoff::Digital { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::RateIndex;

    #[test]
    fn test_option_type_sign() {
        assert_eq!(OptionType::Call.sign(), 1.0);
        assert_eq!(OptionType::Put.sign(), -1.0);
    }

    #[test]
    fn test_payoff_fixed() {
        let payoff = Payoff::fixed(0.05);

        assert!(payoff.is_fixed());
        assert!(!payoff.is_linear());
        assert!(!payoff.is_option());
        assert!(payoff.required_index().is_none());
    }

    #[test]
    fn test_payoff_floating() {
        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));

        assert!(!payoff.is_fixed());
        assert!(payoff.is_linear());
        assert!(!payoff.is_option());
        assert!(payoff.required_index().is_some());
    }

    #[test]
    fn test_payoff_floating_with_spread() {
        let payoff = Payoff::floating_with_spread(IndexType::Rate(RateIndex::Euribor3M), 0.001);

        if let Payoff::Linear { spread, .. } = payoff {
            assert_eq!(spread, 0.001);
        } else {
            panic!("Expected Linear payoff");
        }
    }

    #[test]
    fn test_payoff_cap() {
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sofr), 0.03);

        assert!(payoff.is_option());
        if let Payoff::VanillaOption {
            option_type,
            strike,
            ..
        } = payoff
        {
            assert_eq!(option_type, OptionType::Call);
            assert_eq!(strike, 0.03);
        } else {
            panic!("Expected VanillaOption payoff");
        }
    }

    #[test]
    fn test_payoff_floor() {
        let payoff = Payoff::floor(IndexType::Rate(RateIndex::Sofr), 0.01);

        assert!(payoff.is_option());
        if let Payoff::VanillaOption {
            option_type,
            strike,
            ..
        } = payoff
        {
            assert_eq!(option_type, OptionType::Put);
            assert_eq!(strike, 0.01);
        } else {
            panic!("Expected VanillaOption payoff");
        }
    }

    #[test]
    fn test_payoff_digital() {
        let payoff = Payoff::Digital {
            index: IndexType::Rate(RateIndex::Sofr),
            strike: 0.02,
            option_type: OptionType::Call,
            payout: 0.01,
        };

        assert!(payoff.is_option());
        assert!(!payoff.is_fixed());
        assert!(!payoff.is_linear());
        assert!(payoff.required_index().is_some());
    }

    #[test]
    fn test_required_index_fixed() {
        let payoff = Payoff::fixed(0.05);
        assert!(payoff.required_index().is_none());
    }

    #[test]
    fn test_required_index_linear() {
        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Tonar));

        let index = payoff.required_index().unwrap();
        assert!(matches!(index, IndexType::Rate(RateIndex::Tonar)));
    }

    #[test]
    fn test_required_index_option() {
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sonia), 0.025);

        let index = payoff.required_index().unwrap();
        assert!(matches!(index, IndexType::Rate(RateIndex::Sonia)));
    }

    #[test]
    fn test_payoff_clone() {
        let payoff = Payoff::fixed(0.04);
        let cloned = payoff.clone();
        assert_eq!(payoff, cloned);
    }

    #[test]
    fn test_payoff_debug() {
        let payoff = Payoff::fixed(0.05);
        let debug = format!("{:?}", payoff);
        assert!(debug.contains("Fixed"));
        assert!(debug.contains("0.05"));
    }

    #[test]
    fn test_option_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(OptionType::Call);
        set.insert(OptionType::Put);
        set.insert(OptionType::Call); // Duplicate
        assert_eq!(set.len(), 2);
    }
}

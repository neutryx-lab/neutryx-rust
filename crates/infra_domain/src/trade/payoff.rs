//! Payoff definitions for cashflow calculations.

use super::index::IndexType;

/// Option type for vanilla options.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Payoff {
    /// Fixed rate payment.
    Fixed {
        /// Fixed rate (as decimal, e.g., 0.05 for 5%).
        rate: f64,
    },

    /// Linear (floating) rate payment.
    Linear {
        /// Index to observe.
        index: IndexType,
        /// Spread over the index (as decimal).
        spread: f64,
        /// Multiplier (gearing), typically 1.0.
        multiplier: f64,
    },

    /// Vanilla option (Cap/Floor).
    VanillaOption {
        /// Index to observe.
        index: IndexType,
        /// Strike rate (as decimal).
        strike: f64,
        /// Call or Put.
        option_type: OptionType,
    },

    /// Digital option.
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

    /// Reciprocal (inverse) floating rate: multiplier / index + spread.
    Reciprocal {
        /// Index to observe.
        index: IndexType,
        /// Multiplier (numerator).
        multiplier: f64,
        /// Additive spread.
        spread: f64,
    },

    /// Spread between two indices: multiplier1 * index1 - multiplier2 * index2
    /// + spread.
    Spread {
        /// First index.
        index1: IndexType,
        /// Second index.
        index2: IndexType,
        /// Multiplier for first index.
        multiplier1: f64,
        /// Multiplier for second index.
        multiplier2: f64,
        /// Additive spread.
        spread: f64,
    },

    /// Capped/floored spread option.
    SpreadCap {
        /// First index.
        index1: IndexType,
        /// Second index.
        index2: IndexType,
        /// Multiplier for first index.
        multiplier1: f64,
        /// Multiplier for second index.
        multiplier2: f64,
        /// Additive spread.
        spread: f64,
        /// Call or Put.
        option_type: OptionType,
    },

    /// Product of two indices: multiplier * index1 * index2.
    Product {
        /// First index.
        index1: IndexType,
        /// Second index.
        index2: IndexType,
        /// Multiplier.
        multiplier: f64,
    },

    /// Quotient of two indices: multiplier * index1 / index2.
    Quotient {
        /// First index (numerator).
        index1: IndexType,
        /// Second index (denominator).
        index2: IndexType,
        /// Multiplier.
        multiplier: f64,
    },

    /// Average of sub-period observations.
    Average {
        /// Index to observe.
        index: IndexType,
        /// Spread over the averaged rate.
        spread: f64,
    },

    /// Capped average rate option.
    AverageCap {
        /// Index to observe.
        index: IndexType,
        /// Strike rate.
        strike: f64,
        /// Call or Put.
        option_type: OptionType,
    },

    /// Credit default swap payoff: (1 - recovery_rate) on credit event.
    Cds {
        /// Expected recovery rate (e.g., 0.4 for 40%).
        recovery_rate: f64,
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

    /// Creates a spread payoff between two indices.
    #[must_use]
    pub fn spread(index1: IndexType, index2: IndexType, spread: f64) -> Self {
        Payoff::Spread {
            index1,
            index2,
            multiplier1: 1.0,
            multiplier2: 1.0,
            spread,
        }
    }

    /// Creates a product payoff.
    #[must_use]
    pub fn product(index1: IndexType, index2: IndexType) -> Self {
        Payoff::Product {
            index1,
            index2,
            multiplier: 1.0,
        }
    }

    /// Creates a quotient payoff.
    #[must_use]
    pub fn quotient(index1: IndexType, index2: IndexType) -> Self {
        Payoff::Quotient {
            index1,
            index2,
            multiplier: 1.0,
        }
    }

    /// Creates a CDS payoff.
    #[must_use]
    pub fn cds(recovery_rate: f64) -> Self { Payoff::Cds { recovery_rate } }

    /// Creates an average rate payoff.
    #[must_use]
    pub fn average(index: IndexType, spread: f64) -> Self { Payoff::Average { index, spread } }

    /// Returns the index required for this payoff, if any.
    ///
    /// For multi-index payoffs, returns the first index. Use
    /// [`required_indices`](Self::required_indices) to obtain all indices.
    #[must_use]
    pub fn required_index(&self) -> Option<&IndexType> {
        match self {
            Payoff::Fixed { .. } | Payoff::Cds { .. } => None,
            Payoff::Linear { index, .. }
            | Payoff::VanillaOption { index, .. }
            | Payoff::Digital { index, .. }
            | Payoff::Reciprocal { index, .. }
            | Payoff::Average { index, .. }
            | Payoff::AverageCap { index, .. } => Some(index),
            Payoff::Spread { index1, .. }
            | Payoff::SpreadCap { index1, .. }
            | Payoff::Product { index1, .. }
            | Payoff::Quotient { index1, .. } => Some(index1),
        }
    }

    /// Returns all indices required for this payoff.
    #[must_use]
    pub fn required_indices(&self) -> Vec<&IndexType> {
        match self {
            Payoff::Fixed { .. } | Payoff::Cds { .. } => vec![],
            Payoff::Linear { index, .. }
            | Payoff::VanillaOption { index, .. }
            | Payoff::Digital { index, .. }
            | Payoff::Reciprocal { index, .. }
            | Payoff::Average { index, .. }
            | Payoff::AverageCap { index, .. } => vec![index],
            Payoff::Spread { index1, index2, .. }
            | Payoff::SpreadCap { index1, index2, .. }
            | Payoff::Product { index1, index2, .. }
            | Payoff::Quotient { index1, index2, .. } => vec![index1, index2],
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
        matches!(
            self,
            Payoff::VanillaOption { .. }
                | Payoff::Digital { .. }
                | Payoff::SpreadCap { .. }
                | Payoff::AverageCap { .. }
        )
    }

    /// Returns true if this is a spread payoff.
    #[must_use]
    pub fn is_spread(&self) -> bool {
        matches!(self, Payoff::Spread { .. } | Payoff::SpreadCap { .. })
    }

    /// Returns true if this is a multi-index payoff.
    #[must_use]
    pub fn is_multi_index(&self) -> bool {
        matches!(
            self,
            Payoff::Spread { .. }
                | Payoff::SpreadCap { .. }
                | Payoff::Product { .. }
                | Payoff::Quotient { .. }
        )
    }

    /// Returns true if this is a CDS payoff.
    #[must_use]
    pub fn is_cds(&self) -> bool { matches!(self, Payoff::Cds { .. }) }
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
        set.insert(OptionType::Call);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_payoff_reciprocal() {
        let payoff = Payoff::Reciprocal {
            index: IndexType::Rate(RateIndex::Sofr),
            multiplier: 1.0,
            spread: 0.005,
        };

        assert!(!payoff.is_fixed());
        assert!(!payoff.is_linear());
        assert!(!payoff.is_option());
        assert!(!payoff.is_multi_index());
        assert!(payoff.required_index().is_some());
        assert_eq!(payoff.required_indices().len(), 1);
    }

    #[test]
    fn test_payoff_spread_factory() {
        let idx1 = IndexType::Rate(RateIndex::Sofr);
        let idx2 = IndexType::Rate(RateIndex::Sonia);
        let payoff = Payoff::spread(idx1, idx2, 0.002);

        assert!(payoff.is_spread());
        assert!(payoff.is_multi_index());
        assert!(!payoff.is_option());
        assert_eq!(payoff.required_indices().len(), 2);

        if let Payoff::Spread {
            multiplier1,
            multiplier2,
            spread,
            ..
        } = &payoff
        {
            assert_eq!(*multiplier1, 1.0);
            assert_eq!(*multiplier2, 1.0);
            assert_eq!(*spread, 0.002);
        } else {
            panic!("Expected Spread payoff");
        }
    }

    #[test]
    fn test_payoff_spread_cap() {
        let payoff = Payoff::SpreadCap {
            index1: IndexType::Rate(RateIndex::Sofr),
            index2: IndexType::Rate(RateIndex::Sonia),
            multiplier1: 1.0,
            multiplier2: 1.0,
            spread: 0.0,
            option_type: OptionType::Call,
        };

        assert!(payoff.is_spread());
        assert!(payoff.is_multi_index());
        assert!(payoff.is_option());
        assert_eq!(payoff.required_indices().len(), 2);
        // required_index returns the first index
        assert!(payoff.required_index().is_some());
    }

    #[test]
    fn test_payoff_product_factory() {
        let idx1 = IndexType::Rate(RateIndex::Sofr);
        let idx2 = IndexType::Fx {
            base: "EUR".into(),
            quote: "USD".into(),
        };
        let payoff = Payoff::product(idx1, idx2);

        assert!(payoff.is_multi_index());
        assert!(!payoff.is_spread());
        assert!(!payoff.is_option());
        assert_eq!(payoff.required_indices().len(), 2);

        if let Payoff::Product { multiplier, .. } = &payoff {
            assert_eq!(*multiplier, 1.0);
        } else {
            panic!("Expected Product payoff");
        }
    }

    #[test]
    fn test_payoff_quotient_factory() {
        let idx1 = IndexType::Fx {
            base: "EUR".into(),
            quote: "USD".into(),
        };
        let idx2 = IndexType::Fx {
            base: "GBP".into(),
            quote: "USD".into(),
        };
        let payoff = Payoff::quotient(idx1, idx2);

        assert!(payoff.is_multi_index());
        assert!(!payoff.is_spread());
        assert_eq!(payoff.required_indices().len(), 2);

        if let Payoff::Quotient { multiplier, .. } = &payoff {
            assert_eq!(*multiplier, 1.0);
        } else {
            panic!("Expected Quotient payoff");
        }
    }

    #[test]
    fn test_payoff_average_factory() {
        let payoff = Payoff::average(IndexType::Rate(RateIndex::Sofr), 0.001);

        assert!(!payoff.is_fixed());
        assert!(!payoff.is_option());
        assert!(!payoff.is_multi_index());
        assert_eq!(payoff.required_indices().len(), 1);

        if let Payoff::Average { spread, .. } = &payoff {
            assert_eq!(*spread, 0.001);
        } else {
            panic!("Expected Average payoff");
        }
    }

    #[test]
    fn test_payoff_average_cap() {
        let payoff = Payoff::AverageCap {
            index: IndexType::Rate(RateIndex::Sofr),
            strike: 0.04,
            option_type: OptionType::Call,
        };

        assert!(payoff.is_option());
        assert!(!payoff.is_multi_index());
        assert_eq!(payoff.required_indices().len(), 1);
    }

    #[test]
    fn test_payoff_cds_factory() {
        let payoff = Payoff::cds(0.4);

        assert!(payoff.is_cds());
        assert!(!payoff.is_fixed());
        assert!(!payoff.is_option());
        assert!(!payoff.is_multi_index());
        assert!(payoff.required_index().is_none());
        assert!(payoff.required_indices().is_empty());

        if let Payoff::Cds { recovery_rate } = &payoff {
            assert_eq!(*recovery_rate, 0.4);
        } else {
            panic!("Expected Cds payoff");
        }
    }

    #[test]
    fn test_required_indices_fixed() {
        let payoff = Payoff::fixed(0.05);
        assert!(payoff.required_indices().is_empty());
    }

    #[test]
    fn test_required_indices_linear() {
        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let indices = payoff.required_indices();
        assert_eq!(indices.len(), 1);
        assert!(matches!(indices[0], IndexType::Rate(RateIndex::Sofr)));
    }

    #[test]
    fn test_required_indices_spread() {
        let payoff = Payoff::spread(
            IndexType::Rate(RateIndex::Sofr),
            IndexType::Rate(RateIndex::Sonia),
            0.0,
        );
        let indices = payoff.required_indices();
        assert_eq!(indices.len(), 2);
        assert!(matches!(indices[0], IndexType::Rate(RateIndex::Sofr)));
        assert!(matches!(indices[1], IndexType::Rate(RateIndex::Sonia)));
    }
}

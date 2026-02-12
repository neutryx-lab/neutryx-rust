//! Credit rating and parameters for XVA calculations.

#![allow(clippy::must_use_candidate)]

use super::CounterPartyError;

/// Credit rating with +/- notches (20 grades).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize, strum::Display)]
pub enum CreditRating {
    /// AAA - Highest quality.
    #[strum(serialize = "AAA")]
    Aaa,
    /// AA+ - High quality.
    #[strum(serialize = "AA+")]
    AaPlus,
    /// AA - High quality.
    #[strum(serialize = "AA")]
    Aa,
    /// AA- - High quality.
    #[strum(serialize = "AA-")]
    AaMinus,
    /// A+ - Upper medium grade.
    #[strum(serialize = "A+")]
    APlus,
    /// A - Upper medium grade.
    A,
    /// A- - Upper medium grade.
    #[strum(serialize = "A-")]
    AMinus,
    /// BBB+ - Lower medium grade (investment grade boundary).
    #[strum(serialize = "BBB+")]
    BbbPlus,
    /// BBB - Lower medium grade.
    #[strum(serialize = "BBB")]
    Bbb,
    /// BBB- - Lowest investment grade.
    #[strum(serialize = "BBB-")]
    BbbMinus,
    /// BB+ - Non-investment grade speculative.
    #[strum(serialize = "BB+")]
    BbPlus,
    /// BB - Non-investment grade speculative.
    #[strum(serialize = "BB")]
    Bb,
    /// BB- - Non-investment grade speculative.
    #[strum(serialize = "BB-")]
    BbMinus,
    /// B+ - Highly speculative.
    #[strum(serialize = "B+")]
    BPlus,
    /// B - Highly speculative.
    B,
    /// B- - Highly speculative.
    #[strum(serialize = "B-")]
    BMinus,
    /// CCC - Substantial risks.
    #[strum(serialize = "CCC")]
    Ccc,
    /// CC - Extremely speculative.
    #[strum(serialize = "CC")]
    Cc,
    /// C - Exceptionally high risk.
    C,
    /// D - In default.
    D,
}

impl CreditRating {
    /// Returns whether this rating is investment grade (BBB- or better).
    #[inline]
    pub fn is_investment_grade(&self) -> bool { *self <= CreditRating::BbbMinus }

    /// Returns indicative annual hazard rate for this rating.
    #[inline]
    pub fn indicative_hazard_rate(&self) -> f64 {
        match self {
            CreditRating::Aaa => 0.0001,
            CreditRating::AaPlus => 0.0003,
            CreditRating::Aa => 0.0005,
            CreditRating::AaMinus => 0.0007,
            CreditRating::APlus => 0.0008,
            CreditRating::A => 0.001,
            CreditRating::AMinus => 0.0012,
            CreditRating::BbbPlus => 0.0015,
            CreditRating::Bbb => 0.002,
            CreditRating::BbbMinus => 0.003,
            CreditRating::BbPlus => 0.005,
            CreditRating::Bb => 0.01,
            CreditRating::BbMinus => 0.015,
            CreditRating::BPlus => 0.02,
            CreditRating::B => 0.03,
            CreditRating::BMinus => 0.05,
            CreditRating::Ccc => 0.10,
            CreditRating::Cc => 0.20,
            CreditRating::C => 0.40,
            CreditRating::D => 1.0,
        }
    }

    /// Returns all investment grade ratings.
    pub fn investment_grades() -> &'static [CreditRating] {
        &[
            CreditRating::Aaa,
            CreditRating::AaPlus,
            CreditRating::Aa,
            CreditRating::AaMinus,
            CreditRating::APlus,
            CreditRating::A,
            CreditRating::AMinus,
            CreditRating::BbbPlus,
            CreditRating::Bbb,
            CreditRating::BbbMinus,
        ]
    }

    /// Returns all speculative grade ratings.
    pub fn speculative_grades() -> &'static [CreditRating] {
        &[
            CreditRating::BbPlus,
            CreditRating::Bb,
            CreditRating::BbMinus,
            CreditRating::BPlus,
            CreditRating::B,
            CreditRating::BMinus,
            CreditRating::Ccc,
            CreditRating::Cc,
            CreditRating::C,
            CreditRating::D,
        ]
    }
}


/// Credit parameters for XVA calculations.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CreditParams {
    hazard_rate: f64,
    lgd: f64,
    pd_1y: Option<f64>,
    rating: Option<CreditRating>,
}

impl CreditParams {
    /// Creates new credit parameters with validation.
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, CounterPartyError> {
        if hazard_rate < 0.0 {
            return Err(CounterPartyError::InvalidCreditParams(
                "Hazard rate must be non-negative".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&lgd) {
            return Err(CounterPartyError::InvalidCreditParams(
                "LGD must be in range [0, 1]".to_string(),
            ));
        }
        Ok(Self {
            hazard_rate,
            lgd,
            pd_1y: None,
            rating: None,
        })
    }

    /// Creates credit parameters from a credit rating.
    pub fn from_rating(rating: CreditRating, lgd: f64) -> Result<Self, CounterPartyError> {
        let mut params = Self::new(rating.indicative_hazard_rate(), lgd)?;
        params.rating = Some(rating);
        Ok(params)
    }

    /// Creates credit parameters from 1-year default probability.
    pub fn from_pd_1y(pd_1y: f64, lgd: f64) -> Result<Self, CounterPartyError> {
        if !(0.0..=1.0).contains(&pd_1y) {
            return Err(CounterPartyError::InvalidCreditParams(
                "PD must be in range [0, 1]".to_string(),
            ));
        }
        let hazard_rate = if pd_1y < 1.0 {
            -(1.0 - pd_1y).ln()
        } else {
            f64::INFINITY
        };
        let mut params = Self::new(hazard_rate, lgd)?;
        params.pd_1y = Some(pd_1y);
        Ok(params)
    }

    /// Sets the credit rating.
    pub fn with_rating(mut self, rating: CreditRating) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Returns the annual hazard rate (λ).
    #[inline]
    pub fn hazard_rate(&self) -> f64 { self.hazard_rate }

    /// Returns the loss given default (LGD).
    #[inline]
    pub fn lgd(&self) -> f64 { self.lgd }

    /// Returns the recovery rate (1 - LGD).
    #[inline]
    pub fn recovery_rate(&self) -> f64 { 1.0 - self.lgd }

    /// Returns the 1-year default probability.
    #[inline]
    pub fn pd_1y(&self) -> f64 { self.pd_1y.unwrap_or_else(|| self.default_prob(1.0)) }

    /// Returns the credit rating if set.
    #[inline]
    pub fn rating(&self) -> Option<CreditRating> { self.rating }

    /// Calculates survival probability to time t: Q(t) = exp(-λt).
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 { (-self.hazard_rate * t).exp() }

    /// Calculates default probability to time t: PD(t) = 1 - Q(t).
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 { 1.0 - self.survival_prob(t) }

    /// Calculates marginal default probability: PD(t1, t2) = Q(t1) - Q(t2).
    #[inline]
    pub fn marginal_default_prob(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }

    /// Calculates expected loss at time t: EL(t) = LGD × PD(t).
    #[inline]
    pub fn expected_loss(&self, t: f64) -> f64 { self.lgd * self.default_prob(t) }
}

impl Default for CreditParams {
    /// Creates default credit parameters with zero hazard rate and 40% LGD.
    fn default() -> Self {
        Self {
            hazard_rate: 0.0,
            lgd: 0.4,
            pd_1y: None,
            rating: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_rating_investment_grade_boundary() {
        assert!(CreditRating::Aaa.is_investment_grade());
        assert!(CreditRating::BbbMinus.is_investment_grade());
        assert!(!CreditRating::BbPlus.is_investment_grade());
        assert!(!CreditRating::D.is_investment_grade());
    }

    #[test]
    fn test_credit_rating_ordering_and_hazard_rates() {
        assert!(CreditRating::Aaa < CreditRating::D);
        let aaa = CreditRating::Aaa.indicative_hazard_rate();
        let bbb = CreditRating::Bbb.indicative_hazard_rate();
        let d = CreditRating::D.indicative_hazard_rate();
        assert!(aaa < bbb && bbb < d);
        assert!((d - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_credit_params_new_and_validation() {
        let params = CreditParams::new(0.01, 0.4).unwrap();
        assert!((params.hazard_rate() - 0.01).abs() < f64::EPSILON);
        assert!((params.recovery_rate() - 0.6).abs() < f64::EPSILON);

        assert!(CreditParams::new(-0.01, 0.4).is_err());
        assert!(CreditParams::new(0.01, -0.1).is_err());
        assert!(CreditParams::new(0.01, 1.1).is_err());
    }

    #[test]
    fn test_credit_params_from_rating() {
        let params = CreditParams::from_rating(CreditRating::Bbb, 0.4).unwrap();
        assert_eq!(params.rating(), Some(CreditRating::Bbb));
        assert!(
            (params.hazard_rate() - CreditRating::Bbb.indicative_hazard_rate()).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn test_credit_params_probabilities() {
        let params = CreditParams::new(0.01, 0.4).unwrap();
        assert!((params.survival_prob(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((params.survival_prob(1.0) - (-0.01_f64).exp()).abs() < 1e-10);
        assert!((params.default_prob(1.0) - (1.0 - (-0.01_f64).exp())).abs() < 1e-10);

        let marginal = params.marginal_default_prob(1.0, 2.0);
        let expected = params.survival_prob(1.0) - params.survival_prob(2.0);
        assert!((marginal - expected).abs() < 1e-10);
    }
}

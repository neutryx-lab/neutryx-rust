//! Credit rating and parameters for XVA calculations.
//!
//! This module provides types for managing credit risk parameters,
//! including credit ratings (20-grade scale) and credit parameters
//! (hazard rates, LGD, default probabilities).

#![allow(clippy::must_use_candidate)]

use super::CounterPartyError;

// ============================================================================
// CreditRating
// ============================================================================

/// Credit rating with +/- notches (20 grades).
///
/// Ratings are ordered from highest (Aaa) to lowest (D/Default).
/// Implements [`PartialOrd`] and [`Ord`] to allow comparison.
///
/// # Investment Grade
///
/// Ratings from Aaa to BbbMinus (inclusive) are considered investment grade.
/// Use [`is_investment_grade`](CreditRating::is_investment_grade) to check.
///
/// # Indicative Hazard Rates
///
/// Each rating has an indicative hazard rate that can be used as a starting
/// point for credit parameter calibration. Use
/// [`indicative_hazard_rate`](CreditRating::indicative_hazard_rate) to
/// retrieve.
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::CreditRating;
///
/// let rating = CreditRating::APlus;
/// assert!(rating.is_investment_grade());
/// assert!(rating < CreditRating::BbPlus); // A+ is better than BB+
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CreditRating {
    /// AAA - Highest quality
    Aaa,
    /// AA+ - High quality
    AaPlus,
    /// AA - High quality
    Aa,
    /// AA- - High quality
    AaMinus,
    /// A+ - Upper medium grade
    APlus,
    /// A - Upper medium grade
    A,
    /// A- - Upper medium grade
    AMinus,
    /// BBB+ - Lower medium grade (investment grade boundary)
    BbbPlus,
    /// BBB - Lower medium grade
    Bbb,
    /// BBB- - Lowest investment grade
    BbbMinus,
    /// BB+ - Non-investment grade speculative
    BbPlus,
    /// BB - Non-investment grade speculative
    Bb,
    /// BB- - Non-investment grade speculative
    BbMinus,
    /// B+ - Highly speculative
    BPlus,
    /// B - Highly speculative
    B,
    /// B- - Highly speculative
    BMinus,
    /// CCC - Substantial risks
    Ccc,
    /// CC - Extremely speculative
    Cc,
    /// C - Exceptionally high risk
    C,
    /// D - In default
    D,
}

impl CreditRating {
    /// Returns whether this rating is investment grade (BBB- or better).
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::counterparty::CreditRating;
    ///
    /// assert!(CreditRating::Aaa.is_investment_grade());
    /// assert!(CreditRating::BbbMinus.is_investment_grade());
    /// assert!(!CreditRating::BbPlus.is_investment_grade());
    /// ```
    #[inline]
    pub fn is_investment_grade(&self) -> bool { *self <= CreditRating::BbbMinus }

    /// Returns indicative annual hazard rate for this rating.
    ///
    /// These are approximate values based on historical default rates.
    /// For production use, calibrate to market CDS spreads.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::counterparty::CreditRating;
    ///
    /// let rate = CreditRating::Bbb.indicative_hazard_rate();
    /// assert!((rate - 0.002).abs() < 1e-10); // ~20 bp
    /// ```
    #[inline]
    pub fn indicative_hazard_rate(&self) -> f64 {
        match self {
            CreditRating::Aaa => 0.0001,     // 1 bp
            CreditRating::AaPlus => 0.0003,  // 3 bp
            CreditRating::Aa => 0.0005,      // 5 bp
            CreditRating::AaMinus => 0.0007, // 7 bp
            CreditRating::APlus => 0.0008,   // 8 bp
            CreditRating::A => 0.001,        // 10 bp
            CreditRating::AMinus => 0.0012,  // 12 bp
            CreditRating::BbbPlus => 0.0015, // 15 bp
            CreditRating::Bbb => 0.002,      // 20 bp
            CreditRating::BbbMinus => 0.003, // 30 bp
            CreditRating::BbPlus => 0.005,   // 50 bp
            CreditRating::Bb => 0.01,        // 100 bp
            CreditRating::BbMinus => 0.015,  // 150 bp
            CreditRating::BPlus => 0.02,     // 200 bp
            CreditRating::B => 0.03,         // 300 bp
            CreditRating::BMinus => 0.05,    // 500 bp
            CreditRating::Ccc => 0.10,       // 1000 bp
            CreditRating::Cc => 0.20,        // 2000 bp
            CreditRating::C => 0.40,         // 4000 bp
            CreditRating::D => 1.0,          // Default
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

impl std::fmt::Display for CreditRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CreditRating::Aaa => "AAA",
            CreditRating::AaPlus => "AA+",
            CreditRating::Aa => "AA",
            CreditRating::AaMinus => "AA-",
            CreditRating::APlus => "A+",
            CreditRating::A => "A",
            CreditRating::AMinus => "A-",
            CreditRating::BbbPlus => "BBB+",
            CreditRating::Bbb => "BBB",
            CreditRating::BbbMinus => "BBB-",
            CreditRating::BbPlus => "BB+",
            CreditRating::Bb => "BB",
            CreditRating::BbMinus => "BB-",
            CreditRating::BPlus => "B+",
            CreditRating::B => "B",
            CreditRating::BMinus => "B-",
            CreditRating::Ccc => "CCC",
            CreditRating::Cc => "CC",
            CreditRating::C => "C",
            CreditRating::D => "D",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// CreditParams
// ============================================================================

/// Credit parameters for XVA calculations.
///
/// Contains hazard rate (λ), loss given default (LGD), and optional
/// 1-year default probability (PD) and rating. Provides methods for
/// calculating survival and default probabilities.
///
/// # Mathematical Model
///
/// Assumes a constant hazard rate model where:
/// - Survival probability: Q(t) = exp(-λt)
/// - Default probability: PD(t) = 1 - Q(t)
/// - Marginal default probability: PD(t1, t2) = Q(t1) - Q(t2)
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::{CreditParams, CreditRating};
///
/// // Create from hazard rate and LGD
/// let params = CreditParams::new(0.01, 0.4).unwrap(); // 100bp hazard, 40% LGD
/// assert!((params.survival_prob(1.0) - 0.99005).abs() < 0.0001);
///
/// // Create from rating
/// let params = CreditParams::from_rating(CreditRating::Bbb, 0.4).unwrap();
/// assert_eq!(params.rating(), Some(CreditRating::Bbb));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreditParams {
    hazard_rate: f64,
    lgd: f64,
    pd_1y: Option<f64>,
    rating: Option<CreditRating>,
}

impl CreditParams {
    /// Creates new credit parameters with validation.
    ///
    /// # Arguments
    ///
    /// * `hazard_rate` - Annual hazard rate (λ), must be non-negative
    /// * `lgd` - Loss given default, must be in range [0, 1]
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidCreditParams`] if:
    /// - `hazard_rate` is negative
    /// - `lgd` is not in range [0, 1]
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
    ///
    /// Uses the rating's indicative hazard rate.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidCreditParams`] if `lgd` is not in
    /// [0, 1].
    pub fn from_rating(rating: CreditRating, lgd: f64) -> Result<Self, CounterPartyError> {
        let mut params = Self::new(rating.indicative_hazard_rate(), lgd)?;
        params.rating = Some(rating);
        Ok(params)
    }

    /// Creates credit parameters from 1-year default probability.
    ///
    /// Converts PD(1y) to hazard rate using: λ = -ln(1 - PD)
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidCreditParams`] if:
    /// - `pd_1y` is not in range [0, 1]
    /// - `lgd` is not in range [0, 1]
    pub fn from_pd_1y(pd_1y: f64, lgd: f64) -> Result<Self, CounterPartyError> {
        if !(0.0..=1.0).contains(&pd_1y) {
            return Err(CounterPartyError::InvalidCreditParams(
                "PD must be in range [0, 1]".to_string(),
            ));
        }
        // hazard_rate = -ln(1 - pd_1y)
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
    ///
    /// If not explicitly set, calculates from hazard rate.
    #[inline]
    pub fn pd_1y(&self) -> f64 { self.pd_1y.unwrap_or_else(|| self.default_prob(1.0)) }

    /// Returns the credit rating if set.
    #[inline]
    pub fn rating(&self) -> Option<CreditRating> { self.rating }

    /// Calculates survival probability to time t: Q(t) = exp(-λt)
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 { (-self.hazard_rate * t).exp() }

    /// Calculates default probability to time t: PD(t) = 1 - Q(t)
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 { 1.0 - self.survival_prob(t) }

    /// Calculates marginal default probability: PD(t1, t2) = Q(t1) - Q(t2)
    ///
    /// This is the probability of defaulting between t1 and t2,
    /// conditional on surviving to t1.
    ///
    /// # Arguments
    ///
    /// * `t1` - Start time in years
    /// * `t2` - End time in years (must be > t1)
    #[inline]
    pub fn marginal_default_prob(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }

    /// Calculates expected loss at time t: EL(t) = LGD × PD(t)
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

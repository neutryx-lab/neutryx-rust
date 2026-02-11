//! Counterparty structures with credit parameters.

use super::{error::PortfolioError, ids::CounterpartyId};

/// Credit rating enum following standard rating agencies, from AAA (highest) to
/// D (default).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum CreditRating {
    /// Highest quality (prime)
    AAA,
    /// High quality
    AA,
    /// Upper medium grade
    A,
    /// Lower medium grade (investment grade threshold)
    BBB,
    /// Non-investment grade speculative
    BB,
    /// Highly speculative
    B,
    /// Substantial risks
    CCC,
    /// Extremely speculative
    CC,
    /// In default with little prospect for recovery
    C,
    /// In default
    D,
}

impl CreditRating {
    /// Returns whether this rating is investment grade (BBB or better).
    #[inline]
    pub fn is_investment_grade(&self) -> bool {
        matches!(
            self,
            CreditRating::AAA | CreditRating::AA | CreditRating::A | CreditRating::BBB
        )
    }

    /// Returns a typical indicative hazard rate for this rating (should be
    /// replaced with actual CDS quotes in production).
    pub fn indicative_hazard_rate(&self) -> f64 {
        match self {
            CreditRating::AAA => 0.0001,
            CreditRating::AA => 0.0005,
            CreditRating::A => 0.001,
            CreditRating::BBB => 0.002,
            CreditRating::BB => 0.01,
            CreditRating::B => 0.03,
            CreditRating::CCC => 0.10,
            CreditRating::CC => 0.20,
            CreditRating::C => 0.40,
            CreditRating::D => 1.0,
        }
    }
}

/// Credit parameters for a counterparty containing hazard rate and LGD for
/// survival probabilities and expected losses.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CreditParams {
    hazard_rate: f64,
    lgd: f64,
    rating: Option<CreditRating>,
}

impl CreditParams {
    /// Creates new credit parameters with validated hazard rate (non-negative)
    /// and LGD (in [0, 1]).
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, PortfolioError> {
        if hazard_rate < 0.0 {
            return Err(PortfolioError::InvalidCreditParams(
                "Hazard rate must be non-negative".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&lgd) {
            return Err(PortfolioError::InvalidCreditParams(
                "LGD must be in range [0, 1]".to_string(),
            ));
        }

        Ok(Self {
            hazard_rate,
            lgd,
            rating: None,
        })
    }

    /// Creates credit parameters with a rating.
    pub fn with_rating(mut self, rating: CreditRating) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Creates credit parameters from a rating with typical values.
    pub fn from_rating(rating: CreditRating, lgd: f64) -> Result<Self, PortfolioError> {
        let params = Self::new(rating.indicative_hazard_rate(), lgd)?;
        Ok(params.with_rating(rating))
    }

    /// Returns the hazard rate.
    #[inline]
    pub fn hazard_rate(&self) -> f64 { self.hazard_rate }

    /// Returns the Loss Given Default.
    #[inline]
    pub fn lgd(&self) -> f64 { self.lgd }

    /// Returns the recovery rate (1 - LGD).
    #[inline]
    pub fn recovery_rate(&self) -> f64 { 1.0 - self.lgd }

    /// Returns the credit rating if set.
    #[inline]
    pub fn rating(&self) -> Option<CreditRating> { self.rating }

    /// Computes the survival probability to time t: Q(t) = exp(-lambda * t).
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 { (-self.hazard_rate * t).exp() }

    /// Computes the default probability to time t: PD(t) = 1 - Q(t).
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 { 1.0 - self.survival_prob(t) }

    /// Computes the marginal default probability between t1 and t2: PD(t1, t2)
    /// = Q(t1) - Q(t2).
    #[inline]
    pub fn marginal_default_prob(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }
}

/// Counterparty entity with credit parameters.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Counterparty {
    id: CounterpartyId,
    name: Option<String>,
    credit_params: CreditParams,
}

impl Counterparty {
    /// Creates a new counterparty.
    #[inline]
    pub fn new(id: CounterpartyId, credit_params: CreditParams) -> Self {
        Self {
            id,
            name: None,
            credit_params,
        }
    }

    /// Sets the counterparty name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns the counterparty ID.
    #[inline]
    pub fn id(&self) -> &CounterpartyId { &self.id }

    /// Returns the counterparty name if set.
    #[inline]
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }

    /// Returns the credit parameters.
    #[inline]
    pub fn credit_params(&self) -> &CreditParams { &self.credit_params }

    /// Convenience method: survival probability to time t.
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 { self.credit_params.survival_prob(t) }

    /// Convenience method: default probability to time t.
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 { self.credit_params.default_prob(t) }

    /// Convenience method: Loss Given Default.
    #[inline]
    pub fn lgd(&self) -> f64 { self.credit_params.lgd() }

    /// Convenience method: recovery rate.
    #[inline]
    pub fn recovery_rate(&self) -> f64 { self.credit_params.recovery_rate() }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_credit_rating_investment_grade() {
        assert!(CreditRating::AAA.is_investment_grade());
        assert!(CreditRating::AA.is_investment_grade());
        assert!(CreditRating::A.is_investment_grade());
        assert!(CreditRating::BBB.is_investment_grade());
        assert!(!CreditRating::BB.is_investment_grade());
        assert!(!CreditRating::B.is_investment_grade());
        assert!(!CreditRating::D.is_investment_grade());
    }

    #[test]
    fn test_credit_rating_ordering() {
        assert!(CreditRating::AAA < CreditRating::AA);
        assert!(CreditRating::AA < CreditRating::A);
        assert!(CreditRating::BBB < CreditRating::BB);
        assert!(CreditRating::C < CreditRating::D);
    }

    #[test]
    fn test_credit_params_valid() {
        let params = CreditParams::new(0.02, 0.4).unwrap();
        assert_eq!(params.hazard_rate(), 0.02);
        assert_eq!(params.lgd(), 0.4);
        assert_eq!(params.recovery_rate(), 0.6);
    }

    #[test]
    fn test_credit_params_invalid_lgd_negative() {
        let result = CreditParams::new(0.02, -0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_credit_params_invalid_lgd_above_one() {
        let result = CreditParams::new(0.02, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_credit_params_invalid_hazard_rate_negative() {
        let result = CreditParams::new(-0.01, 0.4);
        assert!(result.is_err());
    }

    #[test]
    fn test_credit_params_lgd_boundary() {
        assert!(CreditParams::new(0.02, 0.0).is_ok());
        assert!(CreditParams::new(0.02, 1.0).is_ok());
    }

    #[test]
    fn test_survival_probability() {
        let params = CreditParams::new(0.02, 0.4).unwrap();

        assert_relative_eq!(params.survival_prob(0.0), 1.0, epsilon = 1e-10);
        assert_relative_eq!(
            params.survival_prob(1.0),
            (-0.02_f64).exp(),
            epsilon = 1e-10
        );
        assert!(params.survival_prob(5.0) < params.survival_prob(1.0));
    }

    #[test]
    fn test_default_probability() {
        let params = CreditParams::new(0.02, 0.4).unwrap();

        assert_relative_eq!(params.default_prob(0.0), 0.0, epsilon = 1e-10);

        let t = 1.0;
        assert_relative_eq!(
            params.default_prob(t) + params.survival_prob(t),
            1.0,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_marginal_default_probability() {
        let params = CreditParams::new(0.05, 0.4).unwrap();

        let marginal = params.marginal_default_prob(1.0, 2.0);
        let expected = params.survival_prob(1.0) - params.survival_prob(2.0);

        assert_relative_eq!(marginal, expected, epsilon = 1e-10);
        assert!(marginal > 0.0);
    }

    #[test]
    fn test_credit_params_with_rating() {
        let params = CreditParams::new(0.02, 0.4)
            .unwrap()
            .with_rating(CreditRating::BBB);
        assert_eq!(params.rating(), Some(CreditRating::BBB));
    }

    #[test]
    fn test_credit_params_from_rating() {
        let params = CreditParams::from_rating(CreditRating::A, 0.45).unwrap();
        assert_eq!(params.hazard_rate(), 0.001);
        assert_eq!(params.lgd(), 0.45);
        assert_eq!(params.rating(), Some(CreditRating::A));
    }

    #[test]
    fn test_counterparty_creation() {
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let cp = Counterparty::new(CounterpartyId::new("CP001"), credit);

        assert_eq!(cp.id().as_str(), "CP001");
        assert!(cp.name().is_none());
    }

    #[test]
    fn test_counterparty_with_name() {
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let cp = Counterparty::new(CounterpartyId::new("CP001"), credit).with_name("Acme Corp");

        assert_eq!(cp.name(), Some("Acme Corp"));
    }

    #[test]
    fn test_counterparty_convenience_methods() {
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let cp = Counterparty::new(CounterpartyId::new("CP001"), credit);

        assert_eq!(cp.lgd(), 0.4);
        assert_eq!(cp.recovery_rate(), 0.6);
        assert_relative_eq!(cp.survival_prob(1.0), (-0.02_f64).exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_counterparty_clone() {
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let cp1 = Counterparty::new(CounterpartyId::new("CP001"), credit);
        let cp2 = cp1.clone();

        assert_eq!(cp1.id(), cp2.id());
    }
}

//! Counterparty structures with credit parameters.
//!
//! This module provides counterparty definitions with credit risk parameters
//! for CVA/DVA calculations.

use super::error::PortfolioError;
use super::ids::CounterpartyId;

/// Credit rating enum following standard rating agencies.
///
/// Ratings range from AAA (highest quality) to D (default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Returns a typical hazard rate for this rating (annual, indicative only).
    ///
    /// These are rough estimates and should be replaced with actual
    /// credit spreads/CDS quotes in production.
    pub fn indicative_hazard_rate(&self) -> f64 {
        match self {
            CreditRating::AAA => 0.0001, // 1 bp
            CreditRating::AA => 0.0005,  // 5 bp
            CreditRating::A => 0.001,    // 10 bp
            CreditRating::BBB => 0.002,  // 20 bp
            CreditRating::BB => 0.01,    // 100 bp
            CreditRating::B => 0.03,     // 300 bp
            CreditRating::CCC => 0.10,   // 1000 bp
            CreditRating::CC => 0.20,    // 2000 bp
            CreditRating::C => 0.40,     // 4000 bp
            CreditRating::D => 1.0,      // Default
        }
    }
}

/// Credit parameters for a counterparty.
///
/// Contains hazard rate and Loss Given Default (LGD) for
/// computing survival probabilities and expected losses.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::CreditParams;
///
/// let params = CreditParams::new(0.02, 0.4).unwrap();
/// assert_eq!(params.recovery_rate(), 0.6);
///
/// // Survival probability decreases over time
/// assert!(params.survival_prob(1.0) < 1.0);
/// assert!(params.survival_prob(2.0) < params.survival_prob(1.0));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreditParams {
    /// Hazard rate (annualised intensity)
    hazard_rate: f64,
    /// Loss Given Default as fraction [0, 1]
    lgd: f64,
    /// Optional credit rating
    rating: Option<CreditRating>,
}

impl CreditParams {
    /// Creates new credit parameters.
    ///
    /// # Arguments
    ///
    /// * `hazard_rate` - Annualised hazard rate (must be non-negative)
    /// * `lgd` - Loss Given Default, must be in range [0, 1]
    ///
    /// # Errors
    ///
    /// Returns `PortfolioError::InvalidCreditParams` if:
    /// - LGD is outside [0, 1]
    /// - Hazard rate is negative
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::portfolio::CreditParams;
    ///
    /// let params = CreditParams::new(0.02, 0.4).unwrap();
    /// ```
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
    ///
    /// # Arguments
    ///
    /// * `rating` - Credit rating
    /// * `lgd` - Loss Given Default
    pub fn from_rating(rating: CreditRating, lgd: f64) -> Result<Self, PortfolioError> {
        let params = Self::new(rating.indicative_hazard_rate(), lgd)?;
        Ok(params.with_rating(rating))
    }

    /// Returns the hazard rate.
    #[inline]
    pub fn hazard_rate(&self) -> f64 {
        self.hazard_rate
    }

    /// Returns the Loss Given Default.
    #[inline]
    pub fn lgd(&self) -> f64 {
        self.lgd
    }

    /// Returns the recovery rate (1 - LGD).
    #[inline]
    pub fn recovery_rate(&self) -> f64 {
        1.0 - self.lgd
    }

    /// Returns the credit rating if set.
    #[inline]
    pub fn rating(&self) -> Option<CreditRating> {
        self.rating
    }

    /// Computes the survival probability to time t.
    ///
    /// Q(t) = exp(-λ * t)
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::portfolio::CreditParams;
    ///
    /// let params = CreditParams::new(0.02, 0.4).unwrap();
    /// let surv_1y = params.survival_prob(1.0);
    /// assert!((surv_1y - 0.9802).abs() < 0.001);
    /// ```
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 {
        (-self.hazard_rate * t).exp()
    }

    /// Computes the default probability to time t.
    ///
    /// PD(t) = 1 - Q(t) = 1 - exp(-λ * t)
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 {
        1.0 - self.survival_prob(t)
    }

    /// Computes the marginal default probability between times t1 and t2.
    ///
    /// PD(t1, t2) = Q(t1) - Q(t2)
    ///
    /// # Arguments
    ///
    /// * `t1` - Start time
    /// * `t2` - End time (must be >= t1)
    #[inline]
    pub fn marginal_default_prob(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }
}

/// Counterparty entity with credit parameters.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::{Counterparty, CounterpartyId, CreditParams};
///
/// let credit = CreditParams::new(0.02, 0.4).unwrap();
/// let cp = Counterparty::new(
///     CounterpartyId::new("CP001"),
///     credit,
/// ).with_name("Acme Corp");
///
/// assert_eq!(cp.id().as_str(), "CP001");
/// assert_eq!(cp.name(), Some("Acme Corp"));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub fn id(&self) -> &CounterpartyId {
        &self.id
    }

    /// Returns the counterparty name if set.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the credit parameters.
    #[inline]
    pub fn credit_params(&self) -> &CreditParams {
        &self.credit_params
    }

    /// Convenience method: survival probability to time t.
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 {
        self.credit_params.survival_prob(t)
    }

    /// Convenience method: default probability to time t.
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 {
        self.credit_params.default_prob(t)
    }

    /// Convenience method: Loss Given Default.
    #[inline]
    pub fn lgd(&self) -> f64 {
        self.credit_params.lgd()
    }

    /// Convenience method: recovery rate.
    #[inline]
    pub fn recovery_rate(&self) -> f64 {
        self.credit_params.recovery_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

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
        // Zero LGD is valid
        assert!(CreditParams::new(0.02, 0.0).is_ok());
        // LGD = 1 is valid
        assert!(CreditParams::new(0.02, 1.0).is_ok());
    }

    #[test]
    fn test_survival_probability() {
        let params = CreditParams::new(0.02, 0.4).unwrap();

        // At t=0, survival prob = 1
        assert_relative_eq!(params.survival_prob(0.0), 1.0, epsilon = 1e-10);

        // At t=1, survival prob = exp(-0.02) ≈ 0.9802
        assert_relative_eq!(
            params.survival_prob(1.0),
            (-0.02_f64).exp(),
            epsilon = 1e-10
        );

        // Survival probability decreases over time
        assert!(params.survival_prob(5.0) < params.survival_prob(1.0));
    }

    #[test]
    fn test_default_probability() {
        let params = CreditParams::new(0.02, 0.4).unwrap();

        // At t=0, default prob = 0
        assert_relative_eq!(params.default_prob(0.0), 0.0, epsilon = 1e-10);

        // PD + Q = 1
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

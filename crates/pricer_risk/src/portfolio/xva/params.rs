//! XVA parameter structures for funding and own-credit calculations.

use super::error::XvaError;

/// Funding spread parameters for FVA calculation (borrowing cost and lending benefit).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FundingParams {
    pub spread_borrow: f64,
    pub spread_lend: f64,
}

impl FundingParams {
    /// Creates symmetric funding spreads (same for borrowing and lending).
    #[inline]
    pub fn symmetric(spread: f64) -> Self {
        Self {
            spread_borrow: spread,
            spread_lend: spread,
        }
    }

    /// Creates asymmetric funding spreads with different borrowing and lending rates.
    #[inline]
    pub fn asymmetric(spread_borrow: f64, spread_lend: f64) -> Self {
        Self {
            spread_borrow,
            spread_lend,
        }
    }

    /// Creates funding parameters from basis points.
    #[inline]
    pub fn from_bps(borrow_bps: f64, lend_bps: f64) -> Self {
        Self {
            spread_borrow: borrow_bps / 10_000.0,
            spread_lend: lend_bps / 10_000.0,
        }
    }

    /// Creates zero funding spreads (no FVA impact).
    #[inline]
    pub fn zero() -> Self { Self::symmetric(0.0) }

    /// Validates the funding parameters.
    pub fn validate(&self) -> Result<(), XvaError> {
        if self.spread_borrow < 0.0 {
            return Err(XvaError::InvalidFundingSpread(
                "Borrowing spread must be non-negative".to_string(),
            ));
        }
        if self.spread_lend < 0.0 {
            return Err(XvaError::InvalidFundingSpread(
                "Lending spread must be non-negative".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for FundingParams {
    fn default() -> Self { Self::zero() }
}

/// Own credit parameters for DVA calculation (institution's own default risk).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OwnCreditParams {
    hazard_rate: f64,
    lgd: f64,
}

impl OwnCreditParams {
    /// Creates new own credit parameters with validated hazard rate and LGD.
    pub fn new(hazard_rate: f64, lgd: f64) -> Result<Self, XvaError> {
        if hazard_rate < 0.0 {
            return Err(XvaError::InvalidCreditParam(
                "Hazard rate must be non-negative".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&lgd) {
            return Err(XvaError::InvalidCreditParam(
                "LGD must be in range [0, 1]".to_string(),
            ));
        }
        Ok(Self { hazard_rate, lgd })
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

    /// Computes the survival probability to time t: Q(t) = exp(-lambda * t).
    #[inline]
    pub fn survival_prob(&self, t: f64) -> f64 { (-self.hazard_rate * t).exp() }

    /// Computes the default probability to time t: PD(t) = 1 - Q(t).
    #[inline]
    pub fn default_prob(&self, t: f64) -> f64 { 1.0 - self.survival_prob(t) }

    /// Computes the marginal default probability between t1 and t2: PD(t1, t2) = Q(t1) - Q(t2).
    #[inline]
    pub fn marginal_pd(&self, t1: f64, t2: f64) -> f64 {
        self.survival_prob(t1) - self.survival_prob(t2)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_funding_params_symmetric() {
        let params = FundingParams::symmetric(0.005);
        assert_eq!(params.spread_borrow, 0.005);
        assert_eq!(params.spread_lend, 0.005);
    }

    #[test]
    fn test_funding_params_asymmetric() {
        let params = FundingParams::asymmetric(0.006, 0.004);
        assert_eq!(params.spread_borrow, 0.006);
        assert_eq!(params.spread_lend, 0.004);
    }

    #[test]
    fn test_funding_params_from_bps() {
        let params = FundingParams::from_bps(50.0, 30.0);
        assert_relative_eq!(params.spread_borrow, 0.005, epsilon = 1e-10);
        assert_relative_eq!(params.spread_lend, 0.003, epsilon = 1e-10);
    }

    #[test]
    fn test_funding_params_zero() {
        let params = FundingParams::zero();
        assert_eq!(params.spread_borrow, 0.0);
        assert_eq!(params.spread_lend, 0.0);
    }

    #[test]
    fn test_funding_params_default() {
        let params = FundingParams::default();
        assert_eq!(params.spread_borrow, 0.0);
        assert_eq!(params.spread_lend, 0.0);
    }

    #[test]
    fn test_funding_params_validate_valid() {
        let params = FundingParams::symmetric(0.005);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_funding_params_validate_negative_borrow() {
        let params = FundingParams::asymmetric(-0.001, 0.005);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_own_credit_params_valid() {
        let params = OwnCreditParams::new(0.02, 0.4).unwrap();
        assert_eq!(params.hazard_rate(), 0.02);
        assert_eq!(params.lgd(), 0.4);
        assert_eq!(params.recovery_rate(), 0.6);
    }

    #[test]
    fn test_own_credit_params_invalid_hazard_rate() {
        let result = OwnCreditParams::new(-0.01, 0.4);
        assert!(result.is_err());
    }

    #[test]
    fn test_own_credit_params_invalid_lgd() {
        let result = OwnCreditParams::new(0.02, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_own_credit_survival_prob() {
        let params = OwnCreditParams::new(0.02, 0.4).unwrap();

        assert_relative_eq!(params.survival_prob(0.0), 1.0, epsilon = 1e-10);

        assert_relative_eq!(
            params.survival_prob(1.0),
            (-0.02_f64).exp(),
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_own_credit_marginal_pd() {
        let params = OwnCreditParams::new(0.05, 0.4).unwrap();

        let marginal = params.marginal_pd(1.0, 2.0);
        let expected = params.survival_prob(1.0) - params.survival_prob(2.0);

        assert_relative_eq!(marginal, expected, epsilon = 1e-10);
        assert!(marginal > 0.0);
    }
}

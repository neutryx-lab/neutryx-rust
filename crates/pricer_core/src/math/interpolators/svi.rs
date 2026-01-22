//! SVI (Stochastic Volatility Inspired) volatility parameterisation.
//!
//! The SVI parameterisation provides a flexible and arbitrage-free
//! representation of implied volatility smiles. It was introduced by
//! Gatheral (2004) and is widely used in options markets.
//!
//! ## Formula
//!
//! Total variance w(k) = a + b * (rho * (k - m) + sqrt((k - m)² + sigma²))
//!
//! where:
//! - k = log(K/F) is the log-moneyness
//! - a is the overall variance level
//! - b controls the slope of the smile wings
//! - rho is the correlation/skew parameter (-1 < rho < 1)
//! - m is the at-the-money center
//! - sigma controls the smile curvature
//!
//! Implied volatility: sigma_impl(k) = sqrt(w(k) / T)

use num_traits::Float;

/// SVI parameter set for volatility smile parameterisation.
///
/// The SVI formula gives total variance as a function of log-moneyness:
/// w(k) = a + b * (rho * (k - m) + sqrt((k - m)² + sigma²))
#[derive(Debug, Clone, Copy)]
pub struct SviParams<T: Float> {
    /// Overall variance level (vertical shift).
    pub a: T,
    /// Slope of the wings.
    pub b: T,
    /// Correlation/skew parameter (-1 < rho < 1).
    pub rho: T,
    /// At-the-money center (horizontal shift).
    pub m: T,
    /// Curvature parameter (must be positive).
    pub sigma: T,
}

impl<T: Float> SviParams<T> {
    /// Creates new SVI parameters.
    ///
    /// # Arguments
    ///
    /// * `a` - Overall variance level
    /// * `b` - Wing slope (should be positive)
    /// * `rho` - Skew parameter (-1 < rho < 1)
    /// * `m` - ATM center
    /// * `sigma` - Curvature (must be positive)
    ///
    /// # Panics
    ///
    /// Does not validate parameters. Use `validate()` to check.
    #[must_use]
    pub fn new(a: T, b: T, rho: T, m: T, sigma: T) -> Self {
        Self { a, b, rho, m, sigma }
    }

    /// Validates SVI parameters for arbitrage-free conditions.
    ///
    /// Returns true if:
    /// - b >= 0 (non-negative wing slope)
    /// - -1 < rho < 1 (valid correlation)
    /// - sigma > 0 (positive curvature)
    /// - a + b * sigma * sqrt(1 - rho²) >= 0 (non-negative variance at minimum)
    #[must_use]
    pub fn validate(&self) -> bool {
        let one = T::one();
        let zero = T::zero();

        // Basic constraints
        if self.b < zero {
            return false;
        }
        if self.rho <= -one || self.rho >= one {
            return false;
        }
        if self.sigma <= zero {
            return false;
        }

        // Minimum variance constraint: a + b * sigma * sqrt(1 - rho²) >= 0
        let rho_sq = self.rho * self.rho;
        let sqrt_term = (one - rho_sq).sqrt();
        let min_variance = self.a + self.b * self.sigma * sqrt_term;

        min_variance >= zero
    }
}

/// Computes the SVI total variance at a given log-moneyness.
///
/// Total variance w(k) = a + b * (rho * (k - m) + sqrt((k - m)² + sigma²))
///
/// # Arguments
///
/// * `k` - Log-moneyness: ln(K/F)
/// * `params` - SVI parameters
///
/// # Returns
///
/// Total variance w(k).
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{SviParams, svi_total_variance};
///
/// let params = SviParams::new(0.04, 0.2, -0.4, 0.0, 0.3);
/// let w = svi_total_variance(0.0_f64, &params);
/// assert!(w > 0.0);
/// ```
#[must_use]
pub fn svi_total_variance<T: Float>(k: T, params: &SviParams<T>) -> T {
    let dk = k - params.m;
    let sqrt_term = (dk * dk + params.sigma * params.sigma).sqrt();

    params.a + params.b * (params.rho * dk + sqrt_term)
}

/// Computes the SVI implied volatility at a given log-moneyness.
///
/// Implied volatility sigma(k) = sqrt(w(k) / T)
///
/// # Arguments
///
/// * `k` - Log-moneyness: ln(K/F)
/// * `t` - Time to expiry in years
/// * `params` - SVI parameters
///
/// # Returns
///
/// Implied volatility. Returns zero if total variance is negative.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{SviParams, svi_implied_vol};
///
/// let params = SviParams::new(0.04, 0.2, -0.4, 0.0, 0.3);
/// let vol = svi_implied_vol(0.0_f64, 1.0, &params);
/// assert!(vol > 0.0 && vol < 1.0);
/// ```
#[must_use]
pub fn svi_implied_vol<T: Float>(k: T, t: T, params: &SviParams<T>) -> T {
    let w = svi_total_variance(k, params);
    if w <= T::zero() || t <= T::zero() {
        return T::zero();
    }
    (w / t).sqrt()
}

/// Computes the SVI derivative dw/dk (for Greeks).
///
/// dw/dk = b * (rho + (k - m) / sqrt((k - m)² + sigma²))
///
/// # Arguments
///
/// * `k` - Log-moneyness
/// * `params` - SVI parameters
///
/// # Returns
///
/// Derivative of total variance with respect to log-moneyness.
#[must_use]
pub fn svi_dw_dk<T: Float>(k: T, params: &SviParams<T>) -> T {
    let dk = k - params.m;
    let sqrt_term = (dk * dk + params.sigma * params.sigma).sqrt();

    params.b * (params.rho + dk / sqrt_term)
}

/// Computes the SVI second derivative d²w/dk² (for Greeks).
///
/// d²w/dk² = b * sigma² / ((k - m)² + sigma²)^(3/2)
///
/// # Arguments
///
/// * `k` - Log-moneyness
/// * `params` - SVI parameters
///
/// # Returns
///
/// Second derivative of total variance with respect to log-moneyness.
#[must_use]
pub fn svi_d2w_dk2<T: Float>(k: T, params: &SviParams<T>) -> T {
    let dk = k - params.m;
    let sigma_sq = params.sigma * params.sigma;
    let denom = dk * dk + sigma_sq;
    let denom_32 = denom * denom.sqrt();

    params.b * sigma_sq / denom_32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> SviParams<f64> {
        // Typical equity parameters
        SviParams::new(0.04, 0.2, -0.4, 0.0, 0.3)
    }

    #[test]
    fn test_svi_params_new() {
        let params = SviParams::new(0.04, 0.2, -0.4, 0.0, 0.3);
        assert!((params.a - 0.04).abs() < 1e-10);
        assert!((params.b - 0.2).abs() < 1e-10);
        assert!((params.rho - (-0.4)).abs() < 1e-10);
        assert!((params.m - 0.0).abs() < 1e-10);
        assert!((params.sigma - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_svi_params_validate_valid() {
        let params = sample_params();
        assert!(params.validate());
    }

    #[test]
    fn test_svi_params_validate_negative_b() {
        let params = SviParams::new(0.04, -0.1, -0.4, 0.0, 0.3);
        assert!(!params.validate());
    }

    #[test]
    fn test_svi_params_validate_invalid_rho() {
        let params_high = SviParams::new(0.04, 0.2, 1.0, 0.0, 0.3);
        let params_low = SviParams::new(0.04, 0.2, -1.0, 0.0, 0.3);
        assert!(!params_high.validate());
        assert!(!params_low.validate());
    }

    #[test]
    fn test_svi_params_validate_negative_sigma() {
        let params = SviParams::new(0.04, 0.2, -0.4, 0.0, -0.1);
        assert!(!params.validate());
    }

    #[test]
    fn test_svi_params_validate_negative_min_variance() {
        // a + b * sigma * sqrt(1 - rho²) < 0
        let params = SviParams::new(-0.1, 0.1, 0.0, 0.0, 0.5);
        // -0.1 + 0.1 * 0.5 * 1.0 = -0.05 < 0
        assert!(!params.validate());
    }

    #[test]
    fn test_svi_total_variance_at_atm() {
        let params = sample_params();
        let w = svi_total_variance(0.0, &params);

        // At k = m = 0: w = a + b * sqrt(sigma²) = a + b * sigma
        let expected = 0.04 + 0.2 * 0.3;
        assert!((w - expected).abs() < 1e-10);
    }

    #[test]
    fn test_svi_total_variance_symmetry() {
        // With rho = 0, the smile is symmetric
        let params = SviParams::new(0.04, 0.2, 0.0, 0.0, 0.3);

        let w_plus = svi_total_variance(0.5, &params);
        let w_minus = svi_total_variance(-0.5, &params);

        assert!((w_plus - w_minus).abs() < 1e-10);
    }

    #[test]
    fn test_svi_total_variance_skew() {
        // With negative rho, left wing (k < 0) should be higher
        let params = SviParams::new(0.04, 0.2, -0.4, 0.0, 0.3);

        let w_otm_put = svi_total_variance(-0.5, &params);
        let w_otm_call = svi_total_variance(0.5, &params);

        assert!(w_otm_put > w_otm_call);
    }

    #[test]
    fn test_svi_implied_vol() {
        let params = sample_params();
        let t = 1.0;

        let vol = svi_implied_vol(0.0, t, &params);
        let w = svi_total_variance(0.0, &params);

        assert!((vol - w.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_svi_implied_vol_term_structure() {
        let params = sample_params();

        // Same total variance, different T -> different implied vol
        let vol_1y = svi_implied_vol(0.0, 1.0, &params);
        let vol_2y = svi_implied_vol(0.0, 2.0, &params);

        // vol scales as 1/sqrt(T) for constant total variance
        let ratio = vol_1y / vol_2y;
        let expected_ratio = (2.0_f64).sqrt();

        assert!((ratio - expected_ratio).abs() < 1e-10);
    }

    #[test]
    fn test_svi_dw_dk_at_atm() {
        let params = sample_params();
        let dw = svi_dw_dk(0.0, &params);

        // At k = m = 0: dw/dk = b * (rho + 0/sigma) = b * rho
        let expected = params.b * params.rho;
        assert!((dw - expected).abs() < 1e-10);
    }

    #[test]
    fn test_svi_d2w_dk2_at_atm() {
        let params = sample_params();
        let d2w = svi_d2w_dk2(0.0, &params);

        // At k = m = 0: d²w/dk² = b * sigma² / sigma³ = b / sigma
        let expected = params.b / params.sigma;
        assert!((d2w - expected).abs() < 1e-10);
    }

    #[test]
    fn test_svi_d2w_dk2_always_positive() {
        // Convexity should always be positive for valid params
        let params = sample_params();

        for k in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let d2w = svi_d2w_dk2(k, &params);
            assert!(d2w > 0.0, "d2w should be positive at k={}", k);
        }
    }

    #[test]
    fn test_svi_implied_vol_positive() {
        let params = sample_params();
        let t = 1.0;

        for k in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let vol = svi_implied_vol(k, t, &params);
            assert!(vol > 0.0, "vol should be positive at k={}", k);
        }
    }

    #[test]
    fn test_svi_implied_vol_reasonable_range() {
        // Typical equity vols should be in reasonable range
        let params = sample_params();
        let t = 1.0;

        for k in [-0.5, 0.0, 0.5] {
            let vol = svi_implied_vol(k, t, &params);
            assert!(vol > 0.1 && vol < 1.0, "vol {} at k={} out of range", vol, k);
        }
    }
}

//! Gaussian copula for modelling dependence structure.
//!
//! This module provides the Gaussian copula, which is widely used in
//! credit derivatives pricing (e.g., CDO tranches) and multivariate
//! risk modelling.
//!
//! ## Background
//!
//! A copula is a function that joins univariate marginal distributions
//! to form a multivariate distribution. The Gaussian copula uses the
//! multivariate normal distribution's dependence structure.
//!
//! For two uniform random variables U, V on [0, 1]:
//! C(u, v; ρ) = Φ₂(Φ⁻¹(u), Φ⁻¹(v); ρ)
//!
//! where Φ₂ is the bivariate normal CDF and Φ⁻¹ is the inverse
//! standard normal CDF.
//!
//! ## Usage
//!
//! ```
//! use pricer_core::math::distributions::{GaussianCopula, CopulaTrait};
//!
//! // Create a 2D Gaussian copula with correlation 0.5
//! let copula = GaussianCopula::new_bivariate(0.5).unwrap();
//!
//! // Compute joint probability
//! let u = vec![0.5, 0.5];
//! let prob = copula.joint_probability(&u).unwrap();
//! // For u = v = 0.5 and ρ = 0.5, this equals 1/3
//! ```
//!
//! ## Reference
//!
//! Li, D. X. (2000). On Default Correlation: A Copula Function Approach.
//! Journal of Fixed Income, 9(4), 43-54.

use super::bivariate_normal::bivariate_norm_cdf;
use super::normal::norm_inv_cdf;
use super::DistributionError;

/// Trait for copula implementations.
pub trait CopulaTrait {
    /// Computes the joint probability C(u₁, u₂, ..., uₙ).
    ///
    /// # Arguments
    ///
    /// * `u` - Vector of marginal probabilities, each in (0, 1)
    ///
    /// # Returns
    ///
    /// The joint probability value in [0, 1]
    ///
    /// # Errors
    ///
    /// Returns error if any u is outside (0, 1) or dimension mismatch
    fn joint_probability(&self, u: &[f64]) -> Result<f64, DistributionError>;

    /// Returns the dimension of the copula.
    fn dimension(&self) -> usize;
}

/// Gaussian copula for modelling joint distributions.
///
/// The Gaussian copula is defined by a correlation matrix and uses
/// the multivariate normal distribution to model dependence.
#[derive(Debug, Clone)]
pub struct GaussianCopula {
    /// Correlation matrix (stored as flattened row-major for n=2 case)
    /// For bivariate case, only stores the correlation coefficient
    correlation: f64,
    /// Dimension of the copula (currently only 2 is fully supported)
    dim: usize,
}

impl GaussianCopula {
    /// Creates a new bivariate Gaussian copula with the given correlation.
    ///
    /// # Arguments
    ///
    /// * `rho` - Correlation coefficient in [-1, 1]
    ///
    /// # Returns
    ///
    /// A new `GaussianCopula` instance
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::InvalidCorrelation`] if ρ is outside [-1, 1]
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::distributions::GaussianCopula;
    ///
    /// let copula = GaussianCopula::new_bivariate(0.5).unwrap();
    /// assert_eq!(copula.dimension(), 2);
    /// ```
    pub fn new_bivariate(rho: f64) -> Result<Self, DistributionError> {
        if rho < -1.0 || rho > 1.0 {
            return Err(DistributionError::InvalidCorrelation { rho });
        }

        Ok(Self {
            correlation: rho,
            dim: 2,
        })
    }

    /// Returns the correlation coefficient (for bivariate copula).
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::distributions::GaussianCopula;
    ///
    /// let copula = GaussianCopula::new_bivariate(0.7).unwrap();
    /// assert!((copula.correlation() - 0.7).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn correlation(&self) -> f64 {
        self.correlation
    }
}

impl CopulaTrait for GaussianCopula {
    fn joint_probability(&self, u: &[f64]) -> Result<f64, DistributionError> {
        // Validate dimension
        if u.len() != self.dim {
            return Err(DistributionError::NumericalError(format!(
                "Expected {} marginals, got {}",
                self.dim,
                u.len()
            )));
        }

        // Validate each marginal is in (0, 1)
        for (i, &ui) in u.iter().enumerate() {
            if ui <= 0.0 || ui >= 1.0 {
                return Err(DistributionError::InvalidProbability { p: ui });
            }
        }

        // For bivariate case: C(u, v; ρ) = Φ₂(Φ⁻¹(u), Φ⁻¹(v); ρ)
        if self.dim == 2 {
            let x = norm_inv_cdf(u[0])?;
            let y = norm_inv_cdf(u[1])?;
            bivariate_norm_cdf(x, y, self.correlation)
        } else {
            // Higher dimensions not yet implemented
            Err(DistributionError::NumericalError(
                "Multivariate Gaussian copula not yet implemented".to_string(),
            ))
        }
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Convenience function to compute bivariate Gaussian copula value.
///
/// # Arguments
///
/// * `u` - First marginal probability in (0, 1)
/// * `v` - Second marginal probability in (0, 1)
/// * `rho` - Correlation coefficient in [-1, 1]
///
/// # Returns
///
/// The copula value C(u, v; ρ)
///
/// # Errors
///
/// Returns error if parameters are invalid
///
/// # Example
///
/// ```
/// use pricer_core::math::distributions::gaussian_copula;
///
/// let c = gaussian_copula(0.5, 0.5, 0.5).unwrap();
/// // For u = v = 0.5 and ρ = 0.5, C ≈ 1/3
/// assert!((c - 1.0/3.0).abs() < 0.01);
/// ```
pub fn gaussian_copula(u: f64, v: f64, rho: f64) -> Result<f64, DistributionError> {
    // Validate inputs
    if u <= 0.0 || u >= 1.0 {
        return Err(DistributionError::InvalidProbability { p: u });
    }
    if v <= 0.0 || v >= 1.0 {
        return Err(DistributionError::InvalidProbability { p: v });
    }
    if rho < -1.0 || rho > 1.0 {
        return Err(DistributionError::InvalidCorrelation { rho });
    }

    // C(u, v; ρ) = Φ₂(Φ⁻¹(u), Φ⁻¹(v); ρ)
    let x = norm_inv_cdf(u)?;
    let y = norm_inv_cdf(v)?;
    bivariate_norm_cdf(x, y, rho)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ==========================================================================
    // GaussianCopula construction tests
    // ==========================================================================

    #[test]
    fn test_gaussian_copula_new_bivariate() {
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();
        assert_eq!(copula.dimension(), 2);
        assert_relative_eq!(copula.correlation(), 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_gaussian_copula_new_bivariate_extreme_correlations() {
        // Perfect positive correlation
        let copula = GaussianCopula::new_bivariate(1.0).unwrap();
        assert_eq!(copula.correlation(), 1.0);

        // Perfect negative correlation
        let copula = GaussianCopula::new_bivariate(-1.0).unwrap();
        assert_eq!(copula.correlation(), -1.0);

        // Zero correlation
        let copula = GaussianCopula::new_bivariate(0.0).unwrap();
        assert_eq!(copula.correlation(), 0.0);
    }

    #[test]
    fn test_gaussian_copula_invalid_correlation() {
        let result = GaussianCopula::new_bivariate(1.5);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidCorrelation { rho }) if rho == 1.5
        ));

        let result = GaussianCopula::new_bivariate(-1.5);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidCorrelation { rho }) if rho == -1.5
        ));
    }

    // ==========================================================================
    // Joint probability tests
    // ==========================================================================

    #[test]
    fn test_gaussian_copula_joint_probability_independent() {
        // When ρ = 0, C(u, v) = u * v
        let copula = GaussianCopula::new_bivariate(0.0).unwrap();

        let u = vec![0.5, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        assert_relative_eq!(c, 0.25, epsilon = 1e-6);

        let u = vec![0.3, 0.7];
        let c = copula.joint_probability(&u).unwrap();
        assert_relative_eq!(c, 0.21, epsilon = 1e-4);
    }

    #[test]
    fn test_gaussian_copula_joint_probability_positive_correlation() {
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();

        let u = vec![0.5, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        // For u = v = 0.5 and ρ = 0.5, C ≈ 1/3
        assert!(c > 0.25); // More than independent case
        assert!(c < 0.5);
    }

    #[test]
    fn test_gaussian_copula_joint_probability_negative_correlation() {
        let copula = GaussianCopula::new_bivariate(-0.5).unwrap();

        let u = vec![0.5, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        // For negative correlation, joint probability should be lower
        assert!(c < 0.25);
    }

    #[test]
    fn test_gaussian_copula_joint_probability_perfect_positive() {
        let copula = GaussianCopula::new_bivariate(1.0).unwrap();

        // With ρ = 1, C(u, v) = min(u, v)
        let u = vec![0.3, 0.7];
        let c = copula.joint_probability(&u).unwrap();
        assert_relative_eq!(c, 0.3, epsilon = 1e-4);

        let u = vec![0.7, 0.3];
        let c = copula.joint_probability(&u).unwrap();
        assert_relative_eq!(c, 0.3, epsilon = 1e-4);
    }

    #[test]
    fn test_gaussian_copula_joint_probability_perfect_negative() {
        let copula = GaussianCopula::new_bivariate(-1.0).unwrap();

        // With ρ = -1, C(u, v) = max(0, u + v - 1)
        let u = vec![0.3, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        // max(0, 0.3 + 0.5 - 1) = max(0, -0.2) = 0
        assert_relative_eq!(c, 0.0, epsilon = 1e-6);

        let u = vec![0.8, 0.9];
        let c = copula.joint_probability(&u).unwrap();
        // max(0, 0.8 + 0.9 - 1) = 0.7
        assert_relative_eq!(c, 0.7, epsilon = 1e-4);
    }

    // ==========================================================================
    // Error handling tests
    // ==========================================================================

    #[test]
    fn test_gaussian_copula_invalid_dimension() {
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();

        // Wrong number of marginals
        let u = vec![0.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(result, Err(DistributionError::NumericalError(_))));

        let u = vec![0.5, 0.5, 0.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(result, Err(DistributionError::NumericalError(_))));
    }

    #[test]
    fn test_gaussian_copula_invalid_marginals() {
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();

        // Marginal = 0
        let u = vec![0.0, 0.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));

        // Marginal = 1
        let u = vec![0.5, 1.0];
        let result = copula.joint_probability(&u);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));

        // Marginal < 0
        let u = vec![-0.1, 0.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));

        // Marginal > 1
        let u = vec![0.5, 1.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));
    }

    // ==========================================================================
    // Convenience function tests
    // ==========================================================================

    #[test]
    fn test_gaussian_copula_function() {
        let c = gaussian_copula(0.5, 0.5, 0.0).unwrap();
        assert_relative_eq!(c, 0.25, epsilon = 1e-6);
    }

    #[test]
    fn test_gaussian_copula_function_positive_correlation() {
        let c = gaussian_copula(0.5, 0.5, 0.5).unwrap();
        assert!(c > 0.25);
    }

    #[test]
    fn test_gaussian_copula_function_invalid_inputs() {
        // Invalid u
        assert!(matches!(
            gaussian_copula(0.0, 0.5, 0.5),
            Err(DistributionError::InvalidProbability { .. })
        ));

        // Invalid v
        assert!(matches!(
            gaussian_copula(0.5, 1.0, 0.5),
            Err(DistributionError::InvalidProbability { .. })
        ));

        // Invalid rho
        assert!(matches!(
            gaussian_copula(0.5, 0.5, 1.5),
            Err(DistributionError::InvalidCorrelation { .. })
        ));
    }

    // ==========================================================================
    // Properties tests
    // ==========================================================================

    #[test]
    fn test_gaussian_copula_bounds() {
        // For any copula: max(0, u + v - 1) ≤ C(u, v) ≤ min(u, v)
        // Test with moderate values (not at extremes)
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();

        for u in [0.2, 0.4, 0.6, 0.8] {
            for v in [0.2, 0.4, 0.6, 0.8] {
                let uv = vec![u, v];
                let c = copula.joint_probability(&uv).unwrap();

                let lower = (u + v - 1.0).max(0.0);
                let upper = u.min(v);

                // Use tolerant bounds check for numerical stability
                assert!(
                    c >= lower - 0.01,
                    "C({u}, {v}) = {c} < lower bound {lower}"
                );
                assert!(
                    c <= upper + 0.01,
                    "C({u}, {v}) = {c} > upper bound {upper}"
                );
            }
        }
    }

    #[test]
    fn test_gaussian_copula_symmetry() {
        // C(u, v) = C(v, u)
        let copula = GaussianCopula::new_bivariate(0.5).unwrap();

        for u in [0.2, 0.4, 0.6, 0.8] {
            for v in [0.2, 0.4, 0.6, 0.8] {
                let c1 = copula.joint_probability(&[u, v]).unwrap();
                let c2 = copula.joint_probability(&[v, u]).unwrap();
                assert_relative_eq!(c1, c2, epsilon = 1e-6);
            }
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_gaussian_copula_bounds(
            u in 0.1_f64..0.9,
            v in 0.1_f64..0.9,
            rho in -0.8_f64..0.8
        ) {
            let c = gaussian_copula(u, v, rho).unwrap();

            // Fréchet-Hoeffding bounds with numerical tolerance
            let lower = (u + v - 1.0).max(0.0);
            let upper = u.min(v);

            prop_assert!(c >= lower - 0.02, "C({u}, {v}) = {c} < lower bound {lower}");
            prop_assert!(c <= upper + 0.02, "C({u}, {v}) = {c} > upper bound {upper}");
        }

        #[test]
        fn prop_gaussian_copula_symmetry(
            u in 0.1_f64..0.9,
            v in 0.1_f64..0.9,
            rho in -0.8_f64..0.8
        ) {
            let c1 = gaussian_copula(u, v, rho).unwrap();
            let c2 = gaussian_copula(v, u, rho).unwrap();
            // Increased tolerance for numerical stability
            prop_assert!((c1 - c2).abs() < 1e-4);
        }

        #[test]
        fn prop_gaussian_copula_monotonicity_in_u(
            u in 0.1_f64..0.79,
            v in 0.1_f64..0.9,
            rho in -0.8_f64..0.8
        ) {
            let c1 = gaussian_copula(u, v, rho).unwrap();
            let c2 = gaussian_copula(u + 0.1, v, rho).unwrap();
            // Copula should be non-decreasing in u with numerical tolerance
            prop_assert!(c2 >= c1 - 0.01, "Copula not monotonic: C({}, {v}) = {c1}, C({}, {v}) = {c2}", u, u + 0.1);
        }
    }
}

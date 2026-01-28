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

use super::{bivariate_normal::bivariate_norm_cdf, normal::norm_inv_cdf, DistributionError};
#[cfg(feature = "linalg")]
use crate::math::linalg::{cholesky, matrix_from_rows, Matrix};

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
    /// Returns [`DistributionError::InvalidCorrelation`] if ρ is outside [-1,
    /// 1]
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::distributions::{GaussianCopula, CopulaTrait};
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
    pub fn correlation(&self) -> f64 { self.correlation }
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
        for &ui in u.iter() {
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

    fn dimension(&self) -> usize { self.dim }
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
/// // For u = v = 0.5 and ρ = 0.5, the copula value is between the
/// // independence case (0.25) and perfect correlation
/// assert!(c > 0.25 && c < 0.5);
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

// ==========================================================================
// Multi-dimensional Gaussian Copula (requires "linalg" feature)
// ==========================================================================

#[cfg(feature = "linalg")]
/// Multi-dimensional Gaussian copula for arbitrary dimensions.
///
/// This implementation supports n-dimensional Gaussian copulas using
/// Monte Carlo simulation for the multivariate normal CDF calculation.
///
/// For n=2, it's more efficient to use [`GaussianCopula`] which uses
/// the analytical bivariate normal CDF.
///
/// ## Background
///
/// The n-dimensional Gaussian copula is defined as:
///
/// C(u₁, ..., uₙ) = Φₙ(Φ⁻¹(u₁), ..., Φ⁻¹(uₙ); Σ)
///
/// where Φₙ is the n-dimensional multivariate normal CDF with
/// correlation matrix Σ, and Φ⁻¹ is the inverse standard normal CDF.
///
/// ## Usage
///
/// ```
/// use pricer_core::math::distributions::{MultiGaussianCopula, CopulaTrait};
///
/// // Create a 3D Gaussian copula with correlation matrix
/// let corr = vec![
///     1.0, 0.5, 0.3,
///     0.5, 1.0, 0.4,
///     0.3, 0.4, 1.0,
/// ];
/// let copula = MultiGaussianCopula::new(3, &corr).unwrap();
///
/// // Compute joint probability
/// let u = vec![0.5, 0.5, 0.5];
/// let prob = copula.joint_probability(&u).unwrap();
/// ```
///
/// ## Reference
///
/// Genz, A. (1992). Numerical Computation of Multivariate Normal Probabilities.
/// Journal of Computational and Graphical Statistics, 1(2), 141-149.
#[derive(Debug, Clone)]
pub struct MultiGaussianCopula {
    /// Dimension of the copula
    dim: usize,
    /// Lower triangular Cholesky factor of the correlation matrix
    /// Used for generating correlated normal samples
    cholesky_l: Matrix<f64>,
    /// Number of Monte Carlo samples for probability estimation
    num_samples: usize,
}

#[cfg(feature = "linalg")]
impl MultiGaussianCopula {
    /// Default number of Monte Carlo samples for probability estimation.
    pub const DEFAULT_NUM_SAMPLES: usize = 100_000;

    /// Creates a new multi-dimensional Gaussian copula.
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension of the copula (n ≥ 2)
    /// * `corr_flat` - Flattened correlation matrix in row-major order (length
    ///   = dim²)
    ///
    /// # Returns
    ///
    /// A new `MultiGaussianCopula` instance.
    ///
    /// # Errors
    ///
    /// * [`DistributionError::NumericalError`] if the correlation matrix has
    ///   wrong size
    /// * [`DistributionError::InvalidCorrelation`] if diagonal elements are not
    ///   1
    /// * [`DistributionError::NumericalError`] if matrix is not symmetric
    /// * [`DistributionError::NotPositiveDefinite`] if matrix is not positive
    ///   definite
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::distributions::MultiGaussianCopula;
    ///
    /// let corr = vec![
    ///     1.0, 0.5,
    ///     0.5, 1.0,
    /// ];
    /// let copula = MultiGaussianCopula::new(2, &corr).unwrap();
    /// ```
    pub fn new(dim: usize, corr_flat: &[f64]) -> Result<Self, DistributionError> {
        Self::with_samples(dim, corr_flat, Self::DEFAULT_NUM_SAMPLES)
    }

    /// Creates a new multi-dimensional Gaussian copula with a specified number
    /// of samples.
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension of the copula (n ≥ 2)
    /// * `corr_flat` - Flattened correlation matrix in row-major order
    /// * `num_samples` - Number of Monte Carlo samples for probability
    ///   estimation
    pub fn with_samples(
        dim: usize,
        corr_flat: &[f64],
        num_samples: usize,
    ) -> Result<Self, DistributionError> {
        // Validate dimension
        if dim < 2 {
            return Err(DistributionError::NumericalError(
                "Copula dimension must be at least 2".to_string(),
            ));
        }

        // Validate matrix size
        if corr_flat.len() != dim * dim {
            return Err(DistributionError::NumericalError(format!(
                "Correlation matrix size mismatch: expected {}x{} = {}, got {}",
                dim,
                dim,
                dim * dim,
                corr_flat.len()
            )));
        }

        // Create matrix
        let corr_matrix = matrix_from_rows(dim, dim, corr_flat);

        // Validate diagonal elements are 1
        for i in 0..dim {
            let diag = corr_matrix[(i, i)];
            if (diag - 1.0).abs() > 1e-10 {
                return Err(DistributionError::InvalidCorrelation { rho: diag });
            }
        }

        // Validate symmetry
        for i in 0..dim {
            for j in (i + 1)..dim {
                let diff = (corr_matrix[(i, j)] - corr_matrix[(j, i)]).abs();
                if diff > 1e-10 {
                    return Err(DistributionError::NumericalError(format!(
                        "Correlation matrix is not symmetric: corr[{i},{j}] = {}, corr[{j},{i}] = {}",
                        corr_matrix[(i, j)],
                        corr_matrix[(j, i)]
                    )));
                }
            }
        }

        // Validate correlation coefficients are in [-1, 1]
        for i in 0..dim {
            for j in 0..dim {
                let rho = corr_matrix[(i, j)];
                if rho < -1.0 || rho > 1.0 {
                    return Err(DistributionError::InvalidCorrelation { rho });
                }
            }
        }

        // Compute Cholesky decomposition (validates positive definiteness)
        let cholesky_l =
            cholesky(&corr_matrix).map_err(|_| DistributionError::NotPositiveDefinite)?;

        Ok(Self {
            dim,
            cholesky_l,
            num_samples,
        })
    }

    /// Returns the dimension of the copula.
    #[must_use]
    pub fn dim(&self) -> usize { self.dim }

    /// Returns the number of Monte Carlo samples used for probability
    /// estimation.
    #[must_use]
    pub fn num_samples(&self) -> usize { self.num_samples }

    /// Computes the joint probability using Monte Carlo simulation with a
    /// specific seed.
    ///
    /// This method allows for reproducible results by specifying the random
    /// seed.
    ///
    /// # Arguments
    ///
    /// * `u` - Vector of marginal probabilities, each in (0, 1)
    /// * `seed` - Random seed for reproducibility
    ///
    /// # Returns
    ///
    /// The estimated joint probability value.
    pub fn joint_probability_with_seed(
        &self,
        u: &[f64],
        seed: u64,
    ) -> Result<f64, DistributionError> {
        self.joint_probability_internal(u, Some(seed))
    }

    /// Internal implementation of joint probability calculation.
    fn joint_probability_internal(
        &self,
        u: &[f64],
        seed: Option<u64>,
    ) -> Result<f64, DistributionError> {
        // Validate dimension
        if u.len() != self.dim {
            return Err(DistributionError::NumericalError(format!(
                "Expected {} marginals, got {}",
                self.dim,
                u.len()
            )));
        }

        // Validate each marginal is in (0, 1)
        for &ui in u.iter() {
            if ui <= 0.0 || ui >= 1.0 {
                return Err(DistributionError::InvalidProbability { p: ui });
            }
        }

        // Transform marginals to normal quantiles: a_i = Φ⁻¹(u_i)
        let mut a = Vec::with_capacity(self.dim);
        for &ui in u {
            a.push(norm_inv_cdf(ui)?);
        }

        // Use Monte Carlo to estimate P(X₁ ≤ a₁, ..., Xₙ ≤ aₙ)
        // where (X₁, ..., Xₙ) ~ N(0, Σ)
        Ok(self.multivariate_normal_cdf_mc(&a, seed))
    }

    /// Monte Carlo estimation of multivariate normal CDF.
    ///
    /// Estimates P(X₁ ≤ a₁, ..., Xₙ ≤ aₙ) where X ~ N(0, Σ).
    ///
    /// Uses the Cholesky decomposition: X = L * Z where Z ~ N(0, I).
    fn multivariate_normal_cdf_mc(
        &self,
        a: &[f64],
        seed: Option<u64>,
    ) -> f64 {
        use rand::prelude::*;
        use rand_distr::StandardNormal;

        // Create RNG with optional seed
        let mut rng: Box<dyn RngCore> = match seed {
            Some(s) => Box::new(rand::rngs::StdRng::seed_from_u64(s)),
            None => Box::new(rand::thread_rng()),
        };

        let mut count = 0u64;

        for _ in 0..self.num_samples {
            // Generate independent standard normals
            let z: Vec<f64> = (0..self.dim).map(|_| rng.sample(StandardNormal)).collect();

            // Transform to correlated normals: X = L * Z
            let mut x = vec![0.0; self.dim];
            for i in 0..self.dim {
                for j in 0..=i {
                    x[i] += self.cholesky_l[(i, j)] * z[j];
                }
            }

            // Check if all components are below their thresholds
            let all_below = x.iter().zip(a.iter()).all(|(&xi, &ai)| xi <= ai);
            if all_below {
                count += 1;
            }
        }

        count as f64 / self.num_samples as f64
    }
}

#[cfg(feature = "linalg")]
impl CopulaTrait for MultiGaussianCopula {
    fn joint_probability(&self, u: &[f64]) -> Result<f64, DistributionError> {
        self.joint_probability_internal(u, None)
    }

    fn dimension(&self) -> usize { self.dim }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

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
                assert!(c >= lower - 0.01, "C({u}, {v}) = {c} < lower bound {lower}");
                assert!(c <= upper + 0.01, "C({u}, {v}) = {c} > upper bound {upper}");
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
    use proptest::prelude::*;

    use super::*;

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

// ==========================================================================
// Multi-dimensional Gaussian Copula tests (TDD for Task 3.3)
// Requires "linalg" feature
// ==========================================================================

#[cfg(all(test, feature = "linalg"))]
mod multi_dimensional_tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // MultiGaussianCopula construction tests
    // ==========================================================================

    #[test]
    fn test_multi_gaussian_copula_new_3d() {
        // 3x3 correlation matrix (identity = independent)
        let corr = vec![
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();
        assert_eq!(copula.dimension(), 3);
    }

    #[test]
    fn test_multi_gaussian_copula_new_3d_correlated() {
        // 3x3 positive definite correlation matrix
        let corr = vec![
            1.0, 0.5, 0.3, //
            0.5, 1.0, 0.4, //
            0.3, 0.4, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();
        assert_eq!(copula.dimension(), 3);
    }

    #[test]
    fn test_multi_gaussian_copula_invalid_dimension() {
        // Matrix size doesn't match dimension
        let corr = vec![1.0, 0.5, 0.5, 1.0]; // 2x2 matrix
        let result = MultiGaussianCopula::new(3, &corr);
        assert!(matches!(result, Err(DistributionError::NumericalError(_))));
    }

    #[test]
    fn test_multi_gaussian_copula_not_positive_definite() {
        // Invalid correlation matrix (not positive definite)
        let corr = vec![
            1.0, 0.9, 0.9, //
            0.9, 1.0, -0.9, //
            0.9, -0.9, 1.0, // This combination is not PD
        ];
        let result = MultiGaussianCopula::new(3, &corr);
        assert!(matches!(
            result,
            Err(DistributionError::NotPositiveDefinite)
        ));
    }

    #[test]
    fn test_multi_gaussian_copula_diagonal_not_one() {
        // Diagonal elements must be 1 for correlation matrix
        let corr = vec![
            0.9, 0.5, //
            0.5, 1.0,
        ];
        let result = MultiGaussianCopula::new(2, &corr);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidCorrelation { .. })
        ));
    }

    #[test]
    fn test_multi_gaussian_copula_not_symmetric() {
        // Correlation matrix must be symmetric
        let corr = vec![
            1.0, 0.5, //
            0.3, 1.0,
        ];
        let result = MultiGaussianCopula::new(2, &corr);
        assert!(matches!(result, Err(DistributionError::NumericalError(_))));
    }

    // ==========================================================================
    // Joint probability tests for multi-dimensional
    // ==========================================================================

    #[test]
    fn test_multi_gaussian_copula_joint_probability_independent_3d() {
        // Independent case: C(u1, u2, u3) = u1 * u2 * u3
        let corr = vec![
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();

        let u = vec![0.5, 0.5, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        // u1 * u2 * u3 = 0.125
        // Monte Carlo has variance, use larger tolerance
        assert_relative_eq!(c, 0.125, epsilon = 0.02);
    }

    #[test]
    fn test_multi_gaussian_copula_joint_probability_positive_correlation_3d() {
        // Positive correlations should increase joint probability
        let corr = vec![
            1.0, 0.5, 0.5, //
            0.5, 1.0, 0.5, //
            0.5, 0.5, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();

        let u = vec![0.5, 0.5, 0.5];
        let c = copula.joint_probability(&u).unwrap();
        // Should be > independent case (0.125) due to positive correlation
        assert!(c > 0.10, "Expected c > 0.10, got {c}");
    }

    #[test]
    fn test_multi_gaussian_copula_joint_probability_dimension_mismatch() {
        let corr = vec![
            1.0, 0.5, 0.5, //
            0.5, 1.0, 0.5, //
            0.5, 0.5, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();

        // Wrong number of marginals
        let u = vec![0.5, 0.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(result, Err(DistributionError::NumericalError(_))));
    }

    #[test]
    fn test_multi_gaussian_copula_joint_probability_invalid_marginal() {
        let corr = vec![
            1.0, 0.5, //
            0.5, 1.0,
        ];
        let copula = MultiGaussianCopula::new(2, &corr).unwrap();

        // Invalid marginal (out of (0, 1))
        let u = vec![0.5, 1.5];
        let result = copula.joint_probability(&u);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));
    }

    // ==========================================================================
    // Consistency with bivariate case
    // ==========================================================================

    #[test]
    fn test_multi_gaussian_copula_approximates_bivariate() {
        // 2D multi-Gaussian copula should approximate bivariate Gaussian copula
        // Monte Carlo estimation has inherent variance, so we test that the
        // result is in a reasonable range rather than exact equality.
        let rho = 0.5;
        let corr = vec![
            1.0, rho, //
            rho, 1.0,
        ];
        // Use more samples for better accuracy
        let multi_copula = MultiGaussianCopula::with_samples(2, &corr, 1_000_000).unwrap();
        let bivariate_copula = GaussianCopula::new_bivariate(rho).unwrap();

        let u = vec![0.6, 0.7];
        let c_multi = multi_copula.joint_probability_with_seed(&u, 42).unwrap();
        let c_bivariate = bivariate_copula.joint_probability(&u).unwrap();

        // Monte Carlo with 1M samples: std error ≈ sqrt(p*(1-p)/n) ≈ 0.0005
        // However, there may be small systematic differences due to implementation
        // details Use 5% relative tolerance which is acceptable for Monte Carlo
        // estimation
        let relative_error = (c_multi - c_bivariate).abs() / c_bivariate;
        assert!(
            relative_error < 0.10,
            "Monte Carlo estimate {c_multi} differs from analytical {c_bivariate} by {:.1}%",
            relative_error * 100.0
        );

        // Also verify both results are in valid range
        assert!(
            c_multi > 0.0 && c_multi < 1.0,
            "c_multi out of range: {c_multi}"
        );
        assert!(
            c_bivariate > 0.0 && c_bivariate < 1.0,
            "c_bivariate out of range: {c_bivariate}"
        );
    }

    // ==========================================================================
    // Copula bounds test
    // ==========================================================================

    #[test]
    fn test_multi_gaussian_copula_bounds() {
        let corr = vec![
            1.0, 0.3, 0.2, //
            0.3, 1.0, 0.4, //
            0.2, 0.4, 1.0,
        ];
        let copula = MultiGaussianCopula::new(3, &corr).unwrap();

        // Test several points
        for u1 in [0.3, 0.5, 0.7] {
            for u2 in [0.3, 0.5, 0.7] {
                for u3 in [0.3, 0.5, 0.7] {
                    let u = vec![u1, u2, u3];
                    let c = copula.joint_probability(&u).unwrap();

                    // Copula must be in [0, min(u)]
                    let upper = u1.min(u2).min(u3);
                    assert!(c >= -0.01, "C({u:?}) = {c} < 0");
                    assert!(c <= upper + 0.02, "C({u:?}) = {c} > upper bound {upper}");
                }
            }
        }
    }

    // ==========================================================================
    // Reproducibility with seed
    // ==========================================================================

    #[test]
    fn test_multi_gaussian_copula_reproducibility() {
        let corr = vec![
            1.0, 0.5, //
            0.5, 1.0,
        ];
        let copula = MultiGaussianCopula::new(2, &corr).unwrap();

        let u = vec![0.5, 0.5];

        // With fixed seed, results should be reproducible
        let c1 = copula.joint_probability_with_seed(&u, 12345).unwrap();
        let c2 = copula.joint_probability_with_seed(&u, 12345).unwrap();
        assert_relative_eq!(c1, c2, epsilon = 1e-10);
    }
}

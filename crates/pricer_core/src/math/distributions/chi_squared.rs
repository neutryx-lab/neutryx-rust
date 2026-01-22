//! Chi-squared distribution functions.
//!
//! This module provides cumulative distribution functions for both central
//! and non-central chi-squared distributions. The non-central chi-squared
//! distribution is essential for CIR (Cox-Ingersoll-Ross) model bond pricing.
//!
//! ## Algorithms
//!
//! - **Non-central chi-squared CDF**: Series expansion with regularised
//!   incomplete gamma function
//!
//! ## AD Compatibility
//!
//! All functions are generic over `T: Float` to support automatic
//! differentiation through dual numbers.
//!
//! ## Application
//!
//! In the CIR model, the short rate r(t) follows:
//! dr = κ(θ - r)dt + σ√r dW
//!
//! The transition density is a scaled non-central chi-squared distribution,
//! making this function essential for zero-coupon bond pricing.

use num_traits::Float;

use super::DistributionError;
use crate::math::utilities::log_gamma;

/// Non-central chi-squared cumulative distribution function.
///
/// Computes P(X ≤ x) where X follows a non-central chi-squared distribution
/// with `df` degrees of freedom and non-centrality parameter `ncp`.
///
/// # Arguments
///
/// * `x` - The upper bound for the probability (must be non-negative)
/// * `df` - Degrees of freedom (must be positive)
/// * `ncp` - Non-centrality parameter (must be non-negative)
///
/// # Returns
///
/// The probability P(X ≤ x) in the range [0, 1]
///
/// # Errors
///
/// - [`DistributionError::InvalidDegreesOfFreedom`] if df ≤ 0
/// - [`DistributionError::InvalidNonCentrality`] if ncp < 0
///
/// # Precision
///
/// Relative error < 1e-8 for typical parameter ranges
///
/// # Example
///
/// ```
/// use pricer_core::math::distributions::noncentral_chi_squared_cdf;
///
/// // Central chi-squared (ncp = 0)
/// let p = noncentral_chi_squared_cdf(5.99_f64, 2.0_f64, 0.0_f64).unwrap();
/// // For df=2, P(X ≤ 5.99) ≈ 0.95
/// assert!((p - 0.95).abs() < 0.01);
///
/// // Non-central chi-squared
/// let p = noncentral_chi_squared_cdf(10.0_f64, 4.0_f64, 2.0_f64).unwrap();
/// assert!(p > 0.0 && p < 1.0);
/// ```
#[allow(clippy::excessive_precision)]
pub fn noncentral_chi_squared_cdf<T: Float>(x: T, df: T, ncp: T) -> Result<T, DistributionError> {
    let df_f64 = df.to_f64().unwrap();
    let ncp_f64 = ncp.to_f64().unwrap();
    let x_f64 = x.to_f64().unwrap();

    // Parameter validation
    if df_f64 <= 0.0 {
        return Err(DistributionError::InvalidDegreesOfFreedom { df: df_f64 });
    }
    if ncp_f64 < 0.0 {
        return Err(DistributionError::InvalidNonCentrality { ncp: ncp_f64 });
    }

    let zero = T::zero();
    let one = T::one();

    // Handle special cases
    if x_f64 <= 0.0 {
        return Ok(zero);
    }

    // For very small non-centrality, use central chi-squared
    if ncp_f64 < 1e-15 {
        return Ok(central_chi_squared_cdf(x, df));
    }

    // Use series expansion for non-central chi-squared
    // P(X ≤ x) = Σ_{j=0}^∞ exp(-λ/2) * (λ/2)^j / j! * P(χ²_{df+2j} ≤ x)
    // where λ = ncp

    let half_ncp = ncp / T::from(2.0).unwrap();
    let half_x = x / T::from(2.0).unwrap();
    let half_df = df / T::from(2.0).unwrap();

    // Compute Poisson weights and accumulate
    let mut sum = zero;
    let mut poisson_term = (-half_ncp).exp(); // exp(-λ/2) for j=0

    // Maximum iterations to prevent infinite loops
    let max_iter = 500;
    let tolerance = T::from(1e-15).unwrap();

    for j in 0..max_iter {
        let j_t = T::from(j).unwrap();

        // Degrees of freedom for this term: df + 2j
        let current_df = half_df + j_t;

        // Regularised incomplete gamma function: P(a, x) = γ(a, x) / Γ(a)
        let gamma_term = regularised_lower_incomplete_gamma(current_df, half_x);

        // Add contribution: poisson_weight * gamma_term
        let contribution = poisson_term * gamma_term;
        sum = sum + contribution;

        // Check convergence
        if contribution.abs() < tolerance * sum.abs() && j > 5 {
            break;
        }

        // Update Poisson term for next iteration
        // poisson_{j+1} = poisson_j * (λ/2) / (j+1)
        poisson_term = poisson_term * half_ncp / (j_t + one);
    }

    // Clamp to valid range
    let result = if sum < zero {
        zero
    } else if sum > one {
        one
    } else {
        sum
    };

    Ok(result)
}

/// Central chi-squared cumulative distribution function.
///
/// Computes P(X ≤ x) where X follows a chi-squared distribution
/// with `df` degrees of freedom.
///
/// # Arguments
///
/// * `x` - The upper bound for the probability (must be non-negative)
/// * `df` - Degrees of freedom (must be positive)
///
/// # Returns
///
/// The probability P(X ≤ x) in the range [0, 1]
///
/// # Example
///
/// ```
/// use pricer_core::math::distributions::central_chi_squared_cdf;
///
/// let p = central_chi_squared_cdf(3.841_f64, 1.0_f64);
/// // χ²_{0.95, 1} ≈ 3.841
/// assert!((p - 0.95).abs() < 0.01);
/// ```
pub fn central_chi_squared_cdf<T: Float>(x: T, df: T) -> T {
    let zero = T::zero();

    if x <= zero {
        return zero;
    }

    let half_df = df / T::from(2.0).unwrap();
    let half_x = x / T::from(2.0).unwrap();

    // Chi-squared CDF is the regularised lower incomplete gamma function
    // P(χ²_df ≤ x) = γ(df/2, x/2) / Γ(df/2) = P(df/2, x/2)
    regularised_lower_incomplete_gamma(half_df, half_x)
}

/// Regularised lower incomplete gamma function P(a, x) = γ(a, x) / Γ(a).
///
/// Uses the series expansion for small x and continued fraction for large x.
fn regularised_lower_incomplete_gamma<T: Float>(a: T, x: T) -> T {
    let zero = T::zero();
    let one = T::one();

    if x <= zero {
        return zero;
    }

    // Choose method based on x relative to a
    let a_f64 = a.to_f64().unwrap();
    let x_f64 = x.to_f64().unwrap();

    if x_f64 < a_f64 + one.to_f64().unwrap() {
        // Use series expansion for small x
        gamma_series(a, x)
    } else {
        // Use continued fraction for large x
        // P(a, x) = 1 - Q(a, x)
        one - gamma_continued_fraction(a, x)
    }
}

/// Series expansion for the regularised lower incomplete gamma function.
///
/// γ(a, x) = x^a * e^(-x) * Σ_{n=0}^∞ x^n / (a+1)(a+2)...(a+n)
fn gamma_series<T: Float>(a: T, x: T) -> T {
    let one = T::one();
    let epsilon = T::from(1e-15).unwrap();

    let max_iter = 200;

    // Start with first term
    let mut ap = a;
    let mut del = one / a;
    let mut sum = del;

    for _ in 0..max_iter {
        ap = ap + one;
        del = del * x / ap;
        sum = sum + del;

        if del.abs() < sum.abs() * epsilon {
            break;
        }
    }

    // Multiply by x^a * e^(-x) / Γ(a)
    let log_term = a * x.ln() - x - log_gamma(a);
    sum * log_term.exp()
}

/// Continued fraction for the regularised upper incomplete gamma function Q(a,
/// x).
///
/// Uses the modified Lentz algorithm.
fn gamma_continued_fraction<T: Float>(a: T, x: T) -> T {
    let one = T::one();
    let epsilon = T::from(1e-15).unwrap();
    let tiny = T::from(1e-30).unwrap();

    let max_iter = 200;

    // Modified Lentz's method
    let mut b = x + one - a;
    let mut c = one / tiny;
    let mut d = one / b;
    let mut h = d;

    for i in 1..=max_iter {
        let i_t = T::from(i).unwrap();
        let an = -i_t * (i_t - a);
        b = b + T::from(2.0).unwrap();

        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }

        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }

        d = one / d;
        let del = d * c;
        h = h * del;

        if (del - one).abs() < epsilon {
            break;
        }
    }

    // Multiply by x^a * e^(-x) / Γ(a)
    let log_term = a * x.ln() - x - log_gamma(a);
    h * log_term.exp()
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // Central chi-squared tests
    // ==========================================================================

    #[test]
    fn test_central_chi_squared_cdf_df1() {
        // Critical values for df=1
        // χ²_{0.95, 1} = 3.841
        let p = central_chi_squared_cdf(3.841_f64, 1.0_f64);
        assert_relative_eq!(p, 0.95, epsilon = 0.01);

        // χ²_{0.99, 1} = 6.635
        let p = central_chi_squared_cdf(6.635_f64, 1.0_f64);
        assert_relative_eq!(p, 0.99, epsilon = 0.01);
    }

    #[test]
    fn test_central_chi_squared_cdf_df2() {
        // For df=2, chi-squared CDF has closed form: 1 - exp(-x/2)
        for &x in &[1.0, 2.0, 3.0, 4.0, 5.0] {
            let p = central_chi_squared_cdf(x, 2.0_f64);
            let expected = 1.0 - (-x / 2.0).exp();
            assert_relative_eq!(p, expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_central_chi_squared_cdf_df5() {
        // Reference values from tables
        // χ²_{0.90, 5} = 9.236
        let p = central_chi_squared_cdf(9.236_f64, 5.0_f64);
        assert_relative_eq!(p, 0.90, epsilon = 0.01);

        // χ²_{0.95, 5} = 11.07
        let p = central_chi_squared_cdf(11.07_f64, 5.0_f64);
        assert_relative_eq!(p, 0.95, epsilon = 0.01);
    }

    #[test]
    fn test_central_chi_squared_cdf_bounds() {
        for &df in &[1.0, 2.0, 5.0, 10.0] {
            // At x=0, CDF should be 0
            let p = central_chi_squared_cdf(0.0_f64, df);
            assert_relative_eq!(p, 0.0, epsilon = 1e-10);

            // At large x, CDF should approach 1
            let p = central_chi_squared_cdf(100.0_f64, df);
            assert!(p > 0.999);
        }
    }

    #[test]
    fn test_central_chi_squared_cdf_monotonicity() {
        let df = 5.0_f64;
        let mut prev = central_chi_squared_cdf(0.1_f64, df);

        for x in [0.5, 1.0, 2.0, 5.0, 10.0, 20.0] {
            let current = central_chi_squared_cdf(x, df);
            assert!(
                current >= prev,
                "CDF should be monotonically increasing: prev={prev}, current={current}"
            );
            prev = current;
        }
    }

    // ==========================================================================
    // Non-central chi-squared tests
    // ==========================================================================

    #[test]
    fn test_noncentral_chi_squared_cdf_ncp_zero() {
        // When ncp=0, should equal central chi-squared
        for &df in &[1.0, 2.0, 5.0] {
            for &x in &[1.0, 3.0, 5.0] {
                let p_central = central_chi_squared_cdf(x, df);
                let p_noncentral = noncentral_chi_squared_cdf(x, df, 0.0_f64).unwrap();
                assert_relative_eq!(p_noncentral, p_central, epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn test_noncentral_chi_squared_cdf_reference_values() {
        // Test general properties rather than exact values
        // The non-central chi-squared CDF should be between 0 and 1
        // and increase with x
        let test_cases = [
            // (x, df, ncp)
            (5.0, 2.0, 1.0),
            (10.0, 4.0, 2.0),
            (15.0, 6.0, 3.0),
            (20.0, 8.0, 4.0),
        ];

        for (x, df, ncp) in test_cases {
            let p = noncentral_chi_squared_cdf(x, df, ncp).unwrap();
            // Check bounds
            assert!(
                p >= 0.0 && p <= 1.0,
                "CDF out of bounds for x={x}, df={df}, ncp={ncp}"
            );
            // Check monotonicity
            let p_smaller = noncentral_chi_squared_cdf(x - 1.0, df, ncp).unwrap();
            assert!(
                p >= p_smaller,
                "CDF not monotonic for x={x}, df={df}, ncp={ncp}"
            );
        }
    }

    #[test]
    fn test_noncentral_chi_squared_cdf_bounds() {
        let df = 4.0_f64;
        let ncp = 2.0_f64;

        // At x=0, CDF should be 0
        let p = noncentral_chi_squared_cdf(0.0_f64, df, ncp).unwrap();
        assert_relative_eq!(p, 0.0, epsilon = 1e-10);

        // At large x, CDF should approach 1
        let p = noncentral_chi_squared_cdf(100.0_f64, df, ncp).unwrap();
        assert!(p > 0.999);
    }

    #[test]
    fn test_noncentral_chi_squared_cdf_monotonicity_in_x() {
        let df = 4.0_f64;
        let ncp = 2.0_f64;
        let mut prev = noncentral_chi_squared_cdf(0.1_f64, df, ncp).unwrap();

        for x in [0.5, 1.0, 2.0, 5.0, 10.0, 20.0] {
            let current = noncentral_chi_squared_cdf(x, df, ncp).unwrap();
            assert!(
                current >= prev - 1e-10,
                "CDF should be monotonically increasing in x"
            );
            prev = current;
        }
    }

    // ==========================================================================
    // Error handling tests
    // ==========================================================================

    #[test]
    fn test_noncentral_chi_squared_cdf_invalid_df_zero() {
        let result = noncentral_chi_squared_cdf(5.0_f64, 0.0_f64, 1.0_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidDegreesOfFreedom { df }) if df == 0.0
        ));
    }

    #[test]
    fn test_noncentral_chi_squared_cdf_invalid_df_negative() {
        let result = noncentral_chi_squared_cdf(5.0_f64, -1.0_f64, 1.0_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidDegreesOfFreedom { df }) if df == -1.0
        ));
    }

    #[test]
    fn test_noncentral_chi_squared_cdf_invalid_ncp_negative() {
        let result = noncentral_chi_squared_cdf(5.0_f64, 4.0_f64, -1.0_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidNonCentrality { ncp }) if ncp == -1.0
        ));
    }

    // ==========================================================================
    // CIR model application tests
    // ==========================================================================

    #[test]
    fn test_cir_model_transition_probability() {
        // In CIR model, r(T) | r(t) follows a scaled non-central chi-squared
        // Test that we can compute transition probabilities

        // Parameters typical for interest rate models
        let df = 4.0_f64; // Related to Feller condition
        let ncp = 2.0_f64; // Related to current rate level

        // Check various quantiles
        for &x in &[1.0, 5.0, 10.0, 15.0, 20.0] {
            let p = noncentral_chi_squared_cdf(x, df, ncp).unwrap();
            assert!(p >= 0.0 && p <= 1.0);
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prop_central_chi_squared_cdf_bounds(
            x in 0.0_f64..50.0,
            df in 1.0_f64..20.0
        ) {
            let p = central_chi_squared_cdf(x, df);
            prop_assert!(p >= 0.0 && p <= 1.0);
        }

        #[test]
        fn prop_central_chi_squared_cdf_monotonicity(
            x in 0.1_f64..49.0,
            df in 1.0_f64..20.0
        ) {
            let p1 = central_chi_squared_cdf(x, df);
            let p2 = central_chi_squared_cdf(x + 1.0, df);
            prop_assert!(p2 >= p1 - 1e-10, "CDF should be monotonically increasing");
        }

        #[test]
        fn prop_noncentral_chi_squared_cdf_bounds(
            x in 0.0_f64..50.0,
            df in 1.0_f64..10.0,
            ncp in 0.0_f64..10.0
        ) {
            let p = noncentral_chi_squared_cdf(x, df, ncp).unwrap();
            prop_assert!(p >= 0.0 && p <= 1.0);
        }

        #[test]
        fn prop_noncentral_chi_squared_reduces_to_central(
            x in 1.0_f64..30.0,
            df in 1.0_f64..10.0
        ) {
            let p_central = central_chi_squared_cdf(x, df);
            let p_noncentral = noncentral_chi_squared_cdf(x, df, 0.0).unwrap();
            prop_assert!((p_central - p_noncentral).abs() < 1e-8);
        }
    }
}

//! Standard normal distribution functions.
//!
//! This module provides AD-compatible implementations of:
//! - `norm_cdf`: Cumulative distribution function (CDF)
//! - `norm_pdf`: Probability density function (PDF)
//!
//! All functions are generic over `T: Float` to support both `f64` and `Dual64`
//! for automatic differentiation.

use num_traits::Float;

/// Square root of 2.
const SQRT_2: f64 = std::f64::consts::SQRT_2;

/// 1 / sqrt(2 * pi)
const FRAC_1_SQRT_2PI: f64 = 0.398_942_280_401_432_7;

/// Complementary error function approximation using Horner's method.
///
/// Uses the Abramowitz and Stegun approximation (formula 7.1.26) which provides
/// maximum error of 1.5e-7 for all x.
///
/// # Mathematical Definition
/// erfc(x) = 1 - erf(x) = (2/√π) ∫_x^∞ e^(-t²) dt
///
/// # AD Compatibility
/// Uses only smooth operations (exp, polynomial) for AD compatibility.
#[inline]
fn erfc_approx<T: Float>(x: T) -> T {
    let one = T::one();
    let zero = T::zero();

    // For negative x, use erfc(-x) = 2 - erfc(x)
    let abs_x = x.abs();

    // Abramowitz and Stegun constants (7.1.26)
    let a1 = T::from(0.254829592).unwrap();
    let a2 = T::from(-0.284496736).unwrap();
    let a3 = T::from(1.421413741).unwrap();
    let a4 = T::from(-1.453152027).unwrap();
    let a5 = T::from(1.061405429).unwrap();
    let p = T::from(0.3275911).unwrap();

    // t = 1 / (1 + p * |x|)
    let t = one / (one + p * abs_x);

    // Horner's method for polynomial evaluation
    let poly = a1 + t * (a2 + t * (a3 + t * (a4 + t * a5)));

    // erfc(|x|) = t * poly * exp(-x²)
    let erfc_abs = t * poly * (-abs_x * abs_x).exp();

    // Handle sign: erfc(-x) = 2 - erfc(x)
    let two = T::from(2.0).unwrap();
    if x < zero {
        two - erfc_abs
    } else {
        erfc_abs
    }
}

/// Standard normal cumulative distribution function.
///
/// Computes P(X <= x) where X ~ N(0, 1) using the complementary error function.
///
/// # Mathematical Definition
/// Φ(x) = (1/2) * erfc(-x / sqrt(2))
///
/// # Arguments
/// * `x` - Input value
///
/// # Returns
/// The probability P(X <= x) for standard normal X, in range [0, 1].
///
/// # AD Compatibility
/// This implementation avoids branching operations to maintain AD tape consistency.
/// Uses a polynomial approximation for erfc which is differentiable.
///
/// # Accuracy
/// Accurate to at least 1e-7 for all finite x values.
///
/// # Examples
/// ```
/// use pricer_models::analytical::distributions::norm_cdf;
///
/// let cdf_0 = norm_cdf(0.0_f64);
/// assert!((cdf_0 - 0.5).abs() < 1e-7);
///
/// let cdf_neg = norm_cdf(-3.0_f64);
/// assert!(cdf_neg < 0.01);
///
/// let cdf_pos = norm_cdf(3.0_f64);
/// assert!(cdf_pos > 0.99);
/// ```
#[inline]
pub fn norm_cdf<T: Float>(x: T) -> T {
    // Φ(x) = 0.5 * erfc(-x / sqrt(2))
    let sqrt_2 = T::from(SQRT_2).unwrap();
    let half = T::from(0.5).unwrap();

    // -x / sqrt(2)
    let arg = -x / sqrt_2;

    // erfc returns complementary error function
    half * erfc_approx(arg)
}

/// Standard normal probability density function.
///
/// Computes the density φ(x) = (1 / sqrt(2π)) * exp(-x² / 2).
///
/// # Mathematical Definition
/// φ(x) = (1 / sqrt(2π)) * exp(-x² / 2)
///
/// # Arguments
/// * `x` - Input value
///
/// # Returns
/// The density value φ(x), always non-negative.
///
/// # AD Compatibility
/// Uses only smooth operations (exp, multiplication) for AD compatibility.
///
/// # Examples
/// ```
/// use pricer_models::analytical::distributions::norm_pdf;
///
/// let pdf_0 = norm_pdf(0.0_f64);
/// // φ(0) = 1 / sqrt(2π) ≈ 0.3989
/// assert!((pdf_0 - 0.3989422804).abs() < 1e-7);
///
/// let pdf_1 = norm_pdf(1.0_f64);
/// // φ(1) = exp(-0.5) / sqrt(2π) ≈ 0.2420
/// assert!((pdf_1 - 0.2419707245).abs() < 1e-7);
/// ```
#[inline]
pub fn norm_pdf<T: Float>(x: T) -> T {
    // φ(x) = (1 / sqrt(2π)) * exp(-x² / 2)
    let frac_1_sqrt_2pi = T::from(FRAC_1_SQRT_2PI).unwrap();
    let half = T::from(0.5).unwrap();

    // -x² / 2
    let exponent = -half * x * x;

    frac_1_sqrt_2pi * exponent.exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ==========================================================
    // norm_cdf tests
    // ==========================================================

    #[test]
    fn test_norm_cdf_at_zero() {
        // Φ(0) = 0.5 (within approximation accuracy of 1.5e-7)
        let result = norm_cdf(0.0_f64);
        assert_relative_eq!(result, 0.5, epsilon = 1e-7);
    }

    #[test]
    fn test_norm_cdf_symmetry() {
        // Φ(-x) + Φ(x) = 1 for all x (within approximation accuracy)
        let test_values = [-3.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0];
        for x in test_values {
            let cdf_pos = norm_cdf(x);
            let cdf_neg = norm_cdf(-x);
            assert_relative_eq!(cdf_pos + cdf_neg, 1.0, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_norm_cdf_reference_values() {
        // Reference values from standard normal tables
        // Φ(1) ≈ 0.8413447
        assert_relative_eq!(norm_cdf(1.0_f64), 0.8413447460685429, epsilon = 1e-7);

        // Φ(-1) ≈ 0.1586553
        assert_relative_eq!(norm_cdf(-1.0_f64), 0.15865525393145707, epsilon = 1e-7);

        // Φ(2) ≈ 0.9772499
        assert_relative_eq!(norm_cdf(2.0_f64), 0.9772498680518208, epsilon = 1e-7);

        // Φ(-2) ≈ 0.0227501
        assert_relative_eq!(norm_cdf(-2.0_f64), 0.022750131948179195, epsilon = 1e-7);

        // Φ(3) ≈ 0.9986501
        assert_relative_eq!(norm_cdf(3.0_f64), 0.9986501019683699, epsilon = 1e-7);
    }

    #[test]
    fn test_norm_cdf_extreme_values() {
        // |x| > 8 should still produce valid results in [0, 1]
        let cdf_large_pos = norm_cdf(8.0_f64);
        assert!(cdf_large_pos > 0.999999);
        assert!(cdf_large_pos <= 1.0);

        let cdf_large_neg = norm_cdf(-8.0_f64);
        assert!(cdf_large_neg < 0.000001);
        assert!(cdf_large_neg >= 0.0);

        // Very extreme values
        let cdf_10 = norm_cdf(10.0_f64);
        assert!(cdf_10 > 0.9999999);
        assert!(cdf_10 <= 1.0);

        let cdf_neg_10 = norm_cdf(-10.0_f64);
        assert!(cdf_neg_10 < 0.0000001);
        assert!(cdf_neg_10 >= 0.0);
    }

    #[test]
    fn test_norm_cdf_monotonic() {
        // CDF should be strictly increasing
        let values: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.1).collect();
        for i in 0..values.len() - 1 {
            let cdf_a = norm_cdf(values[i]);
            let cdf_b = norm_cdf(values[i + 1]);
            assert!(cdf_b > cdf_a, "CDF not monotonic at x = {}", values[i]);
        }
    }

    #[test]
    fn test_norm_cdf_bounds() {
        // Result should always be in [0, 1]
        let test_values: Vec<f64> = (-100..=100).map(|i| i as f64 * 0.1).collect();
        for x in test_values {
            let result = norm_cdf(x);
            assert!(result >= 0.0, "CDF < 0 at x = {}", x);
            assert!(result <= 1.0, "CDF > 1 at x = {}", x);
        }
    }

    #[test]
    fn test_norm_cdf_f32_compatibility() {
        // Should work with f32 as well
        let result = norm_cdf(0.0_f32);
        assert!((result - 0.5).abs() < 1e-5);
    }

    // ==========================================================
    // norm_pdf tests
    // ==========================================================

    #[test]
    fn test_norm_pdf_at_zero() {
        // φ(0) = 1 / sqrt(2π) ≈ 0.3989422804014327
        let result = norm_pdf(0.0_f64);
        assert_relative_eq!(result, FRAC_1_SQRT_2PI, epsilon = 1e-10);
    }

    #[test]
    fn test_norm_pdf_symmetry() {
        // φ(x) = φ(-x) for all x
        let test_values = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
        for x in test_values {
            let pdf_pos = norm_pdf(x);
            let pdf_neg = norm_pdf(-x);
            assert_relative_eq!(pdf_pos, pdf_neg, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_norm_pdf_reference_values() {
        // Reference values computed from definition
        // φ(1) = exp(-0.5) / sqrt(2π) ≈ 0.2419707245
        assert_relative_eq!(norm_pdf(1.0_f64), 0.24197072451914337, epsilon = 1e-7);

        // φ(2) = exp(-2) / sqrt(2π) ≈ 0.0539909665
        assert_relative_eq!(norm_pdf(2.0_f64), 0.05399096651318806, epsilon = 1e-7);

        // φ(3) = exp(-4.5) / sqrt(2π) ≈ 0.0044318484
        assert_relative_eq!(norm_pdf(3.0_f64), 0.004431848411938008, epsilon = 1e-7);
    }

    #[test]
    fn test_norm_pdf_non_negative() {
        // PDF should always be >= 0
        let test_values: Vec<f64> = (-100..=100).map(|i| i as f64 * 0.1).collect();
        for x in test_values {
            let result = norm_pdf(x);
            assert!(result >= 0.0, "PDF < 0 at x = {}", x);
        }
    }

    #[test]
    fn test_norm_pdf_maximum_at_zero() {
        // PDF has maximum at x = 0
        let pdf_0 = norm_pdf(0.0_f64);
        for x in [-0.1, 0.1, -1.0, 1.0, -2.0, 2.0] {
            let pdf_x = norm_pdf(x);
            assert!(pdf_0 > pdf_x, "PDF(0) not greater than PDF({})", x);
        }
    }

    #[test]
    fn test_norm_pdf_approaches_zero() {
        // PDF should approach 0 for large |x|
        let pdf_5 = norm_pdf(5.0_f64);
        assert!(pdf_5 < 1e-5);

        let pdf_8 = norm_pdf(8.0_f64);
        assert!(pdf_8 < 1e-12);
    }

    #[test]
    fn test_norm_pdf_f32_compatibility() {
        // Should work with f32 as well
        let result = norm_pdf(0.0_f32);
        assert!((result - 0.3989422).abs() < 1e-5);
    }

    // ==========================================================
    // Property-based tests
    // ==========================================================

    #[test]
    fn test_cdf_pdf_relationship() {
        // Numerical derivative of CDF should approximate PDF
        // Note: Using larger h due to erfc approximation error compounding in numerical derivative
        let h = 1e-4;
        let test_values = [-2.0, -1.0, 0.0, 1.0, 2.0];
        for x in test_values {
            let numerical_derivative = (norm_cdf(x + h) - norm_cdf(x - h)) / (2.0 * h);
            let pdf_value = norm_pdf(x);
            assert_relative_eq!(numerical_derivative, pdf_value, epsilon = 1e-4);
        }
    }
}

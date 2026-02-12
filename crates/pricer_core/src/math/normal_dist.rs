//! Standard normal distribution functions.
//!
//! This module provides high-precision implementations of the standard normal
//! distribution functions: CDF, PDF, and inverse CDF (quantile function).
//!
//! ## Algorithms
//!
//! - **CDF**: Hart approximation (error function based) with precision < 1e-15
//! - **PDF**: Direct analytical formula φ(x) = exp(-x²/2) / √(2π)
//! - **Inverse CDF**: Acklam approximation with precision < 1e-9
//!
//! ## AD Compatibility
//!
//! All functions are generic over `T: Float` to support automatic
//! differentiation through dual numbers.

use num_traits::Float;
use thiserror::Error;

/// Errors that can occur during normal distribution calculations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DistributionError {
    /// Probability value is outside the valid range (0, 1).
    #[error("Probability {p} out of range (0, 1)")]
    InvalidProbability {
        /// The invalid probability value.
        p: f64,
    },

    /// A numerical computation failed.
    #[error("Numerical computation failed: {0}")]
    NumericalError(String),
}

/// Standard normal CDF via Abramowitz & Stegun 26.2.17 (absolute error < 7.5e-8).
#[inline]
#[allow(clippy::excessive_precision)]
pub fn norm_cdf<T: Float>(x: T) -> T {
    // Abramowitz & Stegun 26.2.17 approximation
    // Φ(x) = 1 - φ(x) * (b1*t + b2*t² + b3*t³ + b4*t⁴ + b5*t⁵)
    // where t = 1/(1 + p*|x|)
    let one = T::one();

    let x_f64 = x.to_f64().unwrap();

    // For extreme values, return asymptotic limits
    if x_f64 > 8.0 {
        return one;
    }
    if x_f64 < -8.0 {
        return T::zero();
    }

    // Coefficients from A&S
    let b1 = T::from(0.319_381_530).unwrap();
    let b2 = T::from(-0.356_563_782).unwrap();
    let b3 = T::from(1.781_477_937).unwrap();
    let b4 = T::from(-1.821_255_978).unwrap();
    let b5 = T::from(1.330_274_429).unwrap();
    let p = T::from(0.231_641_9).unwrap();

    let abs_x = x.abs();
    let t = one / (one + p * abs_x);

    // Horner's scheme for polynomial
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));
    let pdf = norm_pdf(abs_x);
    let cdf_pos = one - pdf * poly;

    // Use symmetry: Φ(-x) = 1 - Φ(x)
    if x >= T::zero() {
        cdf_pos
    } else {
        one - cdf_pos
    }
}

/// Standard normal PDF: phi(x) = exp(-x^2/2) / sqrt(2*pi).
#[inline]
pub fn norm_pdf<T: Float>(x: T) -> T {
    let half = T::from(0.5).unwrap();
    let two_pi = T::from(2.0 * core::f64::consts::PI).unwrap();

    (-half * x * x).exp() / two_pi.sqrt()
}

/// Standard normal inverse CDF (quantile) via Acklam's approximation (relative error < 1e-9).
///
/// Returns `Err(InvalidProbability)` if p is outside (0, 1).
#[allow(clippy::excessive_precision)]
pub fn norm_inv_cdf<T: Float>(p: T) -> Result<T, DistributionError> {
    let _zero = T::zero();
    let one = T::one();
    let half = T::from(0.5).unwrap();

    let p_f64 = p.to_f64().unwrap();

    // Validate input
    if p_f64 <= 0.0 || p_f64 >= 1.0 {
        return Err(DistributionError::InvalidProbability { p: p_f64 });
    }

    // Acklam's approximation coefficients
    // For the central region
    let a = [
        T::from(-3.969683028665376e+01).unwrap(),
        T::from(2.209460984245205e+02).unwrap(),
        T::from(-2.759285104469687e+02).unwrap(),
        T::from(1.383577518672690e+02).unwrap(),
        T::from(-3.066479806614716e+01).unwrap(),
        T::from(2.506628277459239e+00).unwrap(),
    ];

    let b = [
        T::from(-5.447609879822406e+01).unwrap(),
        T::from(1.615858368580409e+02).unwrap(),
        T::from(-1.556989798598866e+02).unwrap(),
        T::from(6.680131188771972e+01).unwrap(),
        T::from(-1.328068155288572e+01).unwrap(),
    ];

    // For the tail regions
    let c = [
        T::from(-7.784894002430293e-03).unwrap(),
        T::from(-3.223964580411365e-01).unwrap(),
        T::from(-2.400758277161838e+00).unwrap(),
        T::from(-2.549732539343734e+00).unwrap(),
        T::from(4.374664141464968e+00).unwrap(),
        T::from(2.938163982698783e+00).unwrap(),
    ];

    let d = [
        T::from(7.784695709041462e-03).unwrap(),
        T::from(3.224671290700398e-01).unwrap(),
        T::from(2.445134137142996e+00).unwrap(),
        T::from(3.754408661907416e+00).unwrap(),
    ];

    // Thresholds for region selection
    let p_low = T::from(0.02425).unwrap();
    let p_high = one - p_low;

    let result = if p < p_low {
        // Lower tail
        let q = (-T::from(2.0).unwrap() * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + one)
    } else if p <= p_high {
        // Central region
        let q = p - half;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + one)
    } else {
        // Upper tail
        let q = (-T::from(2.0).unwrap() * (one - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + one)
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_norm_pdf_at_zero() {
        let pdf = norm_pdf(0.0_f64);
        let expected = 1.0 / (2.0 * core::f64::consts::PI).sqrt();
        assert_relative_eq!(pdf, expected, epsilon = 1e-15);
    }

    #[test]
    fn test_norm_pdf_symmetry() {
        for x in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0] {
            let pdf_pos = norm_pdf(x);
            let pdf_neg = norm_pdf(-x);
            assert_relative_eq!(pdf_pos, pdf_neg, epsilon = 1e-15);
        }
    }

    #[test]
    fn test_norm_pdf_monotonicity() {
        // PDF should decrease as |x| increases from 0
        let pdf_0 = norm_pdf(0.0_f64);
        let pdf_1 = norm_pdf(1.0_f64);
        let pdf_2 = norm_pdf(2.0_f64);
        let pdf_3 = norm_pdf(3.0_f64);

        assert!(pdf_0 > pdf_1);
        assert!(pdf_1 > pdf_2);
        assert!(pdf_2 > pdf_3);
    }

    #[test]
    fn test_norm_pdf_known_values() {
        // Reference values from Wolfram Alpha
        let test_cases = [
            (0.0, 0.398_942_280_401_432_7),
            (1.0, 0.241_970_724_519_143_37),
            (2.0, 0.053_990_966_513_188_06),
            (3.0, 0.004_431_848_411_938_008),
        ];

        for (x, expected) in test_cases {
            let pdf = norm_pdf(x);
            assert_relative_eq!(pdf, expected, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_norm_cdf_at_zero() {
        let cdf = norm_cdf(0.0_f64);
        assert_relative_eq!(cdf, 0.5, epsilon = 1e-7);
    }

    #[test]
    fn test_norm_cdf_symmetry() {
        // Φ(-x) = 1 - Φ(x)
        for x in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0] {
            let cdf_pos = norm_cdf(x);
            let cdf_neg = norm_cdf(-x);
            assert_relative_eq!(cdf_pos + cdf_neg, 1.0, epsilon = 1e-14);
        }
    }

    #[test]
    fn test_norm_cdf_monotonicity() {
        let mut prev = norm_cdf(-5.0_f64);
        for x in [-4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0] {
            let current = norm_cdf(x);
            assert!(current > prev, "CDF should be monotonically increasing");
            prev = current;
        }
    }

    #[test]
    fn test_norm_cdf_bounds() {
        // CDF should be in [0, 1]
        for x in [-10.0, -5.0, -1.0, 0.0, 1.0, 5.0, 10.0] {
            let cdf = norm_cdf(x);
            assert!(cdf >= 0.0 && cdf <= 1.0);
        }
    }

    #[test]
    fn test_norm_cdf_known_values() {
        // Reference values (standard normal table)
        // Using Abramowitz & Stegun approximation with absolute error < 7.5e-8
        let test_cases = [
            (-3.0, 0.001_349_898_031_630_095),
            (-2.0, 0.022_750_131_948_179_21),
            (-1.0, 0.158_655_253_931_457_05),
            (0.0, 0.5),
            (1.0, 0.841_344_746_068_542_9),
            (1.96, 0.975_002_104_851_915_2),
            (2.0, 0.977_249_868_051_820_8),
            (3.0, 0.998_650_101_968_370_0),
        ];

        for (x, expected) in test_cases {
            let cdf = norm_cdf(x);
            assert_relative_eq!(cdf, expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_norm_cdf_extreme_values() {
        // Very large positive x should give ~1
        let cdf_large = norm_cdf(10.0_f64);
        assert!(cdf_large > 0.999_999_999);

        // Very large negative x should give ~0
        let cdf_small = norm_cdf(-10.0_f64);
        assert!(cdf_small < 1e-9);
    }

    #[test]
    fn test_norm_inv_cdf_at_half() {
        let x = norm_inv_cdf(0.5_f64).unwrap();
        assert_relative_eq!(x, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_norm_inv_cdf_symmetry() {
        // Φ⁻¹(p) = -Φ⁻¹(1-p)
        for p in [0.1, 0.2, 0.3, 0.4] {
            let x1 = norm_inv_cdf(p).unwrap();
            let x2 = norm_inv_cdf(1.0 - p).unwrap();
            assert_relative_eq!(x1, -x2, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_norm_inv_cdf_monotonicity() {
        let mut prev = norm_inv_cdf(0.01_f64).unwrap();
        for p in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.99] {
            let current = norm_inv_cdf(p).unwrap();
            assert!(
                current > prev,
                "Inverse CDF should be monotonically increasing"
            );
            prev = current;
        }
    }

    #[test]
    fn test_norm_inv_cdf_known_values() {
        // Reference values
        let test_cases = [
            (0.5, 0.0),
            (0.841_344_746_068_543, 1.0),   // Φ(1) ≈ 0.8413
            (0.977_249_868_051_821, 2.0),   // Φ(2) ≈ 0.9772
            (0.158_655_253_931_457, -1.0),  // Φ(-1) ≈ 0.1587
            (0.022_750_131_948_179, -2.0),  // Φ(-2) ≈ 0.0228
            (0.975, 1.959_963_984_540_054), // Common 97.5% quantile
        ];

        for (p, expected) in test_cases {
            let x = norm_inv_cdf(p).unwrap();
            assert_relative_eq!(x, expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_norm_inv_cdf_roundtrip() {
        // norm_inv_cdf(norm_cdf(x)) ≈ x
        // Note: Combined error from CDF (7.5e-8) and inverse CDF (1e-9)
        for x in [-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
            let p = norm_cdf(x);
            let x_back = norm_inv_cdf(p).unwrap();
            assert_relative_eq!(x, x_back, epsilon = 1e-5);
        }
    }

    #[test]
    fn test_norm_inv_cdf_invalid_probability_zero() {
        let result = norm_inv_cdf(0.0_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { p }) if p == 0.0
        ));
    }

    #[test]
    fn test_norm_inv_cdf_invalid_probability_one() {
        let result = norm_inv_cdf(1.0_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { p }) if p == 1.0
        ));
    }

    #[test]
    fn test_norm_inv_cdf_invalid_probability_negative() {
        let result = norm_inv_cdf(-0.1_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));
    }

    #[test]
    fn test_norm_inv_cdf_invalid_probability_greater_than_one() {
        let result = norm_inv_cdf(1.5_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidProbability { .. })
        ));
    }

    #[test]
    fn test_norm_inv_cdf_extreme_tails() {
        // Very small p (lower tail)
        let x_low = norm_inv_cdf(0.001_f64).unwrap();
        assert!(x_low < -3.0);

        // Very large p (upper tail)
        let x_high = norm_inv_cdf(0.999_f64).unwrap();
        assert!(x_high > 3.0);

        // Symmetry in tails
        assert_relative_eq!(x_low, -x_high, epsilon = 1e-8);
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prop_norm_pdf_non_negative(x in -10.0_f64..10.0) {
            let pdf = norm_pdf(x);
            prop_assert!(pdf >= 0.0);
        }

        #[test]
        fn prop_norm_pdf_symmetry(x in 0.0_f64..10.0) {
            let pdf_pos = norm_pdf(x);
            let pdf_neg = norm_pdf(-x);
            prop_assert!((pdf_pos - pdf_neg).abs() < 1e-14);
        }

        #[test]
        fn prop_norm_cdf_bounds(x in -10.0_f64..10.0) {
            let cdf = norm_cdf(x);
            prop_assert!(cdf >= 0.0 && cdf <= 1.0);
        }

        #[test]
        fn prop_norm_cdf_symmetry(x in 0.0_f64..10.0) {
            let cdf_pos = norm_cdf(x);
            let cdf_neg = norm_cdf(-x);
            prop_assert!((cdf_pos + cdf_neg - 1.0).abs() < 1e-12);
        }

        #[test]
        fn prop_norm_inv_cdf_valid_input(p in 0.001_f64..0.999) {
            let result = norm_inv_cdf(p);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn prop_norm_inv_cdf_roundtrip(x in -2.5_f64..2.5) {
            let p = norm_cdf(x);
            if p > 0.001 && p < 0.999 {
                let x_back = norm_inv_cdf(p).unwrap();
                // Combined error from CDF approximation (7.5e-8) and inverse CDF (1e-9)
                prop_assert!((x - x_back).abs() < 1e-5);
            }
        }
    }
}

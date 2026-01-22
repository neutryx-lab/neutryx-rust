//! Special mathematical functions.
//!
//! This module provides implementations of special functions commonly
//! used in statistical and financial calculations.

use num_traits::Float;

/// Computes the natural logarithm of the gamma function.
///
/// Uses the Lanczos approximation with g=7 for high precision.
///
/// ln(Γ(x)) is defined for all x > 0.
///
/// # Arguments
///
/// * `x` - The input value (must be positive)
///
/// # Returns
///
/// The natural logarithm of Γ(x)
///
/// # Precision
///
/// Relative error < 2e-10 for x > 0.5
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::log_gamma;
///
/// // Γ(1) = 1, so ln(Γ(1)) = 0
/// let lg1 = log_gamma(1.0_f64);
/// assert!(lg1.abs() < 1e-10);
///
/// // Γ(n) = (n-1)! for positive integers
/// // Γ(6) = 5! = 120, so ln(Γ(6)) = ln(120)
/// let lg6 = log_gamma(6.0_f64);
/// assert!((lg6 - 120.0_f64.ln()).abs() < 1e-10);
/// ```
#[allow(clippy::excessive_precision)]
pub fn log_gamma<T: Float>(x: T) -> T {
    // Lanczos approximation coefficients for g=7
    // These provide good precision for x > 0.5
    let lanczos_g = T::from(7.0).unwrap();
    let coefficients = [
        T::from(0.999_999_999_999_809_93).unwrap(),
        T::from(676.520_368_121_885_1).unwrap(),
        T::from(-1259.139_216_722_402_8).unwrap(),
        T::from(771.323_428_777_653_08).unwrap(),
        T::from(-176.615_029_162_140_63).unwrap(),
        T::from(12.507_343_278_686_905).unwrap(),
        T::from(-0.138_571_095_265_720_12).unwrap(),
        T::from(9.984_369_578_019_571_6e-6).unwrap(),
        T::from(1.505_632_735_149_311_6e-7).unwrap(),
    ];

    let half = T::from(0.5).unwrap();
    let one = T::one();
    let pi = T::from(core::f64::consts::PI).unwrap();
    let ln_sqrt_2pi = T::from(0.918_938_533_204_672_74).unwrap();

    // Handle x < 0.5 using reflection formula
    // Γ(x) * Γ(1-x) = π / sin(πx)
    if x < half {
        let sin_pix = (pi * x).sin();
        return pi.ln() - sin_pix.abs().ln() - log_gamma(one - x);
    }

    // Lanczos approximation for x >= 0.5
    let z = x - one;

    // Compute Ag(z) = sum of coefficients[i] / (z + i)
    let mut ag = coefficients[0];
    for i in 1..coefficients.len() {
        ag = ag + coefficients[i] / (z + T::from(i).unwrap());
    }

    // ln(Γ(z+1)) = 0.5*ln(2π) + (z+0.5)*ln(z+g+0.5) - (z+g+0.5) + ln(Ag)
    let t = z + lanczos_g + half;

    ln_sqrt_2pi + (z + half) * t.ln() - t + ag.ln()
}

/// Computes the beta function B(a, b).
///
/// B(a, b) = Γ(a) * Γ(b) / Γ(a + b)
///
/// Uses the log-gamma function to avoid overflow:
/// B(a, b) = exp(ln(Γ(a)) + ln(Γ(b)) - ln(Γ(a+b)))
///
/// # Arguments
///
/// * `a` - First parameter (must be positive)
/// * `b` - Second parameter (must be positive)
///
/// # Returns
///
/// The beta function value B(a, b)
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::beta;
///
/// // B(1, 1) = 1
/// let b11 = beta(1.0_f64, 1.0);
/// assert!((b11 - 1.0).abs() < 1e-10);
///
/// // B(a, b) = B(b, a) (symmetry)
/// let b23 = beta(2.0_f64, 3.0);
/// let b32 = beta(3.0_f64, 2.0);
/// assert!((b23 - b32).abs() < 1e-10);
/// ```
#[inline]
pub fn beta<T: Float>(a: T, b: T) -> T {
    // Use log-gamma to compute beta and avoid overflow
    (log_gamma(a) + log_gamma(b) - log_gamma(a + b)).exp()
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // log_gamma tests
    // ==========================================================================

    #[test]
    fn test_log_gamma_at_one() {
        // Γ(1) = 1, ln(Γ(1)) = 0
        let lg = log_gamma(1.0_f64);
        assert_relative_eq!(lg, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_gamma_at_two() {
        // Γ(2) = 1! = 1, ln(Γ(2)) = 0
        let lg = log_gamma(2.0_f64);
        assert_relative_eq!(lg, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_gamma_integers() {
        // Γ(n) = (n-1)! for positive integers
        let factorials = [1.0, 1.0, 2.0, 6.0, 24.0, 120.0, 720.0, 5040.0];

        for (n, &fact) in factorials.iter().enumerate().skip(1) {
            let lg = log_gamma((n + 1) as f64);
            let expected = fact.ln();
            assert_relative_eq!(lg, expected, epsilon = 1e-8);
        }
    }

    #[test]
    fn test_log_gamma_half() {
        // Γ(0.5) = √π, ln(Γ(0.5)) = 0.5 * ln(π)
        let lg = log_gamma(0.5_f64);
        let expected = 0.5 * core::f64::consts::PI.ln();
        assert_relative_eq!(lg, expected, epsilon = 1e-8);
    }

    #[test]
    fn test_log_gamma_recurrence() {
        // Γ(x+1) = x * Γ(x), so ln(Γ(x+1)) = ln(x) + ln(Γ(x))
        for x in [1.5, 2.5, 3.5, 5.0, 7.5, 10.0] {
            let lg_x = log_gamma(x);
            let lg_x_plus_1 = log_gamma(x + 1.0);
            let expected = x.ln() + lg_x;
            assert_relative_eq!(lg_x_plus_1, expected, epsilon = 1e-8);
        }
    }

    #[test]
    fn test_log_gamma_small_positive() {
        // Test for small positive values
        let lg = log_gamma(0.1_f64);
        // Reference: Γ(0.1) ≈ 9.5135, ln(9.5135) ≈ 2.2527
        assert_relative_eq!(lg, 2.252_712_651_734_206, epsilon = 1e-6);
    }

    // ==========================================================================
    // beta tests
    // ==========================================================================

    #[test]
    fn test_beta_one_one() {
        // B(1, 1) = Γ(1)Γ(1)/Γ(2) = 1*1/1 = 1
        let b = beta(1.0_f64, 1.0);
        assert_relative_eq!(b, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_beta_symmetry() {
        // B(a, b) = B(b, a)
        for (a, b) in [(2.0, 3.0), (1.5, 4.5), (0.5, 2.5), (3.0, 3.0)] {
            let b_ab = beta(a, b);
            let b_ba = beta(b, a);
            assert_relative_eq!(b_ab, b_ba, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_beta_integers() {
        // B(m, n) = (m-1)!(n-1)!/(m+n-1)! for positive integers
        let test_cases = [
            (2.0, 3.0, 1.0 / 12.0), // B(2,3) = 1!*2!/4! = 1*2/24 = 1/12
            (3.0, 3.0, 1.0 / 30.0), // B(3,3) = 2!*2!/5! = 4/120 = 1/30
            (2.0, 2.0, 1.0 / 6.0),  // B(2,2) = 1!*1!/3! = 1/6
            (1.0, 5.0, 1.0 / 5.0),  // B(1,5) = 0!*4!/5! = 24/120 = 1/5
        ];

        for (a, b, expected) in test_cases {
            let result = beta(a, b);
            assert_relative_eq!(result, expected, epsilon = 1e-8);
        }
    }

    #[test]
    fn test_beta_half_values() {
        // B(0.5, 0.5) = π
        let b = beta(0.5_f64, 0.5);
        assert_relative_eq!(b, core::f64::consts::PI, epsilon = 1e-8);
    }

    #[test]
    fn test_beta_relation_to_binomial() {
        // B(n-k+1, k+1) = 1 / ((n+1) * C(n, k))
        for n in 2..=10 {
            for k in 0..=n {
                let b = beta((n - k + 1) as f64, (k + 1) as f64);
                let expected = 1.0 / ((n + 1) as f64 * super::super::binomial::<f64>(n, k));
                assert_relative_eq!(b, expected, epsilon = 1e-8);
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
        fn prop_log_gamma_positive(x in 0.5_f64..100.0) {
            // log_gamma is defined for positive x
            let lg = log_gamma(x);
            prop_assert!(lg.is_finite());
        }

        #[test]
        fn prop_log_gamma_recurrence(x in 1.0_f64..50.0) {
            // Γ(x+1) = x * Γ(x)
            let lg_x = log_gamma(x);
            let lg_x_plus_1 = log_gamma(x + 1.0);
            let diff = (lg_x_plus_1 - lg_x - x.ln()).abs();
            prop_assert!(diff < 1e-7);
        }

        #[test]
        fn prop_beta_symmetric(a in 0.5_f64..10.0, b in 0.5_f64..10.0) {
            let b_ab = beta(a, b);
            let b_ba = beta(b, a);
            prop_assert!((b_ab - b_ba).abs() / b_ab.max(b_ba) < 1e-10);
        }

        #[test]
        fn prop_beta_positive(a in 0.5_f64..20.0, b in 0.5_f64..20.0) {
            let b_val = beta(a, b);
            prop_assert!(b_val > 0.0);
            prop_assert!(b_val.is_finite());
        }
    }
}

//! Shared smooth approximation functions for Monte Carlo payoffs.

use num_traits::Float;

/// Soft-plus function: smooth approximation of max(x, 0).
///
/// Uses piecewise implementation for numerical stability:
/// - For large positive x: returns x directly
/// - For large negative x: uses exponential approximation
/// - For near-zero x: uses smooth logarithm
#[inline]
pub fn soft_plus<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();
    if scaled > twenty {
        x
    } else if scaled < -twenty {
        epsilon * scaled.exp()
    } else {
        epsilon * (T::one() + scaled.exp()).ln()
    }
}

/// Smooth indicator function: approximation of Heaviside step.
///
/// Approximates the unit step function H(x) with a smooth sigmoid:
/// - H(x) ~ 0 for large negative x
/// - H(x) ~ 1 for large positive x
/// - H(x) ~ 0.5 at x = 0
#[inline]
pub fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();
    if scaled > twenty {
        T::one()
    } else if scaled < -twenty {
        T::zero()
    } else {
        T::one() / (T::one() + (-scaled).exp())
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_soft_plus_positive() {
        let result = soft_plus(10.0_f64, 0.01);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_soft_plus_negative() {
        let result = soft_plus(-10.0_f64, 0.01);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_soft_plus_at_zero() {
        let epsilon = 1.0_f64;
        let result = soft_plus(0.0, epsilon);
        assert_relative_eq!(result, 2.0_f64.ln(), epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_indicator_positive() {
        let result = smooth_indicator(10.0_f64, 0.01);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_smooth_indicator_negative() {
        let result = smooth_indicator(-10.0_f64, 0.01);
        assert!(result < 1e-6);
    }

    #[test]
    fn test_smooth_indicator_at_zero() {
        let result = smooth_indicator(0.0_f64, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 1e-10);
    }
}

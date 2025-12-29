//! Dual number type integration for automatic differentiation.
//!
//! This module provides a type alias for num-dual's Dual64 type,
//! enabling forward-mode automatic differentiation for verification
//! against Enzyme's LLVM-level AD (implemented in Layer 3).
//!
//! ## Usage
//!
//! ```
//! use pricer_core::types::dual::DualNumber;
//! use pricer_core::math::smoothing::smooth_max;
//!
//! // Create dual numbers
//! let a = DualNumber::from(3.0).derivative();  // da/da = 1.0
//! let b = DualNumber::from(5.0);                // db/da = 0.0
//!
//! // Compute smooth_max with automatic differentiation
//! let result = smooth_max(a, b, 1e-6);
//!
//! // Extract value and gradient
//! let value = result.re();      // ≈ 5.0
//! let gradient = result.eps;    // ∂smooth_max/∂a
//! ```

/// Type alias for num-dual's Dual64 (f64-based dual numbers).
///
/// This type supports first-order automatic differentiation with:
/// - `re()`: Real part (function value)
/// - `eps`: Dual part (derivative/gradient)
///
/// # Integration with Smoothing Functions
///
/// All smoothing functions in `math::smoothing` are generic over `T: num_traits::Float`,
/// which `DualNumber` implements. This allows seamless AD:
///
/// ```
/// use pricer_core::types::dual::DualNumber;
/// use pricer_core::math::smoothing::{smooth_max, smooth_min, smooth_abs, smooth_indicator};
///
/// let x = DualNumber::from(2.0).derivative();
/// let y = DualNumber::from(3.0);
///
/// // All smoothing functions propagate gradients
/// let max_result = smooth_max(x, y, 1e-6);
/// let min_result = smooth_min(x, y, 1e-6);
/// let abs_result = smooth_abs(x, 1e-6);
/// let ind_result = smooth_indicator(x, 1e-6);
/// ```
pub type DualNumber = num_dual::Dual64;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::smoothing::{smooth_abs, smooth_indicator, smooth_max, smooth_min};
    use approx::assert_relative_eq;
    use num_traits::Float;

    // Task 3.2: Dual number integration tests

    #[test]
    fn test_dual_number_basic_arithmetic() {
        // Verify num_traits integration
        let a = DualNumber::from(3.0).derivative();
        let b = DualNumber::from(5.0);

        // Addition
        let sum = a + b;
        assert_eq!(sum.re(), 8.0);
        assert_eq!(sum.eps, 1.0); // d(a+b)/da = 1

        // Subtraction
        let diff = a - b;
        assert_eq!(diff.re(), -2.0);
        assert_eq!(diff.eps, 1.0); // d(a-b)/da = 1

        // Multiplication
        let prod = a * b;
        assert_eq!(prod.re(), 15.0);
        assert_eq!(prod.eps, 5.0); // d(a*b)/da = b

        // Division
        let quot = b / a;
        assert_relative_eq!(quot.re(), 5.0 / 3.0, epsilon = 1e-10);
        assert_relative_eq!(quot.eps, -5.0 / 9.0, epsilon = 1e-10); // d(b/a)/da = -b/a²
    }

    #[test]
    fn test_dual_number_transcendental_functions() {
        let x = DualNumber::from(2.0).derivative();

        // Exponential: d(exp(x))/dx = exp(x)
        let exp_x = x.exp();
        assert_relative_eq!(exp_x.re(), 2.0_f64.exp(), epsilon = 1e-10);
        assert_relative_eq!(exp_x.eps, 2.0_f64.exp(), epsilon = 1e-10);

        // Logarithm: d(ln(x))/dx = 1/x
        let ln_x = x.ln();
        assert_relative_eq!(ln_x.re(), 2.0_f64.ln(), epsilon = 1e-10);
        assert_relative_eq!(ln_x.eps, 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_smooth_max_gradient_propagation() {
        // Test that smooth_max correctly propagates gradients through DualNumber
        let a = DualNumber::from(3.0).derivative();
        let b = DualNumber::from(5.0);
        let epsilon = 1e-6;

        let result = smooth_max(a, b, epsilon);

        // Value should be approximately max(3.0, 5.0) = 5.0
        assert_relative_eq!(result.re(), 5.0, epsilon = 1e-3);

        // Gradient ∂smooth_max/∂a should be small (a is not the max)
        // Analytically: ∂smooth_max/∂a = exp((a-b)/ε) / (exp((a-b)/ε) + exp(0))
        // For a=3, b=5, ε=1e-6: exp(-2/1e-6) ≈ 0
        assert!(result.eps.abs() < 0.01);
    }

    #[test]
    fn test_smooth_min_gradient_propagation() {
        let a = DualNumber::from(3.0).derivative();
        let b = DualNumber::from(5.0);
        let epsilon = 1e-6;

        let result = smooth_min(a, b, epsilon);

        // Value should be approximately min(3.0, 5.0) = 3.0
        assert_relative_eq!(result.re(), 3.0, epsilon = 1e-3);

        // Gradient ∂smooth_min/∂a should be close to 1 (a is the min)
        assert_relative_eq!(result.eps, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_smooth_indicator_gradient_propagation() {
        let x = DualNumber::from(0.0).derivative();
        let epsilon = 1e-3; // Larger epsilon for better gradient at x=0

        let result = smooth_indicator(x, epsilon);

        // Value at x=0 should be 0.5
        assert_relative_eq!(result.re(), 0.5, epsilon = 1e-6);

        // Gradient at x=0: d/dx[1/(1+exp(-x/ε))] = exp(-x/ε) / (ε(1+exp(-x/ε))²)
        // At x=0: 1 / (4ε)
        let expected_gradient = 1.0 / (4.0 * epsilon);
        assert_relative_eq!(result.eps, expected_gradient, epsilon = 0.01);
    }

    #[test]
    fn test_smooth_abs_gradient_propagation() {
        // Test positive x
        let x_pos = DualNumber::from(3.5).derivative();
        let epsilon = 1e-6;

        let result_pos = smooth_abs(x_pos, epsilon);
        assert_relative_eq!(result_pos.re(), 3.5, epsilon = 1e-3);
        // Gradient should be close to 1 for positive x
        assert_relative_eq!(result_pos.eps, 1.0, epsilon = 0.01);

        // Test negative x
        let x_neg = DualNumber::from(-3.5).derivative();
        let result_neg = smooth_abs(x_neg, epsilon);
        assert_relative_eq!(result_neg.re(), 3.5, epsilon = 1e-3);
        // Gradient should be close to -1 for negative x
        assert_relative_eq!(result_neg.eps, -1.0, epsilon = 0.01);
    }

    #[test]
    fn test_all_smoothing_functions_with_dual_numbers() {
        // Verify that all smoothing functions work with DualNumber type
        let a = DualNumber::from(2.0).derivative();
        let b = DualNumber::from(4.0);
        let epsilon = 1e-6;

        // All functions should execute without panic and return finite values
        let max_result = smooth_max(a, b, epsilon);
        assert!(max_result.re().is_finite());
        assert!(max_result.eps.is_finite());

        let min_result = smooth_min(a, b, epsilon);
        assert!(min_result.re().is_finite());
        assert!(min_result.eps.is_finite());

        let ind_result = smooth_indicator(a, epsilon);
        assert!(ind_result.re().is_finite());
        assert!(ind_result.eps.is_finite());

        let abs_result = smooth_abs(a, epsilon);
        assert!(abs_result.re().is_finite());
        assert!(abs_result.eps.is_finite());
    }

    #[test]
    fn test_analytical_vs_dual_gradient_smooth_max() {
        // Compare analytical derivative with dual number gradient
        let a = 3.0;
        let b = 5.0;
        let epsilon = 1e-4; // Larger epsilon for numerical stability

        // Dual number computation
        let a_dual = DualNumber::from(a).derivative();
        let b_dual = DualNumber::from(b);
        let result_dual = smooth_max(a_dual, b_dual, epsilon);

        // Analytical gradient: ∂smooth_max/∂a = exp((a-m)/ε) / (exp((a-m)/ε) + exp((b-m)/ε))
        // where m = max(a, b)
        let m = a.max(b);
        let exp_a = ((a - m) / epsilon).exp();
        let exp_b = ((b - m) / epsilon).exp();
        let analytical_gradient = exp_a / (exp_a + exp_b);

        assert_relative_eq!(result_dual.eps, analytical_gradient, epsilon = 1e-6);
    }
}

//! Enzyme gradient verification tests.
//!
//! This module provides dedicated tests for verifying Enzyme automatic differentiation
//! infrastructure. These tests validate that gradient computation is correct for
//! simple mathematical functions.
//!
//! # Phase 3.0 Status
//!
//! These tests use the placeholder implementation (finite difference approximation).
//! Phase 4 will verify actual Enzyme AD integration.
//!
//! # Test Coverage
//!
//! - `f(x) = x * x` → gradient = `2x` (Requirements 5.1, 5.2)
//! - Tests at x = 1.0, 2.0, 5.0 (Requirement 5.3)
//! - Floating-point comparison with epsilon tolerance (Requirement 5.4)
//! - Edge cases: zero, negative values

#[cfg(test)]
mod tests {
    use crate::enzyme::gradient;
    use crate::verify::{square, square_gradient};
    use approx::{assert_relative_eq, relative_eq};

    // Requirement 5.2: Gradient of f(x) = x * x shall return 2 * x

    #[test]
    fn test_square_gradient_equals_2x_at_1() {
        let x = 1.0;
        let grad = square_gradient(x);
        let expected = 2.0 * x;
        assert_relative_eq!(grad, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_square_gradient_equals_2x_at_2() {
        let x = 2.0;
        let grad = square_gradient(x);
        let expected = 2.0 * x;
        assert_relative_eq!(grad, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_square_gradient_equals_2x_at_5() {
        let x = 5.0;
        let grad = square_gradient(x);
        let expected = 2.0 * x;
        assert_relative_eq!(grad, expected, epsilon = 1e-10);
    }

    // Requirement 5.3: Test at multiple distinct values

    #[test]
    fn test_square_gradient_multiple_values() {
        let test_values = [1.0, 2.0, 5.0, 0.5, 10.0, 100.0];

        for x in test_values {
            let grad = square_gradient(x);
            let expected = 2.0 * x;
            if !relative_eq!(grad, expected, epsilon = 1e-10) {
                panic!("Gradient mismatch at x = {}: left = {}, right = {}", x, grad, expected);
            }
        }
    }

    // Edge case: gradient at zero

    #[test]
    fn test_square_gradient_at_zero() {
        let x = 0.0;
        let grad = square_gradient(x);
        assert_eq!(grad, 0.0, "Gradient at x=0 should be exactly 0");
    }

    // Edge case: negative values

    #[test]
    fn test_square_gradient_negative_values() {
        let test_values = [-1.0, -2.0, -5.0, -0.5];

        for x in test_values {
            let grad = square_gradient(x);
            let expected = 2.0 * x;
            if !relative_eq!(grad, expected, epsilon = 1e-10) {
                panic!("Gradient mismatch at x = {}: left = {}, right = {}", x, grad, expected);
            }
        }
    }

    // Verify square function returns correct values

    #[test]
    fn test_square_function_values() {
        assert_eq!(square(0.0), 0.0);
        assert_eq!(square(1.0), 1.0);
        assert_eq!(square(2.0), 4.0);
        assert_eq!(square(3.0), 9.0);
        assert_eq!(square(-2.0), 4.0);
        assert_eq!(square(0.5), 0.25);
    }

    // Test enzyme::gradient function matches verify::square_gradient

    #[test]
    fn test_enzyme_gradient_matches_analytical() {
        let test_values = [1.0, 2.0, 3.0, 5.0, 10.0];

        for x in test_values {
            let enzyme_grad = gradient(square, x);
            let analytical_grad = square_gradient(x);

            // Allow slightly larger epsilon for finite difference approximation
            if !relative_eq!(enzyme_grad, analytical_grad, epsilon = 1e-6) {
                panic!("Enzyme gradient should match analytical at x = {}: left = {}, right = {}", x, enzyme_grad, analytical_grad);
            }
        }
    }

    // Test finite difference approximation accuracy

    #[test]
    fn test_finite_difference_approximation_accuracy() {
        let x = 3.0;
        let h = 1e-8;

        // Central difference: (f(x+h) - f(x-h)) / (2h)
        let fd_gradient = (square(x + h) - square(x - h)) / (2.0 * h);
        let analytical_gradient = square_gradient(x);

        if !relative_eq!(fd_gradient, analytical_gradient, epsilon = 1e-6) {
            panic!("Finite difference should approximate analytical gradient: left = {}, right = {}", fd_gradient, analytical_gradient);
        }
    }

    // Test gradient of more complex functions using enzyme::gradient

    #[test]
    fn test_gradient_of_cubic() {
        // f(x) = x^3, f'(x) = 3x^2
        let x = 2.0;
        let grad = gradient(|x| x * x * x, x);
        let expected = 3.0 * x * x;
        assert_relative_eq!(grad, expected, epsilon = 1e-5);
    }

    #[test]
    fn test_gradient_of_linear() {
        // f(x) = 5x + 3, f'(x) = 5
        let x = 7.0;
        let grad = gradient(|x| 5.0 * x + 3.0, x);
        assert_relative_eq!(grad, 5.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_of_constant() {
        // f(x) = 42, f'(x) = 0
        let x = 10.0;
        let grad = gradient(|_| 42.0, x);
        assert_relative_eq!(grad, 0.0, epsilon = 1e-10);
    }
}

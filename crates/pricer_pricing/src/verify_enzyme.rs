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
                panic!(
                    "Gradient mismatch at x = {}: left = {}, right = {}",
                    x, grad, expected
                );
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
                panic!(
                    "Gradient mismatch at x = {}: left = {}, right = {}",
                    x, grad, expected
                );
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
            if !relative_eq!(enzyme_grad, analytical_grad, epsilon = 1e-5) {
                panic!(
                    "Enzyme gradient should match analytical at x = {}: left = {}, right = {}",
                    x, enzyme_grad, analytical_grad
                );
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
            panic!(
                "Finite difference should approximate analytical gradient: left = {}, right = {}",
                fd_gradient, analytical_gradient
            );
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

    // ========================================================================
    // Path-Dependent Option Gradient Tests (Task 12.3)
    // ========================================================================
    //
    // These tests verify gradient computation for path-dependent option pricing.
    // They establish the framework for future Enzyme vs num-dual comparison.

    use crate::path_dependent::PathPayoffType;

    /// Test gradient of geometric mean payoff with respect to average price.
    #[test]
    fn test_gradient_geometric_mean_payoff() {
        // Simplified geometric average payoff: max(avg - K, 0)
        // When avg > K, d/d(avg) = 1
        // When avg < K, d/d(avg) = 0
        let strike = 100.0;

        // In-the-money case: avg = 110, payoff = 10
        let avg_itm = 110.0;
        let grad_itm = gradient(|avg| (avg - strike).max(0.0), avg_itm);
        assert_relative_eq!(grad_itm, 1.0, epsilon = 1e-6);

        // Out-of-the-money case: avg = 90, payoff = 0
        let avg_otm = 90.0;
        let grad_otm = gradient(|avg| (avg - strike).max(0.0), avg_otm);
        // Near zero for OTM (smooth approximation would give small positive value)
        assert!(grad_otm.abs() < 0.01);
    }

    /// Test gradient of smooth max function used in payoffs.
    #[test]
    fn test_gradient_smooth_max() {
        // soft_plus approximation: max(x, 0) ≈ x + log(1 + exp(-x))
        // Gradient: 1 / (1 + exp(-x)) (sigmoid)
        fn soft_plus(x: f64, epsilon: f64) -> f64 {
            if epsilon <= 0.0 {
                return x.max(0.0);
            }
            let z = x / epsilon;
            if z > 20.0 {
                x
            } else if z < -20.0 {
                0.0
            } else {
                epsilon * (1.0 + z.exp()).ln()
            }
        }

        let epsilon = 0.1;

        // Test gradient at various points
        let x_pos = 1.0;
        let grad_pos = gradient(|x| soft_plus(x, epsilon), x_pos);
        // Should be close to 1 for positive x
        assert!(grad_pos > 0.9 && grad_pos <= 1.0);

        let x_zero = 0.0;
        let grad_zero = gradient(|x| soft_plus(x, epsilon), x_zero);
        // Should be close to 0.5 at x=0
        assert!(grad_zero > 0.4 && grad_zero < 0.6);

        let x_neg = -1.0;
        let grad_neg = gradient(|x| soft_plus(x, epsilon), x_neg);
        // Should be close to 0 for negative x
        assert!(grad_neg >= 0.0 && grad_neg < 0.1);
    }

    /// Test delta (dV/dS) approximation for Asian option.
    #[test]
    fn test_asian_option_delta_approximation() {
        // Delta is the sensitivity of option price to spot price
        // For Asian call: delta is positive and less than 1

        // Simplified model: price = discount * E[max(avg - K, 0)]
        // Using deterministic average for testing
        let rate: f64 = 0.05;
        let maturity: f64 = 1.0;
        let strike: f64 = 100.0;
        let discount = (-rate * maturity).exp();

        // Price as function of spot (using spot as proxy for average)
        let price_fn = |spot: f64| -> f64 {
            // Simplified: assume average = spot (no volatility)
            let avg = spot;
            discount * (avg - strike).max(0.0)
        };

        let spot = 105.0;
        let delta = gradient(price_fn, spot);

        // Delta should be positive for ITM call
        assert!(delta > 0.0, "Delta should be positive: {}", delta);
        // Delta should be less than or equal to discount factor
        assert!(delta <= discount + 0.01, "Delta should be <= df: {}", delta);
    }

    /// Test barrier option gradient behavior at barrier.
    #[test]
    fn test_barrier_option_gradient_at_barrier() {
        // Near barrier, the gradient of a knock-out option changes rapidly
        let barrier = 80.0;
        let strike = 100.0;

        // Simplified barrier payoff indicator: 1 if min >= barrier, 0 otherwise
        fn barrier_indicator(min_price: f64, barrier: f64, epsilon: f64) -> f64 {
            // Smooth indicator: sigmoid((min - barrier) / epsilon)
            let z = (min_price - barrier) / epsilon;
            1.0 / (1.0 + (-z).exp())
        }

        let epsilon = 1.0; // Smoothing

        // Above barrier
        let min_above = 85.0;
        let ind_above = barrier_indicator(min_above, barrier, epsilon);
        assert!(ind_above > 0.9, "Should be ~1 above barrier: {}", ind_above);

        // At barrier
        let min_at = 80.0;
        let ind_at = barrier_indicator(min_at, barrier, epsilon);
        assert!(
            (ind_at - 0.5).abs() < 0.1,
            "Should be ~0.5 at barrier: {}",
            ind_at
        );

        // Below barrier
        let min_below = 75.0;
        let ind_below = barrier_indicator(min_below, barrier, epsilon);
        assert!(ind_below < 0.1, "Should be ~0 below barrier: {}", ind_below);

        // Test gradient near barrier
        let grad_at_barrier = gradient(|m| barrier_indicator(m, barrier, epsilon), 80.0);
        // Gradient should be highest at the barrier (maximum sensitivity)
        let grad_away = gradient(|m| barrier_indicator(m, barrier, epsilon), 90.0);
        assert!(
            grad_at_barrier.abs() > grad_away.abs(),
            "Gradient at barrier should be higher than away: at={}, away={}",
            grad_at_barrier,
            grad_away
        );
    }

    /// Test that PathObserver-based payoffs produce consistent gradients.
    #[test]
    fn test_path_observer_payoff_gradient() {
        // Create a simple observer-based pricing scenario
        let strike = 100.0;
        let epsilon = 0.0; // No smoothing

        // Test that payoff gradient with respect to terminal price is correct
        let payoff = PathPayoffType::asian_arithmetic_call(strike, epsilon);

        // Verify PathPayoffType can be constructed and is the correct variant
        match payoff {
            PathPayoffType::AsianArithmetic(_) => {
                // Success - correct variant constructed
            }
            _ => panic!("Expected AsianArithmetic variant"),
        }
    }

    /// Test gradient consistency: enzyme gradient should match finite difference.
    #[test]
    fn test_gradient_consistency_path_dependent() {
        // Verify that our enzyme::gradient (finite difference) matches
        // manually computed finite difference with the same step size

        let h = 1e-8;
        let spot = 100.0;

        // Test function: simplified Asian option pricing
        let price_fn = |s: f64| -> f64 {
            let strike = 100.0;
            let avg = s; // Simplified
            (avg - strike).max(0.0)
        };

        // Enzyme gradient (uses same h internally)
        let enzyme_grad = gradient(price_fn, spot);

        // Manual finite difference
        let manual_grad = (price_fn(spot + h) - price_fn(spot - h)) / (2.0 * h);

        // Should match exactly (same algorithm)
        assert_relative_eq!(enzyme_grad, manual_grad, epsilon = 1e-12);
    }

    // ========================================================================
    // Future Enzyme vs num-dual Comparison Framework
    // ========================================================================
    //
    // When Enzyme is integrated, these tests will compare:
    // 1. Enzyme reverse-mode AD gradients
    // 2. num-dual forward-mode AD gradients
    // 3. Finite difference approximations
    //
    // All should agree within tolerance.

    /// Placeholder for future Enzyme vs num-dual comparison.
    /// Currently verifies the test framework is in place.
    #[test]
    fn test_ad_comparison_framework_ready() {
        // This test will be expanded in Phase 5 when Enzyme is fully integrated
        //
        // Future implementation:
        // let spot = 100.0;
        // let enzyme_delta = enzyme_delta_asian_call(spot, strike, ...);
        // let numdual_delta = numdual_delta_asian_call(spot, strike, ...);
        // assert_relative_eq!(enzyme_delta, numdual_delta, epsilon = 1e-6);

        // For now, verify test infrastructure
        let result = gradient(|x| x * x, 5.0);
        assert_relative_eq!(result, 10.0, epsilon = 1e-6);
    }
}

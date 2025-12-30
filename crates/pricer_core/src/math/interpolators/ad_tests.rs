//! Automatic differentiation compatibility tests for interpolators.
//!
//! These tests verify that all interpolators correctly propagate gradients
//! when used with `Dual64` inputs.

#![cfg(all(test, feature = "num-dual-mode"))]

use super::*;
use num_dual::Dual64;

// ========================================
// Task 11.1: Dual Number Tests
// ========================================

mod linear_ad_tests {
    use super::*;

    #[test]
    fn test_linear_with_dual64_returns_finite() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(2.0),
            Dual64::from(4.0),
            Dual64::from(6.0),
        ];

        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // Query with derivative seed
        let x = Dual64::new(1.5, 1.0);
        let y = interp.interpolate(x).unwrap();

        assert!(y.re.is_finite(), "Value should be finite");
        assert!(y.eps.is_finite(), "Gradient should be finite");
    }

    #[test]
    fn test_linear_gradient_correct() {
        // For y = 2x, gradient should be 2
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0), Dual64::from(2.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(2.0), Dual64::from(4.0)];

        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        let x = Dual64::new(0.5, 1.0);
        let y = interp.interpolate(x).unwrap();

        assert!((y.re - 1.0).abs() < 1e-10, "Value should be 1.0");
        assert!((y.eps - 2.0).abs() < 1e-10, "Gradient should be 2.0");
    }
}

mod cubic_spline_ad_tests {
    use super::*;

    #[test]
    fn test_cubic_spline_with_dual64_returns_finite() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(4.0),
            Dual64::from(9.0),
        ];

        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        let x = Dual64::new(1.5, 1.0);
        let y = interp.interpolate(x).unwrap();

        assert!(y.re.is_finite(), "Value should be finite");
        assert!(y.eps.is_finite(), "Gradient should be finite");
    }

    #[test]
    fn test_cubic_spline_gradient_reasonable() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(4.0),
            Dual64::from(9.0),
        ];

        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        let x = Dual64::new(1.5, 1.0);
        let y = interp.interpolate(x).unwrap();

        // Gradient should be positive and reasonable for increasing data
        assert!(
            y.eps > 0.0,
            "Gradient should be positive for increasing data"
        );
        assert!(y.eps < 10.0, "Gradient should be reasonable magnitude");
    }
}

mod monotonic_ad_tests {
    use super::*;

    #[test]
    fn test_monotonic_with_dual64_returns_finite() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(3.0),
            Dual64::from(6.0),
        ];

        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        let x = Dual64::new(1.5, 1.0);
        let y = interp.interpolate(x).unwrap();

        assert!(y.re.is_finite(), "Value should be finite");
        assert!(y.eps.is_finite(), "Gradient should be finite");
    }

    #[test]
    fn test_monotonic_gradient_positive_for_increasing() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(3.0),
            Dual64::from(6.0),
        ];

        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        // Test at multiple points
        for x_val in [0.5, 1.0, 1.5, 2.0, 2.5] {
            let x = Dual64::new(x_val, 1.0);
            let y = interp.interpolate(x).unwrap();

            assert!(
                y.eps >= 0.0,
                "Gradient should be non-negative at x={}, got {}",
                x_val,
                y.eps
            );
        }
    }
}

mod bilinear_ad_tests {
    use super::*;

    #[test]
    fn test_bilinear_with_dual64_returns_finite() {
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let row0: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let row1: Vec<Dual64> = vec![Dual64::from(2.0), Dual64::from(3.0)];
        let zs: [&[Dual64]; 2] = [&row0, &row1];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        let x = Dual64::new(0.5, 1.0);
        let y = Dual64::new(0.5, 0.0); // Only seed x derivative
        let z = interp.interpolate(x, y).unwrap();

        assert!(z.re.is_finite(), "Value should be finite");
        assert!(z.eps.is_finite(), "Gradient should be finite");
    }

    #[test]
    fn test_bilinear_partial_derivatives() {
        // z = x + y surface
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let row0: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0)];
        let row1: Vec<Dual64> = vec![Dual64::from(1.0), Dual64::from(2.0)];
        let zs: [&[Dual64]; 2] = [&row0, &row1];

        let interp = BilinearInterpolator::new(&xs, &ys, &zs).unwrap();

        // Test ∂z/∂x with y fixed
        let x = Dual64::new(0.5, 1.0);
        let y = Dual64::new(0.5, 0.0);
        let z = interp.interpolate(x, y).unwrap();

        // For z = x + y, ∂z/∂x = 1
        assert!(
            (z.eps - 1.0).abs() < 1e-10,
            "∂z/∂x should be 1.0, got {}",
            z.eps
        );
    }
}

mod smooth_interp_ad_tests {
    use super::*;

    #[test]
    fn test_smooth_interp_with_dual64_returns_finite() {
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0), Dual64::from(2.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(2.0), Dual64::from(4.0)];

        let x = Dual64::new(0.5, 1.0);
        let epsilon = Dual64::from(1e-4);

        let y = smooth_interp(&xs, &ys, x, epsilon).unwrap();

        assert!(y.re.is_finite(), "Value should be finite");
        assert!(y.eps.is_finite(), "Gradient should be finite");
    }

    #[test]
    fn test_smooth_interp_gradient_propagates() {
        // For y = 2x, gradient should be approximately 2
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0), Dual64::from(2.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(2.0), Dual64::from(4.0)];

        let x = Dual64::new(0.5, 1.0);
        let epsilon = Dual64::from(1e-6);

        let y = smooth_interp(&xs, &ys, x, epsilon).unwrap();

        // Gradient should be close to 2 for linear data
        assert!(
            (y.eps - 2.0).abs() < 0.5,
            "Gradient should be approximately 2.0, got {}",
            y.eps
        );
    }
}

// ========================================
// Task 11.2: Gradient Verification Tests
// ========================================

mod gradient_verification {
    use super::*;

    /// Compare AD gradient to finite difference approximation.
    fn verify_gradient_with_finite_diff<F>(f: F, x: f64, h: f64, tolerance: f64)
    where
        F: Fn(Dual64) -> Dual64,
    {
        // AD gradient
        let x_dual = Dual64::new(x, 1.0);
        let y_dual = f(x_dual);
        let ad_gradient = y_dual.eps;

        // Finite difference gradient
        let y_plus = f(Dual64::from(x + h)).re;
        let y_minus = f(Dual64::from(x - h)).re;
        let fd_gradient = (y_plus - y_minus) / (2.0 * h);

        let relative_error = if fd_gradient.abs() > 1e-10 {
            ((ad_gradient - fd_gradient) / fd_gradient).abs()
        } else {
            (ad_gradient - fd_gradient).abs()
        };

        assert!(
            relative_error < tolerance,
            "AD gradient {} differs from FD gradient {} by {}",
            ad_gradient,
            fd_gradient,
            relative_error
        );
    }

    #[test]
    fn test_linear_gradient_vs_finite_diff() {
        let xs: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0), Dual64::from(2.0)];
        let ys: Vec<Dual64> = vec![Dual64::from(0.0), Dual64::from(1.0), Dual64::from(4.0)];

        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        let f = |x: Dual64| interp.interpolate(x).unwrap();

        // Test at interior points
        for x in [0.25, 0.5, 0.75, 1.25, 1.5, 1.75] {
            verify_gradient_with_finite_diff(f, x, 1e-6, 1e-4);
        }
    }

    #[test]
    fn test_cubic_spline_gradient_vs_finite_diff() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(4.0),
            Dual64::from(9.0),
        ];

        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        let f = |x: Dual64| interp.interpolate(x).unwrap();

        // Test at interior points
        for x in [0.5, 1.0, 1.5, 2.0, 2.5] {
            verify_gradient_with_finite_diff(f, x, 1e-6, 1e-4);
        }
    }

    #[test]
    fn test_monotonic_gradient_vs_finite_diff() {
        let xs: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(2.0),
            Dual64::from(3.0),
        ];
        let ys: Vec<Dual64> = vec![
            Dual64::from(0.0),
            Dual64::from(1.0),
            Dual64::from(3.0),
            Dual64::from(6.0),
        ];

        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        let f = |x: Dual64| interp.interpolate(x).unwrap();

        // Test at interior points
        for x in [0.5, 1.0, 1.5, 2.0, 2.5] {
            verify_gradient_with_finite_diff(f, x, 1e-6, 1e-4);
        }
    }
}

// ========================================
// Task 11.3: Property-Based Tests
// ========================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_linear_gradient_is_finite(
            x in 0.1f64..1.9f64
        ) {
            let xs: Vec<Dual64> = vec![
                Dual64::from(0.0),
                Dual64::from(1.0),
                Dual64::from(2.0),
            ];
            let ys: Vec<Dual64> = vec![
                Dual64::from(0.0),
                Dual64::from(1.0),
                Dual64::from(4.0),
            ];

            let interp = LinearInterpolator::new(&xs, &ys).unwrap();

            let x_dual = Dual64::new(x, 1.0);
            let y = interp.interpolate(x_dual).unwrap();

            prop_assert!(y.re.is_finite());
            prop_assert!(y.eps.is_finite());
            prop_assert!(!y.eps.is_nan());
        }

        #[test]
        fn test_monotonic_gradient_sign_matches_data(
            x in 0.1f64..2.9f64
        ) {
            // Increasing data
            let xs: Vec<Dual64> = vec![
                Dual64::from(0.0),
                Dual64::from(1.0),
                Dual64::from(2.0),
                Dual64::from(3.0),
            ];
            let ys: Vec<Dual64> = vec![
                Dual64::from(0.0),
                Dual64::from(1.0),
                Dual64::from(3.0),
                Dual64::from(6.0),
            ];

            let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

            let x_dual = Dual64::new(x, 1.0);
            let y = interp.interpolate(x_dual).unwrap();

            // For increasing data, gradient should be non-negative
            prop_assert!(y.eps >= -1e-10, "Gradient should be non-negative");
        }
    }
}

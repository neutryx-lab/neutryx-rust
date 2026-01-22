//! Integration tests for the math module.
//!
//! These tests verify that different math submodules work correctly together
//! and are properly integrated.

// =============================================================================
// Distribution Integration Tests
// =============================================================================

mod distributions {
    use pricer_core::math::distributions::{bivariate_norm_cdf, norm_cdf, norm_inv_cdf, norm_pdf};

    /// Test CDF-PDF relationship: d/dx CDF(x) = PDF(x)
    #[test]
    fn test_cdf_pdf_derivative_relationship() {
        let h = 1e-5; // Slightly larger h for better numerical stability
        let test_points = [-2.0_f64, -1.0, 0.0, 1.0, 2.0];

        for x in test_points {
            let numerical_derivative = (norm_cdf(x + h) - norm_cdf(x - h)) / (2.0 * h);
            let pdf = norm_pdf(x);
            // Tolerance accounts for CDF approximation error + numerical derivative error
            assert!(
                (numerical_derivative - pdf).abs() < 1e-3,
                "CDF derivative should equal PDF at x={}: got {}, expected {}",
                x,
                numerical_derivative,
                pdf
            );
        }
    }

    /// Test CDF-inverse CDF roundtrip
    #[test]
    fn test_cdf_inverse_roundtrip() {
        let probabilities = [0.1_f64, 0.25, 0.5, 0.75, 0.9];

        for p in probabilities {
            let x = norm_inv_cdf(p).unwrap();
            let p_back = norm_cdf(x);
            assert!(
                (p - p_back).abs() < 1e-6,
                "CDF(inv_cdf(p)) should equal p: p={}, got {}",
                p,
                p_back
            );
        }
    }

    /// Test bivariate normal CDF with zero correlation equals product of marginals
    #[test]
    fn test_bivariate_zero_correlation() {
        let test_points = [(0.0_f64, 0.0_f64), (1.0, 1.0), (-1.0, 1.0), (0.5, -0.5)];

        for (x, y) in test_points {
            let bivariate = bivariate_norm_cdf(x, y, 0.0).unwrap();
            let product = norm_cdf(x) * norm_cdf(y);
            assert!(
                (bivariate - product).abs() < 1e-6,
                "Bivariate CDF with rho=0 should equal product of marginals"
            );
        }
    }

    /// Test bivariate normal CDF bounds
    #[test]
    fn test_bivariate_bounds() {
        let correlations = [-0.9_f64, -0.5, 0.0, 0.5, 0.9];
        let points = [(0.0_f64, 0.0_f64), (1.0, 1.0), (-1.0, -1.0)];

        for rho in correlations {
            for (x, y) in points {
                let result = bivariate_norm_cdf(x, y, rho).unwrap();
                assert!(
                    result >= 0.0 && result <= 1.0,
                    "Bivariate CDF should be in [0,1]: got {} for x={}, y={}, rho={}",
                    result,
                    x,
                    y,
                    rho
                );
            }
        }
    }
}

// =============================================================================
// Interpolation Integration Tests
// =============================================================================

mod interpolation {
    use pricer_core::math::interpolators::{
        CubicSplineInterpolator, Interpolator, LinearInterpolator, LogLinearInterpolator,
    };

    /// Test that all interpolators pass through data points exactly
    #[test]
    fn test_interpolators_pass_through_points() {
        let xs = vec![0.0_f64, 1.0, 2.0, 3.0, 4.0];
        let ys = vec![1.0_f64, 2.0, 1.5, 3.0, 2.5];

        // Linear
        let linear = LinearInterpolator::new(&xs, &ys).unwrap();
        for (i, &x) in xs.iter().enumerate() {
            let y = linear.interpolate(x).unwrap();
            assert!(
                (y - ys[i]).abs() < 1e-10,
                "Linear interpolator should pass through point ({}, {})",
                x,
                ys[i]
            );
        }

        // Cubic spline
        let cubic = CubicSplineInterpolator::new(&xs, &ys).unwrap();
        for (i, &x) in xs.iter().enumerate() {
            let y = cubic.interpolate(x).unwrap();
            assert!(
                (y - ys[i]).abs() < 1e-10,
                "Cubic interpolator should pass through point ({}, {})",
                x,
                ys[i]
            );
        }
    }

    /// Test log-linear interpolation for exponential data
    #[test]
    fn test_log_linear_exponential_data() {
        let xs = vec![0.0_f64, 1.0, 2.0, 3.0];
        // y = exp(x) at these points
        let ys: Vec<f64> = xs.iter().map(|&x| x.exp()).collect();

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        // Log-linear should be exact for exponential data
        let test_x = 1.5_f64;
        let expected = test_x.exp();
        let result = interp.interpolate(test_x).unwrap();
        assert!(
            (result - expected).abs() < 0.01,
            "Log-linear should be near-exact for exp data: got {}, expected {}",
            result,
            expected
        );
    }
}

// =============================================================================
// Solver Integration Tests
// =============================================================================

mod solvers {
    use pricer_core::math::solvers::{
        BacktrackingNewtonSolver, BisectionSolver, BrentSolver, NewtonRaphsonSolver, SolverConfig,
    };

    /// Compare different solvers on the same problem
    #[test]
    fn test_solver_consistency() {
        // f(x) = x^3 - 2x - 5, root near 2.0945514815
        let f = |x: f64| x * x * x - 2.0 * x - 5.0;
        let f_prime = |x: f64| 3.0 * x * x - 2.0;

        let config = SolverConfig::default();

        // Bisection
        let bisection = BisectionSolver::new(config);
        let root_bisection = bisection.find_root(f, 2.0, 3.0).unwrap();

        // Brent
        let brent = BrentSolver::new(config);
        let root_brent = brent.find_root(f, 2.0, 3.0).unwrap();

        // Newton-Raphson
        let newton = NewtonRaphsonSolver::new(config);
        let root_newton = newton.find_root(f, f_prime, 2.5).unwrap();

        // Backtracking Newton
        let bt_newton = BacktrackingNewtonSolver::new(config);
        let root_bt = bt_newton.find_root(f, f_prime, 2.5).unwrap();

        // All should find the same root (approximately)
        let expected = 2.094551481542327;
        assert!(
            (root_bisection - expected).abs() < 1e-6,
            "Bisection: {}",
            root_bisection
        );
        assert!((root_brent - expected).abs() < 1e-6, "Brent: {}", root_brent);
        assert!(
            (root_newton - expected).abs() < 1e-6,
            "Newton: {}",
            root_newton
        );
        assert!((root_bt - expected).abs() < 1e-6, "BT Newton: {}", root_bt);
    }

    /// Test solvers with trigonometric function
    #[test]
    fn test_solver_trig_function() {
        // f(x) = cos(x) - x, root near 0.739085
        let f = |x: f64| x.cos() - x;
        let f_prime = |x: f64| -x.sin() - 1.0;

        let config = SolverConfig::default();
        let newton = NewtonRaphsonSolver::new(config);
        let root = newton.find_root(f, f_prime, 0.5).unwrap();

        let expected = 0.7390851332151607;
        assert!(
            (root - expected).abs() < 1e-8,
            "cos(x) = x fixed point: got {}, expected {}",
            root,
            expected
        );
    }
}

// =============================================================================
// Integration (Quadrature) Integration Tests
// =============================================================================

mod integrators {
    use pricer_core::math::integrators::{
        integrate_gauss_kronrod, integrate_gauss_legendre, GaussKronrodRule, GaussLegendreOrder,
    };

    /// Test different integrators on polynomial (exact for Gauss)
    #[test]
    fn test_integrator_polynomial() {
        // Integrate x^2 from 0 to 1, exact answer = 1/3
        let f = |x: f64| x * x;

        let expected = 1.0 / 3.0;

        let result_gl = integrate_gauss_legendre(f, 0.0, 1.0, GaussLegendreOrder::N7);
        let result_gk = integrate_gauss_kronrod(f, 0.0, 1.0, GaussKronrodRule::G7K15);

        assert!(
            (result_gl.value - expected).abs() < 1e-10,
            "Gauss-Legendre: {}",
            result_gl.value
        );
        assert!(
            (result_gk.value - expected).abs() < 1e-10,
            "Gauss-Kronrod: {}",
            result_gk.value
        );
    }

    /// Test integration of exp function
    #[test]
    fn test_integrator_exponential() {
        // Integrate e^x from 0 to 1, exact answer = e - 1
        let f = |x: f64| x.exp();

        let expected = std::f64::consts::E - 1.0;
        let result = integrate_gauss_kronrod(f, 0.0, 1.0, GaussKronrodRule::G7K15);

        assert!(
            (result.value - expected).abs() < 1e-10,
            "Integral of e^x: got {}, expected {}",
            result.value,
            expected
        );
    }

    /// Test integration of normal PDF (should equal CDF difference)
    #[test]
    fn test_integrator_normal_pdf() {
        use pricer_core::math::distributions::{norm_cdf, norm_pdf};

        // Integrate norm_pdf from -1 to 1
        let result = integrate_gauss_kronrod(norm_pdf, -1.0, 1.0, GaussKronrodRule::G10K21);
        let expected: f64 = norm_cdf(1.0) - norm_cdf(-1.0);

        // Tolerance accounts for CDF approximation error + integration error
        assert!(
            (result.value - expected).abs() < 1e-6,
            "Integral of norm_pdf should equal CDF difference: got {}, expected {}",
            result.value,
            expected
        );
    }
}

// =============================================================================
// Optimisation Integration Tests
// =============================================================================

mod optimisers {
    use pricer_core::math::optimisers::{minimize_nelder_mead, NelderMeadConfig};

    /// Test Nelder-Mead on 1D quadratic
    #[test]
    fn test_nelder_mead_1d_quadratic() {
        // Minimise (x - 3)^2, minimum at x = 3
        let f = |x: &[f64]| (x[0] - 3.0).powi(2);

        let config = NelderMeadConfig::default();
        let x0 = [0.0];
        let result = minimize_nelder_mead(f, &x0, config).unwrap();

        assert!(
            (result.params[0] - 3.0).abs() < 1e-4,
            "Minimum should be at x=3: got {}",
            result.params[0]
        );
        assert!(
            result.value < 1e-8,
            "Function value at minimum should be near 0: got {}",
            result.value
        );
    }
}

// =============================================================================
// Cross-Module Integration Tests
// =============================================================================

mod cross_module {
    use pricer_core::math::{
        distributions::norm_cdf,
        integrators::{integrate_gauss_kronrod, GaussKronrodRule},
        interpolators::{Interpolator, LinearInterpolator},
        solvers::{NewtonRaphsonSolver, SolverConfig},
    };

    /// Test: Find x such that CDF(x) = 0.95 using Newton's method
    /// (alternative to norm_inv_cdf using solver)
    #[test]
    fn test_solver_with_distribution() {
        use pricer_core::math::distributions::{norm_inv_cdf, norm_pdf};

        // f(x) = CDF(x) - 0.95
        let target = 0.95;
        let f = |x: f64| norm_cdf(x) - target;
        let f_prime = |x: f64| norm_pdf(x);

        let config = SolverConfig::default();
        let newton = NewtonRaphsonSolver::new(config);

        let root = newton.find_root(f, f_prime, 1.5).unwrap();
        let expected = norm_inv_cdf(target).unwrap();

        assert!(
            (root - expected).abs() < 1e-6,
            "Newton solve for CDF^-1(0.95): got {}, expected {}",
            root,
            expected
        );
    }

    /// Test: Interpolate a function and then integrate the interpolation
    #[test]
    fn test_interpolator_integrator_pipeline() {
        // Create sampled data from sin(x) on [0, pi]
        let n = 10;
        let xs: Vec<f64> = (0..=n)
            .map(|i| i as f64 * std::f64::consts::PI / n as f64)
            .collect();
        let ys: Vec<f64> = xs.iter().map(|&x| x.sin()).collect();

        // Create interpolator
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // Integrate the interpolated function
        let interp_fn = |x: f64| interp.interpolate(x).unwrap();
        let result =
            integrate_gauss_kronrod(interp_fn, 0.0, std::f64::consts::PI, GaussKronrodRule::G10K21);

        // Exact integral of sin(x) from 0 to pi is 2.0
        // Linear interpolation of sin will underestimate
        assert!(
            (result.value - 2.0).abs() < 0.1,
            "Interpolated integral should be close to 2: got {}",
            result.value
        );
    }

    /// Test: Build a CDF from PDF using integration
    #[test]
    fn test_build_cdf_from_pdf() {
        use pricer_core::math::distributions::norm_pdf;

        // Build CDF at several points by integrating PDF
        let test_points = [-2.0_f64, -1.0, 0.0, 1.0, 2.0];

        for x in test_points {
            // Integrate from -infinity (approx -10) to x
            let computed_cdf =
                integrate_gauss_kronrod(norm_pdf, -10.0, x, GaussKronrodRule::G10K21);
            let actual_cdf = norm_cdf(x);

            assert!(
                (computed_cdf.value - actual_cdf).abs() < 1e-6,
                "Integrated CDF at x={}: got {}, expected {}",
                x,
                computed_cdf.value,
                actual_cdf
            );
        }
    }
}

// =============================================================================
// Utilities Integration Tests
// =============================================================================

mod utilities {
    use pricer_core::math::utilities::{binomial, clamp, factorial, lerp, sign};

    #[test]
    fn test_factorial_values() {
        assert!((factorial::<f64>(0) - 1.0).abs() < 1e-10);
        assert!((factorial::<f64>(1) - 1.0).abs() < 1e-10);
        assert!((factorial::<f64>(5) - 120.0).abs() < 1e-10);
        assert!((factorial::<f64>(10) - 3628800.0).abs() < 1e-6);
    }

    #[test]
    fn test_binomial_coefficients() {
        // C(5,2) = 10
        assert!((binomial::<f64>(5, 2) - 10.0).abs() < 1e-10);
        // C(n,0) = 1
        assert!((binomial::<f64>(10, 0) - 1.0).abs() < 1e-10);
        // C(n,n) = 1
        assert!((binomial::<f64>(10, 10) - 1.0).abs() < 1e-10);
        // Pascal's triangle: C(n,k) = C(n-1,k-1) + C(n-1,k)
        assert!(
            (binomial::<f64>(6, 3) - (binomial::<f64>(5, 2) + binomial::<f64>(5, 3))).abs()
                < 1e-10
        );
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0_f64, 10.0_f64, 0.0_f64) - 0.0).abs() < 1e-10);
        assert!((lerp(0.0_f64, 10.0_f64, 1.0_f64) - 10.0).abs() < 1e-10);
        assert!((lerp(0.0_f64, 10.0_f64, 0.5_f64) - 5.0).abs() < 1e-10);
        assert!((lerp(-5.0_f64, 5.0_f64, 0.5_f64) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sign() {
        assert_eq!(sign(5.0_f64), 1.0);
        assert_eq!(sign(-5.0_f64), -1.0);
        assert_eq!(sign(0.0_f64), 0.0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0_f64, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-5.0_f64, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0_f64, 0.0, 10.0), 10.0);
    }
}

// =============================================================================
// Calculus Integration Tests
// =============================================================================

mod calculus {
    use pricer_core::math::calculus::{
        finite_diff, finite_diff_second, partial_diff, partial_diff_second, DifferenceType,
    };

    /// Test numerical derivatives against analytical values
    #[test]
    fn test_derivative_quadratic() {
        // f(x) = x^2, f'(x) = 2x
        let f = |x: f64| x * x;
        let h = 1e-5;

        for x in [-2.0_f64, -1.0, 0.0, 1.0, 2.0] {
            let numerical = finite_diff(&f, x, h, DifferenceType::Central);
            let analytical = 2.0 * x;
            assert!(
                (numerical - analytical).abs() < 1e-6,
                "Derivative at x={}: got {}, expected {}",
                x,
                numerical,
                analytical
            );
        }
    }

    /// Test second derivative
    #[test]
    fn test_second_derivative() {
        // f(x) = x^3, f''(x) = 6x
        let f = |x: f64| x * x * x;
        let h = 1e-4;

        for x in [-2.0_f64, -1.0, 1.0, 2.0] {
            let numerical = finite_diff_second(&f, x, h);
            let analytical = 6.0 * x;
            assert!(
                (numerical - analytical).abs() < 1e-3,
                "Second derivative at x={}: got {}, expected {}",
                x,
                numerical,
                analytical
            );
        }
    }

    /// Test partial derivative of multivariate function
    #[test]
    fn test_partial_derivative() {
        // f(x,y) = x^2 + y^2, df/dx = 2x, df/dy = 2y
        let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];
        let h = 1e-5;

        let point = [3.0, 4.0];

        let df_dx = partial_diff(&f, &point, 0, h, DifferenceType::Central);
        let df_dy = partial_diff(&f, &point, 1, h, DifferenceType::Central);

        assert!((df_dx - 6.0).abs() < 1e-5, "df/dx: {}", df_dx);
        assert!((df_dy - 8.0).abs() < 1e-5, "df/dy: {}", df_dy);
    }

    /// Test partial second derivative
    #[test]
    fn test_partial_second_derivative() {
        // f(x,y) = x^2 + 2*y^2, d²f/dx² = 2, d²f/dy² = 4
        let f = |x: &[f64]| x[0] * x[0] + 2.0 * x[1] * x[1];
        let h = 1e-4;

        let point = [1.0, 1.0];

        let d2f_dx2 = partial_diff_second(&f, &point, 0, h);
        let d2f_dy2 = partial_diff_second(&f, &point, 1, h);

        assert!((d2f_dx2 - 2.0).abs() < 1e-3, "d²f/dx²: {}", d2f_dx2);
        assert!((d2f_dy2 - 4.0).abs() < 1e-3, "d²f/dy²: {}", d2f_dy2);
    }
}

// =============================================================================
// Smoothing Integration Tests
// =============================================================================

mod smoothing {
    use pricer_core::math::smoothing::{smooth_abs, smooth_indicator, smooth_max, smooth_min};

    /// Test smooth_max converges to max as epsilon -> 0
    #[test]
    fn test_smooth_max_limit() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let exact_max = a.max(b);

        for eps in [1e-2, 1e-4, 1e-6, 1e-8] {
            let smooth = smooth_max(a, b, eps);
            let error = (smooth - exact_max).abs();
            assert!(
                error < eps * 10.0,
                "smooth_max with eps={}: error={} should be < {}",
                eps,
                error,
                eps * 10.0
            );
        }
    }

    /// Test smooth_min converges to min as epsilon -> 0
    #[test]
    fn test_smooth_min_limit() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let exact_min = a.min(b);

        for eps in [1e-2, 1e-4, 1e-6, 1e-8] {
            let smooth = smooth_min(a, b, eps);
            let error = (smooth - exact_min).abs();
            assert!(
                error < eps * 10.0,
                "smooth_min with eps={}: error={}",
                eps,
                error
            );
        }
    }

    /// Test smooth_abs converges to abs
    #[test]
    fn test_smooth_abs_limit() {
        let test_values = [-3.0_f64, -0.1, 0.0, 0.1, 3.0];

        for x in test_values {
            for eps in [1e-2, 1e-4, 1e-6] {
                let smooth = smooth_abs(x, eps);
                let exact = x.abs();
                let error = (smooth - exact).abs();
                // Smooth abs has O(eps) error
                assert!(
                    error < eps + 1e-10,
                    "smooth_abs({}) with eps={}: error={}",
                    x,
                    eps,
                    error
                );
            }
        }
    }

    /// Test smooth_indicator transition
    #[test]
    fn test_smooth_indicator() {
        let eps = 1e-3_f64;

        // Far from boundary should be clear
        assert!(smooth_indicator(-1.0_f64, eps) < 0.01);
        assert!(smooth_indicator(1.0_f64, eps) > 0.99);

        // At zero should be 0.5
        let at_zero: f64 = smooth_indicator(0.0_f64, eps);
        assert!(
            (at_zero - 0.5).abs() < 1e-10,
            "smooth_indicator(0) = {}",
            at_zero
        );
    }
}

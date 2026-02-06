//! Integration tests for module exports.
//!
//! Task 8.1: Verify that all public modules and types are correctly exported
//! and accessible via absolute paths.
//!
//! Note: Tests for infra_domain types (Date, Currency, DayCounter,
//! BusinessDayConvention) should be in infra_domain crate. This file tests
//! pricer_core-specific exports only.

use chrono::NaiveDate;

/// Test that smoothing functions are accessible via absolute path.
#[test]
fn test_smoothing_module_exports() {
    use pricer_core::math::smoothing::{smooth_abs, smooth_indicator, smooth_max, smooth_min};

    // Verify all functions are callable
    let _ = smooth_max(3.0_f64, 5.0, 1e-6);
    let _ = smooth_min(3.0_f64, 5.0, 1e-6);
    let _ = smooth_indicator(0.5_f64, 1e-6);
    let _ = smooth_abs(-2.0_f64, 1e-6);
}

/// Test that trait module is accessible via absolute path.
#[test]
fn test_traits_module_exports() {
    use pricer_core::{
        traits::{
            priceable::{Differentiable, Priceable},
            Float,
        },
        types::error::PricingError,
    };

    // Verify traits can be used
    struct TestInstrument {
        value: f64,
    }

    impl Priceable<f64> for TestInstrument {
        fn price(&self) -> Result<f64, PricingError> { Ok(self.value) }
    }

    impl Differentiable for TestInstrument {}

    let instrument = TestInstrument { value: 100.0 };
    assert_eq!(instrument.price().unwrap(), 100.0);

    // Verify Float trait re-export works
    fn generic_sqrt<T: Float>(x: T) -> T { x.sqrt() }
    assert_eq!(generic_sqrt(4.0_f64), 2.0);
}

/// Test that math module is correctly structured.
#[test]
fn test_math_module_structure() {
    // smoothing submodule
    use pricer_core::math::smoothing;

    let _ = smoothing::smooth_max(1.0_f64, 2.0, 1e-6);
}

/// Test that all DayCountConvention variants are accessible.
/// Note: This tests pricer_core's own DayCountConvention type, not
/// infra_domain's.
#[test]
fn test_day_count_convention_variants() {
    use pricer_core::types::time::DayCountConvention;

    let conventions = [
        DayCountConvention::ActualActual365,
        DayCountConvention::ActualActual360,
        DayCountConvention::Thirty360,
    ];

    for conv in &conventions {
        let _name = conv.name();
        assert!(!_name.is_empty());
    }
}

/// Test that pricer_core-specific error types are accessible.
#[test]
fn test_error_types_exports() {
    use pricer_core::types::error::{InterpolationError, PricingError, SolverError};

    // Verify error types can be created
    let _pricing_err = PricingError::InvalidInput("test".to_string());
    let _interp_err = InterpolationError::InsufficientData { got: 1, need: 2 };
    let _solver_err = SolverError::MaxIterationsExceeded { iterations: 100 };
}

/// Test that FxRate is accessible.
#[test]
fn test_fx_rate_exports() {
    use infra_domain::Currency;
    use pricer_core::types::FxRate;

    let pair: FxRate<f64> = FxRate::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    assert_eq!(pair.base(), Currency::EUR);
    assert_eq!(pair.quote(), Currency::USD);
}

/// Test time module exports (pricer_core-specific utilities).
#[test]
fn test_time_module_exports() {
    use pricer_core::types::time::{time_to_maturity, time_to_maturity_dates, DayCountConvention};

    // Test time_to_maturity with NaiveDate
    let start_naive = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end_naive = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
    let ttm = time_to_maturity(start_naive, end_naive);
    assert!((ttm - 0.4986).abs() < 0.001);

    // Test time_to_maturity_dates with Date
    use infra_domain::Date;
    let start = Date::from_ymd(2024, 1, 1).unwrap();
    let end = Date::from_ymd(2024, 7, 1).unwrap();
    let ttm_dates = time_to_maturity_dates(start, end);
    assert!((ttm_dates - 0.4986).abs() < 0.001);

    // Test DayCountConvention year_fraction
    let yf = DayCountConvention::ActualActual365.year_fraction(start_naive, end_naive);
    assert!((yf - ttm).abs() < 1e-10);
}

/// Test interpolator module exports.
#[test]
fn test_interpolator_exports() {
    use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};

    let xs = vec![0.0_f64, 1.0, 2.0];
    let ys = vec![0.0_f64, 2.0, 4.0];
    let interp = LinearInterpolator::new(&xs, &ys).unwrap();

    let result = interp.interpolate(0.5_f64).unwrap();
    assert!((result - 1.0_f64).abs() < 1e-10);
}

/// Test solver module exports.
#[test]
fn test_solver_exports() {
    use pricer_core::math::solvers::{NewtonRaphsonSolver, SolverConfig};

    let config = SolverConfig::default();
    let solver = NewtonRaphsonSolver::new(config);

    // Find root of f(x) = x^2 - 4, f'(x) = 2x
    // Root is x = 2
    let f = |x: f64| x * x - 4.0;
    let f_prime = |x: f64| 2.0 * x;

    let result = solver.find_root(f, f_prime, 1.0);
    assert!(result.is_ok());
    assert!((result.unwrap() - 2.0).abs() < 1e-8);
}

/// Test that all main modules are public.
#[test]
fn test_main_module_structure() {
    // Verify main module paths
    use pricer_core::math;

    // These should compile if modules are properly exported
    let _ = math::smoothing::smooth_max(1.0_f64, 2.0, 1e-6);
}

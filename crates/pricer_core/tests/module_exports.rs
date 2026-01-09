//! Integration tests for module exports.
//!
//! Task 8.1: Verify that all public modules and types are correctly exported
//! and accessible via absolute paths.

use chrono::NaiveDate;

/// Test that smoothing functions are accessible via absolute path.
#[test]
fn test_smoothing_module_exports() {
    use pricer_core::math::smoothing::smooth_abs;
    use pricer_core::math::smoothing::smooth_indicator;
    use pricer_core::math::smoothing::smooth_max;
    use pricer_core::math::smoothing::smooth_min;

    // Verify all functions are callable
    let _ = smooth_max(3.0_f64, 5.0, 1e-6);
    let _ = smooth_min(3.0_f64, 5.0, 1e-6);
    let _ = smooth_indicator(0.5_f64, 1e-6);
    let _ = smooth_abs(-2.0_f64, 1e-6);
}

/// Test that trait module is accessible via absolute path.
#[test]
fn test_traits_module_exports() {
    use pricer_core::traits::priceable::Differentiable;
    use pricer_core::traits::priceable::Priceable;
    use pricer_core::traits::Float;
    use pricer_core::types::error::PricingError;

    // Verify traits can be used
    struct TestInstrument {
        value: f64,
    }

    impl Priceable<f64> for TestInstrument {
        fn price(&self) -> Result<f64, PricingError> {
            Ok(self.value)
        }
    }

    impl Differentiable for TestInstrument {}

    let instrument = TestInstrument { value: 100.0 };
    assert_eq!(instrument.price().unwrap(), 100.0);

    // Verify Float trait re-export works
    fn generic_sqrt<T: Float>(x: T) -> T {
        x.sqrt()
    }
    assert_eq!(generic_sqrt(4.0_f64), 2.0);
}

/// Test that types module is accessible via absolute path.
#[test]
fn test_types_module_exports() {
    use pricer_core::types::time::time_to_maturity;
    use pricer_core::types::time::time_to_maturity_dates;
    use pricer_core::types::time::Date;
    use pricer_core::types::time::DayCountConvention;

    // Test Date
    let start = Date::from_ymd(2024, 1, 1).unwrap();
    let end = Date::from_ymd(2024, 7, 1).unwrap();

    assert_eq!(start.year(), 2024);
    assert_eq!(start.month(), 1);
    assert_eq!(start.day(), 1);

    // Test DayCountConvention
    let act_365 = DayCountConvention::ActualActual365;
    let yf = act_365.year_fraction_dates(start, end);
    assert!((yf - 0.4986).abs() < 0.001);

    // Test time_to_maturity functions
    let start_naive = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end_naive = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
    let ttm = time_to_maturity(start_naive, end_naive);
    assert!((ttm - 0.4986).abs() < 0.001);

    let ttm_dates = time_to_maturity_dates(start, end);
    assert!((ttm_dates - 0.4986).abs() < 0.001);
}

/// Test that types re-exports work at module level.
#[test]
fn test_types_reexports() {
    use pricer_core::types::Currency;
    use pricer_core::types::CurrencyPair;
    use pricer_core::types::Date;
    use pricer_core::types::DayCountConvention;
    use pricer_core::types::PricingError;

    // Verify re-exports work
    let _usd = Currency::USD;
    let _date = Date::from_ymd(2024, 6, 15).unwrap();
    let _dcc = DayCountConvention::ActualActual365;
    let _pair: CurrencyPair<f64> = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
    let _err = PricingError::InvalidInput("test".to_string());
}

/// Test that DualNumber type is accessible when feature is enabled.
#[cfg(feature = "num-dual-mode")]
#[test]
fn test_dual_module_export() {
    use pricer_core::types::dual::DualNumber;

    let dual = DualNumber::new(3.0, 1.0);
    assert_eq!(dual.re, 3.0);
    assert_eq!(dual.eps, 1.0);
}

/// Test that math module is correctly structured.
#[test]
fn test_math_module_structure() {
    // smoothing submodule
    use pricer_core::math::smoothing;

    let _ = smoothing::smooth_max(1.0_f64, 2.0, 1e-6);
}

/// Test that all DayCountConvention variants are accessible.
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

/// Test that error types are accessible and work correctly.
#[test]
fn test_error_types_exports() {
    use pricer_core::types::error::CurrencyError;
    use pricer_core::types::error::DateError;
    use pricer_core::types::error::InterpolationError;
    use pricer_core::types::error::PricingError;
    use pricer_core::types::error::SolverError;

    // Verify error types can be created
    let _pricing_err = PricingError::InvalidInput("test".to_string());
    let _date_err = DateError::InvalidDate {
        year: 2024,
        month: 13,
        day: 1,
    };
    let _currency_err = CurrencyError::UnknownCurrency("XXX".to_string());
    let _interp_err = InterpolationError::InsufficientData { got: 1, need: 2 };
    let _solver_err = SolverError::MaxIterationsExceeded { iterations: 100 };
}

/// Test that Currency enum variants are accessible.
#[test]
fn test_currency_exports() {
    use pricer_core::types::Currency;

    let currencies = [
        Currency::USD,
        Currency::EUR,
        Currency::GBP,
        Currency::JPY,
        Currency::CHF,
    ];

    for currency in &currencies {
        let code = currency.code();
        assert_eq!(code.len(), 3);
    }
}

/// Test chrono integration with time module.
#[test]
fn test_chrono_integration() {
    use chrono::NaiveDate;
    use pricer_core::types::time::time_to_maturity;
    use pricer_core::types::time::DayCountConvention;

    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

    // Test with NaiveDate directly
    let ttm = time_to_maturity(start, end);
    assert!(ttm > 0.0);

    // Test year_fraction with NaiveDate
    let yf = DayCountConvention::ActualActual365.year_fraction(start, end);
    assert_eq!(ttm, yf);
}

/// Test that BusinessDayConvention is accessible.
#[test]
fn test_business_day_convention_exports() {
    use pricer_core::types::time::BusinessDayConvention;

    let conventions = [
        BusinessDayConvention::Following,
        BusinessDayConvention::ModifiedFollowing,
        BusinessDayConvention::Preceding,
        BusinessDayConvention::ModifiedPreceding,
        BusinessDayConvention::Unadjusted,
    ];

    for conv in &conventions {
        let name = conv.name();
        let code = conv.code();
        assert!(!name.is_empty());
        assert!(!code.is_empty());
    }
}

/// Test that market_data module is accessible.
#[test]
fn test_market_data_module_exports() {
    use pricer_core::market_data::curves::FlatCurve;
    use pricer_core::market_data::curves::YieldCurve;
    use pricer_core::market_data::surfaces::FlatVol;
    use pricer_core::market_data::surfaces::VolatilitySurface;

    // Test FlatCurve
    let curve: FlatCurve<f64> = FlatCurve::new(0.05);
    let df = curve.discount_factor(1.0).unwrap();
    assert!(df > 0.0 && df < 1.0);

    // Test FlatVol
    let vol_surface: FlatVol<f64> = FlatVol::new(0.2);
    let vol = vol_surface.volatility(1.0, 100.0).unwrap();
    assert!((vol - 0.2).abs() < 1e-10);
}

/// Test interpolator module exports.
#[test]
fn test_interpolator_exports() {
    use pricer_core::math::interpolators::Interpolator;
    use pricer_core::math::interpolators::LinearInterpolator;

    let xs = vec![0.0_f64, 1.0, 2.0];
    let ys = vec![0.0_f64, 2.0, 4.0];
    let interp = LinearInterpolator::new(&xs, &ys).unwrap();

    let result = interp.interpolate(0.5_f64).unwrap();
    assert!((result - 1.0_f64).abs() < 1e-10);
}

/// Test solver module exports.
#[test]
fn test_solver_exports() {
    use pricer_core::math::solvers::NewtonRaphsonSolver;
    use pricer_core::math::solvers::SolverConfig;

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
    use pricer_core::market_data;
    use pricer_core::math;
    use pricer_core::types;

    // These should compile if modules are properly exported
    let _ = math::smoothing::smooth_max(1.0_f64, 2.0, 1e-6);
    let _ = types::Date::from_ymd(2024, 1, 1);
    let _: FlatCurve<f64> = market_data::curves::FlatCurve::new(0.05);
}

use pricer_core::market_data::curves::FlatCurve;

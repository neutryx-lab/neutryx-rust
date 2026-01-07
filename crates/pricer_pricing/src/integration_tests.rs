//! L1/L2 integration tests for pricer_kernel.
//!
//! These tests verify that pricer_kernel correctly integrates with
//! pricer_core (L1) and pricer_models (L2) when the `l1l2-integration` feature is enabled.

#[cfg(all(test, feature = "l1l2-integration"))]
mod tests {
    use pricer_core::math::smoothing::{smooth_indicator, smooth_max};
    use pricer_core::traits::Float;

    /// Test that smooth_max from pricer_core is accessible and works correctly.
    #[test]
    fn test_smooth_max_integration() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let epsilon = 1e-6;

        let result = smooth_max(a, b, epsilon);

        // smooth_max should approximate max(a, b) = 5.0
        assert!((result - 5.0).abs() < 1e-3);
    }

    /// Test that smooth_indicator from pricer_core is accessible and works correctly.
    #[test]
    fn test_smooth_indicator_integration() {
        let epsilon = 1e-6;

        // At x=0, smooth_indicator should be approximately 0.5
        let at_zero = smooth_indicator(0.0_f64, epsilon);
        assert!((at_zero - 0.5).abs() < 1e-3);

        // For large positive x, should be close to 1
        let positive = smooth_indicator(10.0_f64, epsilon);
        assert!(positive > 0.99);

        // For large negative x, should be close to 0
        let negative = smooth_indicator(-10.0_f64, epsilon);
        assert!(negative < 0.01);
    }

    /// Test that Float trait from pricer_core is accessible.
    #[test]
    fn test_float_trait_integration() {
        fn use_float<T: Float>(x: T) -> T {
            x + x
        }

        let result = use_float(2.0_f64);
        assert!((result - 4.0).abs() < 1e-10);
    }
}

#[cfg(all(test, feature = "l1l2-integration"))]
mod pricer_models_tests {
    use pricer_models::models::stochastic::{SingleState, StochasticModel, StochasticState};
    use pricer_models::models::StochasticModelEnum;

    /// Test that StochasticModel trait from pricer_models is accessible.
    #[test]
    fn test_stochastic_model_trait_integration() {
        // Verify the trait is importable and usable
        fn accepts_stochastic_model<M: StochasticModel<f64>>(_model: &M) {
            // Just verify the trait bound compiles
        }

        // This test just verifies the import works
        // Actual model usage will be tested in later phases
    }

    /// Test that StochasticModelEnum is accessible for static dispatch.
    #[test]
    fn test_stochastic_model_enum_accessible() {
        // Verify the enum can be imported
        // Full usage tests will be in later phases
        let _enum_type_check: Option<StochasticModelEnum<f64>> = None;
    }

    /// Test that StochasticState trait is importable and usable.
    #[test]
    fn test_stochastic_state_trait() {
        let state = SingleState(100.0_f64);
        assert_eq!(SingleState::<f64>::dimension(), 1);
        assert_eq!(state.get(0), Some(100.0));
    }
}

/// Task 1.4: Instrument enum integration tests.
///
/// These tests verify that the Instrument enum from pricer_models can be
/// used for static dispatch payoff calculations.
#[cfg(all(test, feature = "l1l2-integration"))]
mod instrument_tests {
    use pricer_models::instruments::{
        Direction, ExerciseStyle, Forward, Instrument, InstrumentParams, PayoffType, VanillaOption,
    };

    /// Test that Instrument enum is accessible.
    #[test]
    fn test_instrument_enum_accessible() {
        let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let instrument = Instrument::Vanilla(call);

        // Verify static dispatch payoff works
        let payoff = instrument.payoff(110.0);
        assert!((payoff - 10.0).abs() < 0.01);
    }

    /// Test Forward instrument payoff via static dispatch.
    #[test]
    fn test_forward_instrument_payoff() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        let instrument = Instrument::Forward(forward);

        let payoff_itm = instrument.payoff(110.0);
        assert!((payoff_itm - 10.0).abs() < 1e-10);

        let payoff_otm = instrument.payoff(90.0);
        assert!((payoff_otm - (-10.0)).abs() < 1e-10);
    }

    /// Test that static dispatch is maintained (no Box<dyn>).
    #[test]
    fn test_static_dispatch_pattern() {
        // Create different instruments
        let call_params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(call_params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();

        // Use enum for static dispatch
        let instruments: Vec<Instrument<f64>> =
            vec![Instrument::Vanilla(call), Instrument::Forward(forward)];

        // Compute payoffs via static dispatch (no dynamic allocation)
        let spot = 110.0;
        let payoffs: Vec<f64> = instruments.iter().map(|inst| inst.payoff(spot)).collect();

        // Both should have payoff of 10.0
        assert!((payoffs[0] - 10.0).abs() < 0.01);
        assert!((payoffs[1] - 10.0).abs() < 1e-10);
    }

    /// Test expiry accessor via enum dispatch.
    #[test]
    fn test_instrument_expiry() {
        let params = InstrumentParams::new(100.0_f64, 0.5, 1.0).unwrap();
        let put = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);
        let instrument = Instrument::Vanilla(put);

        assert!((instrument.expiry() - 0.5).abs() < 1e-10);
    }

    /// Test type checking helpers.
    #[test]
    fn test_instrument_type_helpers() {
        let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();

        let vanilla_inst = Instrument::Vanilla(call);
        let forward_inst = Instrument::Forward(forward);

        assert!(vanilla_inst.is_vanilla());
        assert!(!vanilla_inst.is_forward());

        assert!(!forward_inst.is_vanilla());
        assert!(forward_inst.is_forward());
    }
}

/// Task 1.3: YieldCurve trait integration tests.
///
/// These tests verify that the Monte Carlo pricer can use YieldCurve
/// from pricer_core for discount factor calculations.
#[cfg(all(test, feature = "l1l2-integration"))]
mod yield_curve_tests {
    use crate::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams};
    use pricer_core::market_data::curves::{FlatCurve, YieldCurve};

    /// Test that FlatCurve is accessible and can be used for discounting.
    #[test]
    fn test_flat_curve_accessible() {
        let curve = FlatCurve::new(0.05_f64);

        // Discount factor at T=1: exp(-0.05 * 1) ≈ 0.9512
        let df = curve.discount_factor(1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    /// Test pricing European call with YieldCurve.
    #[test]
    fn test_price_with_yield_curve() {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let gbm = GbmParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
            maturity: 1.0,
        };
        let payoff = PayoffParams::call(100.0);
        let curve = FlatCurve::new(0.05_f64);

        // Use the new method that accepts a YieldCurve
        let mut pricer1 = MonteCarloPricer::new(config.clone()).unwrap();
        let result = pricer1.price_european_with_curve(gbm, payoff, &curve);

        // Price should be positive
        assert!(result.price > 0.0);

        // Compare with manual discount factor calculation using a fresh pricer
        let mut pricer2 = MonteCarloPricer::new(config).unwrap();
        let manual_df = curve.discount_factor(gbm.maturity).unwrap();
        let result_manual = pricer2.price_european(gbm, payoff, manual_df);

        // Results should be identical (same seed, same discount factor calculation)
        assert!(
            (result.price - result_manual.price).abs() < 1e-10,
            "Prices differ: {} vs {}",
            result.price,
            result_manual.price
        );
    }

    /// Test that discount factor from YieldCurve matches manual calculation.
    #[test]
    fn test_yield_curve_discount_factor_consistency() {
        let rate = 0.05_f64;
        let maturity = 1.0_f64;
        let curve = FlatCurve::new(rate);

        let df_from_curve = curve.discount_factor(maturity).unwrap();
        let df_manual = (-rate * maturity).exp();

        assert!((df_from_curve - df_manual).abs() < 1e-10);
    }

    /// Test pricing with Greeks using YieldCurve.
    #[test]
    fn test_price_with_greeks_and_curve() {
        use crate::mc::Greek;

        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();

        let mut pricer = MonteCarloPricer::new(config).unwrap();

        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let curve = FlatCurve::new(0.05_f64);

        let result = pricer.price_with_greeks_and_curve(gbm, payoff, &curve, &[Greek::Delta]);

        assert!(result.delta.is_some());
        // Delta of ATM call should be around 0.5-0.6
        let delta = result.delta.unwrap();
        assert!(delta > 0.3 && delta < 0.8, "Delta = {}", delta);
    }
}

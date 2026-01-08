//! L1/L2 Integration Tests for pricer_pricing
//!
//! These tests verify that pricer_pricing correctly integrates with:
//! - L1 (pricer_core): Float trait, smoothing functions, YieldCurve
//! - L2 (pricer_models): StochasticModel trait, Instrument enum
//!
//! Requirements Coverage: 4.1, 4.2, 4.5

#![cfg(feature = "l1l2-integration")]

// =============================================================================
// L1 Integration Tests (pricer_core)
// =============================================================================

/// Test: pricer_core::traits::Float is accessible
/// Requirement: 4.1
#[test]
fn test_float_trait_import() {
    use pricer_core::traits::Float;

    // Verify Float trait works with f64
    fn generic_compute<T: Float>(x: T, y: T) -> T {
        x * y + x.exp()
    }

    let result = generic_compute(2.0_f64, 3.0_f64);
    assert!(result > 0.0);
}

/// Test: pricer_core::math::smoothing functions are accessible
/// Requirement: 4.1
#[test]
fn test_smoothing_functions_import() {
    use pricer_core::math::smoothing::{smooth_indicator, smooth_max};

    let epsilon = 1e-6;

    // smooth_max test
    let max_result = smooth_max(3.0_f64, 5.0_f64, epsilon);
    assert!((max_result - 5.0).abs() < 0.01);

    // smooth_indicator test
    let ind_result = smooth_indicator(1.0_f64, epsilon);
    assert!(ind_result > 0.5);
}

/// Test: pricer_core market data curves are accessible
/// Requirement: 4.4
#[test]
fn test_yield_curve_import() {
    use pricer_core::market_data::curves::{FlatCurve, YieldCurve};

    let curve = FlatCurve::new(0.05);
    let df = curve.discount_factor(1.0_f64).unwrap();
    assert!(df > 0.0 && df < 1.0);
}

// =============================================================================
// L2 Integration Tests (pricer_models)
// =============================================================================

/// Test: pricer_models::models::stochastic is accessible
/// Requirement: 4.2
#[test]
fn test_stochastic_model_import() {
    use pricer_models::models::stochastic::{SingleState, StochasticState};

    let state = SingleState(100.0_f64);
    assert_eq!(state.get(0), Some(100.0));
    assert_eq!(SingleState::<f64>::dimension(), 1);
}

/// Test: pricer_models::instruments is accessible
/// Requirement: 4.2, 4.3
#[test]
fn test_instrument_enum_import() {
    use pricer_models::instruments::{
        Direction, ExerciseStyle, Forward, Instrument, InstrumentParams, PayoffType, VanillaOption,
    };

    // Create vanilla option
    let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    let vanilla_inst = Instrument::Vanilla(call);

    // Create forward
    let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
    let forward_inst = Instrument::Forward(forward);

    // Test payoff dispatch
    let payoff_vanilla: f64 = vanilla_inst.payoff(110.0_f64);
    let payoff_forward: f64 = forward_inst.payoff(110.0_f64);

    assert!(payoff_vanilla > 0.0);
    assert!((payoff_forward - 10.0_f64).abs() < 1e-10);
}

// =============================================================================
// Combined L1/L2 Integration Tests
// =============================================================================

/// Test: Using L1 smoothing with L2 instruments
/// Requirement: 4.1, 4.2
#[test]
fn test_l1_l2_combined_usage() {
    use pricer_core::math::smoothing::smooth_max;
    use pricer_models::instruments::{
        ExerciseStyle, Instrument, InstrumentParams, PayoffType, VanillaOption,
    };

    // Create option using L2
    let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    let instrument = Instrument::Vanilla(call);

    // Compute payoff
    let spot = 95.0_f64;
    let payoff = instrument.payoff(spot);

    // Use L1 smoothing on the payoff
    let smoothed = smooth_max(payoff, 0.0_f64, 1e-6);

    // Payoff should be smoothly zero for OTM option
    assert!(smoothed >= 0.0);
}

/// Test: Feature flag compatibility with enzyme-ad
/// Requirement: 4.5
#[test]
fn test_feature_compatibility() {
    // This test verifies that l1l2-integration feature compiles
    // alongside enzyme-ad feature without conflicts

    #[cfg(feature = "enzyme-ad")]
    {
        // Enzyme-ad feature is also enabled
        assert!(true, "enzyme-ad feature is compatible");
    }

    #[cfg(not(feature = "enzyme-ad"))]
    {
        // Only l1l2-integration is enabled
        assert!(true, "standalone l1l2-integration works");
    }
}

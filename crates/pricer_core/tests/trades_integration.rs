//! Integration tests for trades module.
//!
//! These tests verify that the trades module (instruments + schedules)
//! is correctly integrated into pricer_core.

use pricer_core::{
    trades::instruments::{
        Direction, ExerciseStyle, Forward, Instrument, InstrumentError, InstrumentParams,
        PaymentFrequency, PayoffType, Swap, VanillaOption,
    },
    types::Currency,
};

// ============================================================================
// VanillaOption Tests
// ============================================================================

#[test]
fn test_vanilla_option_call_payoff_itm() {
    let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);

    let payoff = call.payoff(110.0);
    assert!((payoff - 10.0).abs() < 0.01);
}

#[test]
fn test_vanilla_option_put_payoff_itm() {
    let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let put = VanillaOption::new(params, PayoffType::Put, ExerciseStyle::European, 1e-6);

    let payoff = put.payoff(90.0);
    assert!((payoff - 10.0).abs() < 0.01);
}

#[test]
fn test_vanilla_option_accessors() {
    let params = InstrumentParams::new(105.0_f64, 0.5, 1000.0).unwrap();
    let option = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::American, 1e-8);

    assert_eq!(option.strike(), 105.0);
    assert_eq!(option.expiry(), 0.5);
    assert_eq!(option.notional(), 1000.0);
    assert_eq!(option.payoff_type(), PayoffType::Call);
    assert!(option.exercise_style().is_american());
}

// ============================================================================
// Forward Tests
// ============================================================================

#[test]
fn test_forward_long_payoff() {
    let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();

    let payoff = forward.payoff(110.0);
    assert!((payoff - 10.0).abs() < 1e-10);
}

#[test]
fn test_forward_short_payoff() {
    let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Short).unwrap();

    let payoff = forward.payoff(110.0);
    assert!((payoff - (-10.0)).abs() < 1e-10);
}

#[test]
fn test_forward_invalid_strike() {
    let result = Forward::new(-100.0_f64, 1.0, 1.0, Direction::Long);
    assert!(matches!(result, Err(InstrumentError::InvalidStrike { .. })));
}

// ============================================================================
// Swap Tests
// ============================================================================

#[test]
fn test_swap_creation() {
    let dates = vec![0.5_f64, 1.0, 1.5, 2.0];
    let swap = Swap::new(
        1_000_000.0,
        0.03,
        dates,
        PaymentFrequency::SemiAnnual,
        Currency::USD,
    )
    .unwrap();

    assert_eq!(swap.notional(), 1_000_000.0);
    assert_eq!(swap.fixed_rate(), 0.03);
    assert_eq!(swap.num_periods(), 4);
    assert_eq!(swap.maturity(), 2.0);
    assert_eq!(swap.currency(), Currency::USD);
}

#[test]
fn test_swap_fixed_leg_cashflow() {
    let dates = vec![0.5_f64, 1.0];
    let swap = Swap::new(
        1_000_000.0,
        0.04,
        dates,
        PaymentFrequency::SemiAnnual,
        Currency::USD,
    )
    .unwrap();

    let cashflow = swap.fixed_leg_cashflow(0.5);
    assert!((cashflow - 20_000.0).abs() < 1e-10);
}

// ============================================================================
// Instrument Enum Tests
// ============================================================================

#[test]
fn test_instrument_enum_vanilla() {
    let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    let instrument = Instrument::Vanilla(call);

    assert!(instrument.is_vanilla());
    assert!(!instrument.is_forward());
    assert!(!instrument.is_swap());

    let payoff = instrument.payoff(110.0);
    assert!((payoff - 10.0).abs() < 0.01);
}

#[test]
fn test_instrument_enum_forward() {
    let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
    let instrument = Instrument::Forward(forward);

    assert!(!instrument.is_vanilla());
    assert!(instrument.is_forward());
    assert!(!instrument.is_swap());

    let payoff = instrument.payoff(110.0);
    assert!((payoff - 10.0).abs() < 1e-10);
}

// ============================================================================
// PayoffType Tests
// ============================================================================

#[test]
fn test_payoff_type_call() {
    let payoff = PayoffType::Call.evaluate(110.0_f64, 100.0, 1e-6);
    assert!((payoff - 10.0).abs() < 0.01);
}

#[test]
fn test_payoff_type_put() {
    let payoff = PayoffType::Put.evaluate(90.0_f64, 100.0, 1e-6);
    assert!((payoff - 10.0).abs() < 0.01);
}

#[test]
fn test_payoff_type_digital_call() {
    let payoff_itm = PayoffType::DigitalCall.evaluate(110.0_f64, 100.0, 1e-6);
    let payoff_otm = PayoffType::DigitalCall.evaluate(90.0_f64, 100.0, 1e-6);

    assert!(payoff_itm > 0.99);
    assert!(payoff_otm < 0.01);
}

// ============================================================================
// ExerciseStyle Tests
// ============================================================================

#[test]
fn test_exercise_style_european() {
    let style: ExerciseStyle<f64> = ExerciseStyle::European;
    assert!(style.is_european());
    assert!(!style.allows_early_exercise());
    assert!(!style.is_path_dependent());
}

#[test]
fn test_exercise_style_american() {
    let style: ExerciseStyle<f64> = ExerciseStyle::American;
    assert!(style.is_american());
    assert!(style.allows_early_exercise());
}

#[test]
fn test_exercise_style_bermudan() {
    let style = ExerciseStyle::bermudan(vec![0.25_f64, 0.5, 0.75]);
    assert!(style.is_bermudan());
    assert!(style.allows_early_exercise());
}

#[test]
fn test_exercise_style_asian() {
    let style = ExerciseStyle::asian(0.0_f64, 1.0, 12);
    assert!(style.is_asian());
    assert!(style.is_path_dependent());
}

// ============================================================================
// PaymentFrequency Tests
// ============================================================================

#[test]
fn test_payment_frequency() {
    assert_eq!(PaymentFrequency::Annual.periods_per_year(), 1);
    assert_eq!(PaymentFrequency::SemiAnnual.periods_per_year(), 2);
    assert_eq!(PaymentFrequency::Quarterly.periods_per_year(), 4);
    assert_eq!(PaymentFrequency::Monthly.periods_per_year(), 12);
}

#[test]
fn test_payment_frequency_period_fraction() {
    let annual: f64 = PaymentFrequency::Annual.period_fraction();
    let quarterly: f64 = PaymentFrequency::Quarterly.period_fraction();

    assert!((annual - 1.0).abs() < 1e-10);
    assert!((quarterly - 0.25).abs() < 1e-10);
}

// ============================================================================
// InstrumentParams Tests
// ============================================================================

#[test]
fn test_instrument_params_valid() {
    let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
    assert_eq!(params.strike(), 100.0);
    assert_eq!(params.expiry(), 1.0);
    assert_eq!(params.notional(), 1_000_000.0);
}

#[test]
fn test_instrument_params_invalid_strike() {
    let result = InstrumentParams::new(-100.0_f64, 1.0, 1_000_000.0);
    assert!(matches!(result, Err(InstrumentError::InvalidStrike { .. })));
}

#[test]
fn test_instrument_params_invalid_expiry() {
    let result = InstrumentParams::new(100.0_f64, -1.0, 1_000_000.0);
    assert!(matches!(result, Err(InstrumentError::InvalidExpiry { .. })));
}

#[test]
fn test_instrument_params_invalid_notional() {
    let result = InstrumentParams::new(100.0_f64, 1.0, -1_000_000.0);
    assert!(matches!(
        result,
        Err(InstrumentError::InvalidNotional { .. })
    ));
}

// ============================================================================
// Direction Tests
// ============================================================================

#[test]
fn test_direction() {
    assert!(Direction::Long.is_long());
    assert!(!Direction::Long.is_short());
    assert!(Direction::Short.is_short());
    assert!(!Direction::Short.is_long());
}

//! Tests for IRS Greeks module.

use super::*;

// =============================================================================
// IrsGreeksConfig Tests
// =============================================================================

#[test]
fn test_irs_greeks_config_default() {
    let config = IrsGreeksConfig::default();
    assert!((config.bump_size - 0.0001).abs() < 1e-10);
    assert!((config.tolerance - 1e-6).abs() < 1e-10);
}

#[test]
fn test_irs_greeks_config_builder() {
    let config = IrsGreeksConfig::new()
        .with_bump_size(0.0005)
        .with_tolerance(1e-8);

    assert!((config.bump_size - 0.0005).abs() < 1e-10);
    assert!((config.tolerance - 1e-8).abs() < 1e-10);
}

#[test]
fn test_irs_greeks_config_validation_valid() {
    let config = IrsGreeksConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_irs_greeks_config_validation_invalid_bump() {
    let config = IrsGreeksConfig {
        bump_size: -0.0001,
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_irs_greeks_config_validation_invalid_tolerance() {
    let config = IrsGreeksConfig {
        tolerance: 0.0,
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

// =============================================================================
// IrsGreeksError Tests
// =============================================================================

#[test]
fn test_irs_greeks_error_invalid_swap() {
    let err = IrsGreeksError::InvalidSwap("missing notional".to_string());
    assert!(err.to_string().contains("Invalid swap"));
}

#[test]
fn test_irs_greeks_error_curve_not_found() {
    let err = IrsGreeksError::CurveNotFound("SOFR".to_string());
    assert!(err.to_string().contains("SOFR"));
}

#[test]
fn test_irs_greeks_error_conversion_to_greeks_error() {
    use crate::greeks::GreeksError;

    let err = IrsGreeksError::InvalidSwap("test".to_string());
    let greeks_err: GreeksError = err.into();
    assert!(matches!(greeks_err, GreeksError::InvalidSwap(_)));
}

// =============================================================================
// IrsDeltaResult Tests
// =============================================================================

#[test]
fn test_irs_delta_result_new() {
    let tenors: Vec<f64> = vec![0.25, 0.5, 1.0, 2.0, 5.0];
    let deltas: Vec<f64> = vec![100.0, 200.0, 400.0, 800.0, 2000.0];
    let dv01: f64 = 3500.0;

    let result = IrsDeltaResult::new(tenors.clone(), deltas.clone(), dv01, 1000);

    assert_eq!(result.num_tenors(), 5);
    assert!(!result.is_empty());
    assert!((result.dv01 - 3500.0).abs() < 1e-10);
    assert_eq!(result.compute_time_ns, 1000);
}

#[test]
fn test_irs_delta_result_default() {
    let result: IrsDeltaResult<f64> = IrsDeltaResult::default();
    assert!(result.is_empty());
    assert_eq!(result.num_tenors(), 0);
}

// =============================================================================
// IrsGreeksResult Tests
// =============================================================================

#[test]
fn test_irs_greeks_result_new() {
    let result = IrsGreeksResult::<f64>::new(1000.0);
    assert!((result.npv - 1000.0).abs() < 1e-10);
    assert!(!result.has_aad_result());
    assert!(!result.has_bump_result());
}

#[test]
fn test_irs_greeks_result_with_aad() {
    let delta_result = IrsDeltaResult::new(vec![1.0], vec![100.0], 100.0, 1000);
    let result = IrsGreeksResult::<f64>::new(1000.0).with_aad_result(delta_result);

    assert!(result.has_aad_result());
    assert!(result.dv01().is_some());
}

#[test]
fn test_irs_greeks_result_with_both() {
    let aad_result = IrsDeltaResult::new(vec![1.0], vec![100.0], 100.0, 1000);
    let bump_result = IrsDeltaResult::new(vec![1.0], vec![100.5], 100.5, 5000);

    let result = IrsGreeksResult::<f64>::new(1000.0)
        .with_aad_result(aad_result)
        .with_bump_result(bump_result)
        .with_accuracy_check(vec![0.005]);

    assert!(result.has_aad_result());
    assert!(result.has_bump_result());
    assert!(result.accuracy_check.is_some());
    // AAD result is preferred
    assert!((result.dv01().unwrap() - 100.0).abs() < 1e-10);
}

// =============================================================================
// IrsGreeksCalculator Tests
// =============================================================================

#[test]
fn test_irs_greeks_calculator_new() {
    let config = IrsGreeksConfig::default();
    let calculator = IrsGreeksCalculator::<f64>::new(config);
    assert!((calculator.config().bump_size - 0.0001).abs() < 1e-10);
}

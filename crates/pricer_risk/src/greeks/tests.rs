//! Tests for Greeks module.

use super::*;

// =============================================================================
// GreeksConfig Tests
// =============================================================================

#[test]
fn test_greeks_config_default() {
    let config = GreeksConfig::default();
    assert_eq!(config.mode, GreeksMode::BumpRevalue);
    assert!((config.spot_bump_relative - 0.01).abs() < 1e-10);
    assert!((config.vol_bump_absolute - 0.01).abs() < 1e-10);
}

#[test]
fn test_greeks_config_builder() {
    let config = GreeksConfig::builder()
        .mode(GreeksMode::BumpRevalue)
        .spot_bump_relative(0.02)
        .vol_bump_absolute(0.005)
        .build()
        .unwrap();

    assert_eq!(config.mode, GreeksMode::BumpRevalue);
    assert!((config.spot_bump_relative - 0.02).abs() < 1e-10);
    assert!((config.vol_bump_absolute - 0.005).abs() < 1e-10);
}

#[test]
fn test_greeks_config_validate_invalid_spot_bump_negative() {
    let result = GreeksConfig::builder().spot_bump_relative(-0.01).build();
    assert!(result.is_err());
}

#[test]
fn test_greeks_config_validate_invalid_spot_bump_too_large() {
    let result = GreeksConfig::builder().spot_bump_relative(1.5).build();
    assert!(result.is_err());
}

#[test]
fn test_greeks_config_compute_spot_bump() {
    let config = GreeksConfig::default();
    let bump = config.compute_spot_bump(100.0);
    assert!((bump - 1.0).abs() < 1e-10); // 1% of 100
}

// =============================================================================
// GreeksError Tests
// =============================================================================

#[test]
fn test_greeks_error_invalid_spot_bump() {
    let err = GreeksError::invalid_spot_bump("must be positive");
    assert!(err.to_string().contains("spot bump"));
    assert!(err.is_config_error());
    assert!(!err.is_calculation_error());
}

#[test]
fn test_greeks_error_curve_not_found() {
    let err = GreeksError::curve_not_found("SOFR");
    assert!(err.to_string().contains("SOFR"));
    assert!(err.is_calculation_error());
}

#[test]
fn test_greeks_error_accuracy_check_failed() {
    let err = GreeksError::accuracy_check_failed(0.01, 1e-6);
    let display = err.to_string();
    assert!(display.contains("0.01"));
    assert!(err.is_calculation_error());
}

#[test]
fn test_greeks_error_clone_and_equality() {
    let err1 = GreeksError::invalid_spot_bump("test");
    let err2 = err1.clone();
    assert_eq!(err1, err2);
}

// =============================================================================
// GreeksResult Tests
// =============================================================================

#[test]
fn test_greeks_result_default() {
    let result = GreeksResult::<f64>::default();
    assert!((result.price - 0.0).abs() < 1e-10);
    assert!(result.delta.is_none());
    assert!(result.gamma.is_none());
}

#[test]
fn test_greeks_result_new() {
    let result = GreeksResult::<f64>::new(10.5, 0.05);
    assert!((result.price - 10.5).abs() < 1e-10);
    assert!((result.std_error - 0.05).abs() < 1e-10);
    assert!(result.delta.is_none());
}

#[test]
fn test_greeks_result_with_methods() {
    let result = GreeksResult::<f64>::new(10.5, 0.05)
        .with_delta(0.55)
        .with_gamma(0.02)
        .with_vega(25.0)
        .with_theta(-0.05)
        .with_rho(15.0);

    assert_eq!(result.delta, Some(0.55));
    assert_eq!(result.gamma, Some(0.02));
    assert_eq!(result.vega, Some(25.0));
    assert_eq!(result.theta, Some(-0.05));
    assert_eq!(result.rho, Some(15.0));
}

#[test]
fn test_greeks_result_has_greeks() {
    let result = GreeksResult::<f64>::new(10.0, 0.1).with_delta(0.5);
    assert!(result.has_first_order_greeks());
    assert!(!result.has_second_order_greeks());

    let result2 = GreeksResult::<f64>::new(10.0, 0.1).with_gamma(0.02);
    assert!(!result2.has_first_order_greeks());
    assert!(result2.has_second_order_greeks());
}

#[test]
fn test_greeks_result_confidence_95() {
    let result = GreeksResult::<f64>::new(10.0, 0.1);
    let ci = result.confidence_95();
    assert!((ci - 0.196).abs() < 1e-6);
}

#[test]
fn test_greeks_result_confidence_99() {
    let result = GreeksResult::<f64>::new(10.0, 0.1);
    let ci = result.confidence_99();
    assert!((ci - 0.2576).abs() < 1e-6);
}

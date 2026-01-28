//! Integration tests for the Global Curve Solver.
//!
//! These tests verify the complete global curve calibration workflow,
//! including OIS curve construction, mixed instrument calibration,
//! and Jacobian verification.
//!
//! # Requirements Traceability
//!
//! - Task 6.1: OIS curve construction integration test
//! - Task 6.2: Mixed instrument convergence test
//! - Task 6.3: Jacobian consistency verification

#![cfg(feature = "global-bootstrap")]

use approx::assert_relative_eq;
use pricer_models::builder::{
    CalibrationInstrument, CalibrationProblem, GlobalBootstrapConfig, GlobalBootstrapper,
    JacobianMethod,
};
use pricer_models::market::curves::MarketInstrument;

// =============================================================================
// Task 6.1: OIS Curve Construction Integration Test
// =============================================================================

/// Test basic OIS curve construction with a simple upward sloping curve.
#[test]
fn test_ois_curve_construction_basic() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.045),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Verify convergence
    assert!(result.converged);
    assert!(result.iterations < 20, "Should converge quickly");

    // Verify all instruments reprice correctly
    for (i, instrument) in instruments.iter().enumerate() {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(
            error.abs() < 1e-8,
            "Instrument {} has pricing error {} (expected < 1e-8)",
            i,
            error
        );
    }

    // Verify discount factors are positive and decreasing
    for i in 1..result.discount_factors.len() {
        assert!(result.discount_factors[i] < result.discount_factors[i - 1]);
    }
}

/// Test OIS curve construction with a flat curve.
#[test]
fn test_ois_curve_construction_flat() {
    let rate = 0.03;
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, rate),
        MarketInstrument::ois(2.0, rate),
        MarketInstrument::ois(5.0, rate),
        MarketInstrument::ois(10.0, rate),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);

    // For a flat curve, all instruments should reprice exactly
    for instrument in &instruments {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-10);
    }
}

/// Test OIS curve construction with an inverted curve.
#[test]
fn test_ois_curve_construction_inverted() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.05),
        MarketInstrument::ois(2.0, 0.045),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.035),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);

    // Verify all instruments reprice correctly
    for instrument in &instruments {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-8);
    }
}

/// Test OIS curve construction using the problem-based approach.
#[test]
fn test_ois_curve_construction_with_problem() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.045),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate_with_problem(instruments.clone()).unwrap();

    assert!(result.converged);

    // Verify pricing errors
    if let Some(errors) = &result.pricing_errors {
        for (i, error) in errors.iter().enumerate() {
            let e: f64 = *error;
            assert!(
                e.abs() < 1e-8,
                "Instrument {} has pricing error {}",
                i,
                e
            );
        }
    }
}

// =============================================================================
// Task 6.2: Mixed Instrument Type Convergence Test
// =============================================================================

/// Test convergence with mixed instrument types (OIS and FRA).
#[test]
fn test_mixed_instruments_convergence() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::fra(0.0, 0.5, 0.025),
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);

    // All instruments should reprice
    for instrument in &instruments {
        let error = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-7);
    }
}

/// Test with only FRA instruments.
#[test]
fn test_fra_only_curve() {
    let instruments = vec![
        MarketInstrument::fra(0.0, 0.5, 0.025),
        MarketInstrument::fra(0.5, 1.0, 0.028),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
}

// =============================================================================
// Task 6.3: Jacobian Consistency Verification
// =============================================================================

/// Verify that finite difference and central difference Jacobians are consistent.
#[test]
fn test_jacobian_finite_vs_central_difference() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    // Create problem with finite difference
    let problem_fd = CalibrationProblem::with_config(
        instruments.clone(),
        pricer_models::builder::CalibrationProblemConfig {
            jacobian_method: JacobianMethod::FiniteDifference,
            ..Default::default()
        },
    )
    .unwrap();

    // Create problem with central difference
    let problem_cd = CalibrationProblem::with_config(
        instruments,
        pricer_models::builder::CalibrationProblemConfig {
            jacobian_method: JacobianMethod::CentralDifference,
            ..Default::default()
        },
    )
    .unwrap();

    // Evaluate Jacobian at initial guess
    let initial = problem_fd.initial_guess();
    let j_fd = problem_fd.compute_jacobian_finite_diff(&initial).unwrap();
    let j_cd = problem_cd.compute_jacobian_central_diff(&initial).unwrap();

    // Jacobians should be similar
    for i in 0..3 {
        for j in 0..3 {
            assert_relative_eq!(j_fd[(i, j)], j_cd[(i, j)], epsilon = 1e-4);
        }
    }
}

/// Verify that calibrated result satisfies all pricing constraints.
#[test]
fn test_calibrated_result_satisfies_constraints() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.045),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Create a problem to verify constraints
    let problem = CalibrationProblem::new(instruments.clone()).unwrap();

    // Get log(DF) from result
    let log_df: Vec<f64> = result.discount_factors.iter().map(|df: &f64| df.ln()).collect();

    // Compute residuals at the solution
    let curve = problem.build_curve(&log_df).unwrap();
    let residuals = problem.compute_residuals(&curve).unwrap();

    // All residuals should be near zero
    for (i, r) in residuals.iter().enumerate() {
        assert!(
            r.abs() < 1e-8,
            "Residual {} = {} exceeds tolerance",
            i,
            r
        );
    }
}

// =============================================================================
// Additional Robustness Tests
// =============================================================================

/// Test with high precision configuration.
#[test]
fn test_high_precision_calibration() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::high_precision();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);

    // High precision should achieve very small residuals
    for instrument in &instruments {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-12);
    }
}

/// Test with fast configuration.
#[test]
fn test_fast_calibration() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::fast();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Fast config should still converge (but with lower precision)
    assert!(result.converged);

    // Pricing errors should be reasonable
    for instrument in &instruments {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-5);
    }
}

/// Test that single instrument calibration works.
#[test]
fn test_single_instrument() {
    let instruments = vec![MarketInstrument::ois(5.0, 0.035)];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert_eq!(result.pillars.len(), 1);

    // The single instrument should reprice exactly
    let error: f64 = instruments[0].pricing_error(&result.curve).unwrap();
    assert!(error.abs() < 1e-10);
}

/// Test with many instruments (stress test).
#[test]
fn test_many_instruments() {
    let maturities: Vec<f64> = (1..=20).map(|i| i as f64).collect();
    let instruments: Vec<MarketInstrument<f64>> = maturities
        .iter()
        .map(|&t| {
            let rate = 0.02 + 0.001 * t; // Linear interpolation
            MarketInstrument::ois(t, rate)
        })
        .collect();

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert_eq!(result.pillars.len(), 20);

    // All instruments should reprice
    for instrument in &instruments {
        let error = instrument.pricing_error(&result.curve).unwrap();
        assert!(error.abs() < 1e-7);
    }
}

/// Test debug logging produces residual history.
#[test]
fn test_debug_logging_residual_history() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default().with_debug_logging(true);
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(result.has_residual_history());

    let history = result.residual_history.as_ref().unwrap();
    assert!(!history.is_empty());
}

/// Test condition number is computed.
#[test]
fn test_condition_number_computed() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(result.condition_number.is_some());

    // Condition number should be reasonable for a well-posed problem
    let cond = result.condition_number.unwrap();
    assert!(cond > 0.0);
    assert!(cond < 1e12);
}

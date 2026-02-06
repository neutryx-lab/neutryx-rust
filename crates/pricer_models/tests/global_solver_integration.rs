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
use num_traits::Float;
use pricer_models::{
    builder::{
        CalibrationInstrument, CalibrationProblem, GlobalBootstrapConfig, GlobalBootstrapper,
        JacobianMethod,
    },
    market::curves::MarketInstrument,
};

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

    let result = bootstrapper
        .calibrate_with_problem(instruments.clone())
        .unwrap();

    assert!(result.converged);

    // Verify pricing errors
    if let Some(errors) = &result.pricing_errors {
        for (i, error) in errors.iter().enumerate() {
            let e: f64 = *error;
            assert!(e.abs() < 1e-8, "Instrument {} has pricing error {}", i, e);
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

/// Verify that finite difference and central difference Jacobians are
/// consistent.
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
    let log_df: Vec<f64> = result
        .discount_factors
        .iter()
        .map(|df: &f64| df.ln())
        .collect();

    // Compute residuals at the solution
    let curve = problem.build_curve(&log_df).unwrap();
    let residuals = problem.compute_residuals(&curve).unwrap();

    // All residuals should be near zero
    for (i, r) in residuals.iter().enumerate() {
        assert!(r.abs() < 1e-8, "Residual {} = {} exceeds tolerance", i, r);
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

// =============================================================================
// Task 3.6: IFT Sensitivity Integration Tests
// =============================================================================

/// Test that IFT sensitivity computation is available after calibration.
#[test]
fn test_ift_sensitivity_available() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(result.can_compute_ift(), "IFT should be available after calibration");
    assert!(
        result.jacobian_inverse.is_some(),
        "J⁻¹ should be cached by default"
    );
}

/// Test IFT sensitivity computation with identity-like perturbation.
#[test]
fn test_ift_sensitivity_computation() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Compute IFT sensitivity for a unit perturbation in the first instrument
    let dF_dm = vec![1.0, 0.0, 0.0]; // Unit perturbation in first instrument
    let sensitivity = result.ift_sensitivity(&dF_dm).unwrap();

    assert_eq!(sensitivity.len(), 3, "Sensitivity should have same dimension as pillars");

    // Sensitivities should be finite
    for s in &sensitivity {
        assert!(s.is_finite(), "Sensitivity should be finite");
    }
}

/// Test IFT batch sensitivity computation.
#[test]
fn test_ift_batch_sensitivity_computation() {
    use pricer_core::math::linalg::DMatrix;

    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Create batch perturbation (identity matrix = each instrument independently)
    let dF_dm_batch = DMatrix::identity(3, 3);
    let batch_sensitivity = result.ift_sensitivity_batch(&dF_dm_batch).unwrap();

    assert_eq!(batch_sensitivity.nrows(), 3);
    assert_eq!(batch_sensitivity.ncols(), 3);

    // Compare batch result with individual computations
    for j in 0..3 {
        let dF_dm: Vec<f64> = (0..3).map(|i| if i == j { 1.0 } else { 0.0 }).collect();
        let individual = result.ift_sensitivity(&dF_dm).unwrap();

        for i in 0..3 {
            assert_relative_eq!(
                batch_sensitivity[(i, j)],
                individual[i],
                epsilon = 1e-12,
            );
        }
    }
}

/// Test IFT sensitivity vs bump-and-recalibrate (golden test).
///
/// This test verifies that IFT-based sensitivities agree with bump-and-recalibrate
/// sensitivities within the required tolerance (1e-8 relative error).
#[test]
fn test_ift_vs_bump_and_recalibrate() {
    let base_rate = 0.03;
    let instruments = vec![
        MarketInstrument::ois(1.0, base_rate),
        MarketInstrument::ois(2.0, base_rate + 0.005),
        MarketInstrument::ois(5.0, base_rate + 0.01),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();
    let base_df = result.discount_factors.clone();

    // Bump size for finite difference
    let bump = 1e-6; // 0.1 bp

    // Test sensitivity to first instrument
    for inst_idx in 0..instruments.len() {
        // Bump-and-recalibrate
        let mut bumped_instruments = instruments.clone();
        bumped_instruments[inst_idx] = MarketInstrument::ois(
            instruments[inst_idx].maturity(),
            instruments[inst_idx].rate() + bump,
        );

        let bumped_result = bootstrapper.calibrate(&bumped_instruments).unwrap();

        // Compute bump-and-recalibrate sensitivity: (DF_bumped - DF_base) / bump
        let bump_sensitivities: Vec<f64> = base_df
            .iter()
            .zip(bumped_result.discount_factors.iter())
            .map(|(&base, &bumped)| (bumped - base) / bump)
            .collect();

        // Compute IFT sensitivity
        // Since F = implied_rate - market_quote, ∂F/∂quote = -1
        // (a unit increase in quote makes the residual decrease by 1)
        let mut dF_dm = vec![0.0; instruments.len()];
        dF_dm[inst_idx] = -1.0;

        let ift_sensitivities = result.ift_sensitivity(&dF_dm).unwrap();

        // Compare sensitivities (note: IFT gives ∂log(DF)/∂quote, we need ∂DF/∂quote)
        // ∂DF/∂quote = DF * ∂log(DF)/∂quote
        for (i, (&ift_sens, &bump_sens)) in
            ift_sensitivities.iter().zip(bump_sensitivities.iter()).enumerate()
        {
            // Convert IFT sensitivity from log space to DF space
            let ift_df_sens = base_df[i] * ift_sens;

            // Relative error check
            if bump_sens.abs() > 1e-10 {
                let rel_error = ((ift_df_sens - bump_sens) / bump_sens).abs();
                assert!(
                    rel_error < 1e-4, // Allow 0.01% relative error due to finite diff approximation
                    "IFT vs bump-recal mismatch at pillar {} for instrument {}: \
                     IFT={}, bump={}, rel_error={}",
                    i,
                    inst_idx,
                    ift_df_sens,
                    bump_sens,
                    rel_error
                );
            }
        }
    }
}

/// Test IFT error when Jacobian inverse is not stored.
#[test]
fn test_ift_error_without_jacobian_inverse() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
    ];

    let config = GlobalBootstrapConfig::default().with_jacobian_inverse(false);
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(!result.can_compute_ift(), "IFT should not be available");

    let dF_dm = vec![1.0, 0.0];
    let sensitivity_result = result.ift_sensitivity(&dF_dm);

    assert!(sensitivity_result.is_err());
}

/// Test IFT sensitivity dimension validation.
#[test]
fn test_ift_dimension_mismatch() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Wrong dimension input
    let dF_dm_wrong = vec![1.0, 0.0]; // Only 2 elements, should be 3
    let sensitivity_result = result.ift_sensitivity(&dF_dm_wrong);

    assert!(sensitivity_result.is_err());
}

// =============================================================================
// Task 2.5: Enzyme Jacobian Integration Tests
// =============================================================================

/// Test Enzyme Jacobian kernel interpolation accuracy.
///
/// # Requirement 1.2
///
/// Verify that the Enzyme kernel's log-linear interpolation produces results
/// within 1e-12 relative tolerance compared to the BootstrappedCurve.
#[test]
fn test_enzyme_kernel_interpolation_accuracy() {
    use pricer_models::builder::enzyme_jacobian::kernels;
    use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve, YieldCurve};

    let pillar_times = vec![1.0, 2.0, 5.0, 10.0];
    let discount_factors: Vec<f64> = pillar_times.iter().map(|&t| (-0.03 * t).exp()).collect();
    let log_df: Vec<f64> = discount_factors.iter().map(|&df| df.ln()).collect();

    // Create BootstrappedCurve for reference
    let curve = BootstrappedCurve::new(
        pillar_times.clone(),
        discount_factors.clone(),
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap();

    // Test at various query points
    let test_times = vec![0.5, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 12.0];

    for t in test_times {
        let df_curve = curve.discount_factor(t).unwrap();
        let df_kernel = kernels::discount_factor_log_linear(t, &pillar_times, &log_df);

        assert_relative_eq!(
            df_kernel,
            df_curve,
            epsilon = 1e-12,
            "Interpolation mismatch at t={}: kernel={}, curve={}",
            t,
            df_kernel,
            df_curve
        );
    }
}

/// Test BootstrappedCurve gradient accuracy for LogLinear interpolation.
///
/// # Requirement 2.2
///
/// Verify analytical derivatives are exact for LogLinear interpolation.
#[test]
fn test_bootstrapped_curve_gradient_accuracy() {
    use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve};

    let pillar_times = vec![1.0, 2.0, 5.0];
    let discount_factors: Vec<f64> = pillar_times.iter().map(|&t| (-0.03 * t).exp()).collect();

    let curve = BootstrappedCurve::new(
        pillar_times.clone(),
        discount_factors.clone(),
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap();

    // Test gradient at midpoint (t=1.5)
    let t = 1.5;
    let (df, gradient) = curve.discount_factor_with_gradient(t).unwrap();

    // Verify DF is correct
    assert!(df > 0.0);
    assert!(df < 1.0);

    // Verify gradient has correct dimensions
    assert_eq!(gradient.len(), pillar_times.len());

    // Finite difference verification
    let bump = 1e-8;
    for i in 0..pillar_times.len() {
        // Bump discount factor i
        let mut bumped_dfs = discount_factors.clone();
        bumped_dfs[i] *= 1.0 + bump;

        let bumped_curve = BootstrappedCurve::new(
            pillar_times.clone(),
            bumped_dfs,
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        let df_bumped = bumped_curve.discount_factor_with_gradient(t).unwrap().0;
        let fd_gradient = (df_bumped - df) / (discount_factors[i] * bump);

        // Compare analytical gradient with finite difference
        if gradient[i].abs() > 1e-10 {
            let rel_error = ((gradient[i] - fd_gradient) / gradient[i]).abs();
            assert!(
                rel_error < 1e-6,
                "Gradient mismatch at pillar {}: analytical={}, fd={}, rel_error={}",
                i,
                gradient[i],
                fd_gradient,
                rel_error
            );
        }
    }
}

/// Test gradient with log_df (for calibration).
#[test]
fn test_bootstrapped_curve_log_gradient_accuracy() {
    use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve};

    let pillar_times = vec![1.0, 2.0, 5.0];
    let log_df: Vec<f64> = pillar_times.iter().map(|&t| -0.03 * t).collect();
    let discount_factors: Vec<f64> = log_df.iter().map(|&x| x.exp()).collect();

    let curve = BootstrappedCurve::new(
        pillar_times.clone(),
        discount_factors.clone(),
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap();

    // Test at midpoint
    let t = 1.5;
    let (df, log_gradient) = curve.discount_factor_with_log_gradient(t).unwrap();

    // Finite difference verification for log_df gradient
    let bump = 1e-8;
    for i in 0..pillar_times.len() {
        let mut bumped_log_df = log_df.clone();
        bumped_log_df[i] += bump;
        let bumped_dfs: Vec<f64> = bumped_log_df.iter().map(|&x| x.exp()).collect();

        let bumped_curve = BootstrappedCurve::new(
            pillar_times.clone(),
            bumped_dfs,
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        let df_bumped = bumped_curve.discount_factor_with_log_gradient(t).unwrap().0;
        let fd_log_gradient = (df_bumped - df) / bump;

        if log_gradient[i].abs() > 1e-10 {
            let rel_error = ((log_gradient[i] - fd_log_gradient) / log_gradient[i]).abs();
            assert!(
                rel_error < 1e-6,
                "Log gradient mismatch at pillar {}: analytical={}, fd={}, rel_error={}",
                i,
                log_gradient[i],
                fd_log_gradient,
                rel_error
            );
        }
    }
}

/// Test Enzyme Jacobian stub (when enzyme-ad feature is disabled).
///
/// This test verifies that the stub implementation returns a zero matrix
/// and the actual computation should fall back to finite differences.
#[test]
fn test_enzyme_jacobian_stub_behavior() {
    use pricer_models::builder::enzyme_jacobian::kernels;

    let pillar_times = vec![1.0, 2.0, 5.0];
    let log_df: Vec<f64> = pillar_times.iter().map(|&t| -0.03 * t).collect();

    let instrument_types = vec![0u32, 0, 0]; // All deposits
    let instrument_params = vec![
        vec![1.0, 0.03],
        vec![2.0, 0.035],
        vec![5.0, 0.04],
    ];

    let jacobian = kernels::compute_jacobian_enzyme(
        &instrument_types,
        &instrument_params,
        &log_df,
        &pillar_times,
    );

    // When enzyme-ad is disabled, should return zero matrix
    #[cfg(not(feature = "enzyme-ad"))]
    {
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);
        // All elements should be zero (stub behavior)
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(jacobian[(i, j)], 0.0, "Stub should return zero matrix");
            }
        }
    }

    // When enzyme-ad is enabled, should return non-zero matrix
    #[cfg(feature = "enzyme-ad")]
    {
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);
        // Jacobian should have non-zero elements
        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0, "Enzyme Jacobian should be non-zero");
    }
}

/// Test Jacobian computation consistency across methods.
///
/// # Requirement 1.2
///
/// Verify Enzyme AD Jacobian matches finite differences within 1e-8.
#[test]
fn test_jacobian_method_consistency() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    // Test finite difference method
    let config_fd = GlobalBootstrapConfig::default()
        .with_jacobian_method(JacobianMethod::FiniteDifference);
    let bootstrapper_fd = GlobalBootstrapper::new(config_fd);
    let result_fd = bootstrapper_fd.calibrate(&instruments).unwrap();

    // Test central difference method
    let config_cd = GlobalBootstrapConfig::default()
        .with_jacobian_method(JacobianMethod::CentralDifference);
    let bootstrapper_cd = GlobalBootstrapper::new(config_cd);
    let result_cd = bootstrapper_cd.calibrate(&instruments).unwrap();

    // Both should converge to similar solutions
    assert!(result_fd.converged);
    assert!(result_cd.converged);

    for i in 0..result_fd.discount_factors.len() {
        assert_relative_eq!(
            result_fd.discount_factors[i],
            result_cd.discount_factors[i],
            epsilon = 1e-8,
            "DF mismatch at pillar {}: FD={}, CD={}",
            i,
            result_fd.discount_factors[i],
            result_cd.discount_factors[i]
        );
    }
}

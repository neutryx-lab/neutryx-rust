use approx::assert_relative_eq;
use pricer_core::{math::linalg::DMatrix, types::SolverError};

use super::*;
use crate::{
    builder::{
        jump::{JumpConfig, JumpPillar},
        problem::JacobianMethod,
        CalibrationInstrument, CalibrationProblem,
    },
    market::curves::MarketInstrument,
};

fn create_test_instruments() -> Vec<MarketInstrument<f64>> {
    vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.032),
        MarketInstrument::ois(5.0, 0.035),
        MarketInstrument::ois(10.0, 0.04),
    ]
}

/// Calibrates instruments with the given config and asserts convergence
/// plus per-instrument pricing error within tolerance.
fn assert_calibration_converges(
    instruments: &[MarketInstrument<f64>],
    config: GlobalBootstrapConfig<f64>,
    tolerance: f64,
) -> GlobalBootstrapResult<f64> {
    let bootstrapper = GlobalBootstrapper::new(config);
    let result = bootstrapper.calibrate(instruments).unwrap();
    assert!(result.converged);
    for (i, instrument) in instruments.iter().enumerate() {
        let error: f64 = instrument.pricing_error(&result.curve).unwrap();
        assert!(
            error.abs() < tolerance,
            "Instrument {i} has pricing error {error}"
        );
    }
    result
}

#[test]
fn test_config_default() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default();
    assert_relative_eq!(config.tolerance, 1e-10, epsilon = 1e-15);
    assert_eq!(config.max_iterations, 100);
    assert!(config.store_jacobian_inverse);
    assert_eq!(config.jacobian_method, JacobianMethod::FiniteDifference);
    assert!(config.enable_telescoping);
    assert!(config.damping_factor.is_none());
    assert!(!config.debug_logging);
}

#[test]
fn test_config_high_precision() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::high_precision();
    assert!(config.tolerance < 1e-12);
    assert!(config.max_iterations >= 500);
    assert_eq!(config.jacobian_method, JacobianMethod::CentralDifference);
}

#[test]
fn test_config_fast() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::fast();
    assert!(config.tolerance > 1e-8);
    assert!(!config.store_jacobian_inverse);
    assert_eq!(config.jacobian_method, JacobianMethod::FiniteDifference);
}

#[test]
fn test_config_builder_methods() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig {
        jacobian_method: JacobianMethod::Analytical,
        enable_telescoping: false,
        damping_factor: Some(0.01),
        debug_logging: true,
        max_condition_number: 1e8,
        tolerance: 1e-12,
        param_tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    assert_eq!(config.jacobian_method, JacobianMethod::Analytical);
    assert!(!config.enable_telescoping);
    assert_relative_eq!(config.damping_factor.unwrap(), 0.01, epsilon = 1e-15);
    assert!(config.debug_logging);
    assert_relative_eq!(config.max_condition_number, 1e8, epsilon = 1e-5);
    assert_relative_eq!(config.tolerance, 1e-12, epsilon = 1e-15);
    assert_eq!(config.max_iterations, 200);
}

#[test]
fn test_calibrate_basic() {
    let instruments = create_test_instruments();
    let result = assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-8);
    assert_eq!(result.pillars.len(), 4);
    assert_eq!(result.discount_factors.len(), 4);
    for &df in &result.discount_factors {
        assert!(df > 0.0 && df <= 1.0);
    }
}

#[test]
fn test_calibrate_stores_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.has_jacobian_inverse());
    let j_inv = result.jacobian_inverse.as_ref().unwrap();
    assert_eq!(j_inv.nrows(), 4);
    assert_eq!(j_inv.ncols(), 4);
}

#[test]
fn test_calibrate_without_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::fast();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(!result.has_jacobian_inverse());
}

#[test]
fn test_calibrate_empty_instruments_error() {
    let instruments: Vec<MarketInstrument<f64>> = vec![];
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

    let result = bootstrapper.calibrate(&instruments);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SolverError::NumericalInstability(_)
    ));
}

#[test]
fn test_calibrate_single_instrument() {
    let instruments = vec![MarketInstrument::ois(5.0, 0.03)];
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert_eq!(result.pillars.len(), 1);

    let error = instruments[0].pricing_error(&result.curve).unwrap();
    assert!(error.abs() < 1e-8);
}

#[test]
fn test_vector_norm() {
    let v = vec![3.0, 4.0];
    assert_relative_eq!(vector_norm(&v), 5.0, epsilon = 1e-10);

    let v2 = vec![1.0, 1.0, 1.0, 1.0];
    assert_relative_eq!(vector_norm(&v2), 2.0, epsilon = 1e-10);
}

#[test]
fn test_calibrate_with_debug_logging() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig {
        debug_logging: true,
        ..Default::default()
    };
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(result.has_residual_history());

    let history = result.residual_history.as_ref().unwrap();
    assert!(!history.is_empty());
}

#[test]
fn test_convergence_quality() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let quality = result.convergence_quality(1e-10);
    assert!(quality == "excellent" || quality == "good");
}

#[test]
fn test_config_default_no_jumps() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig::default();
    assert!(config.jump_config.is_none());
    assert!(!config.has_jumps());
    assert_eq!(config.num_jumps(), 0);
}

#[test]
fn test_config_with_jump_config() {
    let jump_config =
        JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(1.0, 25.0)]);

    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig {
        jump_config: Some(jump_config),
        ..Default::default()
    };

    assert!(config.jump_config.is_some());
    assert!(config.has_jumps());
    assert_eq!(config.num_jumps(), 2);
}

#[test]
fn test_config_with_jumps_convenience() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig {
        jump_config: Some(JumpConfig::with_pillars(vec![
            JumpPillar::new(0.25, 25.0),
            JumpPillar::new(0.5, 25.0),
            JumpPillar::new(1.0, 25.0),
        ])),
        ..Default::default()
    };

    assert!(config.has_jumps());
    assert_eq!(config.num_jumps(), 3);

    let jump_config = config.jump_config.unwrap();
    assert!(jump_config.enabled);
    assert_eq!(jump_config.jump_pillars.len(), 3);
}

#[test]
fn test_config_with_empty_jumps() {
    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig {
        jump_config: Some(JumpConfig::with_pillars(vec![])),
        ..Default::default()
    };

    // Empty jump list should not activate jumps
    assert!(!config.has_jumps());
    assert_eq!(config.num_jumps(), 0);
}

#[test]
fn test_config_with_disabled_jump_config() {
    let jump_config = JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0)]).disable();

    let config: GlobalBootstrapConfig<f64> = GlobalBootstrapConfig {
        jump_config: Some(jump_config),
        ..Default::default()
    };

    // Jump config exists but is disabled
    assert!(config.jump_config.is_some());
    assert!(!config.has_jumps()); // Not active because disabled
    assert_eq!(config.num_jumps(), 1); // But pillars still counted
}

#[test]
fn test_merge_pillars_no_overlap() {
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
    let regular = vec![1.0, 2.0, 5.0];
    let jumps = vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(3.0, 10.0)];

    let (merged, indices) = bootstrapper.merge_pillars(&regular, &jumps, 1e-10);

    assert_eq!(merged.len(), 5); // 3 regular + 2 jumps
    assert_eq!(merged, vec![0.5, 1.0, 2.0, 3.0, 5.0]);
    assert_eq!(indices, vec![0, 3]); // Positions of jump pillars
}

#[test]
fn test_merge_pillars_with_overlap() {
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();
    let regular = vec![0.5, 1.0, 2.0, 5.0]; // 0.5 coincides with jump
    let jumps = vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(3.0, 10.0)];

    let (merged, indices) = bootstrapper.merge_pillars(&regular, &jumps, 1e-10);

    assert_eq!(merged.len(), 5); // Only one 0.5, plus 3.0 added
    assert_eq!(merged, vec![0.5, 1.0, 2.0, 3.0, 5.0]);
    assert_eq!(indices, vec![0, 3]); // First jump at index 0 (existing),
                                     // second at 3
}

#[test]
fn test_calibrate_with_jumps_basic() {
    let instruments = create_test_instruments();
    let jump_pillars = vec![JumpPillar::new(0.5, 10.0)]; // Small 10bps jump
    let config = GlobalBootstrapConfig {
        tolerance: 1e-8,
        param_tolerance: 1e-8,
        ..Default::default()
    };
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate_with_jumps(&instruments, jump_pillars);

    // The calibration may or may not converge depending on the setup,
    // but it should not panic
    match result {
        Ok(res) => {
            assert!(res.has_jumps());
            assert_eq!(res.num_jumps(), 1);
            // Verify jumps have realised values
            let jumps = res.realised_jumps.unwrap();
            assert!(jumps[0].is_calibrated());
        }
        Err(e) => {
            // If it fails, check it's a convergence issue not a panic
            assert!(matches!(
                e,
                SolverError::MaxIterationsExceeded { .. } | SolverError::NumericalInstability(_)
            ));
        }
    }
}

#[test]
fn test_calibrate_with_empty_jumps_falls_back() {
    let instruments = create_test_instruments();
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

    // Empty jump pillars should fall back to regular calibrate
    let result = bootstrapper
        .calibrate_with_jumps(&instruments, vec![])
        .unwrap();

    assert!(result.converged);
    // Regular calibrate returns no realised jumps
    assert!(result.realised_jumps.is_none());
}

#[test]
fn test_result_jump_helpers() {
    let instruments = create_test_instruments();
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Regular calibration has no jumps
    assert!(!result.has_jumps());
    assert_eq!(result.num_jumps(), 0);
    assert_eq!(result.total_jump_bps(), 0.0);
}

#[test]
fn test_can_compute_ift_with_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(result.has_jacobian_inverse());
    assert!(result.can_compute_ift());
}

#[test]
fn test_can_compute_ift_without_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::fast(); // Does not store J⁻¹
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert!(!result.has_jacobian_inverse());
    assert!(!result.can_compute_ift());
}

#[test]
fn test_ift_sensitivity_basic() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Sensitivity of residuals to a 1bp parallel shift
    let n = result.pillars.len();
    let dF_dm: Vec<f64> = vec![0.0001; n]; // 1bp shift

    let sensitivity = result.ift_sensitivity(&dF_dm).unwrap();

    assert_eq!(sensitivity.len(), n);
    // All sensitivities should be finite
    for &s in &sensitivity {
        assert!(s.is_finite());
    }
}

#[test]
fn test_ift_sensitivity_no_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::fast(); // No J⁻¹ stored
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let dF_dm = vec![0.0001; result.pillars.len()];
    let err = result.ift_sensitivity(&dF_dm).unwrap_err();

    assert!(matches!(
        err,
        super::super::super::IftError::NoJacobianInverse
    ));
}

#[test]
fn test_ift_sensitivity_dimension_mismatch() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Wrong length: 2 instead of 4
    let dF_dm = vec![0.0001, 0.0001];
    let err = result.ift_sensitivity(&dF_dm).unwrap_err();

    match err {
        super::super::super::IftError::DimensionMismatch { expected, got } => {
            assert_eq!(expected, 4);
            assert_eq!(got, 2);
        }
        _ => panic!("Expected DimensionMismatch error"),
    }
}

#[test]
fn test_ift_sensitivity_batch_basic() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let n = result.pillars.len();
    let n_params = 3;

    // Create batch sensitivity matrix
    let dF_dm_batch = DMatrix::from_fn(n, n_params, |i, j| 0.0001 * ((i + j + 1) as f64));

    let sensitivities = result.ift_sensitivity_batch(&dF_dm_batch).unwrap();

    assert_eq!(sensitivities.nrows(), n);
    assert_eq!(sensitivities.ncols(), n_params);

    // All values should be finite
    for &val in sensitivities.iter() {
        assert!(val.is_finite());
    }
}

#[test]
fn test_ift_sensitivity_batch_no_jacobian_inverse() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::fast();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let dF_dm_batch = DMatrix::from_element(result.pillars.len(), 2, 0.0001);
    let err = result.ift_sensitivity_batch(&dF_dm_batch).unwrap_err();

    assert!(matches!(
        err,
        super::super::super::IftError::NoJacobianInverse
    ));
}

#[test]
fn test_ift_sensitivity_batch_dimension_mismatch() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    // Wrong number of rows: 2 instead of 4
    let dF_dm_batch = DMatrix::from_element(2, 3, 0.0001);
    let err = result.ift_sensitivity_batch(&dF_dm_batch).unwrap_err();

    match err {
        super::super::super::IftError::BatchDimensionMismatch { expected, got } => {
            assert_eq!(expected, 4);
            assert_eq!(got, 2);
        }
        _ => panic!("Expected BatchDimensionMismatch error"),
    }
}

#[test]
fn test_ift_sensitivity_single_vs_batch_consistency() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let n = result.pillars.len();

    // Single sensitivity
    let dF_dm = vec![0.0001; n];
    let single_result = result.ift_sensitivity(&dF_dm).unwrap();

    // Same as batch with 1 column
    let dF_dm_batch = DMatrix::from_column_slice(n, 1, &dF_dm);
    let batch_result = result.ift_sensitivity_batch(&dF_dm_batch).unwrap();

    // Results should match
    for i in 0..n {
        assert_relative_eq!(single_result[i], batch_result[(i, 0)], epsilon = 1e-14);
    }
}

#[test]
fn test_ift_sensitivity_linearity() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let n = result.pillars.len();

    // dF1 and dF2
    let dF1: Vec<f64> = vec![0.0001; n];
    let dF2: Vec<f64> = (0..n).map(|i| 0.0002 * (i + 1) as f64).collect();

    // Combined: dF1 + dF2
    let dF_combined: Vec<f64> = dF1.iter().zip(&dF2).map(|(&a, &b)| a + b).collect();

    // Compute sensitivities
    let sens1 = result.ift_sensitivity(&dF1).unwrap();
    let sens2 = result.ift_sensitivity(&dF2).unwrap();
    let sens_combined = result.ift_sensitivity(&dF_combined).unwrap();

    // IFT should be linear: sens(dF1 + dF2) = sens(dF1) + sens(dF2)
    for i in 0..n {
        let expected = sens1[i] + sens2[i];
        assert_relative_eq!(sens_combined[i], expected, epsilon = 1e-12);
    }
}

#[test]
fn test_ift_sensitivity_zero_input() {
    let instruments = create_test_instruments();
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    let n = result.pillars.len();
    let dF_dm = vec![0.0; n];

    let sensitivity = result.ift_sensitivity(&dF_dm).unwrap();

    // Zero input should give zero output
    for &s in &sensitivity {
        assert_relative_eq!(s, 0.0, epsilon = 1e-15);
    }
}

// =============================================================================
// Integration tests (migrated from integration_tests/global_solver.rs)
// =============================================================================

#[test]
fn test_ois_curve_construction_basic() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.045),
    ];
    let result = assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-8);
    assert!(result.iterations < 20, "Should converge quickly");
    for i in 1..result.discount_factors.len() {
        assert!(result.discount_factors[i] < result.discount_factors[i - 1]);
    }
}

#[test]
fn test_ois_curve_construction_flat() {
    let rate = 0.03;
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, rate),
        MarketInstrument::ois(2.0, rate),
        MarketInstrument::ois(5.0, rate),
        MarketInstrument::ois(10.0, rate),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-10);
}

#[test]
fn test_ois_curve_construction_inverted() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.05),
        MarketInstrument::ois(2.0, 0.045),
        MarketInstrument::ois(5.0, 0.04),
        MarketInstrument::ois(10.0, 0.035),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-8);
}

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

    if let Some(errors) = &result.pricing_errors {
        for (i, error) in errors.iter().enumerate() {
            let e: f64 = *error;
            assert!(e.abs() < 1e-8, "Instrument {} has pricing error {}", i, e);
        }
    }
}

#[test]
fn test_mixed_instruments_convergence() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::fra(0.0, 0.5, 0.025),
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-7);
}

#[test]
fn test_fra_only_curve() {
    let instruments = vec![
        MarketInstrument::fra(0.0, 0.5, 0.025),
        MarketInstrument::fra(0.5, 1.0, 0.028),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-8);
}

#[test]
fn test_jacobian_finite_vs_central_difference() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let problem_fd = CalibrationProblem::with_config(
        instruments.clone(),
        crate::builder::CalibrationProblemConfig {
            jacobian_method: JacobianMethod::FiniteDifference,
            ..Default::default()
        },
    )
    .unwrap();

    let problem_cd = CalibrationProblem::with_config(
        instruments,
        crate::builder::CalibrationProblemConfig {
            jacobian_method: JacobianMethod::CentralDifference,
            ..Default::default()
        },
    )
    .unwrap();

    let initial = problem_fd.initial_guess();
    let j_fd = problem_fd.compute_jacobian_finite_diff(&initial).unwrap();
    let j_cd = problem_cd.compute_jacobian_central_diff(&initial).unwrap();

    for i in 0..3 {
        for j in 0..3 {
            assert_relative_eq!(j_fd[(i, j)], j_cd[(i, j)], epsilon = 1e-4);
        }
    }
}

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

    let problem = CalibrationProblem::new(instruments.clone()).unwrap();

    let log_df: Vec<f64> = result
        .discount_factors
        .iter()
        .map(|df: &f64| df.ln())
        .collect();

    let curve = problem.build_curve(&log_df).unwrap();
    let residuals = problem.compute_residuals(&curve).unwrap();

    for (i, r) in residuals.iter().enumerate() {
        assert!(r.abs() < 1e-8, "Residual {} = {} exceeds tolerance", i, r);
    }
}

#[test]
fn test_high_precision_calibration() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::high_precision(), 1e-12);
}

#[test]
fn test_fast_calibration() {
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];
    assert_calibration_converges(&instruments, GlobalBootstrapConfig::fast(), 1e-5);
}

#[test]
fn test_many_instruments() {
    let instruments: Vec<MarketInstrument<f64>> = (1..=20)
        .map(|i| {
            let t = i as f64;
            MarketInstrument::ois(t, 0.02 + 0.001 * t)
        })
        .collect();
    let result = assert_calibration_converges(&instruments, GlobalBootstrapConfig::default(), 1e-7);
    assert_eq!(result.pillars.len(), 20);
}

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

    let cond = result.condition_number.unwrap();
    assert!(cond > 0.0);
    assert!(cond < 1e12);
}

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

    let bump = 1e-6;

    for inst_idx in 0..instruments.len() {
        let mut bumped_instruments = instruments.clone();
        bumped_instruments[inst_idx] = MarketInstrument::ois(
            instruments[inst_idx].maturity(),
            instruments[inst_idx].rate() + bump,
        );

        let bumped_result = bootstrapper.calibrate(&bumped_instruments).unwrap();

        let bump_sensitivities: Vec<f64> = base_df
            .iter()
            .zip(bumped_result.discount_factors.iter())
            .map(|(&base, &bumped)| (bumped - base) / bump)
            .collect();

        let mut dF_dm = vec![0.0; instruments.len()];
        dF_dm[inst_idx] = -1.0;

        let ift_sensitivities = result.ift_sensitivity(&dF_dm).unwrap();

        for (i, (&ift_sens, &bump_sens)) in ift_sensitivities
            .iter()
            .zip(bump_sensitivities.iter())
            .enumerate()
        {
            let ift_df_sens = base_df[i] * ift_sens;

            if bump_sens.abs() > 1e-10 {
                let rel_error = ((ift_df_sens - bump_sens) / bump_sens).abs();
                assert!(
                    rel_error < 1e-4,
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

#[test]
fn test_enzyme_kernel_interpolation_accuracy() {
    use crate::{
        builder::enzyme_jacobian::kernels,
        market::curves::{BootstrapInterpolation, BootstrappedCurve, YieldCurve},
    };

    let pillar_times = vec![1.0, 2.0, 5.0, 10.0];
    let discount_factors: Vec<f64> = pillar_times.iter().map(|&t| (-0.03 * t).exp()).collect();
    let log_df: Vec<f64> = discount_factors.iter().map(|&df| df.ln()).collect();

    let curve = BootstrappedCurve::new(
        pillar_times.clone(),
        discount_factors.clone(),
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap();

    let test_times = vec![1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0];

    for t in test_times {
        let df_curve = curve.discount_factor(t).unwrap();
        let df_kernel = kernels::discount_factor_log_linear(t, &pillar_times, &log_df);

        assert_relative_eq!(df_kernel, df_curve, epsilon = 1e-12);
    }
}

#[test]
fn test_bootstrapped_curve_gradient_accuracy() {
    use crate::market::curves::{BootstrapInterpolation, BootstrappedCurve};

    let pillar_times = vec![1.0, 2.0, 5.0];
    let discount_factors: Vec<f64> = pillar_times.iter().map(|&t| (-0.03 * t).exp()).collect();

    let curve = BootstrappedCurve::new(
        pillar_times.clone(),
        discount_factors.clone(),
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap();

    let t = 1.5;
    let (df, gradient) = curve.discount_factor_with_gradient(t).unwrap();

    assert!(df > 0.0);
    assert!(df < 1.0);
    assert_eq!(gradient.len(), pillar_times.len());

    let bump = 1e-8;
    for i in 0..pillar_times.len() {
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

#[test]
fn test_bootstrapped_curve_log_gradient_accuracy() {
    use crate::market::curves::{BootstrapInterpolation, BootstrappedCurve};

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

    let t = 1.5;
    let (df, log_gradient) = curve.discount_factor_with_log_gradient(t).unwrap();

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

#[test]
fn test_enzyme_jacobian_stub_behavior() {
    use crate::builder::enzyme_jacobian::kernels;

    let pillar_times = vec![1.0, 2.0, 5.0];
    let log_df: Vec<f64> = pillar_times.iter().map(|&t| -0.03 * t).collect();

    let instrument_types = vec![0u32, 0, 0];
    let instrument_params = vec![vec![1.0, 0.03], vec![2.0, 0.035], vec![5.0, 0.04]];

    let jacobian = kernels::compute_jacobian_enzyme(
        &instrument_types,
        &instrument_params,
        &log_df,
        &pillar_times,
    );

    #[cfg(not(feature = "enzyme-ad"))]
    {
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(jacobian[(i, j)], 0.0, "Stub should return zero matrix");
            }
        }
    }

    #[cfg(feature = "enzyme-ad")]
    {
        assert_eq!(jacobian.nrows(), 3);
        assert_eq!(jacobian.ncols(), 3);
        let max_elem: f64 = jacobian.iter().map(|&x| x.abs()).fold(0.0, f64::max);
        assert!(max_elem > 0.0, "Enzyme Jacobian should be non-zero");
    }
}

#[test]
fn test_jacobian_method_consistency() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config_fd = GlobalBootstrapConfig::default();
    let bootstrapper_fd = GlobalBootstrapper::new(config_fd);
    let result_fd = bootstrapper_fd.calibrate(&instruments).unwrap();

    let config_cd = GlobalBootstrapConfig {
        jacobian_method: JacobianMethod::CentralDifference,
        ..Default::default()
    };
    let bootstrapper_cd = GlobalBootstrapper::new(config_cd);
    let result_cd = bootstrapper_cd.calibrate(&instruments).unwrap();

    assert!(result_fd.converged);
    assert!(result_cd.converged);

    for i in 0..result_fd.discount_factors.len() {
        assert_relative_eq!(
            result_fd.discount_factors[i],
            result_cd.discount_factors[i],
            epsilon = 1e-8
        );
    }
}

#[test]
fn test_jacobian_quality_validation_integration() {
    use crate::builder::{JacobianQuality, NumericalDiagnostics};

    let mut diagnostics = NumericalDiagnostics::<f64>::default();
    diagnostics.jacobian_quality = JacobianQuality::Good;
    diagnostics.residual_history.push(1e-3);
    diagnostics.residual_history.push(1e-10);

    assert_eq!(diagnostics.jacobian_quality, JacobianQuality::Good);
    assert_eq!(diagnostics.iteration_count(), 2);
    assert!(diagnostics.final_residual().is_some());
    assert!(!diagnostics.has_issues());
}

#[test]
fn test_max_condition_number_config_integration() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let config = GlobalBootstrapConfig {
        max_condition_number: 1e15,
        ..Default::default()
    };

    let bootstrapper = GlobalBootstrapper::new(config);
    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);

    if let Some(cond) = result.condition_number {
        assert!(cond > 0.0);
    }
}

#[test]
fn test_tikhonov_regularisation_integration() {
    use crate::builder::error::should_apply_regularisation;

    let high_cond: f64 = 1e14;
    let max_cond: f64 = 1e10;
    let damping = should_apply_regularisation(high_cond, max_cond);
    assert!(damping.is_some());
    assert!(damping.unwrap() > 0.0, "Damping should be positive");

    let low_cond: f64 = 1e6;
    let no_damping = should_apply_regularisation(low_cond, max_cond);
    assert!(
        no_damping.is_none(),
        "Should not apply regularisation for low condition number"
    );
}

#[test]
fn test_numerical_diagnostics_summary_integration() {
    use crate::builder::{JacobianQuality, NumericalDiagnostics, RegularisationType};

    let mut diagnostics = NumericalDiagnostics::<f64>::default();
    diagnostics.condition_number = Some(1e8);
    diagnostics.residual_history.push(1e-3);
    diagnostics.residual_history.push(1e-6);
    diagnostics.residual_history.push(1e-10);
    diagnostics.jacobian_quality = JacobianQuality::Good;
    diagnostics.regularisation_applied = RegularisationType::None;

    let summary = diagnostics.summary();

    assert!(summary.contains("Iterations: 3"));
    assert!(summary.contains("Condition:"));
    assert!(summary.contains("Quality:"));
}

#[test]
fn test_calibration_problem_variance_calculation() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let problem: CalibrationProblem<f64, _> = CalibrationProblem::new(instruments).unwrap();
    let x = problem.initial_guess();

    let jacobian_fd = problem.compute_jacobian_finite_diff(&x).unwrap();
    let jacobian_cd = problem.compute_jacobian_central_diff(&x).unwrap();

    let variance = problem.compute_jacobian_variance(&jacobian_fd, &jacobian_cd);

    assert!(
        variance < 1e-6,
        "FD and CD Jacobians should have low variance, got {}",
        variance
    );
}

#[test]
fn test_ad_fallback_decision_integration() {
    let instruments: Vec<MarketInstrument<f64>> = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ];

    let problem: CalibrationProblem<f64, _> = CalibrationProblem::new(instruments).unwrap();
    let x = problem.initial_guess();

    let jacobian1 = problem.compute_jacobian_finite_diff(&x).unwrap();
    let jacobian2 = problem.compute_jacobian_central_diff(&x).unwrap();

    let threshold = 1e6;
    let (should_fallback, variance) =
        problem.should_fallback_from_ad(&jacobian1, &jacobian2, threshold);

    assert!(
        !should_fallback,
        "Should not fallback for similar Jacobians"
    );
    assert!(variance < threshold);
}

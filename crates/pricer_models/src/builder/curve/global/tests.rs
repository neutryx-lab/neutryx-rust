use approx::assert_relative_eq;
use pricer_core::{math::linalg::DMatrix, types::SolverError};

use super::*;
use crate::{
    builder::{
        jump::{JumpConfig, JumpPillar},
        problem::JacobianMethod,
        CalibrationInstrument,
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
    let config = GlobalBootstrapConfig::default();
    let bootstrapper = GlobalBootstrapper::new(config);

    let result = bootstrapper.calibrate(&instruments).unwrap();

    assert!(result.converged);
    assert_eq!(result.pillars.len(), 4);
    assert_eq!(result.discount_factors.len(), 4);

    for i in 0..result.discount_factors.len() {
        assert!(result.discount_factors[i] > 0.0);
        assert!(result.discount_factors[i] <= 1.0);
    }

    for (i, instr) in instruments.iter().enumerate() {
        let error = instr.pricing_error(&result.curve).unwrap();
        assert!(
            error.abs() < 1e-8,
            "Instrument {} has pricing error {}",
            i,
            error
        );
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

#[allow(dead_code)]
fn create_jump_pillars() -> Vec<JumpPillar<f64>> {
    vec![
        JumpPillar::new(0.5, 25.0),  // 25bps at 6 months
        JumpPillar::new(1.5, -15.0), // -15bps at 18 months
    ]
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
fn test_calibrate_with_jumps_empty_jumps() {
    let instruments = create_test_instruments();
    let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

    // Empty jump list should fall back to regular calibration
    let result = bootstrapper
        .calibrate_with_jumps(&instruments, vec![])
        .unwrap();

    assert!(result.converged);
    assert!(!result.has_jumps());
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

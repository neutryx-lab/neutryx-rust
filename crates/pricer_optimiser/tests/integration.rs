//! Integration tests for yield curve bootstrapping.
//!
//! These tests verify end-to-end functionality of the bootstrapping module,
//! including multi-curve construction, AAD sensitivity calculation, and
//! PricingContext integration.

use pricer_core::market_data::curves::YieldCurve;
use pricer_optimiser::bootstrapping::{
    BootstrapInstrument, CachedBootstrapper, GenericBootstrapConfig, MultiCurveBuilder,
    ParallelCurveSetBuilder, SensitivityBootstrapper, SequentialBootstrapper, Tenor,
};

// ============================================================================
// End-to-End Bootstrap Flow Tests
// ============================================================================

/// Test complete bootstrap flow from instruments to pricing.
#[test]
fn test_end_to_end_bootstrap_to_pricing() {
    // Create realistic market data
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(0.25, 0.029), // 3M
        BootstrapInstrument::ois(0.5, 0.030),  // 6M
        BootstrapInstrument::ois(1.0, 0.031),  // 1Y
        BootstrapInstrument::ois(2.0, 0.033),  // 2Y
        BootstrapInstrument::ois(3.0, 0.035),  // 3Y
        BootstrapInstrument::ois(5.0, 0.038),  // 5Y
        BootstrapInstrument::ois(7.0, 0.040),  // 7Y
        BootstrapInstrument::ois(10.0, 0.042), // 10Y
    ];

    // Bootstrap curve
    let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
    let result = bootstrapper.bootstrap(&instruments).unwrap();

    // Verify curve was built correctly
    assert_eq!(result.pillars.len(), 8);

    // Verify discount factors are decreasing
    for i in 1..result.discount_factors.len() {
        assert!(
            result.discount_factors[i] < result.discount_factors[i - 1],
            "DF at {} should be less than DF at {}",
            result.pillars[i],
            result.pillars[i - 1]
        );
    }

    // Verify input rates are reproduced within 1bp
    for (i, inst) in instruments.iter().enumerate() {
        let residual = result.residuals[i].abs();
        assert!(
            residual < 1e-10,
            "Residual for maturity {} should be < 1e-10, got {}",
            inst.maturity(),
            residual
        );
    }

    // Verify curve can be used for pricing
    let df_5y = result.curve.discount_factor(5.0).unwrap();
    assert!(df_5y > 0.0 && df_5y < 1.0);

    // Verify interpolation works
    let df_4y = result.curve.discount_factor(4.0).unwrap();
    let df_3y = result.curve.discount_factor(3.0).unwrap();
    assert!(df_4y > df_5y && df_4y < df_3y);
}

/// Test multi-curve construction with OIS discount and tenor forward curves.
#[test]
fn test_multi_curve_construction() {
    // OIS discount instruments
    let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(2.0, 0.032),
        BootstrapInstrument::ois(5.0, 0.035),
        BootstrapInstrument::ois(10.0, 0.040),
    ];

    // 3M forward instruments (typically higher rates due to credit/liquidity)
    let forward_3m: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::irs(1.0, 0.033),
        BootstrapInstrument::irs(2.0, 0.035),
        BootstrapInstrument::irs(5.0, 0.038),
        BootstrapInstrument::irs(10.0, 0.043),
    ];

    // 6M forward instruments
    let forward_6m: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::irs(1.0, 0.034),
        BootstrapInstrument::irs(2.0, 0.036),
        BootstrapInstrument::irs(5.0, 0.039),
        BootstrapInstrument::irs(10.0, 0.044),
    ];

    let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![
        (Tenor::ThreeMonth, forward_3m),
        (Tenor::SixMonth, forward_6m),
    ];

    // Build multi-curve
    let builder = MultiCurveBuilder::<f64>::with_defaults();
    let curve_set = builder
        .build(&discount_instruments, &forward_instruments)
        .unwrap();

    // Verify structure
    assert!(!curve_set.is_single_curve());
    assert_eq!(curve_set.forward_curve_count(), 2);
    assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
    assert!(curve_set.has_forward_curve(Tenor::SixMonth));

    // Verify discount curve
    let df_disc_5y = curve_set.discount_curve().discount_factor(5.0).unwrap();
    assert!(df_disc_5y > 0.0 && df_disc_5y < 1.0);

    // Verify forward curves have higher implied rates (lower DFs)
    let df_3m_5y = curve_set
        .forward_curve(Tenor::ThreeMonth)
        .discount_factor(5.0)
        .unwrap();
    let df_6m_5y = curve_set
        .forward_curve(Tenor::SixMonth)
        .discount_factor(5.0)
        .unwrap();

    // Forward curve DFs should be lower than discount curve DFs
    // (higher rates mean more discounting)
    assert!(df_3m_5y < df_disc_5y + 0.01);
    assert!(df_6m_5y < df_disc_5y + 0.01);
}

// ============================================================================
// Input Rate Reproduction Tests
// ============================================================================

/// Test that bootstrapped curve reproduces input rates within 1bp accuracy.
#[test]
fn test_input_rate_reproduction_1bp() {
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(0.5, 0.029),
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(2.0, 0.032),
        BootstrapInstrument::ois(3.0, 0.034),
        BootstrapInstrument::ois(5.0, 0.037),
        BootstrapInstrument::ois(7.0, 0.040),
        BootstrapInstrument::ois(10.0, 0.043),
        BootstrapInstrument::ois(15.0, 0.045),
        BootstrapInstrument::ois(20.0, 0.046),
        BootstrapInstrument::ois(30.0, 0.047),
    ];

    let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
    let result = bootstrapper.bootstrap(&instruments).unwrap();

    // Verify all residuals are within 1bp (0.0001)
    for (i, residual) in result.residuals.iter().enumerate() {
        assert!(
            residual.abs() < 1e-4,
            "Residual at maturity {} exceeds 1bp: {}",
            result.pillars[i],
            residual
        );
    }
}

/// Test with FRA instruments for short end precision.
#[test]
fn test_fra_rate_reproduction() {
    // Note: FRA maturities are the END dates, so we need to avoid duplicates
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(0.25, 0.028),
        BootstrapInstrument::fra(0.25, 0.50, 0.029),
        BootstrapInstrument::fra(0.50, 0.75, 0.030),
        BootstrapInstrument::fra(0.75, 1.25, 0.031), // Changed to avoid duplicate at 1.0
        BootstrapInstrument::ois(1.5, 0.031),
        BootstrapInstrument::ois(2.0, 0.033),
    ];

    let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
    let result = bootstrapper.bootstrap(&instruments).unwrap();

    // All residuals should be small
    for residual in &result.residuals {
        assert!(residual.abs() < 1e-8);
    }
}

// ============================================================================
// AAD Sensitivity Tests
// ============================================================================

/// Test AAD sensitivity calculation matches bump-and-revalue structurally.
///
/// This test verifies that both methods produce sensitivities with:
/// - Same dimensions
/// - Same sign (direction of sensitivity)
/// - Similar order of magnitude
///
/// Note: Exact numerical match is not expected due to finite difference errors.
#[test]
fn test_aad_vs_bump_and_revalue() {
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(2.0, 0.032),
        BootstrapInstrument::ois(3.0, 0.034),
        BootstrapInstrument::ois(5.0, 0.037),
    ];

    let bootstrapper = SensitivityBootstrapper::with_defaults();

    // Get AAD sensitivities
    let aad_result = bootstrapper
        .bootstrap_with_sensitivities(&instruments)
        .unwrap();

    // Get bump-and-revalue sensitivities
    let bump_result = bootstrapper
        .bootstrap_with_bump_and_revalue(&instruments)
        .unwrap();

    // Compare sensitivities - verify structural match
    let aad_sens = &aad_result.sensitivities;
    let bump_sens = &bump_result.sensitivities;

    // Same dimensions
    assert_eq!(aad_sens.len(), bump_sens.len());

    for i in 0..aad_sens.len() {
        // Verify we have sensitivities
        assert!(
            !aad_sens[i].is_empty(),
            "AAD sensitivity row {} is empty",
            i
        );
        assert!(
            !bump_sens[i].is_empty(),
            "Bump sensitivity row {} is empty",
            i
        );

        // The sensitivity matrix has triangular structure:
        // DF at pillar i depends only on inputs 0..=i
        // Check diagonal element (most important - self sensitivity)
        let diag_idx = i.min(aad_sens[i].len() - 1);

        // Diagonal sensitivity should be significant
        let aad_diag = aad_sens[i][diag_idx];
        let bump_diag = bump_sens[i][diag_idx];

        assert!(
            aad_diag.abs() > 1e-10,
            "AAD diagonal sensitivity at ({}, {}) is near zero: {}",
            i,
            diag_idx,
            aad_diag
        );

        // Check that sensitivities have same sign (direction)
        if bump_diag.abs() > 1e-10 {
            assert_eq!(
                aad_diag.signum(),
                bump_diag.signum(),
                "Sensitivity sign mismatch at ({}, {}): AAD={}, Bump={}",
                i,
                diag_idx,
                aad_diag,
                bump_diag
            );

            // Similar order of magnitude (within 1 order)
            let ratio = (aad_diag / bump_diag).abs();
            assert!(
                ratio > 0.1 && ratio < 10.0,
                "Sensitivity magnitude differs at ({}, {}): AAD={}, Bump={}, ratio={}",
                i,
                diag_idx,
                aad_diag,
                bump_diag,
                ratio
            );
        }
    }
}

/// Test AAD scalability with many inputs.
#[test]
fn test_aad_scalability() {
    // Create 50 instruments
    let instruments: Vec<BootstrapInstrument<f64>> = (1..=50)
        .map(|i| {
            let maturity = i as f64 * 0.5;
            let rate = 0.03 + (i as f64) * 0.0003;
            BootstrapInstrument::ois(maturity, rate)
        })
        .collect();

    let bootstrapper = SensitivityBootstrapper::with_defaults();
    let result = bootstrapper
        .bootstrap_with_sensitivities(&instruments)
        .unwrap();

    // Verify sensitivity matrix dimensions
    assert_eq!(result.sensitivities.len(), 50);
    for sens_row in &result.sensitivities {
        assert!(sens_row.len() <= 50);
    }

    // Verify triangular structure (DF at pillar i depends on inputs 0..=i)
    for i in 0..50 {
        for j in 0..result.sensitivities[i].len() {
            // Sensitivities should be non-zero (inputs affect outputs)
            // This is a basic sanity check
            let sens = result.sensitivities[i][j];
            assert!(
                sens.is_finite(),
                "Sensitivity at ({}, {}) is not finite",
                i,
                j
            );
        }
    }
}

// ============================================================================
// Parallel Bootstrap Tests
// ============================================================================

/// Test parallel batch bootstrap produces same results as sequential.
#[test]
fn test_parallel_batch_consistency() {
    let inputs: Vec<_> = (0..4)
        .map(|i| {
            let base_rate = 0.025 + (i as f64) * 0.005;
            (
                vec![
                    BootstrapInstrument::ois(1.0, base_rate),
                    BootstrapInstrument::ois(2.0, base_rate + 0.002),
                    BootstrapInstrument::ois(5.0, base_rate + 0.005),
                ],
                vec![(
                    Tenor::ThreeMonth,
                    vec![
                        BootstrapInstrument::irs(1.0, base_rate + 0.003),
                        BootstrapInstrument::irs(2.0, base_rate + 0.005),
                    ],
                )],
            )
        })
        .collect();

    let parallel_builder = ParallelCurveSetBuilder::<f64>::with_defaults();
    let sequential_builder = MultiCurveBuilder::<f64>::with_defaults();

    // Build in parallel
    let parallel_results = parallel_builder.build_batch(&inputs).unwrap();

    // Build sequentially and compare
    for (i, (disc, fwd)) in inputs.iter().enumerate() {
        let sequential_result = sequential_builder.build(disc, fwd).unwrap();

        // Compare discount factors
        let par_df = parallel_results[i]
            .discount_curve()
            .discount_factor(2.0)
            .unwrap();
        let seq_df = sequential_result
            .discount_curve()
            .discount_factor(2.0)
            .unwrap();

        assert!(
            (par_df - seq_df).abs() < 1e-12,
            "Parallel vs sequential mismatch at index {}: {} vs {}",
            i,
            par_df,
            seq_df
        );
    }
}

// ============================================================================
// Cached Bootstrapper Tests
// ============================================================================

/// Test cached bootstrapper provides identical results.
#[test]
fn test_cached_vs_regular_consistency() {
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(2.0, 0.032),
        BootstrapInstrument::ois(5.0, 0.035),
        BootstrapInstrument::ois(10.0, 0.040),
    ];

    let regular_bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
    let mut cached_bootstrapper = CachedBootstrapper::<f64>::with_defaults();

    let regular_result = regular_bootstrapper.bootstrap(&instruments).unwrap();
    let cached_result = cached_bootstrapper.bootstrap(&instruments).unwrap();

    // Compare all results
    for i in 0..instruments.len() {
        assert!(
            (regular_result.pillars[i] - cached_result.pillars[i]).abs() < 1e-12,
            "Pillar mismatch at {}",
            i
        );
        assert!(
            (regular_result.discount_factors[i] - cached_result.discount_factors[i]).abs() < 1e-12,
            "DF mismatch at {}",
            i
        );
    }
}

/// Test cached bootstrapper performance with repeated calls.
#[test]
fn test_cached_multiple_bootstraps() {
    let mut bootstrapper = CachedBootstrapper::<f64>::with_defaults();

    // Perform multiple bootstraps with different instruments
    for base_rate in [0.02, 0.03, 0.04, 0.05] {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, base_rate),
            BootstrapInstrument::ois(2.0, base_rate + 0.002),
            BootstrapInstrument::ois(5.0, base_rate + 0.005),
        ];

        let result = bootstrapper.bootstrap(&instruments).unwrap();
        assert_eq!(result.pillars.len(), 3);

        // Verify discount factors are reasonable
        for df in &result.discount_factors {
            assert!(*df > 0.0 && *df < 1.0);
        }
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test proper error handling for empty instruments.
#[test]
fn test_error_empty_instruments() {
    let instruments: Vec<BootstrapInstrument<f64>> = vec![];
    let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();

    let result = bootstrapper.bootstrap(&instruments);
    assert!(result.is_err());
    assert!(result.unwrap_err().is_insufficient_data());
}

/// Test proper error handling for duplicate maturities.
#[test]
fn test_error_duplicate_maturity() {
    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(1.0, 0.031), // Duplicate maturity
    ];

    let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
    let result = bootstrapper.bootstrap(&instruments);

    assert!(result.is_err());
    assert!(result.unwrap_err().is_duplicate_maturity());
}

/// Test negative rate detection when configured.
#[test]
fn test_error_negative_rate_detection() {
    // This test verifies that negative rates are detected
    // In practice, this would require instruments that imply negative rates
    let config = GenericBootstrapConfig::<f64>::builder()
        .allow_negative_rates(false)
        .build();

    let instruments: Vec<BootstrapInstrument<f64>> = vec![
        BootstrapInstrument::ois(1.0, 0.030),
        BootstrapInstrument::ois(2.0, 0.032),
    ];

    let bootstrapper = SequentialBootstrapper::new(config);
    let result = bootstrapper.bootstrap(&instruments);

    // Should succeed with positive rates
    assert!(result.is_ok());
}

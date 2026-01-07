//! End-to-end tests for path-dependent option pricing.
//!
//! These tests verify complete pricing workflows for all path-dependent
//! option types with checkpoint support and L1/L2 integration.
//!
//! # Test Coverage
//!
//! - All PathPayoffType variants
//! - Checkpoint-enabled pricing
//! - Greeks computation (Delta via finite difference)
//! - L1/L2 crate integration (pricer_core, pricer_models)

use pricer_pricing::checkpoint::CheckpointStrategy;
use pricer_pricing::mc::pricer_checkpoint::{CheckpointPricer, CheckpointPricingConfig};
use pricer_pricing::mc::{GbmParams, MonteCarloConfig};
use pricer_pricing::path_dependent::PathPayoffType;

/// Standard test GBM parameters.
fn standard_gbm() -> GbmParams {
    GbmParams {
        spot: 100.0,
        rate: 0.05,
        volatility: 0.2,
        maturity: 1.0,
    }
}

/// Standard checkpoint pricing configuration.
fn standard_config() -> CheckpointPricingConfig {
    let mc_config = MonteCarloConfig::builder()
        .n_paths(10_000)
        .n_steps(100)
        .seed(42)
        .build()
        .unwrap();

    CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 20 })
}

// ============================================================================
// Asian Option E2E Tests
// ============================================================================

#[test]
fn e2e_asian_arithmetic_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // ATM Asian call should have positive price
    assert!(
        result.price > 0.0,
        "Price should be positive: {}",
        result.price
    );
    // Asian options have lower value than vanilla due to averaging
    assert!(
        result.price < 15.0,
        "Price should be reasonable: {}",
        result.price
    );
    // Standard error should be reasonable
    assert!(result.std_error > 0.0 && result.std_error < 1.0);
}

#[test]
fn e2e_asian_arithmetic_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_arithmetic_put(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(
        result.price > 0.0,
        "Put price should be positive: {}",
        result.price
    );
    assert!(result.price < 15.0);
}

#[test]
fn e2e_asian_geometric_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Geometric average is lower than arithmetic, so price should be lower
    assert!(result.price > 0.0);
    assert!(result.price < 10.0);
}

#[test]
fn e2e_asian_geometric_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_geometric_put(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(result.price > 0.0);
    assert!(result.price < 10.0);
}

// ============================================================================
// Barrier Option E2E Tests
// ============================================================================

#[test]
fn e2e_barrier_up_out_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::barrier_up_out_call(100.0, 130.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Up-out call should be cheaper than vanilla due to knock-out
    assert!(
        result.price >= 0.0,
        "Price should be non-negative: {}",
        result.price
    );
    assert!(result.price < 15.0);
}

#[test]
fn e2e_barrier_up_in_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::barrier_up_in_call(100.0, 130.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(result.price >= 0.0);
}

#[test]
fn e2e_barrier_down_out_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::barrier_down_out_put(100.0, 80.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(result.price >= 0.0);
    assert!(result.price < 15.0);
}

#[test]
fn e2e_barrier_down_in_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::barrier_down_in_put(100.0, 80.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(result.price >= 0.0);
}

// ============================================================================
// Lookback Option E2E Tests
// ============================================================================

#[test]
fn e2e_lookback_fixed_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Lookback uses max price, so value can be significant
    assert!(result.price > 0.0);
}

#[test]
fn e2e_lookback_fixed_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::lookback_fixed_put(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    assert!(result.price > 0.0);
}

#[test]
fn e2e_lookback_floating_call() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::lookback_floating_call(1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Floating strike: S_T - S_min
    assert!(result.price > 0.0);
}

#[test]
fn e2e_lookback_floating_put() {
    let config = standard_config();
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::lookback_floating_put(1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Floating strike: S_max - S_T
    assert!(result.price > 0.0);
}

// ============================================================================
// Checkpoint Integration E2E Tests
// ============================================================================

#[test]
fn e2e_all_checkpoint_strategies() {
    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    let mc_config = MonteCarloConfig::builder()
        .n_paths(5_000)
        .n_steps(100)
        .seed(42)
        .build()
        .unwrap();

    // Test with different checkpoint strategies
    let strategies = [
        CheckpointStrategy::None,
        CheckpointStrategy::Uniform { interval: 10 },
        CheckpointStrategy::Uniform { interval: 25 },
        CheckpointStrategy::Logarithmic { base_interval: 5 },
    ];

    let mut prices = Vec::new();

    for strategy in &strategies {
        let config = CheckpointPricingConfig::new(mc_config.clone(), *strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);
        prices.push(result.price);
    }

    // All strategies should produce the same price (within floating point tolerance)
    let reference = prices[0];
    for (i, &price) in prices.iter().enumerate().skip(1) {
        assert!(
            (price - reference).abs() < 1e-10,
            "Strategy {} produced different price: {} vs {}",
            i,
            price,
            reference
        );
    }
}

#[test]
fn e2e_checkpoint_memory_tracking() {
    let mc_config = MonteCarloConfig::builder()
        .n_paths(5_000)
        .n_steps(100)
        .seed(42)
        .build()
        .unwrap();

    let config =
        CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 10 });
    let mut pricer = CheckpointPricer::new(config).unwrap();

    let gbm = standard_gbm();
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
    let df = 0.95;

    // Before pricing, no checkpoints
    assert_eq!(pricer.checkpoint_count(), 0);
    assert_eq!(pricer.checkpoint_memory_usage(), 0);

    // After pricing, checkpoints should be created
    let _ = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // With interval=10 and n_steps=100, should have ~10 checkpoints
    assert!(pricer.checkpoint_count() > 0);
    assert!(pricer.checkpoint_memory_usage() > 0);
}

// ============================================================================
// Greeks E2E Tests (Finite Difference Delta)
// ============================================================================

#[test]
fn e2e_delta_asian_call_finite_difference() {
    let mc_config = MonteCarloConfig::builder()
        .n_paths(10_000)
        .n_steps(50)
        .seed(42)
        .build()
        .unwrap();

    let config = CheckpointPricingConfig::new(
        mc_config.clone(),
        CheckpointStrategy::Uniform { interval: 10 },
    );

    let base_gbm = standard_gbm();
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    // Price at spot
    let mut pricer = CheckpointPricer::new(config.clone()).unwrap();
    let price_base = pricer.price_path_dependent_with_checkpoints(base_gbm, payoff, df);

    // Price at spot + bump
    let bump = 0.01;
    let gbm_up = GbmParams {
        spot: base_gbm.spot * (1.0 + bump),
        ..base_gbm
    };
    let mut pricer_up = CheckpointPricer::new(config.clone()).unwrap();
    let price_up = pricer_up.price_path_dependent_with_checkpoints(gbm_up, payoff, df);

    // Price at spot - bump
    let gbm_down = GbmParams {
        spot: base_gbm.spot * (1.0 - bump),
        ..base_gbm
    };
    let mut pricer_down = CheckpointPricer::new(config).unwrap();
    let price_down = pricer_down.price_path_dependent_with_checkpoints(gbm_down, payoff, df);

    // Central difference delta
    let delta = (price_up.price - price_down.price) / (2.0 * bump * base_gbm.spot);

    // Delta for ATM Asian call should be positive and less than 1
    assert!(delta > 0.0, "Delta should be positive: {}", delta);
    assert!(delta < 1.0, "Delta should be less than 1: {}", delta);
}

// ============================================================================
// All Option Types E2E Test
// ============================================================================

#[test]
fn e2e_all_option_types_complete() {
    let mc_config = MonteCarloConfig::builder()
        .n_paths(5_000)
        .n_steps(50)
        .seed(42)
        .build()
        .unwrap();

    let config =
        CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 10 });

    let gbm = standard_gbm();
    let df = (-0.05_f64 * 1.0).exp();

    // All path-dependent option types
    let payoffs: Vec<(&str, PathPayoffType<f64>)> = vec![
        (
            "Asian Arithmetic Call",
            PathPayoffType::asian_arithmetic_call(100.0, 1e-6),
        ),
        (
            "Asian Arithmetic Put",
            PathPayoffType::asian_arithmetic_put(100.0, 1e-6),
        ),
        (
            "Asian Geometric Call",
            PathPayoffType::asian_geometric_call(100.0, 1e-6),
        ),
        (
            "Asian Geometric Put",
            PathPayoffType::asian_geometric_put(100.0, 1e-6),
        ),
        (
            "Barrier Up-Out Call",
            PathPayoffType::barrier_up_out_call(100.0, 130.0, 1e-6),
        ),
        (
            "Barrier Up-In Call",
            PathPayoffType::barrier_up_in_call(100.0, 130.0, 1e-6),
        ),
        (
            "Barrier Down-Out Put",
            PathPayoffType::barrier_down_out_put(100.0, 80.0, 1e-6),
        ),
        (
            "Barrier Down-In Put",
            PathPayoffType::barrier_down_in_put(100.0, 80.0, 1e-6),
        ),
        (
            "Lookback Fixed Call",
            PathPayoffType::lookback_fixed_call(100.0, 1e-6),
        ),
        (
            "Lookback Fixed Put",
            PathPayoffType::lookback_fixed_put(100.0, 1e-6),
        ),
        (
            "Lookback Floating Call",
            PathPayoffType::lookback_floating_call(1e-6),
        ),
        (
            "Lookback Floating Put",
            PathPayoffType::lookback_floating_put(1e-6),
        ),
    ];

    for (name, payoff) in payoffs {
        let mut pricer = CheckpointPricer::new(config.clone()).unwrap();
        let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

        // All prices should be non-negative
        assert!(
            result.price >= 0.0,
            "{}: Price should be non-negative: {}",
            name,
            result.price
        );

        // All should have positive standard error
        assert!(
            result.std_error > 0.0,
            "{}: Std error should be positive: {}",
            name,
            result.std_error
        );

        // Checkpoints should be created
        assert!(
            pricer.checkpoint_count() > 0,
            "{}: Checkpoints should be created",
            name
        );
    }
}

// ============================================================================
// L1/L2 Integration Verification
// ============================================================================

#[test]
fn e2e_l1_l2_integration_verification() {
    // This test verifies that pricer_pricing works correctly with L1/L2 crates
    // by testing the full pricing pipeline

    let mc_config = MonteCarloConfig::builder()
        .n_paths(10_000)
        .n_steps(100)
        .seed(42)
        .build()
        .unwrap();

    let config =
        CheckpointPricingConfig::new(mc_config, CheckpointStrategy::Uniform { interval: 20 });

    let mut pricer = CheckpointPricer::new(config).unwrap();

    // Test GBM parameters (could come from pricer_models)
    let gbm = GbmParams {
        spot: 100.0,
        rate: 0.05,
        volatility: 0.2,
        maturity: 1.0,
    };

    // Test payoff (uses smooth functions from pricer_core)
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);

    // Discount factor (could come from pricer_core YieldCurve)
    let df = (-0.05_f64 * 1.0).exp();

    // Price
    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);

    // Verify result structure
    assert!(result.price > 0.0);
    assert!(result.std_error > 0.0);

    // Verify checkpoint integration
    assert!(pricer.checkpoint_count() > 0);
    assert!(pricer.checkpoint_memory_usage() > 0);

    // Test equivalence (same result regardless of checkpointing)
    let is_equivalent = pricer.verify_checkpoint_equivalence(gbm, payoff, df, 1e-10);
    assert!(
        is_equivalent,
        "Checkpoint and non-checkpoint results should match"
    );
}

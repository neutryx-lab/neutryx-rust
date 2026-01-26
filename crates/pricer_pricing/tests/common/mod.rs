//! Common test fixtures and utilities for pricer_pricing tests.
//!
//! This module provides shared test utilities to reduce code duplication
//! across integration tests.

use pricer_pricing::mc::GbmParams;

/// Standard GBM parameters for option pricing tests.
///
/// Returns consistent parameters used across multiple test files:
/// - spot: 100.0
/// - rate: 0.05 (5%)
/// - volatility: 0.2 (20%)
/// - maturity: 1.0 (1 year)
#[allow(dead_code)]
pub fn standard_gbm() -> GbmParams {
    GbmParams {
        spot: 100.0,
        rate: 0.05,
        volatility: 0.2,
        maturity: 1.0,
    }
}

/// Standard test parameters for analytical comparison.
///
/// Returns tuple of (spot, strike, rate, dividend, volatility, maturity).
#[allow(dead_code)]
pub fn standard_params() -> (f64, f64, f64, f64, f64, f64) {
    (100.0, 100.0, 0.05, 0.0, 0.2, 1.0)
}

/// Standard Monte Carlo configuration for tests.
///
/// Returns a configuration with:
/// - 10,000 paths (balance between speed and accuracy)
/// - 252 steps (daily observations)
/// - Fixed seed for reproducibility
#[allow(dead_code)]
pub fn standard_mc_config() -> pricer_pricing::mc::MonteCarloConfig {
    pricer_pricing::mc::MonteCarloConfig::builder()
        .n_paths(10_000)
        .n_steps(252)
        .seed(42)
        .build()
        .unwrap()
}

/// High-accuracy Monte Carlo configuration for convergence tests.
///
/// Returns a configuration with:
/// - 100,000 paths (higher accuracy)
/// - 252 steps (daily observations)
/// - Fixed seed for reproducibility
#[allow(dead_code)]
pub fn high_accuracy_mc_config() -> pricer_pricing::mc::MonteCarloConfig {
    pricer_pricing::mc::MonteCarloConfig::builder()
        .n_paths(100_000)
        .n_steps(252)
        .seed(42)
        .build()
        .unwrap()
}

/// Calculate discount factor for given rate and maturity.
#[allow(dead_code)]
pub fn discount_factor(rate: f64, maturity: f64) -> f64 {
    (-rate * maturity).exp()
}

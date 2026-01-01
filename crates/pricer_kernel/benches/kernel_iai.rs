//! Iai-callgrind benchmarks for pricer_kernel Monte Carlo pricing.
//!
//! These benchmarks measure instruction counts for deterministic CI regression detection.
//! Unlike Criterion (wall-clock time), iai-callgrind provides reproducible metrics
//! independent of system load.
//!
//! # Requirements
//!
//! - Linux with Valgrind installed (`apt install valgrind`)
//! - iai-callgrind-runner (`cargo install iai-callgrind-runner`)
//!
//! # Usage
//!
//! ```bash
//! # Run benchmarks (Linux only)
//! cargo bench --bench kernel_iai
//!
//! # Compare with baseline
//! cargo bench --bench kernel_iai -- --save-baseline=main
//! cargo bench --bench kernel_iai -- --baseline=main
//! ```
//!
//! # Benchmark Coverage
//!
//! - GBM path generation (core MC kernel)
//! - European option pricing
//! - Greeks computation (Delta via forward AD)
//! - RNG generation
//! - Workspace operations

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use pricer_kernel::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams};
use pricer_kernel::rng::PricerRng;
use std::hint::black_box;

// =============================================================================
// RNG Benchmarks
// =============================================================================

#[library_benchmark]
fn rng_single_normal() -> f64 {
    let mut rng = PricerRng::from_seed(42);
    black_box(rng.gen_normal())
}

#[library_benchmark]
fn rng_batch_normal_1k() -> f64 {
    let mut rng = PricerRng::from_seed(42);
    let mut buffer = vec![0.0; 1_000];
    rng.fill_normal(&mut buffer);
    black_box(buffer.iter().sum())
}

// =============================================================================
// Monte Carlo Pricing Benchmarks
// =============================================================================

fn setup_pricer(n_paths: usize, n_steps: usize) -> MonteCarloPricer {
    let config = MonteCarloConfig::builder()
        .n_paths(n_paths)
        .n_steps(n_steps)
        .seed(42)
        .build()
        .unwrap();
    MonteCarloPricer::new(config).unwrap()
}

#[library_benchmark]
fn mc_european_call_1k() -> f64 {
    let mut pricer = setup_pricer(1_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_european(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result.price)
}

#[library_benchmark]
fn mc_european_call_10k() -> f64 {
    let mut pricer = setup_pricer(10_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_european(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result.price)
}

#[library_benchmark]
fn mc_european_put_10k() -> f64 {
    let mut pricer = setup_pricer(10_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::put(100.0);
    let result = pricer.price_european(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result.price)
}

// =============================================================================
// Greeks Benchmarks
// =============================================================================

#[library_benchmark]
fn greeks_delta_ad_1k() -> (f64, f64) {
    let mut pricer = setup_pricer(1_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_with_delta_ad(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result)
}

#[library_benchmark]
fn greeks_delta_ad_10k() -> (f64, f64) {
    let mut pricer = setup_pricer(10_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_with_delta_ad(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result)
}

// =============================================================================
// Workspace Benchmarks
// =============================================================================

#[library_benchmark]
fn workspace_creation_10k() -> MonteCarloPricer {
    black_box(setup_pricer(10_000, 50))
}

#[library_benchmark]
fn workspace_reuse() -> f64 {
    let mut pricer = setup_pricer(10_000, 50);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let df = 0.95;

    // 5 consecutive pricing calls reusing workspace
    let mut total = 0.0;
    for _ in 0..5 {
        let result = pricer.price_european(gbm, payoff, df);
        total += result.price;
    }
    black_box(total)
}

// =============================================================================
// Time Steps Scaling Benchmarks
// =============================================================================

#[library_benchmark]
fn mc_steps_10() -> f64 {
    let mut pricer = setup_pricer(10_000, 10);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_european(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result.price)
}

#[library_benchmark]
fn mc_steps_252() -> f64 {
    let mut pricer = setup_pricer(10_000, 252);
    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let result = pricer.price_european(black_box(gbm), black_box(payoff), black_box(0.95));
    black_box(result.price)
}

// =============================================================================
// Benchmark Groups
// =============================================================================

library_benchmark_group!(
    name = rng_benchmarks;
    benchmarks = rng_single_normal, rng_batch_normal_1k
);

library_benchmark_group!(
    name = mc_pricing_benchmarks;
    benchmarks = mc_european_call_1k, mc_european_call_10k, mc_european_put_10k
);

library_benchmark_group!(
    name = greeks_benchmarks;
    benchmarks = greeks_delta_ad_1k, greeks_delta_ad_10k
);

library_benchmark_group!(
    name = workspace_benchmarks;
    benchmarks = workspace_creation_10k, workspace_reuse
);

library_benchmark_group!(
    name = steps_scaling_benchmarks;
    benchmarks = mc_steps_10, mc_steps_252
);

main!(
    library_benchmark_groups = rng_benchmarks,
    mc_pricing_benchmarks,
    greeks_benchmarks,
    workspace_benchmarks,
    steps_scaling_benchmarks
);

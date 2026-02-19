//! Criterion benchmarks for pricer_pricing Monte Carlo pricing.
//!
//! Benchmarks cover:
//! - Monte Carlo path generation (1K, 10K, 100K paths)
//! - European option pricing with varying path counts
//! - Greeks computation (Delta via bump-and-revalue / forward AD)
//! - RNG performance

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_core::math::rng::PricerRng;
use pricer_pricing::{
    checkpoint::CheckpointStrategy,
    mc::{
        pricer_checkpoint::{CheckpointPricer, CheckpointPricingConfig},
        thread_local::{current_thread_index, ParallelWorkspaces},
        GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams,
    },
    payoff::PayoffKind,
    tree::{BinomialTree, TrinomialTree},
};
use rayon::prelude::*;

/// Benchmark RNG generation (foundation for MC simulations).
fn bench_rng_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rng_generation");

    for n_samples in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("normal_samples", n_samples),
            &n_samples,
            |b, &n| {
                let mut rng = PricerRng::from_seed(42);
                b.iter(|| {
                    let mut sum = 0.0;
                    for _ in 0..n {
                        sum += rng.gen_normal();
                    }
                    black_box(sum)
                });
            },
        );
    }

    // Batch generation (more efficient)
    for n_samples in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("normal_batch", n_samples),
            &n_samples,
            |b, &n| {
                let mut rng = PricerRng::from_seed(42);
                let mut buffer = vec![0.0; n];
                b.iter(|| {
                    rng.fill_normal(&mut buffer);
                    black_box(buffer.iter().sum::<f64>())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Monte Carlo pricing with varying path counts.
fn bench_mc_pricing(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_pricing");
    group.sample_size(50); // Reduce sample size for slower benchmarks

    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let discount_factor = 0.95;
    let n_steps = 50; // Fixed time steps for pricing benchmark

    // Benchmark different path counts
    for n_paths in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("european_call", n_paths),
            &n_paths,
            |b, &n| {
                let config = MonteCarloConfig::builder()
                    .n_paths(n)
                    .n_steps(n_steps)
                    .seed(42)
                    .build()
                    .unwrap();
                let mut pricer = MonteCarloPricer::new(config).unwrap();
                b.iter(|| {
                    pricer.price_european(
                        black_box(gbm),
                        black_box(payoff),
                        black_box(discount_factor),
                    )
                });
            },
        );
    }

    // Benchmark put option
    let put_payoff = PayoffParams::put(100.0);
    group.bench_with_input(
        BenchmarkId::new("european_put", 10_000),
        &10_000,
        |b, &n| {
            let config = MonteCarloConfig::builder()
                .n_paths(n)
                .n_steps(n_steps)
                .seed(42)
                .build()
                .unwrap();
            let mut pricer = MonteCarloPricer::new(config).unwrap();
            b.iter(|| {
                pricer.price_european(
                    black_box(gbm),
                    black_box(put_payoff),
                    black_box(discount_factor),
                )
            });
        },
    );

    group.finish();
}

/// Benchmark pricing with varying time steps.
fn bench_mc_steps_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_steps_scaling");
    group.sample_size(50);

    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let discount_factor = 0.95;
    let n_paths = 10_000; // Fixed path count

    // Benchmark different step counts
    for n_steps in [10, 50, 252, 500] {
        group.bench_with_input(
            BenchmarkId::new("european_call", n_steps),
            &n_steps,
            |b, &steps| {
                let config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(steps)
                    .seed(42)
                    .build()
                    .unwrap();
                let mut pricer = MonteCarloPricer::new(config).unwrap();
                b.iter(|| {
                    pricer.price_european(
                        black_box(gbm),
                        black_box(payoff),
                        black_box(discount_factor),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Greeks computation.
fn bench_greeks(c: &mut Criterion) {
    let mut group = c.benchmark_group("greeks");
    group.sample_size(30); // Greeks are slower to compute

    let gbm = GbmParams::default();
    let payoff = PayoffParams::call(100.0);
    let discount_factor = 0.95;
    let n_steps = 50;

    // Benchmark Delta computation (bump-and-revalue or forward AD)
    for n_paths in [1_000, 10_000] {
        group.bench_with_input(BenchmarkId::new("delta", n_paths), &n_paths, |b, &n| {
            let config = MonteCarloConfig::builder()
                .n_paths(n)
                .n_steps(n_steps)
                .seed(42)
                .build()
                .unwrap();
            let mut pricer = MonteCarloPricer::new(config).unwrap();
            b.iter(|| {
                pricer.price_with_delta_ad(
                    black_box(gbm),
                    black_box(payoff),
                    black_box(discount_factor),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark workspace allocation overhead.
fn bench_workspace_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_allocation");

    // Benchmark pricer creation (includes workspace allocation)
    for n_paths in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("pricer_creation", n_paths),
            &n_paths,
            |b, &n| {
                let config = MonteCarloConfig::builder()
                    .n_paths(n)
                    .n_steps(50)
                    .seed(42)
                    .build()
                    .unwrap();
                b.iter(|| black_box(MonteCarloPricer::new(config.clone()).unwrap()));
            },
        );
    }

    // Benchmark workspace reuse (multiple pricing calls with same pricer)
    group.bench_function("reuse_vs_recreate", |b| {
        let config = MonteCarloConfig::builder()
            .n_paths(10_000)
            .n_steps(50)
            .seed(42)
            .build()
            .unwrap();
        let mut pricer = MonteCarloPricer::new(config).unwrap();
        let gbm = GbmParams::default();
        let payoff = PayoffParams::call(100.0);
        let discount_factor = 0.95;

        b.iter(|| {
            // Multiple pricing calls reusing workspace
            for _ in 0..10 {
                black_box(pricer.price_european(gbm, payoff, discount_factor));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Checkpoint Overhead Benchmarks (Task 13.1)
// ============================================================================

/// Benchmark checkpoint time overhead with different intervals.
///
/// Measures computation time for path-dependent option pricing with:
/// - No checkpoints (baseline)
/// - Uniform checkpoints at intervals: 5, 10, 20, 50, 100
///
/// Target: Checkpoint overhead should be within 2x of baseline.
fn bench_checkpoint_time_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_time_overhead");
    group.sample_size(30);

    let n_paths = 10_000;
    let n_steps = 252; // 1 year of daily observations
    let gbm = GbmParams::default();
    let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    // Baseline: No checkpoints
    group.bench_function("no_checkpoint", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let config = CheckpointPricingConfig::new(mc_config, CheckpointStrategy::None);
        let mut pricer = CheckpointPricer::new(config).unwrap();

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Uniform checkpoint intervals
    for interval in [5, 10, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("uniform_interval", interval),
            &interval,
            |b, &interval| {
                let mc_config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(n_steps)
                    .seed(42)
                    .build()
                    .unwrap();
                let strategy = CheckpointStrategy::Uniform { interval };
                let config = CheckpointPricingConfig::new(mc_config, strategy);
                let mut pricer = CheckpointPricer::new(config).unwrap();

                b.iter(|| {
                    pricer.reset_with_seed(42);
                    black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
                });
            },
        );
    }

    // Logarithmic checkpoint strategy
    group.bench_function("logarithmic", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Logarithmic { base_interval: 5 };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    group.finish();
}

/// Benchmark checkpoint memory usage with different intervals.
///
/// This benchmark prints memory usage metrics rather than timing.
/// Use `--nocapture` to see output.
fn bench_checkpoint_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_memory_usage");
    group.sample_size(10);

    let n_paths = 10_000;
    let n_steps = 252;
    let gbm = GbmParams::default();
    let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    // Test memory usage for different intervals
    for interval in [5, 10, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("interval", interval),
            &interval,
            |b, &interval| {
                let mc_config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(n_steps)
                    .seed(42)
                    .build()
                    .unwrap();
                let strategy = CheckpointStrategy::Uniform { interval };
                let config = CheckpointPricingConfig::new(mc_config, strategy);
                let mut pricer = CheckpointPricer::new(config).unwrap();

                b.iter(|| {
                    pricer.reset_with_seed(42);
                    let result = pricer.price_path_dependent_with_checkpoints(gbm, payoff, df);
                    // Memory usage can be checked after run
                    let mem = pricer.checkpoint_memory_usage();
                    black_box((result, mem))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark path-dependent option types with checkpoints.
///
/// Compares performance across different payoff types.
fn bench_checkpoint_payoff_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_payoff_types");
    group.sample_size(20);

    let n_paths = 10_000;
    let n_steps = 100;
    let gbm = GbmParams::default();
    let df = (-0.05_f64 * 1.0).exp();
    let interval = 20;

    // Asian Arithmetic Call
    group.bench_function("asian_arithmetic", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Uniform { interval };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Asian Geometric Call
    group.bench_function("asian_geometric", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Uniform { interval };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_geometric_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Barrier Up-Out Call
    group.bench_function("barrier_up_out", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Uniform { interval };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::barrier_up_out_call(100.0, 150.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Fixed Call
    group.bench_function("lookback_fixed", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Uniform { interval };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_fixed_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Floating Call
    group.bench_function("lookback_floating", |b| {
        let mc_config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .seed(42)
            .build()
            .unwrap();
        let strategy = CheckpointStrategy::Uniform { interval };
        let config = CheckpointPricingConfig::new(mc_config, strategy);
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_floating_call(1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    group.finish();
}

// ============================================================================
// Scaling Benchmarks (Task 13.2)
// ============================================================================

/// Benchmark path count scaling for path-dependent options.
///
/// Measures how computation time scales with increasing path count.
/// Expected: O(n) scaling where n = n_paths.
fn bench_path_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_scaling");
    group.sample_size(20);

    let n_steps = 100;
    let gbm = GbmParams::default();
    let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    // Test path count scaling
    for n_paths in [1_000, 5_000, 10_000, 50_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("asian_arithmetic", n_paths),
            &n_paths,
            |b, &n_paths| {
                let mc_config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(n_steps)
                    .seed(42)
                    .build()
                    .unwrap();
                let config = CheckpointPricingConfig::new(
                    mc_config,
                    CheckpointStrategy::Uniform { interval: 20 },
                );
                let mut pricer = CheckpointPricer::new(config).unwrap();

                b.iter(|| {
                    pricer.reset_with_seed(42);
                    black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark time step scaling for path-dependent options.
///
/// Measures how computation time scales with increasing time steps.
/// Expected: O(n) scaling where n = n_steps.
fn bench_step_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("step_scaling");
    group.sample_size(20);

    let n_paths = 10_000;
    let gbm = GbmParams::default();
    let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);
    let df = (-0.05_f64 * 1.0).exp();

    // Test step count scaling
    for n_steps in [50, 100, 252, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("asian_arithmetic", n_steps),
            &n_steps,
            |b, &n_steps| {
                let mc_config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(n_steps)
                    .seed(42)
                    .build()
                    .unwrap();
                // Checkpoint interval proportional to steps
                let interval = (n_steps / 10).max(5);
                let config = CheckpointPricingConfig::new(
                    mc_config,
                    CheckpointStrategy::Uniform { interval },
                );
                let mut pricer = CheckpointPricer::new(config).unwrap();

                b.iter(|| {
                    pricer.reset_with_seed(42);
                    black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark path-dependent option type comparison.
///
/// Compares relative performance of different option types.
fn bench_payoff_type_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("payoff_type_comparison");
    group.sample_size(30);

    let n_paths = 10_000;
    let n_steps = 100;
    let gbm = GbmParams::default();
    let df = (-0.05_f64 * 1.0).exp();

    let mc_config = MonteCarloConfig::builder()
        .n_paths(n_paths)
        .n_steps(n_steps)
        .seed(42)
        .build()
        .unwrap();

    // Asian Arithmetic Call
    group.bench_function("asian_arithmetic_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_arithmetic_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Asian Arithmetic Put
    group.bench_function("asian_arithmetic_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_arithmetic_put(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Asian Geometric Call
    group.bench_function("asian_geometric_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_geometric_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Asian Geometric Put
    group.bench_function("asian_geometric_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::asian_geometric_put(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Barrier Up-Out Call
    group.bench_function("barrier_up_out_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::barrier_up_out_call(100.0, 150.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Barrier Down-Out Put
    group.bench_function("barrier_down_out_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::barrier_down_out_put(100.0, 80.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Barrier Up-In Call
    group.bench_function("barrier_up_in_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::barrier_up_in_call(100.0, 150.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Barrier Down-In Put
    group.bench_function("barrier_down_in_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::barrier_down_in_put(100.0, 80.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Fixed Call
    group.bench_function("lookback_fixed_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_fixed_call(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Fixed Put
    group.bench_function("lookback_fixed_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_fixed_put(100.0, 1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Floating Call
    group.bench_function("lookback_floating_call", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_floating_call(1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    // Lookback Floating Put
    group.bench_function("lookback_floating_put", |b| {
        let config = CheckpointPricingConfig::new(
            mc_config.clone(),
            CheckpointStrategy::Uniform { interval: 20 },
        );
        let mut pricer = CheckpointPricer::new(config).unwrap();
        let payoff = PayoffKind::lookback_floating_put(1e-6);

        b.iter(|| {
            pricer.reset_with_seed(42);
            black_box(pricer.price_path_dependent_with_checkpoints(gbm, payoff, df))
        });
    });

    group.finish();
}

// ============================================================================
// Parallel Scalability Benchmarks (Task 13.3)
// ============================================================================

/// Benchmark parallel workspace access pattern.
///
/// Tests thread-local workspace pattern with Rayon parallel iteration.
fn bench_parallel_workspace_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_workspace");
    group.sample_size(30);

    let n_paths = 10_000;
    let n_steps = 100;
    let n_threads = rayon::current_num_threads().max(2);

    // Serial access pattern
    group.bench_function("serial", |b| {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(1, n_paths, n_steps);
        let values: Vec<f64> = (0..n_paths).map(|i| i as f64).collect();

        b.iter(|| {
            let mut sum = 0.0_f64;
            for (i, &v) in values.iter().enumerate() {
                workspaces.with_workspace(0, |ws| {
                    ws.observer_mut(i % ws.capacity_paths()).observe(v);
                    sum += v;
                });
            }
            black_box(sum)
        });
    });

    // Parallel access pattern with thread-local workspaces
    group.bench_function("parallel_rayon", |b| {
        let workspaces: ParallelWorkspaces<f64> =
            ParallelWorkspaces::new(n_threads, n_paths, n_steps);

        b.iter(|| {
            let sum: f64 = (0..n_paths)
                .into_par_iter()
                .map(|i| {
                    let thread_idx = current_thread_index() % n_threads;
                    workspaces.with_workspace(thread_idx, |ws| {
                        let path_idx = i % ws.capacity_paths();
                        ws.observer_mut(path_idx).observe(i as f64);
                        i as f64
                    })
                })
                .sum();
            black_box(sum)
        });
    });

    group.finish();
}

/// Benchmark parallel path simulation scalability.
///
/// Compares simulation time with increasing thread counts.
fn bench_parallel_path_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_simulation");
    group.sample_size(20);

    let n_paths = 100_000;
    let n_steps = 50;
    let n_threads = rayon::current_num_threads().max(2);

    // Serial simulation (all paths on single thread)
    group.bench_function("serial", |b| {
        let workspaces: ParallelWorkspaces<f64> = ParallelWorkspaces::new(1, n_paths, n_steps);

        b.iter(|| {
            // Simulate path observations serially
            for path_idx in 0..n_paths {
                workspaces.with_workspace(0, |ws| {
                    let obs_idx = path_idx % ws.capacity_paths();
                    for step in 0..n_steps {
                        ws.observer_mut(obs_idx).observe((path_idx + step) as f64);
                    }
                });
            }
            black_box(())
        });
    });

    // Parallel simulation (paths distributed across threads)
    group.bench_function("parallel", |b| {
        let workspaces: ParallelWorkspaces<f64> =
            ParallelWorkspaces::new(n_threads, n_paths / n_threads + 1, n_steps);

        b.iter(|| {
            (0..n_paths).into_par_iter().for_each(|path_idx| {
                let thread_idx = current_thread_index() % n_threads;
                workspaces.with_workspace(thread_idx, |ws| {
                    let obs_idx = path_idx % ws.capacity_paths();
                    for step in 0..n_steps {
                        ws.observer_mut(obs_idx).observe((path_idx + step) as f64);
                    }
                });
            });
            black_box(())
        });
    });

    group.finish();
}

/// Benchmark thread count scalability.
///
/// Measures how performance scales with number of threads.
fn bench_thread_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scalability");
    group.sample_size(20);

    let n_paths = 50_000;
    let n_steps = 50;
    let max_threads = rayon::current_num_threads();

    // Test with different thread pool sizes
    for n_threads in [1, 2, 4, 8].iter().filter(|&&t| t <= max_threads) {
        group.bench_with_input(
            BenchmarkId::new("threads", n_threads),
            n_threads,
            |b, &n_threads| {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(n_threads)
                    .build()
                    .unwrap();

                let workspaces: ParallelWorkspaces<f64> =
                    ParallelWorkspaces::new(n_threads, n_paths / n_threads + 1, n_steps);

                b.iter(|| {
                    pool.install(|| {
                        (0..n_paths).into_par_iter().for_each(|path_idx| {
                            let thread_idx = current_thread_index() % n_threads;
                            workspaces.with_workspace(thread_idx, |ws| {
                                let obs_idx = path_idx % ws.capacity_paths();
                                ws.observer_mut(obs_idx).observe(path_idx as f64);
                            });
                        });
                    });
                    black_box(())
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Graph Extraction Benchmarks (Task 2.3)
// ============================================================================

use pricer_pricing::graph::{GraphExtractable, SimpleGraphExtractor};

/// Benchmark graph extraction with varying trade counts.
///
/// Measures time to extract computation graphs from pricing contexts.
/// Target: 10,000 nodes in < 1 second.
fn bench_graph_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_extraction");
    group.sample_size(50);

    // Benchmark single trade extraction
    for n_params in [3, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("single_trade_params", n_params),
            &n_params,
            |b, &n| {
                let mut extractor = SimpleGraphExtractor::new();
                let params: Vec<String> = (0..n).map(|i| format!("param_{}", i)).collect();
                extractor.register_trade("T001", params);

                b.iter(|| black_box(extractor.extract_graph(Some("T001")).unwrap()));
            },
        );
    }

    // Benchmark multi-trade extraction
    for n_trades in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("multi_trade", n_trades),
            &n_trades,
            |b, &n| {
                let mut extractor = SimpleGraphExtractor::new()
                    .with_timeout(5000)
                    .with_capacity(10_000, 20_000);

                for i in 0..n {
                    let trade_id = format!("T{:04}", i);
                    let params: Vec<String> = (0..5).map(|j| format!("param_{}", j)).collect();
                    extractor.register_trade(&trade_id, params);
                }

                b.iter(|| black_box(extractor.extract_graph(None).unwrap()));
            },
        );
    }

    group.finish();
}

/// Benchmark GraphBuilder construction.
///
/// Measures graph building overhead with pre-allocated buffers.
fn bench_graph_builder(c: &mut Criterion) {
    use pricer_pricing::graph::{GraphBuilder, GraphEdge, GraphNode, NodeGroup, NodeType};

    let mut group = c.benchmark_group("graph_builder");
    group.sample_size(50);

    // Benchmark node addition
    for n_nodes in [100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::new("add_nodes", n_nodes), &n_nodes, |b, &n| {
            b.iter(|| {
                let mut builder = GraphBuilder::with_capacity(n, n * 2);
                for i in 0..n {
                    builder.add_node(GraphNode {
                        id: format!("N{}", i),
                        node_type: NodeType::Add,
                        label: format!("node_{}", i),
                        value: Some(i as f64),
                        is_sensitivity_target: i < 10,
                        group: NodeGroup::Intermediate,
                        trade_ids: vec![],
                    });
                }
                black_box(builder.node_count())
            });
        });
    }

    // Benchmark edge addition and depth calculation
    for n_nodes in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("build_graph", n_nodes),
            &n_nodes,
            |b, &n| {
                b.iter(|| {
                    let mut builder = GraphBuilder::with_capacity(n, n);

                    // Create a linear chain graph
                    for i in 0..n {
                        builder.add_node(GraphNode {
                            id: format!("N{}", i),
                            node_type: if i == 0 {
                                NodeType::Input
                            } else if i == n - 1 {
                                NodeType::Output
                            } else {
                                NodeType::Add
                            },
                            label: format!("n{}", i),
                            value: Some(i as f64),
                            is_sensitivity_target: i == 0,
                            group: if i == 0 {
                                NodeGroup::Input
                            } else if i == n - 1 {
                                NodeGroup::Output
                            } else {
                                NodeGroup::Intermediate
                            },
                            trade_ids: vec![],
                        });

                        if i > 0 {
                            builder.add_edge(GraphEdge {
                                source: format!("N{}", i - 1),
                                target: format!("N{}", i),
                                weight: None,
                            });
                        }
                    }

                    let graph = builder.build(Some("T001".to_string()));
                    black_box(graph.metadata.depth)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Tree Pricing Benchmarks (Task 12.1)
// ============================================================================

/// Benchmark Binomial Tree pricing with different step counts.
///
/// Performance targets:
/// - 100 steps: < 1ms
/// - 5000 steps: < 500ms
fn bench_binomial_tree_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group("binomial_tree_steps");
    group.sample_size(100);

    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;

    // Test different step counts
    for num_steps in [100, 200, 500, 1000, 2000, 5000] {
        group.bench_with_input(
            BenchmarkId::new("european_call", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    BinomialTree::new(spot, strike, expiry, rate, volatility, steps, true, false)
                        .unwrap();
                b.iter(|| black_box(tree.price()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("american_put", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    BinomialTree::new(spot, strike, expiry, rate, volatility, steps, false, true)
                        .unwrap();
                b.iter(|| black_box(tree.price()));
            },
        );
    }

    group.finish();
}

/// Benchmark Binomial Tree Greeks computation.
fn bench_binomial_tree_greeks(c: &mut Criterion) {
    let mut group = c.benchmark_group("binomial_tree_greeks");
    group.sample_size(100);

    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;

    for num_steps in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("delta", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    BinomialTree::new(spot, strike, expiry, rate, volatility, steps, true, false)
                        .unwrap();
                b.iter(|| black_box(tree.delta()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("gamma", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    BinomialTree::new(spot, strike, expiry, rate, volatility, steps, true, false)
                        .unwrap();
                b.iter(|| black_box(tree.gamma()));
            },
        );
    }

    group.finish();
}

/// Benchmark Trinomial Tree pricing.
fn bench_trinomial_tree_steps(c: &mut Criterion) {
    let mut group = c.benchmark_group("trinomial_tree_steps");
    group.sample_size(100);

    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;

    // Trinomial converges faster, so use fewer steps
    for num_steps in [50, 100, 200, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("european_call", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    TrinomialTree::new(spot, strike, expiry, rate, volatility, steps, true, false)
                        .unwrap();
                b.iter(|| black_box(tree.price()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("american_put", num_steps),
            &num_steps,
            |b, &steps| {
                let tree =
                    TrinomialTree::new(spot, strike, expiry, rate, volatility, steps, false, true)
                        .unwrap();
                b.iter(|| black_box(tree.price()));
            },
        );
    }

    group.finish();
}

/// Benchmark Binomial vs Trinomial comparison.
fn bench_tree_type_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_type_comparison");
    group.sample_size(100);

    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;
    let num_steps = 200;

    // Binomial European call
    group.bench_function("binomial_european_call", |b| {
        let tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, false,
        )
        .unwrap();
        b.iter(|| black_box(tree.price()));
    });

    // Trinomial European call (same steps)
    group.bench_function("trinomial_european_call", |b| {
        let tree = TrinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, true, false,
        )
        .unwrap();
        b.iter(|| black_box(tree.price()));
    });

    // Binomial American put
    group.bench_function("binomial_american_put", |b| {
        let tree = BinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, false, true,
        )
        .unwrap();
        b.iter(|| black_box(tree.price()));
    });

    // Trinomial American put
    group.bench_function("trinomial_american_put", |b| {
        let tree = TrinomialTree::new(
            spot, strike, expiry, rate, volatility, num_steps, false, true,
        )
        .unwrap();
        b.iter(|| black_box(tree.price()));
    });

    group.finish();
}

// ============================================================================
// Memory Layout Benchmarks (mc-memory-layout-optimisation)
// ============================================================================

use pricer_pricing::methods::mc::{
    layout_config::{PathLayout, PathLayoutConfig, StreamingConfig},
    workspace_enum::WorkspaceEnum,
    workspace_trait::PathWorkspaceTrait,
    ArithmeticAverageObserver, EuropeanObserver, LookbackObserver, StreamingEngine,
};

/// Benchmark PathFirst vs TimeStepFirst layout for path generation.
///
/// Measures the performance difference between traditional PathFirst layout
/// and optimised TimeStepFirst layout for different workload sizes.
fn bench_layout_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_layout");
    group.sample_size(50);

    let gbm = GbmParams::default();

    // Test different path × step combinations
    for (n_paths, n_steps) in [(10_000, 100), (50_000, 252), (100_000, 50)] {
        let label = format!("{}x{}", n_paths, n_steps);

        // PathFirst layout
        group.bench_with_input(
            BenchmarkId::new("path_first", &label),
            &(n_paths, n_steps),
            |b, &(paths, steps)| {
                let config = MonteCarloConfig::builder()
                    .n_paths(paths)
                    .n_steps(steps)
                    .layout(PathLayoutConfig::with_layout(PathLayout::PathFirst))
                    .seed(42)
                    .build()
                    .unwrap();
                let mut pricer = MonteCarloPricer::new(config).unwrap();
                let payoff = PayoffParams::call(100.0);
                let df = 0.95;

                b.iter(|| black_box(pricer.price_european(gbm, payoff, df)));
            },
        );

        // TimeStepFirst layout
        group.bench_with_input(
            BenchmarkId::new("timestep_first", &label),
            &(n_paths, n_steps),
            |b, &(paths, steps)| {
                let config = MonteCarloConfig::builder()
                    .n_paths(paths)
                    .n_steps(steps)
                    .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
                    .seed(42)
                    .build()
                    .unwrap();
                let mut pricer = MonteCarloPricer::new(config).unwrap();
                let payoff = PayoffParams::call(100.0);
                let df = 0.95;

                b.iter(|| black_box(pricer.price_european(gbm, payoff, df)));
            },
        );
    }

    group.finish();
}

/// Benchmark streaming mode vs batch mode memory efficiency.
///
/// Compares:
/// - Batch: O(paths × steps) memory
/// - Streaming: O(paths) memory
fn bench_streaming_vs_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_vs_batch");
    group.sample_size(30);

    let gbm = GbmParams::default();
    let df = 0.95;

    // Test with varying step counts to demonstrate memory advantage
    for n_steps in [100, 252, 500] {
        let n_paths = 50_000;

        // Batch mode (traditional)
        group.bench_with_input(BenchmarkId::new("batch", n_steps), &n_steps, |b, &steps| {
            let config = MonteCarloConfig::builder()
                .n_paths(n_paths)
                .n_steps(steps)
                .seed(42)
                .build()
                .unwrap();
            let mut pricer = MonteCarloPricer::new(config).unwrap();
            let payoff = PayoffParams::call(100.0);

            b.iter(|| black_box(pricer.price_european(gbm, payoff, df)));
        });

        // Streaming mode
        group.bench_with_input(
            BenchmarkId::new("streaming", n_steps),
            &n_steps,
            |b, &steps| {
                let config = MonteCarloConfig::builder()
                    .n_paths(n_paths)
                    .n_steps(steps)
                    .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
                    .streaming(StreamingConfig::enabled())
                    .seed(42)
                    .build()
                    .unwrap();
                let mut pricer = MonteCarloPricer::new(config).unwrap();
                let payoff = PayoffParams::call(100.0);

                b.iter(|| black_box(pricer.price_streaming(gbm, payoff, df)));
            },
        );
    }

    group.finish();
}

/// Benchmark streaming engine for path-dependent options.
///
/// Streaming is particularly efficient for options requiring path statistics.
fn bench_streaming_path_dependent(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_path_dependent");
    group.sample_size(30);

    let n_paths = 50_000;
    let n_steps = 252;
    let gbm = GbmParams::default();
    let df = 0.95;

    // Asian option (arithmetic average)
    group.bench_function("asian_call", |b| {
        let config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();
        let mut pricer = MonteCarloPricer::new(config).unwrap();

        b.iter(|| black_box(pricer.price_asian_streaming(gbm, 100.0, true, df)));
    });

    // Lookback option (floating strike)
    group.bench_function("lookback_floating_call", |b| {
        let config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();
        let mut pricer = MonteCarloPricer::new(config).unwrap();

        b.iter(|| black_box(pricer.price_lookback_streaming(gbm, None, true, true, df)));
    });

    // Barrier option (up-and-out)
    group.bench_function("barrier_up_out_call", |b| {
        let config = MonteCarloConfig::builder()
            .n_paths(n_paths)
            .n_steps(n_steps)
            .layout(PathLayoutConfig::with_layout(PathLayout::TimeStepFirst))
            .streaming(StreamingConfig::enabled())
            .seed(42)
            .build()
            .unwrap();
        let mut pricer = MonteCarloPricer::new(config).unwrap();

        b.iter(|| {
            black_box(pricer.price_barrier_streaming(gbm, 100.0, 150.0, true, true, true, df))
        });
    });

    group.finish();
}

/// Benchmark memory usage comparison.
///
/// Demonstrates memory footprint differences between layouts.
fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_footprint");
    group.sample_size(20);

    let n_paths = 100_000;
    let n_steps = 252;

    // Measure workspace allocation sizes
    group.bench_function("pathfirst_allocation", |b| {
        b.iter(|| {
            let workspace = WorkspaceEnum::new(PathLayout::PathFirst, n_paths, n_steps);
            black_box(workspace.paths().len())
        });
    });

    group.bench_function("timestepfirst_allocation", |b| {
        b.iter(|| {
            let workspace = WorkspaceEnum::new(PathLayout::TimeStepFirst, n_paths, n_steps);
            black_box(workspace.paths().len())
        });
    });

    // Streaming engine has constant memory regardless of steps
    group.bench_function("streaming_allocation", |b| {
        let streaming_config = StreamingConfig::enabled();
        b.iter(|| {
            let engine = StreamingEngine::new(n_paths, n_steps, streaming_config, 42);
            black_box(engine.memory_usage())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_rng_generation,
    bench_mc_pricing,
    bench_mc_steps_scaling,
    bench_greeks,
    bench_workspace_allocation,
    bench_checkpoint_time_overhead,
    bench_checkpoint_memory_usage,
    bench_checkpoint_payoff_types,
    bench_path_scaling,
    bench_step_scaling,
    bench_payoff_type_comparison,
    bench_parallel_workspace_access,
    bench_parallel_path_simulation,
    bench_thread_scalability,
    bench_graph_extraction,
    bench_graph_builder,
    bench_binomial_tree_steps,
    bench_binomial_tree_greeks,
    bench_trinomial_tree_steps,
    bench_tree_type_comparison,
    // Memory layout optimisation benchmarks
    bench_layout_comparison,
    bench_streaming_vs_batch,
    bench_streaming_path_dependent,
    bench_memory_footprint
);
criterion_main!(benches);

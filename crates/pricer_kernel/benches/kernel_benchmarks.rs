//! Criterion benchmarks for pricer_kernel Monte Carlo pricing.
//!
//! Benchmarks cover:
//! - Monte Carlo path generation (1K, 10K, 100K paths)
//! - European option pricing with varying path counts
//! - Greeks computation (Delta via bump-and-revalue / forward AD)
//! - RNG performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_kernel::checkpoint::CheckpointStrategy;
use pricer_kernel::mc::pricer_checkpoint::{CheckpointPricer, CheckpointPricingConfig};
use pricer_kernel::mc::thread_local::{current_thread_index, ParallelWorkspaces};
use pricer_kernel::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams};
use pricer_kernel::path_dependent::PathPayoffType;
use pricer_kernel::rng::PricerRng;
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
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
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
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
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
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::barrier_up_out_call(100.0, 150.0, 1e-6);

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
        let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::lookback_floating_call(1e-6);

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
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
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
    let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
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
        let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::asian_arithmetic_put(100.0, 1e-6);

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
        let payoff = PathPayoffType::asian_geometric_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::asian_geometric_put(100.0, 1e-6);

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
        let payoff = PathPayoffType::barrier_up_out_call(100.0, 150.0, 1e-6);

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
        let payoff = PathPayoffType::barrier_down_out_put(100.0, 80.0, 1e-6);

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
        let payoff = PathPayoffType::barrier_up_in_call(100.0, 150.0, 1e-6);

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
        let payoff = PathPayoffType::barrier_down_in_put(100.0, 80.0, 1e-6);

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
        let payoff = PathPayoffType::lookback_fixed_call(100.0, 1e-6);

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
        let payoff = PathPayoffType::lookback_fixed_put(100.0, 1e-6);

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
        let payoff = PathPayoffType::lookback_floating_call(1e-6);

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
        let payoff = PathPayoffType::lookback_floating_put(1e-6);

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
    bench_thread_scalability
);
criterion_main!(benches);

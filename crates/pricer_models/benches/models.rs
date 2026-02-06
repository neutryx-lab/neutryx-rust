//! Criterion benchmarks for pricer_models pricing computations.
//!
//! Benchmarks cover:
//! - Normal distribution functions (norm_cdf, norm_pdf) - core of Black-Scholes
//! - Payoff evaluation with smooth approximations
//! - GBM model step evolution

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use infra_domain::trade::{
    ExerciseStyle, InstrumentParams, PayoffType, PricingInstrument, VanillaOption,
};
use pricer_core::math::{
    distributions::{norm_cdf, norm_pdf},
    smoothing::smooth_max,
};
use pricer_models::stochastic::{GBMModel, GBMParams, StochasticModel};

/// Benchmark normal distribution CDF computation.
fn bench_norm_cdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("norm_cdf");

    // Single evaluation
    group.bench_function("single", |b| {
        b.iter(|| norm_cdf(black_box(0.5_f64)));
    });

    // Multiple evaluations (typical in option pricing - d1, d2 calculations)
    group.bench_function("batch_1000", |b| {
        let xs: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) / 100.0).collect();
        b.iter(|| {
            let mut sum = 0.0;
            for &x in &xs {
                sum += norm_cdf(black_box(x));
            }
            sum
        });
    });

    group.finish();
}

/// Benchmark normal distribution PDF computation.
fn bench_norm_pdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("norm_pdf");

    // Single evaluation
    group.bench_function("single", |b| {
        b.iter(|| norm_pdf(black_box(0.5_f64)));
    });

    // Multiple evaluations (used in Greek calculations)
    group.bench_function("batch_1000", |b| {
        let xs: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) / 100.0).collect();
        b.iter(|| {
            let mut sum = 0.0;
            for &x in &xs {
                sum += norm_pdf(black_box(x));
            }
            sum
        });
    });

    group.finish();
}

/// Evaluates a smooth payoff using smooth_max approximation.
///
/// Call: max(spot - strike, 0)
/// Put: max(strike - spot, 0)
#[inline]
fn evaluate_payoff(payoff: PayoffType, spot: f64, strike: f64, epsilon: f64) -> f64 {
    smooth_max(payoff.sign() * (spot - strike), 0.0, epsilon)
}

/// Benchmark payoff evaluation with smooth approximations.
fn bench_payoff_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("payoff_evaluation");

    // Call payoff evaluation
    group.bench_function("call_single", |b| {
        let payoff = PayoffType::Call;
        b.iter(|| {
            evaluate_payoff(
                payoff,
                black_box(110.0_f64),
                black_box(100.0),
                black_box(1e-6),
            )
        });
    });

    // Put payoff evaluation
    group.bench_function("put_single", |b| {
        let payoff = PayoffType::Put;
        b.iter(|| {
            evaluate_payoff(
                payoff,
                black_box(90.0_f64),
                black_box(100.0),
                black_box(1e-6),
            )
        });
    });

    // Batch evaluation (typical portfolio scenario)
    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("call_batch", size), &size, |b, &n| {
            let payoff = PayoffType::Call;
            let spots: Vec<f64> = (0..n)
                .map(|i| 80.0 + (i as f64 / n as f64) * 40.0)
                .collect();
            let strike = 100.0;
            let epsilon = 1e-6;
            b.iter(|| {
                let mut sum = 0.0;
                for &spot in &spots {
                    sum += evaluate_payoff(
                        payoff,
                        black_box(spot),
                        black_box(strike),
                        black_box(epsilon),
                    );
                }
                sum
            });
        });
    }

    group.finish();
}

/// Benchmark vanilla option via Instrument enum (static dispatch).
fn bench_instrument_payoff(c: &mut Criterion) {
    let mut group = c.benchmark_group("instrument_payoff");

    // Create instruments
    let call_params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    let call = VanillaOption::new(call_params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    let call_instrument = PricingInstrument::Vanilla(call);

    // Single instrument payoff
    group.bench_function("vanilla_call", |b| {
        b.iter(|| call_instrument.payoff(black_box(110.0)));
    });

    // Portfolio of instruments
    for size in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("portfolio", size), &size, |b, &n| {
            let instruments: Vec<PricingInstrument<f64>> = (0..n)
                .map(|i| {
                    let strike = 90.0 + (i as f64 / n as f64) * 20.0;
                    let params = InstrumentParams::new(strike, 1.0, 1.0).unwrap();
                    let option = VanillaOption::new(
                        params,
                        if i % 2 == 0 {
                            PayoffType::Call
                        } else {
                            PayoffType::Put
                        },
                        ExerciseStyle::European,
                        1e-6,
                    );
                    PricingInstrument::Vanilla(option)
                })
                .collect();
            let spot = 100.0;
            b.iter(|| {
                let mut sum = 0.0;
                for inst in &instruments {
                    sum += inst.payoff(black_box(spot));
                }
                sum
            });
        });
    }

    group.finish();
}

/// Benchmark GBM model step evolution.
fn bench_gbm_evolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("gbm_evolution");

    let params = GBMParams::new(100.0, 0.05, 0.2).unwrap();
    let dt = 1.0 / 252.0; // Daily step

    // Single step evolution
    group.bench_function("single_step", |b| {
        let state = GBMModel::<f64>::initial_state(&params);
        let dw = 0.1; // Sample Brownian increment
        b.iter(|| {
            GBMModel::<f64>::evolve_step(black_box(state), black_box(dt), &[black_box(dw)], &params)
        });
    });

    // Path simulation (252 steps = 1 year)
    group.bench_function("path_252_steps", |b| {
        let dws: Vec<f64> = (0..252).map(|i| ((i as f64 * 0.1).sin()) * 0.01).collect();
        b.iter(|| {
            let mut state = GBMModel::<f64>::initial_state(&params);
            for &dw in &dws {
                state = GBMModel::<f64>::evolve_step(state, dt, &[dw], &params);
            }
            state
        });
    });

    // Multiple paths
    for n_paths in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("multiple_paths_50_steps", n_paths),
            &n_paths,
            |b, &n| {
                let n_steps = 50;
                // Pre-generate random increments for all paths
                let dws: Vec<Vec<f64>> = (0..n)
                    .map(|p| {
                        (0..n_steps)
                            .map(|s| ((p * n_steps + s) as f64 * 0.01).sin() * 0.01)
                            .collect()
                    })
                    .collect();
                b.iter(|| {
                    let mut final_states = Vec::with_capacity(n);
                    for path_dws in &dws {
                        let mut state = GBMModel::<f64>::initial_state(&params);
                        for &dw in path_dws {
                            state = GBMModel::<f64>::evolve_step(state, dt, &[dw], &params);
                        }
                        final_states.push(state);
                    }
                    final_states
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_norm_cdf,
    bench_norm_pdf,
    bench_payoff_evaluation,
    bench_instrument_payoff,
    bench_gbm_evolution
);
criterion_main!(benches);

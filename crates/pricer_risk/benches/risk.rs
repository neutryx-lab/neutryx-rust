//! Criterion benchmarks for pricer_risk XVA and risk calculations.
//!
//! Benchmarks cover:
//! - Portfolio construction with varying trade counts
//! - CVA/DVA computation
//! - SoA data structure operations
//! - ImplicitSolver AAD curve sensitivity computation
//!
//! # Requirements Coverage
//!
//! - Requirement 3.4: Criterion format benchmark output
//! - Requirement 10: Performance requirements (AAD 5x speedup)

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use infra_master::trade::{
    ExerciseStyle, InstrumentParams, PayoffType, PricingInstrument, VanillaOption,
};
use infra_master::Currency;
use nalgebra::{DMatrix, DVector};
use pricer_risk::{
    compute_cva, compute_dva, generate_flat_discount_factors,
    greeks::ad::implicit_solver::ImplicitSolver,
    portfolio::{
        Counterparty, CounterpartyId, CreditParams, NettingSet, NettingSetId, PortfolioBuilder,
        Trade, TradeId,
    },
    OwnCreditParams,
};

/// Generate time grid for XVA calculations.
fn generate_time_grid(n_times: usize, maturity: f64) -> Vec<f64> {
    (0..n_times)
        .map(|i| maturity * i as f64 / (n_times - 1) as f64)
        .collect()
}

/// Generate synthetic expected exposure profile for benchmarking.
fn generate_expected_exposure(n_times: usize) -> Vec<f64> {
    (0..n_times)
        .map(|t| {
            let mid = n_times as f64 / 2.0;
            let t_frac = t as f64;
            100.0 * (1.0 - (t_frac - mid).abs() / mid)
        })
        .collect()
}

/// Benchmark CVA calculation.
fn bench_cva_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cva_calculation");

    let credit = CreditParams::new(0.02, 0.4).unwrap();

    for n_times in [50, 100, 252, 500] {
        let ee = generate_expected_exposure(n_times);
        let time_grid = generate_time_grid(n_times, 5.0);

        group.bench_function(format!("cva_{}", n_times), |b| {
            b.iter(|| {
                compute_cva(
                    black_box(&ee),
                    black_box(&time_grid),
                    black_box(&credit),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark DVA calculation.
fn bench_dva_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dva_calculation");

    let own_credit = OwnCreditParams::new(0.015, 0.35).unwrap();
    let time_grid = generate_time_grid(50, 5.0);

    // Generate negative exposure profile for DVA
    let ene: Vec<f64> = generate_expected_exposure(50)
        .into_iter()
        .map(|x| -x)
        .collect();

    group.bench_function("dva_50_times", |b| {
        b.iter(|| {
            compute_dva(
                black_box(&ene),
                black_box(&time_grid),
                black_box(&own_credit),
            )
        });
    });

    group.finish();
}

/// Benchmark portfolio construction with different sizes.
fn bench_portfolio_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("portfolio_construction");

    for n_trades in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("build", n_trades), &n_trades, |b, &n| {
            let credit = CreditParams::new(0.02, 0.4).unwrap();
            let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit);
            let netting_set =
                NettingSet::new(NettingSetId::new("NS001"), CounterpartyId::new("CP001"));

            let trades: Vec<Trade> = (0..n)
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

                    Trade::new(
                        TradeId::new(format!("T{:05}", i)),
                        PricingInstrument::Vanilla(option),
                        Currency::USD,
                        CounterpartyId::new("CP001"),
                        NettingSetId::new("NS001"),
                        1_000_000.0,
                    )
                })
                .collect();

            b.iter(|| {
                let mut builder = PortfolioBuilder::new()
                    .add_counterparty(counterparty.clone())
                    .add_netting_set(netting_set.clone());

                for trade in &trades {
                    builder = builder.add_trade(trade.clone());
                }

                black_box(builder.build().unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark discount factor generation.
fn bench_discount_factors(c: &mut Criterion) {
    let mut group = c.benchmark_group("discount_factors");

    for n_times in [50, 100, 252, 500] {
        let time_grid = generate_time_grid(n_times, 5.0);

        group.bench_with_input(
            BenchmarkId::new("flat_curve", n_times),
            &time_grid,
            |b, time_grid| {
                b.iter(|| generate_flat_discount_factors(black_box(0.03), black_box(time_grid)));
            },
        );
    }

    group.finish();
}

/// Benchmark ImplicitSolver curve sensitivity computation (Implicit Function Theorem).
///
/// Requirements Coverage:
/// - Requirement 10: AAD performance (5x speedup vs bump-and-revalue)
fn bench_implicit_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("implicit_solver");

    // Benchmark compute_curve_sensitivities with different matrix sizes
    for n in [5, 10, 30, 50] {
        // Create random Jacobian inverse and adjoint vector
        let j_inv = DMatrix::from_fn(n, n, |i, j| {
            if i == j {
                0.99 - 0.01 * (i as f64)
            } else {
                0.01 / ((i as f64 - j as f64).abs() + 1.0)
            }
        });
        let adjoint = DVector::from_fn(n, |i, _| 100.0 * (1.0 - i as f64 / n as f64));

        group.bench_with_input(
            BenchmarkId::new("implicit_function_theorem", n),
            &(j_inv.clone(), adjoint.clone()),
            |b, (j_inv, adjoint)| {
                b.iter(|| {
                    black_box(
                        ImplicitSolver::compute_curve_sensitivities(black_box(j_inv), black_box(adjoint))
                            .unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark ImplicitSolver finite difference fallback.
fn bench_implicit_solver_fd(c: &mut Criterion) {
    let mut group = c.benchmark_group("implicit_solver_fd");

    for n in [5, 10, 30] {
        // Simple quadratic loss function for testing
        let nodes = DVector::from_fn(n, |i, _| 0.03 + 0.002 * i as f64);

        group.bench_with_input(
            BenchmarkId::new("finite_difference", n),
            &nodes,
            |b, nodes| {
                // Loss function: L(x) = sum(x_i^2)
                let loss_fn = |x: &DVector<f64>| x.iter().map(|&v| v * v).sum();
                b.iter(|| {
                    black_box(ImplicitSolver::compute_curve_sensitivities_fd(
                        black_box(&loss_fn),
                        black_box(nodes),
                        black_box(1e-6),
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark ImplicitSolver vs finite difference comparison.
///
/// This demonstrates the speedup of using the Implicit Function Theorem
/// with pre-computed Jacobian inverse vs bump-and-revalue.
fn bench_implicit_vs_fd(c: &mut Criterion) {
    let mut group = c.benchmark_group("implicit_vs_fd_comparison");

    let n = 30; // Typical curve pillar count

    // Setup Jacobian inverse (from calibration)
    let j_inv = DMatrix::from_fn(n, n, |i, j| {
        if i == j {
            0.99 - 0.01 * (i as f64)
        } else {
            0.01 / ((i as f64 - j as f64).abs() + 1.0)
        }
    });
    let adjoint = DVector::from_fn(n, |i, _| 100.0 * (1.0 - i as f64 / n as f64));
    let nodes = DVector::from_fn(n, |i, _| 0.03 + 0.002 * i as f64);

    // Implicit Function Theorem approach
    group.bench_function("implicit_30_nodes", |b| {
        b.iter(|| {
            black_box(
                ImplicitSolver::compute_curve_sensitivities(black_box(&j_inv), black_box(&adjoint))
                    .unwrap(),
            )
        });
    });

    // Finite difference (bump-and-revalue) approach
    group.bench_function("finite_diff_30_nodes", |b| {
        let loss_fn = |x: &DVector<f64>| x.iter().map(|&v| v * v).sum();
        b.iter(|| {
            black_box(ImplicitSolver::compute_curve_sensitivities_fd(
                black_box(&loss_fn),
                black_box(&nodes),
                black_box(1e-6),
            ))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cva_calculation,
    bench_dva_calculation,
    bench_portfolio_construction,
    bench_discount_factors,
);

criterion_group!(
    implicit_solver_benches,
    bench_implicit_solver,
    bench_implicit_solver_fd,
    bench_implicit_vs_fd,
);

criterion_main!(benches, implicit_solver_benches);

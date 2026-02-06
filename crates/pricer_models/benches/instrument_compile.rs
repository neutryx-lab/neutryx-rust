//! Criterion benchmarks for instrument pre-compilation.
//!
//! This benchmark compares the performance of:
//! - Direct MarketInstrument pricing (with calendar/convention lookups each time)
//! - CompiledInstrument pricing (pre-computed cashflows)
//!
//! Target: 30% or more improvement in iteration-level pricing_error computation.
//!
//! Run with: `cargo bench --bench instrument_compile --features global-bootstrap`

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use pricer_models::builder::{
    CalibrationInstrument, CalibrationProblem, CompiledInstrument,
};
use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve, MarketInstrument};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create OIS instruments for benchmarking using pricer_models MarketInstrument.
fn create_pricer_models_instruments(n: usize) -> Vec<MarketInstrument<f64>> {
    let maturities = [
        0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 50.0,
    ];
    let base_rate = 0.03;

    maturities
        .iter()
        .take(n)
        .enumerate()
        .map(|(i, &t)| {
            let rate = base_rate + (i as f64) * 0.002;
            MarketInstrument::ois(t, rate)
        })
        .collect()
}

/// Create equivalent CompiledInstruments.
fn create_compiled_instruments(n: usize) -> Vec<CompiledInstrument<f64>> {
    let maturities = [
        0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 50.0,
    ];
    let base_rate = 0.03;

    maturities
        .iter()
        .take(n)
        .enumerate()
        .map(|(i, &t)| {
            let rate = base_rate + (i as f64) * 0.002;
            CompiledInstrument::deposit(rate, t).unwrap()
        })
        .collect()
}

/// Create a yield curve for pricing.
fn create_test_curve(pillars: &[f64]) -> BootstrappedCurve<f64> {
    let discount_factors: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
    BootstrappedCurve::new(
        pillars.to_vec(),
        discount_factors,
        BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap()
}

// ============================================================================
// Benchmark Group 1: Single Instrument Pricing Error
// ============================================================================

fn bench_single_pricing_error(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_pricing_error");

    let curve = create_test_curve(&[0.25, 0.5, 1.0, 2.0, 5.0, 10.0]);

    // Original MarketInstrument (pricer_models)
    let market_inst = MarketInstrument::<f64>::ois(5.0, 0.04);

    // Compiled instrument
    let compiled_inst = CompiledInstrument::<f64>::deposit(0.04, 5.0).unwrap();

    group.bench_function("market_instrument", |b| {
        b.iter(|| {
            let error = black_box(&market_inst).pricing_error(black_box(&curve)).unwrap();
            black_box(error)
        });
    });

    group.bench_function("compiled_instrument", |b| {
        b.iter(|| {
            let error = black_box(&compiled_inst)
                .pricing_error(black_box(&curve))
                .unwrap();
            black_box(error)
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 2: Batch Pricing Error (Calibration Iteration)
// ============================================================================

fn bench_batch_pricing_error(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_pricing_error");

    for n_instruments in [3, 6, 10, 12] {
        let pillars: Vec<f64> = [0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 50.0]
            .iter()
            .take(n_instruments)
            .copied()
            .collect();
        let curve = create_test_curve(&pillars);

        // Original MarketInstrument
        let market_instruments = create_pricer_models_instruments(n_instruments);

        // Compiled instruments
        let compiled_instruments = create_compiled_instruments(n_instruments);

        group.throughput(Throughput::Elements(n_instruments as u64));

        group.bench_with_input(
            BenchmarkId::new("market_instrument_batch", n_instruments),
            &market_instruments,
            |b, insts| {
                b.iter(|| {
                    let mut total_error = 0.0_f64;
                    for inst in black_box(insts) {
                        total_error += inst.pricing_error(black_box(&curve)).unwrap().abs();
                    }
                    black_box(total_error)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compiled_instrument_batch", n_instruments),
            &compiled_instruments,
            |b, insts| {
                b.iter(|| {
                    let mut total_error = 0.0_f64;
                    for inst in black_box(insts) {
                        total_error += inst.pricing_error(black_box(&curve)).unwrap().abs();
                    }
                    black_box(total_error)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 3: Full Calibration Problem Evaluation
// ============================================================================

fn bench_calibration_problem_evaluate(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration_problem_evaluate");

    for n_instruments in [6, 10, 12] {
        let market_instruments = create_pricer_models_instruments(n_instruments);
        let compiled_instruments = create_compiled_instruments(n_instruments);

        // Create CalibrationProblem with MarketInstrument<f64>
        let market_problem = CalibrationProblem::new(market_instruments.clone()).unwrap();

        // Create CalibrationProblem with CompiledInstrument<f64>
        let compiled_problem = CalibrationProblem::from_compiled(compiled_instruments).unwrap();

        let x_market = market_problem.initial_guess_vector();
        let x_compiled = compiled_problem.initial_guess_vector();

        group.throughput(Throughput::Elements(n_instruments as u64));

        group.bench_with_input(
            BenchmarkId::new("market_problem_evaluate", n_instruments),
            &(market_problem.clone(), x_market.clone()),
            |b, (problem, x)| {
                use pricer_core::math::solvers::SystemOfEquations;
                b.iter(|| {
                    let residuals = problem.evaluate(black_box(x)).unwrap();
                    black_box(residuals)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compiled_problem_evaluate", n_instruments),
            &(compiled_problem.clone(), x_compiled.clone()),
            |b, (problem, x)| {
                use pricer_core::math::solvers::SystemOfEquations;
                b.iter(|| {
                    let residuals = problem.evaluate(black_box(x)).unwrap();
                    black_box(residuals)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 4: Full Calibration Problem Jacobian
// ============================================================================

fn bench_calibration_problem_jacobian(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration_problem_jacobian");
    group.sample_size(50); // Fewer samples for expensive Jacobian computation

    for n_instruments in [6, 10] {
        let market_instruments = create_pricer_models_instruments(n_instruments);
        let compiled_instruments = create_compiled_instruments(n_instruments);

        let market_problem = CalibrationProblem::new(market_instruments).unwrap();
        let compiled_problem = CalibrationProblem::from_compiled(compiled_instruments).unwrap();

        let x_market = market_problem.initial_guess_vector();
        let x_compiled = compiled_problem.initial_guess_vector();

        group.throughput(Throughput::Elements((n_instruments * n_instruments) as u64));

        group.bench_with_input(
            BenchmarkId::new("market_problem_jacobian", n_instruments),
            &(market_problem, x_market),
            |b, (problem, x)| {
                use pricer_core::math::solvers::SystemOfEquations;
                b.iter(|| {
                    let jacobian = problem.jacobian(black_box(x)).unwrap();
                    black_box(jacobian)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compiled_problem_jacobian", n_instruments),
            &(compiled_problem, x_compiled),
            |b, (problem, x)| {
                use pricer_core::math::solvers::SystemOfEquations;
                b.iter(|| {
                    let jacobian = problem.jacobian(black_box(x)).unwrap();
                    black_box(jacobian)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 5: Compilation Overhead
// ============================================================================

fn bench_compilation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("compilation_overhead");

    // Measure how much time compilation takes vs total calibration benefit
    for n_instruments in [6, 10, 12] {
        let market_instruments = create_pricer_models_instruments(n_instruments);

        group.throughput(Throughput::Elements(n_instruments as u64));

        group.bench_with_input(
            BenchmarkId::new("create_compiled_instruments", n_instruments),
            &n_instruments,
            |b, &n| {
                b.iter(|| {
                    let insts = create_compiled_instruments(n);
                    black_box(insts)
                });
            },
        );

        // Compare: creating CalibrationProblem with market instruments
        group.bench_with_input(
            BenchmarkId::new("create_market_problem", n_instruments),
            &market_instruments,
            |b, insts| {
                b.iter(|| {
                    let problem = CalibrationProblem::new(black_box(insts.clone())).unwrap();
                    black_box(problem)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = single_benches;
    config = Criterion::default().significance_level(0.05).sample_size(100);
    targets = bench_single_pricing_error
);

criterion_group!(
    name = batch_benches;
    config = Criterion::default().significance_level(0.05).sample_size(100);
    targets = bench_batch_pricing_error
);

criterion_group!(
    name = problem_benches;
    config = Criterion::default().significance_level(0.05).sample_size(50);
    targets = bench_calibration_problem_evaluate
);

criterion_group!(
    name = jacobian_benches;
    config = Criterion::default().significance_level(0.1).sample_size(30);
    targets = bench_calibration_problem_jacobian
);

criterion_group!(
    name = overhead_benches;
    config = Criterion::default().significance_level(0.05).sample_size(50);
    targets = bench_compilation_overhead
);

criterion_main!(
    single_benches,
    batch_benches,
    problem_benches,
    jacobian_benches,
    overhead_benches
);

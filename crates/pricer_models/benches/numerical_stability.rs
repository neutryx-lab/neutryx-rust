//! Criterion benchmarks for numerical stability features.
//!
//! Benchmarks cover:
//! - Jacobian quality validation performance
//! - Condition number estimation performance
//! - AD variance calculation performance
//! - Tikhonov regularisation overhead
//!
//! Run with: `cargo bench --bench numerical_stability --features global-bootstrap`

#![cfg(feature = "global-bootstrap")]
#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pricer_core::math::linalg::DMatrix;
use pricer_models::{
    builder::{
        apply_tikhonov_regularisation, estimate_condition_number, should_apply_regularisation,
        validate_jacobian_dmatrix, CalibrationProblem, GlobalBootstrapConfig, GlobalBootstrapper,
        JacobianMethod,
    },
    market::curves::MarketInstrument,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test Jacobian matrices of various sizes.
fn create_test_jacobian(n: usize) -> DMatrix<f64> {
    let mut matrix = DMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            // Create a diagonally dominant matrix for numerical stability
            if i == j {
                matrix[(i, j)] = 1.0 + (i as f64) * 0.1;
            } else {
                let diff = if i > j { i - j } else { j - i };
                matrix[(i, j)] = 0.01 / (diff as f64 + 1.0);
            }
        }
    }
    matrix
}

/// Create OIS instruments for calibration benchmarks.
fn create_ois_instruments(n: usize) -> Vec<MarketInstrument<f64>> {
    (1..=n)
        .map(|i| {
            let maturity = i as f64;
            let rate = 0.03 + (i as f64) * 0.002;
            MarketInstrument::ois(maturity, rate)
        })
        .collect()
}

// ============================================================================
// Jacobian Quality Validation Benchmarks
// ============================================================================

fn bench_jacobian_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("jacobian_validation");

    for size in [3, 5, 10, 20, 50].iter() {
        let jacobian = create_test_jacobian(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("validate_dmatrix", size),
            &jacobian,
            |b, j| {
                b.iter(|| validate_jacobian_dmatrix(black_box(j), 1e-14))
            },
        );
    }

    group.finish();
}

// ============================================================================
// Condition Number Estimation Benchmarks
// ============================================================================

fn bench_condition_number(c: &mut Criterion) {
    let mut group = c.benchmark_group("condition_number");

    for size in [3, 5, 10, 20, 50].iter() {
        let jacobian = create_test_jacobian(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("estimate", size),
            &jacobian,
            |b, j| {
                b.iter(|| estimate_condition_number(black_box(j)))
            },
        );
    }

    group.finish();
}

// ============================================================================
// Tikhonov Regularisation Benchmarks
// ============================================================================

fn bench_tikhonov_regularisation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tikhonov_regularisation");

    for size in [3, 5, 10, 20, 50].iter() {
        let jacobian = create_test_jacobian(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("apply", size),
            &jacobian,
            |b, j: &DMatrix<f64>| {
                let mut matrix = j.clone();
                b.iter(|| {
                    apply_tikhonov_regularisation(black_box(&mut matrix), 0.01);
                    matrix = j.clone(); // Reset for next iteration
                })
            },
        );
    }

    // Benchmark should_apply_regularisation decision function
    group.bench_function("should_apply_decision", |b| {
        b.iter(|| {
            let high_cond = black_box(1e14_f64);
            let max_cond = black_box(1e10_f64);
            should_apply_regularisation(high_cond, max_cond)
        })
    });

    group.finish();
}

// ============================================================================
// AD Variance Calculation Benchmarks
// ============================================================================

fn bench_jacobian_variance(c: &mut Criterion) {
    let mut group = c.benchmark_group("jacobian_variance");

    for size in [3, 5, 10].iter() {
        let instruments = create_ois_instruments(*size);
        let problem: CalibrationProblem<f64, _> =
            CalibrationProblem::new(instruments).unwrap();
        let x = problem.initial_guess();

        // Pre-compute Jacobians
        let jacobian1 = problem.compute_jacobian_finite_diff(&x).unwrap();
        let jacobian2 = problem.compute_jacobian_central_diff(&x).unwrap();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("compute_variance", size),
            &(&problem, &jacobian1, &jacobian2),
            |b, (prob, j1, j2)| {
                b.iter(|| prob.compute_jacobian_variance(black_box(j1), black_box(j2)))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("should_fallback", size),
            &(&problem, &jacobian1, &jacobian2),
            |b, (prob, j1, j2)| {
                b.iter(|| {
                    prob.should_fallback_from_ad(black_box(j1), black_box(j2), 1e6)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Jacobian Method Comparison Benchmarks
// ============================================================================

fn bench_jacobian_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("jacobian_methods");

    for size in [3, 5, 10].iter() {
        let instruments = create_ois_instruments(*size);
        let problem: CalibrationProblem<f64, _> =
            CalibrationProblem::new(instruments).unwrap();
        let x = problem.initial_guess();

        group.throughput(Throughput::Elements(*size as u64));

        // Finite difference method
        group.bench_with_input(
            BenchmarkId::new("finite_diff", size),
            &(&problem, &x),
            |b, (prob, x_val)| {
                b.iter(|| prob.compute_jacobian_finite_diff(black_box(x_val)))
            },
        );

        // Central difference method
        group.bench_with_input(
            BenchmarkId::new("central_diff", size),
            &(&problem, &x),
            |b, (prob, x_val)| {
                b.iter(|| prob.compute_jacobian_central_diff(black_box(x_val)))
            },
        );
    }

    group.finish();
}

// ============================================================================
// Full Calibration Benchmarks with Numerical Stability Features
// ============================================================================

fn bench_calibration_with_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration_with_stability");

    for size in [3, 5, 10].iter() {
        let instruments = create_ois_instruments(*size);

        group.throughput(Throughput::Elements(*size as u64));

        // Default configuration (finite difference)
        group.bench_with_input(
            BenchmarkId::new("default_config", size),
            &instruments,
            |b, instr| {
                let config = GlobalBootstrapConfig::default();
                let bootstrapper = GlobalBootstrapper::new(config);
                b.iter(|| bootstrapper.calibrate(black_box(instr)))
            },
        );

        // With Jacobian inverse storage (for IFT)
        group.bench_with_input(
            BenchmarkId::new("with_jacobian_inverse", size),
            &instruments,
            |b, instr| {
                let config = GlobalBootstrapConfig::default()
                    .with_jacobian_inverse(true);
                let bootstrapper = GlobalBootstrapper::new(config);
                b.iter(|| bootstrapper.calibrate(black_box(instr)))
            },
        );

        // With central difference (more accurate but slower)
        group.bench_with_input(
            BenchmarkId::new("central_difference", size),
            &instruments,
            |b, instr| {
                let config = GlobalBootstrapConfig::default()
                    .with_jacobian_method(JacobianMethod::CentralDifference);
                let bootstrapper = GlobalBootstrapper::new(config);
                b.iter(|| bootstrapper.calibrate(black_box(instr)))
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_jacobian_validation,
    bench_condition_number,
    bench_tikhonov_regularisation,
    bench_jacobian_variance,
    bench_jacobian_methods,
    bench_calibration_with_stability,
);
criterion_main!(benches);

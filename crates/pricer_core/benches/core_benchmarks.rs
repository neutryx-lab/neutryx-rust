//! Criterion benchmarks for pricer_core interpolation methods.
//!
//! Measures performance of linear, cubic spline, and bilinear interpolation
//! across different data sizes to characterise scaling behaviour.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_core::math::interpolators::{
    BilinearInterpolator, CubicSplineInterpolator, Interpolator, LinearInterpolator,
};

/// Generate test data for 1D interpolation benchmarks.
fn generate_1d_data(n: usize) -> (Vec<f64>, Vec<f64>) {
    let xs: Vec<f64> = (0..n).map(|i| i as f64 / (n - 1) as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|&x| x.sin() + 0.5 * x * x).collect();
    (xs, ys)
}

/// Generate test data for 2D interpolation benchmarks.
fn generate_2d_data(nx: usize, ny: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let xs: Vec<f64> = (0..nx).map(|i| i as f64 / (nx - 1) as f64).collect();
    let ys: Vec<f64> = (0..ny).map(|j| j as f64 / (ny - 1) as f64).collect();
    let zs: Vec<Vec<f64>> = xs
        .iter()
        .map(|&x| ys.iter().map(|&y| x.sin() * y.cos()).collect())
        .collect();
    (xs, ys, zs)
}

/// Benchmark linear interpolation construction and lookup.
fn bench_linear_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_interpolation");

    for size in [100, 1000, 10000] {
        let (xs, ys) = generate_1d_data(size);

        // Benchmark construction
        group.bench_with_input(
            BenchmarkId::new("construction", size),
            &(&xs, &ys),
            |b, (xs, ys)| {
                b.iter(|| LinearInterpolator::new(black_box(xs), black_box(ys)).unwrap());
            },
        );

        // Benchmark lookup (create interpolator once, then benchmark lookups)
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();
        group.bench_with_input(BenchmarkId::new("lookup", size), &interp, |b, interp| {
            let test_x = 0.5; // Mid-point lookup
            b.iter(|| interp.interpolate(black_box(test_x)).unwrap());
        });

        // Benchmark multiple lookups (100 random points)
        group.bench_with_input(
            BenchmarkId::new("lookup_100", size),
            &interp,
            |b, interp| {
                let test_xs: Vec<f64> = (0..100).map(|i| i as f64 / 99.0).collect();
                b.iter(|| {
                    for &x in &test_xs {
                        let _ = interp.interpolate(black_box(x));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cubic spline interpolation construction and lookup.
fn bench_cubic_spline_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cubic_spline_interpolation");

    for size in [100, 1000, 10000] {
        let (xs, ys) = generate_1d_data(size);

        // Benchmark construction (includes tridiagonal solve)
        group.bench_with_input(
            BenchmarkId::new("construction", size),
            &(&xs, &ys),
            |b, (xs, ys)| {
                b.iter(|| CubicSplineInterpolator::new(black_box(xs), black_box(ys)).unwrap());
            },
        );

        // Benchmark lookup
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();
        group.bench_with_input(BenchmarkId::new("lookup", size), &interp, |b, interp| {
            let test_x = 0.5;
            b.iter(|| interp.interpolate(black_box(test_x)).unwrap());
        });

        // Benchmark multiple lookups (100 random points)
        group.bench_with_input(
            BenchmarkId::new("lookup_100", size),
            &interp,
            |b, interp| {
                let test_xs: Vec<f64> = (0..100).map(|i| i as f64 / 99.0).collect();
                b.iter(|| {
                    for &x in &test_xs {
                        let _ = interp.interpolate(black_box(x));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark bilinear interpolation construction and lookup.
fn bench_bilinear_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("bilinear_interpolation");

    // Grid sizes: 10x10, 32x32, 100x100
    for (nx, ny) in [(10, 10), (32, 32), (100, 100)] {
        let (xs, ys, zs) = generate_2d_data(nx, ny);
        let zs_refs: Vec<&[f64]> = zs.iter().map(|row| row.as_slice()).collect();
        let size_label = format!("{}x{}", nx, ny);

        // Benchmark construction
        group.bench_with_input(
            BenchmarkId::new("construction", &size_label),
            &(&xs, &ys, &zs_refs),
            |b, (xs, ys, zs)| {
                b.iter(|| {
                    BilinearInterpolator::new(black_box(xs), black_box(ys), black_box(zs)).unwrap()
                });
            },
        );

        // Benchmark lookup
        let interp = BilinearInterpolator::new(&xs, &ys, &zs_refs).unwrap();
        group.bench_with_input(
            BenchmarkId::new("lookup", &size_label),
            &interp,
            |b, interp| {
                let (test_x, test_y) = (0.5, 0.5);
                b.iter(|| {
                    interp
                        .interpolate(black_box(test_x), black_box(test_y))
                        .unwrap()
                });
            },
        );

        // Benchmark multiple lookups (100 random points on 10x10 grid)
        group.bench_with_input(
            BenchmarkId::new("lookup_100", &size_label),
            &interp,
            |b, interp| {
                let test_points: Vec<(f64, f64)> = (0..100)
                    .map(|i| {
                        let t = i as f64 / 99.0;
                        (t, (1.0 - t).abs())
                    })
                    .collect();
                b.iter(|| {
                    for &(x, y) in &test_points {
                        let _ = interp.interpolate(black_box(x), black_box(y));
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_linear_interpolation,
    bench_cubic_spline_interpolation,
    bench_bilinear_interpolation
);
criterion_main!(benches);

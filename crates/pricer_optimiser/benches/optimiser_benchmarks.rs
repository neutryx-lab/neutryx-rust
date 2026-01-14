//! Benchmarks for pricer_optimiser.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_optimiser::bootstrapping::{
    BootstrapCache, BootstrapInstrument, CachedBootstrapper, GenericBootstrapConfig,
    MultiCurveBuilder, ParallelCurveSetBuilder, SequentialBootstrapper, Tenor,
};
use pricer_optimiser::solvers::{Bfgs, LevenbergMarquardt};

fn benchmark_levenberg_marquardt(c: &mut Criterion) {
    let lm = LevenbergMarquardt::new();

    c.bench_function("lm_rosenbrock", |b| {
        b.iter(|| {
            let residuals = |p: &[f64]| vec![1.0 - p[0], 10.0 * (p[1] - p[0] * p[0])];
            let _ = lm.solve(black_box(&[-1.0, 1.0]), residuals);
        })
    });
}

fn benchmark_bfgs(c: &mut Criterion) {
    let bfgs = Bfgs::new();

    c.bench_function("bfgs_quadratic", |b| {
        b.iter(|| {
            let objective = |p: &[f64]| (p[0] - 2.0).powi(2) + (p[1] - 3.0).powi(2);
            let _ = bfgs.minimise(black_box(&[0.0, 0.0]), objective);
        })
    });
}

/// Generate OIS instruments for benchmarking.
fn generate_ois_instruments(count: usize) -> Vec<BootstrapInstrument<f64>> {
    (1..=count)
        .map(|i| {
            let maturity = i as f64 * 0.5; // 6M, 1Y, 1.5Y, ...
            let rate = 0.03 + (i as f64) * 0.001; // Increasing rates
            BootstrapInstrument::ois(maturity, rate)
        })
        .collect()
}

fn benchmark_bootstrap_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("bootstrap_sequential");

    for size in [10, 20, 50, 100] {
        let instruments = generate_ois_instruments(size);
        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &instruments,
            |b, insts| b.iter(|| bootstrapper.bootstrap(black_box(insts))),
        );
    }

    group.finish();
}

fn benchmark_bootstrap_cached(c: &mut Criterion) {
    let mut group = c.benchmark_group("bootstrap_cached");

    for size in [10, 20, 50, 100] {
        let instruments = generate_ois_instruments(size);
        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let mut cache = BootstrapCache::with_capacity(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &instruments,
            |b, insts| {
                b.iter(|| {
                    cache.clear();
                    bootstrapper.bootstrap_cached(black_box(insts), &mut cache)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_bootstrap_cached_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("bootstrap_cached_reuse");

    for size in [10, 20, 50, 100] {
        let instruments = generate_ois_instruments(size);
        let mut bootstrapper =
            CachedBootstrapper::<f64>::with_capacity(GenericBootstrapConfig::default(), size);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &instruments,
            |b, insts| b.iter(|| bootstrapper.bootstrap(black_box(insts))),
        );
    }

    group.finish();
}

fn benchmark_multi_curve_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_curve_build");

    for size in [10, 20] {
        let discount_instruments = generate_ois_instruments(size);
        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![
            (Tenor::ThreeMonth, generate_ois_instruments(size)),
            (Tenor::SixMonth, generate_ois_instruments(size)),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(discount_instruments.clone(), forward_instruments.clone()),
            |b, (disc, fwd)| b.iter(|| builder.build(black_box(disc), black_box(fwd))),
        );
    }

    group.finish();
}

fn benchmark_parallel_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_batch");

    // Create multiple curve sets to build in parallel
    for batch_size in [4, 8, 16] {
        let inputs: Vec<_> = (0..batch_size)
            .map(|i| {
                let base_rate = 0.02 + (i as f64) * 0.005;
                (
                    (1..=20)
                        .map(|j| {
                            let maturity = j as f64 * 0.5;
                            let rate = base_rate + (j as f64) * 0.0005;
                            BootstrapInstrument::ois(maturity, rate)
                        })
                        .collect::<Vec<_>>(),
                    vec![(
                        Tenor::ThreeMonth,
                        (1..=10)
                            .map(|j| {
                                let maturity = j as f64 * 0.5;
                                let rate = base_rate + 0.005 + (j as f64) * 0.0005;
                                BootstrapInstrument::irs(maturity, rate)
                            })
                            .collect::<Vec<_>>(),
                    )],
                )
            })
            .collect();

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &inputs,
            |b, inp| b.iter(|| builder.build_batch(black_box(inp))),
        );
    }

    group.finish();
}

fn benchmark_100_point_curve(c: &mut Criterion) {
    // Performance target: 100-point curve in <10ms
    let instruments = generate_ois_instruments(100);

    c.bench_function("100_point_curve_sequential", |b| {
        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        b.iter(|| bootstrapper.bootstrap(black_box(&instruments)))
    });

    c.bench_function("100_point_curve_cached", |b| {
        let mut bootstrapper =
            CachedBootstrapper::<f64>::with_capacity(GenericBootstrapConfig::default(), 100);
        b.iter(|| bootstrapper.bootstrap(black_box(&instruments)))
    });
}

criterion_group!(
    benches,
    benchmark_levenberg_marquardt,
    benchmark_bfgs,
    benchmark_bootstrap_sequential,
    benchmark_bootstrap_cached,
    benchmark_bootstrap_cached_reuse,
    benchmark_multi_curve_build,
    benchmark_parallel_batch,
    benchmark_100_point_curve,
);
criterion_main!(benches);

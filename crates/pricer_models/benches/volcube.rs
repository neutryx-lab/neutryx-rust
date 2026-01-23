//! Criterion benchmarks for VolCube operations.
//!
//! Benchmarks cover:
//! - VolCube volatility lookup throughput
//! - Cache lookup performance
//! - Probability density calculation

#![allow(missing_docs)]

use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_models::market::volcube::{
    InstrumentId, SabrParameterSurface, SabrParams, SharedVolCubeCache, VolCube, VolCubeBuilder,
    VolCubeCache, VolCubeConfig, VolCubeKey, VolInstrument, VolatilityCube,
};

/// Create a test VolCube with specified grid size.
fn create_test_cube(n_expiries: usize, n_tenors: usize) -> VolCube<f64> {
    let beta = 0.5;

    // Generate expiry and tenor grids
    let expiries: Vec<f64> = (0..n_expiries).map(|i| 0.25 + (i as f64) * 0.5).collect();
    let tenors: Vec<f64> = (0..n_tenors).map(|i| 1.0 + (i as f64) * 2.0).collect();

    // Generate SABR parameters for each grid point
    let params: Vec<Vec<SabrParams<f64>>> = (0..n_expiries)
        .map(|i| {
            (0..n_tenors)
                .map(|j| {
                    let alpha = 0.03 + (i as f64) * 0.005 + (j as f64) * 0.002;
                    let rho = -0.3 - (i as f64) * 0.02;
                    let nu = 0.4 - (j as f64) * 0.02;
                    SabrParams::new(alpha, beta, rho.max(-0.9), nu.max(0.1))
                })
                .collect()
        })
        .collect();

    let sabr_surface =
        SabrParameterSurface::new(expiries.clone(), tenors.clone(), &params, beta).unwrap();

    // Generate forwards
    let forwards: Vec<Vec<f64>> = (0..n_expiries)
        .map(|i| {
            (0..n_tenors)
                .map(|j| 0.02 + (i as f64) * 0.005 + (j as f64) * 0.003)
                .collect()
        })
        .collect();

    let config = VolCubeConfig::default();
    let source_instruments: Vec<InstrumentId> = (0..n_expiries * n_tenors * 3)
        .map(|i| InstrumentId::new(format!("INST-{}", i)))
        .collect();
    let strike_domain = (0.001, 0.15);

    VolCube::new(
        sabr_surface,
        forwards,
        config,
        source_instruments,
        strike_domain,
    )
}

/// Benchmark single volatility lookup.
fn bench_vol_lookup_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_vol_lookup");

    for (n_expiries, n_tenors) in [(2, 2), (5, 5), (10, 10)] {
        let cube = create_test_cube(n_expiries, n_tenors);

        group.bench_with_input(
            BenchmarkId::new("single", format!("{}x{}", n_expiries, n_tenors)),
            &cube,
            |b, cube| {
                b.iter(|| cube.volatility(black_box(0.75), black_box(3.0), black_box(0.03)));
            },
        );
    }

    group.finish();
}

/// Benchmark batch volatility lookups (10,000 queries).
fn bench_vol_lookup_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_vol_batch");

    let cube = create_test_cube(5, 5);

    // Generate 10,000 query points
    let queries: Vec<(f64, f64, f64)> = (0..10_000)
        .map(|i| {
            let expiry = 0.5 + (i % 100) as f64 * 0.02;
            let tenor = 2.0 + (i / 100 % 50) as f64 * 0.1;
            let strike = 0.01 + (i / 5000) as f64 * 0.04;
            (expiry, tenor, strike)
        })
        .collect();

    group.bench_function("10000_queries", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &(expiry, tenor, strike) in &queries {
                if let Ok(vol) =
                    cube.volatility(black_box(expiry), black_box(tenor), black_box(strike))
                {
                    sum += vol;
                }
            }
            sum
        });
    });

    group.finish();
}

/// Benchmark cache lookup performance.
fn bench_cache_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_cache");

    let cube = create_test_cube(5, 5);
    let cache: VolCubeCache<VolCube<f64>> = VolCubeCache::new(100);

    // Insert the cube into cache
    let key = VolCubeKey::new(12345, 67890);
    cache.insert(key.clone(), cube.clone());

    // Benchmark cache hit
    group.bench_function("lookup_hit", |b| {
        b.iter(|| cache.lookup(black_box(&key)));
    });

    // Benchmark cache miss
    let miss_key = VolCubeKey::new(99999, 88888);
    group.bench_function("lookup_miss", |b| {
        b.iter(|| cache.lookup(black_box(&miss_key)));
    });

    // Benchmark shared cache (Arc wrapper)
    let shared_cache: SharedVolCubeCache<VolCube<f64>> = Arc::new(cache);
    group.bench_function("shared_lookup_hit", |b| {
        b.iter(|| shared_cache.lookup(black_box(&key)));
    });

    group.finish();
}

/// Benchmark probability density calculation.
fn bench_probability_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_density");

    let cube = create_test_cube(5, 5);

    // Single density calculation
    group.bench_function("single", |b| {
        b.iter(|| cube.probability_density(black_box(1.0), black_box(0.03)));
    });

    // Batch density calculations (for risk-neutral PDF)
    let strikes: Vec<f64> = (0..100).map(|i| 0.01 + (i as f64) * 0.001).collect();
    group.bench_function("batch_100_strikes", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &strike in &strikes {
                if let Ok(d) = cube.probability_density(black_box(1.0), black_box(strike)) {
                    sum += d;
                }
            }
            sum
        });
    });

    group.finish();
}

/// Benchmark VolCube construction (not calibration, just struct creation).
fn bench_cube_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_construction");

    for (n_expiries, n_tenors) in [(2, 2), (5, 5), (10, 10)] {
        group.bench_with_input(
            BenchmarkId::new("grid", format!("{}x{}", n_expiries, n_tenors)),
            &(n_expiries, n_tenors),
            |b, &(ne, nt)| {
                b.iter(|| create_test_cube(black_box(ne), black_box(nt)));
            },
        );
    }

    group.finish();
}

/// Benchmark VolCubeBuilder with simple instruments.
fn bench_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_builder");
    group.sample_size(20); // Reduce sample size for slower benchmarks

    // Create test instruments
    let forward = 0.03;
    let instruments: Vec<VolInstrument<f64>> = vec![
        VolInstrument::new("1Y5Y_ATM", 1.0, 5.0, forward, 0.20, forward),
        VolInstrument::new("1Y5Y_LOW", 1.0, 5.0, 0.02, 0.25, forward),
        VolInstrument::new("1Y5Y_HIGH", 1.0, 5.0, 0.04, 0.22, forward),
        VolInstrument::new("2Y5Y_ATM", 2.0, 5.0, forward, 0.18, forward),
        VolInstrument::new("2Y5Y_LOW", 2.0, 5.0, 0.02, 0.23, forward),
        VolInstrument::new("2Y5Y_HIGH", 2.0, 5.0, 0.04, 0.20, forward),
        VolInstrument::new("1Y10Y_ATM", 1.0, 10.0, forward, 0.18, forward),
        VolInstrument::new("1Y10Y_LOW", 1.0, 10.0, 0.02, 0.22, forward),
        VolInstrument::new("1Y10Y_HIGH", 1.0, 10.0, 0.04, 0.19, forward),
        VolInstrument::new("2Y10Y_ATM", 2.0, 10.0, forward, 0.16, forward),
        VolInstrument::new("2Y10Y_LOW", 2.0, 10.0, 0.02, 0.20, forward),
        VolInstrument::new("2Y10Y_HIGH", 2.0, 10.0, 0.04, 0.17, forward),
    ];

    let config = VolCubeConfig::default();

    group.bench_function("build_12_instruments", |b| {
        b.iter(|| {
            VolCubeBuilder::new()
                .with_instruments(black_box(instruments.clone()))
                .with_config(black_box(config.clone()))
                .build()
        });
    });

    group.finish();
}

/// Benchmark cache with LRU eviction under load.
fn bench_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("volcube_cache_eviction");
    group.sample_size(20);

    group.bench_function("insert_100_capacity_50", |b| {
        b.iter(|| {
            let cache: VolCubeCache<VolCube<f64>> = VolCubeCache::new(50);
            let cube = create_test_cube(2, 2);

            for i in 0..100 {
                let key = VolCubeKey::new(i as u64, 0);
                cache.insert(key, cube.clone());
            }
            cache
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_vol_lookup_single,
    bench_vol_lookup_batch,
    bench_cache_lookup,
    bench_probability_density,
    bench_cube_construction,
    bench_builder,
    bench_cache_eviction
);
criterion_main!(benches);

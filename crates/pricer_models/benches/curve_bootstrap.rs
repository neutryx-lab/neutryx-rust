//! Criterion benchmarks for curve bootstrap engine.
//!
//! Benchmarks cover (Task 9.3 requirements):
//! - Cache hit response time vs construction time (target: <10%)
//! - Parallel access throughput (RwLock contention evaluation)
//! - Memory footprint with cached curves
//!
//! Run with: `cargo bench --bench curve_bootstrap`

#![allow(missing_docs)]

use std::{sync::Arc, thread};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pricer_models::market::{
    calibration::bootstrapping::{
        CalibrationInstrument, BootstrappedCurve, CurveDefinition, CurveEngine, CurveEngineBuilder,
        CurveKey, CurveResultCache, GenericBootstrapConfig, InstrumentTenor,
        SequentialBootstrapper,
    },
    curves::YieldCurve,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a standard set of OIS instruments for benchmarking.
fn create_ois_instruments(n: usize) -> Vec<CalibrationInstrument<f64>> {
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
            CalibrationInstrument::ois(t, rate)
        })
        .collect()
}

/// Create standard rates matching default USD-SOFR definition.
fn create_standard_rates() -> Vec<(InstrumentTenor, f64)> {
    vec![
        (InstrumentTenor::OneMonth, 0.025),
        (InstrumentTenor::ThreeMonths, 0.028),
        (InstrumentTenor::SixMonths, 0.030),
        (InstrumentTenor::OneYear, 0.032),
        (InstrumentTenor::TwoYears, 0.034),
        (InstrumentTenor::ThreeYears, 0.036),
        (InstrumentTenor::FiveYears, 0.038),
        (InstrumentTenor::SevenYears, 0.040),
        (InstrumentTenor::TenYears, 0.042),
        (InstrumentTenor::FifteenYears, 0.044),
        (InstrumentTenor::TwentyYears, 0.045),
        (InstrumentTenor::ThirtyYears, 0.046),
    ]
}

/// Create a pre-built curve for cache insertion.
fn create_sample_curve() -> BootstrappedCurve<f64> {
    let instruments = create_ois_instruments(6);
    let bootstrapper = SequentialBootstrapper::<f64>::new(GenericBootstrapConfig::default());
    bootstrapper.bootstrap(&instruments).unwrap().curve
}

// ============================================================================
// Benchmark Group 1: Curve Construction Baseline
// ============================================================================

fn bench_curve_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("curve_construction");

    // Benchmark construction with different number of pillars
    for n_pillars in [3, 6, 12] {
        let instruments = create_ois_instruments(n_pillars);
        let config = GenericBootstrapConfig::<f64>::default();

        group.throughput(Throughput::Elements(n_pillars as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential_bootstrap", n_pillars),
            &instruments,
            |b, insts| {
                let bootstrapper = SequentialBootstrapper::new(config.clone());
                b.iter(|| {
                    black_box(bootstrapper.bootstrap(black_box(insts)).unwrap());
                });
            },
        );
    }

    // Standard 12-pillar USD-SOFR curve via CurveEngine
    let definition = CurveDefinition::default_usd_sofr();
    let rates = create_standard_rates();

    group.bench_function("engine_build_curve_12_pillars", |b| {
        let engine = CurveEngine::<f64>::new();
        b.iter(|| {
            black_box(
                engine
                    .build_curve(black_box(&definition), black_box(&rates))
                    .unwrap(),
            );
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 2: Cache Performance
// ============================================================================

fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    // Setup: pre-populate cache
    let cache: CurveResultCache<f64> = CurveResultCache::new(100);
    let sample_curve = create_sample_curve();
    let rates = vec![0.03_f64, 0.032, 0.034, 0.037, 0.039, 0.042];
    let config_hash = 12345_u64;
    let key = CurveKey::from_rates(infra_master::market::RateIndex::Sofr, &rates, config_hash);
    cache.insert(key.clone(), sample_curve.clone());

    // Benchmark cache hit (lookup existing)
    group.bench_function("cache_hit_lookup", |b| {
        b.iter(|| {
            black_box(cache.lookup(black_box(&key)).unwrap());
        });
    });

    // Benchmark cache miss (lookup non-existing)
    let miss_key =
        CurveKey::from_rates(infra_master::market::RateIndex::Sofr, &[0.05, 0.06], 99999);
    group.bench_function("cache_miss_lookup", |b| {
        b.iter(|| {
            black_box(cache.lookup(black_box(&miss_key)));
        });
    });

    // Benchmark cache insert
    group.bench_function("cache_insert", |b| {
        let insert_cache: CurveResultCache<f64> = CurveResultCache::new(1000);
        let mut counter = 0_u64;
        b.iter(|| {
            counter += 1;
            let key = CurveKey::from_rates(
                infra_master::market::RateIndex::Sofr,
                &[counter as f64],
                counter,
            );
            insert_cache.insert(key, black_box(sample_curve.clone()));
        });
    });

    // Benchmark cache key creation
    group.bench_function("cache_key_creation", |b| {
        let rates = [0.03_f64, 0.032, 0.034, 0.037, 0.039, 0.042];
        b.iter(|| {
            black_box(CurveKey::from_rates(
                infra_master::market::RateIndex::Sofr,
                black_box(&rates),
                black_box(12345_u64),
            ));
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 3: Cache Hit vs Construction Time Comparison
// ============================================================================

fn bench_cache_hit_vs_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_vs_construction");

    let definition = CurveDefinition::default_usd_sofr();
    let rates = create_standard_rates();

    // Engine without cache (always constructs)
    group.bench_function("construction_no_cache", |b| {
        let engine = CurveEngine::<f64>::new();
        b.iter(|| {
            black_box(
                engine
                    .build_curve(black_box(&definition), black_box(&rates))
                    .unwrap(),
            );
        });
    });

    // Engine with cache - first call constructs
    // After warmup, subsequent calls hit cache
    let engine_with_cache = CurveEngineBuilder::<f64>::default().with_cache(100).build();
    // Warm up cache
    let _ = engine_with_cache.build_curve(&definition, &rates).unwrap();

    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            let result = engine_with_cache
                .build_curve(black_box(&definition), black_box(&rates))
                .unwrap();
            assert!(result.from_cache, "Should be cache hit");
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 4: Parallel Access Throughput
// ============================================================================

fn bench_parallel_cache_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_cache_access");
    group.sample_size(50); // Reduce sample size for parallel tests

    let cache = Arc::new(CurveResultCache::<f64>::new(1000));
    let sample_curve = create_sample_curve();

    // Pre-populate cache with 100 curves
    for i in 0..100 {
        let key = CurveKey::from_rates(
            infra_master::market::RateIndex::Sofr,
            &[i as f64 * 0.001],
            i as u64,
        );
        cache.insert(key, sample_curve.clone());
    }

    // Benchmark concurrent reads
    for n_threads in [2, 4, 8] {
        group.throughput(Throughput::Elements((n_threads * 100) as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent_reads", n_threads),
            &n_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let mut handles = Vec::with_capacity(num_threads);
                    for t in 0..num_threads {
                        let cache_clone = Arc::clone(&cache);
                        handles.push(thread::spawn(move || {
                            let mut total = 0_usize;
                            for i in 0..100 {
                                let key = CurveKey::from_rates(
                                    infra_master::market::RateIndex::Sofr,
                                    &[((t * 100 + i) % 100) as f64 * 0.001],
                                    ((t * 100 + i) % 100) as u64,
                                );
                                if cache_clone.lookup(&key).is_some() {
                                    total += 1;
                                }
                            }
                            total
                        }));
                    }
                    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
                    black_box(results);
                });
            },
        );
    }

    // Benchmark mixed read/write workload
    for n_threads in [2, 4] {
        let fresh_cache = Arc::new(CurveResultCache::<f64>::new(1000));
        let curve_for_insert = sample_curve.clone();

        group.throughput(Throughput::Elements((n_threads * 50) as u64));
        group.bench_with_input(
            BenchmarkId::new("mixed_read_write", n_threads),
            &n_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let mut handles = Vec::with_capacity(num_threads);
                    for t in 0..num_threads {
                        let cache_clone = Arc::clone(&fresh_cache);
                        let curve_clone = curve_for_insert.clone();
                        handles.push(thread::spawn(move || {
                            for i in 0..50 {
                                let key = CurveKey::from_rates(
                                    infra_master::market::RateIndex::Sofr,
                                    &[(t * 1000 + i) as f64 * 0.0001],
                                    (t * 1000 + i) as u64,
                                );
                                if i % 2 == 0 {
                                    // Even: write
                                    cache_clone.insert(key, curve_clone.clone());
                                } else {
                                    // Odd: read
                                    let _ = cache_clone.lookup(&key);
                                }
                            }
                        }));
                    }
                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 5: Memory Footprint Estimation
// ============================================================================

fn bench_cache_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory_footprint");
    group.sample_size(20); // Fewer samples for memory tests

    // Benchmark time to fill cache with N curves
    for n_curves in [10, 50, 100] {
        let sample_curve = create_sample_curve();

        group.throughput(Throughput::Elements(n_curves as u64));
        group.bench_with_input(
            BenchmarkId::new("fill_cache", n_curves),
            &n_curves,
            |b, &n| {
                b.iter(|| {
                    let cache: CurveResultCache<f64> = CurveResultCache::new(n);
                    for i in 0..n {
                        let key = CurveKey::from_rates(
                            infra_master::market::RateIndex::Sofr,
                            &[i as f64 * 0.001],
                            i as u64,
                        );
                        cache.insert(key, sample_curve.clone());
                    }
                    black_box(cache.stats());
                });
            },
        );
    }

    // Benchmark LRU eviction overhead when cache is full
    let sample_curve = create_sample_curve();
    let full_cache: CurveResultCache<f64> = CurveResultCache::new(100);

    // Fill the cache
    for i in 0..100 {
        let key = CurveKey::from_rates(
            infra_master::market::RateIndex::Sofr,
            &[i as f64 * 0.001],
            i as u64,
        );
        full_cache.insert(key, sample_curve.clone());
    }

    group.bench_function("insert_with_eviction", |b| {
        let mut counter = 1000_u64;
        b.iter(|| {
            counter += 1;
            let key = CurveKey::from_rates(
                infra_master::market::RateIndex::Sofr,
                &[counter as f64 * 0.0001],
                counter,
            );
            full_cache.insert(key, black_box(sample_curve.clone()));
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 6: Curve Operations After Construction
// ============================================================================

fn bench_curve_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("curve_operations");

    // Create a curve once for benchmarking operations
    let instruments = create_ois_instruments(12);
    let bootstrapper = SequentialBootstrapper::<f64>::new(GenericBootstrapConfig::default());
    let curve = bootstrapper.bootstrap(&instruments).unwrap().curve;

    // Benchmark discount_factor calculation
    group.bench_function("discount_factor_single", |b| {
        b.iter(|| {
            black_box(curve.discount_factor(black_box(5.0)).unwrap());
        });
    });

    // Benchmark zero_rate calculation
    group.bench_function("zero_rate_single", |b| {
        b.iter(|| {
            black_box(curve.zero_rate(black_box(5.0)).unwrap());
        });
    });

    // Benchmark forward_rate calculation
    group.bench_function("forward_rate_single", |b| {
        b.iter(|| {
            black_box(curve.forward_rate(black_box(4.0), black_box(5.0)).unwrap());
        });
    });

    // Batch discount factor calculations (typical pricing scenario)
    group.bench_function("discount_factor_batch_100", |b| {
        let times: Vec<f64> = (0..100).map(|i| (i as f64 + 1.0) * 0.1).collect();
        b.iter(|| {
            let mut sum = 0.0;
            for &t in &times {
                sum += curve.discount_factor(black_box(t)).unwrap();
            }
            black_box(sum);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = construction_benches;
    config = Criterion::default().significance_level(0.05).sample_size(100);
    targets = bench_curve_construction
);

criterion_group!(
    name = cache_benches;
    config = Criterion::default().significance_level(0.05).sample_size(100);
    targets = bench_cache_performance, bench_cache_hit_vs_construction
);

criterion_group!(
    name = parallel_benches;
    config = Criterion::default().significance_level(0.1).sample_size(50);
    targets = bench_parallel_cache_access
);

criterion_group!(
    name = memory_benches;
    config = Criterion::default().significance_level(0.1).sample_size(20);
    targets = bench_cache_memory_footprint
);

criterion_group!(
    name = operation_benches;
    config = Criterion::default().significance_level(0.05).sample_size(100);
    targets = bench_curve_operations
);

criterion_main!(
    construction_benches,
    cache_benches,
    parallel_benches,
    memory_benches,
    operation_benches
);

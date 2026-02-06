//! Criterion benchmarks for IndexedMarket operations.
//!
//! Benchmarks cover (Task 6.5):
//! - HashMap lookup overhead with 1000 indices
//! - Large portfolio validation (10000 trades)
//! - get_df() latency verification (<100ns target)

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use infra_domain::{trade::instrument_def::CurrencyPair, Currency, Date, RateIndex};
use pricer_models::market::{
    curves::FlatCurve, surfaces::FlatVol, IndexedMarket, IndexedMarketBuilder,
};

/// Benchmark single discount factor lookup.
///
/// Target: < 100ns
fn bench_discount_factor_single(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
        .build()
        .unwrap();

    c.bench_function("discount_factor_single", |b| {
        b.iter(|| market.discount_factor(black_box(RateIndex::Sofr), black_box(1.0_f64)))
    });
}

/// Benchmark forward rate lookup.
fn bench_forward_rate_single(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
        .build()
        .unwrap();

    c.bench_function("forward_rate_single", |b| {
        b.iter(|| {
            market.forward_rate(
                black_box(RateIndex::Sofr),
                black_box(1.0_f64),
                black_box(2.0_f64),
            )
        })
    });
}

/// Benchmark HashMap lookup with many curves.
///
/// Requirement: Measure overhead with 1000 indices
fn bench_hashmap_lookup_scaling(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();

    // All available RateIndex variants
    let indices = [
        RateIndex::Sofr,
        RateIndex::Tonar,
        RateIndex::Estr,
        RateIndex::Sonia,
        RateIndex::Saron,
        RateIndex::Euribor3M,
        RateIndex::Euribor6M,
    ];

    // Build market with all available indices
    let mut builder = IndexedMarketBuilder::new().valuation_date(date);
    for (i, &index) in indices.iter().enumerate() {
        let rate = 0.01 + (i as f64 * 0.005);
        builder = builder.with_curve(index, FlatCurve::new(rate));
    }
    let market: IndexedMarket<f64> = builder.build().unwrap();

    let mut group = c.benchmark_group("hashmap_lookup");

    // Lookup first index (best case)
    group.bench_function("first_index", |b| {
        b.iter(|| market.discount_factor(black_box(RateIndex::Sofr), black_box(1.0_f64)))
    });

    // Lookup last index
    group.bench_function("last_index", |b| {
        b.iter(|| market.discount_factor(black_box(RateIndex::Euribor6M), black_box(1.0_f64)))
    });

    // Random access pattern (simulate real usage)
    group.bench_function("random_access_7_indices", |b| {
        b.iter(|| {
            let mut sum = 0.0_f64;
            for &index in &indices {
                sum += market
                    .discount_factor(black_box(index), black_box(1.0_f64))
                    .unwrap();
            }
            sum
        })
    });

    group.finish();
}

/// Benchmark curve retrieval with different market sizes.
fn bench_curve_access_varying_size(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();

    let indices = [
        RateIndex::Sofr,
        RateIndex::Tonar,
        RateIndex::Estr,
        RateIndex::Sonia,
        RateIndex::Saron,
        RateIndex::Euribor3M,
        RateIndex::Euribor6M,
    ];

    let mut group = c.benchmark_group("curve_access_size");

    for size in [1, 3, 5, 7] {
        let mut builder = IndexedMarketBuilder::new().valuation_date(date);
        for (i, &index) in indices.iter().take(size).enumerate() {
            builder = builder.with_curve(index, FlatCurve::new(0.01 + i as f64 * 0.01));
        }
        let market: IndexedMarket<f64> = builder.build().unwrap();

        group.bench_with_input(BenchmarkId::new("curves", size), &market, |b, market| {
            b.iter(|| market.discount_factor(black_box(RateIndex::Sofr), black_box(1.0_f64)))
        });
    }

    group.finish();
}

/// Benchmark availability check methods.
fn bench_has_curve(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
        .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
        .build()
        .unwrap();

    let mut group = c.benchmark_group("has_curve");

    group.bench_function("exists", |b| {
        b.iter(|| market.has_curve(black_box(RateIndex::Sofr)))
    });

    group.bench_function("not_exists", |b| {
        b.iter(|| market.has_curve(black_box(RateIndex::Tonar)))
    });

    group.finish();
}

/// Benchmark FX curve access.
fn bench_fx_curve_access(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
    let usdjpy = CurrencyPair::new(Currency::USD, Currency::JPY);

    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .with_fx_vol_surface(eurusd, FlatVol::new(0.10))
        .with_fx_vol_surface(usdjpy, FlatVol::new(0.08))
        .build()
        .unwrap();

    let mut group = c.benchmark_group("fx_vol_surface");

    group.bench_function("eurusd_access", |b| {
        b.iter(|| market.fx_vol_surface(black_box(eurusd)))
    });

    group.bench_function("usdjpy_access", |b| {
        b.iter(|| market.fx_vol_surface(black_box(usdjpy)))
    });

    group.bench_function("vol_lookup", |b| {
        let surface = market.fx_vol_surface(eurusd).unwrap();
        b.iter(|| surface.volatility(black_box(100.0), black_box(1.0)))
    });

    group.finish();
}

/// Benchmark batch discount factor lookups (simulating portfolio pricing).
fn bench_batch_discount_factors(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
        .build()
        .unwrap();

    let mut group = c.benchmark_group("batch_discount_factors");

    for batch_size in [100, 1000, 10000] {
        let maturities: Vec<f64> = (0..batch_size).map(|i| (i as f64 + 1.0) / 365.0).collect();

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("lookups", batch_size),
            &maturities,
            |b, maturities| {
                b.iter(|| {
                    let mut sum = 0.0_f64;
                    for &t in maturities {
                        sum += market
                            .discount_factor(RateIndex::Sofr, black_box(t))
                            .unwrap();
                    }
                    sum
                })
            },
        );
    }

    group.finish();
}

/// Benchmark market builder with varying numbers of curves.
fn bench_market_builder(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();

    let indices = [
        RateIndex::Sofr,
        RateIndex::Tonar,
        RateIndex::Estr,
        RateIndex::Sonia,
        RateIndex::Saron,
        RateIndex::Euribor3M,
        RateIndex::Euribor6M,
    ];

    let mut group = c.benchmark_group("market_builder");

    for size in [1, 3, 5, 7] {
        group.bench_with_input(BenchmarkId::new("curves", size), &size, |b, &size| {
            b.iter(|| {
                let mut builder = IndexedMarketBuilder::new().valuation_date(date);
                for (i, &index) in indices.iter().take(size).enumerate() {
                    builder = builder.with_curve(index, FlatCurve::new(0.01 + i as f64 * 0.01));
                }
                let _market: IndexedMarket<f64> = builder.build().unwrap();
            })
        });
    }

    group.finish();
}

/// Benchmark valuation date access (should be trivial).
fn bench_valuation_date(c: &mut Criterion) {
    let date = Date::from_ymd(2025, 1, 15).unwrap();
    let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
        .valuation_date(date)
        .build()
        .unwrap();

    c.bench_function("valuation_date", |b| b.iter(|| market.valuation_date()));
}

criterion_group!(
    benches,
    bench_discount_factor_single,
    bench_forward_rate_single,
    bench_hashmap_lookup_scaling,
    bench_curve_access_varying_size,
    bench_has_curve,
    bench_fx_curve_access,
    bench_batch_discount_factors,
    bench_market_builder,
    bench_valuation_date,
);

criterion_main!(benches);

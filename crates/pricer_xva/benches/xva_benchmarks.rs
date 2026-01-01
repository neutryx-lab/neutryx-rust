//! Criterion benchmarks for pricer_xva XVA calculations.
//!
//! Benchmarks cover:
//! - Portfolio construction with varying trade counts
//! - Exposure calculation (EE, EPE, PFE)
//! - CVA/DVA computation
//! - SoA data structure operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_core::types::Currency;
use pricer_models::instruments::{
    ExerciseStyle, Instrument, InstrumentParams, PayoffType, VanillaOption,
};
use pricer_xva::exposure::ExposureCalculator;
use pricer_xva::portfolio::{
    Counterparty, CounterpartyId, CreditParams, NettingSet, NettingSetId, PortfolioBuilder, Trade,
    TradeId,
};
use pricer_xva::soa::TradeSoA;
use pricer_xva::xva::{compute_cva, compute_dva, generate_flat_discount_factors, OwnCreditParams};

/// Generate synthetic exposure scenarios for benchmarking.
fn generate_exposure_scenarios(n_scenarios: usize, n_times: usize) -> Vec<Vec<f64>> {
    (0..n_scenarios)
        .map(|s| {
            (0..n_times)
                .map(|t| {
                    // Synthetic exposure: starts at 0, peaks mid-life, decays
                    let mid = n_times as f64 / 2.0;
                    let t_frac = t as f64;
                    let base = 100.0 * (1.0 - (t_frac - mid).abs() / mid);
                    base + (((s * 17 + t * 13) % 100) as f64 - 50.0) * 0.3 // Add noise
                })
                .collect()
        })
        .collect()
}

/// Generate time grid for XVA calculations.
fn generate_time_grid(n_times: usize, maturity: f64) -> Vec<f64> {
    (0..n_times)
        .map(|i| maturity * i as f64 / (n_times - 1) as f64)
        .collect()
}

/// Benchmark Expected Exposure calculation.
fn bench_expected_exposure(c: &mut Criterion) {
    let mut group = c.benchmark_group("expected_exposure");

    // Different scenario/time combinations
    for (n_scenarios, n_times) in [(100, 50), (1000, 50), (10000, 50), (1000, 252)] {
        let label = format!("{}scenarios_{}times", n_scenarios, n_times);
        let values = generate_exposure_scenarios(n_scenarios, n_times);

        group.bench_with_input(BenchmarkId::new("ee", &label), &values, |b, values| {
            b.iter(|| ExposureCalculator::expected_exposure(black_box(values)));
        });
    }

    group.finish();
}

/// Benchmark EPE (time-weighted) calculation.
fn bench_expected_positive_exposure(c: &mut Criterion) {
    let mut group = c.benchmark_group("expected_positive_exposure");

    let values = generate_exposure_scenarios(1000, 50);
    let ee = ExposureCalculator::expected_exposure(&values);
    let time_grid = generate_time_grid(50, 5.0);

    group.bench_function("epe_50_times", |b| {
        b.iter(|| {
            ExposureCalculator::expected_positive_exposure(black_box(&ee), black_box(&time_grid))
        });
    });

    // Longer time grid
    let values_long = generate_exposure_scenarios(1000, 252);
    let ee_long = ExposureCalculator::expected_exposure(&values_long);
    let time_grid_long = generate_time_grid(252, 5.0);

    group.bench_function("epe_252_times", |b| {
        b.iter(|| {
            ExposureCalculator::expected_positive_exposure(
                black_box(&ee_long),
                black_box(&time_grid_long),
            )
        });
    });

    group.finish();
}

/// Benchmark PFE calculation (includes sorting per time step).
fn bench_potential_future_exposure(c: &mut Criterion) {
    let mut group = c.benchmark_group("potential_future_exposure");
    group.sample_size(50); // PFE is more expensive due to sorting

    for n_scenarios in [100, 1000, 10000] {
        let values = generate_exposure_scenarios(n_scenarios, 50);
        group.bench_with_input(
            BenchmarkId::new("pfe_95", n_scenarios),
            &values,
            |b, values| {
                b.iter(|| {
                    ExposureCalculator::potential_future_exposure(black_box(values), black_box(0.95))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark CVA calculation.
fn bench_cva_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cva_calculation");

    let credit = CreditParams::new(0.02, 0.4).unwrap();

    for n_times in [50, 100, 252, 500] {
        let values = generate_exposure_scenarios(1000, n_times);
        let ee = ExposureCalculator::expected_exposure(&values);
        let time_grid = generate_time_grid(n_times, 5.0);

        group.bench_with_input(
            BenchmarkId::new("cva", n_times),
            &(&ee, &time_grid, &credit),
            |b, (ee, time_grid, credit)| {
                b.iter(|| compute_cva(black_box(ee), black_box(time_grid), black_box(credit)));
            },
        );
    }

    group.finish();
}

/// Benchmark DVA calculation.
fn bench_dva_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dva_calculation");

    let own_credit = OwnCreditParams::new(0.015, 0.35).unwrap(); // Own credit params
    let time_grid = generate_time_grid(50, 5.0);

    // Generate negative exposure profile for DVA (we owe counterparty)
    let neg_values: Vec<Vec<f64>> = generate_exposure_scenarios(1000, 50)
        .into_iter()
        .map(|v| v.into_iter().map(|x| -x).collect())
        .collect();
    let ene = ExposureCalculator::expected_exposure(&neg_values);

    group.bench_function("dva_50_times", |b| {
        b.iter(|| compute_dva(black_box(&ene), black_box(&time_grid), black_box(&own_credit)));
    });

    group.finish();
}

/// Benchmark portfolio construction with different sizes.
fn bench_portfolio_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("portfolio_construction");

    for n_trades in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("build", n_trades),
            &n_trades,
            |b, &n| {
                // Pre-create trades outside the benchmark loop
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
                            Instrument::Vanilla(option),
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
            },
        );
    }

    group.finish();
}

/// Benchmark TradeSoA operations.
fn bench_trade_soa(c: &mut Criterion) {
    let mut group = c.benchmark_group("trade_soa");

    for n_trades in [100, 1000, 10000] {
        // Create trades for SoA benchmark (counterparty/netting_set not needed for TradeSoA)
        let _credit = CreditParams::new(0.02, 0.4).unwrap();

        let trades: Vec<Trade> = (0..n_trades)
            .map(|i| {
                let strike = 90.0 + (i as f64 / n_trades as f64) * 20.0;
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
                    Instrument::Vanilla(option),
                    Currency::USD,
                    CounterpartyId::new("CP001"),
                    NettingSetId::new("NS001"),
                    1_000_000.0,
                )
            })
            .collect();

        // Benchmark SoA creation
        let trade_refs: Vec<&Trade> = trades.iter().collect();
        group.bench_with_input(
            BenchmarkId::new("from_trades", n_trades),
            &trade_refs,
            |b, trade_refs| {
                b.iter(|| TradeSoA::from_trades(black_box(trade_refs)));
            },
        );
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

criterion_group!(
    benches,
    bench_expected_exposure,
    bench_expected_positive_exposure,
    bench_potential_future_exposure,
    bench_cva_calculation,
    bench_dva_calculation,
    bench_portfolio_construction,
    bench_trade_soa,
    bench_discount_factors
);
criterion_main!(benches);

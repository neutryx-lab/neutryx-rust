//! Criterion benchmarks for pricer_risk XVA and risk calculations.
//!
//! Benchmarks cover:
//! - Portfolio construction with varying trade counts
//! - Exposure calculation (EE, EPE, PFE)
//! - CVA/DVA computation
//! - SoA data structure operations
//! - IRS Greeks calculation (AAD vs Bump-and-Revalue comparison)
//! - Parallel portfolio Greeks calculation
//!
//! # Requirements Coverage
//!
//! - Requirement 3.1: Rayon parallel processing for 1000+ trades
//! - Requirement 3.2: AAD vs Bump-and-Revalue speedup (5x target)
//! - Requirement 3.4: Criterion format benchmark output

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet};
use pricer_core::types::time::{Date, DayCountConvention};
use pricer_core::types::Currency;
use pricer_models::instruments::rates::{
    FixedLeg, FloatingLeg, InterestRateSwap, RateIndex, SwapDirection,
};
use pricer_models::instruments::{
    ExerciseStyle, Instrument, InstrumentParams, PayoffType, VanillaOption,
};
use pricer_models::schedules::{Frequency, ScheduleBuilder};
use pricer_pricing::greeks::GreeksMode;
use pricer_risk::exposure::ExposureCalculator;
use pricer_risk::parallel::{ParallelGreeksConfig, ParallelPortfolioGreeksCalculator};
use pricer_risk::portfolio::{
    Counterparty, CounterpartyId, CreditParams, NettingSet, NettingSetId, PortfolioBuilder, Trade,
    TradeId,
};
use pricer_risk::scenarios::{GreeksByFactorConfig, IrsGreeksByFactorCalculator};
use pricer_risk::soa::TradeSoA;
use pricer_risk::xva::{compute_cva, compute_dva, generate_flat_discount_factors, OwnCreditParams};

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
                    ExposureCalculator::potential_future_exposure(
                        black_box(values),
                        black_box(0.95),
                    )
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
        });
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

// =============================================================================
// IRS Greeks Benchmarks (Task 3.2)
// =============================================================================

/// Helper function to create a test IRS swap with given notional and tenor
fn create_benchmark_swap(notional: f64, tenor_years: u32) -> InterestRateSwap<f64> {
    let start_date = Date::from_ymd(2025, 1, 15).unwrap();
    let end_date = Date::from_ymd(2025 + tenor_years as i32, 1, 15).unwrap();

    let fixed_schedule = ScheduleBuilder::new()
        .start(start_date)
        .end(end_date)
        .frequency(Frequency::SemiAnnual)
        .day_count(DayCountConvention::Thirty360)
        .build()
        .unwrap();

    let floating_schedule = ScheduleBuilder::new()
        .start(start_date)
        .end(end_date)
        .frequency(Frequency::Quarterly)
        .day_count(DayCountConvention::ActualActual360)
        .build()
        .unwrap();

    let fixed_leg = FixedLeg::new(fixed_schedule, 0.02, DayCountConvention::Thirty360);
    let floating_leg = FloatingLeg::new(
        floating_schedule,
        0.0,
        RateIndex::Sofr,
        DayCountConvention::ActualActual360,
    );

    InterestRateSwap::new(
        notional,
        fixed_leg,
        floating_leg,
        Currency::USD,
        SwapDirection::PayFixed,
    )
}

/// Helper function to create test curves
fn create_benchmark_curves() -> CurveSet<f64> {
    let mut curves = CurveSet::new();
    curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
    curves.insert(CurveName::Forward, CurveEnum::flat(0.035));
    curves.set_discount_curve(CurveName::Discount);
    curves
}

/// Helper function to create multiple test swaps with varying notionals
fn create_benchmark_swaps(n: usize) -> Vec<InterestRateSwap<f64>> {
    (0..n)
        .map(|i| {
            let notional = 10_000_000.0 * (1.0 + (i % 10) as f64 * 0.1);
            let tenor = 5 + (i % 5) as u32; // Tenors from 5Y to 9Y
            create_benchmark_swap(notional, tenor)
        })
        .collect()
}

/// Benchmark IRS Greeks calculation - Bump-and-Revalue method.
///
/// # Requirements Coverage
///
/// - Requirement 3.2: Bump-and-Revalue baseline measurement
fn bench_irs_greeks_bump(c: &mut Criterion) {
    let mut group = c.benchmark_group("irs_greeks_bump");
    group.sample_size(30); // Reduce sample size for expensive operations

    let config = GreeksByFactorConfig::new()
        .with_second_order(false)
        .with_validation(true);
    let calculator = IrsGreeksByFactorCalculator::new(config);
    let curves = create_benchmark_curves();
    let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

    // Benchmark different swap tenors
    for tenor in [5, 10, 15, 20] {
        let swap = create_benchmark_swap(10_000_000.0, tenor);
        group.bench_with_input(
            BenchmarkId::new("first_order", format!("{}Y", tenor)),
            &swap,
            |b, swap| {
                b.iter(|| {
                    calculator.compute_first_order_greeks(
                        black_box(swap),
                        black_box(&curves),
                        black_box(valuation_date),
                    )
                });
            },
        );
    }

    // Benchmark with second-order Greeks
    let config_gamma = GreeksByFactorConfig::new()
        .with_second_order(true)
        .with_validation(true);
    let calculator_gamma = IrsGreeksByFactorCalculator::new(config_gamma);
    let swap_5y = create_benchmark_swap(10_000_000.0, 5);

    group.bench_function("second_order_5Y", |b| {
        b.iter(|| {
            calculator_gamma.compute_second_order_greeks(
                black_box(&swap_5y),
                black_box(&curves),
                black_box(valuation_date),
            )
        });
    });

    group.finish();
}

/// Benchmark AAD vs Bump-and-Revalue comparison.
///
/// Note: AAD mode currently falls back to bump-and-revalue when
/// enzyme-ad feature is not enabled. This benchmark documents
/// the baseline for comparison when AAD becomes available.
///
/// # Requirements Coverage
///
/// - Requirement 3.2: AAD vs Bump speedup comparison (target: 5x)
fn bench_aad_vs_bump_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("aad_vs_bump");
    group.sample_size(30);

    let config = GreeksByFactorConfig::new()
        .with_second_order(false)
        .with_validation(true);
    let calculator = IrsGreeksByFactorCalculator::new(config);
    let curves = create_benchmark_curves();
    let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();
    let swap = create_benchmark_swap(10_000_000.0, 10);

    // Bump-and-Revalue baseline
    group.bench_function("bump_10Y", |b| {
        b.iter(|| {
            calculator.compute_greeks_by_factor(
                black_box(&swap),
                black_box(&curves),
                black_box(valuation_date),
                black_box(GreeksMode::BumpRevalue),
            )
        });
    });

    // NumDual mode (analytical when available)
    group.bench_function("numdual_10Y", |b| {
        b.iter(|| {
            calculator.compute_greeks_by_factor(
                black_box(&swap),
                black_box(&curves),
                black_box(valuation_date),
                black_box(GreeksMode::NumDual),
            )
        });
    });

    group.finish();
}

/// Benchmark parallel portfolio Greeks calculation.
///
/// # Requirements Coverage
///
/// - Requirement 3.1: Rayon parallel processing for 1000+ trades
fn bench_parallel_portfolio_greeks(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_portfolio_greeks");
    group.sample_size(20); // Portfolio computations are expensive

    let curves = create_benchmark_curves();
    let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

    // Benchmark different portfolio sizes
    for n_trades in [10, 50, 100, 200] {
        let swaps = create_benchmark_swaps(n_trades);

        // Sequential processing
        let config_seq = ParallelGreeksConfig::new()
            .with_batch_size(64)
            .with_parallel_threshold(10000); // Force sequential

        let calculator_seq = ParallelPortfolioGreeksCalculator::new(config_seq);

        group.bench_with_input(
            BenchmarkId::new("sequential", n_trades),
            &swaps,
            |b, swaps| {
                b.iter(|| {
                    calculator_seq.compute_portfolio_greeks_sequential(
                        black_box(swaps),
                        black_box(&curves),
                        black_box(valuation_date),
                    )
                });
            },
        );

        // Parallel processing
        let config_par = ParallelGreeksConfig::new()
            .with_batch_size(32)
            .with_parallel_threshold(10); // Force parallel

        let calculator_par = ParallelPortfolioGreeksCalculator::new(config_par);

        group.bench_with_input(
            BenchmarkId::new("parallel", n_trades),
            &swaps,
            |b, swaps| {
                b.iter(|| {
                    calculator_par.compute_portfolio_greeks_parallel(
                        black_box(swaps),
                        black_box(&curves),
                        black_box(valuation_date),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch size impact on parallel performance.
///
/// # Requirements Coverage
///
/// - Requirement 3.1: Optimal batch size tuning
fn bench_batch_size_tuning(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_size_tuning");
    group.sample_size(20);

    let swaps = create_benchmark_swaps(200);
    let curves = create_benchmark_curves();
    let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

    // Test different batch sizes
    for batch_size in [16, 32, 64, 128] {
        let config = ParallelGreeksConfig::new()
            .with_batch_size(batch_size)
            .with_parallel_threshold(10);

        let calculator = ParallelPortfolioGreeksCalculator::new(config);

        group.bench_with_input(BenchmarkId::new("batch", batch_size), &swaps, |b, swaps| {
            b.iter(|| {
                calculator.compute_portfolio_greeks_parallel(
                    black_box(swaps),
                    black_box(&curves),
                    black_box(valuation_date),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark portfolio scalability (1000+ trades).
///
/// # Requirements Coverage
///
/// - Requirement 3.1: Large portfolio (1000+ trades) performance
fn bench_large_portfolio_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_portfolio_scalability");
    group.sample_size(10); // Very expensive benchmarks

    let curves = create_benchmark_curves();
    let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

    // Large portfolio benchmarks
    for n_trades in [500, 1000] {
        let swaps = create_benchmark_swaps(n_trades);

        let config = ParallelGreeksConfig::new()
            .with_batch_size(64)
            .with_parallel_threshold(100)
            .with_second_order(false);

        let calculator = ParallelPortfolioGreeksCalculator::new(config);

        group.bench_with_input(BenchmarkId::new("trades", n_trades), &swaps, |b, swaps| {
            b.iter(|| {
                calculator.compute_portfolio_greeks(
                    black_box(swaps),
                    black_box(&curves),
                    black_box(valuation_date),
                )
            });
        });
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
    bench_discount_factors,
    // Task 3.2: IRS Greeks benchmarks
    bench_irs_greeks_bump,
    bench_aad_vs_bump_comparison,
    bench_parallel_portfolio_greeks,
    bench_batch_size_tuning,
    bench_large_portfolio_scalability,
);
criterion_main!(benches);

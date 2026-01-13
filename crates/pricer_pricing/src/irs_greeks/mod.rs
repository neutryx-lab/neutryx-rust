//! IRS Greeks calculation module for AAD demonstration.
//!
//! This module provides:
//! - [`IrsGreeksCalculator`]: IRS Greeks calculator (AAD/Bump-and-Revalue)
//! - [`IrsGreeksConfig`]: Configuration for IRS Greeks calculation
//! - [`IrsDeltaResult`]: Delta calculation results with timing
//! - [`IrsGreeksResult`]: Combined Greeks results for mode comparison
//! - [`IrsLazyEvaluator`]: Lazy evaluation with caching and dependency tracking
//! - [`BenchmarkRunner`]: Performance benchmarking for AAD vs Bump-and-Revalue
//! - [`XvaDemoRunner`]: XVA demo with exposure profiles and sensitivity benchmarking
//!
//! # AAD Integration
//!
//! When the `enzyme-ad` feature is enabled, this module uses Enzyme LLVM-level
//! automatic differentiation for computing all tenor Deltas in a single reverse
//! pass. Without the feature, it falls back to bump-and-revalue.
//!
//! # LazyValuation
//!
//! The `IrsLazyEvaluator` provides:
//! - Dependency graph tracking curve tenor -> swap relationships
//! - Result caching with automatic invalidation on curve updates
//! - Selective recalculation of only affected swaps
//! - AAD tape reuse capability
//!
//! # Performance Benchmarking
//!
//! The `BenchmarkRunner` provides:
//! - PV, single Delta, and all-tenor Delta benchmarking
//! - Statistical analysis (mean, std dev, min, max)
//! - Scalability measurement with varying tenor counts
//! - JSON and Markdown output formats
//!
//! # XVA Demo
//!
//! The `XvaDemoRunner` provides (Tasks 5.1-5.4):
//! - Exposure profile calculation (Expected Exposure - EE)
//! - CVA/DVA calculation with counterparty and own credit parameters
//! - XVA AAD sensitivity calculation
//! - XVA performance comparison between AAD and Bump-and-Revalue
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_pricing::irs_greeks::{IrsGreeksCalculator, IrsGreeksConfig, GreeksMode};
//! use pricer_pricing::irs_greeks::{IrsLazyEvaluator, SwapId, TenorPoint};
//! use pricer_pricing::irs_greeks::{BenchmarkRunner, BenchmarkConfig};
//! use pricer_pricing::irs_greeks::xva_demo::{XvaDemoRunner, XvaDemoConfig, CreditParams};
//! use pricer_models::instruments::rates::{InterestRateSwap, ...};
//! use pricer_core::market_data::curves::CurveSet;
//!
//! let config = IrsGreeksConfig::default();
//! let calculator = IrsGreeksCalculator::<f64>::new(config);
//!
//! let npv = calculator.compute_npv(&swap, &curves, valuation_date)?;
//! let dv01 = calculator.compute_dv01(&swap, &curves, valuation_date)?;
//!
//! // LazyValuation usage
//! let mut evaluator = IrsLazyEvaluator::<f64>::new();
//! evaluator.register_dependencies(swap_id, tenor_points);
//! // Cache automatically invalidates on curve update
//! evaluator.notify_curve_update(curve_id, tenor);
//!
//! // Benchmarking usage
//! let runner = BenchmarkRunner::new(BenchmarkConfig::default());
//! let result = runner.benchmark_deltas(&swap, &curves, valuation_date, &tenor_points)?;
//! println!("Speedup ratio: {}", result.speedup_ratio);
//!
//! // XVA Demo usage
//! let xva_config = XvaDemoConfig::default();
//! let xva_runner = XvaDemoRunner::new(xva_config);
//! let credit = CreditParams::new(0.02, 0.4).unwrap();
//! let xva_result = xva_runner.run_xva_demo(&swap, &curves, valuation_date, &credit, None)?;
//! println!("CVA: {}", xva_result.cva);
//! ```

mod benchmark;
mod calculator;
mod config;
mod error;
mod lazy_evaluator;
mod result;
pub mod xva_demo;

#[cfg(test)]
mod tests;

pub use benchmark::{
    BenchmarkConfig, BenchmarkError, BenchmarkRunner, DeltaBenchmarkResult, FullBenchmarkResult,
    PvBenchmarkResult, ScalabilityResult, SingleDeltaBenchmarkResult, SwapParams, TimingStats,
};
pub use calculator::IrsGreeksCalculator;
pub use config::IrsGreeksConfig;
pub use error::IrsGreeksError;
pub use lazy_evaluator::{
    AadTapeCache, CacheKey, CacheState, CacheStats, CachedResult, CachedTape, DependencyGraph,
    IrsLazyEvaluator, SwapId, TapeCacheStats, TenorPoint,
};
pub use result::{IrsDeltaResult, IrsGreeksResult};
pub use xva_demo::{
    CreditParams as XvaCreditParams, ExposureProfile, XvaDemoConfig, XvaDemoError, XvaDemoRunner,
    XvaResult, XvaSensitivityBenchmark,
};

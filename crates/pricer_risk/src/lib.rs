// Clippy configuration for pricer_risk
// FCA/FBA/FVA are standard finance abbreviations
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::expect_used)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::if_not_else)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::cast_possible_wrap)]
// Enzyme AD: Enable autodiff feature when enzyme-ad feature is active
// This requires nightly Rust (nightly-2025-01-15) with Enzyme LLVM plugin
#![cfg_attr(feature = "enzyme-ad", feature(autodiff))]

//! # Pricer Risk (L4: Application)
//!
//! Portfolio risk management, XVA calculations, and parallelisation.
//!
//! **Note**: This crate was renamed from `pricer_risk` to `pricer_risk` in
//! version 0.7.0. The new name better reflects the broader risk management
//! capabilities including risk factors, scenario analysis, and Greeks
//! aggregation.
//!
//! This crate provides:
//! - Portfolio and trade structures with netting sets
//! - Counterparty credit parameters
//! - Exposure aggregation (EE, EPE, PFE)
//! - CVA, DVA, FVA calculations
//! - Structure of Arrays (SoA) for cache efficiency
//! - Rayon-based parallelisation for Greeks computation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │            pricer_risk (L4)             │
//! ├─────────────────────────────────────────┤
//! │  portfolio/  - Trade, Counterparty,    │
//! │               NettingSet, Portfolio     │
//! │  exposure/   - EE, EPE, PFE metrics    │
//! │  xva/        - CVA, DVA, FVA           │
//! │  soa/        - Structure of Arrays     │
//! │  parallel/   - Rayon utilities         │
//! └─────────────────────────────────────────┘
//!          ↓
//! ┌─────────────────────────────────────────┐
//! │           pricer_pricing (L3)          │
//! │  Monte Carlo engine with Enzyme AD     │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Performance
//!
//! - Uses SoA layout for SIMD-friendly memory access
//! - Parallel computation with Rayon (target: >80% efficiency on 8+ cores)
//! - Batch processing for large portfolios
//!
//! ## Example
//!
//! ```
//! use pricer_risk::portfolio::{
//!     PortfolioBuilder, Trade, TradeId, Counterparty, CounterpartyId,
//!     NettingSet, NettingSetId, CreditParams,
//! };
//! use infra_domain::market::Currency;
//! use infra_domain::trade::{
//!     PricingInstrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
//! };
//!
//! // Build a portfolio
//! let credit = CreditParams::new(0.02, 0.4).unwrap();
//! let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit);
//!
//! let netting_set = NettingSet::new(
//!     NettingSetId::new("NS001"),
//!     CounterpartyId::new("CP001"),
//! );
//!
//! let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//!
//! let trade = Trade::new(
//!     TradeId::new("T001"),
//!     PricingInstrument::Vanilla(call),
//!     Currency::USD,
//!     CounterpartyId::new("CP001"),
//!     NettingSetId::new("NS001"),
//!     1_000_000.0,
//! );
//!
//! let portfolio = PortfolioBuilder::new()
//!     .add_counterparty(counterparty)
//!     .add_netting_set(netting_set)
//!     .add_trade(trade)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(portfolio.trade_count(), 1);
//! ```

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

mod engine;
mod error;
mod result;

pub mod greeks;
pub mod portfolio;
pub mod scenarios;

// Re-export commonly used types
// Risk Engine facade
pub use engine::{RiskEngine, RiskEngineConfig, ScenarioGreeksResult, ScenarioPortfolioResult};
pub use error::{PartialGreeksResult, RiskError};
pub use result::{
    AggregatedGreeks, ComputedGreeks, ExecutionStats, FailedCalculation, PerformanceMetrics,
    PortfolioRiskResult, RiskResult,
};
// AD types (automatic differentiation)
pub use greeks::ad::{gradient, gradient_with_step, ADMode, Activity};
pub use greeks::{
    GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksError, GreeksMode, GreeksResult,
};
// TODO: Re-enable when rates instruments are restored
// pub use scenarios::{
//     BucketDv01Calculator, BucketDv01Config, BucketDv01Entry, BucketDv01Error,
//     BucketDv01Result, GreeksByFactorConfig, GreeksByFactorError,
//     IrsGreeksByFactorCalculator, KeyRateDurationEntry, KeyRateDurationResult,
//     STANDARD_TENOR_LABELS, STANDARD_TENOR_POINTS,
// };
pub use portfolio::xva::{
    compute_cva, compute_cva_with_survival, compute_dva, compute_dva_with_survival, compute_fba,
    compute_fca, compute_fva, generate_flat_discount_factors, CounterpartyXva, ExposureSoA,
    FundingParams, NettingSetXva, OwnCreditParams, PortfolioXva, XvaCalculator, XvaConfig,
    XvaError,
};
pub use portfolio::{
    CollateralAgreement, Counterparty, CounterpartyId, CreditParams, CreditRating,
    ExposureCalculator, NettingSet, NettingSetId, Portfolio, PortfolioBuilder, PortfolioError,
    Trade, TradeBuilder, TradeId,
};
pub use scenarios::{
    AggregationMethod, BumpScenario, CurveShiftError, CurveShiftSpec, CurveShiftType, CurveShifter,
    GreeksAggregator, GreeksResultByFactor, PortfolioGreeks, PresetScenario, PresetScenarioType,
    RiskFactorId, RiskFactorShift, Scenario, ScenarioEngine, ScenarioPnL, ScenarioResult,
};

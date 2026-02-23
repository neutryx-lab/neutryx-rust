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
#![cfg_attr(feature = "enzyme-ad", feature(autodiff))]

//! Pricer Risk (L4): Portfolio risk management, XVA calculations, and
//! parallelisation.

#![allow(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

mod engine;
mod error;
mod result;

pub mod greeks;
pub mod portfolio;
pub mod scenarios;
pub mod xva_engine;

pub use engine::{RiskEngine, RiskEngineConfig, ScenarioGreeksResult, ScenarioPortfolioResult};
pub use error::{PartialGreeksResult, RiskError};
pub use greeks::{
    ad::{gradient, gradient_with_step, ADMode, Activity},
    GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksError, GreeksMode, GreeksResult,
};
// TODO: Re-enable when rates instruments are restored
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
pub use result::{
    AggregatedGreeks, ComputedGreeks, ExecutionStats, FailedCalculation, PerformanceMetrics,
    PortfolioRiskResult, RiskResult,
};
pub use scenarios::{
    AggregationMethod, BumpScenario, CurveShiftError, CurveShiftSpec, CurveShiftType, CurveShifter,
    GreeksAggregator, GreeksResultByFactor, PortfolioGreeks, PresetScenario, PresetScenarioType,
    RiskFactorId, RiskFactorShift, Scenario, ScenarioEngine, ScenarioPnL, ScenarioResult,
};

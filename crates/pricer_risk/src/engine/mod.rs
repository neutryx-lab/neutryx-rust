//! Risk Engine module.
//!
//! Provides the unified [`RiskEngine`] for all risk operations.
//!
//! # Overview
//!
//! The `RiskEngine` is the **single entry point** for:
//!
//! - **Greeks calculation**: AAD (Enzyme) or Bump-and-Revalue
//! - **Scenario analysis**: Stress testing with `ScenarioEngine`
//! - **Portfolio operations**: Pricing, aggregation, XVA
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        RiskEngine                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Greeks API:                                                │
//! │    compute_greeks()           - Single trade                │
//! │    compute_portfolio_greeks() - Portfolio                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Scenario API (delegates to ScenarioEngine):                │
//! │    add_scenario()             - Register scenario           │
//! │    run_all_scenarios()        - Execute all                 │
//! │    worst_case_scenario()      - Get worst P&L               │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Portfolio API:                                             │
//! │    price_portfolio()          - Price all trades            │
//! │    aggregate_by_netting_set() - Aggregate by NS             │
//! │    total_portfolio_value()    - Sum of prices               │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_risk::{RiskEngine, RiskEngineConfig};
//! use pricer_risk::scenarios::{Scenario, BumpScenario, RiskFactorShift};
//! use infra_config::{RiskConfig, GreeksMethod};
//!
//! // Create engine
//! let mut engine = RiskEngine::with_defaults();
//!
//! // === Greeks Calculation ===
//! let result = engine.compute_greeks("T001", || {
//!     Ok(GreeksResult::new(100.0, 0.01).with_delta(0.5))
//! })?;
//!
//! // === Scenario Analysis ===
//! engine.add_scenario(Scenario::named(
//!     "IR +100bp",
//!     BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.01)),
//! ));
//! let results = engine.run_all_scenarios(1_000_000.0, |name| stressed_value(name));
//!
//! // === Portfolio Pricing ===
//! let prices = engine.price_portfolio(&portfolio, |trade| pricer.price(trade));
//! ```
//!
//! # Requirements Coverage
//!
//! - Requirement 5.1: RiskEngine facade
//! - Requirement 5.2: compute_greeks() method
//! - Requirement 5.3: AAD/Bump mode selection
//! - Requirement 5.4: Risk factor identification
//! - Requirement 5.5: RiskResult with metrics
//! - Requirement 10.4: ScenarioEngine integration

mod engine;
mod error;
mod result;

pub use engine::{RiskEngine, RiskEngineConfig, ScenarioGreeksResult, ScenarioPortfolioResult};
pub use error::{PartialGreeksResult, RiskError};
pub use result::{
    AggregatedGreeks, ComputedGreeks, ExecutionStats, FailedCalculation, PerformanceMetrics,
    PortfolioRiskResult, RiskResult,
};

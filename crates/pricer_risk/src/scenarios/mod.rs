//! Scenario analysis and risk factor management.
//!
//! This module provides infrastructure for:
//! - Risk factor shifts (parallel, twist, butterfly)
//! - Scenario definition and execution
//! - Greeks aggregation
//! - Preset stress scenarios
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │              Scenario Engine                  │
//! ├──────────────────────────────────────────────┤
//! │  RiskFactorShift  - Individual factor shifts │
//! │  Scenario         - Named set of shifts      │
//! │  ScenarioEngine   - Execution & results      │
//! │  GreeksAggregator - Portfolio-level Greeks   │
//! └──────────────────────────────────────────────┘
//! ```

mod aggregator;
mod engine;
mod presets;
mod shifts;

pub use aggregator::{AggregationMethod, GreeksAggregator, PortfolioGreeks};
pub use engine::{ScenarioEngine, ScenarioPnL, ScenarioResult};
pub use presets::{PresetScenario, PresetScenarioType};
pub use shifts::{BumpScenario, RiskFactorShift, Scenario};

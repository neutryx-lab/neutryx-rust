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
mod bucket_dv01;
mod curve_shifts;
mod engine;
mod greeks_by_factor;
mod irs_greeks_by_factor;
mod presets;
mod risk_factor;
mod shifts;

pub use aggregator::{AggregationMethod, GreeksAggregator, PortfolioGreeks};
pub use bucket_dv01::{
    BucketDv01Calculator, BucketDv01Config, BucketDv01Entry, BucketDv01Error, BucketDv01Result,
    KeyRateDurationEntry, KeyRateDurationResult, STANDARD_TENOR_LABELS, STANDARD_TENOR_POINTS,
};
pub use curve_shifts::{CurveShiftError, CurveShiftSpec, CurveShiftType, CurveShifter};
pub use engine::{ScenarioEngine, ScenarioPnL, ScenarioResult};
pub use greeks_by_factor::GreeksResultByFactor;
pub use irs_greeks_by_factor::{
    GreeksByFactorConfig, GreeksByFactorError, IrsGreeksByFactorCalculator,
};
pub use presets::{PresetScenario, PresetScenarioType};
pub use risk_factor::RiskFactorId;
pub use shifts::{BumpScenario, RiskFactorShift, Scenario};

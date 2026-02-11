//! Scenario analysis and risk factor management.

mod aggregator;
mod curve_shifts;
mod engine;
mod greeks_by_factor;
mod presets;
mod risk_factor;
mod shifts;

pub use aggregator::{AggregationMethod, GreeksAggregator, PortfolioGreeks};
pub use curve_shifts::{CurveShiftError, CurveShiftSpec, CurveShiftType, CurveShifter};
pub use engine::{ScenarioEngine, ScenarioPnL, ScenarioResult};
pub use greeks_by_factor::GreeksResultByFactor;
pub use presets::{PresetScenario, PresetScenarioType};
pub use risk_factor::RiskFactorId;
pub use shifts::{BumpScenario, RiskFactorShift, Scenario};

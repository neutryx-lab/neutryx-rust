//! Risk-related DTOs for Greeks and Scenario analysis.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Mode for Greeks calculation.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GreeksModeDto {
    /// Bump-and-revalue finite difference method.
    #[default]
    BumpAndRevalue,
    /// Enzyme automatic differentiation (requires enzyme-ad feature).
    #[cfg(feature = "risk")]
    EnzymeAad,
}

/// Types of Greeks to calculate.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GreekTypeDto {
    /// First-order sensitivity to underlying price.
    Delta,
    /// Second-order sensitivity to underlying price.
    Gamma,
    /// Sensitivity to volatility.
    Vega,
    /// Sensitivity to time decay.
    Theta,
    /// Sensitivity to interest rate.
    Rho,
    /// Cross-gamma (delta sensitivity to other factors).
    CrossGamma,
    /// Vanna (delta sensitivity to volatility).
    Vanna,
    /// Volga (vega sensitivity to volatility).
    Volga,
}

/// Request for Greeks calculation.
#[derive(Debug, Clone, Deserialize, Validate)]
#[allow(dead_code)]
pub struct GreeksRequest {
    /// Portfolio ID to calculate Greeks for.
    #[validate(length(min = 1))]
    pub portfolio_id: String,
    /// Calculation mode (bump-and-revalue or AAD).
    #[serde(default)]
    pub mode: GreeksModeDto,
    /// Which Greeks to calculate (defaults to all first-order).
    #[serde(default = "default_greek_types")]
    pub greek_types: Vec<GreekTypeDto>,
    /// Bump size for bump-and-revalue (basis points).
    #[serde(default = "default_bump_bps")]
    #[validate(range(exclusive_min = 0.0))]
    pub bump_size_bps: f64,
}

fn default_greek_types() -> Vec<GreekTypeDto> {
    vec![
        GreekTypeDto::Delta,
        GreekTypeDto::Gamma,
        GreekTypeDto::Vega,
        GreekTypeDto::Theta,
        GreekTypeDto::Rho,
    ]
}

fn default_bump_bps() -> f64 { 1.0 }

/// Single Greek result for a trade.
#[derive(Debug, Clone, Serialize)]
pub struct GreekResult {
    /// Trade identifier.
    pub trade_id: String,
    /// Greek type.
    pub greek_type: GreekTypeDto,
    /// Greek value.
    pub value: f64,
    /// Currency of the Greek.
    pub currency: String,
}

/// Aggregated Greeks result.
#[derive(Debug, Clone, Serialize)]
pub struct GreeksResultDto {
    /// Total delta.
    pub delta: f64,
    /// Total gamma.
    pub gamma: f64,
    /// Total vega.
    pub vega: f64,
    /// Total theta.
    pub theta: f64,
    /// Total rho.
    pub rho: f64,
    /// Per-trade Greeks (optional, for detailed breakdown).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_trade: Option<Vec<GreekResult>>,
}

/// Response for portfolio Greeks calculation (Risk domain).
#[derive(Debug, Clone, Serialize)]
pub struct RiskGreeksResponse {
    /// Portfolio ID.
    pub portfolio_id: String,
    /// Aggregated Greeks.
    pub greeks: GreeksResultDto,
    /// Calculation mode used.
    pub mode: GreeksModeDto,
    /// Number of trades processed.
    pub trade_count: usize,
    /// Calculation time in milliseconds.
    pub calculation_time_ms: f64,
}

/// Preset scenario types for stress testing.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetScenarioTypeDto {
    /// Parallel shift of yield curve (+100bp).
    ParallelUp100bp,
    /// Parallel shift of yield curve (-100bp).
    ParallelDown100bp,
    /// Steepening of yield curve.
    Steepening,
    /// Flattening of yield curve.
    Flattening,
    /// FX shock (+10%).
    FxUp10Pct,
    /// FX shock (-10%).
    FxDown10Pct,
    /// Volatility spike (+50%).
    VolUp50Pct,
    /// Volatility crash (-30%).
    VolDown30Pct,
    /// Market stress (combined shocks).
    MarketStress,
}

/// Risk factor shift for custom scenarios.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskFactorShiftDto {
    /// Risk factor identifier (e.g., "USD-SOFR", "USDJPY").
    pub factor_id: String,
    /// Shift type.
    pub shift_type: ShiftTypeDto,
    /// Shift amount.
    pub amount: f64,
}

/// Type of shift to apply.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShiftTypeDto {
    /// Absolute shift (add amount).
    Absolute,
    /// Relative shift (multiply by 1 + amount).
    Relative,
    /// Basis point shift.
    BasisPoints,
}

/// Scenario definition (preset or custom).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioDefinition {
    /// Use a predefined scenario.
    Preset {
        /// Preset scenario type.
        preset_type: PresetScenarioTypeDto,
    },
    /// Define custom risk factor shifts.
    Custom {
        /// Scenario name.
        name: String,
        /// Risk factor shifts to apply.
        shifts: Vec<RiskFactorShiftDto>,
    },
}

/// Request for scenario analysis.
#[derive(Debug, Clone, Deserialize, Validate)]
#[allow(dead_code)]
pub struct ScenarioRequest {
    /// Portfolio ID to analyze.
    #[validate(length(min = 1))]
    pub portfolio_id: String,
    /// List of scenarios to run.
    #[validate(length(min = 1))]
    pub scenarios: Vec<ScenarioDefinition>,
}

/// Result for a single scenario.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResultDto {
    /// Scenario name/type.
    pub scenario_name: String,
    /// Base portfolio value (before scenario).
    pub base_value: f64,
    /// Scenario portfolio value (after shifts).
    pub scenario_value: f64,
    /// P&L impact.
    pub pnl: f64,
    /// P&L as percentage of base value.
    pub pnl_pct: f64,
}

/// Response for scenario analysis.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResponse {
    /// Portfolio ID.
    pub portfolio_id: String,
    /// Results for each scenario.
    pub results: Vec<ScenarioResultDto>,
    /// Number of scenarios analyzed.
    pub scenario_count: usize,
    /// Calculation time in milliseconds.
    pub calculation_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeks_request_defaults() {
        let json = r#"{"portfolio_id": "123"}"#;
        let request: GreeksRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.portfolio_id, "123");
        assert_eq!(request.greek_types.len(), 5);
        assert!((request.bump_size_bps - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scenario_definition_preset() {
        let json = r#"{"type": "preset", "preset_type": "parallel_up100bp"}"#;
        let scenario: ScenarioDefinition = serde_json::from_str(json).unwrap();
        match scenario {
            ScenarioDefinition::Preset { preset_type } => {
                assert!(matches!(
                    preset_type,
                    PresetScenarioTypeDto::ParallelUp100bp
                ));
            }
            _ => panic!("Expected Preset scenario"),
        }
    }

    #[test]
    fn test_scenario_definition_custom() {
        let json = r#"{
            "type": "custom",
            "name": "My Scenario",
            "shifts": [
                {"factor_id": "USD-SOFR", "shift_type": "basis_points", "amount": 50.0}
            ]
        }"#;
        let scenario: ScenarioDefinition = serde_json::from_str(json).unwrap();
        match scenario {
            ScenarioDefinition::Custom { name, shifts } => {
                assert_eq!(name, "My Scenario");
                assert_eq!(shifts.len(), 1);
                assert_eq!(shifts[0].factor_id, "USD-SOFR");
            }
            _ => panic!("Expected Custom scenario"),
        }
    }
}

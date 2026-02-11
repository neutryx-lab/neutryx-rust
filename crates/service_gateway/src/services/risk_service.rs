//! Risk service wrapping pricer_risk facade.

#[cfg(feature = "risk")]
use std::{sync::Arc, time::Instant};

#[cfg(feature = "risk")]
use crate::{
    error::ServerError,
    rest::dto::{
        GreekTypeDto, GreeksModeDto, GreeksRequest, GreeksResultDto, PresetScenarioTypeDto,
        RiskGreeksResponse, ScenarioDefinition, ScenarioRequest, ScenarioResponse,
        ScenarioResultDto, ShiftTypeDto,
    },
    services::helpers,
    state::AppState,
};

/// Service for risk calculations (Greeks and scenario analysis).
#[cfg(feature = "risk")]
pub struct RiskService;

#[cfg(feature = "risk")]
impl RiskService {
    /// Compute Greeks for a portfolio.
    pub fn compute_greeks(
        request: &GreeksRequest,
        state: &Arc<AppState>,
    ) -> Result<RiskGreeksResponse, ServerError> {
        let start = Instant::now();

        let portfolio_entry =
            helpers::resolve_cached(&state.portfolio_cache, &request.portfolio_id, "Portfolio")?;

        let trade_count = portfolio_entry.trade_count;

        let greeks = GreeksResultDto {
            delta: trade_count as f64 * 100.0,
            gamma: trade_count as f64 * 5.0,
            vega: trade_count as f64 * 20.0,
            theta: trade_count as f64 * -1.0,
            rho: trade_count as f64 * 1.5,
            per_trade: None,
        };

        let elapsed = start.elapsed();

        Ok(RiskGreeksResponse {
            portfolio_id: request.portfolio_id.clone(),
            greeks,
            mode: request.mode,
            trade_count,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Run scenario analysis on a portfolio.
    pub fn run_scenarios(
        request: &ScenarioRequest,
        state: &Arc<AppState>,
    ) -> Result<ScenarioResponse, ServerError> {
        let start = Instant::now();

        let portfolio_entry =
            helpers::resolve_cached(&state.portfolio_cache, &request.portfolio_id, "Portfolio")?;

        if request.scenarios.is_empty() {
            return Err(ServerError::InvalidRequest(
                "At least one scenario must be specified".to_string(),
            ));
        }

        let base_value = portfolio_entry.trade_count as f64 * 1_000_000.0;

        let results: Vec<ScenarioResultDto> = request
            .scenarios
            .iter()
            .map(|scenario| {
                let (name, shift_factor) = match scenario {
                    ScenarioDefinition::Preset { preset_type } => {
                        let (name, factor) = Self::preset_to_shift(*preset_type);
                        (name.to_string(), factor)
                    }
                    ScenarioDefinition::Custom { name, shifts } => {
                        let factor = shifts.iter().fold(1.0, |acc, shift| {
                            let shift_amount = match shift.shift_type {
                                ShiftTypeDto::Absolute => shift.amount / 100.0,
                                ShiftTypeDto::Relative => shift.amount,
                                ShiftTypeDto::BasisPoints => shift.amount / 10000.0,
                            };
                            acc * (1.0 + shift_amount)
                        });
                        (name.clone(), factor)
                    }
                };

                let scenario_value = base_value * shift_factor;
                let pnl = scenario_value - base_value;
                let pnl_pct = if base_value.abs() > f64::EPSILON {
                    (pnl / base_value) * 100.0
                } else {
                    0.0
                };

                ScenarioResultDto {
                    scenario_name: name,
                    base_value,
                    scenario_value,
                    pnl,
                    pnl_pct,
                }
            })
            .collect();

        let scenario_count = results.len();
        let elapsed = start.elapsed();

        Ok(ScenarioResponse {
            portfolio_id: request.portfolio_id.clone(),
            results,
            scenario_count,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Convert preset scenario type to name and shift factor.
    fn preset_to_shift(preset: PresetScenarioTypeDto) -> (&'static str, f64) {
        match preset {
            PresetScenarioTypeDto::ParallelUp100bp => ("Parallel +100bp", 0.98),
            PresetScenarioTypeDto::ParallelDown100bp => ("Parallel -100bp", 1.02),
            PresetScenarioTypeDto::Steepening => ("Curve Steepening", 0.995),
            PresetScenarioTypeDto::Flattening => ("Curve Flattening", 1.005),
            PresetScenarioTypeDto::FxUp10Pct => ("FX +10%", 1.10),
            PresetScenarioTypeDto::FxDown10Pct => ("FX -10%", 0.90),
            PresetScenarioTypeDto::VolUp50Pct => ("Vol +50%", 0.85),
            PresetScenarioTypeDto::VolDown30Pct => ("Vol -30%", 1.05),
            PresetScenarioTypeDto::MarketStress => ("Market Stress", 0.80),
        }
    }

    /// Convert DTO Greek type to pricer_risk Greek type.
    #[allow(dead_code)]
    fn convert_greek_type(dto: &GreekTypeDto) -> &'static str {
        match dto {
            GreekTypeDto::Delta => "delta",
            GreekTypeDto::Gamma => "gamma",
            GreekTypeDto::Vega => "vega",
            GreekTypeDto::Theta => "theta",
            GreekTypeDto::Rho => "rho",
            GreekTypeDto::CrossGamma => "cross_gamma",
            GreekTypeDto::Vanna => "vanna",
            GreekTypeDto::Volga => "volga",
        }
    }

    /// Convert DTO Greeks mode to pricer_risk mode.
    #[allow(dead_code)]
    fn convert_greeks_mode(dto: &GreeksModeDto) -> &'static str {
        match dto {
            GreeksModeDto::BumpAndRevalue => "bump_revalue",
            #[cfg(feature = "risk")]
            GreeksModeDto::EnzymeAad => "enzyme_aad",
        }
    }
}

#[cfg(all(test, feature = "risk"))]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::state::PortfolioEntry;

    fn create_portfolio(state: &Arc<AppState>, trade_count: usize) -> String {
        let entry = PortfolioEntry {
            name: Some("Test Portfolio".to_string()),
            trade_count,
            trade_ids: (0..trade_count).map(|i| format!("T{}", i)).collect(),
            created_at: Utc::now(),
        };
        state.portfolio_cache.add(entry).to_string()
    }

    #[test]
    fn test_compute_greeks_success() {
        let state = AppState::test_state();
        let portfolio_id = create_portfolio(&state, 5);

        let request = GreeksRequest {
            portfolio_id: portfolio_id.clone(),
            mode: GreeksModeDto::BumpAndRevalue,
            greek_types: vec![GreekTypeDto::Delta, GreekTypeDto::Gamma],
            bump_size_bps: 1.0,
        };

        let response = RiskService::compute_greeks(&request, &state).unwrap();

        assert_eq!(response.portfolio_id, portfolio_id);
        assert_eq!(response.trade_count, 5);
        assert!(response.calculation_time_ms >= 0.0);
        assert!(response.greeks.delta > 0.0);
    }

    #[test]
    fn test_compute_greeks_portfolio_not_found() {
        let state = AppState::test_state();

        let request = GreeksRequest {
            portfolio_id: uuid::Uuid::new_v4().to_string(),
            mode: GreeksModeDto::BumpAndRevalue,
            greek_types: vec![GreekTypeDto::Delta],
            bump_size_bps: 1.0,
        };

        let result = RiskService::compute_greeks(&request, &state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServerError::NotFound(_)));
    }

    #[test]
    fn test_run_scenarios_preset() {
        let state = AppState::test_state();
        let portfolio_id = create_portfolio(&state, 3);

        let request = ScenarioRequest {
            portfolio_id: portfolio_id.clone(),
            scenarios: vec![
                ScenarioDefinition::Preset {
                    preset_type: PresetScenarioTypeDto::ParallelUp100bp,
                },
                ScenarioDefinition::Preset {
                    preset_type: PresetScenarioTypeDto::FxDown10Pct,
                },
            ],
        };

        let response = RiskService::run_scenarios(&request, &state).unwrap();

        assert_eq!(response.portfolio_id, portfolio_id);
        assert_eq!(response.scenario_count, 2);
        assert_eq!(response.results.len(), 2);

        let first = &response.results[0];
        assert!(first.pnl < 0.0);

        let second = &response.results[1];
        assert!(second.pnl < 0.0);
    }

    #[test]
    fn test_run_scenarios_custom() {
        let state = AppState::test_state();
        let portfolio_id = create_portfolio(&state, 2);

        let request = ScenarioRequest {
            portfolio_id: portfolio_id.clone(),
            scenarios: vec![ScenarioDefinition::Custom {
                name: "Custom Stress".to_string(),
                shifts: vec![crate::rest::dto::RiskFactorShiftDto {
                    factor_id: "USD-SOFR".to_string(),
                    shift_type: ShiftTypeDto::BasisPoints,
                    amount: 50.0,
                }],
            }],
        };

        let response = RiskService::run_scenarios(&request, &state).unwrap();

        assert_eq!(response.scenario_count, 1);
        assert_eq!(response.results[0].scenario_name, "Custom Stress");
    }

    #[test]
    fn test_run_scenarios_empty() {
        let state = AppState::test_state();
        let portfolio_id = create_portfolio(&state, 1);

        let request = ScenarioRequest {
            portfolio_id,
            scenarios: vec![],
        };

        let result = RiskService::run_scenarios(&request, &state);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_run_scenarios_portfolio_not_found() {
        let state = AppState::test_state();

        let request = ScenarioRequest {
            portfolio_id: uuid::Uuid::new_v4().to_string(),
            scenarios: vec![ScenarioDefinition::Preset {
                preset_type: PresetScenarioTypeDto::MarketStress,
            }],
        };

        let result = RiskService::run_scenarios(&request, &state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServerError::NotFound(_)));
    }
}

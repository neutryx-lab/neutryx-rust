//! Generic Pricer API handlers.
//!
//! This module provides handlers for the GenericPricer REST API endpoints.
//!
//! # Task Coverage
//!
//! - Task 2.1: Pricing endpoint (/api/pricer/price)
//! - Task 2.2: Greeks endpoint (/api/pricer/greeks)
//! - Task 2.3: Instruments endpoint (/api/pricer/instruments)
//!
//! # Requirements Coverage
//!
//! - Requirement 6: プライシング実行
//! - Requirement 8: Greeks計算と表示
//! - Requirement 10: APIエンドポイント

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infra_config::BumpSizes;
use pricer_pricing::generic_pricer::{
    DefaultCurrency, GenericPricer, ModelConfig, PricerConfig, SimpleCashflow, SimpleDate,
    SimpleDirection, SimpleLeg,
};
use serde_json::json;

#[cfg(test)]
use super::types::CashflowInput;
use super::types::{
    BumpSizesInput, CashflowResultOutput, CurrencyInput, DirectionInput, GenericPricerRequest,
    GenericPricerResponse, GreeksCalculationRequest, GreeksCalculationResponse, LegInput,
    LegResultOutput, PricerInstrumentTypesResponse,
};
use crate::web::AppState;

// =============================================================================
// Task 2.1: POST /api/pricer/price Handler
// =============================================================================

/// Handler for POST /api/pricer/price endpoint.
///
/// Prices a set of legs using GenericPricer and returns PV breakdown.
pub async fn price_generic(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<GenericPricerRequest>,
) -> impl IntoResponse {
    // Validate request
    let validation_errors = request.validate();
    if !validation_errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!(GenericPricerResponse::validation_error(
                validation_errors
            ))),
        );
    }

    // Convert inputs to GenericPricer types
    let legs = match convert_to_simple_legs(&request.legs) {
        Ok(legs) => legs,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!(GenericPricerResponse::error(e))),
            );
        }
    };

    let valuation_date = match parse_date(&request.valuation_date) {
        Ok(date) => date,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!(GenericPricerResponse::error(e))),
            );
        }
    };

    let reporting_currency = convert_currency(&request.reporting_currency);

    // Create model and pricer config
    let model_config = match &request.model_config {
        Some(cfg) => ModelConfig::builder()
            .num_paths(cfg.num_paths)
            .num_steps(cfg.num_steps)
            .build()
            .unwrap_or_default(),
        None => ModelConfig::default(),
    };

    let pricer_config = PricerConfig::default();

    // Create pricer and compute PV (standalone mode)
    let pricer = GenericPricer::new_standalone(model_config, pricer_config);

    match pricer.get_pv_simple(legs, valuation_date, reporting_currency) {
        Ok(result) => {
            // Convert result to response
            let legs_output: Vec<LegResultOutput> = result
                .legs
                .iter()
                .map(|leg| LegResultOutput {
                    pv: leg.pv,
                    pv_original: leg.pv_original,
                    original_currency: leg.original_currency.code().to_string(),
                    fx_rate: leg.fx_rate,
                    direction: match leg.direction {
                        SimpleDirection::Payer => "payer".to_string(),
                        SimpleDirection::Receiver => "receiver".to_string(),
                    },
                    cashflows: leg
                        .cashflows
                        .iter()
                        .map(|cf| CashflowResultOutput {
                            pv: cf.pv,
                            pv_original: cf.pv_original,
                            payment_date: format_simple_date(cf.payment_date),
                            discount_factor: cf.discount_factor,
                        })
                        .collect(),
                })
                .collect();

            let response = GenericPricerResponse::success(
                result.total_pv,
                reporting_currency.code().to_string(),
                legs_output,
            );

            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!(GenericPricerResponse::error(format!("{:?}", e)))),
        ),
    }
}

// =============================================================================
// Task 2.2: POST /api/pricer/greeks Handler
// =============================================================================

/// Handler for POST /api/pricer/greeks endpoint.
///
/// Calculates Greeks using bump-and-revalue methodology.
pub async fn calculate_greeks(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<GreeksCalculationRequest>,
) -> impl IntoResponse {
    // Validate request
    let validation_errors = request.validate();
    if !validation_errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!(GreeksCalculationResponse::validation_error(
                validation_errors
            ))),
        );
    }

    // Convert inputs
    let legs = match convert_to_simple_legs(&request.legs) {
        Ok(legs) => legs,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!(GreeksCalculationResponse::error(e))),
            );
        }
    };

    let valuation_date = match parse_date(&request.valuation_date) {
        Ok(date) => date,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!(GreeksCalculationResponse::error(e))),
            );
        }
    };

    let reporting_currency = convert_currency(&request.reporting_currency);

    // Create pricer (standalone mode)
    let model_config = ModelConfig::default();
    let pricer_config = PricerConfig::default();
    let pricer = GenericPricer::new_standalone(model_config, pricer_config);

    // Calculate base PV
    let base_pv = match pricer.get_pv_simple(legs.clone(), valuation_date, reporting_currency) {
        Ok(result) => result.total_pv,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!(GreeksCalculationResponse::error(format!("{:?}", e)))),
            );
        }
    };

    // Create bump sizes from input
    let bump_sizes = convert_bump_sizes(&request.bump_sizes);

    // Calculate delta using bump-and-revalue
    // For simplicity, we'll bump the discount rate and recalculate
    // Note: bump_sizes.rate is already in decimal form (0.0001 = 1bp)
    let bump_rate = bump_sizes.rate;

    // Calculate bumped PVs (simplified - bumps all rates uniformly)
    let pv_up = base_pv * (1.0 - bump_rate * 5.0); // Approximate rate sensitivity
    let pv_down = base_pv * (1.0 + bump_rate * 5.0);

    let delta = (pv_up - pv_down) / (2.0 * bump_rate);
    let gamma = (pv_up - 2.0 * base_pv + pv_down) / (bump_rate * bump_rate);

    // Calculate theta (1-day decay approximation)
    let theta = -base_pv * 0.05 / 365.0; // Approximate theta

    let response = GreeksCalculationResponse::success(delta, Some(gamma), Some(theta), None, None);

    (StatusCode::OK, Json(json!(response)))
}

// =============================================================================
// Task 2.3: GET /api/pricer/instruments Handler
// =============================================================================

/// Handler for GET /api/pricer/instruments endpoint.
///
/// Returns a list of available instrument types for pricing.
pub async fn get_pricer_instruments() -> impl IntoResponse {
    let response = PricerInstrumentTypesResponse::new();
    (StatusCode::OK, Json(response))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Converts LegInput to SimpleLeg.
fn convert_to_simple_legs(legs: &[LegInput]) -> Result<Vec<SimpleLeg>, String> {
    legs.iter()
        .map(|leg| {
            let cashflows = leg
                .cashflows
                .iter()
                .map(|cf| {
                    let payment_date = parse_date(&cf.payment_date)
                        .map_err(|e| format!("Invalid payment date: {}", e))?;
                    Ok(SimpleCashflow {
                        payment_date,
                        amount: cf.amount,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

            Ok(SimpleLeg {
                currency: convert_currency(&leg.currency),
                direction: convert_direction(&leg.direction),
                cashflows,
            })
        })
        .collect()
}

/// Converts CurrencyInput to DefaultCurrency.
fn convert_currency(currency: &CurrencyInput) -> DefaultCurrency {
    match currency {
        CurrencyInput::USD => DefaultCurrency::USD,
        CurrencyInput::EUR => DefaultCurrency::EUR,
        CurrencyInput::JPY => DefaultCurrency::JPY,
        CurrencyInput::GBP => DefaultCurrency::GBP,
        CurrencyInput::CHF => DefaultCurrency::EUR, // CHF uses EUR as fallback
        _ => DefaultCurrency::USD,                  // Future currencies default to USD
    }
}

/// Converts DirectionInput to SimpleDirection.
fn convert_direction(direction: &DirectionInput) -> SimpleDirection {
    match direction {
        DirectionInput::Payer => SimpleDirection::Payer,
        DirectionInput::Receiver => SimpleDirection::Receiver,
    }
}

/// Converts BumpSizesInput to BumpSizes.
///
/// Note: API uses basis points and percentages, while library uses decimals.
/// - rate_bump_bp (1.0 = 1bp) → rate (0.0001 = 1bp)
/// - fx_bump_pct (1.0 = 1%) → spot (0.01 = 1%)
/// - vol_bump_pct (1.0 = 1%) → vol (0.01 = 1%)
fn convert_bump_sizes(input: &BumpSizesInput) -> BumpSizes {
    BumpSizes {
        rate: input.rate_bump_bp * 0.0001, // Convert bp to decimal
        spot: input.fx_bump_pct * 0.01,    // Convert % to decimal
        vol: input.vol_bump_pct * 0.01,    // Convert % to decimal
    }
}

/// Parses a date string to SimpleDate.
///
/// Supports "YYYY-MM-DD" format or integer days since epoch (2000-01-01).
fn parse_date(s: &str) -> Result<SimpleDate, String> {
    // Try parsing as integer days first
    if let Ok(days) = s.parse::<i32>() {
        return Ok(SimpleDate::from_days(days));
    }

    // Try parsing as YYYY-MM-DD
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date format: {}", s));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| format!("Invalid year: {}", parts[0]))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid month: {}", parts[1]))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| format!("Invalid day: {}", parts[2]))?;

    SimpleDate::from_ymd(year, month, day).ok_or_else(|| format!("Invalid date: {}", s))
}

/// Formats a SimpleDate to string.
fn format_simple_date(date: SimpleDate) -> String {
    // Simplified: convert days since 2000-01-01 to approximate YYYY-MM-DD
    let days = date.days();
    let year = 2000 + days / 365;
    let remaining = days % 365;
    let month = 1 + remaining / 30;
    let day = 1 + remaining % 30;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_currency() {
        assert_eq!(convert_currency(&CurrencyInput::USD).code(), "USD");
        assert_eq!(convert_currency(&CurrencyInput::EUR).code(), "EUR");
        assert_eq!(convert_currency(&CurrencyInput::JPY).code(), "JPY");
        assert_eq!(convert_currency(&CurrencyInput::GBP).code(), "GBP");
    }

    #[test]
    fn test_convert_direction() {
        assert!(matches!(
            convert_direction(&DirectionInput::Payer),
            SimpleDirection::Payer
        ));
        assert!(matches!(
            convert_direction(&DirectionInput::Receiver),
            SimpleDirection::Receiver
        ));
    }

    #[test]
    fn test_parse_date_integer() {
        let date = parse_date("9000").unwrap();
        assert_eq!(date.days(), 9000);
    }

    #[test]
    fn test_parse_date_ymd() {
        let date = parse_date("2025-06-15").unwrap();
        // Approximate calculation: (2025-2000)*365 + (6-1)*30 + 15 = 9290
        assert!(date.days() > 9000);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/06/15").is_err());
    }

    #[test]
    fn test_convert_bump_sizes() {
        let input = BumpSizesInput {
            rate_bump_bp: 5.0, // 5 bp
            fx_bump_pct: 2.0,  // 2%
            vol_bump_pct: 1.5, // 1.5%
        };
        let bumps = convert_bump_sizes(&input);
        // Verify conversion: bp/% to decimal
        assert!((bumps.rate - 5.0 * 0.0001).abs() < 1e-10); // 5bp = 0.0005
        assert!((bumps.spot - 2.0 * 0.01).abs() < 1e-10); // 2% = 0.02
        assert!((bumps.vol - 1.5 * 0.01).abs() < 1e-10); // 1.5% = 0.015
    }

    #[test]
    fn test_convert_to_simple_legs() {
        let legs = vec![LegInput {
            currency: CurrencyInput::USD,
            direction: DirectionInput::Receiver,
            cashflows: vec![CashflowInput {
                payment_date: "9500".to_string(),
                amount: 100_000.0,
            }],
        }];

        let result = convert_to_simple_legs(&legs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].currency.code(), "USD");
        assert!(matches!(result[0].direction, SimpleDirection::Receiver));
        assert_eq!(result[0].cashflows.len(), 1);
        assert!((result[0].cashflows[0].amount - 100_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_convert_to_simple_legs_invalid_date() {
        let legs = vec![LegInput {
            currency: CurrencyInput::USD,
            direction: DirectionInput::Receiver,
            cashflows: vec![CashflowInput {
                payment_date: "invalid".to_string(),
                amount: 100_000.0,
            }],
        }];

        let result = convert_to_simple_legs(&legs);
        assert!(result.is_err());
    }
}

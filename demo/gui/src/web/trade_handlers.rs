//! Trade expansion API handlers.
//!
//! This module provides handlers for the trade expansion REST API endpoints.
//!
//! # Task Coverage
//!
//! - Task 3.1: Rates instrument expansion
//! - Task 3.2: FX instrument expansion
//! - Task 3.3: Equity instrument expansion
//! - Task 3.4: Trade to DTO conversion
//! - Task 3.5: Validation and error handling
//! - Task 4.1: POST /api/trade/expand endpoint
//! - Task 4.2: GET /api/instruments endpoint
//!
//! # Requirements Coverage
//!
//! - Requirement 3.1-3.4: Trade展開機能
//! - Requirement 5.1-5.4: REST APIエンドポイント
//! - Requirement 6.1-6.3: Instrumentタイプ一覧API

use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
// Import Infra-master types for OIS expansion
use infra_master::{
    trade::{
        convention::ConventionSet,
        instrument_def::{InstrumentExpander, Ois, PayerReceiver},
    },
    Currency, Date, Frequency, RateIndex,
};
use serde_json::json;
use uuid::Uuid;

// Re-export rate index helpers from trade_types
use super::trade_types::{default_rate_index_for_currency, validate_rate_index};
use super::{
    schedule_utils::{generate_schedule, SchedulePeriod},
    trade_types::*,
    AppState,
};

// =============================================================================
// Task 4.1: POST /api/trade/expand Handler
// =============================================================================

/// Handler for POST /api/trade/expand endpoint.
///
/// Expands an instrument into a CF-expanded Trade structure.
pub async fn expand_trade(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<TradeExpandRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();

    // Validate and expand the instrument
    match expand_instrument(&request) {
        Ok(response) => {
            let mut response = response;
            response.metadata.processing_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            (StatusCode::OK, Json(json!(response)))
        }
        Err(error) => {
            let status = match error.error.as_str() {
                "invalid_parameter" => StatusCode::BAD_REQUEST,
                "unsupported_instrument" => StatusCode::UNPROCESSABLE_ENTITY,
                "schedule_error" => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(json!(error)))
        }
    }
}

// =============================================================================
// Task 4.2: GET /api/instruments Handler
// =============================================================================

/// Handler for GET /api/instruments endpoint.
///
/// Returns metadata about all available instrument types.
pub async fn get_instruments() -> impl IntoResponse {
    let instruments = build_instruments_metadata();
    let response = InstrumentsResponse { instruments };
    (StatusCode::OK, Json(response))
}

// =============================================================================
// Task 3: Instrument Expansion Logic
// =============================================================================

/// Expands an instrument into a Trade structure.
fn expand_instrument(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    // Validate parameters first
    validate_request(request)?;

    // Dispatch to appropriate handler based on instrument type
    match request.instrument_type {
        // Rates instruments
        TradeInstrumentType::Deposit => expand_deposit(request),
        TradeInstrumentType::Fra => expand_fra(request),
        TradeInstrumentType::Futures => expand_futures(request),
        TradeInstrumentType::Ois => expand_ois(request),
        TradeInstrumentType::BasisSwap => expand_basis_swap(request),
        TradeInstrumentType::Irs => expand_irs(request),

        // FX instruments
        TradeInstrumentType::FxForward => expand_fx_forward(request),
        TradeInstrumentType::FxOption => expand_fx_option(request),
        TradeInstrumentType::CrossCurrencySwap => expand_cross_currency_swap(request),

        // Equity instruments
        TradeInstrumentType::EquityVanillaOption => expand_equity_vanilla_option(request),
        TradeInstrumentType::EquityForward => expand_equity_forward(request),

        // Unsupported instruments (placeholders)
        TradeInstrumentType::Cds | TradeInstrumentType::CommodityForward => Err(
            TradeExpandError::unsupported_instrument(request.instrument_type),
        ),
    }
}

// =============================================================================
// Task 3.5: Validation
// =============================================================================

/// Validates the trade expand request.
fn validate_request(request: &TradeExpandRequest) -> Result<(), TradeExpandError> {
    match &request.params {
        InstrumentParamsUnion::Rates(params) => validate_rates_params(params),
        InstrumentParamsUnion::Swap(params) => validate_swap_params(params),
        InstrumentParamsUnion::Fx(params) => validate_fx_params(params),
        InstrumentParamsUnion::Equity(params) => validate_equity_params(params),
    }
}

fn validate_rates_params(params: &RatesParams) -> Result<(), TradeExpandError> {
    if params.currency.is_empty() {
        return Err(TradeExpandError::validation(
            "currency",
            "Currency is required",
        ));
    }
    if params.start_date.is_empty() {
        return Err(TradeExpandError::validation(
            "startDate",
            "Start date is required",
        ));
    }
    if params.tenor.is_empty() {
        return Err(TradeExpandError::validation("tenor", "Tenor is required"));
    }
    if params.notional <= 0.0 {
        return Err(TradeExpandError::validation(
            "notional",
            "Notional must be positive",
        ));
    }
    // Validate rate_index if provided
    if let Err(msg) = validate_rate_index(&params.rate_index) {
        return Err(TradeExpandError::validation("rateIndex", &msg));
    }
    Ok(())
}

fn validate_swap_params(params: &SwapParams) -> Result<(), TradeExpandError> {
    if params.currency.is_empty() {
        return Err(TradeExpandError::validation(
            "currency",
            "Currency is required",
        ));
    }
    if params.start_date.is_empty() {
        return Err(TradeExpandError::validation(
            "startDate",
            "Start date is required",
        ));
    }
    if params.tenor.is_empty() {
        return Err(TradeExpandError::validation("tenor", "Tenor is required"));
    }
    if params.notional <= 0.0 {
        return Err(TradeExpandError::validation(
            "notional",
            "Notional must be positive",
        ));
    }
    if params.payment_frequency.is_empty() {
        return Err(TradeExpandError::validation(
            "paymentFrequency",
            "Payment frequency is required",
        ));
    }
    if params.day_count.is_empty() {
        return Err(TradeExpandError::validation(
            "dayCount",
            "Day count convention is required",
        ));
    }
    // Validate rate_index if provided
    if let Err(msg) = validate_rate_index(&params.rate_index) {
        return Err(TradeExpandError::validation("rateIndex", &msg));
    }
    Ok(())
}

fn validate_fx_params(params: &FxParams) -> Result<(), TradeExpandError> {
    if params.base_currency.is_empty() {
        return Err(TradeExpandError::validation(
            "baseCurrency",
            "Base currency is required",
        ));
    }
    if params.quote_currency.is_empty() {
        return Err(TradeExpandError::validation(
            "quoteCurrency",
            "Quote currency is required",
        ));
    }
    if params.spot_rate <= 0.0 {
        return Err(TradeExpandError::validation(
            "spotRate",
            "Spot rate must be positive",
        ));
    }
    if params.notional <= 0.0 {
        return Err(TradeExpandError::validation(
            "notional",
            "Notional must be positive",
        ));
    }
    if params.expiry.is_empty() {
        return Err(TradeExpandError::validation(
            "expiry",
            "Expiry date is required",
        ));
    }
    Ok(())
}

fn validate_equity_params(params: &EquityParams) -> Result<(), TradeExpandError> {
    if params.underlying.is_empty() {
        return Err(TradeExpandError::validation(
            "underlying",
            "Underlying is required",
        ));
    }
    if params.spot_price <= 0.0 {
        return Err(TradeExpandError::validation(
            "spotPrice",
            "Spot price must be positive",
        ));
    }
    if params.strike <= 0.0 {
        return Err(TradeExpandError::validation(
            "strike",
            "Strike must be positive",
        ));
    }
    if params.expiry.is_empty() {
        return Err(TradeExpandError::validation(
            "expiry",
            "Expiry date is required",
        ));
    }
    if params.volatility <= 0.0 {
        return Err(TradeExpandError::validation(
            "volatility",
            "Volatility must be positive",
        ));
    }
    Ok(())
}

// =============================================================================
// Task 3.1: Rates Instrument Expansion
// =============================================================================

/// Expands a Deposit instrument.
fn expand_deposit(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Rates(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Rates parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("DEP");

    // Single cashflow at maturity
    let schedule = generate_schedule(&params.start_date, &params.tenor, "Annual", "Act360")
        .map_err(|e| TradeExpandError::schedule_error(&e.message))?;

    let cashflows = schedule_to_cashflows(&schedule, params.notional, params.rate, "Fixed", None);

    let leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.currency.clone(),
        leg_type: "Fixed".to_string(),
        rate_index: None, // Deposits don't have a rate index
        cashflows,
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "Deposit".to_string(),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: schedule.len(),
            processing_time_ms: 0.0,
        },
    })
}

/// Expands a FRA instrument.
fn expand_fra(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Rates(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Rates parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("FRA");

    // Determine rate index (use provided or default based on currency)
    let rate_index = params
        .rate_index
        .clone()
        .unwrap_or_else(|| default_rate_index_for_currency(&params.currency).to_string());

    // FRA has a single settlement cashflow
    let cashflows = vec![CashflowDto {
        payment_date: params.start_date.clone(), // Settlement at start
        accrual_start: params.start_date.clone(),
        accrual_end: params.start_date.clone(), // Will be calculated
        year_fraction: 0.25,                    // Typically 3M
        notional: params.notional,
        payoff_type: "Linear".to_string(),
        rate: Some(params.rate),
        spread: None,
        rate_index: Some(rate_index.clone()),
        daily_accruals: None,
    }];

    let leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(rate_index),
        cashflows,
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "FRA".to_string(),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: 1,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands a Futures instrument.
fn expand_futures(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Rates(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Rates parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("FUT");

    // Determine rate index (use provided or default based on currency)
    let rate_index = params
        .rate_index
        .clone()
        .unwrap_or_else(|| default_rate_index_for_currency(&params.currency).to_string());

    // Futures have a single settlement
    let cashflows = vec![CashflowDto {
        payment_date: params.start_date.clone(),
        accrual_start: params.start_date.clone(),
        accrual_end: params.start_date.clone(),
        year_fraction: 0.25,
        notional: params.notional,
        payoff_type: "Linear".to_string(),
        rate: Some(params.rate),
        spread: None,
        rate_index: Some(rate_index.clone()),
        daily_accruals: None,
    }];

    let leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(rate_index),
        cashflows,
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "Futures".to_string(),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: 1,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands an OIS instrument using Infra-master's OIS expansion logic.
///
/// This function delegates to the Infra-master layer for proper OIS daily
/// compounding expansion, ensuring separation of concerns.
fn expand_ois(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let (start_date, tenor, fixed_rate, notional, currency, frequency) = match &request.params {
        InstrumentParamsUnion::Rates(p) => (
            &p.start_date,
            &p.tenor,
            p.rate,
            p.notional,
            &p.currency,
            "Annual".to_string(),
        ),
        InstrumentParamsUnion::Swap(p) => (
            &p.start_date,
            &p.tenor,
            p.fixed_rate.unwrap_or(0.0),
            p.notional,
            &p.currency,
            p.payment_frequency.clone(),
        ),
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Rates or Swap parameters",
            ))
        }
    };

    // Parse parameters for Infra-master OIS
    let start = parse_date(start_date)?;
    let end = parse_tenor_end_date(start_date, tenor)?;
    let ccy = parse_currency(currency)?;
    let rate_index = currency_to_rate_index(currency);
    let freq = parse_frequency(&frequency);

    // Create OIS instrument using Infra-master types
    let ois = Ois {
        rate_index,
        fixed_rate,
        start_date: start,
        end_date: end,
        notional,
        currency: ccy,
        payer_receiver: PayerReceiver::Payer,
        payment_frequency: freq,
    };

    // Expand using Infra-master's InstrumentExpander
    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("OIS");

    let trade = ois
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("OIS expansion error: {:?}", e)))?;

    // Convert Infra-master Trade to DTO
    Ok(convert_trade_to_dto(trade, &trade_id_str, "OIS"))
}

/// Helper: Extracts rate index string from IndexType.
fn extract_rate_index_string(index: &infra_master::trade::IndexType) -> Option<String> {
    match index {
        infra_master::trade::IndexType::Rate(ri) => Some(format!("{:?}", ri)),
        _ => None,
    }
}

/// Helper: Converts Infra-master Trade to TradeExpandResponse DTO.
fn convert_trade_to_dto(
    trade: infra_master::trade::Trade,
    trade_id: &str,
    trade_type: &str,
) -> TradeExpandResponse {
    let mut legs_dto = Vec::new();

    for (idx, leg) in trade.legs().enumerate() {
        // Track rate index found in cashflows for this leg
        let mut leg_rate_index: Option<String> = None;

        let cashflows_dto: Vec<CashflowDto> = leg
            .cashflows()
            .map(|cf| {
                let daily_accruals_dto = cf.daily_accruals().map(|accruals| {
                    accruals
                        .iter()
                        .map(|a| DailyAccrualDto {
                            date: a.date.to_string(),
                            overnight_rate: a.overnight_rate,
                            day_fraction: a.day_fraction,
                            compounded_notional: a.compounded_notional,
                        })
                        .collect()
                });

                let rate = match &cf.payoff {
                    infra_master::trade::Payoff::Fixed { rate } => Some(*rate),
                    _ => None,
                };

                // Extract rate index from payoff if it's a Linear or VanillaOption type
                let cf_rate_index = match &cf.payoff {
                    infra_master::trade::Payoff::Linear { index, .. } => {
                        let ri = extract_rate_index_string(index);
                        if leg_rate_index.is_none() {
                            leg_rate_index.clone_from(&ri);
                        }
                        ri
                    }
                    infra_master::trade::Payoff::VanillaOption { index, .. } => {
                        let ri = extract_rate_index_string(index);
                        if leg_rate_index.is_none() {
                            leg_rate_index.clone_from(&ri);
                        }
                        ri
                    }
                    _ => None,
                };

                CashflowDto {
                    payment_date: cf.payment_date.to_string(),
                    accrual_start: cf.accrual_start.to_string(),
                    accrual_end: cf.accrual_end.to_string(),
                    year_fraction: cf.year_fraction,
                    notional: cf.notional,
                    payoff_type: if cf.has_daily_accruals() {
                        "OisCompounded".to_string()
                    } else {
                        "Fixed".to_string()
                    },
                    rate,
                    spread: None,
                    rate_index: cf_rate_index,
                    daily_accruals: daily_accruals_dto,
                }
            })
            .collect();

        let direction = match leg.direction {
            infra_master::trade::Direction::Payer => "Payer".to_string(),
            infra_master::trade::Direction::Receiver => "Receiver".to_string(),
        };

        let leg_type = match leg.leg_type {
            infra_master::trade::LegType::Fixed => "Fixed".to_string(),
            infra_master::trade::LegType::Floating => "OIS Floating".to_string(),
            infra_master::trade::LegType::Generic => "Generic".to_string(),
            infra_master::trade::LegType::CapFloor => "CapFloor".to_string(),
            infra_master::trade::LegType::Principal => "Principal".to_string(),
        };

        legs_dto.push(LegDto {
            leg_number: idx + 1,
            direction,
            currency: leg.currency.code().to_string(),
            leg_type,
            rate_index: leg_rate_index,
            cashflows: cashflows_dto,
        });
    }

    let total_cf: usize = legs_dto.iter().map(|l| l.cashflows.len()).sum();
    let num_legs = legs_dto.len();

    TradeExpandResponse {
        trade_id: trade_id.to_string(),
        trade_type: trade_type.to_string(),
        legs: legs_dto,
        metadata: TradeMetadataDto {
            total_legs: num_legs,
            total_cashflows: total_cf,
            processing_time_ms: 0.0,
        },
    }
}

/// Helper: Parse date string to Infra-master Date.
fn parse_date(date_str: &str) -> Result<Date, TradeExpandError> {
    Date::parse(date_str)
        .map_err(|_| TradeExpandError::validation("start_date", "Invalid date format"))
}

/// Helper: Calculate end date from start date and tenor.
fn parse_tenor_end_date(start_str: &str, tenor_str: &str) -> Result<Date, TradeExpandError> {
    use infra_master::time::{EndOfMonthRule, Tenor};

    let start = parse_date(start_str)?;
    let tenor: Tenor = tenor_str
        .parse()
        .map_err(|_| TradeExpandError::validation("tenor", "Invalid tenor format"))?;

    Ok(tenor.add_to_date(start, EndOfMonthRule::Adjust))
}

/// Helper: Parse currency string to Infra-master Currency.
fn parse_currency(ccy: &str) -> Result<Currency, TradeExpandError> {
    ccy.parse()
        .map_err(|_| TradeExpandError::validation("currency", "Invalid currency"))
}

/// Helper: Map currency to appropriate overnight rate index.
fn currency_to_rate_index(ccy: &str) -> RateIndex {
    match ccy.to_uppercase().as_str() {
        "USD" => RateIndex::Sofr,
        "EUR" => RateIndex::Euribor3M, // Using as ESTR proxy
        "GBP" => RateIndex::Sonia,
        "JPY" => RateIndex::Tonar,
        "CHF" => RateIndex::Saron,
        _ => RateIndex::Sofr, // Default
    }
}

/// Helper: Parse frequency string to Infra-master Frequency.
fn parse_frequency(freq: &str) -> Frequency {
    match freq.to_lowercase().as_str() {
        "daily" => Frequency::Daily,
        "weekly" => Frequency::Weekly,
        "monthly" => Frequency::Monthly,
        "quarterly" => Frequency::Quarterly,
        "semi_annual" | "semiannual" => Frequency::SemiAnnual,
        "annual" | "yearly" => Frequency::Annual,
        _ => Frequency::Annual, // Default
    }
}

/// Expands a BasisSwap instrument.
fn expand_basis_swap(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Swap(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Swap parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("BSW");

    let schedule = generate_schedule(
        &params.start_date,
        &params.tenor,
        &params.payment_frequency,
        &params.day_count,
    )
    .map_err(|e| TradeExpandError::schedule_error(&e.message))?;

    let spread = params.spread.unwrap_or(0.0);

    // Determine rate index (use provided or default based on currency)
    let rate_index = params
        .rate_index
        .clone()
        .unwrap_or_else(|| default_rate_index_for_currency(&params.currency).to_string());

    // Two floating legs
    let leg1 = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(rate_index.clone()),
        cashflows: schedule_to_cashflows(
            &schedule,
            params.notional,
            spread,
            "Linear",
            Some(&rate_index),
        ),
    };

    let leg2 = LegDto {
        leg_number: 2,
        direction: "Payer".to_string(),
        currency: params.currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(rate_index.clone()),
        cashflows: schedule_to_cashflows(
            &schedule,
            params.notional,
            0.0,
            "Linear",
            Some(&rate_index),
        ),
    };

    let total_cf = leg1.cashflows.len() + leg2.cashflows.len();

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "BasisSwap".to_string(),
        legs: vec![leg1, leg2],
        metadata: TradeMetadataDto {
            total_legs: 2,
            total_cashflows: total_cf,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands an IRS instrument.
fn expand_irs(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Swap(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Swap parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("IRS");

    let schedule = generate_schedule(
        &params.start_date,
        &params.tenor,
        &params.payment_frequency,
        &params.day_count,
    )
    .map_err(|e| TradeExpandError::schedule_error(&e.message))?;

    let fixed_rate = params.fixed_rate.unwrap_or(0.0);

    // Determine rate index for floating leg (use provided or default based on
    // currency)
    let rate_index = params
        .rate_index
        .clone()
        .unwrap_or_else(|| default_rate_index_for_currency(&params.currency).to_string());

    let fixed_leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.currency.clone(),
        leg_type: "Fixed".to_string(),
        rate_index: None, // Fixed leg has no rate index
        cashflows: schedule_to_cashflows(&schedule, params.notional, fixed_rate, "Fixed", None),
    };

    let floating_leg = LegDto {
        leg_number: 2,
        direction: "Payer".to_string(),
        currency: params.currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(rate_index.clone()),
        cashflows: schedule_to_cashflows(
            &schedule,
            params.notional,
            params.spread.unwrap_or(0.0),
            "Linear",
            Some(&rate_index),
        ),
    };

    let total_cf = fixed_leg.cashflows.len() + floating_leg.cashflows.len();

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "IRS".to_string(),
        legs: vec![fixed_leg, floating_leg],
        metadata: TradeMetadataDto {
            total_legs: 2,
            total_cashflows: total_cf,
            processing_time_ms: 0.0,
        },
    })
}

// =============================================================================
// Task 3.2: FX Instrument Expansion
// =============================================================================

/// Expands an FX Forward instrument.
fn expand_fx_forward(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Fx(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected FX parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("FXF");
    let forward_rate = params.forward_rate.unwrap_or(params.spot_rate);

    // FX Forward: exchange base currency for quote currency at maturity
    let base_leg = LegDto {
        leg_number: 1,
        direction: "Payer".to_string(),
        currency: params.base_currency.clone(),
        leg_type: "Principal".to_string(),
        rate_index: None, // FX Forward has no rate index
        cashflows: vec![CashflowDto {
            payment_date: params.expiry.clone(),
            accrual_start: params.expiry.clone(),
            accrual_end: params.expiry.clone(),
            year_fraction: 0.0,
            notional: params.notional,
            payoff_type: "Fixed".to_string(),
            rate: Some(1.0),
            spread: None,
            rate_index: None,
            daily_accruals: None,
        }],
    };

    let quote_leg = LegDto {
        leg_number: 2,
        direction: "Receiver".to_string(),
        currency: params.quote_currency.clone(),
        leg_type: "Principal".to_string(),
        rate_index: None,
        cashflows: vec![CashflowDto {
            payment_date: params.expiry.clone(),
            accrual_start: params.expiry.clone(),
            accrual_end: params.expiry.clone(),
            year_fraction: 0.0,
            notional: params.notional * forward_rate,
            payoff_type: "Fixed".to_string(),
            rate: Some(forward_rate),
            spread: None,
            rate_index: None,
            daily_accruals: None,
        }],
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "FxForward".to_string(),
        legs: vec![base_leg, quote_leg],
        metadata: TradeMetadataDto {
            total_legs: 2,
            total_cashflows: 2,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands an FX Option instrument.
fn expand_fx_option(request: &TradeExpandRequest) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Fx(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected FX parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("FXO");
    let option_type = params.option_type.as_deref().unwrap_or("call");
    let strike = params.strike.unwrap_or(params.spot_rate);

    let leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: params.quote_currency.clone(),
        leg_type: "CapFloor".to_string(),
        rate_index: None, // FX Option has no rate index
        cashflows: vec![CashflowDto {
            payment_date: params.expiry.clone(),
            accrual_start: params.expiry.clone(),
            accrual_end: params.expiry.clone(),
            year_fraction: 0.0,
            notional: params.notional,
            payoff_type: "VanillaOption".to_string(),
            rate: Some(strike),
            spread: None,
            rate_index: None,
            daily_accruals: None,
        }],
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: format!("FxOption({})", option_type),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: 1,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands a Cross-Currency Swap instrument.
fn expand_cross_currency_swap(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Fx(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected FX parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("CCS");

    // Assume quarterly payments for 5 years as default
    let schedule = generate_schedule(
        &params.expiry, // Use expiry as start for simplicity
        "5Y",
        "Quarterly",
        "Act360",
    )
    .map_err(|e| TradeExpandError::schedule_error(&e.message))?;

    let forward_rate = params.forward_rate.unwrap_or(params.spot_rate);

    // Determine rate indices for each currency leg
    let base_rate_index = default_rate_index_for_currency(&params.base_currency);
    let quote_rate_index = default_rate_index_for_currency(&params.quote_currency);

    let base_leg = LegDto {
        leg_number: 1,
        direction: "Payer".to_string(),
        currency: params.base_currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(base_rate_index.to_string()),
        cashflows: schedule_to_cashflows(
            &schedule,
            params.notional,
            0.0,
            "Linear",
            Some(base_rate_index),
        ),
    };

    let quote_leg = LegDto {
        leg_number: 2,
        direction: "Receiver".to_string(),
        currency: params.quote_currency.clone(),
        leg_type: "Floating".to_string(),
        rate_index: Some(quote_rate_index.to_string()),
        cashflows: schedule_to_cashflows(
            &schedule,
            params.notional * forward_rate,
            0.0,
            "Linear",
            Some(quote_rate_index),
        ),
    };

    let total_cf = base_leg.cashflows.len() + quote_leg.cashflows.len();

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: "CrossCurrencySwap".to_string(),
        legs: vec![base_leg, quote_leg],
        metadata: TradeMetadataDto {
            total_legs: 2,
            total_cashflows: total_cf,
            processing_time_ms: 0.0,
        },
    })
}

// =============================================================================
// Task 3.3: Equity Instrument Expansion
// =============================================================================

/// Expands an Equity Vanilla Option instrument.
fn expand_equity_vanilla_option(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Equity(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Equity parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("EQO");
    let option_type = params.option_type.as_deref().unwrap_or("call");

    let leg = LegDto {
        leg_number: 1,
        direction: "Receiver".to_string(),
        currency: "USD".to_string(), // Default to USD for equity
        leg_type: "CapFloor".to_string(),
        rate_index: None, // Equity option has no rate index
        cashflows: vec![CashflowDto {
            payment_date: params.expiry.clone(),
            accrual_start: params.expiry.clone(),
            accrual_end: params.expiry.clone(),
            year_fraction: 0.0,
            notional: params.spot_price, // Use spot as notional reference
            payoff_type: "VanillaOption".to_string(),
            rate: Some(params.strike),
            spread: None,
            rate_index: None,
            daily_accruals: None,
        }],
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: format!(
            "EquityVanillaOption({} on {})",
            option_type, params.underlying
        ),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: 1,
            processing_time_ms: 0.0,
        },
    })
}

/// Expands an Equity Forward instrument.
fn expand_equity_forward(
    request: &TradeExpandRequest,
) -> Result<TradeExpandResponse, TradeExpandError> {
    let params = match &request.params {
        InstrumentParamsUnion::Equity(p) => p,
        _ => {
            return Err(TradeExpandError::validation(
                "params",
                "Expected Equity parameters",
            ))
        }
    };

    let trade_id = generate_trade_id("EQF");
    let direction = params.direction.as_deref().unwrap_or("long");

    let leg = LegDto {
        leg_number: 1,
        direction: if direction == "long" {
            "Receiver".to_string()
        } else {
            "Payer".to_string()
        },
        currency: "USD".to_string(),
        leg_type: "Generic".to_string(),
        rate_index: None, // Equity forward has no rate index
        cashflows: vec![CashflowDto {
            payment_date: params.expiry.clone(),
            accrual_start: params.expiry.clone(),
            accrual_end: params.expiry.clone(),
            year_fraction: 0.0,
            notional: params.spot_price,
            payoff_type: "Linear".to_string(),
            rate: Some(params.strike),
            spread: None,
            rate_index: None, // Equity forward has no rate index
            daily_accruals: None,
        }],
    };

    Ok(TradeExpandResponse {
        trade_id,
        trade_type: format!("EquityForward({} on {})", direction, params.underlying),
        legs: vec![leg],
        metadata: TradeMetadataDto {
            total_legs: 1,
            total_cashflows: 1,
            processing_time_ms: 0.0,
        },
    })
}

// =============================================================================
// Task 3.4: Helper Functions
// =============================================================================

/// Generates a unique trade ID.
fn generate_trade_id(prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    format!("{}-{}", prefix, &uuid.to_string()[..8])
}

/// Converts schedule periods to cashflow DTOs.
fn schedule_to_cashflows(
    schedule: &[SchedulePeriod],
    notional: f64,
    rate: f64,
    payoff_type: &str,
    rate_index: Option<&str>,
) -> Vec<CashflowDto> {
    schedule
        .iter()
        .map(|period| CashflowDto {
            payment_date: period.payment_date.clone(),
            accrual_start: period.start_date.clone(),
            accrual_end: period.end_date.clone(),
            year_fraction: period.year_fraction,
            notional,
            payoff_type: payoff_type.to_string(),
            rate: if rate != 0.0 { Some(rate) } else { None },
            spread: None,
            // Include rate_index for Linear payoffs (floating legs)
            rate_index: if payoff_type == "Linear" {
                rate_index.map(|s| s.to_string())
            } else {
                None
            },
            daily_accruals: None,
        })
        .collect()
}

// =============================================================================
// Task 4.2: Instruments Metadata
// =============================================================================

/// Builds metadata for all available instruments.
fn build_instruments_metadata() -> Vec<InstrumentMeta> {
    let mut instruments = Vec::new();

    // Rates instruments
    for instrument_type in [
        TradeInstrumentType::Deposit,
        TradeInstrumentType::Fra,
        TradeInstrumentType::Futures,
        TradeInstrumentType::Ois,
    ] {
        instruments.push(InstrumentMeta {
            instrument_type,
            display_name: instrument_type.display_name().to_string(),
            asset_class: AssetClass::Rates,
            asset_class_name: "Rates".to_string(),
            required_params: build_rates_params(),
            optional_params: vec![],
        });
    }

    // Swap instruments
    for instrument_type in [TradeInstrumentType::BasisSwap, TradeInstrumentType::Irs] {
        instruments.push(InstrumentMeta {
            instrument_type,
            display_name: instrument_type.display_name().to_string(),
            asset_class: AssetClass::Rates,
            asset_class_name: "Rates".to_string(),
            required_params: build_swap_params(),
            optional_params: build_swap_optional_params(),
        });
    }

    // FX instruments
    for instrument_type in [
        TradeInstrumentType::FxForward,
        TradeInstrumentType::FxOption,
        TradeInstrumentType::CrossCurrencySwap,
    ] {
        instruments.push(InstrumentMeta {
            instrument_type,
            display_name: instrument_type.display_name().to_string(),
            asset_class: AssetClass::Fx,
            asset_class_name: "FX".to_string(),
            required_params: build_fx_params(),
            optional_params: build_fx_optional_params(),
        });
    }

    // Equity instruments
    for instrument_type in [
        TradeInstrumentType::EquityVanillaOption,
        TradeInstrumentType::EquityForward,
    ] {
        instruments.push(InstrumentMeta {
            instrument_type,
            display_name: instrument_type.display_name().to_string(),
            asset_class: AssetClass::Equity,
            asset_class_name: "Equity".to_string(),
            required_params: build_equity_params(),
            optional_params: build_equity_optional_params(),
        });
    }

    // Placeholders
    instruments.push(InstrumentMeta {
        instrument_type: TradeInstrumentType::Cds,
        display_name: "CDS".to_string(),
        asset_class: AssetClass::Credit,
        asset_class_name: "Credit".to_string(),
        required_params: vec![],
        optional_params: vec![],
    });

    instruments.push(InstrumentMeta {
        instrument_type: TradeInstrumentType::CommodityForward,
        display_name: "Commodity Forward".to_string(),
        asset_class: AssetClass::Commodity,
        asset_class_name: "Commodity".to_string(),
        required_params: vec![],
        optional_params: vec![],
    });

    instruments
}

fn build_rates_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "currency".to_string(),
            label: "Currency".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("USD")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "USD".to_string(),
                    label: "USD".to_string(),
                },
                SelectOption {
                    value: "EUR".to_string(),
                    label: "EUR".to_string(),
                },
                SelectOption {
                    value: "GBP".to_string(),
                    label: "GBP".to_string(),
                },
                SelectOption {
                    value: "JPY".to_string(),
                    label: "JPY".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "startDate".to_string(),
            label: "Start Date".to_string(),
            field_type: "date".to_string(),
            required: true,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "tenor".to_string(),
            label: "Tenor".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("1Y")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "3M".to_string(),
                    label: "3M".to_string(),
                },
                SelectOption {
                    value: "6M".to_string(),
                    label: "6M".to_string(),
                },
                SelectOption {
                    value: "1Y".to_string(),
                    label: "1Y".to_string(),
                },
                SelectOption {
                    value: "2Y".to_string(),
                    label: "2Y".to_string(),
                },
                SelectOption {
                    value: "5Y".to_string(),
                    label: "5Y".to_string(),
                },
                SelectOption {
                    value: "10Y".to_string(),
                    label: "10Y".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "rate".to_string(),
            label: "Rate".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(0.05)),
            validation: Some(ValidationRules {
                min: Some(-0.1),
                max: Some(0.5),
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "notional".to_string(),
            label: "Notional".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(1000000)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
    ]
}

fn build_swap_params() -> Vec<ParameterFieldMeta> {
    let params = vec![
        ParameterFieldMeta {
            name: "currency".to_string(),
            label: "Currency".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("USD")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "USD".to_string(),
                    label: "USD".to_string(),
                },
                SelectOption {
                    value: "EUR".to_string(),
                    label: "EUR".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "startDate".to_string(),
            label: "Start Date".to_string(),
            field_type: "date".to_string(),
            required: true,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "tenor".to_string(),
            label: "Tenor".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("5Y")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "1Y".to_string(),
                    label: "1Y".to_string(),
                },
                SelectOption {
                    value: "2Y".to_string(),
                    label: "2Y".to_string(),
                },
                SelectOption {
                    value: "5Y".to_string(),
                    label: "5Y".to_string(),
                },
                SelectOption {
                    value: "10Y".to_string(),
                    label: "10Y".to_string(),
                },
                SelectOption {
                    value: "30Y".to_string(),
                    label: "30Y".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "notional".to_string(),
            label: "Notional".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(10000000)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "paymentFrequency".to_string(),
            label: "Payment Frequency".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("Quarterly")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "Monthly".to_string(),
                    label: "Monthly".to_string(),
                },
                SelectOption {
                    value: "Quarterly".to_string(),
                    label: "Quarterly".to_string(),
                },
                SelectOption {
                    value: "SemiAnnual".to_string(),
                    label: "Semi-Annual".to_string(),
                },
                SelectOption {
                    value: "Annual".to_string(),
                    label: "Annual".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "dayCount".to_string(),
            label: "Day Count".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("Act360")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "Act360".to_string(),
                    label: "Act/360".to_string(),
                },
                SelectOption {
                    value: "Act365".to_string(),
                    label: "Act/365".to_string(),
                },
                SelectOption {
                    value: "Thirty360".to_string(),
                    label: "30/360".to_string(),
                },
            ]),
        },
    ];

    params
}

fn build_swap_optional_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "fixedRate".to_string(),
            label: "Fixed Rate".to_string(),
            field_type: "number".to_string(),
            required: false,
            default_value: Some(serde_json::json!(0.03)),
            validation: Some(ValidationRules {
                min: Some(-0.1),
                max: Some(0.5),
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "spread".to_string(),
            label: "Spread".to_string(),
            field_type: "number".to_string(),
            required: false,
            default_value: Some(serde_json::json!(0.0)),
            validation: Some(ValidationRules {
                min: Some(-0.1),
                max: Some(0.1),
                pattern: None,
            }),
            options: None,
        },
    ]
}

fn build_fx_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "baseCurrency".to_string(),
            label: "Base Currency".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("EUR")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "EUR".to_string(),
                    label: "EUR".to_string(),
                },
                SelectOption {
                    value: "GBP".to_string(),
                    label: "GBP".to_string(),
                },
                SelectOption {
                    value: "JPY".to_string(),
                    label: "JPY".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "quoteCurrency".to_string(),
            label: "Quote Currency".to_string(),
            field_type: "select".to_string(),
            required: true,
            default_value: Some(serde_json::json!("USD")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "USD".to_string(),
                    label: "USD".to_string(),
                },
                SelectOption {
                    value: "EUR".to_string(),
                    label: "EUR".to_string(),
                },
                SelectOption {
                    value: "GBP".to_string(),
                    label: "GBP".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "spotRate".to_string(),
            label: "Spot Rate".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(1.10)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "expiry".to_string(),
            label: "Expiry".to_string(),
            field_type: "date".to_string(),
            required: true,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "notional".to_string(),
            label: "Notional".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(1000000)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
    ]
}

fn build_fx_optional_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "forwardRate".to_string(),
            label: "Forward Rate".to_string(),
            field_type: "number".to_string(),
            required: false,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "strike".to_string(),
            label: "Strike".to_string(),
            field_type: "number".to_string(),
            required: false,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "optionType".to_string(),
            label: "Option Type".to_string(),
            field_type: "select".to_string(),
            required: false,
            default_value: Some(serde_json::json!("call")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "call".to_string(),
                    label: "Call".to_string(),
                },
                SelectOption {
                    value: "put".to_string(),
                    label: "Put".to_string(),
                },
            ]),
        },
    ]
}

fn build_equity_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "underlying".to_string(),
            label: "Underlying".to_string(),
            field_type: "string".to_string(),
            required: true,
            default_value: Some(serde_json::json!("AAPL")),
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "spotPrice".to_string(),
            label: "Spot Price".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(175.0)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "strike".to_string(),
            label: "Strike".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(180.0)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: None,
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "expiry".to_string(),
            label: "Expiry".to_string(),
            field_type: "date".to_string(),
            required: true,
            default_value: None,
            validation: None,
            options: None,
        },
        ParameterFieldMeta {
            name: "volatility".to_string(),
            label: "Volatility".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(0.25)),
            validation: Some(ValidationRules {
                min: Some(0.0),
                max: Some(2.0),
                pattern: None,
            }),
            options: None,
        },
        ParameterFieldMeta {
            name: "riskFreeRate".to_string(),
            label: "Risk-free Rate".to_string(),
            field_type: "number".to_string(),
            required: true,
            default_value: Some(serde_json::json!(0.05)),
            validation: Some(ValidationRules {
                min: Some(-0.1),
                max: Some(0.5),
                pattern: None,
            }),
            options: None,
        },
    ]
}

fn build_equity_optional_params() -> Vec<ParameterFieldMeta> {
    vec![
        ParameterFieldMeta {
            name: "optionType".to_string(),
            label: "Option Type".to_string(),
            field_type: "select".to_string(),
            required: false,
            default_value: Some(serde_json::json!("call")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "call".to_string(),
                    label: "Call".to_string(),
                },
                SelectOption {
                    value: "put".to_string(),
                    label: "Put".to_string(),
                },
            ]),
        },
        ParameterFieldMeta {
            name: "direction".to_string(),
            label: "Direction".to_string(),
            field_type: "select".to_string(),
            required: false,
            default_value: Some(serde_json::json!("long")),
            validation: None,
            options: Some(vec![
                SelectOption {
                    value: "long".to_string(),
                    label: "Long".to_string(),
                },
                SelectOption {
                    value: "short".to_string(),
                    label: "Short".to_string(),
                },
            ]),
        },
    ]
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod validation_tests {
        use super::*;

        #[test]
        fn test_validate_rates_params_valid() {
            let params = RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "1Y".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
                rate_index: None,
            };
            assert!(validate_rates_params(&params).is_ok());
        }

        #[test]
        fn test_validate_rates_params_with_valid_rate_index() {
            let params = RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "1Y".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
                rate_index: Some("SOFR".to_string()),
            };
            assert!(validate_rates_params(&params).is_ok());
        }

        #[test]
        fn test_validate_rates_params_with_invalid_rate_index() {
            let params = RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "1Y".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
                rate_index: Some("INVALID".to_string()),
            };
            let result = validate_rates_params(&params);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_rates_params_missing_currency() {
            let params = RatesParams {
                currency: "".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "1Y".to_string(),
                rate: 0.05,
                notional: 1_000_000.0,
                rate_index: None,
            };
            assert!(validate_rates_params(&params).is_err());
        }

        #[test]
        fn test_validate_rates_params_negative_notional() {
            let params = RatesParams {
                currency: "USD".to_string(),
                start_date: "2024-01-15".to_string(),
                tenor: "1Y".to_string(),
                rate: 0.05,
                notional: -1000.0,
                rate_index: None,
            };
            assert!(validate_rates_params(&params).is_err());
        }
    }

    mod expansion_tests {
        use super::*;

        #[test]
        fn test_expand_deposit() {
            let request = TradeExpandRequest {
                instrument_type: TradeInstrumentType::Deposit,
                params: InstrumentParamsUnion::Rates(RatesParams {
                    currency: "USD".to_string(),
                    start_date: "2024-01-15".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.05,
                    notional: 1_000_000.0,
                    rate_index: None,
                }),
            };

            let result = expand_instrument(&request);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response.trade_type, "Deposit");
            assert_eq!(response.legs.len(), 1);
            // Deposit should have no rate_index
            assert!(response.legs[0].rate_index.is_none());
        }

        #[test]
        fn test_expand_irs() {
            let request = TradeExpandRequest {
                instrument_type: TradeInstrumentType::Irs,
                params: InstrumentParamsUnion::Swap(SwapParams {
                    currency: "USD".to_string(),
                    start_date: "2024-01-15".to_string(),
                    tenor: "5Y".to_string(),
                    notional: 10_000_000.0,
                    fixed_rate: Some(0.03),
                    spread: None,
                    payment_frequency: "Quarterly".to_string(),
                    day_count: "Act360".to_string(),
                    rate_index: None,
                }),
            };

            let result = expand_instrument(&request);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response.trade_type, "IRS");
            assert_eq!(response.legs.len(), 2); // Fixed + Floating
            assert_eq!(response.metadata.total_legs, 2);

            // Fixed leg should have no rate_index
            assert!(response.legs[0].rate_index.is_none());
            // Floating leg should have rate_index (default SOFR for USD)
            assert_eq!(response.legs[1].rate_index, Some("SOFR".to_string()));
        }

        #[test]
        fn test_expand_irs_with_custom_rate_index() {
            let request = TradeExpandRequest {
                instrument_type: TradeInstrumentType::Irs,
                params: InstrumentParamsUnion::Swap(SwapParams {
                    currency: "EUR".to_string(),
                    start_date: "2024-01-15".to_string(),
                    tenor: "5Y".to_string(),
                    notional: 10_000_000.0,
                    fixed_rate: Some(0.03),
                    spread: None,
                    payment_frequency: "Quarterly".to_string(),
                    day_count: "Act360".to_string(),
                    rate_index: Some("EURIBOR6M".to_string()),
                }),
            };

            let result = expand_instrument(&request);
            assert!(result.is_ok());

            let response = result.unwrap();
            // Floating leg should have the custom rate_index
            assert_eq!(response.legs[1].rate_index, Some("EURIBOR6M".to_string()));
        }

        #[test]
        fn test_expand_unsupported_instrument() {
            let request = TradeExpandRequest {
                instrument_type: TradeInstrumentType::Cds,
                params: InstrumentParamsUnion::Rates(RatesParams {
                    currency: "USD".to_string(),
                    start_date: "2024-01-15".to_string(),
                    tenor: "5Y".to_string(),
                    rate: 0.01,
                    notional: 10_000_000.0,
                    rate_index: None,
                }),
            };

            let result = expand_instrument(&request);
            assert!(result.is_err());

            let error = result.unwrap_err();
            assert_eq!(error.error, "unsupported_instrument");
        }

        #[test]
        fn test_expand_fra_with_rate_index() {
            let request = TradeExpandRequest {
                instrument_type: TradeInstrumentType::Fra,
                params: InstrumentParamsUnion::Rates(RatesParams {
                    currency: "GBP".to_string(),
                    start_date: "2024-01-15".to_string(),
                    tenor: "3M".to_string(),
                    rate: 0.05,
                    notional: 1_000_000.0,
                    rate_index: Some("SONIA".to_string()),
                }),
            };

            let result = expand_instrument(&request);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response.trade_type, "FRA");
            // FRA leg should have the specified rate_index
            assert_eq!(response.legs[0].rate_index, Some("SONIA".to_string()));
            // Cashflow should also have rate_index
            assert_eq!(
                response.legs[0].cashflows[0].rate_index,
                Some("SONIA".to_string())
            );
        }
    }

    mod metadata_tests {
        use super::*;

        #[test]
        fn test_build_instruments_metadata() {
            let instruments = build_instruments_metadata();

            // Should have all instrument types (13 total: 6 rates + 3 FX + 2 equity + 1
            // credit + 1 commodity)
            assert!(instruments.len() >= 13);

            // Check rates instruments are present (6: Deposit, FRA, Futures, OIS,
            // BasisSwap, IRS)
            let rates_count = instruments
                .iter()
                .filter(|i| i.asset_class == AssetClass::Rates)
                .count();
            assert!(rates_count >= 6);

            // Check FX instruments are present
            let fx_count = instruments
                .iter()
                .filter(|i| i.asset_class == AssetClass::Fx)
                .count();
            assert_eq!(fx_count, 3);
        }
    }
}

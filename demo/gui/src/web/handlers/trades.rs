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
use infra_master::{
    market::{Currency, RateIndex},
    time::Tenor,
    trade::{
        convention::ConventionSet,
        instrument_def::{
            BasisSwap, CrossCurrencyBasisSwap, CurrencyPair, Deposit, EquityForward,
            EquityUnderlying, EquityVanillaOption, ExerciseStyle, Fra, FxForward, FxVanillaOption,
            Futures, InstrumentExpander, InterestRateSwap, Ois, PayerReceiver, XccyBasisConvention,
            XccyLeg, BasisSpread,
        },
        AssetClass, OptionType,
    },
    Date, Frequency,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::web::AppState;

// =============================================================================
// Rate Index Validation (using infra_master::RateIndex)
// =============================================================================

/// Validates a rate index string.
pub fn validate_rate_index(rate_index: &Option<String>) -> Result<(), String> {
    if let Some(idx) = rate_index {
        let normalised = idx.to_uppercase();
        let valid_codes = RateIndex::all_codes();
        if !valid_codes.contains(&normalised.as_str()) {
            return Err(format!(
                "Invalid rate_index '{}'. Supported values: {}",
                idx,
                valid_codes.join(", ")
            ));
        }
    }
    Ok(())
}

/// Returns the default rate index for a given currency.
#[must_use]
pub fn default_rate_index_for_currency(currency: &str) -> &'static str {
    let currency_enum = currency.parse::<Currency>().unwrap_or(Currency::USD);
    RateIndex::default_for_currency(currency_enum).api_code()
}

// =============================================================================
// TradeInstrumentType Enum
// =============================================================================

/// Trade instrument type for trade expansion requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeInstrumentType {
    Deposit,
    Fra,
    Futures,
    Ois,
    BasisSwap,
    Irs,
    FxForward,
    FxOption,
    CrossCurrencySwap,
    EquityVanillaOption,
    EquityForward,
    Cds,
    CommodityForward,
}

impl TradeInstrumentType {
    #[must_use]
    pub fn asset_class(&self) -> AssetClass {
        match self {
            Self::Deposit | Self::Fra | Self::Futures | Self::Ois | Self::BasisSwap | Self::Irs => {
                AssetClass::Rates
            }
            Self::FxForward | Self::FxOption | Self::CrossCurrencySwap => AssetClass::Fx,
            Self::EquityVanillaOption | Self::EquityForward => AssetClass::Equity,
            Self::Cds => AssetClass::Credit,
            Self::CommodityForward => AssetClass::Commodity,
        }
    }

    #[must_use]
    pub fn is_rates(&self) -> bool { matches!(self.asset_class(), AssetClass::Rates) }

    #[must_use]
    pub fn is_fx(&self) -> bool { matches!(self.asset_class(), AssetClass::Fx) }

    #[must_use]
    pub fn is_equity(&self) -> bool { matches!(self.asset_class(), AssetClass::Equity) }

    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::Deposit,
            Self::Fra,
            Self::Futures,
            Self::Ois,
            Self::BasisSwap,
            Self::Irs,
            Self::FxForward,
            Self::FxOption,
            Self::CrossCurrencySwap,
            Self::EquityVanillaOption,
            Self::EquityForward,
            Self::Cds,
            Self::CommodityForward,
        ]
    }

    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Fra => "FRA",
            Self::Futures => "Futures",
            Self::Ois => "OIS",
            Self::BasisSwap => "Basis Swap",
            Self::Irs => "IRS",
            Self::FxForward => "FX Forward",
            Self::FxOption => "FX Option",
            Self::CrossCurrencySwap => "Cross Currency Swap",
            Self::EquityVanillaOption => "Equity Vanilla Option",
            Self::EquityForward => "Equity Forward",
            Self::Cds => "CDS",
            Self::CommodityForward => "Commodity Forward",
        }
    }
}

// =============================================================================
// Instrument Parameter Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatesParams {
    pub currency: String,
    pub start_date: String,
    pub tenor: String,
    pub rate: f64,
    pub notional: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapParams {
    pub currency: String,
    pub start_date: String,
    pub tenor: String,
    pub notional: f64,
    pub fixed_rate: Option<f64>,
    pub spread: Option<f64>,
    pub payment_frequency: String,
    pub day_count: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxParams {
    pub base_currency: String,
    pub quote_currency: String,
    pub spot_rate: f64,
    pub forward_rate: Option<f64>,
    pub strike: Option<f64>,
    pub expiry: String,
    pub notional: f64,
    pub option_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityParams {
    pub underlying: String,
    pub spot_price: f64,
    pub strike: f64,
    pub expiry: String,
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub option_type: Option<String>,
    pub direction: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstrumentParamsUnion {
    Rates(RatesParams),
    Swap(SwapParams),
    Fx(FxParams),
    Equity(EquityParams),
}

// =============================================================================
// Trade Expand Request/Response Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandRequest {
    pub instrument_type: TradeInstrumentType,
    pub params: InstrumentParamsUnion,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandResponse {
    pub trade_id: String,
    pub trade_type: String,
    pub legs: Vec<LegDto>,
    pub metadata: TradeMetadataDto,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegDto {
    pub leg_number: usize,
    pub direction: String,
    pub currency: String,
    pub leg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    pub cashflows: Vec<CashflowDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyAccrualDto {
    pub date: String,
    pub overnight_rate: f64,
    pub day_fraction: f64,
    pub compounded_notional: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowDto {
    pub payment_date: String,
    pub accrual_start: String,
    pub accrual_end: String,
    pub year_fraction: f64,
    pub notional: f64,
    pub payoff_type: String,
    pub rate: Option<f64>,
    pub spread: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_accruals: Option<Vec<DailyAccrualDto>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMetadataDto {
    pub total_legs: usize,
    pub total_cashflows: usize,
    pub processing_time_ms: f64,
}

// =============================================================================
// Instruments API Response Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterFieldMeta {
    pub name: String,
    pub label: String,
    pub field_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub validation: Option<ValidationRules>,
    pub options: Option<Vec<SelectOption>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRules {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentMeta {
    pub instrument_type: TradeInstrumentType,
    pub display_name: String,
    pub asset_class: AssetClass,
    pub asset_class_name: String,
    pub required_params: Vec<ParameterFieldMeta>,
    pub optional_params: Vec<ParameterFieldMeta>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentsResponse {
    pub instruments: Vec<InstrumentMeta>,
}

// =============================================================================
// Error Response Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeExpandError {
    pub error: String,
    pub message: String,
    pub field: Option<String>,
}

impl TradeExpandError {
    pub fn validation(field: &str, message: &str) -> Self {
        Self {
            error: "invalid_parameter".to_string(),
            message: message.to_string(),
            field: Some(field.to_string()),
        }
    }

    pub fn unsupported_instrument(instrument_type: TradeInstrumentType) -> Self {
        Self {
            error: "unsupported_instrument".to_string(),
            message: format!(
                "Instrument type '{:?}' is not yet supported",
                instrument_type
            ),
            field: None,
        }
    }

    pub fn schedule_error(message: &str) -> Self {
        Self {
            error: "schedule_error".to_string(),
            message: message.to_string(),
            field: None,
        }
    }

    pub fn internal(message: &str) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.to_string(),
            field: None,
        }
    }
}

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

/// Expands a Deposit instrument using infra_master.
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

    let start = parse_date(&params.start_date)?;
    let tenor = parse_tenor(&params.tenor)?;
    let ccy = parse_currency(&params.currency)?;

    let deposit = Deposit {
        start_date: start,
        tenor,
        rate: params.rate,
        notional: params.notional,
        currency: ccy,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("DEP");

    let trade = deposit
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("Deposit expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "Deposit"))
}

/// Expands a FRA instrument using infra_master.
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

    let start = parse_date(&params.start_date)?;
    let tenor = parse_tenor(&params.tenor)?;
    let ccy = parse_currency(&params.currency)?;
    let rate_index = parse_rate_index(&params.rate_index, &params.currency);

    let fra = Fra {
        fixing_date: start,
        start_date: start,
        tenor,
        strike: params.rate,
        notional: params.notional,
        currency: ccy,
        rate_index,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("FRA");

    let trade = fra
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("FRA expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "FRA"))
}

/// Expands a Futures instrument using infra_master.
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

    let expiry = parse_date(&params.start_date)?;
    let tenor = parse_tenor(&params.tenor)?;
    let ccy = parse_currency(&params.currency)?;
    let rate_index = currency_to_rate_index(&params.currency);

    // Convert rate to price (price = 100 - rate * 100)
    let price = 100.0 - params.rate * 100.0;

    let futures = Futures {
        expiry_date: expiry,
        underlying_tenor: tenor,
        price,
        notional: params.notional,
        currency: ccy,
        rate_index,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("FUT");

    let trade = futures
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("Futures expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "Futures"))
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
        infra_master::trade::IndexType::Rate(ri) => Some(ri.api_code().to_string()),
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

/// Helper: Parse tenor string to Tenor.
fn parse_tenor(tenor_str: &str) -> Result<Tenor, TradeExpandError> {
    tenor_str
        .parse()
        .map_err(|_| TradeExpandError::validation("tenor", "Invalid tenor format"))
}

/// Helper: Parse rate index from string or use default for currency.
fn parse_rate_index(rate_index: &Option<String>, currency: &str) -> RateIndex {
    if let Some(idx) = rate_index {
        // Try to parse the rate index string
        match idx.to_uppercase().as_str() {
            "SOFR" => RateIndex::Sofr,
            "TONAR" | "TONA" => RateIndex::Tonar,
            "ESTR" | "ESTER" => RateIndex::Estr,
            "EURIBOR3M" => RateIndex::Euribor3M,
            "EURIBOR6M" => RateIndex::Euribor6M,
            "SONIA" => RateIndex::Sonia,
            "SARON" => RateIndex::Saron,
            _ => currency_to_rate_index(currency),
        }
    } else {
        currency_to_rate_index(currency)
    }
}

/// Expands a BasisSwap instrument.
/// Expands a BasisSwap instrument using infra_master.
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

    let start = parse_date(&params.start_date)?;
    let tenor = parse_tenor(&params.tenor)?;
    let ccy = parse_currency(&params.currency)?;
    let rate_index = parse_rate_index(&params.rate_index, &params.currency);
    let freq = parse_frequency(&params.payment_frequency);

    let basis_swap = BasisSwap {
        start_date: start,
        tenor,
        notional: params.notional,
        currency: ccy,
        payer_receiver: PayerReceiver::Payer,
        leg1_index: rate_index,
        leg1_spread: params.spread.unwrap_or(0.0),
        leg1_frequency: freq,
        leg2_index: rate_index,
        leg2_spread: 0.0,
        leg2_frequency: freq,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("BSW");

    let trade = basis_swap
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("BasisSwap expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "BasisSwap"))
}

/// Expands an IRS instrument using infra_master.
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

    let start = parse_date(&params.start_date)?;
    let tenor = parse_tenor(&params.tenor)?;
    let ccy = parse_currency(&params.currency)?;
    let rate_index = parse_rate_index(&params.rate_index, &params.currency);
    let freq = parse_frequency(&params.payment_frequency);

    let irs = InterestRateSwap {
        start_date: start,
        tenor,
        fixed_rate: params.fixed_rate.unwrap_or(0.0),
        spread: params.spread.unwrap_or(0.0),
        notional: params.notional,
        currency: ccy,
        payer_receiver: PayerReceiver::Payer,
        fixed_frequency: freq,
        float_frequency: freq,
        rate_index,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("IRS");

    let trade = irs
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("IRS expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "IRS"))
}

// =============================================================================
// Task 3.2: FX Instrument Expansion
// =============================================================================

/// Expands an FX Forward instrument using infra_master.
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

    let delivery_date = parse_date(&params.expiry)?;
    let base_ccy = parse_currency(&params.base_currency)?;
    let quote_ccy = parse_currency(&params.quote_currency)?;
    let forward_rate = params.forward_rate.unwrap_or(params.spot_rate);

    let fx_forward = FxForward {
        currency_pair: CurrencyPair::new(base_ccy, quote_ccy),
        forward_rate,
        settlement_date: delivery_date,
        notional: params.notional,
        notional_currency: base_ccy,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("FXF");

    let trade = fx_forward
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("FxForward expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "FxForward"))
}

/// Expands an FX Option instrument using infra_master.
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

    let expiry = parse_date(&params.expiry)?;
    let base_ccy = parse_currency(&params.base_currency)?;
    let quote_ccy = parse_currency(&params.quote_currency)?;
    let strike = params.strike.unwrap_or(params.spot_rate);
    let option_type_str = params.option_type.as_deref().unwrap_or("call");
    let option_type = if option_type_str.to_lowercase() == "put" {
        OptionType::Put
    } else {
        OptionType::Call
    };

    let fx_option = FxVanillaOption {
        currency_pair: CurrencyPair::new(base_ccy, quote_ccy),
        strike,
        expiry,
        delivery_date: expiry, // Assume same day delivery
        option_type,
        exercise_style: ExerciseStyle::European,
        notional: params.notional,
        notional_currency: base_ccy,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("FXO");

    let trade = fx_option
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("FxOption expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, &format!("FxOption({})", option_type_str)))
}

/// Expands a Cross-Currency Swap instrument using infra_master.
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

    let start_date = parse_date(&params.expiry)?;
    // Default 5Y maturity
    let maturity = Tenor::FiveYears.add_to_date(start_date, infra_master::time::EndOfMonthRule::Adjust);
    let domestic_ccy = parse_currency(&params.base_currency)?;
    let foreign_ccy = parse_currency(&params.quote_currency)?;
    let domestic_index = currency_to_rate_index(&params.base_currency);
    let foreign_index = currency_to_rate_index(&params.quote_currency);

    let xccy = CrossCurrencyBasisSwap {
        domestic_currency: domestic_ccy,
        foreign_currency: foreign_ccy,
        notional: params.notional,
        start_date,
        maturity,
        domestic_leg: XccyLeg::new(domestic_ccy, domestic_index, Frequency::Quarterly),
        foreign_leg: XccyLeg::new(foreign_ccy, foreign_index, Frequency::Quarterly),
        basis_spread: BasisSpread::zero(),
        convention: XccyBasisConvention::default(),
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("CCS");

    let trade = xccy
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("XCCY expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(trade, &trade_id_str, "CrossCurrencySwap"))
}

// =============================================================================
// Task 3.3: Equity Instrument Expansion
// =============================================================================

/// Expands an Equity Vanilla Option instrument using infra_master.
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

    let expiry = parse_date(&params.expiry)?;
    let option_type_str = params.option_type.as_deref().unwrap_or("call");
    let option_type = if option_type_str.to_lowercase() == "put" {
        OptionType::Put
    } else {
        OptionType::Call
    };

    let eq_option = EquityVanillaOption {
        underlying: EquityUnderlying::Index {
            name: params.underlying.clone(),
        },
        strike: params.strike,
        expiry,
        option_type,
        exercise_style: ExerciseStyle::European,
        notional: params.spot_price,
        currency: Currency::USD,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("EQO");

    let trade = eq_option
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("EquityVanillaOption expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(
        trade,
        &trade_id_str,
        &format!("EquityVanillaOption({} on {})", option_type_str, params.underlying),
    ))
}

/// Expands an Equity Forward instrument using infra_master.
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

    let settlement_date = parse_date(&params.expiry)?;
    let direction = params.direction.as_deref().unwrap_or("long");

    let eq_forward = EquityForward {
        underlying: EquityUnderlying::Index {
            name: params.underlying.clone(),
        },
        forward_price: params.strike,
        settlement_date,
        notional: params.spot_price,
        currency: Currency::USD,
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::today();
    let trade_id_str = generate_trade_id("EQF");

    let trade = eq_forward
        .expand_to_trade(trade_id_str.clone(), valuation_date, &conventions)
        .map_err(|e| TradeExpandError::schedule_error(&format!("EquityForward expansion error: {:?}", e)))?;

    Ok(convert_trade_to_dto(
        trade,
        &trade_id_str,
        &format!("EquityForward({} on {})", direction, params.underlying),
    ))
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

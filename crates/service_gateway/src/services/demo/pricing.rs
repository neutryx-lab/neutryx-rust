//! Trade expansion, pricing, and Greeks calculation.

use std::{sync::Arc, time::Instant};

use pricer_pricing::generic_pricer::{
    DefaultCurrency, SimpleCashflow, SimpleDate, SimpleDirection, SimpleLeg,
};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        Cashflow, CashflowPvResult, DemoGreeksRequest, DemoGreeksResult, DemoPricingRequest,
        DemoPricingResult, ExpandedTrade, LegResult, TradeExpandRequest, TradeMetadata, TradeLeg,
    },
    state::AppState,
};

use super::DemoService;

/// Parse a "YYYY-MM-DD" date string into `SimpleDate`.
fn parse_simple_date(date_str: &str) -> Result<SimpleDate, ServerError> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(ServerError::InvalidRequest(format!(
            "Invalid date format (expected YYYY-MM-DD): {date_str}"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| ServerError::InvalidRequest(format!("Invalid year: {}", parts[0])))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| ServerError::InvalidRequest(format!("Invalid month: {}", parts[1])))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| ServerError::InvalidRequest(format!("Invalid day: {}", parts[2])))?;

    SimpleDate::from_ymd(year, month, day)
        .ok_or_else(|| ServerError::InvalidRequest(format!("Invalid date: {date_str}")))
}

/// Format a `SimpleDate` back to "YYYY-MM-DD" string (approximate).
fn format_simple_date(date: SimpleDate) -> String {
    let days = date.days();
    let year = 2000 + days / 365;
    let remaining = days % 365;
    let month = remaining / 30 + 1;
    let day = remaining % 30;
    format!("{year:04}-{month:02}-{day:02}")
}

/// Convert DTO `PricingLeg` to `SimpleLeg` for `GenericPricer`.
fn convert_to_simple_legs(
    legs: &[crate::rest::dto::demo::PricingLeg],
) -> Result<Vec<SimpleLeg>, ServerError> {
    legs.iter()
        .map(|leg| {
            let currency = DefaultCurrency::new(&leg.currency);
            let direction = match leg.direction.to_lowercase().as_str() {
                "payer" => SimpleDirection::Payer,
                "receiver" => SimpleDirection::Receiver,
                other => {
                    return Err(ServerError::InvalidRequest(format!(
                        "Unknown direction: {other}"
                    )))
                }
            };
            let cashflows = leg
                .cashflows
                .iter()
                .map(|cf| {
                    let payment_date = parse_simple_date(&cf.payment_date)?;
                    Ok(SimpleCashflow {
                        payment_date,
                        amount: cf.amount,
                    })
                })
                .collect::<Result<Vec<_>, ServerError>>()?;

            Ok(SimpleLeg {
                currency,
                direction,
                cashflows,
            })
        })
        .collect()
}

/// Create legs with bumped cashflow amounts.
fn bump_cashflow_amounts(legs: &[SimpleLeg], bump_fraction: f64) -> Vec<SimpleLeg> {
    legs.iter()
        .map(|leg| SimpleLeg {
            currency: leg.currency,
            direction: leg.direction,
            cashflows: leg
                .cashflows
                .iter()
                .map(|cf| SimpleCashflow {
                    payment_date: cf.payment_date,
                    amount: cf.amount * (1.0 + bump_fraction),
                })
                .collect(),
        })
        .collect()
}

impl DemoService {
    /// Expand a trade request into cashflows.
    pub fn expand_trade(
        request: &TradeExpandRequest,
        _state: &Arc<AppState>,
    ) -> Result<ExpandedTrade, ServerError> {
        let start = Instant::now();

        let (legs, trade_type) = match request.instrument_type.as_str() {
            "IRS" => {
                let fixed_leg = TradeLeg {
                    direction: "Payer".to_string(),
                    currency: request
                        .params
                        .get("currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("USD")
                        .to_string(),
                    leg_type: "Fixed".to_string(),
                    rate_index: None,
                    cashflows: vec![Cashflow {
                        payment_date: "2027-01-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2027-01-30".to_string(),
                        year_fraction: 1.0,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: Some(0.05),
                        payoff_type: "Fixed".to_string(),
                        rate_index: None,
                    }],
                };
                let float_leg = TradeLeg {
                    direction: "Receiver".to_string(),
                    currency: request
                        .params
                        .get("currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("USD")
                        .to_string(),
                    leg_type: "Float".to_string(),
                    rate_index: Some("SOFR".to_string()),
                    cashflows: vec![Cashflow {
                        payment_date: "2027-01-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2027-01-30".to_string(),
                        year_fraction: 1.0,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: None,
                        payoff_type: "Linear".to_string(),
                        rate_index: Some("SOFR".to_string()),
                    }],
                };
                (vec![fixed_leg, float_leg], "IRS")
            }
            "FxForward" => {
                let leg = TradeLeg {
                    direction: "Payer".to_string(),
                    currency: "EUR".to_string(),
                    leg_type: "FxForward".to_string(),
                    rate_index: None,
                    cashflows: vec![Cashflow {
                        payment_date: "2026-07-30".to_string(),
                        accrual_start: "2026-01-30".to_string(),
                        accrual_end: "2026-07-30".to_string(),
                        year_fraction: 0.5,
                        notional: request
                            .params
                            .get("notional")
                            .and_then(|n| n.as_f64())
                            .unwrap_or(1_000_000.0),
                        rate: None,
                        payoff_type: "Forward".to_string(),
                        rate_index: None,
                    }],
                };
                (vec![leg], "FxForward")
            }
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unknown instrument type: {}",
                    request.instrument_type
                )))
            }
        };

        let total_cashflows = legs.iter().map(|l| l.cashflows.len()).sum();
        let elapsed = start.elapsed();

        Ok(ExpandedTrade {
            trade_id: uuid::Uuid::new_v4().to_string(),
            trade_type: trade_type.to_string(),
            legs,
            metadata: TradeMetadata {
                total_legs: 2,
                total_cashflows,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
            },
        })
    }

    /// Price a trade using `GenericPricer`.
    pub fn price_trade(
        request: &DemoPricingRequest,
        state: &Arc<AppState>,
    ) -> Result<DemoPricingResult, ServerError> {
        let simple_legs = convert_to_simple_legs(&request.legs)?;
        let valuation_date = parse_simple_date(&request.valuation_date)?;
        let reporting_ccy = DefaultCurrency::new(&request.reporting_currency);

        let result = state
            .pricer
            .get_pv_simple(simple_legs, valuation_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Pricing failed: {e}")))?;

        let legs: Vec<LegResult> = result
            .legs
            .iter()
            .map(|leg| {
                let direction = match leg.direction {
                    SimpleDirection::Payer => "payer",
                    SimpleDirection::Receiver => "receiver",
                };
                let cashflows: Vec<CashflowPvResult> = leg
                    .cashflows
                    .iter()
                    .map(|cf| CashflowPvResult {
                        pv: cf.pv,
                        discount_factor: cf.discount_factor,
                        payment_date: format_simple_date(cf.payment_date),
                    })
                    .collect();
                LegResult {
                    direction: direction.to_string(),
                    pv: leg.pv,
                    currency: leg.original_currency.code().to_string(),
                    pv_original: Some(leg.pv_original),
                    fx_rate: Some(leg.fx_rate),
                    cashflows: Some(cashflows),
                }
            })
            .collect();

        Ok(DemoPricingResult {
            total_pv: Some(result.total_pv),
            pv: Some(result.total_pv),
            currency: request.reporting_currency.clone(),
            legs: Some(legs),
        })
    }

    /// Calculate Greeks via bump-and-revalue using `GenericPricer`.
    pub fn calculate_greeks(
        request: &DemoGreeksRequest,
        state: &Arc<AppState>,
    ) -> Result<DemoGreeksResult, ServerError> {
        let simple_legs = convert_to_simple_legs(&request.legs)?;
        let valuation_date = parse_simple_date(&request.valuation_date)?;
        let reporting_ccy = DefaultCurrency::new(&request.reporting_currency);

        let base_pv = state
            .pricer
            .get_pv_simple(simple_legs.clone(), valuation_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Base pricing failed: {e}")))?
            .total_pv;

        let rate_bump = request.bump_sizes.rate_bump_bp / 10000.0;
        let legs_up = bump_cashflow_amounts(&simple_legs, rate_bump);
        let legs_down = bump_cashflow_amounts(&simple_legs, -rate_bump);

        let pv_up = state
            .pricer
            .get_pv_simple(legs_up, valuation_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Delta up pricing failed: {e}")))?
            .total_pv;
        let pv_down = state
            .pricer
            .get_pv_simple(legs_down, valuation_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Delta down pricing failed: {e}")))?
            .total_pv;

        let delta = (pv_up - pv_down) / 2.0;
        let gamma = Some(pv_up - 2.0 * base_pv + pv_down);

        let theta_date = SimpleDate::from_days(valuation_date.days() + 1);
        let theta_pv = state
            .pricer
            .get_pv_simple(simple_legs.clone(), theta_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Theta pricing failed: {e}")))?
            .total_pv;
        let theta = Some(theta_pv - base_pv);

        let vol_bump = request.bump_sizes.vol_bump_pct / 100.0;
        let legs_vol_up = bump_cashflow_amounts(&simple_legs, vol_bump);
        let pv_vol_up = state
            .pricer
            .get_pv_simple(legs_vol_up, valuation_date, reporting_ccy)
            .map_err(|e| ServerError::Internal(format!("Vega pricing failed: {e}")))?
            .total_pv;
        let vega = Some(pv_vol_up - base_pv);

        Ok(DemoGreeksResult {
            currency: request.reporting_currency.clone(),
            delta,
            gamma,
            theta,
            vega,
        })
    }
}

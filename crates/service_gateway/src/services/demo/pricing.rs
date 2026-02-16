//! Trade expansion, pricing, and Greeks calculation.

use std::{sync::Arc, time::Instant};

use chrono::Datelike;
use infra_domain::{
    market::Currency,
    time::Date,
    trade::{
        Cashflow as DomainCashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType,
    },
};
use pricer_pricing::{
    pricer::GreeksMode, CalcSetting, MarketEnvironment, MarketEnvironmentBuilder,
    MonteCarloSetting, Pricer, PricingMethodHint, TreeSetting, TreeType,
};

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::demo::{
        Cashflow, CashflowPvResult, DemoGreeksInline, DemoGreeksRequest, DemoGreeksResult,
        DemoPathDistribution, DemoPricingMethod, DemoPricingRequest, DemoPricingResult,
        DemoTreeType, ExpandedTrade, LegResult, PricingLeg, TradeExpandRequest, TradeLeg,
        TradeMetadata,
    },
    state::AppState,
};

/// Parse currency ISO code to `Currency`.
fn parse_currency(s: &str) -> Result<Currency, ServerError> {
    s.parse::<Currency>()
        .map_err(|_| ServerError::InvalidRequest(format!("Unknown currency: {s}")))
}

/// Parse "YYYY-MM-DD" to domain `Date`.
fn parse_date(s: &str) -> Result<Date, ServerError> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(ServerError::InvalidRequest(format!(
            "Invalid date format (expected YYYY-MM-DD): {s}"
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

    Date::from_ymd(year, month, day)
        .map_err(|_| ServerError::InvalidRequest(format!("Invalid date: {s}")))
}

/// Format a domain `Date` as "YYYY-MM-DD".
fn format_date(date: Date) -> String { date.into_inner().format("%Y-%m-%d").to_string() }

/// Default discount rate for demo pricing (flat curve).
const DEFAULT_DISCOUNT_RATE: f64 = 0.05;

/// Build domain [`Leg`]s from DTO pricing legs.
fn build_domain_legs(
    dto_legs: &[PricingLeg],
    valuation_date: Date,
) -> Result<Vec<Leg>, ServerError> {
    dto_legs
        .iter()
        .map(|dto_leg| {
            let currency = parse_currency(&dto_leg.currency)?;
            let direction = match dto_leg.direction.to_lowercase().as_str() {
                "payer" => Direction::Payer,
                "receiver" => Direction::Receiver,
                other => {
                    return Err(ServerError::InvalidRequest(format!(
                        "Unknown direction: {other}"
                    )))
                }
            };

            let cashflows: Vec<DomainCashflow> = dto_leg
                .cashflows
                .iter()
                .map(|cf| {
                    let payment_date = parse_date(&cf.payment_date)?;
                    let amount = cf.notional * cf.rate * cf.year_fraction;
                    // Encode the computed amount as notional with a unit fixed payoff
                    // so PayoffEvaluator::evaluate returns exactly `amount`.
                    Ok(DomainCashflow::new(
                        CashflowType::Settlement,
                        payment_date,
                        valuation_date, // accrual_start
                        payment_date,   // accrual_end
                        1.0,            // year_fraction
                        amount,         // notional
                        Payoff::fixed(1.0),
                        currency,
                    ))
                })
                .collect::<Result<Vec<_>, ServerError>>()?;

            Ok(Leg::new(cashflows, direction, LegType::Generic, currency))
        })
        .collect()
}

/// Build a [`MarketEnvironment`] with flat discount curves at the given rate.
fn build_market_env(
    valuation_date: Date,
    reporting_currency: Currency,
    dto_legs: &[PricingLeg],
    discount_rate: f64,
) -> Result<MarketEnvironment, ServerError> {
    let currencies = dto_legs
        .iter()
        .map(|l| parse_currency(&l.currency))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MarketEnvironmentBuilder::flat_multi_currency(
        valuation_date,
        reporting_currency,
        &currencies,
        discount_rate,
    ))
}

/// Map DTO pricing method to domain `PricingMethodHint`.
fn map_method(m: DemoPricingMethod) -> PricingMethodHint {
    match m {
        DemoPricingMethod::Auto => PricingMethodHint::Auto,
        DemoPricingMethod::Analytical => PricingMethodHint::Analytical,
        DemoPricingMethod::MonteCarlo => PricingMethodHint::MonteCarlo,
        DemoPricingMethod::Tree => PricingMethodHint::Tree,
    }
}

/// Map DTO tree type to domain `TreeType`.
fn map_tree_type(t: DemoTreeType) -> TreeType {
    match t {
        DemoTreeType::Binomial => TreeType::Binomial,
        DemoTreeType::Trinomial => TreeType::Trinomial,
    }
}

/// Build `CalcSetting` from a `DemoPricingRequest`.
fn build_calc_setting(request: &DemoPricingRequest) -> Result<CalcSetting, ServerError> {
    let reporting_currency = parse_currency(&request.reporting_currency)?;

    let mc_config = request.mc_config.as_ref().map(|mc| MonteCarloSetting {
        num_paths: mc.num_paths,
        num_steps: mc.num_steps,
        seed: mc.seed,
    });

    let tree_config = request.tree_config.as_ref().map(|tc| TreeSetting {
        num_steps: tc.num_steps.unwrap_or(100),
        tree_type: tc.tree_type.map(map_tree_type).unwrap_or_default(),
    });

    Ok(CalcSetting {
        method: map_method(request.method),
        compute_greeks: request.compute_greeks,
        reporting_currency,
        mc_config,
        tree_config,
        greeks_mode: GreeksMode::default(),
    })
}

/// Build unified `Pricer` inputs from a `DemoPricingRequest`.
fn build_pricer_inputs(
    request: &DemoPricingRequest,
) -> Result<(Trade, MarketEnvironment, CalcSetting), ServerError> {
    let valuation_date = parse_date(&request.valuation_date)?;
    let reporting_currency = parse_currency(&request.reporting_currency)?;

    let domain_legs = build_domain_legs(&request.legs, valuation_date)?;
    let trade = Trade::new("DEMO-TRADE", domain_legs, TradeType::Swap);
    let market = build_market_env(
        valuation_date,
        reporting_currency,
        &request.legs,
        DEFAULT_DISCOUNT_RATE,
    )?;
    let calc = build_calc_setting(request)?;

    Ok((trade, market, calc))
}

/// Price a trade with a specific discount rate (used for bump-and-revalue).
fn price_with_rate(
    trade: &Trade,
    valuation_date: Date,
    reporting_currency: Currency,
    dto_legs: &[PricingLeg],
    discount_rate: f64,
) -> Result<f64, ServerError> {
    let market = build_market_env(valuation_date, reporting_currency, dto_legs, discount_rate)?;
    let calc = CalcSetting::builder()
        .reporting_currency(reporting_currency)
        .build();
    Pricer::price_pv(trade, &market, &calc)
        .map_err(|e| ServerError::Internal(format!("Pricing failed: {e}")))
}

/// Format `PricingMethodHint` for output.
fn format_method(m: PricingMethodHint) -> &'static str {
    match m {
        PricingMethodHint::Auto => "Auto",
        PricingMethodHint::Analytical => "Analytical",
        PricingMethodHint::MonteCarlo => "MonteCarlo",
        PricingMethodHint::Tree => "Tree",
    }
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

    /// Price a trade using the unified `Pricer`, returning full `PricingResult`.
    pub fn price_trade(
        request: &DemoPricingRequest,
        _state: &Arc<AppState>,
    ) -> Result<DemoPricingResult, ServerError> {
        let start = Instant::now();
        let (trade, market, calc) = build_pricer_inputs(request)?;
        let method = calc.method;

        let result = Pricer::price(&trade, &market, &calc)
            .map_err(|e| ServerError::Internal(format!("Pricing failed: {e}")))?;

        let elapsed = start.elapsed();

        let legs: Vec<LegResult> = result
            .legs
            .iter()
            .map(|leg| {
                let direction = match leg.direction {
                    Direction::Payer => "payer",
                    Direction::Receiver => "receiver",
                };
                let cashflows: Vec<CashflowPvResult> = leg
                    .cashflows
                    .iter()
                    .map(|cf| CashflowPvResult {
                        pv: cf.pv,
                        discount_factor: cf.discount_factor,
                        payment_date: format_date(cf.payment_date),
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

        let path_distribution = result.path_distribution.as_ref().map(|pd| {
            DemoPathDistribution {
                mean: pd.mean,
                std_dev: pd.std_dev,
                percentiles: pd.percentiles.clone(),
                path_count: pd.path_count,
            }
        });

        // Inline Greeks via bump-and-revalue if requested.
        let greeks = if request.compute_greeks {
            let valuation_date = parse_date(&request.valuation_date)?;
            let reporting_currency = parse_currency(&request.reporting_currency)?;
            let domain_legs = build_domain_legs(&request.legs, valuation_date)?;
            let greeks_trade = Trade::new("DEMO-GREEKS", domain_legs, TradeType::Swap);

            let rate_bump = 1.0 / 10000.0; // 1bp
            let pv_up = price_with_rate(
                &greeks_trade, valuation_date, reporting_currency, &request.legs,
                DEFAULT_DISCOUNT_RATE + rate_bump,
            )?;
            let pv_down = price_with_rate(
                &greeks_trade, valuation_date, reporting_currency, &request.legs,
                DEFAULT_DISCOUNT_RATE - rate_bump,
            )?;

            let delta = (pv_up - pv_down) / 2.0;
            let gamma = pv_up - 2.0 * result.total_pv + pv_down;

            // Theta: shift valuation date +1d.
            let theta_inner = valuation_date.into_inner() + chrono::Duration::days(1);
            let theta_date = Date::from_ymd(
                theta_inner.year(), theta_inner.month(), theta_inner.day(),
            ).ok();
            let theta = theta_date.and_then(|td| {
                let theta_legs = build_domain_legs(&request.legs, td).ok()?;
                let theta_trade = Trade::new("DEMO-THETA", theta_legs, TradeType::Swap);
                price_with_rate(&theta_trade, td, reporting_currency, &request.legs, DEFAULT_DISCOUNT_RATE).ok()
            }).map(|pv| pv - result.total_pv);

            Some(DemoGreeksInline {
                delta: Some(delta),
                gamma: Some(gamma),
                vega: Some(0.0), // No vol surface in demo
                theta,
                rho: Some(delta), // For linear products, rho ≈ rate delta
            })
        } else {
            None
        };

        Ok(DemoPricingResult {
            total_pv: result.total_pv,
            reporting_currency: result.reporting_currency.code().to_string(),
            legs,
            path_distribution,
            method: Some(format_method(method).to_string()),
            greeks,
            computation_time_ms: Some(elapsed.as_secs_f64() * 1000.0),
        })
    }

    /// Calculate Greeks via bump-and-revalue using the unified [`Pricer`].
    ///
    /// Bumps the flat discount rate in the [`MarketEnvironment`] to compute
    /// rate delta, gamma, and theta.  Vega is zero for linear products (no
    /// volatility surface in the demo market).
    pub fn calculate_greeks(
        request: &DemoGreeksRequest,
        _state: &Arc<AppState>,
    ) -> Result<DemoGreeksResult, ServerError> {
        let valuation_date = parse_date(&request.valuation_date)?;
        let reporting_currency = parse_currency(&request.reporting_currency)?;

        let domain_legs = build_domain_legs(&request.legs, valuation_date)?;
        let trade = Trade::new("DEMO-GREEKS", domain_legs, TradeType::Swap);

        // Base PV.
        let base_pv = price_with_rate(
            &trade,
            valuation_date,
            reporting_currency,
            &request.legs,
            DEFAULT_DISCOUNT_RATE,
        )?;

        // Rate delta: bump discount rate up/down.
        let rate_bump = request.bump_sizes.rate_bump_bp / 10000.0;
        let pv_up = price_with_rate(
            &trade,
            valuation_date,
            reporting_currency,
            &request.legs,
            DEFAULT_DISCOUNT_RATE + rate_bump,
        )?;
        let pv_down = price_with_rate(
            &trade,
            valuation_date,
            reporting_currency,
            &request.legs,
            DEFAULT_DISCOUNT_RATE - rate_bump,
        )?;

        let delta = (pv_up - pv_down) / 2.0;
        let gamma = Some(pv_up - 2.0 * base_pv + pv_down);

        // Theta: shift valuation date forward by 1 day.
        let theta_inner = valuation_date.into_inner() + chrono::Duration::days(1);
        let theta_date = Date::from_ymd(theta_inner.year(), theta_inner.month(), theta_inner.day())
            .map_err(|e| ServerError::Internal(format!("Theta date error: {e}")))?;

        // Rebuild legs relative to the new valuation date.
        let theta_legs = build_domain_legs(&request.legs, theta_date)?;
        let theta_trade = Trade::new("DEMO-GREEKS-THETA", theta_legs, TradeType::Swap);
        let theta_pv = price_with_rate(
            &theta_trade,
            theta_date,
            reporting_currency,
            &request.legs,
            DEFAULT_DISCOUNT_RATE,
        )?;
        let theta = Some(theta_pv - base_pv);

        // Vega: zero for linear products (no vol surface in demo market).
        let vega = Some(0.0);

        Ok(DemoGreeksResult {
            currency: request.reporting_currency.clone(),
            delta,
            gamma,
            theta,
            vega,
        })
    }
}

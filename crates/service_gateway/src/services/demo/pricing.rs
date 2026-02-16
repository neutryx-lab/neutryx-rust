//! Trade expansion, pricing, and Greeks calculation.

use std::{sync::Arc, time::Instant};

use chrono::Datelike;
use infra_domain::{
    market::{
        instrument::{
            convention::ConventionSet, AsianOption, AveragingType, BarrierDirection, BarrierType,
            BasisSwap, Bond, BondType, CapFloor, CapFloorType, Cds, CreditEvent, Deposit,
            EquityForward, EquityUnderlying, EquityVanillaOption, ExerciseStyle, Fra, FxBarrierOption,
            FxForward, FxVanillaOption, InstrumentExpander, InterestRateSwap, NotionalSchedule,
            Ois, PayerReceiver, Swaption,
        },
        Currency, CurrencyPair, RateIndex,
    },
    time::{Date, Frequency, Tenor},
    trade::{
        Cashflow as DomainCashflow, CashflowType, Direction, IndexType, Leg, LegType, OptionType,
        Payoff, SettlementType, Trade, TradeType,
    },
};
use pricer_models::market::{CurveEnum, CurveName};
use pricer_pricing::{
    pricer::GreeksMode, CalcSetting, MarketEnvironment, MarketEnvironmentBuilder,
    MonteCarloSetting, Pricer, PricingMethodHint, TreeSetting, TreeType,
};

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::demo::{
        AdvancedGreeksMode, Cashflow, CashflowPvResult, DemoAdvancedGreeksRequest,
        DemoAdvancedGreeksResult, DemoGreeksInline, DemoGreeksRequest, DemoGreeksResult,
        DemoPathDistribution, DemoPricingMethod, DemoPricingRequest, DemoPricingResult,
        DemoTreeType, ExpandedTrade, FactorGreeks, FactorGreeksEntry, LegResult,
        PricerGraphEdge, PricerGraphMetadata, PricerGraphNode, PricerGraphRequest,
        PricerGraphResponse, PricingLeg, RiskFactor, TradeExpandRequest, TradeLeg,
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

/// Parse a rate index name (e.g. "SOFR", "TONA") into a `RateIndex`.
fn parse_rate_index(s: &str) -> Result<RateIndex, ServerError> {
    // Frontend sends "TONA" but the enum variant is `Tonar`.
    let normalised = match s.to_uppercase().as_str() {
        "TONA" => "TONAR".to_string(),
        other => other.to_string(),
    };
    RateIndex::from_index_name(&normalised)
        .ok_or_else(|| ServerError::InvalidRequest(format!("Unknown rate index: {s}")))
}

/// Parse a currency pair string (e.g. "EURUSD") into a `CurrencyPair`.
fn parse_currency_pair(s: &str) -> Result<CurrencyPair, ServerError> {
    if s.len() != 6 {
        return Err(ServerError::InvalidRequest(format!(
            "Currency pair must be 6 characters: {s}"
        )));
    }
    let base = parse_currency(&s[..3])?;
    let quote = parse_currency(&s[3..])?;
    Ok(CurrencyPair::new(base, quote))
}

/// Compute a `Tenor` from two dates (approximate month count).
fn compute_tenor(start: Date, end: Date) -> Tenor {
    let days = end - start;
    let months = (days as f64 / 30.44).round() as u32;
    Tenor::months(months.max(1))
}

/// Default fixed/float payment frequencies for a given rate index.
fn default_swap_frequencies(index: RateIndex) -> (Frequency, Frequency) {
    match index {
        RateIndex::Euribor3M => (Frequency::Annual, Frequency::Quarterly),
        RateIndex::Euribor6M => (Frequency::Annual, Frequency::SemiAnnual),
        _ => (Frequency::Annual, Frequency::Annual),
    }
}

/// Default overnight rate index for a currency.
fn default_rate_index(currency: Currency) -> RateIndex {
    match currency {
        Currency::EUR => RateIndex::Estr,
        Currency::GBP => RateIndex::Sonia,
        Currency::JPY => RateIndex::Tonar,
        Currency::CHF => RateIndex::Saron,
        _ => RateIndex::Sofr,
    }
}

/// Default discount rate for demo pricing (flat curve).
const DEFAULT_DISCOUNT_RATE: f64 = 0.05;

/// Build domain [`Leg`]s from DTO pricing legs.
///
/// Fixed cashflows use `Payoff::fixed(rate)` so the evaluator computes
/// `notional * rate * year_fraction`.  Float cashflows use
/// `Payoff::floating(IndexType::Rate(..))` so the evaluator looks up the
/// forward rate from the `CurveSet`.
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

            let mut has_float = false;
            let cashflows: Vec<DomainCashflow> = dto_leg
                .cashflows
                .iter()
                .map(|cf| {
                    let payment_date = parse_date(&cf.payment_date)?;
                    let accrual_start = cf
                        .accrual_start
                        .as_deref()
                        .map(parse_date)
                        .transpose()?
                        .unwrap_or(valuation_date);
                    let accrual_end = cf
                        .accrual_end
                        .as_deref()
                        .map(parse_date)
                        .transpose()?
                        .unwrap_or(payment_date);

                    let payoff = match cf.payoff_type.as_str() {
                        "Linear" => {
                            has_float = true;
                            let ri = cf
                                .rate_index
                                .as_deref()
                                .map(parse_rate_index)
                                .transpose()?
                                .unwrap_or(RateIndex::Sofr);
                            Payoff::floating(IndexType::Rate(ri))
                        }
                        _ => Payoff::fixed(cf.rate.unwrap_or(0.0)),
                    };

                    Ok(DomainCashflow::new(
                        CashflowType::Coupon,
                        payment_date,
                        accrual_start,
                        accrual_end,
                        cf.year_fraction,
                        cf.notional,
                        payoff,
                        currency,
                    ))
                })
                .collect::<Result<Vec<_>, ServerError>>()?;

            let leg_type = if has_float {
                LegType::Floating
            } else {
                LegType::Fixed
            };
            Ok(Leg::new(cashflows, direction, leg_type, currency))
        })
        .collect()
}

/// Map a `RateIndex` to the `CurveName` expected by `CurveSet::forward_rate_for_index`.
fn rate_index_to_curve_name(ri: RateIndex) -> CurveName {
    match ri {
        RateIndex::Sofr => CurveName::Sofr,
        RateIndex::Euribor3M | RateIndex::Euribor6M => CurveName::Euribor,
        RateIndex::Estr => CurveName::Estr,
        RateIndex::Tonar => CurveName::Tonar,
        RateIndex::Sonia => CurveName::Sonia,
        RateIndex::Saron => CurveName::Custom("SARON"),
    }
}

/// Build a [`MarketEnvironment`] with flat discount **and forward** curves.
///
/// Forward curves are added for every rate index referenced by float legs so
/// that `PayoffEvaluator::evaluate_linear` can look up forward rates.
fn build_market_env(
    valuation_date: Date,
    reporting_currency: Currency,
    dto_legs: &[PricingLeg],
    discount_rate: f64,
) -> Result<MarketEnvironment, ServerError> {
    let currencies: Vec<Currency> = dto_legs
        .iter()
        .map(|l| parse_currency(&l.currency))
        .collect::<Result<Vec<_>, _>>()?;

    let mut builder = MarketEnvironmentBuilder::new(valuation_date)
        .with_discount_curve(reporting_currency, CurveEnum::flat(discount_rate));

    for &ccy in &currencies {
        if ccy != reporting_currency {
            builder = builder
                .with_discount_curve(ccy, CurveEnum::flat(discount_rate))
                .with_fx_spot(CurrencyPair::new(ccy, reporting_currency), 1.0);
        }
    }

    // Collect unique rate indices from float legs and add forward curves.
    let mut seen_curves = std::collections::HashSet::new();
    for leg in dto_legs {
        for cf in &leg.cashflows {
            if cf.payoff_type == "Linear" {
                if let Some(ri_str) = cf.rate_index.as_deref() {
                    if let Ok(ri) = parse_rate_index(ri_str) {
                        let cn = rate_index_to_curve_name(ri);
                        if seen_curves.insert(cn) {
                            builder =
                                builder.with_forward_curve(cn, CurveEnum::flat(discount_rate));
                        }
                    }
                }
            }
        }
    }

    Ok(builder.build())
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

/// Build a [`MarketEnvironment`] with per-currency flat discount rates.
fn build_market_env_with_rates(
    valuation_date: Date,
    reporting_currency: Currency,
    currency_rates: &[(Currency, f64)],
    dto_legs: &[PricingLeg],
) -> Result<MarketEnvironment, ServerError> {
    let mut builder = MarketEnvironmentBuilder::new(valuation_date);

    for &(ccy, rate) in currency_rates {
        builder = builder.with_discount_curve(ccy, CurveEnum::flat(rate));
        if ccy != reporting_currency {
            builder = builder.with_fx_spot(CurrencyPair::new(ccy, reporting_currency), 1.0);
        }
    }

    // Add forward curves for float legs.
    let mut seen_curves = std::collections::HashSet::new();
    for leg in dto_legs {
        for cf in &leg.cashflows {
            if cf.payoff_type == "Linear" {
                if let Some(ri_str) = cf.rate_index.as_deref() {
                    if let Ok(ri) = parse_rate_index(ri_str) {
                        let cn = rate_index_to_curve_name(ri);
                        if seen_curves.insert(cn) {
                            builder = builder
                                .with_forward_curve(cn, CurveEnum::flat(DEFAULT_DISCOUNT_RATE));
                        }
                    }
                }
            }
        }
    }

    Ok(builder.build())
}

/// Price a trade with per-currency discount rates.
fn price_with_rates(
    trade: &Trade,
    valuation_date: Date,
    reporting_currency: Currency,
    currency_rates: &[(Currency, f64)],
    dto_legs: &[PricingLeg],
) -> Result<f64, ServerError> {
    let market = build_market_env_with_rates(
        valuation_date,
        reporting_currency,
        currency_rates,
        dto_legs,
    )?;
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

/// Convert a domain `Trade` (produced by `InstrumentExpander`) into DTO
/// `ExpandedTrade` for the frontend.
fn domain_trade_to_dto(
    trade: &Trade,
    trade_type_label: &str,
    elapsed: std::time::Duration,
) -> ExpandedTrade {
    let dto_legs: Vec<TradeLeg> = trade
        .legs()
        .enumerate()
        .map(|(_leg_idx, leg)| {
            let direction = match leg.direction {
                Direction::Payer => "Payer",
                Direction::Receiver => "Receiver",
            };
            let leg_type_str = match leg.leg_type {
                LegType::Fixed => "Fixed",
                LegType::Floating => "Float",
                LegType::CapFloor => "CapFloor",
                LegType::Principal => "Principal",
                LegType::Generic => "Generic",
            };

            // Determine rate_index from the first floating cashflow in this leg.
            let leg_rate_index: Option<String> = leg.cashflows().find_map(|cf| {
                cf.payoff.required_index().map(|idx| match idx {
                    IndexType::Rate(ri) => ri.to_string(),
                    other => format!("{other:?}"),
                })
            });

            let cashflows: Vec<Cashflow> = leg
                .cashflows()
                .map(|cf| {
                    let rate = match &cf.payoff {
                        Payoff::Fixed { rate } => Some(*rate),
                        _ => None,
                    };
                    let payoff_type = if cf.payoff.is_fixed() {
                        "Fixed"
                    } else if cf.payoff.is_linear() {
                        "Linear"
                    } else {
                        "Option"
                    };
                    let cf_rate_index = cf.payoff.required_index().map(|idx| match idx {
                        IndexType::Rate(ri) => ri.to_string(),
                        other => format!("{other:?}"),
                    });

                    Cashflow {
                        payment_date: format_date(cf.payment_date),
                        accrual_start: format_date(cf.accrual_start),
                        accrual_end: format_date(cf.accrual_end),
                        year_fraction: cf.year_fraction,
                        notional: cf.notional,
                        rate,
                        payoff_type: payoff_type.to_string(),
                        rate_index: cf_rate_index,
                    }
                })
                .collect();

            TradeLeg {
                direction: direction.to_string(),
                currency: leg.currency.code().to_string(),
                leg_type: leg_type_str.to_string(),
                rate_index: leg_rate_index,
                cashflows,
            }
        })
        .collect();

    let total_legs = dto_legs.len();
    let total_cashflows = dto_legs.iter().map(|l| l.cashflows.len()).sum();

    ExpandedTrade {
        trade_id: uuid::Uuid::new_v4().to_string(),
        trade_type: trade_type_label.to_string(),
        legs: dto_legs,
        metadata: TradeMetadata {
            total_legs,
            total_cashflows,
            processing_time_ms: elapsed.as_secs_f64() * 1000.0,
        },
    }
}

/// Build a domain `Trade` from instrument type and JSON params.
///
/// Shared by `expand_trade` (returns DTO) and `get_pricer_graph` (builds
/// computation graph from real structure).
fn build_domain_trade(
    instrument_type: &str,
    params: &serde_json::Value,
) -> Result<(Trade, &'static str), ServerError> {
    let param_str = |key: &str| params.get(key).and_then(|v| v.as_str());
    let param_f64 = |key: &str, default: f64| {
        params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    };
    let param_date = |key: &str| -> Result<Date, ServerError> {
        let s = param_str(key).ok_or_else(|| {
            ServerError::InvalidRequest(format!("Missing required param: {key}"))
        })?;
        parse_date(s)
    };

    let conventions = ConventionSet::default();
    let valuation_date = Date::from_ymd(2026, 1, 1)
        .map_err(|e| ServerError::Internal(format!("Default valuation date: {e}")))?;

    Ok(match instrument_type {
            "IRS" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let fixed_rate = param_f64("fixedRate", 0.05);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;

                let rate_index_str = param_str("rateIndex")
                    .map(String::from)
                    .unwrap_or_else(|| default_rate_index(currency).to_string());
                let rate_index = parse_rate_index(&rate_index_str)?;

                let swap_type = param_str("swapType").unwrap_or("Vanilla");

                if swap_type == "OIS" {
                    let ois = Ois {
                        rate_index,
                        fixed_rate,
                        start_date,
                        end_date,
                        notional,
                        currency,
                        payer_receiver: PayerReceiver::Payer,
                        payment_frequency: Frequency::Annual,
                    };
                    let t = ois
                        .expand_to_trade("DEMO-OIS", valuation_date, &conventions)
                        .map_err(|e| ServerError::Internal(format!("OIS expansion: {e}")))?;
                    (t, "OIS")
                } else {
                    let (fixed_freq, float_freq) = default_swap_frequencies(rate_index);
                    let tenor = compute_tenor(start_date, end_date);
                    let irs = InterestRateSwap {
                        start_date,
                        tenor,
                        fixed_rate,
                        spread: 0.0,
                        notional,
                        currency,
                        payer_receiver: PayerReceiver::Payer,
                        fixed_frequency: fixed_freq,
                        float_frequency: float_freq,
                        rate_index,
                    };
                    let t = irs
                        .expand_to_trade("DEMO-IRS", valuation_date, &conventions)
                        .map_err(|e| ServerError::Internal(format!("IRS expansion: {e}")))?;
                    (t, "IRS")
                }
            }
            "FxForward" => {
                let pair_str = param_str("currencyPair").unwrap_or("EURUSD");
                let pair = parse_currency_pair(pair_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let settlement_date = param_date("endDate")
                    .or_else(|_| param_date("settlementDate"))
                    .unwrap_or_else(|_| {
                        Tenor::SixMonths.add_to_date(
                            valuation_date,
                            infra_domain::time::EndOfMonthRule::Adjust,
                        )
                    });
                let forward_rate = param_f64("forwardRate", 1.0);

                let fx = FxForward {
                    currency_pair: pair,
                    forward_rate,
                    settlement_date,
                    notional,
                    notional_currency: pair.base,
                };
                let t = fx
                    .expand_to_trade("DEMO-FXFWD", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("FxForward expansion: {e}")))?;
                (t, "FxForward")
            }
            "BasisSwap" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let spread = param_f64("spread", 0.0) / 10_000.0;
                let rate_index_str = param_str("rateIndex")
                    .map(String::from)
                    .unwrap_or_else(|| default_rate_index(currency).to_string());
                let rate_index = parse_rate_index(&rate_index_str)?;
                let tenor = compute_tenor(start_date, end_date);

                let basis = BasisSwap {
                    start_date,
                    tenor,
                    notional,
                    currency,
                    payer_receiver: PayerReceiver::Payer,
                    leg1_index: rate_index,
                    leg1_spread: spread,
                    leg1_frequency: Frequency::Quarterly,
                    leg2_index: default_rate_index(currency),
                    leg2_spread: 0.0,
                    leg2_frequency: Frequency::Quarterly,
                };
                let t = basis
                    .expand_to_trade("DEMO-BASIS", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("BasisSwap expansion: {e}")))?;
                (t, "BasisSwap")
            }
            "Swaption" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 0.05);
                let expiry = param_date("expiry")?;

                let swaption = Swaption {
                    underlying_swap_tenor: Tenor::FiveYears,
                    expiry,
                    exercise_type: ExerciseStyle::European,
                    settlement_type: SettlementType::Cash,
                    strike,
                    notional,
                    currency,
                    payer_receiver: PayerReceiver::Payer,
                };
                let t = swaption
                    .expand_to_trade("DEMO-SWAPTION", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("Swaption expansion: {e}")))?;
                (t, "Swaption")
            }
            "CapFloor" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 0.05);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let rate_index_str = param_str("rateIndex")
                    .map(String::from)
                    .unwrap_or_else(|| default_rate_index(currency).to_string());
                let rate_index = parse_rate_index(&rate_index_str)?;
                let tenor = compute_tenor(start_date, end_date);

                let cap = CapFloor {
                    cap_floor_type: CapFloorType::Cap,
                    strikes: vec![strike],
                    index: rate_index,
                    start_date,
                    tenor,
                    notional_schedule: NotionalSchedule {
                        notionals: vec![notional],
                    },
                    payment_frequency: Frequency::Quarterly,
                    currency,
                };
                let t = cap
                    .expand_to_trade("DEMO-CAPFLOOR", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("CapFloor expansion: {e}")))?;
                (t, "CapFloor")
            }
            "Deposit" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let rate = param_f64("fixedRate", 0.05);
                let tenor = compute_tenor(start_date, end_date);

                let deposit = Deposit {
                    start_date,
                    tenor,
                    rate,
                    notional,
                    currency,
                };
                let t = deposit
                    .expand_to_trade("DEMO-DEPOSIT", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("Deposit expansion: {e}")))?;
                (t, "Deposit")
            }
            "Fra" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let strike = param_f64("strike", 0.05);
                let rate_index_str = param_str("rateIndex")
                    .map(String::from)
                    .unwrap_or_else(|| default_rate_index(currency).to_string());
                let rate_index = parse_rate_index(&rate_index_str)?;
                let tenor = compute_tenor(start_date, end_date);

                let fra = Fra {
                    fixing_date: start_date,
                    start_date,
                    tenor,
                    strike,
                    notional,
                    currency,
                    rate_index,
                };
                let t = fra
                    .expand_to_trade("DEMO-FRA", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("FRA expansion: {e}")))?;
                (t, "FRA")
            }
            "Bond" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let coupon_rate = param_f64("fixedRate", 0.03);

                let bond = Bond {
                    issuer: "DEMO".to_string(),
                    coupon_rate,
                    coupon_frequency: Frequency::SemiAnnual,
                    start_date,
                    maturity: end_date,
                    notional,
                    currency,
                    bond_type: BondType::Government,
                    rating: None,
                };
                let t = bond
                    .expand_to_trade("DEMO-BOND", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("Bond expansion: {e}")))?;
                (t, "Bond")
            }
            "FxVanillaOption" => {
                let pair_str = param_str("currencyPair").unwrap_or("EURUSD");
                let pair = parse_currency_pair(pair_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 1.1);
                let expiry = param_date("expiry")?;
                let option_type = match param_str("optionType") {
                    Some("Put") => OptionType::Put,
                    _ => OptionType::Call,
                };

                let opt = FxVanillaOption {
                    currency_pair: pair,
                    strike,
                    expiry,
                    delivery_date: expiry,
                    option_type,
                    exercise_style: ExerciseStyle::European,
                    notional,
                    notional_currency: pair.base,
                };
                let t = opt
                    .expand_to_trade("DEMO-FXOPT", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("FxVanillaOption expansion: {e}")))?;
                (t, "FxVanillaOption")
            }
            "FxBarrierOption" => {
                let pair_str = param_str("currencyPair").unwrap_or("EURUSD");
                let pair = parse_currency_pair(pair_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 1.1);
                let expiry = param_date("expiry")?;
                let option_type = match param_str("optionType") {
                    Some("Put") => OptionType::Put,
                    _ => OptionType::Call,
                };

                let vanilla = FxVanillaOption {
                    currency_pair: pair,
                    strike,
                    expiry,
                    delivery_date: expiry,
                    option_type,
                    exercise_style: ExerciseStyle::European,
                    notional,
                    notional_currency: pair.base,
                };
                let barrier = FxBarrierOption {
                    vanilla,
                    barrier_level: strike * 1.1,
                    barrier_type: BarrierType::KnockOut,
                    barrier_direction: BarrierDirection::Up,
                    rebate: None,
                };
                let t = barrier
                    .expand_to_trade("DEMO-FXBARRIER", valuation_date, &conventions)
                    .map_err(|e| {
                        ServerError::Internal(format!("FxBarrierOption expansion: {e}"))
                    })?;
                (t, "FxBarrierOption")
            }
            "EquityForward" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let settlement_date = param_date("endDate")?;

                let eq_fwd = EquityForward {
                    underlying: EquityUnderlying::Index {
                        name: "SPX".to_string(),
                    },
                    forward_price: 100.0,
                    settlement_date,
                    notional,
                    currency,
                };
                let t = eq_fwd
                    .expand_to_trade("DEMO-EQFWD", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("EquityForward expansion: {e}")))?;
                (t, "EquityForward")
            }
            "EquityVanillaOption" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 100.0);
                let expiry = param_date("expiry")?;
                let option_type = match param_str("optionType") {
                    Some("Put") => OptionType::Put,
                    _ => OptionType::Call,
                };

                let eq_opt = EquityVanillaOption {
                    underlying: EquityUnderlying::Index {
                        name: "SPX".to_string(),
                    },
                    strike,
                    expiry,
                    option_type,
                    exercise_style: ExerciseStyle::European,
                    notional,
                    currency,
                };
                let t = eq_opt
                    .expand_to_trade("DEMO-EQOPT", valuation_date, &conventions)
                    .map_err(|e| {
                        ServerError::Internal(format!("EquityVanillaOption expansion: {e}"))
                    })?;
                (t, "EquityVanillaOption")
            }
            "AsianOption" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let strike = param_f64("strike", 100.0);
                let expiry = param_date("expiry")?;
                let option_type = match param_str("optionType") {
                    Some("Put") => OptionType::Put,
                    _ => OptionType::Call,
                };

                let asian = AsianOption {
                    underlying: EquityUnderlying::Index {
                        name: "SPX".to_string(),
                    },
                    strike,
                    expiry,
                    option_type,
                    averaging_type: AveragingType::Arithmetic,
                    observation_frequency: Frequency::Monthly,
                    observed_values: vec![],
                    notional,
                    currency,
                };
                let t = asian
                    .expand_to_trade("DEMO-ASIAN", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("AsianOption expansion: {e}")))?;
                (t, "AsianOption")
            }
            "CDS" => {
                let currency_str = param_str("currency").unwrap_or("USD");
                let currency = parse_currency(currency_str)?;
                let notional = param_f64("notional", 1_000_000.0);
                let start_date = param_date("startDate")?;
                let end_date = param_date("endDate")?;
                let spread = param_f64("spread", 100.0) / 10_000.0;

                let cds = Cds {
                    reference_entity: "DEMO Corp".to_string(),
                    notional,
                    spread,
                    start_date,
                    maturity: end_date,
                    recovery_rate: Some(0.4),
                    currency,
                    credit_events: vec![
                        CreditEvent::Bankruptcy,
                        CreditEvent::FailureToPay,
                    ],
                };
                let t = cds
                    .expand_to_trade("DEMO-CDS", valuation_date, &conventions)
                    .map_err(|e| ServerError::Internal(format!("CDS expansion: {e}")))?;
                (t, "CDS")
            }
            other => {
                return Err(ServerError::InvalidRequest(format!(
                    "Instrument expansion not yet supported: {other}"
                )))
            }
        })
}

impl DemoService {
    /// Expand a trade request into cashflows using `InstrumentExpander`.
    pub fn expand_trade(
        request: &TradeExpandRequest,
        _state: &Arc<AppState>,
    ) -> Result<ExpandedTrade, ServerError> {
        let start = Instant::now();
        let (trade, trade_type_label) =
            build_domain_trade(&request.instrument_type, &request.params)?;
        let elapsed = start.elapsed();
        Ok(domain_trade_to_dto(&trade, trade_type_label, elapsed))
    }

    /// Price a trade using the unified `Pricer`, returning full
    /// `PricingResult`.
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

        let path_distribution = result
            .path_distribution
            .as_ref()
            .map(|pd| DemoPathDistribution {
                mean: pd.mean,
                std_dev: pd.std_dev,
                percentiles: pd.percentiles.clone(),
                path_count: pd.path_count,
            });

        // Inline Greeks via bump-and-revalue if requested.
        let greeks = if request.compute_greeks {
            let valuation_date = parse_date(&request.valuation_date)?;
            let reporting_currency = parse_currency(&request.reporting_currency)?;
            let domain_legs = build_domain_legs(&request.legs, valuation_date)?;
            let greeks_trade = Trade::new("DEMO-GREEKS", domain_legs, TradeType::Swap);

            let rate_bump = 1.0 / 10000.0; // 1bp
            let pv_up = price_with_rate(
                &greeks_trade,
                valuation_date,
                reporting_currency,
                &request.legs,
                DEFAULT_DISCOUNT_RATE + rate_bump,
            )?;
            let pv_down = price_with_rate(
                &greeks_trade,
                valuation_date,
                reporting_currency,
                &request.legs,
                DEFAULT_DISCOUNT_RATE - rate_bump,
            )?;

            let delta = (pv_up - pv_down) / 2.0;
            let gamma = pv_up - 2.0 * result.total_pv + pv_down;

            // Theta: shift valuation date +1d.
            let theta_inner = valuation_date.into_inner() + chrono::Duration::days(1);
            let theta_date =
                Date::from_ymd(theta_inner.year(), theta_inner.month(), theta_inner.day()).ok();
            let theta = theta_date
                .and_then(|td| {
                    let theta_legs = build_domain_legs(&request.legs, td).ok()?;
                    let theta_trade = Trade::new("DEMO-THETA", theta_legs, TradeType::Swap);
                    price_with_rate(
                        &theta_trade,
                        td,
                        reporting_currency,
                        &request.legs,
                        DEFAULT_DISCOUNT_RATE,
                    )
                    .ok()
                })
                .map(|pv| pv - result.total_pv);

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

    /// Calculate advanced Greeks per risk factor, mirroring
    /// `pricer_risk::GreeksResultByFactor`.
    ///
    /// Bumps each currency's discount curve independently to produce
    /// per-factor delta, gamma, and rho.  Theta is computed as a global
    /// "Time" factor.  Vega/vanna/volga are `None` (no vol surface in demo).
    pub fn calculate_advanced_greeks(
        request: &DemoAdvancedGreeksRequest,
        _state: &Arc<AppState>,
    ) -> Result<DemoAdvancedGreeksResult, ServerError> {
        // Reject EnzymeAAD in demo mode.
        if matches!(request.config.mode, AdvancedGreeksMode::EnzymeAad) {
            return Err(ServerError::InvalidRequest(
                "EnzymeAAD requires nightly + pricer_risk. Use BumpRevalue in demo.".to_string(),
            ));
        }

        let start = Instant::now();
        let valuation_date = parse_date(&request.valuation_date)?;
        let reporting_currency = parse_currency(&request.reporting_currency)?;

        let domain_legs = build_domain_legs(&request.legs, valuation_date)?;
        let trade = Trade::new("DEMO-ADV-GREEKS", domain_legs, TradeType::Swap);

        let cfg = &request.config;
        let h = cfg.rate_bump_absolute;

        // Collect unique currencies from legs.
        let currencies: Vec<Currency> = {
            let mut seen = std::collections::HashSet::new();
            let mut ccys = Vec::new();
            for leg in &request.legs {
                let ccy = parse_currency(&leg.currency)?;
                if seen.insert(ccy) {
                    ccys.push(ccy);
                }
            }
            // Ensure reporting currency is included.
            if seen.insert(reporting_currency) {
                ccys.push(reporting_currency);
            }
            ccys
        };

        // Build base rates: all currencies at DEFAULT_DISCOUNT_RATE.
        let base_rates: Vec<(Currency, f64)> =
            currencies.iter().map(|&c| (c, DEFAULT_DISCOUNT_RATE)).collect();

        // Base PV.
        let base_pv = price_with_rates(
            &trade,
            valuation_date,
            reporting_currency,
            &base_rates,
            &request.legs,
        )?;

        // Per-currency delta & gamma (Curve factors).
        let mut factors = Vec::new();
        let mut total_delta = 0.0_f64;
        let mut total_gamma = 0.0_f64;
        let mut total_rho = 0.0_f64;
        let mut has_delta = false;

        for &ccy in &currencies {
            // Bump only this currency's rate up.
            let rates_up: Vec<(Currency, f64)> = currencies
                .iter()
                .map(|&c| {
                    if c == ccy {
                        (c, DEFAULT_DISCOUNT_RATE + h)
                    } else {
                        (c, DEFAULT_DISCOUNT_RATE)
                    }
                })
                .collect();
            let pv_up = price_with_rates(
                &trade,
                valuation_date,
                reporting_currency,
                &rates_up,
                &request.legs,
            )?;

            // Bump only this currency's rate down.
            let rates_down: Vec<(Currency, f64)> = currencies
                .iter()
                .map(|&c| {
                    if c == ccy {
                        (c, DEFAULT_DISCOUNT_RATE - h)
                    } else {
                        (c, DEFAULT_DISCOUNT_RATE)
                    }
                })
                .collect();
            let pv_down = price_with_rates(
                &trade,
                valuation_date,
                reporting_currency,
                &rates_down,
                &request.legs,
            )?;

            let delta = (pv_up - pv_down) / (2.0 * h);
            let gamma = (pv_up - 2.0 * base_pv + pv_down) / (h * h);
            let rho = delta; // Linear products: rho ≈ rate delta.

            total_delta += delta;
            total_gamma += gamma;
            total_rho += rho;
            has_delta = true;

            factors.push(FactorGreeksEntry {
                factor: RiskFactor {
                    factor_type: "Curve".to_string(),
                    name: ccy.code().to_string(),
                },
                greeks: FactorGreeks {
                    delta: Some(delta),
                    gamma: Some(gamma),
                    vega: None,
                    theta: None,
                    rho: Some(rho),
                    vanna: None,
                    volga: None,
                },
            });
        }

        // Theta: shift valuation date forward (Time factor).
        let bump_days = (cfg.time_bump_years * 365.0).round() as i64;
        let theta_inner = valuation_date.into_inner() + chrono::Duration::days(bump_days);
        let theta = Date::from_ymd(theta_inner.year(), theta_inner.month(), theta_inner.day())
            .ok()
            .and_then(|td| {
                let theta_legs = build_domain_legs(&request.legs, td).ok()?;
                let theta_trade = Trade::new("DEMO-ADV-THETA", theta_legs, TradeType::Swap);
                price_with_rates(
                    &theta_trade,
                    td,
                    reporting_currency,
                    &base_rates,
                    &request.legs,
                )
                .ok()
            })
            .map(|pv| (pv - base_pv) / cfg.time_bump_years);

        factors.push(FactorGreeksEntry {
            factor: RiskFactor {
                factor_type: "Time".to_string(),
                name: "Theta".to_string(),
            },
            greeks: FactorGreeks {
                delta: None,
                gamma: None,
                vega: None,
                theta,
                rho: None,
                vanna: None,
                volga: None,
            },
        });

        // Totals.
        let totals = FactorGreeks {
            delta: if has_delta { Some(total_delta) } else { None },
            gamma: if has_delta { Some(total_gamma) } else { None },
            vega: None,
            theta,
            rho: if has_delta { Some(total_rho) } else { None },
            vanna: None,
            volga: None,
        };

        let elapsed = start.elapsed();

        Ok(DemoAdvancedGreeksResult {
            price: base_pv,
            currency: request.reporting_currency.clone(),
            mode: "BumpRevalue".to_string(),
            computation_time_ms: elapsed.as_secs_f64() * 1000.0,
            factors,
            totals,
        })
    }

    /// Generate a computation graph from the real expanded trade structure.
    pub fn get_pricer_graph(
        request: &PricerGraphRequest,
    ) -> Result<PricerGraphResponse, ServerError> {
        let (trade, label) =
            build_domain_trade(&request.instrument_type, &request.params)?;
        let trade_id = format!("PRICER-{label}");
        let now = chrono::Utc::now().to_rfc3339();
        let detail = request.detail_level.as_deref().unwrap_or("scope");

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // --- Market data input nodes (sensitivity targets) ---
        // Collect unique discount curves (per currency) and forward curves
        // (per rate index) from the actual legs.
        let mut discount_inputs: Vec<(String, String)> = Vec::new(); // (id, label)
        let mut forward_inputs: Vec<(String, String)> = Vec::new();
        let mut seen_ccy = std::collections::HashSet::new();
        let mut seen_idx = std::collections::HashSet::new();

        for leg in trade.legs() {
            let ccy = leg.currency.code();
            if seen_ccy.insert(ccy.to_string()) {
                let id = format!("{trade_id}_disc_{ccy}");
                discount_inputs.push((id, format!("Disc:{ccy}")));
            }
            for cf in leg.cashflows() {
                if let Some(idx) = cf.payoff.required_index() {
                    let idx_str = match idx {
                        IndexType::Rate(ri) => ri.to_string(),
                        other => format!("{other:?}"),
                    };
                    if seen_idx.insert(idx_str.clone()) {
                        let id = format!("{trade_id}_fwd_{idx_str}");
                        forward_inputs.push((id, format!("Fwd:{idx_str}")));
                    }
                }
            }
        }

        // Push input nodes.
        for (id, label) in discount_inputs.iter().chain(forward_inputs.iter()) {
            nodes.push(PricerGraphNode {
                id: id.clone(),
                node_type: "Input".to_string(),
                label: label.clone(),
                value: None,
                is_sensitivity_target: true,
                group: "Sensitivity".to_string(),
                trade_ids: vec![trade_id.clone()],
            });
        }

        // --- Per-leg nodes ---
        let mut leg_pv_ids = Vec::new();

        for (leg_idx, leg) in trade.legs().enumerate() {
            let dir = match leg.direction {
                Direction::Payer => "Pay",
                Direction::Receiver => "Rec",
            };
            let lt = match leg.leg_type {
                LegType::Fixed => "Fixed",
                LegType::Floating => "Float",
                LegType::CapFloor => "CapFloor",
                LegType::Principal => "Principal",
                LegType::Generic => "Generic",
            };
            let ccy = leg.currency.code();
            let cf_count = leg.cashflows().count();

            // Determine which market inputs this leg depends on.
            let disc_id = format!("{trade_id}_disc_{ccy}");
            let fwd_ids: Vec<String> = leg
                .cashflows()
                .filter_map(|cf| {
                    cf.payoff.required_index().map(|idx| {
                        let s = match idx {
                            IndexType::Rate(ri) => ri.to_string(),
                            other => format!("{other:?}"),
                        };
                        format!("{trade_id}_fwd_{s}")
                    })
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if detail == "operation" {
                // Payoff evaluation node.
                let payoff_id = format!("{trade_id}_L{leg_idx}_payoff");
                let payoff_label = if leg.leg_type == LegType::Floating {
                    format!("{lt} payoff ({cf_count} CFs)")
                } else {
                    let rate = leg
                        .cashflows()
                        .next()
                        .and_then(|cf| match &cf.payoff {
                            Payoff::Fixed { rate } => Some(*rate),
                            _ => None,
                        });
                    match rate {
                        Some(r) => format!("{lt} payoff r={r:.4} ({cf_count} CFs)"),
                        None => format!("{lt} payoff ({cf_count} CFs)"),
                    }
                };
                nodes.push(PricerGraphNode {
                    id: payoff_id.clone(),
                    node_type: "Mul".to_string(),
                    label: payoff_label,
                    value: None,
                    is_sensitivity_target: false,
                    group: "Intermediate".to_string(),
                    trade_ids: vec![trade_id.clone()],
                });
                // Forward curves → payoff.
                for fid in &fwd_ids {
                    edges.push(PricerGraphEdge {
                        source: fid.clone(),
                        target: payoff_id.clone(),
                        weight: None,
                    });
                }

                // Discount node.
                let disc_node_id = format!("{trade_id}_L{leg_idx}_disc");
                nodes.push(PricerGraphNode {
                    id: disc_node_id.clone(),
                    node_type: "Mul".to_string(),
                    label: format!("DF({ccy})"),
                    value: None,
                    is_sensitivity_target: false,
                    group: "Intermediate".to_string(),
                    trade_ids: vec![trade_id.clone()],
                });
                edges.push(PricerGraphEdge {
                    source: disc_id,
                    target: disc_node_id.clone(),
                    weight: None,
                });
                edges.push(PricerGraphEdge {
                    source: payoff_id,
                    target: disc_node_id.clone(),
                    weight: None,
                });

                // Leg PV aggregation.
                let leg_pv_id = format!("{trade_id}_L{leg_idx}_pv");
                nodes.push(PricerGraphNode {
                    id: leg_pv_id.clone(),
                    node_type: "Add".to_string(),
                    label: format!("{dir} {lt} {ccy}"),
                    value: None,
                    is_sensitivity_target: false,
                    group: "Leg".to_string(),
                    trade_ids: vec![trade_id.clone()],
                });
                edges.push(PricerGraphEdge {
                    source: disc_node_id,
                    target: leg_pv_id.clone(),
                    weight: None,
                });
                leg_pv_ids.push(leg_pv_id);
            } else {
                // Scope: single node per leg.
                let leg_node_id = format!("{trade_id}_L{leg_idx}");
                nodes.push(PricerGraphNode {
                    id: leg_node_id.clone(),
                    node_type: "Add".to_string(),
                    label: format!("{dir} {lt} {ccy} ({cf_count} CFs)"),
                    value: None,
                    is_sensitivity_target: false,
                    group: "Leg".to_string(),
                    trade_ids: vec![trade_id.clone()],
                });
                // Connect market inputs → leg.
                edges.push(PricerGraphEdge {
                    source: disc_id,
                    target: leg_node_id.clone(),
                    weight: None,
                });
                for fid in &fwd_ids {
                    edges.push(PricerGraphEdge {
                        source: fid.clone(),
                        target: leg_node_id.clone(),
                        weight: None,
                    });
                }
                leg_pv_ids.push(leg_node_id);
            }
        }

        // --- Output node ---
        let out_id = format!("{trade_id}_pv");
        nodes.push(PricerGraphNode {
            id: out_id.clone(),
            node_type: "Output".to_string(),
            label: "Trade PV".to_string(),
            value: None,
            is_sensitivity_target: false,
            group: "Output".to_string(),
            trade_ids: vec![trade_id.clone()],
        });
        for lid in &leg_pv_ids {
            edges.push(PricerGraphEdge {
                source: lid.clone(),
                target: out_id.clone(),
                weight: None,
            });
        }

        let node_count = nodes.len();
        let edge_count = edges.len();
        let depth = if detail == "operation" { 5 } else { 3 };

        Ok(PricerGraphResponse {
            nodes,
            edges,
            metadata: PricerGraphMetadata {
                node_count,
                edge_count,
                depth,
                generated_at: now,
                trade_count: 1,
                shared_node_count: 0,
                optimisation_ratio: 1.0,
                trade_id: Some(trade_id),
                source_locations: None,
            },
        })
    }
}

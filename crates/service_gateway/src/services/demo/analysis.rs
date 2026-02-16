//! Rate detail, instrument analysis, and cashflow generation.

use std::{sync::Arc, time::Instant};

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::demo::{
        CashflowDetail, Convention, ConventionDetail, CurveInstrument, CurveInstrumentsResponse,
        LegCashflows, MarketRate, MarketRateDetailResponse, RateCashflowsResponse,
        RateInstrumentResponse,
    },
    services::helpers,
    state::AppState,
};

impl DemoService {
    /// Get rate detail.
    pub fn get_rate_detail(
        rate_id: &str,
        state: &Arc<AppState>,
    ) -> Result<MarketRateDetailResponse, ServerError> {
        let rates_response = Self::get_market_rates(state)?;
        let rate = rates_response
            .rates
            .into_iter()
            .find(|r| r.id == rate_id)
            .ok_or_else(|| ServerError::NotFound(format!("Rate {} not found", rate_id)))?;

        let instrument = Self::build_instrument_description(&rate);

        let convention = Self::find_convention_for_rate(&rate, state);

        Ok(MarketRateDetailResponse {
            rate,
            instrument,
            convention,
        })
    }

    /// Build instrument description for a rate.
    fn build_instrument_description(rate: &MarketRate) -> Option<serde_json::Value> {
        let description = match rate.rate_type.as_str() {
            "deposit" => serde_json::json!({
                "type": "Money Market Deposit",
                "description": format!(
                    "{} {} deposit rate. A money market instrument representing the cost of \
                     borrowing or lending {} for the {} period.",
                    rate.currency, rate.tenor, rate.currency, rate.tenor
                ),
                "usage": "Used for short-end curve construction and discounting",
                "quoteConvention": "Simple rate, typically ACT/360 (USD/EUR) or ACT/365F (GBP/JPY)",
                "settlementDays": if rate.currency == "GBP" { 0 } else { 2 }
            }),
            "ois" => serde_json::json!({
                "type": "Overnight Index Swap",
                "description": format!(
                    "{} {} OIS rate. An interest rate swap where the floating leg pays the \
                     compounded overnight rate ({}) versus a fixed rate.",
                    rate.currency, rate.tenor,
                    rate.rate_index.as_deref().unwrap_or("overnight index")
                ),
                "usage": "Primary instrument for building risk-free discount curves",
                "quoteConvention": "Par swap rate quoted as annual rate",
                "index": rate.rate_index.clone()
            }),
            "swap" => serde_json::json!({
                "type": "Interest Rate Swap",
                "description": format!(
                    "{} {} IRS rate. A vanilla interest rate swap exchanging fixed rate payments \
                     for floating rate payments indexed to {}.",
                    rate.currency, rate.tenor,
                    rate.rate_index.as_deref().unwrap_or("floating index")
                ),
                "usage": "Key instrument for yield curve construction at longer tenors",
                "quoteConvention": "Par swap rate, fixed leg vs floating leg",
                "index": rate.rate_index.clone()
            }),
            "fra" => serde_json::json!({
                "type": "Forward Rate Agreement",
                "description": format!(
                    "{} {} FRA. A forward contract to exchange a fixed rate for a floating rate \
                     on a notional amount for the specified period.",
                    rate.currency, rate.tenor
                ),
                "usage": "Used for constructing the forward curve between deposit and swap tenors",
                "quoteConvention": "FRA rate quoted as simple forward rate",
                "index": rate.rate_index.clone()
            }),
            "future" => serde_json::json!({
                "type": "Interest Rate Future",
                "description": format!(
                    "{} {} interest rate future. A standardised exchange-traded contract \
                     on the future value of an interest rate index ({}).",
                    rate.currency, rate.tenor,
                    rate.rate_index.as_deref().unwrap_or("rate index")
                ),
                "usage": "Used for curve construction and hedging interest rate exposure",
                "quoteConvention": "Price = 100 - Rate (IMM convention)",
                "index": rate.rate_index.clone()
            }),
            "fxspot" => {
                let pair = &rate.id;
                let base = if pair.len() >= 3 { &pair[..3] } else { "" };
                let quote = if pair.len() >= 6 { &pair[3..6] } else { "" };
                serde_json::json!({
                    "type": "FX Spot Rate",
                    "description": format!(
                        "{}/{} spot exchange rate. The current market rate to exchange {} for {}. \
                         Quote convention: 1 {} = {} {}.",
                        base, quote, base, quote, base, rate.value, quote
                    ),
                    "usage": "Used for FX conversions, cross-currency discounting, and FX derivative pricing",
                    "quoteConvention": format!("{}/{} (1 {} = x {})", base, quote, base, quote),
                    "settlementDays": 2
                })
            }
            "fxforward" => {
                let pair = rate.rate_index.as_deref().unwrap_or("");
                let base = if pair.len() >= 3 { &pair[..3] } else { "" };
                let quote = if pair.len() >= 6 { &pair[3..6] } else { "" };
                serde_json::json!({
                    "type": "FX Forward Points",
                    "description": format!(
                        "{}/{} {} forward points. The difference between the forward rate and \
                         spot rate, quoted in pips. Points: {} pips.",
                        base, quote, rate.tenor, rate.value
                    ),
                    "usage": "Used for FX forward pricing and cross-currency curve construction",
                    "quoteConvention": "Forward points in pips (1 pip = 0.0001 for most pairs)",
                    "settlementDays": 2,
                    "pair": pair
                })
            }
            "xccybasis" => {
                let pair = rate
                    .id
                    .strip_prefix("XCCY-")
                    .and_then(|s| s.rsplit_once('-'))
                    .map(|(p, _)| p)
                    .unwrap_or("");
                let base = if pair.len() >= 3 { &pair[..3] } else { "" };
                let quote = if pair.len() >= 6 { &pair[3..6] } else { "" };
                serde_json::json!({
                    "type": "Cross-Currency Basis Swap",
                    "description": format!(
                        "{}/{} {} cross-currency basis swap spread. The spread added to one leg \
                         of a cross-currency swap to equate the present values. Index: {}.",
                        base, quote, rate.tenor,
                        rate.rate_index.as_deref().unwrap_or("N/A")
                    ),
                    "usage": "Used for cross-currency curve construction and hedging FX funding basis risk",
                    "quoteConvention": "Basis spread in decimal (e.g., -0.001 = -10bp)",
                    "index": rate.rate_index.clone(),
                    "pair": pair
                })
            }
            _ => return None,
        };
        Some(description)
    }

    /// Find matching convention for a rate.
    fn find_convention_for_rate(rate: &MarketRate, state: &Arc<AppState>) -> Option<Convention> {
        let conventions_result = Self::get_conventions(state).ok()?;

        let convention_id = match rate.rate_type.as_str() {
            "deposit" => Some(format!("{}-DEPO", rate.currency)),
            "ois" => Some(format!(
                "{}-{}-OIS",
                rate.currency,
                rate.rate_index.as_deref().unwrap_or("OIS")
            )),
            "swap" => {
                let index = rate.rate_index.as_deref().unwrap_or("SWAP");
                Some(format!("{}-{}-SWAP", rate.currency, index))
            }
            "fra" | "future" => Some(format!(
                "{}-{}-OIS",
                rate.currency,
                rate.rate_index.as_deref().unwrap_or("OIS")
            )),
            "fxspot" => Some("FX-SPOT".to_string()),
            "fxforward" => Some("FX-SPOT".to_string()),
            "xccybasis" => {
                let pair = rate
                    .id
                    .strip_prefix("XCCY-")
                    .and_then(|s| s.rsplit_once('-'))
                    .map(|(p, _)| p)
                    .unwrap_or("");
                Some(format!("XCCY-{}", pair))
            }
            _ => None,
        };

        if let Some(id) = convention_id {
            if let Some(conv) = conventions_result.conventions.iter().find(|c| c.id == id) {
                return Some(conv.clone());
            }
        }

        conventions_result.conventions.into_iter().find(|c| {
            c.currency == rate.currency
                && c.convention_type.to_lowercase().contains(&rate.rate_type)
                && c.is_default == Some(true)
        })
    }

    /// Get instrument details for a rate.
    pub fn get_rate_instrument(
        rate_id: &str,
        state: &Arc<AppState>,
    ) -> Result<RateInstrumentResponse, ServerError> {
        let start = Instant::now();

        let rate_detail = Self::get_rate_detail(rate_id, state)?;
        let rate = &rate_detail.rate;

        let valuation_date = chrono::Utc::now().date_naive();
        let (effective_date, maturity_date) =
            Self::calculate_dates_from_tenor(&rate.tenor, &rate.currency, valuation_date);

        let convention = rate_detail.convention.map(|conv| {
            let mut day_count = None;
            let mut frequency = None;
            let mut business_day_convention = None;
            let mut spot_lag = None;
            let mut calendar = None;

            if let Some(fields) = &conv.fields {
                for field in fields {
                    match field.label.to_lowercase().as_str() {
                        "day count" | "daycount" | "day counter" => {
                            day_count = Some(field.value.clone());
                        }
                        "frequency" | "payment frequency" => {
                            frequency = Some(field.value.clone());
                        }
                        "business day convention" | "bdc" => {
                            business_day_convention = Some(field.value.clone());
                        }
                        "spot lag" | "settlement days" => {
                            spot_lag = field.value.parse().ok();
                        }
                        "calendar" | "calendars" => {
                            calendar = Some(field.value.clone());
                        }
                        _ => {}
                    }
                }
            }

            ConventionDetail {
                convention_type: conv.convention_type,
                day_count,
                frequency,
                business_day_convention,
                spot_lag,
                calendar,
            }
        });

        let instrument_type = match rate.rate_type.as_str() {
            "deposit" => "Money Market Deposit",
            "ois" => "Overnight Index Swap",
            "swap" => "Interest Rate Swap",
            "fra" => "Forward Rate Agreement",
            "future" => "Interest Rate Future",
            "fxspot" => "FX Spot",
            "fxforward" => "FX Forward",
            "xccybasis" => "Cross-Currency Basis Swap",
            other => other,
        };

        let elapsed = start.elapsed();

        Ok(RateInstrumentResponse {
            rate_id: rate.id.clone(),
            rate_value: rate.value,
            instrument_type: instrument_type.to_string(),
            convention,
            effective_date: effective_date.to_string(),
            maturity_date: maturity_date.to_string(),
            notional: 1_000_000.0,
            processing_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Get cashflows for a rate instrument.
    pub fn get_rate_cashflows(
        rate_id: &str,
        state: &Arc<AppState>,
    ) -> Result<RateCashflowsResponse, ServerError> {
        let start = Instant::now();

        let rate_detail = Self::get_rate_detail(rate_id, state)?;
        let rate = &rate_detail.rate;

        let valuation_date = chrono::Utc::now().date_naive();
        let (effective_date, maturity_date) =
            Self::calculate_dates_from_tenor(&rate.tenor, &rate.currency, valuation_date);

        let yf = Self::calculate_year_fraction(effective_date, maturity_date, &rate.currency);
        let mk_cf =
            |pay_date: chrono::NaiveDate, r: Option<f64>, spread: Option<f64>, payoff: &str| {
                CashflowDetail {
                    payment_date: pay_date.to_string(),
                    accrual_start: effective_date.to_string(),
                    accrual_end: maturity_date.to_string(),
                    year_fraction: yf,
                    notional: 1_000_000.0,
                    rate: r,
                    spread,
                    payoff_type: payoff.to_string(),
                }
            };
        let mk_leg = |lt: &str, dir: &str, idx: Option<String>, cf: CashflowDetail| LegCashflows {
            leg_type: lt.to_string(),
            direction: dir.to_string(),
            currency: rate.currency.clone(),
            rate_index: idx,
            cashflows: vec![cf],
        };

        let legs = match rate.rate_type.as_str() {
            "deposit" => vec![mk_leg(
                "Fixed",
                "Receiver",
                None,
                mk_cf(maturity_date, Some(rate.value), None, "Fixed"),
            )],
            "ois" | "swap" => vec![
                mk_leg(
                    "Fixed",
                    "Payer",
                    None,
                    mk_cf(maturity_date, Some(rate.value), None, "Fixed"),
                ),
                mk_leg(
                    "Floating",
                    "Receiver",
                    rate.rate_index.clone(),
                    mk_cf(maturity_date, None, Some(0.0), "Linear"),
                ),
            ],
            "fra" => vec![mk_leg(
                "FRA",
                "Payer",
                rate.rate_index.clone(),
                mk_cf(effective_date, Some(rate.value), None, "FRA"),
            )],
            _ => vec![mk_leg(
                "Unknown",
                "Unknown",
                None,
                mk_cf(maturity_date, Some(rate.value), None, "Other"),
            )],
        };

        let elapsed = start.elapsed();

        Ok(RateCashflowsResponse {
            rate_id: rate.id.clone(),
            legs,
            processing_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Calculate dates from tenor.
    pub(super) fn calculate_dates_from_tenor(
        tenor: &str,
        currency: &str,
        valuation_date: chrono::NaiveDate,
    ) -> (chrono::NaiveDate, chrono::NaiveDate) {
        let spot_lag = if currency == "GBP" { 0 } else { 2 };
        let effective_date = valuation_date + chrono::Duration::days(spot_lag);

        let t = tenor.to_uppercase();
        let maturity_date = match t.as_str() {
            "ON" | "TN" => effective_date + chrono::Duration::days(1),
            "SPOT" => effective_date,
            _ => {
                if let Some(years) = t.strip_suffix('Y').and_then(|s| s.parse::<i32>().ok()) {
                    Self::add_months(effective_date, years * 12)
                } else if let Some(months) = t.strip_suffix('M').and_then(|s| s.parse::<i32>().ok())
                {
                    Self::add_months(effective_date, months)
                } else if let Some(weeks) = t.strip_suffix('W').and_then(|s| s.parse::<i64>().ok())
                {
                    effective_date + chrono::Duration::weeks(weeks)
                } else if let Some(days) = t.strip_suffix('D').and_then(|s| s.parse::<i64>().ok()) {
                    effective_date + chrono::Duration::days(days)
                } else {
                    Self::add_months(effective_date, 12)
                }
            }
        };

        (effective_date, maturity_date)
    }

    /// Add months to a date.
    pub(super) fn add_months(date: chrono::NaiveDate, months: i32) -> chrono::NaiveDate {
        use chrono::Datelike;

        let total_months = date.month0() as i32 + months;
        let years_to_add = total_months / 12;
        let new_month = (total_months % 12) as u32 + 1;
        let new_year = date.year() + years_to_add;

        let max_day = match new_month {
            2 => {
                if new_year % 4 == 0 && (new_year % 100 != 0 || new_year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let new_day = std::cmp::min(date.day(), max_day);

        chrono::NaiveDate::from_ymd_opt(new_year, new_month, new_day).unwrap_or(date)
    }

    /// Calculate year fraction.
    pub(super) fn calculate_year_fraction(
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        currency: &str,
    ) -> f64 {
        let days = (end - start).num_days() as f64;
        let year_basis = if currency == "GBP" || currency == "JPY" {
            365.0
        } else {
            360.0
        };
        days / year_basis
    }

    /// Resolve a tenor string (e.g. "TD", "3M", "1Y") to an ISO date.
    ///
    /// Accepts ISO dates ("2026-03-15") as pass-through.
    /// An optional base date may be provided; defaults to today.
    pub fn resolve_tenor(
        request: &crate::rest::dto::demo::ResolveTenorRequest,
    ) -> Result<crate::rest::dto::demo::ResolveTenorResponse, ServerError> {
        let base = match &request.base {
            Some(b) => chrono::NaiveDate::parse_from_str(b, "%Y-%m-%d").map_err(|_| {
                ServerError::InvalidRequest(format!("Invalid base date: {b}"))
            })?,
            None => chrono::Utc::now().date_naive(),
        };

        let t = request.tenor.trim().to_uppercase();

        // Pass-through ISO date.
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&t, "%Y-%m-%d") {
            return Ok(crate::rest::dto::demo::ResolveTenorResponse {
                date: d.format("%Y-%m-%d").to_string(),
            });
        }

        // Today aliases.
        if matches!(t.as_str(), "TD" | "TODAY" | "T") {
            return Ok(crate::rest::dto::demo::ResolveTenorResponse {
                date: base.format("%Y-%m-%d").to_string(),
            });
        }

        // Parse <N><D|W|M|Y>.
        let (num_str, unit) = t
            .strip_suffix('D')
            .map(|n| (n, 'D'))
            .or_else(|| t.strip_suffix('W').map(|n| (n, 'W')))
            .or_else(|| t.strip_suffix('M').map(|n| (n, 'M')))
            .or_else(|| t.strip_suffix('Y').map(|n| (n, 'Y')))
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!("Unrecognised tenor: {}", request.tenor))
            })?;

        let n: i32 = num_str.parse().map_err(|_| {
            ServerError::InvalidRequest(format!("Invalid tenor number: {num_str}"))
        })?;

        let resolved = match unit {
            'D' => base + chrono::Duration::days(i64::from(n)),
            'W' => base + chrono::Duration::weeks(i64::from(n)),
            'M' => Self::add_months(base, n),
            'Y' => Self::add_months(base, n * 12),
            _ => unreachable!(),
        };

        Ok(crate::rest::dto::demo::ResolveTenorResponse {
            date: resolved.format("%Y-%m-%d").to_string(),
        })
    }

    /// Get instruments for a specific curve index.
    pub fn get_curve_instruments(
        index: &str,
        _state: &Arc<AppState>,
    ) -> Result<CurveInstrumentsResponse, ServerError> {
        let rates_path = std::path::Path::new("demo/data/input/rates/market_quotes.json");
        let data: serde_json::Value = helpers::load_json_value(rates_path, "market_quotes.json")?;

        let currency = match index {
            "SOFR" => "USD",
            "ESTR" => "EUR",
            "TONA" => "JPY",
            "SONIA" => "GBP",
            _ => return Err(ServerError::NotFound(format!("Unknown index: {}", index))),
        };

        let mut instruments = Vec::new();

        if let Some(rates_by_currency) = data.get("rates").and_then(|r| r.as_object()) {
            if let Some(rate_types) = rates_by_currency.get(currency).and_then(|r| r.as_object()) {
                for (rate_type, quotes) in rate_types {
                    if let Some(quotes_arr) = quotes.as_array() {
                        for quote in quotes_arr {
                            let quote_index = quote.get("index").and_then(|i| i.as_str());
                            if quote_index.is_none() || quote_index == Some(index) {
                                let tenor = quote
                                    .get("tenor")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let rate =
                                    quote.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                instruments.push(CurveInstrument {
                                    instrument_type: rate_type.clone(),
                                    tenor,
                                    rate,
                                    enabled: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(CurveInstrumentsResponse { instruments })
    }
}

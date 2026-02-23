//! Market data loading and convention endpoints.
//!
//! All file paths, file lists, and display-name mappings are read from
//! `demo/data/input/market_data_config.json` so that this module holds no
//! hard-coded data knowledge.

use std::{collections::HashMap, path::Path, sync::Arc};

use serde::Deserialize;

use super::DemoService;
use crate::{
    error::ServerError,
    rest::dto::{
        demo::{
            BondQuote, BondQuotesResponse, Convention, ConventionField, ConventionsResponse,
            CreditQuote, CreditQuotesResponse, EventType, EventsResponse, ExportFormat, Holiday,
            HolidaysResponse, Importance, MarketEvent, MarketRate, MarketRatesResponse,
        },
        jy_inflation::{CurveRatePoint, InflationMarketDataResponse},
    },
    services::helpers,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Configuration types (deserialised from market_data_config.json)
// ---------------------------------------------------------------------------

const CONFIG_PATH: &str = "demo/data/input/market_data_config.json";

#[derive(Debug, Deserialize)]
struct MarketDataConfig {
    paths: ConfigPaths,
    defaults: ConfigDefaults,
    #[serde(default)]
    index_display_names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ConfigPaths {
    rates: String,
    curve_files: Vec<String>,
    fx_spots: String,
    fx_forwards: String,
    xccy_basis: String,
    conventions: String,
    bonds: BondPaths,
    credit: CreditPaths,
    events: EventPaths,
    holidays: String,
    #[serde(default)]
    inflation: Option<InflationPaths>,
}

#[derive(Debug, Deserialize)]
struct InflationPaths {
    nominal_rates: String,
    real_rates: String,
}

#[derive(Debug, Deserialize)]
struct BondPaths {
    government: Vec<String>,
    corporate: String,
}

#[derive(Debug, Deserialize)]
struct CreditPaths {
    indices: Vec<String>,
    single_name: String,
}

#[derive(Debug, Deserialize)]
struct EventPaths {
    central_bank_meetings: String,
    economic_releases: String,
    turns: String,
}

#[derive(Debug, Deserialize)]
struct ConfigDefaults {
    source: String,
    quote_type: String,
}

fn load_config() -> Result<MarketDataConfig, ServerError> {
    helpers::load_json_file(Path::new(CONFIG_PATH), "market_data_config.json")
}

/// Create a `MarketRate` using defaults from the configuration.
fn make_market_rate(
    id: String,
    currency: String,
    tenor: String,
    rate_type: String,
    value: f64,
    rate_index: Option<String>,
    timestamp: &str,
    defaults: &ConfigDefaults,
) -> MarketRate {
    MarketRate {
        id,
        currency,
        tenor,
        rate_type,
        value,
        rate_index,
        quote_type: Some(defaults.quote_type.clone()),
        source: defaults.source.clone(),
        timestamp: timestamp.to_string(),
        is_stale: false,
    }
}

/// Normalise raw instrument type to uppercase abbreviation.
fn normalise_rate_type(raw: &str) -> String {
    match raw {
        "deposit" => "DEPO".to_string(),
        "ois" => "OIS".to_string(),
        "swap" => "SWAP".to_string(),
        "fra" => "FRA".to_string(),
        "future" => "FUT".to_string(),
        "fxspot" => "FXSPOT".to_string(),
        "fxforward" => "FXFWD".to_string(),
        "xccybasis" => "XCCY".to_string(),
        _ => raw.to_uppercase(),
    }
}

/// Parse importance string to enum.
fn parse_importance(s: &str) -> Importance {
    match s.to_lowercase().as_str() {
        "critical" => Importance::Critical,
        "high" => Importance::High,
        "medium" => Importance::Medium,
        _ => Importance::Low,
    }
}

/// Load a JSON file, extract an array by key, and parse each element.
fn load_and_collect<T>(
    path: &Path,
    key: &str,
    parser: fn(&serde_json::Value) -> Option<T>,
) -> Vec<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|d| d.get(key).and_then(|a| a.as_array().cloned()))
        .map(|arr| arr.iter().filter_map(parser).collect())
        .unwrap_or_default()
}

impl DemoService {
    /// Get market rates.
    pub fn get_market_rates(_state: &Arc<AppState>) -> Result<MarketRatesResponse, ServerError> {
        let cfg = load_config()?;
        let data: serde_json::Value =
            helpers::load_json_value(Path::new(&cfg.paths.rates), "market_quotes.json")?;

        let mut rates = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();

        if let Some(rates_by_currency) = data.get("rates").and_then(|r| r.as_object()) {
            for (currency, rate_types) in rates_by_currency {
                if let Some(rate_types_obj) = rate_types.as_object() {
                    for (rate_type, quotes) in rate_types_obj {
                        if let Some(quotes_arr) = quotes.as_array() {
                            for quote in quotes_arr {
                                let tenor =
                                    quote.get("tenor").and_then(|t| t.as_str()).unwrap_or("");
                                let value =
                                    quote.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let index = quote.get("index").and_then(|i| i.as_str());

                                let norm_type = normalise_rate_type(rate_type);
                                rates.push(make_market_rate(
                                    format!("{currency}-{norm_type}-{tenor}"),
                                    currency.clone(),
                                    tenor.to_string(),
                                    norm_type,
                                    value,
                                    index.map(String::from),
                                    &timestamp,
                                    &cfg.defaults,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for curve_file in &cfg.paths.curve_files {
            let curve_path = Path::new(curve_file);
            if let Ok(curve_content) = std::fs::read_to_string(curve_path) {
                if let Ok(curve_data) = serde_json::from_str::<serde_json::Value>(&curve_content) {
                    let currency = curve_data
                        .get("currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    let index_name = curve_data
                        .get("index")
                        .and_then(|i| i.as_str())
                        .unwrap_or("");

                    if let Some(instruments) =
                        curve_data.get("instruments").and_then(|i| i.as_array())
                    {
                        for instr in instruments {
                            let raw_type = instr
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            let tenor = instr.get("tenor").and_then(|t| t.as_str()).unwrap_or("");
                            let rate = instr.get("rate").and_then(|r| r.as_f64()).unwrap_or(0.0);
                            let norm_type = normalise_rate_type(raw_type);

                            let id = format!("{}-{}-{}", currency, norm_type, tenor);

                            if rates.iter().any(|r| {
                                r.currency == currency
                                    && r.tenor == tenor
                                    && r.rate_type == norm_type
                            }) {
                                continue;
                            }

                            rates.push(make_market_rate(
                                id,
                                currency.to_string(),
                                tenor.to_string(),
                                norm_type,
                                rate,
                                Some(index_name.to_uppercase()),
                                &timestamp,
                                &cfg.defaults,
                            ));
                        }
                    }
                }
            }
        }

        if let Ok(fx_content) = std::fs::read_to_string(Path::new(&cfg.paths.fx_spots)) {
            if let Ok(fx_data) = serde_json::from_str::<serde_json::Value>(&fx_content) {
                if let Some(spots) = fx_data.get("spots").and_then(|s| s.as_array()) {
                    for spot in spots {
                        let pair = spot.get("pair").and_then(|p| p.as_str()).unwrap_or("");
                        let value = spot.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let base_ccy = if pair.len() >= 3 { &pair[..3] } else { pair };

                        rates.push(make_market_rate(
                            pair.to_string(),
                            base_ccy.to_string(),
                            "SPOT".to_string(),
                            normalise_rate_type("fxspot"),
                            value,
                            Some(pair.to_string()),
                            &timestamp,
                            &cfg.defaults,
                        ));
                    }
                }
            }
        }

        if let Ok(fx_fwd_content) = std::fs::read_to_string(Path::new(&cfg.paths.fx_forwards)) {
            if let Ok(fx_fwd_data) = serde_json::from_str::<serde_json::Value>(&fx_fwd_content) {
                if let Some(forwards) = fx_fwd_data.get("forwards").and_then(|f| f.as_object()) {
                    for (pair, tenors) in forwards {
                        let base_ccy = if pair.len() >= 3 { &pair[..3] } else { pair };
                        if let Some(tenors_arr) = tenors.as_array() {
                            for fwd in tenors_arr {
                                let tenor = fwd.get("tenor").and_then(|t| t.as_str()).unwrap_or("");
                                let points =
                                    fwd.get("points").and_then(|p| p.as_f64()).unwrap_or(0.0);

                                rates.push(make_market_rate(
                                    format!("{pair}-{tenor}"),
                                    base_ccy.to_string(),
                                    tenor.to_string(),
                                    normalise_rate_type("fxforward"),
                                    points,
                                    Some(pair.clone()),
                                    &timestamp,
                                    &cfg.defaults,
                                ));
                            }
                        }
                    }
                }
            }
        }

        if let Ok(xccy_content) = std::fs::read_to_string(Path::new(&cfg.paths.xccy_basis)) {
            if let Ok(xccy_data) = serde_json::from_str::<serde_json::Value>(&xccy_content) {
                if let Some(basis) = xccy_data.get("basis").and_then(|b| b.as_object()) {
                    for (pair, tenors) in basis {
                        let base_ccy = if pair.len() >= 3 { &pair[..3] } else { pair };
                        if let Some(tenors_arr) = tenors.as_array() {
                            for spread in tenors_arr {
                                let tenor =
                                    spread.get("tenor").and_then(|t| t.as_str()).unwrap_or("");
                                let value =
                                    spread.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let index = spread.get("index").and_then(|i| i.as_str());

                                rates.push(make_market_rate(
                                    format!("XCCY-{pair}-{tenor}"),
                                    base_ccy.to_string(),
                                    tenor.to_string(),
                                    normalise_rate_type("xccybasis"),
                                    value,
                                    index.map(String::from),
                                    &timestamp,
                                    &cfg.defaults,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(MarketRatesResponse {
            rates,
            last_updated: timestamp,
        })
    }

    /// Refresh market rates (mock - just returns success).
    pub fn refresh_market_rates(_state: &Arc<AppState>) -> Result<(), ServerError> { Ok(()) }

    /// Get conventions.
    pub fn get_conventions(_state: &Arc<AppState>) -> Result<ConventionsResponse, ServerError> {
        let cfg = load_config()?;
        let data: serde_json::Value =
            helpers::load_json_value(Path::new(&cfg.paths.conventions), "conventions.json")?;

        let mut conventions = Vec::new();

        if let Some(conv_obj) = data.get("conventions").and_then(|c| c.as_object()) {
            for (id, conv) in conv_obj {
                let convention_type = conv
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let currency = conv
                    .get("currency")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_default = conv.get("is_default").and_then(|d| d.as_bool());

                let fields = conv.get("fields").and_then(|f| f.as_object()).map(|obj| {
                    obj.iter()
                        .map(|(key, value)| ConventionField {
                            label: key
                                .replace('_', " ")
                                .split_whitespace()
                                .map(|w| {
                                    let mut c = w.chars();
                                    match c.next() {
                                        None => String::new(),
                                        Some(f) => {
                                            f.to_uppercase().collect::<String>() + c.as_str()
                                        }
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                            value: value.as_str().map(String::from).unwrap_or_else(|| {
                                if let Some(n) = value.as_i64() {
                                    n.to_string()
                                } else if let Some(n) = value.as_f64() {
                                    n.to_string()
                                } else {
                                    value.to_string()
                                }
                            }),
                        })
                        .collect()
                });

                conventions.push(Convention {
                    id: id.clone(),
                    convention_type,
                    currency,
                    is_default,
                    fields,
                });
            }
        }

        conventions.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(ConventionsResponse { conventions })
    }

    /// Get convention detail.
    pub fn get_convention_detail(
        id: &str,
        state: &Arc<AppState>,
    ) -> Result<Convention, ServerError> {
        let conventions_response = Self::get_conventions(state)?;
        conventions_response
            .conventions
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| ServerError::NotFound(format!("Convention {} not found", id)))
    }

    /// Get market events.
    pub fn get_events(_state: &Arc<AppState>) -> Result<EventsResponse, ServerError> {
        let cfg = load_config()?;
        let mut events = Vec::new();

        fn parse_event_type(s: &str) -> EventType {
            match s {
                "central_bank_meeting" => EventType::CentralBankMeeting,
                "economic_release" => EventType::EconomicRelease,
                "holiday" => EventType::Holiday,
                "news" => EventType::News,
                "expiry" => EventType::Expiry,
                "turn" => EventType::Turn,
                _ => EventType::Other,
            }
        }

        fn parse_event(event: &serde_json::Value) -> Option<MarketEvent> {
            let id = event.get("id")?.as_str()?.to_string();
            let date = event.get("date")?.as_str()?.to_string();
            let event_type_str = event.get("eventType")?.as_str()?;
            let title = event.get("title")?.as_str()?.to_string();

            let central_bank = event.get("centralBank").and_then(|cb| {
                Some(crate::rest::dto::demo::CentralBank {
                    name: cb.get("name")?.as_str()?.to_string(),
                    code: cb.get("code")?.as_str()?.to_string(),
                    currency: cb.get("currency")?.as_str()?.to_string(),
                })
            });

            let tags = event.get("tags").and_then(|t| t.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(MarketEvent {
                id,
                date,
                event_type: parse_event_type(event_type_str),
                title,
                description: event
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from),
                currency: event
                    .get("currency")
                    .and_then(|c| c.as_str())
                    .map(String::from),
                region: event
                    .get("region")
                    .and_then(|r| r.as_str())
                    .map(String::from),
                importance: event
                    .get("importance")
                    .and_then(|i| i.as_str())
                    .map(parse_importance)
                    .unwrap_or(Importance::Medium),
                time: event.get("time").and_then(|t| t.as_str()).map(String::from),
                timezone: event
                    .get("timezone")
                    .and_then(|t| t.as_str())
                    .map(String::from),
                source: event
                    .get("source")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Internal")
                    .to_string(),
                tags,
                central_bank,
                previous: event
                    .get("previous")
                    .and_then(|p| p.as_str())
                    .map(String::from),
                forecast: event
                    .get("forecast")
                    .and_then(|f| f.as_str())
                    .map(String::from),
                actual: event
                    .get("actual")
                    .and_then(|a| a.as_str())
                    .map(String::from),
                expected_spike_bp: event.get("expectedRateBp").and_then(|v| v.as_f64()),
            })
        }

        events.extend(load_and_collect(
            Path::new(&cfg.paths.events.central_bank_meetings),
            "events",
            parse_event,
        ));
        events.extend(load_and_collect(
            Path::new(&cfg.paths.events.economic_releases),
            "events",
            parse_event,
        ));
        events.extend(load_and_collect(
            Path::new(&cfg.paths.events.turns),
            "turnEvents",
            parse_turn_event,
        ));

        fn parse_turn_event(turn: &serde_json::Value) -> Option<MarketEvent> {
            let id = turn.get("id")?.as_str()?.to_string();
            let date = turn.get("date")?.as_str()?.to_string();
            let currency = turn.get("currency")?.as_str()?.to_string();
            let turn_type = turn.get("eventType")?.as_str()?;
            let expected_spike = turn.get("expectedSpikeBp")?.as_f64()?;
            let notes = turn
                .get("notes")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();

            let title = match turn_type {
                "turn_of_year" => format!("{} Year-End Turn", currency),
                "turn_of_quarter" => format!("{} Quarter-End Turn", currency),
                "turn_of_month" => format!("{} Month-End Turn", currency),
                _ => format!("{} Turn", currency),
            };

            let event_type = match turn_type {
                "turn_of_year" => EventType::TurnOfYear,
                "turn_of_quarter" => EventType::TurnOfQuarter,
                "turn_of_month" => EventType::TurnOfMonth,
                _ => EventType::Turn,
            };

            let importance = match turn_type {
                "turn_of_year" => Importance::Critical,
                "turn_of_quarter" => Importance::High,
                _ => Importance::Medium,
            };

            let bid_spike = turn.get("bidSpikeBp").and_then(|b| b.as_f64());
            let ask_spike = turn.get("askSpikeBp").and_then(|a| a.as_f64());
            let historical_avg = turn.get("historicalAvgBp").and_then(|h| h.as_f64());

            let mut description = format!("Expected spike: {:.1}bp", expected_spike);
            if let (Some(bid), Some(ask)) = (bid_spike, ask_spike) {
                description.push_str(&format!(" (Bid: {:.1}bp, Ask: {:.1}bp)", bid, ask));
            }
            if let Some(hist) = historical_avg {
                description.push_str(&format!(". Historical avg: {:.1}bp", hist));
            }
            if !notes.is_empty() {
                description.push_str(&format!(". {}", notes));
            }

            Some(MarketEvent {
                id,
                date,
                event_type,
                title,
                description: Some(description),
                currency: Some(currency),
                region: None,
                importance,
                time: None,
                timezone: None,
                source: "Internal".to_string(),
                tags: Some(vec!["turn".to_string(), "curve".to_string()]),
                central_bank: None,
                previous: historical_avg.map(|h| format!("{:.1}bp", h)),
                forecast: Some(format!("{:.1}bp", expected_spike)),
                actual: None,
                expected_spike_bp: Some(expected_spike),
            })
        }

        events.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(EventsResponse { events })
    }

    /// Get market holidays.
    pub fn get_holidays(_state: &Arc<AppState>) -> Result<HolidaysResponse, ServerError> {
        let cfg = load_config()?;
        let mut holidays = Vec::new();

        fn parse_holiday(event: &serde_json::Value) -> Option<Holiday> {
            let id = event.get("id")?.as_str()?.to_string();
            let date = event.get("date")?.as_str()?.to_string();
            let title = event.get("title")?.as_str()?.to_string();
            let region = event
                .get("region")
                .and_then(|r| r.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let currency = event
                .get("currency")
                .and_then(|c| c.as_str())
                .map(String::from);
            let importance = event
                .get("importance")
                .and_then(|i| i.as_str())
                .map(parse_importance)
                .unwrap_or(Importance::Medium);
            let source = event
                .get("source")
                .and_then(|s| s.as_str())
                .map(String::from);

            let holiday_type = event
                .get("tags")
                .and_then(|t| t.as_array())
                .and_then(|arr| {
                    if arr.iter().any(|v| v.as_str() == Some("market-closed")) {
                        Some("market".to_string())
                    } else if arr.iter().any(|v| v.as_str() == Some("settlement-closed")) {
                        Some("settlement".to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "bank".to_string());

            Some(Holiday {
                id,
                date,
                name: title,
                country: region,
                currency,
                holiday_type,
                importance,
                source,
            })
        }

        holidays.extend(load_and_collect(
            Path::new(&cfg.paths.holidays),
            "events",
            parse_holiday,
        ));
        holidays.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(HolidaysResponse { holidays })
    }

    /// Export market data.
    pub fn export_market_data(
        format: ExportFormat,
        state: &Arc<AppState>,
    ) -> Result<Vec<u8>, ServerError> {
        let rates_response = Self::get_market_rates(state)?;

        match format {
            ExportFormat::Csv => {
                let mut csv = String::new();
                csv.push_str("id,currency,tenor,rate_type,value,rate_index,source\n");
                for rate in &rates_response.rates {
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        rate.id,
                        rate.currency,
                        rate.tenor,
                        rate.rate_type,
                        rate.value,
                        rate.rate_index.as_deref().unwrap_or(""),
                        rate.source
                    ));
                }
                Ok(csv.into_bytes())
            }
            ExportFormat::Json => {
                let json = serde_json::to_vec(&rates_response).map_err(|e| {
                    ServerError::Internal(format!("JSON serialization failed: {e}"))
                })?;
                Ok(json)
            }
        }
    }

    /// Get inflation market data (nominal + real rate curves) from input files.
    pub fn get_inflation_market_data(
        _state: &Arc<AppState>,
    ) -> Result<InflationMarketDataResponse, ServerError> {
        let cfg = load_config()?;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let inflation_paths = cfg.paths.inflation.ok_or_else(|| {
            ServerError::Internal("Inflation paths not configured in market_data_config.json".into())
        })?;

        // Load nominal rates
        let nominal_data: serde_json::Value = helpers::load_json_value(
            Path::new(&inflation_paths.nominal_rates),
            "inflation/nominal_rates.json",
        )?;

        let reference_date = nominal_data
            .get("reference_date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let currency = nominal_data
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("USD")
            .to_string();

        let nominal_rates = nominal_data
            .get("instruments")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|instr| {
                        let raw_type = instr.get("type")?.as_str()?;
                        let tenor = instr.get("tenor")?.as_str()?.to_string();
                        let rate = instr.get("rate")?.as_f64()?;
                        let instrument_type = match raw_type {
                            "deposit" => "Deposit",
                            "ois" => "OIS",
                            _ => raw_type,
                        };
                        Some(CurveRatePoint {
                            instrument_type: instrument_type.to_string(),
                            tenor,
                            rate,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Load real rates (TIPS)
        let real_data: serde_json::Value = helpers::load_json_value(
            Path::new(&inflation_paths.real_rates),
            "inflation/real_rates.json",
        )?;

        let inflation_index = real_data
            .get("inflation_index")
            .and_then(|v| v.as_str())
            .unwrap_or("CPI-U")
            .to_string();

        let real_rates = real_data
            .get("instruments")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|instr| {
                        let tenor = instr.get("tenor")?.as_str()?.to_string();
                        let rate = instr.get("rate")?.as_f64()?;
                        Some(CurveRatePoint {
                            instrument_type: "TIPS".to_string(),
                            tenor,
                            rate,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(InflationMarketDataResponse {
            nominal_rates,
            real_rates,
            reference_date,
            currency,
            inflation_index,
            last_updated: timestamp,
        })
    }

    /// Get bond market data quotes.
    pub fn get_bond_quotes(_state: &Arc<AppState>) -> Result<BondQuotesResponse, ServerError> {
        let cfg = load_config()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut quotes = Vec::new();

        // Load government bond files.
        for gov_file in &cfg.paths.bonds.government {
            let path = Path::new(gov_file);
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                    let issuer = data
                        .get("issuer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let currency = data
                        .get("currency")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let bond_type = data
                        .get("bond_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("government")
                        .to_string();
                    let rating = data
                        .get("rating")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if let Some(instruments) = data.get("instruments").and_then(|v| v.as_array()) {
                        for instr in instruments {
                            if let Some(q) = parse_bond_instrument(
                                instr, &issuer, &currency, &bond_type, &rating,
                            ) {
                                quotes.push(q);
                            }
                        }
                    }
                }
            }
        }

        // Load corporate bond file.
        if let Ok(content) = std::fs::read_to_string(Path::new(&cfg.paths.bonds.corporate)) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(issuers) = data.get("issuers").and_then(|v| v.as_array()) {
                    for issuer_obj in issuers {
                        let issuer = issuer_obj
                            .get("issuer")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let currency = issuer_obj
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let bond_type = issuer_obj
                            .get("bond_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("corporate")
                            .to_string();
                        let rating = issuer_obj
                            .get("rating")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if let Some(instruments) =
                            issuer_obj.get("instruments").and_then(|v| v.as_array())
                        {
                            for instr in instruments {
                                if let Some(q) = parse_bond_instrument(
                                    instr, &issuer, &currency, &bond_type, &rating,
                                ) {
                                    quotes.push(q);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(BondQuotesResponse {
            quotes,
            last_updated: timestamp,
        })
    }

    /// Get credit market data quotes (CDS spreads).
    pub fn get_credit_quotes(_state: &Arc<AppState>) -> Result<CreditQuotesResponse, ServerError> {
        let cfg = load_config()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut quotes = Vec::new();

        // Load index CDS files.
        for index_file in &cfg.paths.credit.indices {
            let path = Path::new(index_file);
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                    let index_name = data
                        .get("index")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let currency = data
                        .get("currency")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let recovery_rate = data
                        .get("recovery_rate")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.40);
                    let series = data
                        .get("series")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    let version = data
                        .get("version")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    let rating = data
                        .get("rating")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let is_hy = rating.as_deref() == Some("HY");

                    // Build display name with series.
                    let display_name = if let Some(s) = series {
                        let name = format_index_display_name(&index_name, &cfg.index_display_names);
                        format!("{} S{}", name, s)
                    } else {
                        format_index_display_name(&index_name, &cfg.index_display_names)
                    };

                    let index_type =
                        format_index_display_name(&index_name, &cfg.index_display_names);

                    if let Some(instruments) = data.get("instruments").and_then(|v| v.as_array()) {
                        for instr in instruments {
                            let tenor = instr
                                .get("tenor")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string();
                            let spread = instr.get("rate").and_then(|r| r.as_f64()).unwrap_or(0.0);

                            // HY indices: upfront = (spread - 5%) * 4 (approx 4Y risky annuity).
                            let upfront = if is_hy {
                                ((spread - 0.05) * 4.0 * 10000.0).round() / 10000.0
                            } else {
                                0.0
                            };

                            quotes.push(CreditQuote {
                                id: format!("{}-{}", index_name, tenor),
                                name: display_name.clone(),
                                currency: currency.clone(),
                                tenor,
                                spread,
                                upfront,
                                recovery_rate,
                                index_type: index_type.clone(),
                                series,
                                version,
                                rating: rating.clone(),
                                source: "Demo".to_string(),
                                is_stale: false,
                            });
                        }
                    }
                }
            }
        }

        // Load single-name CDS file.
        if let Ok(content) = std::fs::read_to_string(Path::new(&cfg.paths.credit.single_name)) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                let recovery_rate = data
                    .get("recovery_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.40);

                if let Some(entities) = data.get("entities").and_then(|v| v.as_array()) {
                    for entity in entities {
                        let name = entity
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let currency = entity
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let rating = entity
                            .get("rating")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let entity_id = name.replace(' ', "").replace('&', "");

                        if let Some(instruments) =
                            entity.get("instruments").and_then(|v| v.as_array())
                        {
                            for instr in instruments {
                                let tenor = instr
                                    .get("tenor")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let spread =
                                    instr.get("rate").and_then(|r| r.as_f64()).unwrap_or(0.0);

                                quotes.push(CreditQuote {
                                    id: format!("CDS-{}-{}", entity_id, tenor),
                                    name: name.clone(),
                                    currency: currency.clone(),
                                    tenor,
                                    spread,
                                    upfront: 0.0,
                                    recovery_rate,
                                    index_type: "Single Name".to_string(),
                                    series: None,
                                    version: None,
                                    rating: rating.clone(),
                                    source: "Demo".to_string(),
                                    is_stale: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(CreditQuotesResponse {
            quotes,
            last_updated: timestamp,
        })
    }
}

/// Parse a single bond instrument from JSON into a `BondQuote`.
fn parse_bond_instrument(
    instr: &serde_json::Value,
    issuer: &str,
    currency: &str,
    bond_type: &str,
    rating: &str,
) -> Option<BondQuote> {
    let maturity = instr.get("maturity")?.as_str()?.to_string();
    let coupon_rate = instr.get("coupon_rate")?.as_f64()?;
    let ytm = instr.get("ytm")?.as_f64()?;
    let price = instr.get("price")?.as_f64()?;
    let duration = instr.get("duration")?.as_f64()?;
    let convexity = instr.get("convexity")?.as_f64()?;
    let coupon_frequency = instr
        .get("coupon_frequency")
        .and_then(|v| v.as_str())
        .unwrap_or("semi_annual");

    let mat_year = &maturity[..4];
    let issuer_id = issuer.replace(' ', "");
    let id = format!("{}-{}-{}", currency, issuer_id, mat_year);

    let freq_display = match coupon_frequency {
        "annual" => "Annual",
        _ => "SemiAnnual",
    };

    Some(BondQuote {
        id,
        currency: currency.to_string(),
        issuer: issuer.to_string(),
        maturity,
        coupon_rate,
        ytm,
        price,
        duration,
        convexity,
        coupon_frequency: freq_display.to_string(),
        rating: rating.to_string(),
        bond_type: bond_type.to_string(),
        source: "Demo".to_string(),
        is_stale: false,
    })
}

/// Convert raw index name to display format using the configured mapping.
fn format_index_display_name(raw: &str, names: &HashMap<String, String>) -> String {
    names
        .get(raw)
        .cloned()
        .unwrap_or_else(|| raw.replace('-', "."))
}

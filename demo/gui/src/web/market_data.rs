//! Market data loading and caching.
//!
//! This module loads market data from multiple JSON files and provides
//! caching functionality for the Market Data Viewer webapp.
//!
//! # Data Sources
//!
//! Market data is loaded from:
//! - `demo/data/input/rates/market_quotes.json` - Interest rates (USD, EUR, JPY, GBP)
//! - `demo/data/input/fx/fx_spots.json` - FX spot rates
//! - `demo/data/input/fx/fx_forwards.json` - FX forward points
//! - `demo/data/input/fx/xccy_basis.json` - Cross currency basis swaps
//! - `demo/data/input/conventions/conventions.json` - Market conventions

use std::{
    collections::HashMap,
    sync::atomic::{AtomicI64, Ordering},
};

use serde::Deserialize;

use super::market_types::{
    ConventionField, ConventionResponse, ConventionSummary, ConventionsListResponse,
    InstrumentResponse, MarketRateDetailResponse, MarketRateQuery, MarketRateResponse,
    MarketRatesListResponse,
};

// =============================================================================
// JSON Data Structures (for loading from files)
// =============================================================================

/// Root structure of the rates market quotes JSON file.
#[derive(Debug, Deserialize, Default)]
struct RatesFile {
    rates: CurrencyRates,
}

/// Root structure of the FX spots JSON file.
#[derive(Debug, Deserialize)]
struct FxSpotsFile {
    spots: Vec<FxSpotData>,
}

/// Root structure of the FX forwards JSON file.
#[derive(Debug, Deserialize)]
struct FxForwardsFile {
    forwards: HashMap<String, Vec<FxForwardData>>,
}

/// Root structure of the XCCY basis JSON file.
#[derive(Debug, Deserialize)]
struct XccyBasisFile {
    basis: HashMap<String, Vec<XccyBasisData>>,
}

/// Root structure of the conventions JSON file.
#[derive(Debug, Deserialize)]
struct ConventionsFile {
    conventions: HashMap<String, ConventionData>,
}

/// Currency-grouped rates.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
struct CurrencyRates {
    usd: Option<RatesByType>,
    eur: Option<RatesByType>,
    jpy: Option<RatesByType>,
    gbp: Option<RatesByType>,
}

/// Rates grouped by type (deposit, ois, swap).
#[derive(Debug, Deserialize)]
struct RatesByType {
    deposit: Option<Vec<RateData>>,
    ois: Option<Vec<RateData>>,
    swap: Option<Vec<RateData>>,
}

/// Individual rate data from JSON.
#[derive(Debug, Deserialize)]
struct RateData {
    tenor: String,
    value: f64,
    index: Option<String>,
}

/// FX spot rate data from JSON.
#[derive(Debug, Deserialize)]
struct FxSpotData {
    pair: String,
    value: f64,
}

/// XCCY Basis Swap data from JSON.
#[derive(Debug, Deserialize)]
struct XccyBasisData {
    tenor: String,
    value: f64,
    index: Option<String>,
}

/// FX Forward data from JSON.
#[derive(Debug, Deserialize)]
struct FxForwardData {
    tenor: String,
    points: f64,
}

/// Convention data from JSON.
#[derive(Debug, Deserialize)]
struct ConventionData {
    #[serde(rename = "type")]
    convention_type: String,
    currency: String,
    is_default: bool,
    fields: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Data File Paths
// =============================================================================

/// Path to the rates market quotes JSON file.
const RATES_FILE: &str = "demo/data/input/rates/market_quotes.json";
/// Path to the FX spots JSON file.
const FX_SPOTS_FILE: &str = "demo/data/input/fx/fx_spots.json";
/// Path to the FX forwards JSON file.
const FX_FORWARDS_FILE: &str = "demo/data/input/fx/fx_forwards.json";
/// Path to the XCCY basis JSON file.
const XCCY_BASIS_FILE: &str = "demo/data/input/fx/xccy_basis.json";
/// Path to the conventions JSON file.
const CONVENTIONS_FILE: &str = "demo/data/input/conventions/conventions.json";

// =============================================================================
// Market Data Cache
// =============================================================================

/// Thread-safe market data cache.
#[derive(Debug)]
pub struct MarketDataCache {
    /// Cached rates (thread-safe).
    rates: tokio::sync::RwLock<Vec<MarketRateResponse>>,
    /// Cached conventions.
    conventions: tokio::sync::RwLock<HashMap<String, ConventionData>>,
    /// Last update timestamp.
    last_updated: AtomicI64,
    /// Staleness threshold in milliseconds (5 minutes).
    staleness_threshold_ms: i64,
}

impl Default for MarketDataCache {
    fn default() -> Self { Self::new() }
}

impl MarketDataCache {
    /// Creates a new cache, loading data from JSON file.
    pub fn new() -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let (rates, conventions) = load_market_data_from_file(now_ms);

        Self {
            rates: tokio::sync::RwLock::new(rates),
            conventions: tokio::sync::RwLock::new(conventions),
            last_updated: AtomicI64::new(now_ms),
            staleness_threshold_ms: 5 * 60 * 1000, // 5 minutes
        }
    }

    /// Gets the current market rates, optionally filtered.
    pub async fn get_rates(&self, query: &MarketRateQuery) -> MarketRatesListResponse {
        let rates = self.rates.read().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let threshold = now_ms - self.staleness_threshold_ms;

        let filtered: Vec<MarketRateResponse> = rates
            .iter()
            .filter(|r| {
                // Apply currency filter
                if let Some(ref currency) = query.currency {
                    if !r.currency.eq_ignore_ascii_case(currency) {
                        return false;
                    }
                }
                // Apply rate_type filter
                if let Some(ref rate_type) = query.rate_type {
                    if !r.rate_type.eq_ignore_ascii_case(rate_type) {
                        return false;
                    }
                }
                // Apply index filter
                if let Some(ref index) = query.index {
                    match &r.rate_index {
                        Some(ri) => {
                            if !ri.eq_ignore_ascii_case(index) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            })
            .map(|r| MarketRateResponse {
                is_stale: r.timestamp < threshold,
                ..r.clone()
            })
            .collect();

        let total_count = filtered.len();

        MarketRatesListResponse {
            rates: filtered,
            last_updated: self.last_updated.load(Ordering::Relaxed),
            total_count,
        }
    }

    /// Gets a single rate by ID.
    pub async fn get_rate(&self, rate_id: &str) -> Option<MarketRateResponse> {
        let rates = self.rates.read().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let threshold = now_ms - self.staleness_threshold_ms;

        rates
            .iter()
            .find(|r| r.id == rate_id)
            .map(|r| MarketRateResponse {
                is_stale: r.timestamp < threshold,
                ..r.clone()
            })
    }

    /// Gets detailed information for a rate including instrument and
    /// convention.
    pub async fn get_rate_detail(&self, rate_id: &str) -> Option<MarketRateDetailResponse> {
        let rate = self.get_rate(rate_id).await?;

        // Generate instrument information based on rate type
        let instrument = generate_instrument_for_rate(&rate);

        // Get convention information based on currency and rate type
        let conventions = self.conventions.read().await;
        let convention = get_convention_for_rate(&rate, &conventions);

        Some(MarketRateDetailResponse {
            rate,
            instrument,
            convention,
        })
    }

    /// Refreshes rates with small random perturbations.
    pub async fn refresh(&self) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut rates = self.rates.write().await;

        // Apply small random perturbations to simulate market movement
        for rate in rates.iter_mut() {
            // Small perturbation: +/- 0.5 bps
            let perturbation = (pseudo_random(rate.timestamp) % 10) as f64 / 10000.0 - 0.0005;
            rate.value += perturbation;
            rate.timestamp = now_ms;
        }

        self.last_updated.store(now_ms, Ordering::Relaxed);
    }

    /// Gets the last update timestamp.
    pub fn last_updated(&self) -> i64 { self.last_updated.load(Ordering::Relaxed) }
}

// =============================================================================
// Data Loading from JSON Files
// =============================================================================

/// Loads market data from multiple JSON files.
fn load_market_data_from_file(
    timestamp: i64,
) -> (Vec<MarketRateResponse>, HashMap<String, ConventionData>) {
    let mut rates = Vec::new();

    // Load rates from rates/market_quotes.json
    if let Ok(content) = std::fs::read_to_string(RATES_FILE) {
        if let Ok(data) = serde_json::from_str::<RatesFile>(&content) {
            rates.extend(convert_rates_to_responses(&data.rates, timestamp));
        }
    }

    // Load FX spots from fx/fx_spots.json
    if let Ok(content) = std::fs::read_to_string(FX_SPOTS_FILE) {
        if let Ok(data) = serde_json::from_str::<FxSpotsFile>(&content) {
            rates.extend(convert_fx_spots_to_responses(&data.spots, timestamp));
        }
    }

    // Load FX forwards from fx/fx_forwards.json
    if let Ok(content) = std::fs::read_to_string(FX_FORWARDS_FILE) {
        if let Ok(data) = serde_json::from_str::<FxForwardsFile>(&content) {
            rates.extend(convert_fx_forwards_to_responses(&data.forwards, timestamp));
        }
    }

    // Load XCCY basis from fx/xccy_basis.json
    if let Ok(content) = std::fs::read_to_string(XCCY_BASIS_FILE) {
        if let Ok(data) = serde_json::from_str::<XccyBasisFile>(&content) {
            rates.extend(convert_xccy_basis_to_responses(&data.basis, timestamp));
        }
    }

    // Load conventions from conventions/conventions.json
    let conventions = if let Ok(content) = std::fs::read_to_string(CONVENTIONS_FILE) {
        serde_json::from_str::<ConventionsFile>(&content)
            .map(|data| data.conventions)
            .unwrap_or_else(|_| generate_fallback_conventions())
    } else {
        generate_fallback_conventions()
    };

    // If no rates were loaded, use fallback
    if rates.is_empty() {
        rates = generate_fallback_rates(timestamp);
    }

    (rates, conventions)
}

/// Converts currency rates to MarketRateResponse vec.
fn convert_rates_to_responses(rates: &CurrencyRates, timestamp: i64) -> Vec<MarketRateResponse> {
    let mut result = Vec::new();

    if let Some(ref usd) = rates.usd {
        result.extend(convert_currency_rates("USD", usd, timestamp));
    }
    if let Some(ref eur) = rates.eur {
        result.extend(convert_currency_rates("EUR", eur, timestamp));
    }
    if let Some(ref jpy) = rates.jpy {
        result.extend(convert_currency_rates("JPY", jpy, timestamp));
    }
    if let Some(ref gbp) = rates.gbp {
        result.extend(convert_currency_rates("GBP", gbp, timestamp));
    }

    result
}

/// Converts FX spots to MarketRateResponse vec.
fn convert_fx_spots_to_responses(spots: &[FxSpotData], timestamp: i64) -> Vec<MarketRateResponse> {
    spots
        .iter()
        .map(|fx| MarketRateResponse {
            id: format!("{}-SPOT", fx.pair),
            currency: fx.pair[..3].to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: fx.value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        })
        .collect()
}

/// Converts FX forwards to MarketRateResponse vec.
fn convert_fx_forwards_to_responses(
    forwards: &HashMap<String, Vec<FxForwardData>>,
    timestamp: i64,
) -> Vec<MarketRateResponse> {
    let mut result = Vec::new();
    for (pair, fwd_data) in forwards {
        for r in fwd_data {
            result.push(MarketRateResponse {
                id: format!("{}-{}-FWD", pair, r.tenor),
                currency: pair[..3].to_string(),
                tenor: r.tenor.clone(),
                rate_type: "FxForward".to_string(),
                value: r.points,
                quote_type: "Points".to_string(),
                timestamp,
                source: "Reuters".to_string(),
                is_stale: false,
                rate_index: None,
            });
        }
    }
    result
}

/// Converts XCCY basis to MarketRateResponse vec.
fn convert_xccy_basis_to_responses(
    basis: &HashMap<String, Vec<XccyBasisData>>,
    timestamp: i64,
) -> Vec<MarketRateResponse> {
    let mut result = Vec::new();
    for (pair, basis_data) in basis {
        for r in basis_data {
            result.push(MarketRateResponse {
                id: format!("{}-{}-XCCY", pair, r.tenor),
                currency: pair.clone(),
                tenor: r.tenor.clone(),
                rate_type: "XccyBasis".to_string(),
                value: r.value,
                quote_type: "Mid".to_string(),
                timestamp,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }
    result
}

/// Converts rates for a single currency.
fn convert_currency_rates(
    currency: &str,
    rates_by_type: &RatesByType,
    timestamp: i64,
) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    // Deposit rates
    if let Some(ref deposits) = rates_by_type.deposit {
        for r in deposits {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-DEPO", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Deposit".to_string(),
                value: r.value,
                quote_type: "Mid".to_string(),
                timestamp,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    // OIS rates
    if let Some(ref ois) = rates_by_type.ois {
        for r in ois {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-OIS", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Ois".to_string(),
                value: r.value,
                quote_type: "Mid".to_string(),
                timestamp,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    // Swap rates
    if let Some(ref swaps) = rates_by_type.swap {
        for r in swaps {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-SWAP", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Swap".to_string(),
                value: r.value,
                quote_type: "Mid".to_string(),
                timestamp,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    rates
}

/// Generates fallback rates if JSON loading fails.
fn generate_fallback_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    vec![
        MarketRateResponse {
            id: "USD-3M-DEPO".to_string(),
            currency: "USD".to_string(),
            tenor: "3M".to_string(),
            rate_type: "Deposit".to_string(),
            value: 0.0525,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Fallback".to_string(),
            is_stale: false,
            rate_index: Some("SOFR".to_string()),
        },
        MarketRateResponse {
            id: "USD-5Y-SWAP".to_string(),
            currency: "USD".to_string(),
            tenor: "5Y".to_string(),
            rate_type: "Swap".to_string(),
            value: 0.0405,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Fallback".to_string(),
            is_stale: false,
            rate_index: Some("SOFR".to_string()),
        },
        MarketRateResponse {
            id: "USD-1Y-OIS".to_string(),
            currency: "USD".to_string(),
            tenor: "1Y".to_string(),
            rate_type: "Ois".to_string(),
            value: 0.0480,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Fallback".to_string(),
            is_stale: false,
            rate_index: Some("SOFR".to_string()),
        },
        MarketRateResponse {
            id: "EURUSD-SPOT".to_string(),
            currency: "EUR".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 1.0850,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Fallback".to_string(),
            is_stale: false,
            rate_index: None,
        },
    ]
}

/// Generates fallback conventions if JSON loading fails.
fn generate_fallback_conventions() -> HashMap<String, ConventionData> {
    let mut conventions = HashMap::new();

    // ===========================================
    // OIS Conventions
    // ===========================================
    conventions.insert(
        "USD-SOFR-OIS".to_string(),
        ConventionData {
            convention_type: "OisConvention".to_string(),
            currency: "USD".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("index".to_string(), serde_json::json!("SOFR"));
                fields.insert("day_count".to_string(), serde_json::json!("ACT/360"));
                fields
            },
        },
    );

    conventions.insert(
        "USD-SOFR-SWAP".to_string(),
        ConventionData {
            convention_type: "SwapConvention".to_string(),
            currency: "USD".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert(
                    "fixed_leg_day_count".to_string(),
                    serde_json::json!("ACT/360"),
                );
                fields.insert("float_leg_index".to_string(), serde_json::json!("SOFR"));
                fields
            },
        },
    );

    // ===========================================
    // Swaption (IRVol) Conventions
    // ===========================================
    conventions.insert(
        "USD-SWAPTION".to_string(),
        ConventionData {
            convention_type: "SwaptionConvention".to_string(),
            currency: "USD".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("vol_type".to_string(), serde_json::json!("Normal"));
                fields.insert("vol_unit".to_string(), serde_json::json!("bp"));
                fields.insert("settlement".to_string(), serde_json::json!("Cash"));
                fields.insert("exercise_style".to_string(), serde_json::json!("European"));
                fields.insert("underlying_tenor".to_string(), serde_json::json!("Swap"));
                fields
            },
        },
    );

    conventions.insert(
        "EUR-SWAPTION".to_string(),
        ConventionData {
            convention_type: "SwaptionConvention".to_string(),
            currency: "EUR".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("vol_type".to_string(), serde_json::json!("Normal"));
                fields.insert("vol_unit".to_string(), serde_json::json!("bp"));
                fields.insert("settlement".to_string(), serde_json::json!("Cash"));
                fields.insert("exercise_style".to_string(), serde_json::json!("European"));
                fields.insert("underlying_tenor".to_string(), serde_json::json!("Swap"));
                fields
            },
        },
    );

    conventions.insert(
        "JPY-SWAPTION".to_string(),
        ConventionData {
            convention_type: "SwaptionConvention".to_string(),
            currency: "JPY".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("vol_type".to_string(), serde_json::json!("Normal"));
                fields.insert("vol_unit".to_string(), serde_json::json!("bp"));
                fields.insert("settlement".to_string(), serde_json::json!("Cash"));
                fields.insert("exercise_style".to_string(), serde_json::json!("European"));
                fields.insert("underlying_tenor".to_string(), serde_json::json!("Swap"));
                fields
            },
        },
    );

    conventions.insert(
        "GBP-SWAPTION".to_string(),
        ConventionData {
            convention_type: "SwaptionConvention".to_string(),
            currency: "GBP".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("vol_type".to_string(), serde_json::json!("Normal"));
                fields.insert("vol_unit".to_string(), serde_json::json!("bp"));
                fields.insert("settlement".to_string(), serde_json::json!("Cash"));
                fields.insert("exercise_style".to_string(), serde_json::json!("European"));
                fields.insert("underlying_tenor".to_string(), serde_json::json!("Swap"));
                fields
            },
        },
    );

    // Cap/Floor Conventions
    conventions.insert(
        "USD-CAPFLOOR".to_string(),
        ConventionData {
            convention_type: "CapFloorConvention".to_string(),
            currency: "USD".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("vol_type".to_string(), serde_json::json!("Normal"));
                fields.insert("vol_unit".to_string(), serde_json::json!("bp"));
                fields.insert("underlying_index".to_string(), serde_json::json!("SOFR"));
                fields.insert("frequency".to_string(), serde_json::json!("Quarterly"));
                fields
            },
        },
    );

    // ===========================================
    // FX Option (FXVol) Conventions
    // ===========================================
    conventions.insert(
        "EURUSD-FXOPTION".to_string(),
        ConventionData {
            convention_type: "FxOptionConvention".to_string(),
            currency: "EUR".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("delta_type".to_string(), serde_json::json!("Spot Delta"));
                fields.insert(
                    "vol_quote_style".to_string(),
                    serde_json::json!("ATM + RR/BF"),
                );
                fields.insert("premium_currency".to_string(), serde_json::json!("USD"));
                fields.insert("settlement".to_string(), serde_json::json!("T+2"));
                fields.insert("cut_time".to_string(), serde_json::json!("NY 10:00"));
                fields
            },
        },
    );

    conventions.insert(
        "USDJPY-FXOPTION".to_string(),
        ConventionData {
            convention_type: "FxOptionConvention".to_string(),
            currency: "USD".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("delta_type".to_string(), serde_json::json!("Spot Delta"));
                fields.insert(
                    "vol_quote_style".to_string(),
                    serde_json::json!("ATM + RR/BF"),
                );
                fields.insert("premium_currency".to_string(), serde_json::json!("JPY"));
                fields.insert("settlement".to_string(), serde_json::json!("T+2"));
                fields.insert("cut_time".to_string(), serde_json::json!("Tokyo 15:00"));
                fields
            },
        },
    );

    conventions.insert(
        "GBPUSD-FXOPTION".to_string(),
        ConventionData {
            convention_type: "FxOptionConvention".to_string(),
            currency: "GBP".to_string(),
            is_default: true,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("delta_type".to_string(), serde_json::json!("Spot Delta"));
                fields.insert(
                    "vol_quote_style".to_string(),
                    serde_json::json!("ATM + RR/BF"),
                );
                fields.insert("premium_currency".to_string(), serde_json::json!("USD"));
                fields.insert("settlement".to_string(), serde_json::json!("T+2"));
                fields.insert("cut_time".to_string(), serde_json::json!("NY 10:00"));
                fields
            },
        },
    );

    conventions
}

// =============================================================================
// Convention Functions
// =============================================================================

/// Gets all available conventions.
pub fn get_conventions_list() -> ConventionsListResponse {
    let conventions_map = match std::fs::read_to_string(CONVENTIONS_FILE) {
        Ok(content) => match serde_json::from_str::<ConventionsFile>(&content) {
            Ok(data) => data.conventions,
            Err(_) => generate_fallback_conventions(),
        },
        Err(_) => generate_fallback_conventions(),
    };

    let conventions: Vec<ConventionSummary> = conventions_map
        .iter()
        .map(|(id, conv)| ConventionSummary {
            id: id.clone(),
            currency: conv.currency.clone(),
            convention_type: conv.convention_type.clone(),
            is_default: conv.is_default,
        })
        .collect();

    ConventionsListResponse { conventions }
}

/// Gets a convention by ID.
pub fn get_convention(convention_id: &str) -> Option<ConventionResponse> {
    let conventions_map = match std::fs::read_to_string(CONVENTIONS_FILE) {
        Ok(content) => match serde_json::from_str::<ConventionsFile>(&content) {
            Ok(data) => data.conventions,
            Err(_) => generate_fallback_conventions(),
        },
        Err(_) => generate_fallback_conventions(),
    };

    conventions_map.get(convention_id).map(|conv| {
        let fields: Vec<ConventionField> = conv
            .fields
            .iter()
            .map(|(key, value)| ConventionField {
                label: format_field_label(key),
                value: format_field_value(value),
            })
            .collect();

        ConventionResponse {
            convention_type: conv.convention_type.clone(),
            fields,
        }
    })
}

/// Formats a field key to a human-readable label.
fn format_field_label(key: &str) -> String {
    key.split('_')
        .map(|word| {
            let mut chars: Vec<char> = word.chars().collect();
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            }
            chars.into_iter().collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Formats a field value to string.
fn format_field_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

/// Gets convention for a rate based on currency and rate type.
fn get_convention_for_rate(
    rate: &MarketRateResponse,
    conventions: &HashMap<String, ConventionData>,
) -> Option<ConventionResponse> {
    let convention_id = match (rate.currency.as_str(), rate.rate_type.as_str()) {
        ("USD", "Ois") => "USD-SOFR-OIS",
        ("USD", "Swap") => "USD-SOFR-SWAP",
        ("USD", "Deposit") => "USD-DEPO",
        ("EUR", "Ois") => "EUR-ESTR-OIS",
        ("EUR", "Swap") => "EUR-EURIBOR-SWAP",
        ("EUR", "Deposit") => "EUR-DEPO",
        ("JPY", "Ois") => "JPY-TONA-OIS",
        ("JPY", "Swap") => "JPY-TIBOR-SWAP",
        ("JPY", "Deposit") => "JPY-DEPO",
        (_, "FxSpot") => "FX-SPOT",
        _ => return None,
    };

    conventions.get(convention_id).map(|conv| {
        let fields: Vec<ConventionField> = conv
            .fields
            .iter()
            .map(|(key, value)| ConventionField {
                label: format_field_label(key),
                value: format_field_value(value),
            })
            .collect();

        ConventionResponse {
            convention_type: conv.convention_type.clone(),
            fields,
        }
    })
}

// =============================================================================
// Instrument Generation
// =============================================================================

/// Generates instrument information for a given rate.
fn generate_instrument_for_rate(rate: &MarketRateResponse) -> Option<InstrumentResponse> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Parse tenor to get end date
    let end_date = calculate_end_date(&today, &rate.tenor)?;

    match rate.rate_type.as_str() {
        "Deposit" => Some(InstrumentResponse {
            instrument_type: "Deposit".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: HashMap::new(),
        }),
        "Swap" => Some(InstrumentResponse {
            instrument_type: "ParSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                params.insert("fixedFrequency".to_string(), serde_json::json!("Annual"));
                params.insert("floatFrequency".to_string(), serde_json::json!("Quarterly"));
                params
            },
        }),
        "Ois" => Some(InstrumentResponse {
            instrument_type: "OisSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                if let Some(ref index) = rate.rate_index {
                    params.insert("index".to_string(), serde_json::json!(index));
                }
                params
            },
        }),
        "FxSpot" | "Fra" | "Futures" => None,
        _ => None,
    }
}

/// Calculates end date from start date and tenor string.
fn calculate_end_date(start_date: &str, tenor: &str) -> Option<String> {
    let date = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d").ok()?;

    let months = match tenor.to_uppercase().as_str() {
        "ON" | "O/N" => return Some(date.succ_opt()?.format("%Y-%m-%d").to_string()),
        "1W" | "1WK" => {
            return Some(
                date.checked_add_days(chrono::Days::new(7))?
                    .format("%Y-%m-%d")
                    .to_string(),
            )
        }
        "2W" | "2WK" => {
            return Some(
                date.checked_add_days(chrono::Days::new(14))?
                    .format("%Y-%m-%d")
                    .to_string(),
            )
        }
        "1M" => 1,
        "2M" => 2,
        "3M" => 3,
        "6M" => 6,
        "9M" => 9,
        "1Y" => 12,
        "2Y" => 24,
        "3Y" => 36,
        "5Y" => 60,
        "7Y" => 84,
        "10Y" => 120,
        "15Y" => 180,
        "20Y" => 240,
        "30Y" => 360,
        "SPOT" => {
            return Some(
                date.checked_add_days(chrono::Days::new(2))?
                    .format("%Y-%m-%d")
                    .to_string(),
            )
        }
        _ => return None,
    };

    let end_date = date.checked_add_months(chrono::Months::new(months as u32))?;
    Some(end_date.format("%Y-%m-%d").to_string())
}

/// Simple pseudo-random number generator for perturbations.
fn pseudo_random(seed: i64) -> i64 {
    let a: i64 = 1103515245;
    let c: i64 = 12345;
    let m: i64 = 2147483648;
    ((seed.wrapping_mul(a).wrapping_add(c)) % m).abs()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod market_data_cache_tests {
        use super::*;

        #[tokio::test]
        async fn test_cache_creates_data() {
            let cache = MarketDataCache::new();
            let response = cache.get_rates(&MarketRateQuery::default()).await;

            // Should have some rates (from JSON or fallback)
            assert!(!response.rates.is_empty());
        }

        #[tokio::test]
        async fn test_cache_filter_by_currency() {
            let cache = MarketDataCache::new();
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: None,
                index: None,
            };

            let response = cache.get_rates(&query).await;

            for rate in &response.rates {
                assert_eq!(rate.currency, "USD");
            }
        }

        #[tokio::test]
        async fn test_cache_filter_by_rate_type() {
            let cache = MarketDataCache::new();
            let query = MarketRateQuery {
                currency: None,
                rate_type: Some("Ois".to_string()),
                index: None,
            };

            let response = cache.get_rates(&query).await;

            for rate in &response.rates {
                assert_eq!(rate.rate_type, "Ois");
            }
        }

        #[tokio::test]
        async fn test_cache_refresh() {
            let cache = MarketDataCache::new();
            let initial_ts = cache.last_updated();

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            cache.refresh().await;

            let new_ts = cache.last_updated();
            assert!(new_ts > initial_ts);
        }
    }

    mod convention_tests {
        use super::*;

        #[test]
        fn test_get_conventions_list() {
            // Should not panic - may return empty list if JSON file not found
            let _list = get_conventions_list();
        }

        #[test]
        fn test_format_field_label() {
            assert_eq!(format_field_label("day_count"), "Day Count");
            assert_eq!(format_field_label("settlement_days"), "Settlement Days");
            assert_eq!(
                format_field_label("business_day_convention"),
                "Business Day Convention"
            );
        }
    }

    mod date_calculation_tests {
        use super::*;

        #[test]
        fn test_calculate_end_date_months() {
            let end = calculate_end_date("2024-01-15", "3M");
            assert_eq!(end, Some("2024-04-15".to_string()));

            let end = calculate_end_date("2024-01-15", "1Y");
            assert_eq!(end, Some("2025-01-15".to_string()));

            let end = calculate_end_date("2024-01-15", "5Y");
            assert_eq!(end, Some("2029-01-15".to_string()));
        }

        #[test]
        fn test_calculate_end_date_weeks() {
            let end = calculate_end_date("2024-01-15", "1W");
            assert_eq!(end, Some("2024-01-22".to_string()));
        }

        #[test]
        fn test_calculate_end_date_overnight() {
            let end = calculate_end_date("2024-01-15", "ON");
            assert_eq!(end, Some("2024-01-16".to_string()));
        }

        #[test]
        fn test_calculate_end_date_spot() {
            let end = calculate_end_date("2024-01-15", "SPOT");
            assert_eq!(end, Some("2024-01-17".to_string()));
        }
    }
}

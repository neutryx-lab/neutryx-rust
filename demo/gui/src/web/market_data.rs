//! Market data loading and caching.
//!
//! This module loads market data from JSON files configured in
//! `demo/data/input/market_data_config.json` and provides caching
//! functionality for the Market Data Viewer webapp.

use std::{
    collections::HashMap,
    sync::atomic::{AtomicI64, Ordering},
};

use infra_master::time::{Date, EndOfMonthRule, Frequency, Tenor};
use pricer_core::math::rng::PricerRng;
use serde::Deserialize;

use super::handlers::market::{
    ConventionField, ConventionResponse, ConventionSummary, ConventionsListResponse,
    InstrumentResponse, MarketRateDetailResponse, MarketRateQuery, MarketRateResponse,
    MarketRatesListResponse,
};

// =============================================================================
// Configuration
// =============================================================================

/// Path to the market data configuration file.
const CONFIG_FILE: &str = "demo/data/input/market_data_config.json";

/// Market data configuration loaded from JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct MarketDataConfig {
    pub paths: DataPaths,
    pub defaults: ConfigDefaults,
    pub convention_mapping: ConventionMappingConfig,
}

/// File paths configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DataPaths {
    pub rates: String,
    pub fx_spots: String,
    pub fx_forwards: String,
    pub xccy_basis: String,
    pub conventions: String,
    #[serde(default)]
    pub events: Option<EventsPaths>,
}

/// Events file paths configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct EventsPaths {
    #[serde(default)]
    pub central_banks: Option<String>,
    #[serde(default)]
    pub central_bank_meetings: Option<String>,
    #[serde(default)]
    pub economic_releases: Option<String>,
    #[serde(default)]
    pub holidays: Option<String>,
}

/// Default values configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigDefaults {
    pub source: String,
    pub quote_type: String,
    pub staleness_threshold_ms: i64,
}

/// Convention mapping configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConventionMappingConfig {
    pub patterns: Vec<ConventionPattern>,
}

/// Pattern for convention lookup.
#[derive(Debug, Clone, Deserialize)]
pub struct ConventionPattern {
    pub currency: String,
    pub rate_type: String,
    #[serde(default)]
    pub convention_suffix: Option<String>,
    #[serde(default)]
    pub convention_id: Option<String>,
}

// =============================================================================
// JSON Data Structures (for loading from files)
// =============================================================================

/// Root structure of the rates market quotes JSON file.
#[derive(Debug, Deserialize, Default)]
struct RatesFile {
    rates: HashMap<String, RatesByType>,
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
#[derive(Debug, Deserialize, Clone)]
struct ConventionData {
    #[serde(rename = "type")]
    convention_type: String,
    currency: String,
    is_default: bool,
    fields: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Configuration Loading
// =============================================================================

/// Loads configuration from the config file.
fn load_config() -> Option<MarketDataConfig> {
    let content = std::fs::read_to_string(CONFIG_FILE).ok()?;
    serde_json::from_str(&content).ok()
}

/// Gets the market data config, falling back to defaults if file not found.
///
/// This function is public so that other modules (e.g., market.rs) can
/// share the same configuration for data file paths.
pub fn get_config() -> MarketDataConfig {
    load_config().unwrap_or_else(|| MarketDataConfig {
        paths: DataPaths {
            rates: "demo/data/input/rates/market_quotes.json".to_string(),
            fx_spots: "demo/data/input/fx/fx_spots.json".to_string(),
            fx_forwards: "demo/data/input/fx/fx_forwards.json".to_string(),
            xccy_basis: "demo/data/input/fx/xccy_basis.json".to_string(),
            conventions: "demo/data/input/conventions/conventions.json".to_string(),
            events: Some(EventsPaths {
                central_banks: Some("demo/data/input/events/central_banks.json".to_string()),
                central_bank_meetings: Some(
                    "demo/data/input/events/central_bank_meetings.json".to_string(),
                ),
                economic_releases: Some(
                    "demo/data/input/events/economic_releases.json".to_string(),
                ),
                holidays: Some("demo/data/input/events/holidays.json".to_string()),
            }),
        },
        defaults: ConfigDefaults {
            source: "Internal".to_string(),
            quote_type: "Mid".to_string(),
            staleness_threshold_ms: 300_000,
        },
        convention_mapping: ConventionMappingConfig { patterns: vec![] },
    })
}

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
    /// Configuration.
    config: MarketDataConfig,
    /// Last update timestamp.
    last_updated: AtomicI64,
}

impl Default for MarketDataCache {
    fn default() -> Self { Self::new() }
}

impl MarketDataCache {
    /// Creates a new cache, loading data from JSON files.
    pub fn new() -> Self {
        let config = get_config();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let (rates, conventions) = load_market_data(&config, now_ms);

        Self {
            rates: tokio::sync::RwLock::new(rates),
            conventions: tokio::sync::RwLock::new(conventions),
            last_updated: AtomicI64::new(now_ms),
            config,
        }
    }

    /// Gets the current market rates, optionally filtered.
    pub async fn get_rates(&self, query: &MarketRateQuery) -> MarketRatesListResponse {
        let rates = self.rates.read().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let threshold = now_ms - self.config.defaults.staleness_threshold_ms;

        let filtered: Vec<MarketRateResponse> = rates
            .iter()
            .filter(|r| {
                if let Some(ref currency) = query.currency {
                    if !r.currency.eq_ignore_ascii_case(currency) {
                        return false;
                    }
                }
                if let Some(ref rate_type) = query.rate_type {
                    if !r.rate_type.eq_ignore_ascii_case(rate_type) {
                        return false;
                    }
                }
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
        let threshold = now_ms - self.config.defaults.staleness_threshold_ms;

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
        let instrument = generate_instrument_for_rate(&rate);
        let conventions = self.conventions.read().await;
        let convention =
            get_convention_for_rate(&rate, &conventions, &self.config.convention_mapping);

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
        let mut rng = PricerRng::from_seed(now_ms as u64);

        for rate in rates.iter_mut() {
            // Small perturbation: +/- 0.5 bps using pricer_core RNG
            let perturbation = rng.gen_uniform() * 0.001 - 0.0005;
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

/// Loads market data from JSON files specified in config.
fn load_market_data(
    config: &MarketDataConfig,
    timestamp: i64,
) -> (Vec<MarketRateResponse>, HashMap<String, ConventionData>) {
    let mut rates = Vec::new();
    let default_source = &config.defaults.source;
    let default_quote_type = &config.defaults.quote_type;

    // Load rates
    if let Ok(content) = std::fs::read_to_string(&config.paths.rates) {
        if let Ok(data) = serde_json::from_str::<RatesFile>(&content) {
            rates.extend(convert_rates_to_responses(
                &data.rates,
                timestamp,
                default_source,
                default_quote_type,
            ));
        }
    }

    // Load FX spots
    if let Ok(content) = std::fs::read_to_string(&config.paths.fx_spots) {
        if let Ok(data) = serde_json::from_str::<FxSpotsFile>(&content) {
            rates.extend(convert_fx_spots_to_responses(
                &data.spots,
                timestamp,
                default_source,
                default_quote_type,
            ));
        }
    }

    // Load FX forwards
    if let Ok(content) = std::fs::read_to_string(&config.paths.fx_forwards) {
        if let Ok(data) = serde_json::from_str::<FxForwardsFile>(&content) {
            rates.extend(convert_fx_forwards_to_responses(
                &data.forwards,
                timestamp,
                default_source,
            ));
        }
    }

    // Load XCCY basis
    if let Ok(content) = std::fs::read_to_string(&config.paths.xccy_basis) {
        if let Ok(data) = serde_json::from_str::<XccyBasisFile>(&content) {
            rates.extend(convert_xccy_basis_to_responses(
                &data.basis,
                timestamp,
                default_source,
                default_quote_type,
            ));
        }
    }

    // Load conventions
    let conventions = std::fs::read_to_string(&config.paths.conventions)
        .ok()
        .and_then(|content| serde_json::from_str::<ConventionsFile>(&content).ok())
        .map(|data| data.conventions)
        .unwrap_or_default();

    (rates, conventions)
}

/// Converts currency rates to MarketRateResponse vec.
fn convert_rates_to_responses(
    rates: &HashMap<String, RatesByType>,
    timestamp: i64,
    source: &str,
    quote_type: &str,
) -> Vec<MarketRateResponse> {
    let mut result = Vec::new();
    for (currency, rates_by_type) in rates {
        result.extend(convert_currency_rates(
            currency,
            rates_by_type,
            timestamp,
            source,
            quote_type,
        ));
    }
    result
}

/// Converts FX spots to MarketRateResponse vec.
fn convert_fx_spots_to_responses(
    spots: &[FxSpotData],
    timestamp: i64,
    source: &str,
    quote_type: &str,
) -> Vec<MarketRateResponse> {
    spots
        .iter()
        .map(|fx| MarketRateResponse {
            id: format!("{}-SPOT", fx.pair),
            currency: fx.pair[..3].to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: fx.value,
            quote_type: quote_type.to_string(),
            timestamp,
            source: source.to_string(),
            is_stale: false,
            rate_index: None,
        })
        .collect()
}

/// Converts FX forwards to MarketRateResponse vec.
fn convert_fx_forwards_to_responses(
    forwards: &HashMap<String, Vec<FxForwardData>>,
    timestamp: i64,
    source: &str,
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
                source: source.to_string(),
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
    source: &str,
    quote_type: &str,
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
                quote_type: quote_type.to_string(),
                timestamp,
                source: source.to_string(),
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
    source: &str,
    quote_type: &str,
) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    if let Some(ref deposits) = rates_by_type.deposit {
        for r in deposits {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-DEPO", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Deposit".to_string(),
                value: r.value,
                quote_type: quote_type.to_string(),
                timestamp,
                source: source.to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    if let Some(ref ois) = rates_by_type.ois {
        for r in ois {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-OIS", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Ois".to_string(),
                value: r.value,
                quote_type: quote_type.to_string(),
                timestamp,
                source: source.to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    if let Some(ref swaps) = rates_by_type.swap {
        for r in swaps {
            rates.push(MarketRateResponse {
                id: format!("{}-{}-SWAP", currency, r.tenor),
                currency: currency.to_string(),
                tenor: r.tenor.clone(),
                rate_type: "Swap".to_string(),
                value: r.value,
                quote_type: quote_type.to_string(),
                timestamp,
                source: source.to_string(),
                is_stale: false,
                rate_index: r.index.clone(),
            });
        }
    }

    rates
}

// =============================================================================
// Convention Functions
// =============================================================================

/// Gets all available conventions.
pub fn get_conventions_list() -> ConventionsListResponse {
    let config = get_config();
    let conventions_map = std::fs::read_to_string(&config.paths.conventions)
        .ok()
        .and_then(|content| serde_json::from_str::<ConventionsFile>(&content).ok())
        .map(|data| data.conventions)
        .unwrap_or_default();

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
    let config = get_config();
    let conventions_map = std::fs::read_to_string(&config.paths.conventions)
        .ok()
        .and_then(|content| serde_json::from_str::<ConventionsFile>(&content).ok())
        .map(|data| data.conventions)
        .unwrap_or_default();

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

/// Gets convention for a rate based on currency, rate type, and config
/// patterns.
fn get_convention_for_rate(
    rate: &MarketRateResponse,
    conventions: &HashMap<String, ConventionData>,
    mapping: &ConventionMappingConfig,
) -> Option<ConventionResponse> {
    // Find matching pattern in config
    let convention_id = mapping
        .patterns
        .iter()
        .find(|p| {
            (p.currency == "*" || p.currency.eq_ignore_ascii_case(&rate.currency))
                && p.rate_type.eq_ignore_ascii_case(&rate.rate_type)
        })
        .and_then(|p| {
            if let Some(ref id) = p.convention_id {
                Some(id.clone())
            } else if let Some(ref suffix) = p.convention_suffix {
                // Build convention ID from currency and suffix
                // Try common patterns: {CCY}-{INDEX}-{SUFFIX} or {CCY}-{SUFFIX}
                let candidates = vec![
                    rate.rate_index
                        .as_ref()
                        .map(|idx| format!("{}-{}-{}", rate.currency, idx, suffix)),
                    Some(format!("{}-{}", rate.currency, suffix)),
                ];
                candidates
                    .into_iter()
                    .flatten()
                    .find(|id| conventions.contains_key(id))
            } else {
                None
            }
        });

    convention_id.and_then(|id| {
        conventions.get(&id).map(|conv| {
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
    })
}

// =============================================================================
// Instrument Generation
// =============================================================================

/// Generates instrument information for a given rate.
///
/// Uses `infra_master::time::Tenor` for date calculations and
/// `infra_master::time::Frequency` for payment frequencies.
fn generate_instrument_for_rate(rate: &MarketRateResponse) -> Option<InstrumentResponse> {
    let today = Date::today();
    let end_date = calculate_end_date(today, &rate.tenor)?;

    match rate.rate_type.as_str() {
        "Deposit" => Some(InstrumentResponse {
            instrument_type: "Deposit".to_string(),
            currency: rate.currency.clone(),
            start_date: today.to_string(),
            end_date: end_date.to_string(),
            rate: rate.value,
            parameters: HashMap::new(),
        }),
        "Swap" => Some(InstrumentResponse {
            instrument_type: "ParSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.to_string(),
            end_date: end_date.to_string(),
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                params.insert(
                    "fixedFrequency".to_string(),
                    serde_json::json!(Frequency::Annual.to_string()),
                );
                params.insert(
                    "floatFrequency".to_string(),
                    serde_json::json!(Frequency::Quarterly.to_string()),
                );
                params
            },
        }),
        "Ois" => Some(InstrumentResponse {
            instrument_type: "OisSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.to_string(),
            end_date: end_date.to_string(),
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                if let Some(ref index) = rate.rate_index {
                    params.insert("index".to_string(), serde_json::json!(index));
                }
                params
            },
        }),
        _ => None,
    }
}

/// Calculates end date from start date and tenor string.
///
/// Uses `infra_master::time::Tenor` for tenor parsing and date arithmetic.
fn calculate_end_date(start_date: Date, tenor_str: &str) -> Option<Date> {
    // Handle SPOT separately (T+2)
    if tenor_str.eq_ignore_ascii_case("SPOT") {
        return Some(start_date + 2);
    }

    // Parse tenor using infra_master's Tenor type
    let tenor: Tenor = tenor_str.parse().ok()?;
    Some(tenor.add_to_date(start_date, EndOfMonthRule::Adjust))
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
        async fn test_cache_creates() {
            let cache = MarketDataCache::new();
            let response = cache.get_rates(&MarketRateQuery::default()).await;
            // May be empty if JSON files don't exist, but should not panic
            assert!(response.total_count >= 0);
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
        fn test_calculate_end_date_uses_tenor() {
            let today = Date::today();

            // Test various tenors return Some (valid result)
            for tenor in ["ON", "1W", "1M", "3M", "1Y", "5Y", "10Y", "SPOT"] {
                let result = calculate_end_date(today, tenor);
                assert!(result.is_some(), "Failed for tenor: {}", tenor);
            }
        }

        #[test]
        fn test_calculate_end_date_invalid_tenor() {
            let today = Date::today();
            let result = calculate_end_date(today, "INVALID");
            assert!(result.is_none());
        }

        #[test]
        fn test_calculate_end_date_relative_order() {
            let today = Date::today();

            let end_1m = calculate_end_date(today, "1M").unwrap();
            let end_1y = calculate_end_date(today, "1Y").unwrap();

            // 1Y should be after 1M
            assert!(end_1y > end_1m);
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_get_config_fallback() {
            // Should return config even if file doesn't exist
            let config = get_config();
            assert!(!config.paths.rates.is_empty());
        }
    }
}

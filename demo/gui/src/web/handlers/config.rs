//! Configuration API handlers and types.
//!
//! This module provides REST API handlers for the Configuration API
//! and all related types. Configuration values are loaded from JSON files
//! in `demo/data/config/`.
//!
//! # Endpoints
//!
//! | Method | Path               | Description                    |
//! |--------|-------------------|--------------------------------|
//! | GET    | /api/config       | Get all configuration          |
//! | GET    | /api/config/enums | Get Enum values only           |
//! | GET    | /api/config/defaults | Get default values only     |

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::Json;
use infra_config::GreekType;
use infra_master::{
    market::Currency,
    time::{DayCounter, Frequency, Tenor},
};
use serde::{Deserialize, Serialize};

/// Path to GUI defaults configuration file.
const GUI_DEFAULTS_PATH: &str = "demo/data/config/gui_defaults.json";
/// Path to rate index mapping configuration file.
const RATE_INDEX_MAPPING_PATH: &str = "demo/data/config/rate_index_mapping.json";

/// Cached configuration loaded from JSON files.
static LOADED_CONFIG: OnceLock<LoadedConfig> = OnceLock::new();

// =============================================================================
// JSON File Loading
// =============================================================================

/// Configuration loaded from JSON files.
#[derive(Debug, Clone)]
struct LoadedConfig {
    defaults: DefaultValues,
    rate_index_mapping: HashMap<String, String>,
}

/// Raw JSON structure for gui_defaults.json.
#[derive(Debug, Clone, Deserialize)]
struct RawGuiDefaults {
    pricing: RawPricingDefaults,
    monte_carlo: RawMonteCarloDefaults,
    bump_sizes: RawBumpSizeDefaults,
    pricer: RawPricerDefaults,
    curve: RawCurveDefaults,
    expansion: RawExpansionDefaults,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPricingDefaults {
    curve_rate: f64,
    volatility: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMonteCarloDefaults {
    num_paths: usize,
    num_steps: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct RawBumpSizeDefaults {
    rate: f64,
    spot: f64,
    vol: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPricerDefaults {
    equity: RawEquityDefaults,
    fx: RawFxDefaults,
    irs: RawIrsDefaults,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEquityDefaults {
    spot: f64,
    strike: f64,
    expiry_years: f64,
    volatility: f64,
    rate: f64,
    option_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFxDefaults {
    spot: f64,
    strike: f64,
    expiry_years: f64,
    volatility: f64,
    domestic_rate: f64,
    foreign_rate: f64,
    option_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIrsDefaults {
    notional: f64,
    fixed_rate: f64,
    tenor_years: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCurveDefaults {
    notional: f64,
    fixed_rate: f64,
    tenor_years: u32,
    interpolation: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawExpansionDefaults {
    rates: RawRatesExpansionDefaults,
    swap: RawSwapExpansionDefaults,
    fx: RawFxExpansionDefaults,
    equity: RawEquityExpansionDefaults,
}

#[derive(Debug, Clone, Deserialize)]
struct RawRatesExpansionDefaults {
    currency: String,
    tenor: String,
    rate: f64,
    notional: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSwapExpansionDefaults {
    currency: String,
    tenor: String,
    fixed_rate: f64,
    spread: f64,
    notional: f64,
    payment_frequency: String,
    day_count: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFxExpansionDefaults {
    base_currency: String,
    quote_currency: String,
    spot_rate: f64,
    forward_rate: f64,
    notional: f64,
    option_type: String,
    volatility: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEquityExpansionDefaults {
    underlying: String,
    spot_price: f64,
    strike: f64,
    volatility: f64,
    risk_free_rate: f64,
    option_type: String,
    direction: String,
}

/// Raw JSON structure for rate_index_mapping.json.
#[derive(Debug, Clone, Deserialize)]
struct RawRateIndexMapping {
    mapping: HashMap<String, String>,
}

/// Load configuration from JSON files, with fallback to embedded defaults.
fn load_config() -> LoadedConfig {
    let defaults = load_gui_defaults().unwrap_or_else(|e| {
        tracing::warn!("Failed to load {}: {}, using embedded defaults", GUI_DEFAULTS_PATH, e);
        DefaultValues::embedded_default()
    });

    let rate_index_mapping = load_rate_index_mapping().unwrap_or_else(|e| {
        tracing::warn!("Failed to load {}: {}, using embedded defaults", RATE_INDEX_MAPPING_PATH, e);
        embedded_rate_index_mapping()
    });

    LoadedConfig {
        defaults,
        rate_index_mapping,
    }
}

/// Load GUI defaults from JSON file.
fn load_gui_defaults() -> Result<DefaultValues, String> {
    let content = std::fs::read_to_string(GUI_DEFAULTS_PATH)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let raw: RawGuiDefaults = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(DefaultValues::from_raw(raw))
}

/// Load rate index mapping from JSON file.
fn load_rate_index_mapping() -> Result<HashMap<String, String>, String> {
    let content = std::fs::read_to_string(RATE_INDEX_MAPPING_PATH)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let raw: RawRateIndexMapping = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(raw.mapping)
}

/// Embedded fallback for rate index mapping.
fn embedded_rate_index_mapping() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("USD".to_string(), "SOFR".to_string());
    map.insert("EUR".to_string(), "EURIBOR3M".to_string());
    map.insert("GBP".to_string(), "SONIA".to_string());
    map.insert("JPY".to_string(), "TONAR".to_string());
    map.insert("CHF".to_string(), "SARON".to_string());
    map
}

/// Get or initialize the cached configuration.
fn get_config_cached() -> &'static LoadedConfig {
    LOADED_CONFIG.get_or_init(load_config)
}

// =============================================================================
// Enum Values Types
// =============================================================================

/// All Enum values exposed to the frontend.
///
/// These values are sourced from `infra_master` and `infra_config` crates,
/// providing a single source of truth for the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumValues {
    /// Currency codes (e.g., "USD", "EUR", "GBP").
    pub currency: Vec<&'static str>,
    /// Tenor codes (e.g., "ON", "1W", "1M", "1Y").
    pub tenor: Vec<&'static str>,
    /// Payment frequency names (e.g., "Daily", "Monthly", "Annual").
    pub frequency: Vec<FrequencyInfo>,
    /// Day count convention names (e.g., "ACT/365", "ACT/360").
    pub day_counter: Vec<DayCounterInfo>,
    /// Quote types (e.g., "Bid", "Ask", "Mid", "Last").
    pub quote_type: Vec<&'static str>,
    /// Greek types (e.g., "delta", "gamma", "vega").
    pub greek_type: Vec<GreekInfo>,
    /// Asset classes (e.g., "rates", "fx", "equity").
    pub asset_class: Vec<&'static str>,
    /// Instrument types for pricing.
    pub instrument_type: Vec<&'static str>,
    /// Option types.
    pub option_type: Vec<&'static str>,
}

/// Frequency information with code and display name.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyInfo {
    /// Internal code (e.g., "SemiAnnual").
    pub code: &'static str,
    /// Display name (e.g., "Semi-Annual").
    pub name: &'static str,
    /// Periods per year (e.g., 2 for semi-annual).
    pub periods_per_year: u32,
}

/// Day counter information with code and display name.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayCounterInfo {
    /// Internal code (e.g., "Actual365Fixed").
    pub code: &'static str,
    /// Display name (e.g., "ACT/365").
    pub name: &'static str,
}

/// Greek type information.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GreekInfo {
    /// Internal code (e.g., "delta").
    pub code: &'static str,
    /// Whether this is a second-order Greek.
    pub is_second_order: bool,
}

impl EnumValues {
    /// Build EnumValues from crates.
    pub fn build() -> Self {
        Self {
            currency: Currency::all_codes().to_vec(),
            tenor: Tenor::all_codes().to_vec(),
            frequency: vec![
                FrequencyInfo {
                    code: "Daily",
                    name: Frequency::Daily.name(),
                    periods_per_year: Frequency::Daily.periods_per_year(),
                },
                FrequencyInfo {
                    code: "Weekly",
                    name: Frequency::Weekly.name(),
                    periods_per_year: Frequency::Weekly.periods_per_year(),
                },
                FrequencyInfo {
                    code: "Monthly",
                    name: Frequency::Monthly.name(),
                    periods_per_year: Frequency::Monthly.periods_per_year(),
                },
                FrequencyInfo {
                    code: "Quarterly",
                    name: Frequency::Quarterly.name(),
                    periods_per_year: Frequency::Quarterly.periods_per_year(),
                },
                FrequencyInfo {
                    code: "SemiAnnual",
                    name: Frequency::SemiAnnual.name(),
                    periods_per_year: Frequency::SemiAnnual.periods_per_year(),
                },
                FrequencyInfo {
                    code: "Annual",
                    name: Frequency::Annual.name(),
                    periods_per_year: Frequency::Annual.periods_per_year(),
                },
            ],
            day_counter: vec![
                DayCounterInfo {
                    code: "Actual360",
                    name: DayCounter::Actual360.name(),
                },
                DayCounterInfo {
                    code: "Actual365Fixed",
                    name: DayCounter::Actual365Fixed.name(),
                },
                DayCounterInfo {
                    code: "Actual36525",
                    name: DayCounter::Actual36525.name(),
                },
                DayCounterInfo {
                    code: "ActualActualIsda",
                    name: DayCounter::ActualActualIsda.name(),
                },
                DayCounterInfo {
                    code: "Thirty360Bond",
                    name: DayCounter::Thirty360Bond.name(),
                },
                DayCounterInfo {
                    code: "Thirty360European",
                    name: DayCounter::Thirty360European.name(),
                },
                DayCounterInfo {
                    code: "ThirtyE360Isda",
                    name: DayCounter::ThirtyE360Isda.name(),
                },
            ],
            quote_type: vec!["Bid", "Ask", "Mid", "Last"],
            greek_type: GreekType::all()
                .into_iter()
                .map(|g| GreekInfo {
                    code: match g {
                        GreekType::Delta => "delta",
                        GreekType::Gamma => "gamma",
                        GreekType::Vega => "vega",
                        GreekType::Theta => "theta",
                        GreekType::Rho => "rho",
                        GreekType::Vanna => "vanna",
                        GreekType::Volga => "volga",
                    },
                    is_second_order: g.is_second_order(),
                })
                .collect(),
            asset_class: vec!["rates", "fx", "equity", "credit", "commodity"],
            instrument_type: vec![
                "equity_vanilla_option",
                "fx_option",
                "irs",
                "deposit",
                "fra",
                "futures",
                "ois",
                "basis_swap",
            ],
            option_type: vec!["call", "put"],
        }
    }
}

// =============================================================================
// Default Values Types
// =============================================================================

/// Default values for pricing and risk calculations.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultValues {
    /// Pricing defaults.
    pub pricing: PricingDefaults,
    /// Monte Carlo simulation defaults.
    pub monte_carlo: MonteCarloDefaults,
    /// Bump sizes for finite difference calculations.
    pub bump_sizes: BumpSizeDefaults,
    /// Pricer-specific defaults (equity, FX, IRS).
    pub pricer: PricerDefaults,
    /// Curve builder defaults.
    pub curve: CurveDefaults,
    /// Trade expansion defaults.
    pub expansion: ExpansionDefaults,
}

/// Pricing calculation defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingDefaults {
    /// Default curve rate (e.g., 0.05 = 5%).
    pub curve_rate: f64,
    /// Default volatility (e.g., 0.20 = 20%).
    pub volatility: f64,
}

/// Monte Carlo simulation defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloDefaults {
    /// Number of simulation paths.
    pub num_paths: usize,
    /// Number of time steps.
    pub num_steps: usize,
}

/// Bump size defaults for risk calculations.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpSizeDefaults {
    /// Rate bump in decimal (e.g., 0.0001 = 1bp).
    pub rate: f64,
    /// FX/spot bump in decimal (e.g., 0.01 = 1%).
    pub spot: f64,
    /// Volatility bump in decimal (e.g., 0.01 = 1%).
    pub vol: f64,
}

/// Pricer-specific defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricerDefaults {
    /// Equity option defaults.
    pub equity: EquityDefaults,
    /// FX option defaults.
    pub fx: FxDefaults,
    /// IRS defaults.
    pub irs: IrsDefaults,
}

/// Equity option defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EquityDefaults {
    pub spot: f64,
    pub strike: f64,
    pub expiry_years: f64,
    pub volatility: f64,
    pub rate: f64,
    pub option_type: String,
}

/// FX option defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct FxDefaults {
    pub spot: f64,
    pub strike: f64,
    pub expiry_years: f64,
    pub volatility: f64,
    pub domestic_rate: f64,
    pub foreign_rate: f64,
    pub option_type: String,
}

/// IRS defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct IrsDefaults {
    pub notional: f64,
    pub fixed_rate: f64,
    pub tenor_years: u32,
}

/// Curve builder defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct CurveDefaults {
    pub notional: f64,
    pub fixed_rate: f64,
    pub tenor_years: u32,
    pub interpolation: String,
}

/// Trade expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct ExpansionDefaults {
    pub rates: RatesExpansionDefaults,
    pub swap: SwapExpansionDefaults,
    pub fx: FxExpansionDefaults,
    pub equity: EquityExpansionDefaults,
}

/// Rates expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct RatesExpansionDefaults {
    pub currency: String,
    pub tenor: String,
    pub rate: f64,
    pub notional: f64,
}

/// Swap expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct SwapExpansionDefaults {
    pub currency: String,
    pub tenor: String,
    pub fixed_rate: f64,
    pub spread: f64,
    pub notional: f64,
    pub payment_frequency: String,
    pub day_count: String,
}

/// FX expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct FxExpansionDefaults {
    pub base_currency: String,
    pub quote_currency: String,
    pub spot_rate: f64,
    pub forward_rate: f64,
    pub notional: f64,
    pub option_type: String,
    pub volatility: f64,
}

/// Equity expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EquityExpansionDefaults {
    pub underlying: String,
    pub spot_price: f64,
    pub strike: f64,
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub option_type: String,
    pub direction: String,
}

impl DefaultValues {
    /// Convert from raw JSON structure.
    fn from_raw(raw: RawGuiDefaults) -> Self {
        Self {
            pricing: PricingDefaults {
                curve_rate: raw.pricing.curve_rate,
                volatility: raw.pricing.volatility,
            },
            monte_carlo: MonteCarloDefaults {
                num_paths: raw.monte_carlo.num_paths,
                num_steps: raw.monte_carlo.num_steps,
            },
            bump_sizes: BumpSizeDefaults {
                rate: raw.bump_sizes.rate,
                spot: raw.bump_sizes.spot,
                vol: raw.bump_sizes.vol,
            },
            pricer: PricerDefaults {
                equity: EquityDefaults {
                    spot: raw.pricer.equity.spot,
                    strike: raw.pricer.equity.strike,
                    expiry_years: raw.pricer.equity.expiry_years,
                    volatility: raw.pricer.equity.volatility,
                    rate: raw.pricer.equity.rate,
                    option_type: raw.pricer.equity.option_type,
                },
                fx: FxDefaults {
                    spot: raw.pricer.fx.spot,
                    strike: raw.pricer.fx.strike,
                    expiry_years: raw.pricer.fx.expiry_years,
                    volatility: raw.pricer.fx.volatility,
                    domestic_rate: raw.pricer.fx.domestic_rate,
                    foreign_rate: raw.pricer.fx.foreign_rate,
                    option_type: raw.pricer.fx.option_type,
                },
                irs: IrsDefaults {
                    notional: raw.pricer.irs.notional,
                    fixed_rate: raw.pricer.irs.fixed_rate,
                    tenor_years: raw.pricer.irs.tenor_years,
                },
            },
            curve: CurveDefaults {
                notional: raw.curve.notional,
                fixed_rate: raw.curve.fixed_rate,
                tenor_years: raw.curve.tenor_years,
                interpolation: raw.curve.interpolation,
            },
            expansion: ExpansionDefaults {
                rates: RatesExpansionDefaults {
                    currency: raw.expansion.rates.currency,
                    tenor: raw.expansion.rates.tenor,
                    rate: raw.expansion.rates.rate,
                    notional: raw.expansion.rates.notional,
                },
                swap: SwapExpansionDefaults {
                    currency: raw.expansion.swap.currency,
                    tenor: raw.expansion.swap.tenor,
                    fixed_rate: raw.expansion.swap.fixed_rate,
                    spread: raw.expansion.swap.spread,
                    notional: raw.expansion.swap.notional,
                    payment_frequency: raw.expansion.swap.payment_frequency,
                    day_count: raw.expansion.swap.day_count,
                },
                fx: FxExpansionDefaults {
                    base_currency: raw.expansion.fx.base_currency,
                    quote_currency: raw.expansion.fx.quote_currency,
                    spot_rate: raw.expansion.fx.spot_rate,
                    forward_rate: raw.expansion.fx.forward_rate,
                    notional: raw.expansion.fx.notional,
                    option_type: raw.expansion.fx.option_type,
                    volatility: raw.expansion.fx.volatility,
                },
                equity: EquityExpansionDefaults {
                    underlying: raw.expansion.equity.underlying,
                    spot_price: raw.expansion.equity.spot_price,
                    strike: raw.expansion.equity.strike,
                    volatility: raw.expansion.equity.volatility,
                    risk_free_rate: raw.expansion.equity.risk_free_rate,
                    option_type: raw.expansion.equity.option_type,
                    direction: raw.expansion.equity.direction,
                },
            },
        }
    }

    /// Embedded default values (fallback when JSON file is not available).
    fn embedded_default() -> Self {
        Self {
            pricing: PricingDefaults {
                curve_rate: 0.05,
                volatility: 0.20,
            },
            monte_carlo: MonteCarloDefaults {
                num_paths: 10_000,
                num_steps: 252,
            },
            bump_sizes: BumpSizeDefaults {
                rate: 0.0001,
                spot: 0.01,
                vol: 0.01,
            },
            pricer: PricerDefaults {
                equity: EquityDefaults {
                    spot: 100.0,
                    strike: 100.0,
                    expiry_years: 1.0,
                    volatility: 0.20,
                    rate: 0.05,
                    option_type: "call".to_string(),
                },
                fx: FxDefaults {
                    spot: 1.10,
                    strike: 1.10,
                    expiry_years: 1.0,
                    volatility: 0.10,
                    domestic_rate: 0.05,
                    foreign_rate: 0.02,
                    option_type: "call".to_string(),
                },
                irs: IrsDefaults {
                    notional: 1_000_000.0,
                    fixed_rate: 0.025,
                    tenor_years: 5,
                },
            },
            curve: CurveDefaults {
                notional: 10_000_000.0,
                fixed_rate: 0.03,
                tenor_years: 5,
                interpolation: "linear_on_log_df".to_string(),
            },
            expansion: ExpansionDefaults {
                rates: RatesExpansionDefaults {
                    currency: "USD".to_string(),
                    tenor: "1Y".to_string(),
                    rate: 0.035,
                    notional: 10_000_000.0,
                },
                swap: SwapExpansionDefaults {
                    currency: "USD".to_string(),
                    tenor: "5Y".to_string(),
                    fixed_rate: 0.03,
                    spread: 0.0,
                    notional: 10_000_000.0,
                    payment_frequency: "SemiAnnual".to_string(),
                    day_count: "Actual365Fixed".to_string(),
                },
                fx: FxExpansionDefaults {
                    base_currency: "EUR".to_string(),
                    quote_currency: "USD".to_string(),
                    spot_rate: 1.085,
                    forward_rate: 1.09,
                    notional: 1_000_000.0,
                    option_type: "call".to_string(),
                    volatility: 0.10,
                },
                equity: EquityExpansionDefaults {
                    underlying: "AAPL".to_string(),
                    spot_price: 180.0,
                    strike: 185.0,
                    volatility: 0.25,
                    risk_free_rate: 0.05,
                    option_type: "call".to_string(),
                    direction: "long".to_string(),
                },
            },
        }
    }
}

impl Default for DefaultValues {
    fn default() -> Self {
        get_config_cached().defaults.clone()
    }
}

// =============================================================================
// Rate Index Mapping
// =============================================================================

/// Build rate index by currency mapping from JSON file.
pub fn build_rate_index_by_currency() -> HashMap<String, String> {
    get_config_cached().rate_index_mapping.clone()
}

// =============================================================================
// Configuration Response
// =============================================================================

/// Complete configuration response for `/api/config`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    /// All Enum values.
    pub enums: EnumValues,
    /// Default values.
    pub defaults: DefaultValues,
    /// Rate index by currency mapping.
    pub rate_index_by_currency: HashMap<String, String>,
}

impl ConfigResponse {
    /// Build complete configuration response.
    pub fn build() -> Self {
        Self {
            enums: EnumValues::build(),
            defaults: DefaultValues::default(),
            rate_index_by_currency: build_rate_index_by_currency(),
        }
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// Get complete configuration including enums and defaults.
///
/// Returns all Enum values from crates and default values for the frontend.
/// This endpoint should be called once at application startup.
///
/// # Returns
///
/// JSON object with:
/// - `enums`: All Enum values (currency, tenor, frequency, etc.)
/// - `defaults`: Default values for pricing and risk calculations
/// - `rateIndexByCurrency`: Mapping of currencies to their default rate indices
///
/// # Example
///
/// ```text
/// GET /api/config
/// ```
pub async fn get_config() -> Json<ConfigResponse> { Json(ConfigResponse::build()) }

/// Get Enum values only.
///
/// Returns all Enum values without default values.
/// Useful for populating dropdowns and select inputs.
///
/// # Example
///
/// ```text
/// GET /api/config/enums
/// ```
pub async fn get_enums() -> Json<EnumValues> { Json(EnumValues::build()) }

/// Get default values only.
///
/// Returns default values for pricing and risk calculations.
/// Useful for initialising form fields.
///
/// # Example
///
/// ```text
/// GET /api/config/defaults
/// ```
pub async fn get_defaults() -> Json<DefaultValues> { Json(DefaultValues::default()) }

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_values_build() {
        let enums = EnumValues::build();
        assert_eq!(enums.currency.len(), 5);
        assert!(enums.currency.contains(&"USD"));
        assert!(enums.tenor.len() >= 17);
        assert!(enums.tenor.contains(&"1Y"));
    }

    #[test]
    fn test_embedded_default_values() {
        let defaults = DefaultValues::embedded_default();
        assert!((defaults.pricing.curve_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(defaults.monte_carlo.num_paths, 10_000);
    }

    #[test]
    fn test_config_response_build() {
        let config = ConfigResponse::build();
        assert!(!config.enums.currency.is_empty());
        assert!(!config.rate_index_by_currency.is_empty());
        assert_eq!(config.rate_index_by_currency.get("USD"), Some(&"SOFR".to_string()));
    }

    #[test]
    fn test_config_response_serializes() {
        let config = ConfigResponse::build();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"currency\""));
        assert!(json.contains("\"USD\""));
        assert!(json.contains("\"tenor\""));
        assert!(json.contains("\"defaults\""));
    }

    #[tokio::test]
    async fn test_get_config() {
        let Json(config) = get_config().await;
        assert!(!config.enums.currency.is_empty());
        assert!(!config.rate_index_by_currency.is_empty());
    }

    #[tokio::test]
    async fn test_get_enums() {
        let Json(enums) = get_enums().await;
        assert!(enums.currency.contains(&"USD"));
        assert!(enums.tenor.contains(&"1Y"));
    }

    #[tokio::test]
    async fn test_get_defaults() {
        let Json(defaults) = get_defaults().await;
        assert!(defaults.pricing.curve_rate > 0.0);
        assert!(defaults.monte_carlo.num_paths > 0);
    }
}

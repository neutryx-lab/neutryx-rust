//! Configuration API types.
//!
//! This module provides types for the `/api/config` endpoint,
//! exposing Enum values and default settings from crates to the frontend.

use std::collections::HashMap;

use serde::Serialize;

use infra_config::{BumpSizes, GreekType};
use infra_master::{
    market::Currency,
    time::{DayCounter, Frequency, Tenor},
};

// =============================================================================
// Enum Values Response
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
// Default Values
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
    pub option_type: &'static str,
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
    pub option_type: &'static str,
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
    pub interpolation: &'static str,
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
    pub currency: &'static str,
    pub tenor: &'static str,
    pub rate: f64,
    pub notional: f64,
}

/// Swap expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct SwapExpansionDefaults {
    pub currency: &'static str,
    pub tenor: &'static str,
    pub fixed_rate: f64,
    pub spread: f64,
    pub notional: f64,
    pub payment_frequency: &'static str,
    pub day_count: &'static str,
}

/// FX expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct FxExpansionDefaults {
    pub base_currency: &'static str,
    pub quote_currency: &'static str,
    pub spot_rate: f64,
    pub forward_rate: f64,
    pub notional: f64,
    pub option_type: &'static str,
    pub volatility: f64,
}

/// Equity expansion defaults.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EquityExpansionDefaults {
    pub underlying: &'static str,
    pub spot_price: f64,
    pub strike: f64,
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub option_type: &'static str,
    pub direction: &'static str,
}

impl Default for DefaultValues {
    fn default() -> Self {
        let bump_sizes = BumpSizes::default();

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
                rate: bump_sizes.rate,
                spot: bump_sizes.spot,
                vol: bump_sizes.vol,
            },
            pricer: PricerDefaults {
                equity: EquityDefaults {
                    spot: 100.0,
                    strike: 100.0,
                    expiry_years: 1.0,
                    volatility: 0.20,
                    rate: 0.05,
                    option_type: "call",
                },
                fx: FxDefaults {
                    spot: 1.10,
                    strike: 1.10,
                    expiry_years: 1.0,
                    volatility: 0.10,
                    domestic_rate: 0.05,
                    foreign_rate: 0.02,
                    option_type: "call",
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
                interpolation: "linear_on_log_df",
            },
            expansion: ExpansionDefaults {
                rates: RatesExpansionDefaults {
                    currency: "USD",
                    tenor: "1Y",
                    rate: 0.035,
                    notional: 10_000_000.0,
                },
                swap: SwapExpansionDefaults {
                    currency: "USD",
                    tenor: "5Y",
                    fixed_rate: 0.03,
                    spread: 0.0,
                    notional: 10_000_000.0,
                    payment_frequency: "SemiAnnual",
                    day_count: "Actual365Fixed",
                },
                fx: FxExpansionDefaults {
                    base_currency: "EUR",
                    quote_currency: "USD",
                    spot_rate: 1.085,
                    forward_rate: 1.09,
                    notional: 1_000_000.0,
                    option_type: "call",
                    volatility: 0.10,
                },
                equity: EquityExpansionDefaults {
                    underlying: "AAPL",
                    spot_price: 180.0,
                    strike: 185.0,
                    volatility: 0.25,
                    risk_free_rate: 0.05,
                    option_type: "call",
                    direction: "long",
                },
            },
        }
    }
}

// =============================================================================
// Rate Index Mapping
// =============================================================================

/// Build rate index by currency mapping.
pub fn build_rate_index_by_currency() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("USD", "SOFR");
    map.insert("EUR", "EURIBOR3M");
    map.insert("GBP", "SONIA");
    map.insert("JPY", "TONAR");
    map.insert("CHF", "SARON");
    map
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
    pub rate_index_by_currency: HashMap<&'static str, &'static str>,
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
    fn test_default_values() {
        let defaults = DefaultValues::default();
        assert!((defaults.pricing.curve_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(defaults.monte_carlo.num_paths, 10_000);
    }

    #[test]
    fn test_config_response_build() {
        let config = ConfigResponse::build();
        assert!(!config.enums.currency.is_empty());
        assert!(!config.rate_index_by_currency.is_empty());
        assert_eq!(config.rate_index_by_currency.get("USD"), Some(&"SOFR"));
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
}

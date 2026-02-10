//! Application configuration export for frontend integration.
//!
//! This module provides `AppConfig` which aggregates:
//! - `EnumRegistry`: All enum variant names for dropdown/form population
//! - `DefaultsRegistry`: Default values for configuration structures
//! - `CurrencyRateIndexMap`: Currency code to rate index mapping
//!
//! Together these form the Single Source of Truth for frontend configuration,
//! eliminating the need for duplicated JSON files.

use std::collections::HashMap;

use serde::Serialize;
use strum::VariantNames;

use crate::{
    BumpSizes, GreekType, GreeksMethod, MonteCarloParams, PricingMethod, SecondOrderMode,
    ShiftType, TreeParams, TreeType,
};

// =============================================================================
// EnumRegistry
// =============================================================================

/// Registry that exports all enum variant names as JSON.
pub struct EnumRegistry;

impl EnumRegistry {
    /// Returns all exportable enums as a JSON object.
    pub fn to_json() -> serde_json::Value {
        serde_json::json!({
            "pricing_method": PricingMethod::VARIANTS,
            "tree_type": TreeType::VARIANTS,
            "greeks_method": GreeksMethod::VARIANTS,
            "greek_type": GreekType::VARIANTS,
            "second_order_mode": SecondOrderMode::VARIANTS,
            "shift_type": ShiftType::VARIANTS,
        })
    }
}

// =============================================================================
// DefaultsRegistry
// =============================================================================

/// Registry that exports default values for configuration structures.
pub struct DefaultsRegistry;

impl DefaultsRegistry {
    /// Returns all default values as a hierarchical JSON object.
    pub fn to_json() -> serde_json::Value {
        serde_json::json!({
            "monte_carlo": MonteCarloParams::default(),
            "tree_params": TreeParams::default(),
            "bump_sizes": BumpSizes::default(),
        })
    }
}

// =============================================================================
// CurrencyRateIndexMap
// =============================================================================

/// Mapping from currency codes to their primary rate indices.
///
/// Provides default mappings for major currencies (USD→SOFR, EUR→ESTR, etc.)
/// which can be used by the frontend to suggest appropriate rate indices
/// when a currency is selected.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CurrencyRateIndexMap {
    mapping: HashMap<String, String>,
}

impl CurrencyRateIndexMap {
    /// Creates a new map with default currency→rate index mappings.
    ///
    /// Default mappings:
    /// - USD → SOFR (Secured Overnight Financing Rate)
    /// - EUR → ESTR (Euro Short-Term Rate)
    /// - GBP → SONIA (Sterling Overnight Index Average)
    /// - JPY → TONA (Tokyo Overnight Average Rate)
    /// - CHF → SARON (Swiss Average Rate Overnight)
    #[must_use]
    pub fn new() -> Self {
        let mut mapping = HashMap::new();
        mapping.insert("USD".to_string(), "SOFR".to_string());
        mapping.insert("EUR".to_string(), "ESTR".to_string());
        mapping.insert("GBP".to_string(), "SONIA".to_string());
        mapping.insert("JPY".to_string(), "TONA".to_string());
        mapping.insert("CHF".to_string(), "SARON".to_string());
        Self { mapping }
    }

    /// Returns the rate index for a given currency code.
    #[must_use]
    pub fn get(&self, currency: &str) -> Option<&String> { self.mapping.get(currency) }

    /// Returns the mapping as a JSON object.
    ///
    /// Output format: `{ "USD": "SOFR", "EUR": "ESTR", ... }`
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.mapping).unwrap_or_default()
    }
}

// =============================================================================
// AppConfig
// =============================================================================

/// Unified application configuration for frontend integration.
///
/// Combines enum registries, default values, and currency mappings
/// into a single structure that can be served via `/api/config`.
///
/// Field names use camelCase for frontend compatibility.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// All enum variant names keyed by enum name.
    pub enums: serde_json::Value,
    /// Default values for configuration structures.
    pub defaults: serde_json::Value,
    /// Currency code to rate index mapping.
    pub rate_index_by_currency: serde_json::Value,
}

impl AppConfig {
    /// Builds a complete `AppConfig` from all registries.
    ///
    /// This is the primary entry point for generating the configuration
    /// to be served via the `/api/config` endpoint.
    #[must_use]
    pub fn build() -> Self {
        let currency_map = CurrencyRateIndexMap::new();
        Self {
            enums: EnumRegistry::to_json(),
            defaults: DefaultsRegistry::to_json(),
            rate_index_by_currency: currency_map.to_json(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // EnumRegistry Tests
    // =========================================================================

    #[test]
    fn test_enum_registry_contains_all_target_enums() {
        let json = EnumRegistry::to_json();

        // Verify all target enums are present
        assert!(json.get("pricing_method").is_some());
        assert!(json.get("tree_type").is_some());
        assert!(json.get("greeks_method").is_some());
        assert!(json.get("greek_type").is_some());
        assert!(json.get("second_order_mode").is_some());
        assert!(json.get("shift_type").is_some());
    }

    #[test]
    fn test_enum_registry_pricing_method_variants() {
        let json = EnumRegistry::to_json();
        let variants = json.get("pricing_method").unwrap().as_array().unwrap();

        assert!(variants.iter().any(|v| v == "analytical"));
        assert!(variants.iter().any(|v| v == "monte_carlo"));
        assert!(variants.iter().any(|v| v == "tree"));
    }

    #[test]
    fn test_enum_registry_greek_type_variants() {
        let json = EnumRegistry::to_json();
        let variants = json.get("greek_type").unwrap().as_array().unwrap();

        // Verify all 7 Greek types are present
        assert_eq!(variants.len(), 7);
        assert!(variants.iter().any(|v| v == "delta"));
        assert!(variants.iter().any(|v| v == "gamma"));
        assert!(variants.iter().any(|v| v == "vega"));
        assert!(variants.iter().any(|v| v == "theta"));
        assert!(variants.iter().any(|v| v == "rho"));
        assert!(variants.iter().any(|v| v == "vanna"));
        assert!(variants.iter().any(|v| v == "volga"));
    }

    #[test]
    fn test_enum_registry_greeks_method_variants() {
        let json = EnumRegistry::to_json();
        let variants = json.get("greeks_method").unwrap().as_array().unwrap();

        assert!(variants.iter().any(|v| v == "aad"));
        assert!(variants.iter().any(|v| v == "bump"));
    }

    #[test]
    fn test_enum_registry_output_is_snake_case() {
        let json = EnumRegistry::to_json();
        let pricing_methods = json.get("pricing_method").unwrap().as_array().unwrap();

        // Verify snake_case format (contains underscore for multi-word variants)
        assert!(pricing_methods.iter().any(|v| v == "monte_carlo"));
        // No PascalCase or camelCase
        assert!(!pricing_methods.iter().any(|v| v == "MonteCarlo"));
        assert!(!pricing_methods.iter().any(|v| v == "monteCarlo"));
    }

    // =========================================================================
    // DefaultsRegistry Tests
    // =========================================================================

    #[test]
    fn test_defaults_registry_contains_all_target_structs() {
        let json = DefaultsRegistry::to_json();

        assert!(json.get("monte_carlo").is_some());
        assert!(json.get("tree_params").is_some());
        assert!(json.get("bump_sizes").is_some());
    }

    #[test]
    fn test_defaults_registry_monte_carlo_values() {
        let json = DefaultsRegistry::to_json();
        let mc = json.get("monte_carlo").unwrap();

        assert_eq!(mc.get("num_paths").unwrap(), 10_000);
        assert_eq!(mc.get("num_steps").unwrap(), 252);
        assert!(mc.get("seed").unwrap().is_null());
    }

    #[test]
    fn test_defaults_registry_tree_params_values() {
        let json = DefaultsRegistry::to_json();
        let tree = json.get("tree_params").unwrap();

        assert_eq!(tree.get("num_steps").unwrap(), 100);
        assert_eq!(tree.get("tree_type").unwrap(), "binomial");
    }

    #[test]
    fn test_defaults_registry_bump_sizes_values() {
        let json = DefaultsRegistry::to_json();
        let bump = json.get("bump_sizes").unwrap();

        assert_eq!(bump.get("rate").unwrap(), 0.0001);
        assert_eq!(bump.get("vol").unwrap(), 0.01);
        assert_eq!(bump.get("spot").unwrap(), 0.01);
    }

    // =========================================================================
    // CurrencyRateIndexMap Tests
    // =========================================================================

    #[test]
    fn test_currency_rate_index_map_default_mappings() {
        let map = CurrencyRateIndexMap::new();

        assert_eq!(map.get("USD"), Some(&"SOFR".to_string()));
        assert_eq!(map.get("EUR"), Some(&"ESTR".to_string()));
        assert_eq!(map.get("GBP"), Some(&"SONIA".to_string()));
        assert_eq!(map.get("JPY"), Some(&"TONA".to_string()));
        assert_eq!(map.get("CHF"), Some(&"SARON".to_string()));
    }

    #[test]
    fn test_currency_rate_index_map_unknown_currency() {
        let map = CurrencyRateIndexMap::new();

        assert_eq!(map.get("AUD"), None);
        assert_eq!(map.get("CAD"), None);
    }

    #[test]
    fn test_currency_rate_index_map_to_json() {
        let map = CurrencyRateIndexMap::new();
        let json = map.to_json();

        assert_eq!(json.get("USD").unwrap(), "SOFR");
        assert_eq!(json.get("EUR").unwrap(), "ESTR");
        assert_eq!(json.get("GBP").unwrap(), "SONIA");
        assert_eq!(json.get("JPY").unwrap(), "TONA");
        assert_eq!(json.get("CHF").unwrap(), "SARON");
    }

    // =========================================================================
    // AppConfig Tests
    // =========================================================================

    #[test]
    fn test_app_config_build_contains_all_sections() {
        let config = AppConfig::build();

        assert!(!config.enums.is_null());
        assert!(!config.defaults.is_null());
        assert!(!config.rate_index_by_currency.is_null());
    }

    #[test]
    fn test_app_config_serializes_with_camel_case() {
        let config = AppConfig::build();
        let json = serde_json::to_string(&config).unwrap();

        // Verify camelCase field names
        assert!(json.contains("\"enums\""));
        assert!(json.contains("\"defaults\""));
        assert!(json.contains("\"rateIndexByCurrency\""));

        // No snake_case field names at top level
        assert!(!json.contains("\"rate_index_by_currency\""));
    }

    #[test]
    fn test_app_config_frontend_compatibility() {
        let config = AppConfig::build();
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        // Parse back to verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify frontend-expected structure
        assert!(parsed.get("enums").unwrap().is_object());
        assert!(parsed.get("defaults").unwrap().is_object());
        assert!(parsed.get("rateIndexByCurrency").unwrap().is_object());

        // Verify enums are arrays
        let enums = parsed.get("enums").unwrap();
        assert!(enums.get("pricing_method").unwrap().is_array());
        assert!(enums.get("greek_type").unwrap().is_array());
    }

    #[test]
    fn test_app_config_enums_match_enum_registry() {
        let config = AppConfig::build();
        let enum_registry_json = EnumRegistry::to_json();

        assert_eq!(config.enums, enum_registry_json);
    }

    #[test]
    fn test_app_config_defaults_match_defaults_registry() {
        let config = AppConfig::build();
        let defaults_registry_json = DefaultsRegistry::to_json();

        assert_eq!(config.defaults, defaults_registry_json);
    }
}

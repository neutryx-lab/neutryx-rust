//! Configuration export handler for frontend integration.
//!
//! Provides the `/api/config` endpoint that returns application configuration
//! including enum values, default settings, and currency mappings.

use axum::Json;
use infra_config::AppConfig;

/// Get application configuration.
///
/// GET /api/config
///
/// Returns a JSON object containing:
/// - `enums`: All enum variant names for form dropdowns
/// - `defaults`: Default values for configuration structures
/// - `rateIndexByCurrency`: Currency code to rate index mapping
///
/// # Responses
///
/// - 200 OK: Returns `AppConfig` as JSON
pub async fn get_config() -> Json<AppConfig> { Json(AppConfig::build()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_config_returns_valid_response() {
        let Json(config) = get_config().await;

        // Verify all sections are present
        assert!(!config.enums.is_null());
        assert!(!config.defaults.is_null());
        assert!(!config.rate_index_by_currency.is_null());
    }

    #[tokio::test]
    async fn test_get_config_enums_contain_expected_keys() {
        let Json(config) = get_config().await;

        assert!(config.enums.get("pricing_method").is_some());
        assert!(config.enums.get("greek_type").is_some());
        assert!(config.enums.get("greeks_method").is_some());
    }

    #[tokio::test]
    async fn test_get_config_defaults_contain_expected_keys() {
        let Json(config) = get_config().await;

        assert!(config.defaults.get("monte_carlo").is_some());
        assert!(config.defaults.get("tree_params").is_some());
        assert!(config.defaults.get("bump_sizes").is_some());
    }

    #[tokio::test]
    async fn test_get_config_rate_index_contains_major_currencies() {
        let Json(config) = get_config().await;

        assert!(config.rate_index_by_currency.get("USD").is_some());
        assert!(config.rate_index_by_currency.get("EUR").is_some());
        assert!(config.rate_index_by_currency.get("JPY").is_some());
    }
}

//! Configuration API handlers.
//!
//! This module provides REST API handlers for the Configuration API.
//!
//! # Endpoints
//!
//! | Method | Path               | Description                    |
//! |--------|-------------------|--------------------------------|
//! | GET    | /api/config       | Get all configuration          |
//! | GET    | /api/config/enums | Get Enum values only           |
//! | GET    | /api/config/defaults | Get default values only     |

use axum::Json;

use crate::web::config_types::{ConfigResponse, DefaultValues, EnumValues};

// =============================================================================
// GET /api/config - Complete Configuration
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
pub async fn get_config() -> Json<ConfigResponse> {
    Json(ConfigResponse::build())
}

// =============================================================================
// GET /api/config/enums - Enum Values Only
// =============================================================================

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
pub async fn get_enums() -> Json<EnumValues> {
    Json(EnumValues::build())
}

// =============================================================================
// GET /api/config/defaults - Default Values Only
// =============================================================================

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
pub async fn get_defaults() -> Json<DefaultValues> {
    Json(DefaultValues::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

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

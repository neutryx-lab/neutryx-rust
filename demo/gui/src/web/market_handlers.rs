//! Market Data API handlers.
//!
//! This module provides REST API handlers for the Market Data Viewer.
//!
//! # Task Coverage
//!
//! - Task 3.2: GET /api/market/rates - List market rates
//! - Task 3.3: GET /api/market/rates/:id - Get rate detail
//! - Task 3.4: GET /api/market/conventions - List conventions GET
//!   /api/market/conventions/:id - Get convention detail
//! - Task 3.5: Router integration
//!
//! # Endpoints
//!
//! | Method | Path                      | Description                    |
//! |--------|---------------------------|--------------------------------|
//! | GET    | /api/market/rates         | List all rates (filterable)    |
//! | GET    | /api/market/rates/:id     | Get rate detail with instrument|
//! | POST   | /api/market/rates/refresh | Refresh rate data              |
//! | GET    | /api/market/conventions   | List all conventions           |
//! | GET    | /api/market/conventions/:id | Get convention detail        |
//! | GET    | /api/market/export/csv    | Export rates as CSV            |
//! | GET    | /api/market/export/json   | Export rates as JSON           |

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{
    market_data::{get_convention, get_conventions_list, MarketDataCache},
    market_types::{
        ConventionResponse, ConventionsListResponse, MarketDataApiError, MarketRateDetailResponse,
        MarketRateQuery, MarketRatesListResponse,
    },
};

// =============================================================================
// Task 3.2: GET /api/market/rates - List Market Rates
// =============================================================================

/// List market rates with optional filtering.
///
/// # Query Parameters
///
/// - `currency`: Filter by currency code (e.g., "USD", "EUR")
/// - `rateType`: Filter by rate type (e.g., "Swap", "Ois", "Deposit")
/// - `index`: Filter by rate index (e.g., "SOFR", "EURIBOR")
///
/// # Returns
///
/// JSON array of market rates with metadata.
///
/// # Example
///
/// ```text
/// GET /api/market/rates?currency=USD&rateType=Swap
/// ```
pub async fn get_market_rates(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> Json<MarketRatesListResponse> {
    let response = cache.get_rates(&query).await;
    Json(response)
}

// =============================================================================
// Task 3.3: GET /api/market/rates/:id - Get Rate Detail
// =============================================================================

/// Get detailed information for a specific rate.
///
/// Returns the rate with linked instrument and convention information.
///
/// # Path Parameters
///
/// - `id`: Rate identifier (e.g., "USD-5Y-SWAP")
///
/// # Returns
///
/// - 200: Rate detail with instrument and convention
/// - 404: Rate not found
///
/// # Example
///
/// ```text
/// GET /api/market/rates/USD-5Y-SWAP
/// ```
pub async fn get_market_rate_detail(
    State(cache): State<Arc<MarketDataCache>>,
    Path(id): Path<String>,
) -> Result<Json<MarketRateDetailResponse>, (StatusCode, Json<MarketDataApiError>)> {
    match cache.get_rate_detail(&id).await {
        Some(detail) => Ok(Json(detail)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(MarketDataApiError::not_found("Rate", &id)),
        )),
    }
}

// =============================================================================
// Task 5.1: POST /api/market/rates/refresh - Refresh Rate Data
// =============================================================================

/// Refresh market rate data.
///
/// Simulates rate refresh by applying small random perturbations
/// to all rates.
///
/// # Returns
///
/// - 200: Refresh successful with new timestamp
pub async fn refresh_market_rates(
    State(cache): State<Arc<MarketDataCache>>,
) -> Json<RefreshResponse> {
    cache.refresh().await;
    Json(RefreshResponse {
        success: true,
        last_updated: cache.last_updated(),
    })
}

/// Response for rate refresh operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    /// Whether refresh was successful.
    pub success: bool,
    /// New last_updated timestamp.
    pub last_updated: i64,
}

// =============================================================================
// Task 3.4: GET /api/market/conventions - List Conventions
// =============================================================================

/// List all available market conventions.
///
/// # Returns
///
/// JSON array of convention summaries.
pub async fn get_market_conventions() -> Json<ConventionsListResponse> {
    Json(get_conventions_list())
}

/// Get detailed information for a specific convention.
///
/// # Path Parameters
///
/// - `id`: Convention identifier (e.g., "USD-SOFR-OIS")
///
/// # Returns
///
/// - 200: Convention detail with all fields
/// - 404: Convention not found
pub async fn get_market_convention_detail(
    Path(id): Path<String>,
) -> Result<Json<ConventionResponse>, (StatusCode, Json<MarketDataApiError>)> {
    match get_convention(&id) {
        Some(convention) => Ok(Json(convention)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(MarketDataApiError::not_found("Convention", &id)),
        )),
    }
}

// =============================================================================
// Task 6.1: GET /api/market/export/csv - Export as CSV
// =============================================================================

/// Export market rates as CSV.
///
/// # Query Parameters
///
/// Same as GET /api/market/rates
///
/// # Returns
///
/// CSV file with Content-Type: text/csv
pub async fn export_rates_csv(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> impl IntoResponse {
    let response = cache.get_rates(&query).await;

    // Build CSV content
    let mut csv = String::from(
        "id,currency,tenor,rateType,value,quoteType,timestamp,source,isStale,rateIndex\n",
    );

    for rate in &response.rates {
        csv.push_str(&format!(
            "{},{},{},{},{:.6},{},{},{},{},{}\n",
            rate.id,
            rate.currency,
            rate.tenor,
            rate.rate_type,
            rate.value,
            rate.quote_type,
            rate.timestamp,
            rate.source,
            rate.is_stale,
            rate.rate_index.as_deref().unwrap_or("")
        ));
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            "attachment; filename=\"market_rates.csv\"",
        )
        .body(csv)
        .unwrap()
}

// =============================================================================
// Task 6.2: GET /api/market/export/json - Export as JSON
// =============================================================================

/// Export market rates as JSON file.
///
/// # Query Parameters
///
/// Same as GET /api/market/rates
///
/// # Returns
///
/// JSON file with Content-Disposition header for download
pub async fn export_rates_json(
    State(cache): State<Arc<MarketDataCache>>,
    Query(query): Query<MarketRateQuery>,
) -> impl IntoResponse {
    let response = cache.get_rates(&query).await;
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json; charset=utf-8")
        .header(
            "Content-Disposition",
            "attachment; filename=\"market_rates.json\"",
        )
        .body(json)
        .unwrap()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod handler_tests {
        use super::*;

        #[tokio::test]
        async fn test_get_market_rates_returns_rates() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery::default();

            let Json(response) = get_market_rates(State(cache), Query(query)).await;

            assert!(response.total_count > 0);
            assert!(!response.rates.is_empty());
        }

        #[tokio::test]
        async fn test_get_market_rates_with_currency_filter() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: None,
                index: None,
            };

            let Json(response) = get_market_rates(State(cache), Query(query)).await;

            for rate in &response.rates {
                assert_eq!(rate.currency, "USD");
            }
        }

        #[tokio::test]
        async fn test_get_market_rate_detail_found() {
            let cache = Arc::new(MarketDataCache::new());

            let result =
                get_market_rate_detail(State(cache), Path("USD-5Y-SWAP".to_string())).await;

            assert!(result.is_ok());
            let Json(detail) = result.unwrap();
            assert_eq!(detail.rate.id, "USD-5Y-SWAP");
            assert!(detail.instrument.is_some());
            assert!(detail.convention.is_some());
        }

        #[tokio::test]
        async fn test_get_market_rate_detail_not_found() {
            let cache = Arc::new(MarketDataCache::new());

            let result =
                get_market_rate_detail(State(cache), Path("INVALID-RATE".to_string())).await;

            assert!(result.is_err());
            let (status, Json(error)) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(error.error, "not_found");
        }

        #[tokio::test]
        async fn test_refresh_market_rates() {
            let cache = Arc::new(MarketDataCache::new());
            let initial_ts = cache.last_updated();

            // Wait a bit
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            let Json(response) = refresh_market_rates(State(cache)).await;

            assert!(response.success);
            assert!(response.last_updated > initial_ts);
        }

        #[tokio::test]
        async fn test_get_market_conventions() {
            let Json(response) = get_market_conventions().await;

            assert!(!response.conventions.is_empty());
        }

        #[tokio::test]
        async fn test_get_market_convention_detail_found() {
            let result = get_market_convention_detail(Path("USD-SOFR-OIS".to_string())).await;

            assert!(result.is_ok());
            let Json(convention) = result.unwrap();
            assert_eq!(convention.convention_type, "OisConvention");
        }

        #[tokio::test]
        async fn test_get_market_convention_detail_not_found() {
            let result = get_market_convention_detail(Path("INVALID-CONVENTION".to_string())).await;

            assert!(result.is_err());
            let (status, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_export_rates_csv() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: Some("Deposit".to_string()),
                index: None,
            };

            let response = export_rates_csv(State(cache), Query(query))
                .await
                .into_response();

            assert_eq!(response.status(), StatusCode::OK);

            // Check content-type header
            let content_type = response.headers().get("content-type");
            assert!(content_type.is_some());
            assert!(content_type.unwrap().to_str().unwrap().contains("text/csv"));
        }

        #[tokio::test]
        async fn test_export_rates_json() {
            let cache = Arc::new(MarketDataCache::new());
            let query = MarketRateQuery::default();

            let response = export_rates_json(State(cache), Query(query))
                .await
                .into_response();

            assert_eq!(response.status(), StatusCode::OK);

            let content_type = response.headers().get("content-type");
            assert!(content_type.is_some());
            assert!(content_type
                .unwrap()
                .to_str()
                .unwrap()
                .contains("application/json"));
        }
    }
}

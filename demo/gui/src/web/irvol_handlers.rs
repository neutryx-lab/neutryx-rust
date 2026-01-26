//! IR Volatility (Swaption Vol) API handlers for the WebApp.
//!
//! Provides REST API endpoints for IR volatility surface operations.

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tokio::{fs, sync::RwLock};
use tracing::info;
use uuid::Uuid;

use infra_master::market::Currency;

use super::{irvol_types::*, AppState};

// =============================================================================
// Cache for Built Surfaces
// =============================================================================

/// LRU cache for built IR vol surfaces.
pub struct IrVolCache {
    surfaces: RwLock<HashMap<String, CachedIrVolSurface>>,
    max_size: usize,
}

pub(crate) struct CachedIrVolSurface {
    currency: String,
    quotes: Vec<SwaptionVolQuote>,
    built_at: i64,
    expiry_interp: InterpMethod,
    tenor_interp: InterpMethod,
}

impl IrVolCache {
    /// Create a new IR vol cache with the given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            surfaces: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    /// Insert a surface into the cache.
    pub(crate) async fn insert(&self, id: String, surface: CachedIrVolSurface) {
        let mut surfaces = self.surfaces.write().await;
        if surfaces.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest) = surfaces.keys().next().cloned() {
                surfaces.remove(&oldest);
            }
        }
        surfaces.insert(id, surface);
    }

    /// Get a surface from the cache by ID.
    pub(crate) async fn get(&self, id: &str) -> Option<CachedIrVolSurface> {
        self.surfaces.read().await.get(id).cloned()
    }
}

impl Clone for CachedIrVolSurface {
    fn clone(&self) -> Self {
        Self {
            currency: self.currency.clone(),
            quotes: self.quotes.clone(),
            built_at: self.built_at,
            expiry_interp: self.expiry_interp,
            tenor_interp: self.tenor_interp,
        }
    }
}

// =============================================================================
// Shared State
// =============================================================================

/// Shared state for IR vol handlers.
pub type IrVolState = Arc<IrVolCache>;

/// Create default IR vol state.
pub fn create_irvol_state() -> IrVolState { Arc::new(IrVolCache::new(10)) }

// =============================================================================
// Data Loading
// =============================================================================

/// Get available currencies for IR vol data from infra_master.
fn get_available_currencies() -> Vec<(&'static str, &'static str)> {
    Currency::all().iter().map(|c| (c.code(), c.name())).collect()
}

/// Load IR vol quotes from JSON file.
async fn load_quotes_from_file(currency: &str) -> Result<Vec<SwaptionVolQuote>, String> {
    let path = format!("demo/data/input/irvol/{}.json", currency.to_lowercase());

    match fs::read_to_string(&path).await {
        Ok(content) => {
            let data: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

            let quotes = data
                .get("quotes")
                .and_then(|q| q.as_array())
                .ok_or_else(|| "Missing quotes array".to_string())?;

            let mut result = Vec::new();
            for q in quotes {
                let quote: SwaptionVolQuote = serde_json::from_value(q.clone())
                    .map_err(|e| format!("Quote parse error: {}", e))?;
                result.push(quote);
            }
            Ok(result)
        }
        Err(_) => {
            // Return demo data if file not found
            Ok(generate_demo_quotes(currency))
        }
    }
}

/// Generate demo swaption vol quotes for a currency.
fn generate_demo_quotes(currency: &str) -> Vec<SwaptionVolQuote> {
    let expiries = ["1M", "3M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y"];
    let tenors = ["1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"];

    // Base ATM vol levels by currency (in %)
    let base_vol = match currency {
        "USD" => 0.55,
        "EUR" => 0.48,
        "GBP" => 0.52,
        "JPY" => 0.35,
        _ => 0.50,
    };

    let mut quotes = Vec::new();

    for (i, expiry) in expiries.iter().enumerate() {
        for (j, tenor) in tenors.iter().enumerate() {
            // Vol surface shape: higher vol for short expiry + long tenor
            let expiry_factor = 1.0 - (i as f64) * 0.02;
            let tenor_factor = 1.0 + (j as f64) * 0.01;
            let vol = base_vol * expiry_factor * tenor_factor;

            // Add some smile for illustration
            let smile = vec![
                SmilePoint {
                    strike_offset_bp: -100,
                    vol: vol * 1.08,
                },
                SmilePoint {
                    strike_offset_bp: -50,
                    vol: vol * 1.03,
                },
                SmilePoint {
                    strike_offset_bp: 0,
                    vol,
                },
                SmilePoint {
                    strike_offset_bp: 50,
                    vol: vol * 1.02,
                },
                SmilePoint {
                    strike_offset_bp: 100,
                    vol: vol * 1.06,
                },
            ];

            quotes.push(SwaptionVolQuote::new_atm(expiry, tenor, vol).with_smile(smile));
        }
    }

    quotes
}

// =============================================================================
// API Handlers
// =============================================================================

/// GET /api/irvol/currencies
/// List available currencies with IR vol data.
pub async fn get_currencies() -> impl IntoResponse {
    let mut currencies = Vec::new();

    for (code, name) in get_available_currencies() {
        let quotes = generate_demo_quotes(code);
        let mut expiries: Vec<String> = quotes.iter().map(|q| q.expiry.clone()).collect();
        expiries.sort();
        expiries.dedup();

        let mut tenors: Vec<String> = quotes.iter().map(|q| q.tenor.clone()).collect();
        tenors.sort();
        tenors.dedup();

        currencies.push(IrVolCurrencyInfo {
            currency: code.to_string(),
            display_name: name.to_string(),
            default_vol_type: VolQuoteType::Normal,
            quote_count: quotes.len(),
            expiries,
            tenors,
        });
    }

    Json(IrVolCurrenciesResponse { currencies })
}

/// GET /api/irvol/quotes/{currency}
/// Get IR vol quotes for a currency.
pub async fn get_quotes(Path(currency): Path<String>) -> impl IntoResponse {
    let currency = currency.to_uppercase();

    if !get_available_currencies().iter().any(|(c, _)| *c == currency) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": format!("Currency {} not found", currency)
            })),
        )
            .into_response();
    }

    match load_quotes_from_file(&currency).await {
        Ok(quotes) => {
            let response = IrVolQuotesResponse {
                currency,
                vol_type: VolQuoteType::Normal,
                quotes,
                last_updated: chrono::Utc::now().timestamp_millis(),
                source: "Demo".to_string(),
            };
            Json(response).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "load_error",
                "message": e
            })),
        )
            .into_response(),
    }
}

/// PUT /api/irvol/quotes/{currency}
/// Update IR vol quotes for a currency.
pub async fn update_quotes(
    Path(currency): Path<String>,
    Json(request): Json<IrVolQuotesRequest>,
) -> impl IntoResponse {
    let currency = currency.to_uppercase();
    info!(
        "Updating IR vol quotes for {}: {} quotes",
        currency,
        request.quotes.len()
    );

    // In a real implementation, this would persist the data
    // For demo, just acknowledge the update
    Json(json!({
        "success": true,
        "currency": currency,
        "quotes_updated": request.quotes.len()
    }))
}

/// POST /api/irvol/build
/// Build IR vol surface from quotes.
pub async fn build_surface(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IrVolSurfaceBuildRequest>,
) -> impl IntoResponse {
    let currency = request.currency.to_uppercase();
    info!("Building IR vol surface for {}", currency);

    match load_quotes_from_file(&currency).await {
        Ok(quotes) => {
            let surface_id = Uuid::new_v4().to_string();
            let built_at = chrono::Utc::now().timestamp_millis();

            let cached = CachedIrVolSurface {
                currency: currency.clone(),
                quotes,
                built_at,
                expiry_interp: request.expiry_interp,
                tenor_interp: request.tenor_interp,
            };
            state.irvol_cache.insert(surface_id.clone(), cached).await;

            let response = IrVolSurfaceBuildResponse {
                success: true,
                surface_id,
                currency,
                built_at,
                warnings: vec![],
            };
            Json(response).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "build_error",
                "message": e
            })),
        )
            .into_response(),
    }
}

/// GET /api/irvol/smile
/// Get smile curve at specific expiry/tenor.
pub async fn get_smile(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IrVolSmileQuery>,
) -> impl IntoResponse {
    let surface = match state.irvol_cache.get(&query.surface_id).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "not_found",
                    "message": "Surface not found"
                })),
            )
                .into_response();
        }
    };

    let expiry_years = parse_tenor_to_years(&query.expiry);
    let tenor_years = parse_tenor_to_years(&query.tenor);

    // Find closest quote
    let closest = surface.quotes.iter().min_by(|a, b| {
        let dist_a =
            (a.expiry_years() - expiry_years).abs() + (a.tenor_years() - tenor_years).abs();
        let dist_b =
            (b.expiry_years() - expiry_years).abs() + (b.tenor_years() - tenor_years).abs();
        dist_a.partial_cmp(&dist_b).unwrap()
    });

    match closest {
        Some(quote) => {
            let (strike_offsets, vols): (Vec<i32>, Vec<f64>) = quote
                .smile
                .as_ref()
                .map(|s| s.iter().map(|p| (p.strike_offset_bp, p.vol)).unzip())
                .unwrap_or_else(|| (vec![0], vec![quote.atm_vol]));

            Json(IrVolSmileResponse {
                expiry: quote.expiry_years(),
                tenor: quote.tenor_years(),
                atm_vol: quote.atm_vol,
                strike_offsets,
                vols,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": "No quotes available"
            })),
        )
            .into_response(),
    }
}

/// GET /api/irvol/atm-term
/// Get ATM term structure.
pub async fn get_atm_term(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IrVolAtmTermQuery>,
) -> impl IntoResponse {
    let surface = match state.irvol_cache.get(&query.surface_id).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "not_found",
                    "message": "Surface not found"
                })),
            )
                .into_response();
        }
    };

    let tenor_years = if query.tenor == "diagonal" {
        None
    } else {
        Some(parse_tenor_to_years(&query.tenor))
    };

    let mut points: Vec<(f64, f64)> = surface
        .quotes
        .iter()
        .filter(|q| {
            if let Some(ty) = tenor_years {
                (q.tenor_years() - ty).abs() < 0.1
            } else {
                // Diagonal: expiry == tenor
                (q.expiry_years() - q.tenor_years()).abs() < 0.1
            }
        })
        .map(|q| (q.expiry_years(), q.atm_vol))
        .collect();

    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let (expiries, atm_vols): (Vec<f64>, Vec<f64>) = points.into_iter().unzip();

    Json(IrVolAtmTermResponse {
        tenor: query.tenor,
        expiries,
        atm_vols,
    })
    .into_response()
}

/// GET /api/irvol/surface
/// Get full 3D surface data for visualisation.
pub async fn get_surface(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IrVolSurfaceQuery>,
) -> impl IntoResponse {
    let surface = match state.irvol_cache.get(&query.surface_id).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "not_found",
                    "message": "Surface not found"
                })),
            )
                .into_response();
        }
    };

    // Collect unique expiries and tenors
    let mut expiries: Vec<f64> = surface
        .quotes
        .iter()
        .map(|q| (q.expiry_years() * 100.0).round() / 100.0)
        .collect();
    expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expiries.dedup();

    let mut tenors: Vec<f64> = surface
        .quotes
        .iter()
        .map(|q| (q.tenor_years() * 100.0).round() / 100.0)
        .collect();
    tenors.sort_by(|a, b| a.partial_cmp(b).unwrap());
    tenors.dedup();

    // Build vol matrix
    let mut vols = vec![vec![0.0; tenors.len()]; expiries.len()];
    for quote in &surface.quotes {
        let exp_y = (quote.expiry_years() * 100.0).round() / 100.0;
        let ten_y = (quote.tenor_years() * 100.0).round() / 100.0;

        if let Some(i) = expiries.iter().position(|&x| (x - exp_y).abs() < 0.01) {
            if let Some(j) = tenors.iter().position(|&x| (x - ten_y).abs() < 0.01) {
                vols[i][j] = quote.atm_vol;
            }
        }
    }

    Json(IrVolSurfaceResponse {
        expiries,
        tenors,
        vols,
        vol_type: VolQuoteType::Normal,
        metadata: SurfaceMetadata {
            currency: surface.currency.clone(),
            built_at: surface.built_at,
            interp_method: format!("{:?}/{:?}", surface.expiry_interp, surface.tenor_interp),
        },
    })
    .into_response()
}

// =============================================================================
// Helpers
// =============================================================================

fn parse_tenor_to_years(tenor: &str) -> f64 {
    let tenor = tenor.to_uppercase();
    if let Some(num) = tenor.strip_suffix('Y') {
        num.parse().unwrap_or(1.0)
    } else if let Some(num) = tenor.strip_suffix('M') {
        num.parse::<f64>().unwrap_or(1.0) / 12.0
    } else if let Some(num) = tenor.strip_suffix('W') {
        num.parse::<f64>().unwrap_or(1.0) / 52.0
    } else if let Some(num) = tenor.strip_suffix('D') {
        num.parse::<f64>().unwrap_or(1.0) / 365.0
    } else {
        tenor.parse().unwrap_or(1.0)
    }
}

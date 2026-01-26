//! IR Volatility (Swaption Vol) API type definitions for the WebApp.
//!
//! This module provides request/response types for the IR volatility surface
//! API, including swaption volatility quotes, ATM vol term structure,
//! and smile interpolation.
//!
//! # API Endpoints Coverage
//!
//! - `GET /api/irvol/currencies` → `IrVolCurrenciesResponse`
//! - `GET /api/irvol/quotes/{currency}` → `IrVolQuotesResponse`
//! - `PUT /api/irvol/quotes/{currency}` → `IrVolQuotesRequest`
//! - `POST /api/irvol/build` → `IrVolSurfaceBuildRequest`,
//!   `IrVolSurfaceBuildResponse`
//! - `GET /api/irvol/smile` → `IrVolSmileResponse`
//! - `GET /api/irvol/atm-term` → `IrVolAtmTermResponse`
//! - `GET /api/irvol/surface` → `IrVolSurfaceResponse`

use serde::{Deserialize, Serialize};

// =============================================================================
// Vol Type Enum
// =============================================================================

/// Volatility quote type for swaptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VolQuoteType {
    /// Normal (Bachelier) volatility in basis points.
    #[default]
    Normal,
    /// Lognormal (Black) volatility in percentage.
    Lognormal,
    /// Shifted Lognormal volatility.
    ShiftedLognormal,
}

impl VolQuoteType {
    /// Get the display name for this vol type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal (bp)",
            Self::Lognormal => "Lognormal (%)",
            Self::ShiftedLognormal => "Shifted Lognormal",
        }
    }

    /// Get the unit string.
    pub fn unit(&self) -> &'static str {
        match self {
            Self::Normal => "bp",
            Self::Lognormal => "%",
            Self::ShiftedLognormal => "%",
        }
    }
}

// =============================================================================
// Strike Type Enum
// =============================================================================

/// Strike convention for swaption volatility quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StrikeType {
    /// Absolute strike rate (e.g., 2.5% = 0.025).
    #[default]
    Absolute,
    /// Relative to ATM forward (e.g., +50bp, -100bp).
    RelativeToAtm,
    /// Moneyness (K/F ratio).
    Moneyness,
}

// =============================================================================
// Swaption Vol Quote Entry
// =============================================================================

/// Swaption volatility quote entry for a single expiry/tenor point.
///
/// Contains ATM vol and optional smile quotes (by strike offset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaptionVolQuote {
    /// Option expiry tenor (e.g., "1Y", "5Y", "10Y").
    pub expiry: String,
    /// Underlying swap tenor (e.g., "1Y", "5Y", "10Y").
    pub tenor: String,
    /// ATM volatility value.
    pub atm_vol: f64,
    /// Volatility quote type.
    #[serde(default)]
    pub vol_type: VolQuoteType,
    /// Smile quotes: strike offset -> volatility.
    /// Keys are strike offsets in basis points (e.g., -100, -50, 0, +50, +100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smile: Option<Vec<SmilePoint>>,
    /// Shift parameter for shifted lognormal (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<f64>,
}

/// A single point on the volatility smile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmilePoint {
    /// Strike offset in basis points relative to ATM.
    pub strike_offset_bp: i32,
    /// Volatility at this strike.
    pub vol: f64,
}

impl SwaptionVolQuote {
    /// Create a new ATM-only quote.
    pub fn new_atm(expiry: &str, tenor: &str, atm_vol: f64) -> Self {
        Self {
            expiry: expiry.to_string(),
            tenor: tenor.to_string(),
            atm_vol,
            vol_type: VolQuoteType::default(),
            smile: None,
            shift: None,
        }
    }

    /// Create with full smile.
    pub fn with_smile(mut self, smile: Vec<SmilePoint>) -> Self {
        self.smile = Some(smile);
        self
    }

    /// Get expiry in years (approximate).
    pub fn expiry_years(&self) -> f64 { parse_tenor_years(&self.expiry) }

    /// Get swap tenor in years (approximate).
    pub fn tenor_years(&self) -> f64 { parse_tenor_years(&self.tenor) }
}

/// Parse a tenor string to years (approximate).
fn parse_tenor_years(tenor: &str) -> f64 {
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

// =============================================================================
// API Request/Response Types
// =============================================================================

/// Response for available currencies with IR vol data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolCurrenciesResponse {
    /// List of available currencies.
    pub currencies: Vec<IrVolCurrencyInfo>,
}

/// Currency info for IR vol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolCurrencyInfo {
    /// Currency code (e.g., "USD", "EUR").
    pub currency: String,
    /// Display name.
    pub display_name: String,
    /// Default volatility type.
    pub default_vol_type: VolQuoteType,
    /// Number of expiry/tenor points available.
    pub quote_count: usize,
    /// Available expiry tenors.
    pub expiries: Vec<String>,
    /// Available swap tenors.
    pub tenors: Vec<String>,
}

/// Response for IR vol quotes for a currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuotesResponse {
    /// Currency code.
    pub currency: String,
    /// Volatility type.
    pub vol_type: VolQuoteType,
    /// Quotes grid (expiry x tenor).
    pub quotes: Vec<SwaptionVolQuote>,
    /// Last update timestamp.
    pub last_updated: i64,
    /// Data source.
    pub source: String,
}

/// Request to update IR vol quotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolQuotesRequest {
    /// Updated quotes.
    pub quotes: Vec<SwaptionVolQuote>,
}

/// Request to build IR vol surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSurfaceBuildRequest {
    /// Currency code.
    pub currency: String,
    /// Interpolation method for expiry dimension.
    #[serde(default)]
    pub expiry_interp: InterpMethod,
    /// Interpolation method for tenor dimension.
    #[serde(default)]
    pub tenor_interp: InterpMethod,
    /// Smile model (if smile data available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smile_model: Option<SmileModel>,
}

/// Interpolation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterpMethod {
    /// Linear interpolation.
    #[default]
    Linear,
    /// Cubic spline.
    CubicSpline,
    /// Flat extrapolation.
    Flat,
}

/// Smile model for vol surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SmileModel {
    /// No smile (ATM only).
    #[default]
    None,
    /// SABR model.
    Sabr,
    /// SVI model.
    Svi,
}

/// Response from building IR vol surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSurfaceBuildResponse {
    /// Build success.
    pub success: bool,
    /// Surface ID for subsequent queries.
    pub surface_id: String,
    /// Currency.
    pub currency: String,
    /// Build timestamp.
    pub built_at: i64,
    /// Diagnostics/warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Request for smile at specific expiry/tenor.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSmileQuery {
    /// Surface ID.
    pub surface_id: String,
    /// Expiry (tenor string or years).
    pub expiry: String,
    /// Swap tenor (tenor string or years).
    pub tenor: String,
    /// Number of strike points.
    #[serde(default = "default_num_points")]
    pub num_points: usize,
    /// Strike range in bp from ATM.
    #[serde(default = "default_strike_range")]
    pub strike_range_bp: i32,
}

fn default_num_points() -> usize { 21 }

fn default_strike_range() -> i32 { 200 }

/// Response for smile curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSmileResponse {
    /// Expiry (years).
    pub expiry: f64,
    /// Swap tenor (years).
    pub tenor: f64,
    /// ATM vol.
    pub atm_vol: f64,
    /// Strike offsets (bp).
    pub strike_offsets: Vec<i32>,
    /// Volatilities at each strike.
    pub vols: Vec<f64>,
}

/// Request for ATM term structure.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolAtmTermQuery {
    /// Surface ID.
    pub surface_id: String,
    /// Fixed tenor (e.g., "10Y") or "diagonal" for matching expiry=tenor.
    pub tenor: String,
}

/// Response for ATM term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolAtmTermResponse {
    /// Tenor description.
    pub tenor: String,
    /// Expiry points (years).
    pub expiries: Vec<f64>,
    /// ATM vols at each expiry.
    pub atm_vols: Vec<f64>,
}

/// Request for full 3D surface data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSurfaceQuery {
    /// Surface ID.
    pub surface_id: String,
    /// Number of expiry points.
    #[serde(default = "default_surface_points")]
    pub num_expiry_points: usize,
    /// Number of tenor points.
    #[serde(default = "default_surface_points")]
    pub num_tenor_points: usize,
}

fn default_surface_points() -> usize { 15 }

/// Response for 3D surface data (for Plotly).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrVolSurfaceResponse {
    /// Expiry axis (years).
    pub expiries: Vec<f64>,
    /// Tenor axis (years).
    pub tenors: Vec<f64>,
    /// Volatility matrix [expiry][tenor].
    pub vols: Vec<Vec<f64>>,
    /// Volatility type.
    pub vol_type: VolQuoteType,
    /// Surface metadata.
    pub metadata: SurfaceMetadata,
}

/// Surface metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceMetadata {
    /// Currency.
    pub currency: String,
    /// Build timestamp.
    pub built_at: i64,
    /// Interpolation methods used.
    pub interp_method: String,
}

// =============================================================================
// Cap/Floor Volatility Types
// =============================================================================

/// Cap/Floor volatility quote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapFloorVolQuote {
    /// Maturity tenor (e.g., "2Y", "5Y").
    pub maturity: String,
    /// Strike rate (absolute).
    pub strike: f64,
    /// Volatility value.
    pub vol: f64,
    /// Quote type (Cap or Floor).
    pub instrument_type: CapFloorType,
    /// Volatility type.
    #[serde(default)]
    pub vol_type: VolQuoteType,
}

/// Re-export CapFloorType from infra_master.
pub use infra_master::trade::instrument_def::CapFloorType;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tenor_years() {
        assert!((parse_tenor_years("1Y") - 1.0).abs() < 0.001);
        assert!((parse_tenor_years("6M") - 0.5).abs() < 0.001);
        assert!((parse_tenor_years("3M") - 0.25).abs() < 0.001);
        assert!((parse_tenor_years("10Y") - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_swaption_vol_quote_serialise() {
        let quote = SwaptionVolQuote::new_atm("5Y", "10Y", 0.65);
        let json = serde_json::to_string(&quote).unwrap();
        assert!(json.contains("\"expiry\":\"5Y\""));
        assert!(json.contains("\"tenor\":\"10Y\""));
        assert!(json.contains("\"atmVol\":0.65"));
    }

    #[test]
    fn test_vol_quote_type_display() {
        assert_eq!(VolQuoteType::Normal.display_name(), "Normal (bp)");
        assert_eq!(VolQuoteType::Lognormal.unit(), "%");
    }

    #[test]
    fn test_smile_point() {
        let point = SmilePoint {
            strike_offset_bp: -50,
            vol: 0.68,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("\"strikeOffsetBp\":-50"));
    }
}

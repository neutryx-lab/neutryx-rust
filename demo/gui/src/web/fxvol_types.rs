//! FxVol API type definitions for the WebApp.
//!
//! This module provides request/response types for the FX volatility surface API,
//! including delta-based quotes, Risk Reversal/Butterfly analysis, and probability
//! density calculation.
//!
//! # API Endpoints Coverage
//!
//! - `GET /api/fxvol/pairs` → `FxVolPairsResponse`
//! - `GET /api/fxvol/quotes/{pair}` → `FxQuotesResponse`
//! - `PUT /api/fxvol/quotes/{pair}` → `FxQuotesRequest`
//! - `POST /api/fxvol/build` → `FxSurfaceBuildRequest`, `FxSurfaceBuildResponse`
//! - `GET /api/fxvol/smile` → `FxSmileResponse`
//! - `GET /api/fxvol/rr-bf` → `RrBfResponse`
//! - `GET /api/fxvol/density` → `FxDensityResponse`
//! - `POST /api/fxvol/delta-strike` → `DeltaStrikeRequest`, `DeltaStrikeResponse`
//!
//! # Requirements Coverage
//!
//! - Requirement 1: ボラティリティデータ管理 (FX)
//! - Requirement 10: FX VolSurface専用機能
//! - Requirement 11: FX VolSurface バックエンドAPI

use serde::{Deserialize, Serialize};

// =============================================================================
// Delta Type Enum (Req 10.5)
// =============================================================================

/// Delta convention type for FX options.
///
/// Different market conventions use different delta definitions based on
/// the regional market practice.
///
/// # Requirements Coverage
///
/// - Requirement 10.5: Delta表現でのスマイル構造
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeltaType {
    /// Spot delta (premium excluded).
    /// Most common in G10 FX markets.
    /// Δ = exp(-r_f * T) * N(d1) for calls.
    #[default]
    SpotDelta,
    /// Forward delta.
    /// Premium excluded, measured vs forward.
    /// Δ = N(d1) for calls.
    ForwardDelta,
    /// Premium-adjusted delta.
    /// Common in EM FX markets.
    /// Δ = exp(-r_f * T) * N(d1) * K / F for calls.
    PremiumAdjusted,
}

impl DeltaType {
    /// Get the display name for this delta type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SpotDelta => "Spot Delta",
            Self::ForwardDelta => "Forward Delta",
            Self::PremiumAdjusted => "Premium-Adjusted Delta",
        }
    }

    /// Get a description of this delta type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::SpotDelta => "Premium excluded, standard G10 convention",
            Self::ForwardDelta => "Premium excluded, measured vs forward",
            Self::PremiumAdjusted => "Premium included, common in EM markets",
        }
    }
}

// =============================================================================
// FX Quote Data Structures (Req 1.6)
// =============================================================================

/// FX volatility quote entry for a single expiry.
///
/// Contains ATM vol and delta-based quotes (Risk Reversal and Butterfly)
/// for 25-delta and optionally 10-delta points.
///
/// # Requirements Coverage
///
/// - Requirement 1.6: atm_vol, rr_25d, bf_25d, rr_10d, bf_10d fields
/// - Requirement 10.2: ATM vol、25D/10D Risk Reversal、Butterfly入力
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxQuoteEntry {
    /// Time to expiry in years
    pub expiry: f64,
    /// ATM volatility (e.g., 0.10 for 10%)
    pub atm_vol: f64,
    /// 25-delta Risk Reversal (Call - Put vol)
    pub rr_25d: f64,
    /// 25-delta Butterfly (average wing - ATM vol)
    pub bf_25d: f64,
    /// 10-delta Risk Reversal (optional for short expiries)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rr_10d: Option<f64>,
    /// 10-delta Butterfly (optional for short expiries)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bf_10d: Option<f64>,
}

impl FxQuoteEntry {
    /// Create a new FX quote entry with 25-delta quotes only.
    pub fn new(expiry: f64, atm_vol: f64, rr_25d: f64, bf_25d: f64) -> Self {
        Self {
            expiry,
            atm_vol,
            rr_25d,
            bf_25d,
            rr_10d: None,
            bf_10d: None,
        }
    }

    /// Add 10-delta quotes.
    pub fn with_10d(mut self, rr_10d: f64, bf_10d: f64) -> Self {
        self.rr_10d = Some(rr_10d);
        self.bf_10d = Some(bf_10d);
        self
    }

    /// Convert RR/BF to 5-point delta vols.
    ///
    /// Returns (10D Put, 25D Put, ATM, 25D Call, 10D Call) vols.
    /// For quotes without 10D, returns (None, 25D Put, ATM, 25D Call, None).
    pub fn to_delta_vols(&self) -> DeltaVols {
        // ATM vol
        let atm = self.atm_vol;

        // 25D vols from RR and BF:
        // BF = (σ_25c + σ_25p) / 2 - ATM
        // RR = σ_25c - σ_25p
        // Therefore:
        // σ_25c = ATM + BF + RR/2
        // σ_25p = ATM + BF - RR/2
        let vol_25d_call = atm + self.bf_25d + self.rr_25d / 2.0;
        let vol_25d_put = atm + self.bf_25d - self.rr_25d / 2.0;

        // 10D vols (if available)
        let (vol_10d_call, vol_10d_put) = match (self.rr_10d, self.bf_10d) {
            (Some(rr), Some(bf)) => {
                let vol_10d_call = atm + bf + rr / 2.0;
                let vol_10d_put = atm + bf - rr / 2.0;
                (Some(vol_10d_call), Some(vol_10d_put))
            }
            _ => (None, None),
        };

        DeltaVols {
            vol_10d_put,
            vol_25d_put,
            atm,
            vol_25d_call,
            vol_10d_call,
        }
    }
}

/// 5-point delta volatilities computed from RR/BF quotes.
///
/// # Requirements Coverage
///
/// - Requirement 10.3: RR/BFから5点Delta volへの自動変換
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaVols {
    /// 10-delta put volatility (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_10d_put: Option<f64>,
    /// 25-delta put volatility
    pub vol_25d_put: f64,
    /// ATM volatility
    pub atm: f64,
    /// 25-delta call volatility
    pub vol_25d_call: f64,
    /// 10-delta call volatility (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_10d_call: Option<f64>,
}

/// Complete FX volatility data file structure.
///
/// JSON schema for files in `demo/data/input/volsurface/`.
///
/// # Requirements Coverage
///
/// - Requirement 1.6: currency_pair, spot, domestic_rate, foreign_rate, quotes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolFile {
    /// Currency pair (e.g., "EURUSD")
    pub currency_pair: String,
    /// Reference date (ISO 8601 format)
    pub reference_date: String,
    /// Spot FX rate (e.g., 1.0850 for EURUSD)
    pub spot: f64,
    /// Domestic interest rate (continuously compounded)
    pub domestic_rate: f64,
    /// Foreign interest rate (continuously compounded)
    pub foreign_rate: f64,
    /// List of quote entries by expiry
    pub quotes: Vec<FxQuoteEntry>,
}

// =============================================================================
// API Request Types (Req 11.1-11.4, 11.8)
// =============================================================================

/// Response for `GET /api/fxvol/pairs`.
///
/// # Requirements Coverage
///
/// - Requirement 11.1: 利用可能な通貨ペア一覧
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxVolPairsResponse {
    /// List of available currency pairs
    pub pairs: Vec<FxPairInfo>,
}

/// Information about an FX currency pair.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxPairInfo {
    /// Currency pair code (e.g., "EURUSD")
    pub pair: String,
    /// Display name (e.g., "EUR/USD")
    pub name: String,
    /// Base currency (e.g., "EUR")
    pub base: String,
    /// Quote currency (e.g., "USD")
    pub quote: String,
    /// Number of decimal places
    pub decimals: u8,
}

impl FxPairInfo {
    /// Create a new FX pair info.
    pub fn new(pair: &str) -> Self {
        let (base, quote) = if pair.len() == 6 {
            (&pair[0..3], &pair[3..6])
        } else {
            (pair, "")
        };

        Self {
            pair: pair.to_string(),
            name: format!("{}/{}", base, quote),
            base: base.to_string(),
            quote: quote.to_string(),
            decimals: if pair.contains("JPY") { 2 } else { 4 },
        }
    }
}

/// Response for `GET /api/fxvol/quotes/{pair}`.
///
/// # Requirements Coverage
///
/// - Requirement 11.2: 指定通貨ペアのボラティリティQuotesを返す
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxQuotesResponse {
    /// Currency pair
    pub currency_pair: String,
    /// Reference date
    pub reference_date: String,
    /// Spot FX rate
    pub spot: f64,
    /// Domestic interest rate
    pub domestic_rate: f64,
    /// Foreign interest rate
    pub foreign_rate: f64,
    /// List of quote entries
    pub quotes: Vec<FxQuoteEntry>,
}

/// Request for `PUT /api/fxvol/quotes/{pair}`.
///
/// # Requirements Coverage
///
/// - Requirement 11.3: Quotesデータを更新・保存
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxQuotesRequest {
    /// Reference date
    pub reference_date: String,
    /// Spot FX rate
    pub spot: f64,
    /// Domestic interest rate
    pub domestic_rate: f64,
    /// Foreign interest rate
    pub foreign_rate: f64,
    /// List of quote entries
    pub quotes: Vec<FxQuoteEntry>,
}

/// Request for `POST /api/fxvol/build`.
///
/// # Requirements Coverage
///
/// - Requirement 11.4: FxVolatilitySurfaceを構築
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxSurfaceBuildRequest {
    /// Currency pair
    pub currency_pair: String,
    /// Reference date
    pub reference_date: String,
    /// Spot FX rate
    pub spot: f64,
    /// Domestic interest rate
    pub domestic_rate: f64,
    /// Foreign interest rate
    pub foreign_rate: f64,
    /// Quote entries
    pub quotes: Vec<FxQuoteEntry>,
    /// Allow extrapolation beyond data range
    #[serde(default = "default_true")]
    pub allow_extrapolation: bool,
}

fn default_true() -> bool { true }

/// Response for `POST /api/fxvol/build`.
///
/// # Requirements Coverage
///
/// - Requirement 11.4: 構築結果を返す
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxSurfaceBuildResponse {
    /// Unique surface identifier
    pub surface_id: String,
    /// Currency pair
    pub currency_pair: String,
    /// Available delta points
    pub delta_points: Vec<f64>,
    /// Available expiry points
    pub expiry_points: Vec<f64>,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

// =============================================================================
// Delta-Strike Conversion (Req 10.6, 11.8)
// =============================================================================

/// Request for `POST /api/fxvol/delta-strike`.
///
/// # Requirements Coverage
///
/// - Requirement 10.6: Delta-Strike変換
/// - Requirement 11.8: Delta-Strike変換結果を返す
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaStrikeRequest {
    /// Spot FX rate
    pub spot: f64,
    /// Domestic interest rate
    pub domestic_rate: f64,
    /// Foreign interest rate
    pub foreign_rate: f64,
    /// Time to expiry in years
    pub expiry: f64,
    /// Implied volatility
    pub volatility: f64,
    /// Delta values to convert (positive for calls, negative for puts)
    pub deltas: Vec<f64>,
    /// Delta convention to use
    #[serde(default)]
    pub delta_type: DeltaType,
}

/// Response for `POST /api/fxvol/delta-strike`.
///
/// Contains the strike prices corresponding to the input deltas.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaStrikeResponse {
    /// Input deltas
    pub deltas: Vec<f64>,
    /// Computed strike prices
    pub strikes: Vec<f64>,
    /// Forward price used
    pub forward: f64,
    /// Delta type used
    pub delta_type: DeltaType,
}

// =============================================================================
// Smile Query Types (Req 10.1, 11.5)
// =============================================================================

/// Query parameters for `GET /api/fxvol/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 10.1: Delta軸でスマイルを表示
/// - Requirement 11.5: 指定ExpiryのDelta-Volスマイルデータ
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FxSmileQuery {
    /// Surface ID from build
    pub surface_id: String,
    /// Time to expiry in years
    pub expiry: f64,
    /// Number of delta points (default 21)
    #[serde(default = "default_smile_points")]
    pub num_points: usize,
}

fn default_smile_points() -> usize { 21 }

/// Delta point for FX smile visualisation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxDeltaPoint {
    /// Delta value (e.g., 0.25 for 25D call, -0.25 for 25D put)
    pub delta: f64,
    /// Delta label (e.g., "25D Call", "ATM", "25D Put")
    pub label: String,
    /// Implied volatility at this delta
    pub volatility: f64,
    /// Corresponding absolute strike
    pub strike: f64,
}

/// Response for `GET /api/fxvol/smile`.
///
/// # Requirements Coverage
///
/// - Requirement 10.1: Delta軸（10D Put、25D Put、ATM、25D Call、10D Call）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxSmileResponse {
    /// Time to expiry
    pub expiry: f64,
    /// Spot rate
    pub spot: f64,
    /// Forward rate at expiry
    pub forward: f64,
    /// ATM volatility
    pub atm_vol: f64,
    /// Smile data points
    pub points: Vec<FxDeltaPoint>,
    /// Risk Reversal (25D)
    pub rr_25d: f64,
    /// Butterfly (25D)
    pub bf_25d: f64,
    /// Risk Reversal (10D) if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rr_10d: Option<f64>,
    /// Butterfly (10D) if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bf_10d: Option<f64>,
}

// =============================================================================
// RR/BF Time Series (Req 10.4, 11.6)
// =============================================================================

/// Query parameters for `GET /api/fxvol/rr-bf`.
///
/// # Requirements Coverage
///
/// - Requirement 10.4: RR/BF時系列チャート
/// - Requirement 11.6: Risk Reversal/Butterflyの時系列データ
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RrBfQuery {
    /// Surface ID from build
    pub surface_id: String,
}

/// Single data point for RR/BF time series.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RrBfDataPoint {
    /// Expiry in years
    pub expiry: f64,
    /// Expiry label (e.g., "1M", "3M", "1Y")
    pub label: String,
    /// ATM volatility
    pub atm_vol: f64,
    /// 25D Risk Reversal
    pub rr_25d: f64,
    /// 25D Butterfly
    pub bf_25d: f64,
    /// 10D Risk Reversal (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rr_10d: Option<f64>,
    /// 10D Butterfly (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bf_10d: Option<f64>,
}

/// Response for `GET /api/fxvol/rr-bf`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RrBfResponse {
    /// Currency pair
    pub currency_pair: String,
    /// Data points by expiry (term structure)
    pub data: Vec<RrBfDataPoint>,
}

// =============================================================================
// FX Density Types (Req 10.7, 10.8, 11.7)
// =============================================================================

/// Query parameters for `GET /api/fxvol/density`.
///
/// # Requirements Coverage
///
/// - Requirement 10.7: FX確率密度関数を計算・表示
/// - Requirement 11.7: 確率密度データを返す
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FxDensityQuery {
    /// Surface ID from build
    pub surface_id: String,
    /// Time to expiry in years
    pub expiry: f64,
    /// Number of strike points (default 100)
    #[serde(default = "default_density_points")]
    pub num_points: usize,
}

fn default_density_points() -> usize { 100 }

/// Density statistics for FX distribution.
///
/// # Requirements Coverage
///
/// - Requirement 10.8: 統計情報（期待値、分散、歪度、尖度）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxDensityStatistics {
    /// Expected FX rate
    pub mean: f64,
    /// Variance of FX rate
    pub variance: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Skewness (third standardised moment)
    pub skewness: f64,
    /// Excess kurtosis (normal = 0)
    pub kurtosis: f64,
}

impl Default for FxDensityStatistics {
    fn default() -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            std_dev: 0.0,
            skewness: 0.0,
            kurtosis: 0.0,
        }
    }
}

/// Response for `GET /api/fxvol/density`.
///
/// # Requirements Coverage
///
/// - Requirement 10.7: Breeden-Litzenberger法による確率密度関数
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxDensityResponse {
    /// Time to expiry
    pub expiry: f64,
    /// Spot rate
    pub spot: f64,
    /// Forward rate
    pub forward: f64,
    /// Strike values
    pub strikes: Vec<f64>,
    /// Probability density values
    pub densities: Vec<f64>,
    /// Cumulative distribution function values
    pub cdf: Vec<f64>,
    /// Distribution statistics
    pub statistics: FxDensityStatistics,
    /// Warnings about numerical issues
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

// =============================================================================
// Delta Types List Response
// =============================================================================

/// Response for delta type options.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxDeltaTypesResponse {
    /// Available delta types
    pub delta_types: Vec<DeltaTypeInfo>,
}

/// Information about a delta type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaTypeInfo {
    /// Type identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
}

impl FxDeltaTypesResponse {
    /// Create a new delta types response with all available options.
    pub fn new() -> Self {
        Self {
            delta_types: vec![
                DeltaTypeInfo {
                    id: "spot_delta".to_string(),
                    name: DeltaType::SpotDelta.display_name().to_string(),
                    description: DeltaType::SpotDelta.description().to_string(),
                },
                DeltaTypeInfo {
                    id: "forward_delta".to_string(),
                    name: DeltaType::ForwardDelta.display_name().to_string(),
                    description: DeltaType::ForwardDelta.description().to_string(),
                },
                DeltaTypeInfo {
                    id: "premium_adjusted".to_string(),
                    name: DeltaType::PremiumAdjusted.display_name().to_string(),
                    description: DeltaType::PremiumAdjusted.description().to_string(),
                },
            ],
        }
    }
}

impl Default for FxDeltaTypesResponse {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DeltaType Tests
    // =========================================================================

    mod delta_type_tests {
        use super::*;

        #[test]
        fn test_delta_type_default() {
            let dt = DeltaType::default();
            assert_eq!(dt, DeltaType::SpotDelta);
        }

        #[test]
        fn test_delta_type_serde() {
            let dt = DeltaType::ForwardDelta;
            let json = serde_json::to_string(&dt).unwrap();
            assert_eq!(json, "\"forward_delta\"");

            let parsed: DeltaType = serde_json::from_str("\"spot_delta\"").unwrap();
            assert_eq!(parsed, DeltaType::SpotDelta);

            let parsed: DeltaType = serde_json::from_str("\"premium_adjusted\"").unwrap();
            assert_eq!(parsed, DeltaType::PremiumAdjusted);
        }

        #[test]
        fn test_delta_type_display_name() {
            assert_eq!(DeltaType::SpotDelta.display_name(), "Spot Delta");
            assert_eq!(DeltaType::ForwardDelta.display_name(), "Forward Delta");
            assert_eq!(
                DeltaType::PremiumAdjusted.display_name(),
                "Premium-Adjusted Delta"
            );
        }
    }

    // =========================================================================
    // FxQuoteEntry Tests
    // =========================================================================

    mod fx_quote_entry_tests {
        use super::*;

        #[test]
        fn test_fx_quote_entry_new() {
            let quote = FxQuoteEntry::new(0.25, 0.10, -0.005, 0.003);
            assert_eq!(quote.expiry, 0.25);
            assert_eq!(quote.atm_vol, 0.10);
            assert_eq!(quote.rr_25d, -0.005);
            assert_eq!(quote.bf_25d, 0.003);
            assert!(quote.rr_10d.is_none());
            assert!(quote.bf_10d.is_none());
        }

        #[test]
        fn test_fx_quote_entry_with_10d() {
            let quote = FxQuoteEntry::new(0.25, 0.10, -0.005, 0.003).with_10d(-0.012, 0.008);
            assert_eq!(quote.rr_10d, Some(-0.012));
            assert_eq!(quote.bf_10d, Some(0.008));
        }

        #[test]
        fn test_fx_quote_entry_to_delta_vols_25d_only() {
            // ATM = 10%, RR25 = -0.5%, BF25 = 0.3%
            let quote = FxQuoteEntry::new(0.25, 0.10, -0.005, 0.003);
            let vols = quote.to_delta_vols();

            assert_eq!(vols.atm, 0.10);
            // 25D Call = 0.10 + 0.003 - 0.005/2 = 0.1005
            // 25D Put = 0.10 + 0.003 + 0.005/2 = 0.1055
            assert!((vols.vol_25d_call - 0.1005).abs() < 1e-10);
            assert!((vols.vol_25d_put - 0.1055).abs() < 1e-10);
            assert!(vols.vol_10d_call.is_none());
            assert!(vols.vol_10d_put.is_none());
        }

        #[test]
        fn test_fx_quote_entry_to_delta_vols_with_10d() {
            let quote =
                FxQuoteEntry::new(0.25, 0.10, -0.005, 0.003).with_10d(-0.012, 0.008);
            let vols = quote.to_delta_vols();

            assert!(vols.vol_10d_call.is_some());
            assert!(vols.vol_10d_put.is_some());
            // 10D Call = 0.10 + 0.008 - 0.012/2 = 0.102
            // 10D Put = 0.10 + 0.008 + 0.012/2 = 0.114
            assert!((vols.vol_10d_call.unwrap() - 0.102).abs() < 1e-10);
            assert!((vols.vol_10d_put.unwrap() - 0.114).abs() < 1e-10);
        }

        #[test]
        fn test_fx_quote_entry_serde() {
            let quote =
                FxQuoteEntry::new(0.25, 0.10, -0.005, 0.003).with_10d(-0.012, 0.008);
            let json = serde_json::to_string(&quote).unwrap();

            assert!(json.contains("\"expiry\":0.25"));
            assert!(json.contains("\"atmVol\":0.1"));
            assert!(json.contains("\"rr25d\":-0.005"));
            assert!(json.contains("\"bf25d\":0.003"));
            assert!(json.contains("\"rr10d\":-0.012"));
            assert!(json.contains("\"bf10d\":0.008"));

            let parsed: FxQuoteEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.expiry, 0.25);
        }

        #[test]
        fn test_fx_quote_entry_deserialise_optional_10d() {
            let json = r#"{
                "expiry": 0.25,
                "atmVol": 0.10,
                "rr25d": -0.005,
                "bf25d": 0.003
            }"#;

            let quote: FxQuoteEntry = serde_json::from_str(json).unwrap();
            assert!(quote.rr_10d.is_none());
            assert!(quote.bf_10d.is_none());
        }
    }

    // =========================================================================
    // FxVolFile Tests
    // =========================================================================

    mod fxvol_file_tests {
        use super::*;

        #[test]
        fn test_fxvol_file_deserialise() {
            let json = r#"{
                "currencyPair": "EURUSD",
                "referenceDate": "2026-01-23",
                "spot": 1.0850,
                "domesticRate": 0.045,
                "foreignRate": 0.035,
                "quotes": [
                    {
                        "expiry": 0.25,
                        "atmVol": 0.085,
                        "rr25d": -0.005,
                        "bf25d": 0.003
                    }
                ]
            }"#;

            let file: FxVolFile = serde_json::from_str(json).unwrap();
            assert_eq!(file.currency_pair, "EURUSD");
            assert_eq!(file.spot, 1.0850);
            assert_eq!(file.domestic_rate, 0.045);
            assert_eq!(file.foreign_rate, 0.035);
            assert_eq!(file.quotes.len(), 1);
        }
    }

    // =========================================================================
    // FxPairInfo Tests
    // =========================================================================

    mod fx_pair_info_tests {
        use super::*;

        #[test]
        fn test_fx_pair_info_new() {
            let pair = FxPairInfo::new("EURUSD");
            assert_eq!(pair.pair, "EURUSD");
            assert_eq!(pair.name, "EUR/USD");
            assert_eq!(pair.base, "EUR");
            assert_eq!(pair.quote, "USD");
            assert_eq!(pair.decimals, 4);
        }

        #[test]
        fn test_fx_pair_info_jpy_decimals() {
            let pair = FxPairInfo::new("USDJPY");
            assert_eq!(pair.decimals, 2);
        }
    }

    // =========================================================================
    // Request/Response Tests
    // =========================================================================

    mod request_response_tests {
        use super::*;

        #[test]
        fn test_fx_quotes_response_serialise() {
            let response = FxQuotesResponse {
                currency_pair: "EURUSD".to_string(),
                reference_date: "2026-01-23".to_string(),
                spot: 1.0850,
                domestic_rate: 0.045,
                foreign_rate: 0.035,
                quotes: vec![FxQuoteEntry::new(0.25, 0.085, -0.005, 0.003)],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"currencyPair\":\"EURUSD\""));
            assert!(json.contains("\"spot\":1.085"));
            assert!(json.contains("\"domesticRate\":0.045"));
        }

        #[test]
        fn test_delta_strike_request_deserialise() {
            let json = r#"{
                "spot": 1.085,
                "domesticRate": 0.045,
                "foreignRate": 0.035,
                "expiry": 0.5,
                "volatility": 0.10,
                "deltas": [0.25, -0.25]
            }"#;

            let req: DeltaStrikeRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.spot, 1.085);
            assert_eq!(req.delta_type, DeltaType::SpotDelta); // default
            assert_eq!(req.deltas, vec![0.25, -0.25]);
        }

        #[test]
        fn test_delta_strike_response_serialise() {
            let response = DeltaStrikeResponse {
                deltas: vec![0.25, -0.25],
                strikes: vec![1.12, 1.05],
                forward: 1.0855,
                delta_type: DeltaType::SpotDelta,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"deltas\""));
            assert!(json.contains("\"strikes\""));
            assert!(json.contains("\"forward\":1.0855"));
            assert!(json.contains("\"deltaType\":\"spot_delta\""));
        }
    }

    // =========================================================================
    // Smile Query Tests
    // =========================================================================

    mod smile_query_tests {
        use super::*;

        #[test]
        fn test_fx_smile_query_defaults() {
            let json = r#"{
                "surface_id": "abc",
                "expiry": 0.5
            }"#;

            let query: FxSmileQuery = serde_json::from_str(json).unwrap();
            assert_eq!(query.num_points, 21);
        }

        #[test]
        fn test_fx_smile_response_serialise() {
            let response = FxSmileResponse {
                expiry: 0.5,
                spot: 1.085,
                forward: 1.0855,
                atm_vol: 0.10,
                points: vec![FxDeltaPoint {
                    delta: 0.0,
                    label: "ATM".to_string(),
                    volatility: 0.10,
                    strike: 1.0855,
                }],
                rr_25d: -0.005,
                bf_25d: 0.003,
                rr_10d: Some(-0.012),
                bf_10d: Some(0.008),
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"atmVol\":0.1"));
            assert!(json.contains("\"rr25d\":-0.005"));
            assert!(json.contains("\"rr10d\":-0.012"));
        }

        #[test]
        fn test_fx_smile_response_without_10d() {
            let response = FxSmileResponse {
                expiry: 0.5,
                spot: 1.085,
                forward: 1.0855,
                atm_vol: 0.10,
                points: vec![],
                rr_25d: -0.005,
                bf_25d: 0.003,
                rr_10d: None,
                bf_10d: None,
            };

            let json = serde_json::to_string(&response).unwrap();
            // rr10d and bf10d should be skipped
            assert!(!json.contains("rr10d"));
            assert!(!json.contains("bf10d"));
        }
    }

    // =========================================================================
    // RR/BF Tests
    // =========================================================================

    mod rr_bf_tests {
        use super::*;

        #[test]
        fn test_rr_bf_response_serialise() {
            let response = RrBfResponse {
                currency_pair: "EURUSD".to_string(),
                data: vec![RrBfDataPoint {
                    expiry: 0.25,
                    label: "3M".to_string(),
                    atm_vol: 0.085,
                    rr_25d: -0.005,
                    bf_25d: 0.003,
                    rr_10d: None,
                    bf_10d: None,
                }],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"currencyPair\":\"EURUSD\""));
            assert!(json.contains("\"label\":\"3M\""));
        }
    }

    // =========================================================================
    // Density Tests
    // =========================================================================

    mod density_tests {
        use super::*;

        #[test]
        fn test_fx_density_query_defaults() {
            let json = r#"{
                "surface_id": "abc",
                "expiry": 0.5
            }"#;

            let query: FxDensityQuery = serde_json::from_str(json).unwrap();
            assert_eq!(query.num_points, 100);
        }

        #[test]
        fn test_fx_density_response_serialise() {
            let response = FxDensityResponse {
                expiry: 0.5,
                spot: 1.085,
                forward: 1.0855,
                strikes: vec![1.0, 1.05, 1.10],
                densities: vec![0.1, 0.5, 0.2],
                cdf: vec![0.1, 0.6, 0.8],
                statistics: FxDensityStatistics {
                    mean: 1.0855,
                    variance: 0.001,
                    std_dev: 0.032,
                    skewness: -0.1,
                    kurtosis: 0.2,
                },
                warnings: vec![],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"densities\""));
            assert!(json.contains("\"cdf\""));
            assert!(json.contains("\"statistics\""));
            assert!(json.contains("\"stdDev\":0.032"));
            assert!(!json.contains("\"warnings\"")); // empty, should be skipped
        }

        #[test]
        fn test_fx_density_response_with_warnings() {
            let response = FxDensityResponse {
                expiry: 0.5,
                spot: 1.085,
                forward: 1.0855,
                strikes: vec![],
                densities: vec![],
                cdf: vec![],
                statistics: FxDensityStatistics::default(),
                warnings: vec!["Extrapolation at far strikes".to_string()],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"warnings\""));
            assert!(json.contains("Extrapolation"));
        }
    }

    // =========================================================================
    // Delta Types Response Tests
    // =========================================================================

    mod delta_types_response_tests {
        use super::*;

        #[test]
        fn test_delta_types_response_new() {
            let response = FxDeltaTypesResponse::new();
            assert_eq!(response.delta_types.len(), 3);

            let spot = response
                .delta_types
                .iter()
                .find(|d| d.id == "spot_delta")
                .unwrap();
            assert_eq!(spot.name, "Spot Delta");
        }

        #[test]
        fn test_delta_types_response_serialise() {
            let response = FxDeltaTypesResponse::new();
            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"deltaTypes\""));
            assert!(json.contains("\"spot_delta\""));
        }
    }
}

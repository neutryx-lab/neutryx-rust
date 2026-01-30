//! Volatility-related DTOs for Vol Surface/Cube operations
//!
//! Request/Response types for `VolatilityService` endpoints.

use serde::{Deserialize, Serialize};

// ============================================================================
// Common Vol Types
// ============================================================================

/// Volatility quote for surface construction
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolQuoteDto {
    /// Expiry in years
    pub expiry: f64,
    /// Delta (for FX: 25D, ATM, 75D) or moneyness
    pub delta_or_strike: f64,
    /// Quote type
    pub quote_type: VolQuoteTypeDto,
    /// Implied volatility (as decimal, e.g., 0.15 for 15%)
    pub vol: f64,
}

/// Type of volatility quote
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VolQuoteTypeDto {
    /// ATM volatility
    #[default]
    Atm,
    /// Risk reversal (25D or 10D)
    RiskReversal,
    /// Butterfly/Strangle
    Butterfly,
    /// Absolute strike quote
    Strike,
    /// Delta quote (e.g., 25D put)
    Delta,
}

/// SABR calibration result for a single expiry
#[derive(Debug, Clone, Serialize)]
pub struct SabrCalibrationDto {
    /// Expiry in years
    pub expiry: f64,
    /// Alpha parameter
    pub alpha: f64,
    /// Beta parameter (usually fixed)
    pub beta: f64,
    /// Rho parameter
    pub rho: f64,
    /// Nu parameter (vol-of-vol)
    pub nu: f64,
    /// Calibration residual (squared error)
    pub residual: f64,
}

// ============================================================================
// FX Vol Surface DTOs
// ============================================================================

/// Request to build an FX volatility surface
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct BuildFxVolSurfaceRequest {
    /// Currency pair (e.g., "USDJPY", "EURUSD")
    pub currency_pair: String,
    /// Volatility quotes
    pub quotes: Vec<VolQuoteDto>,
    /// FX spot rate
    pub fx_spot: f64,
    /// Domestic risk-free rate
    pub domestic_rate: f64,
    /// Foreign risk-free rate
    pub foreign_rate: f64,
    /// Beta parameter for SABR (optional, defaults to 0.5)
    #[serde(default = "default_beta")]
    pub beta: f64,
}

fn default_beta() -> f64 { 0.5 }

/// Response for FX vol surface construction
#[derive(Debug, Clone, Serialize)]
pub struct BuildFxVolSurfaceResponse {
    /// Generated surface ID
    pub surface_id: String,
    /// Currency pair
    pub currency_pair: String,
    /// Number of expiry slices
    pub expiry_count: usize,
    /// SABR parameters per expiry
    pub sabr_params: Vec<SabrCalibrationDto>,
    /// Overall calibration quality
    pub calibration_quality: CalibrationQualityDto,
    /// Calibration time in milliseconds
    pub calibration_time_ms: f64,
}

/// Calibration quality metrics
#[derive(Debug, Clone, Serialize)]
pub struct CalibrationQualityDto {
    /// Did calibration converge?
    pub converged: bool,
    /// Total sum of squared residuals
    pub total_residual_ss: f64,
    /// Maximum residual across all slices
    pub max_residual: f64,
    /// Number of iterations (if iterative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<usize>,
}

// ============================================================================
// Vol Cube DTOs
// ============================================================================

/// Request to build a volatility cube (IR swaptions)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct BuildVolCubeRequest {
    /// Index name (e.g., "USD-SOFR", "EUR-ESTR")
    pub index: String,
    /// Expiry tenors (e.g., ["1Y", "2Y", "5Y"])
    pub expiries: Vec<String>,
    /// Swap tenors (e.g., ["1Y", "2Y", "5Y", "10Y"])
    pub tenors: Vec<String>,
    /// ATM volatilities (expiry x tenor matrix)
    pub atm_vols: Vec<Vec<f64>>,
    /// Smile quotes (optional)
    #[serde(default)]
    pub smile_quotes: Option<VolCubeSmileQuotes>,
    /// Beta parameter for SABR
    #[serde(default = "default_beta")]
    pub beta: f64,
}

/// Smile quotes for vol cube calibration
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct VolCubeSmileQuotes {
    /// Strike offsets from ATM (e.g., [-200bp, -100bp, +100bp, +200bp])
    pub strike_offsets_bp: Vec<f64>,
    /// Volatilities at each offset (expiry x tenor x offset)
    pub vols: Vec<Vec<Vec<f64>>>,
}

/// Response for vol cube construction
#[derive(Debug, Clone, Serialize)]
pub struct BuildVolCubeResponse {
    /// Generated cube ID
    pub cube_id: String,
    /// Index name
    pub index: String,
    /// Number of expiry slices
    pub expiry_count: usize,
    /// Number of tenor slices
    pub tenor_count: usize,
    /// SABR parameters (flattened or per slice)
    pub sabr_params: Vec<SabrCalibrationDto>,
    /// Calibration quality
    pub calibration_quality: CalibrationQualityDto,
    /// Calibration time in milliseconds
    pub calibration_time_ms: f64,
}

// ============================================================================
// Implied Vol Query DTOs
// ============================================================================

/// Request to get implied volatility
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct GetImpliedVolRequest {
    /// Expiry in years
    pub expiry: f64,
    /// Strike (absolute or as forward moneyness)
    pub strike: f64,
    /// Strike type
    #[serde(default)]
    pub strike_type: StrikeTypeDto,
    /// Forward price (optional, for moneyness conversion)
    #[serde(default)]
    pub forward: Option<f64>,
}

/// Strike type for implied vol queries
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StrikeTypeDto {
    /// Absolute strike price
    #[default]
    Absolute,
    /// Forward moneyness (K/F)
    Moneyness,
    /// Log moneyness (ln(K/F))
    LogMoneyness,
    /// Delta (e.g., 0.25 for 25D)
    Delta,
}

/// Response for implied volatility query
#[derive(Debug, Clone, Serialize)]
pub struct GetImpliedVolResponse {
    /// Surface/cube ID
    pub surface_id: String,
    /// Expiry in years
    pub expiry: f64,
    /// Strike queried
    pub strike: f64,
    /// Interpolated implied volatility
    pub implied_vol: f64,
    /// SABR parameters at this expiry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sabr_params: Option<SabrCalibrationDto>,
}

// ============================================================================
// Vol Surface Management DTOs
// ============================================================================

/// Response for vol surface listing
#[derive(Debug, Clone, Serialize)]
pub struct VolSurfaceInfoDto {
    /// Surface ID
    pub surface_id: String,
    /// Surface type (`fx_surface`, `ir_cube`, `equity_surface`)
    pub surface_type: String,
    /// Underlying identifier
    pub underlying: String,
    /// Number of expiry slices
    pub expiry_count: usize,
    /// Creation timestamp
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_quote_dto() {
        let json = r#"{
            "expiry": 0.25,
            "delta_or_strike": 0.25,
            "quote_type": "delta",
            "vol": 0.12
        }"#;
        let quote: VolQuoteDto = serde_json::from_str(json).unwrap();
        assert!((quote.expiry - 0.25).abs() < f64::EPSILON);
        assert!((quote.vol - 0.12).abs() < f64::EPSILON);
        assert!(matches!(quote.quote_type, VolQuoteTypeDto::Delta));
    }

    #[test]
    fn test_build_fx_vol_surface_request() {
        let json = r#"{
            "currency_pair": "USDJPY",
            "quotes": [
                {"expiry": 0.25, "delta_or_strike": 0.0, "quote_type": "atm", "vol": 0.10}
            ],
            "fx_spot": 150.0,
            "domestic_rate": 0.05,
            "foreign_rate": 0.01
        }"#;
        let request: BuildFxVolSurfaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.currency_pair, "USDJPY");
        assert!((request.fx_spot - 150.0).abs() < f64::EPSILON);
        assert!((request.beta - 0.5).abs() < f64::EPSILON); // default
        assert_eq!(request.quotes.len(), 1);
    }

    #[test]
    fn test_get_implied_vol_request_defaults() {
        let json = r#"{
            "expiry": 1.0,
            "strike": 100.0
        }"#;
        let request: GetImpliedVolRequest = serde_json::from_str(json).unwrap();
        assert!((request.expiry - 1.0).abs() < f64::EPSILON);
        assert!((request.strike - 100.0).abs() < f64::EPSILON);
        assert!(matches!(request.strike_type, StrikeTypeDto::Absolute));
        assert!(request.forward.is_none());
    }

    #[test]
    fn test_build_fx_vol_surface_response_serialisation() {
        let response = BuildFxVolSurfaceResponse {
            surface_id: "VS123".to_string(),
            currency_pair: "USDJPY".to_string(),
            expiry_count: 5,
            sabr_params: vec![SabrCalibrationDto {
                expiry: 0.25,
                alpha: 0.2,
                beta: 0.5,
                rho: -0.1,
                nu: 0.3,
                residual: 1e-6,
            }],
            calibration_quality: CalibrationQualityDto {
                converged: true,
                total_residual_ss: 1e-6,
                max_residual: 1e-7,
                iterations: Some(10),
            },
            calibration_time_ms: 50.0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("surface_id"));
        assert!(json.contains("USDJPY"));
        assert!(json.contains("converged"));
    }
}

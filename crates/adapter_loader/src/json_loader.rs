//! JSON file loaders for trade, market, and CSA data.
//!
//! This module provides:
//! - [`JsonLoader`]: Generic JSON loading with glob support
//! - [`TradeLoader`]: Trade data loading from JSON
//! - [`MarketLoader`]: Market data (curves, vol surfaces, FX) loading
//! - [`CsaLoader`]: CSA terms loading from JSON
//!
//! # Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Converts external JSON data into `infra_master` domain types.
//!
//! # Example
//!
//! ```rust,ignore
//! use adapter_loader::{JsonLoader, TradeLoader};
//!
//! // Generic loading
//! let data: MyStruct = JsonLoader::load("config.json")?;
//!
//! // Trade loading
//! let trades = TradeLoader::load_portfolio("trades.json")?;
//! ```

use std::{collections::HashMap, path::Path};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::LoaderError;

// =============================================================================
// JsonLoader - Generic JSON loading
// =============================================================================

/// Generic JSON file loader with glob pattern support.
///
/// Provides utility methods for loading JSON files into any type
/// that implements `DeserializeOwned`.
pub struct JsonLoader;

impl JsonLoader {
    /// Load a single JSON file into type `T`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Returns
    ///
    /// Deserialized instance of type `T`, or error if file not found or parsing
    /// fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use adapter_loader::JsonLoader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Config { name: String }
    ///
    /// let config: Config = JsonLoader::load("config.json")?;
    /// ```
    pub fn load<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, LoaderError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        if !path.exists() {
            return Err(LoaderError::file_not_found(&path_str));
        }

        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| LoaderError::json_error(&path_str, &e))
    }

    /// Load multiple JSON files matching a glob pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "data/*.json")
    ///
    /// # Returns
    ///
    /// Vector of (path, deserialized value) pairs for all matching files.
    /// Files that fail to parse are returned as errors in the result vector.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use adapter_loader::JsonLoader;
    ///
    /// let results: Vec<Result<(String, Trade), LoaderError>> =
    ///     JsonLoader::load_glob("trades/*.json")?;
    /// ```
    pub fn load_glob<T: DeserializeOwned>(
        pattern: &str,
    ) -> Result<Vec<Result<(String, T), LoaderError>>, LoaderError> {
        let paths = glob::glob(pattern)
            .map_err(|e| LoaderError::glob_pattern_error(pattern, e.to_string()))?;

        let results: Vec<Result<(String, T), LoaderError>> = paths
            .filter_map(|entry| entry.ok())
            .map(|path| {
                let path_str = path.display().to_string();
                match Self::load::<T, _>(&path) {
                    Ok(value) => Ok((path_str, value)),
                    Err(e) => Err(e),
                }
            })
            .collect();

        Ok(results)
    }

    /// Load all JSON files matching a glob pattern, collecting successful
    /// results.
    ///
    /// Files that fail to parse are skipped with a warning logged.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "data/*.json")
    ///
    /// # Returns
    ///
    /// Vector of successfully loaded values.
    pub fn load_glob_ok<T: DeserializeOwned>(pattern: &str) -> Result<Vec<T>, LoaderError> {
        let results = Self::load_glob::<T>(pattern)?;
        Ok(results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(|(_, v)| v)
            .collect())
    }
}

// =============================================================================
// TradeLoader - Trade data loading
// =============================================================================

/// Trade data loader for JSON format.
///
/// Loads trade data from JSON files into `infra_master::trade::Trade`
/// structures.
pub struct TradeLoader;

impl TradeLoader {
    /// Load a single trade from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the trade JSON file
    ///
    /// # Returns
    ///
    /// Deserialized `Trade` instance.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<infra_master::trade::Trade, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load a portfolio (array of trades) from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file containing an array of trades
    ///
    /// # Returns
    ///
    /// Vector of `Trade` instances.
    pub fn load_portfolio<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<infra_master::trade::Trade>, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load multiple trades from files matching a glob pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "trades/*.json")
    ///
    /// # Returns
    ///
    /// Vector of successfully loaded trades.
    pub fn load_glob(pattern: &str) -> Result<Vec<infra_master::trade::Trade>, LoaderError> {
        JsonLoader::load_glob_ok(pattern)
    }
}

// =============================================================================
// Market Data Types
// =============================================================================

/// Curve point data for yield curve construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    /// Tenor in years (e.g., 0.25, 0.5, 1.0, 2.0)
    pub tenor: f64,
    /// Rate value (e.g., 0.05 for 5%)
    pub rate: f64,
}

/// Yield curve data loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveData {
    /// Curve identifier (e.g., "USD_SOFR", "EUR_ESTR")
    pub curve_id: String,
    /// Curve type (e.g., "ois", "sofr", "forward", "discount")
    pub curve_type: String,
    /// Currency code (e.g., "USD", "EUR")
    pub currency: String,
    /// Curve points (tenor, rate pairs)
    pub points: Vec<CurvePoint>,
    /// Interpolation method (e.g., "linear_on_zero_rate", "linear_on_df")
    #[serde(default = "default_interpolation")]
    pub interpolation: String,
}

fn default_interpolation() -> String { "linear_on_zero_rate".to_string() }

/// Volatility surface point data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolPoint {
    /// Expiry in years
    pub expiry: f64,
    /// Strike (e.g., absolute value or moneyness)
    pub strike: f64,
    /// Volatility value (e.g., 0.20 for 20%)
    pub volatility: f64,
}

/// Volatility surface data loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolSurfaceData {
    /// Surface identifier (e.g., "USD_SWAPTION_VOL")
    pub surface_id: String,
    /// Surface type (e.g., "swaption", "capfloor", "fx")
    pub surface_type: String,
    /// Currency code
    pub currency: String,
    /// Volatility points (expiry, strike, vol triples)
    pub points: Vec<VolPoint>,
}

/// FX spot rate data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxSpotData {
    /// Currency pair (e.g., "USDJPY", "EURUSD")
    pub pair: String,
    /// Spot rate
    pub rate: f64,
}

/// Aggregated market data loaded from JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketData {
    /// Yield curves
    #[serde(default)]
    pub curves: Vec<CurveData>,
    /// Volatility surfaces
    #[serde(default)]
    pub vol_surfaces: Vec<VolSurfaceData>,
    /// FX spot rates
    #[serde(default)]
    pub fx_spots: Vec<FxSpotData>,
}

impl MarketData {
    /// Get FX spots as a HashMap for easy lookup.
    #[must_use]
    pub fn fx_spot_map(&self) -> HashMap<String, f64> {
        self.fx_spots
            .iter()
            .map(|fx| (fx.pair.clone(), fx.rate))
            .collect()
    }

    /// Find a curve by its ID.
    #[must_use]
    pub fn get_curve(&self, curve_id: &str) -> Option<&CurveData> {
        self.curves.iter().find(|c| c.curve_id == curve_id)
    }

    /// Find curves by currency.
    pub fn get_curves_by_currency(&self, currency: &str) -> Vec<&CurveData> {
        self.curves
            .iter()
            .filter(|c| c.currency == currency)
            .collect()
    }

    /// Find a volatility surface by its ID.
    #[must_use]
    pub fn get_vol_surface(&self, surface_id: &str) -> Option<&VolSurfaceData> {
        self.vol_surfaces
            .iter()
            .find(|s| s.surface_id == surface_id)
    }
}

// =============================================================================
// MarketLoader - Market data loading
// =============================================================================

/// Market data loader for JSON format.
///
/// Loads market data (curves, volatility surfaces, FX spots) from JSON files.
pub struct MarketLoader;

impl MarketLoader {
    /// Load market data from a single JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the market data JSON file
    ///
    /// # Returns
    ///
    /// `MarketData` containing curves, vol surfaces, and FX spots.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<MarketData, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load and merge market data from multiple JSON files matching a glob
    /// pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "market/*.json")
    ///
    /// # Returns
    ///
    /// Merged `MarketData` from all matching files.
    pub fn load_glob(pattern: &str) -> Result<MarketData, LoaderError> {
        let results = JsonLoader::load_glob::<MarketData>(pattern)?;

        let mut merged = MarketData::default();
        for (_, data) in results.into_iter().flatten() {
            merged.curves.extend(data.curves);
            merged.vol_surfaces.extend(data.vol_surfaces);
            merged.fx_spots.extend(data.fx_spots);
        }

        Ok(merged)
    }

    /// Load only curve data from a JSON file.
    ///
    /// Expects the JSON to be an array of `CurveData` objects.
    pub fn load_curves<P: AsRef<Path>>(path: P) -> Result<Vec<CurveData>, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load only volatility surface data from a JSON file.
    ///
    /// Expects the JSON to be an array of `VolSurfaceData` objects.
    pub fn load_vol_surfaces<P: AsRef<Path>>(path: P) -> Result<Vec<VolSurfaceData>, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load only FX spot data from a JSON file.
    ///
    /// Expects the JSON to be an array of `FxSpotData` objects.
    pub fn load_fx_spots<P: AsRef<Path>>(path: P) -> Result<Vec<FxSpotData>, LoaderError> {
        JsonLoader::load(path)
    }
}

// =============================================================================
// CsaLoader - CSA data loading
// =============================================================================

/// CSA (Credit Support Annex) data loader for JSON format.
///
/// Loads CSA terms from JSON files into `infra_master::counterparty::CsaTerms`.
pub struct CsaLoader;

impl CsaLoader {
    /// Load CSA terms from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSA JSON file
    ///
    /// # Returns
    ///
    /// `CsaTerms` instance.
    pub fn load<P: AsRef<Path>>(
        path: P,
    ) -> Result<infra_master::counterparty::CsaTerms, LoaderError> {
        JsonLoader::load(path)
    }

    /// Load multiple CSA terms from files matching a glob pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "csa/*.json")
    ///
    /// # Returns
    ///
    /// Vector of successfully loaded CSA terms.
    pub fn load_glob(
        pattern: &str,
    ) -> Result<Vec<infra_master::counterparty::CsaTerms>, LoaderError> {
        JsonLoader::load_glob_ok(pattern)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    // -------------------------------------------------------------------------
    // JsonLoader Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_json_loader_file_not_found() {
        let result = JsonLoader::load::<serde_json::Value, _>("nonexistent.json");
        assert!(matches!(result, Err(LoaderError::FileNotFound(_))));
    }

    #[test]
    fn test_json_loader_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{ invalid json }}").unwrap();

        let result = JsonLoader::load::<serde_json::Value, _>(file.path());
        assert!(matches!(result, Err(LoaderError::JsonError { .. })));
    }

    #[test]
    fn test_json_loader_valid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{ "name": "test", "value": 42 }}"#).unwrap();

        #[derive(Deserialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let result: TestData = JsonLoader::load(file.path()).unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.value, 42);
    }

    #[test]
    fn test_json_loader_glob_pattern_error() {
        let result = JsonLoader::load_glob::<serde_json::Value>("[invalid");
        assert!(matches!(result, Err(LoaderError::GlobPatternError { .. })));
    }

    // -------------------------------------------------------------------------
    // MarketData Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_market_data_default() {
        let data = MarketData::default();
        assert!(data.curves.is_empty());
        assert!(data.vol_surfaces.is_empty());
        assert!(data.fx_spots.is_empty());
    }

    #[test]
    fn test_market_data_fx_spot_map() {
        let data = MarketData {
            curves: vec![],
            vol_surfaces: vec![],
            fx_spots: vec![
                FxSpotData {
                    pair: "USDJPY".to_string(),
                    rate: 150.0,
                },
                FxSpotData {
                    pair: "EURUSD".to_string(),
                    rate: 1.08,
                },
            ],
        };

        let map = data.fx_spot_map();
        assert_eq!(map.get("USDJPY"), Some(&150.0));
        assert_eq!(map.get("EURUSD"), Some(&1.08));
    }

    #[test]
    fn test_market_data_get_curve() {
        let data = MarketData {
            curves: vec![CurveData {
                curve_id: "USD_SOFR".to_string(),
                curve_type: "sofr".to_string(),
                currency: "USD".to_string(),
                points: vec![],
                interpolation: "linear_on_zero_rate".to_string(),
            }],
            vol_surfaces: vec![],
            fx_spots: vec![],
        };

        assert!(data.get_curve("USD_SOFR").is_some());
        assert!(data.get_curve("EUR_ESTR").is_none());
    }

    #[test]
    fn test_market_data_get_curves_by_currency() {
        let data = MarketData {
            curves: vec![
                CurveData {
                    curve_id: "USD_SOFR".to_string(),
                    curve_type: "sofr".to_string(),
                    currency: "USD".to_string(),
                    points: vec![],
                    interpolation: "linear_on_zero_rate".to_string(),
                },
                CurveData {
                    curve_id: "USD_FORWARD".to_string(),
                    curve_type: "forward".to_string(),
                    currency: "USD".to_string(),
                    points: vec![],
                    interpolation: "linear_on_zero_rate".to_string(),
                },
                CurveData {
                    curve_id: "EUR_ESTR".to_string(),
                    curve_type: "estr".to_string(),
                    currency: "EUR".to_string(),
                    points: vec![],
                    interpolation: "linear_on_zero_rate".to_string(),
                },
            ],
            vol_surfaces: vec![],
            fx_spots: vec![],
        };

        let usd_curves = data.get_curves_by_currency("USD");
        assert_eq!(usd_curves.len(), 2);

        let eur_curves = data.get_curves_by_currency("EUR");
        assert_eq!(eur_curves.len(), 1);
    }

    #[test]
    fn test_market_data_deserialize() {
        let json = r#"{
            "curves": [
                {
                    "curve_id": "USD_SOFR",
                    "curve_type": "sofr",
                    "currency": "USD",
                    "points": [
                        { "tenor": 0.25, "rate": 0.045 },
                        { "tenor": 1.0, "rate": 0.05 }
                    ]
                }
            ],
            "vol_surfaces": [
                {
                    "surface_id": "USD_SWAPTION_VOL",
                    "surface_type": "swaption",
                    "currency": "USD",
                    "points": [
                        { "expiry": 1.0, "strike": 0.05, "volatility": 0.20 }
                    ]
                }
            ],
            "fx_spots": [
                { "pair": "USDJPY", "rate": 150.0 }
            ]
        }"#;

        let data: MarketData = serde_json::from_str(json).unwrap();
        assert_eq!(data.curves.len(), 1);
        assert_eq!(data.curves[0].points.len(), 2);
        assert_eq!(data.vol_surfaces.len(), 1);
        assert_eq!(data.fx_spots.len(), 1);
    }

    // -------------------------------------------------------------------------
    // CurveData Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_curve_data_default_interpolation() {
        let json = r#"{
            "curve_id": "TEST",
            "curve_type": "sofr",
            "currency": "USD",
            "points": []
        }"#;

        let curve: CurveData = serde_json::from_str(json).unwrap();
        assert_eq!(curve.interpolation, "linear_on_zero_rate");
    }

    // -------------------------------------------------------------------------
    // MarketLoader Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_market_loader_load() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
            "curves": [],
            "vol_surfaces": [],
            "fx_spots": [{{ "pair": "USDJPY", "rate": 150.0 }}]
        }}"#
        )
        .unwrap();

        let data = MarketLoader::load(file.path()).unwrap();
        assert_eq!(data.fx_spots.len(), 1);
    }

    // -------------------------------------------------------------------------
    // LoaderError Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_loader_error_json_error_display() {
        let err = LoaderError::JsonError {
            path: "test.json".to_string(),
            line: 10,
            column: 5,
            message: "expected value".to_string(),
        };

        let msg = err.to_string();
        assert!(msg.contains("test.json"));
        assert!(msg.contains("line 10"));
        assert!(msg.contains("column 5"));
    }

    #[test]
    fn test_loader_error_validation_error() {
        let err = LoaderError::validation_error("trades.json", "trade_id", "cannot be empty");

        let msg = err.to_string();
        assert!(msg.contains("trades.json"));
        assert!(msg.contains("trade_id"));
        assert!(msg.contains("cannot be empty"));
    }
}

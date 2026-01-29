//! Volatility Surface Loaders for Swaption and CapFloor vol quotes.
//!
//! # Requirements: 10.1, 10.2, 10.3, 10.5, 10.6
//!
//! This module provides loaders for volatility surface data from:
//! - CSV files with expiry, tenor, strike, vol columns
//! - JSON files with structured vol quote data
//!
//! Loaded data can be converted to `VolQuoteSet` for use with `VolCubeBuilder`.
//!
//! # Example
//!
//! ```rust,ignore
//! use adapter_loader::{VolSurfaceLoader, SwaptionVolCsvRow};
//!
//! // Load from CSV
//! let quotes = VolSurfaceLoader::load_swaption_csv("swaption_vol.csv")?;
//!
//! // Load from JSON
//! let quote_set = VolSurfaceLoader::load_json("vol_quotes.json")?;
//! ```

use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{error::LoaderError, JsonLoader};

// =============================================================================
// CSV Row Structures
// =============================================================================

/// CSV row for swaption volatility quotes.
///
/// # Requirements: 10.1, 10.3
///
/// Expected CSV format:
/// ```csv
/// expiry,tenor,strike,bid,ask,mid,quote_type,strike_type
/// 1Y,5Y,0.03,0.195,0.205,0.20,lognormal,absolute
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaptionVolCsvRow {
    /// Expiry tenor string (e.g., "1Y", "6M", "2Y").
    pub expiry: String,
    /// Underlying tenor string (e.g., "5Y", "10Y").
    pub tenor: String,
    /// Strike value.
    pub strike: f64,
    /// Bid volatility (optional).
    #[serde(default)]
    pub bid: Option<f64>,
    /// Ask volatility (optional).
    #[serde(default)]
    pub ask: Option<f64>,
    /// Mid volatility (required).
    pub mid: f64,
    /// Quote type: "normal", "lognormal", "shifted_lognormal".
    #[serde(default = "default_quote_type")]
    pub quote_type: String,
    /// Strike type: "absolute", "relative", "moneyness", "log_moneyness".
    #[serde(default = "default_strike_type")]
    pub strike_type: String,
    /// Shift value for shifted lognormal (optional).
    #[serde(default)]
    pub shift: Option<f64>,
}

fn default_quote_type() -> String { "lognormal".to_string() }
fn default_strike_type() -> String { "absolute".to_string() }

/// CSV row for capfloor volatility quotes.
///
/// # Requirements: 10.1, 10.3
///
/// Similar to swaption but with cap/floor specific fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapFloorVolCsvRow {
    /// Expiry/maturity tenor string (e.g., "1Y", "5Y").
    pub expiry: String,
    /// Strike value.
    pub strike: f64,
    /// Bid volatility (optional).
    #[serde(default)]
    pub bid: Option<f64>,
    /// Ask volatility (optional).
    #[serde(default)]
    pub ask: Option<f64>,
    /// Mid volatility (required).
    pub mid: f64,
    /// Quote type: "normal", "lognormal", "shifted_lognormal".
    #[serde(default = "default_quote_type")]
    pub quote_type: String,
    /// Strike type: "absolute", "relative", "moneyness".
    #[serde(default = "default_strike_type")]
    pub strike_type: String,
    /// Shift value for shifted lognormal (optional).
    #[serde(default)]
    pub shift: Option<f64>,
    /// Cap or Floor indicator.
    #[serde(default = "default_cap_floor_type")]
    pub cap_floor: String,
}

fn default_cap_floor_type() -> String { "cap".to_string() }

// =============================================================================
// JSON Structures
// =============================================================================

/// JSON structure for a single volatility quote.
///
/// # Requirements: 10.2, 10.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolQuoteJson {
    /// Unique instrument identifier.
    #[serde(default)]
    pub instrument_id: Option<String>,
    /// Expiry date (ISO format: YYYY-MM-DD) or tenor string.
    pub expiry: String,
    /// Underlying tenor in years or string.
    pub tenor: TenorValue,
    /// Strike specification.
    pub strike: StrikeValue,
    /// Bid volatility.
    #[serde(default)]
    pub bid: Option<f64>,
    /// Ask volatility.
    #[serde(default)]
    pub ask: Option<f64>,
    /// Mid volatility.
    pub mid: f64,
    /// Quote type.
    #[serde(default)]
    pub quote_type: Option<QuoteTypeJson>,
}

/// Tenor value - either numeric (years) or string (e.g., "5Y").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TenorValue {
    /// Numeric tenor in years.
    Years(f64),
    /// String tenor (e.g., "5Y", "6M").
    String(String),
}

impl TenorValue {
    /// Convert to years.
    pub fn to_years(&self) -> Result<f64, String> {
        match self {
            TenorValue::Years(y) => Ok(*y),
            TenorValue::String(s) => parse_tenor_string(s),
        }
    }
}

/// Strike value specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StrikeValue {
    /// Simple absolute strike value.
    Absolute(f64),
    /// Structured strike with type.
    Structured {
        /// Strike value.
        value: f64,
        /// Strike type: "absolute", "relative", "moneyness", "log_moneyness".
        #[serde(rename = "type")]
        strike_type: String,
    },
}

impl StrikeValue {
    /// Get strike value and type.
    pub fn value_and_type(&self) -> (f64, &str) {
        match self {
            StrikeValue::Absolute(v) => (*v, "absolute"),
            StrikeValue::Structured { value, strike_type } => (*value, strike_type.as_str()),
        }
    }
}

/// Quote type in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuoteTypeJson {
    /// Simple string: "normal", "lognormal".
    Simple(String),
    /// Structured with shift.
    Shifted {
        /// Type name.
        #[serde(rename = "type")]
        quote_type: String,
        /// Shift value.
        shift: f64,
    },
}

impl Default for QuoteTypeJson {
    fn default() -> Self { QuoteTypeJson::Simple("lognormal".to_string()) }
}

/// JSON structure for a volatility quote set.
///
/// # Requirements: 10.2, 10.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolQuoteSetJson {
    /// Currency code (e.g., "USD", "EUR").
    pub currency: String,
    /// Underlying index (e.g., "SOFR", "ESTR").
    #[serde(default)]
    pub underlying_index: Option<String>,
    /// As-of date (ISO format).
    pub as_of_date: String,
    /// Surface type: "swaption", "capfloor".
    #[serde(default = "default_surface_type")]
    pub surface_type: String,
    /// Individual quotes.
    pub quotes: Vec<VolQuoteJson>,
}

fn default_surface_type() -> String { "swaption".to_string() }

// =============================================================================
// Volatility Surface Loader
// =============================================================================

/// Volatility surface data loader.
///
/// # Requirements: 10.1, 10.2, 10.5, 10.6
///
/// Provides methods to load volatility surface data from CSV and JSON files.
pub struct VolSurfaceLoader;

impl VolSurfaceLoader {
    /// Load swaption volatility quotes from a CSV file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    ///
    /// # Returns
    ///
    /// Vector of parsed swaption vol rows.
    ///
    /// # Errors
    ///
    /// Returns `LoaderError` if file not found or parsing fails.
    pub fn load_swaption_csv<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<SwaptionVolCsvRow>, LoaderError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        if !path.exists() {
            return Err(LoaderError::FileNotFound(path_str));
        }

        let mut reader = csv::Reader::from_path(path)?;
        let mut rows = Vec::new();

        for (idx, result) in reader.deserialize().enumerate() {
            let row: SwaptionVolCsvRow = result.map_err(|e| LoaderError::InvalidFormat {
                row: idx + 2, // CSV row number (1-indexed, +1 for header)
                message: format!("Failed to parse swaption vol row: {}", e),
            })?;

            // Validate row
            Self::validate_swaption_row(&row, idx + 2, &path_str)?;
            rows.push(row);
        }

        Ok(rows)
    }

    /// Load capfloor volatility quotes from a CSV file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    ///
    /// # Returns
    ///
    /// Vector of parsed capfloor vol rows.
    pub fn load_capfloor_csv<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<CapFloorVolCsvRow>, LoaderError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        if !path.exists() {
            return Err(LoaderError::FileNotFound(path_str));
        }

        let mut reader = csv::Reader::from_path(path)?;
        let mut rows = Vec::new();

        for (idx, result) in reader.deserialize().enumerate() {
            let row: CapFloorVolCsvRow = result.map_err(|e| LoaderError::InvalidFormat {
                row: idx + 2,
                message: format!("Failed to parse capfloor vol row: {}", e),
            })?;

            // Validate row
            Self::validate_capfloor_row(&row, idx + 2, &path_str)?;
            rows.push(row);
        }

        Ok(rows)
    }

    /// Load volatility quote set from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Returns
    ///
    /// Parsed volatility quote set.
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<VolQuoteSetJson, LoaderError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        let quote_set: VolQuoteSetJson = JsonLoader::load(path)?;

        // Validate
        Self::validate_quote_set_json(&quote_set, &path_str)?;

        Ok(quote_set)
    }

    /// Load multiple volatility quote sets from JSON files matching a glob
    /// pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "data/volsurface/*.json")
    ///
    /// # Returns
    ///
    /// Vector of successfully loaded quote sets.
    pub fn load_json_glob(pattern: &str) -> Result<Vec<VolQuoteSetJson>, LoaderError> {
        JsonLoader::load_glob_ok(pattern)
    }

    // =========================================================================
    // Validation Helpers
    // =========================================================================

    fn validate_swaption_row(
        row: &SwaptionVolCsvRow,
        row_num: usize,
        path: &str,
    ) -> Result<(), LoaderError> {
        // Validate mid volatility
        if row.mid <= 0.0 {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("mid (row {})", row_num),
                reason: "Mid volatility must be positive".to_string(),
            });
        }

        // Validate expiry tenor format
        if parse_tenor_string(&row.expiry).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("expiry (row {})", row_num),
                reason: format!("Invalid expiry tenor format: {}", row.expiry),
            });
        }

        // Validate underlying tenor format
        if parse_tenor_string(&row.tenor).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("tenor (row {})", row_num),
                reason: format!("Invalid tenor format: {}", row.tenor),
            });
        }

        // Validate quote type
        let valid_quote_types = ["normal", "lognormal", "shifted_lognormal"];
        if !valid_quote_types.contains(&row.quote_type.to_lowercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("quote_type (row {})", row_num),
                reason: format!("Invalid quote type: {}", row.quote_type),
            });
        }

        // Validate strike type
        let valid_strike_types = ["absolute", "relative", "moneyness", "log_moneyness"];
        if !valid_strike_types.contains(&row.strike_type.to_lowercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("strike_type (row {})", row_num),
                reason: format!("Invalid strike type: {}", row.strike_type),
            });
        }

        // Validate bid/ask consistency
        if let (Some(bid), Some(ask)) = (row.bid, row.ask) {
            if bid > ask {
                return Err(LoaderError::ValidationError {
                    path: path.to_string(),
                    field: format!("bid/ask (row {})", row_num),
                    reason: "Bid must be less than or equal to ask".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_capfloor_row(
        row: &CapFloorVolCsvRow,
        row_num: usize,
        path: &str,
    ) -> Result<(), LoaderError> {
        // Validate mid volatility
        if row.mid <= 0.0 {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("mid (row {})", row_num),
                reason: "Mid volatility must be positive".to_string(),
            });
        }

        // Validate expiry tenor format
        if parse_tenor_string(&row.expiry).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("expiry (row {})", row_num),
                reason: format!("Invalid expiry tenor format: {}", row.expiry),
            });
        }

        // Validate cap/floor type
        let valid_types = ["cap", "floor"];
        if !valid_types.contains(&row.cap_floor.to_lowercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("cap_floor (row {})", row_num),
                reason: format!("Invalid cap/floor type: {}", row.cap_floor),
            });
        }

        Ok(())
    }

    fn validate_quote_set_json(quote_set: &VolQuoteSetJson, path: &str) -> Result<(), LoaderError> {
        // Validate currency
        let valid_currencies = ["USD", "EUR", "JPY", "GBP", "CHF", "AUD", "CAD"];
        if !valid_currencies.contains(&quote_set.currency.to_uppercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: "currency".to_string(),
                reason: format!("Unsupported currency: {}", quote_set.currency),
            });
        }

        // Validate as_of_date format
        if NaiveDate::parse_from_str(&quote_set.as_of_date, "%Y-%m-%d").is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: "as_of_date".to_string(),
                reason: format!(
                    "Invalid date format (expected YYYY-MM-DD): {}",
                    quote_set.as_of_date
                ),
            });
        }

        // Validate each quote
        for (i, quote) in quote_set.quotes.iter().enumerate() {
            if quote.mid <= 0.0 {
                return Err(LoaderError::ValidationError {
                    path: path.to_string(),
                    field: format!("quotes[{}].mid", i),
                    reason: "Mid volatility must be positive".to_string(),
                });
            }

            // Validate tenor
            if quote.tenor.to_years().is_err() {
                return Err(LoaderError::ValidationError {
                    path: path.to_string(),
                    field: format!("quotes[{}].tenor", i),
                    reason: "Invalid tenor format".to_string(),
                });
            }
        }

        Ok(())
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Parse a tenor string (e.g., "1Y", "6M", "3M") to years.
///
/// # Arguments
///
/// * `s` - Tenor string
///
/// # Returns
///
/// Tenor in years, or error if parsing fails.
pub fn parse_tenor_string(s: &str) -> Result<f64, String> {
    let s = s.trim().to_uppercase();

    if s.ends_with('Y') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid tenor format: {}", s))
    } else if s.ends_with('M') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|m| m / 12.0)
            .map_err(|_| format!("Invalid tenor format: {}", s))
    } else if s.ends_with('W') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|w| w / 52.0)
            .map_err(|_| format!("Invalid tenor format: {}", s))
    } else if s.ends_with('D') {
        let num_str = &s[..s.len() - 1];
        num_str
            .parse::<f64>()
            .map(|d| d / 365.0)
            .map_err(|_| format!("Invalid tenor format: {}", s))
    } else {
        // Try parsing as a plain number (years)
        s.parse::<f64>()
            .map_err(|_| format!("Invalid tenor format: {}", s))
    }
}

/// Convert expiry string to NaiveDate.
///
/// Supports:
/// - ISO date format: "2027-01-25"
/// - Tenor from as_of_date: "1Y", "6M", etc.
///
/// # Arguments
///
/// * `expiry_str` - Expiry string
/// * `as_of_date` - Reference date for tenor-based expiry
pub fn parse_expiry_string(expiry_str: &str, as_of_date: NaiveDate) -> Result<NaiveDate, String> {
    // Try ISO date format first
    if let Ok(date) = NaiveDate::parse_from_str(expiry_str, "%Y-%m-%d") {
        return Ok(date);
    }

    // Try tenor format
    let years = parse_tenor_string(expiry_str)?;
    let days = (years * 365.0).round() as i64;
    as_of_date
        .checked_add_signed(chrono::Duration::days(days))
        .ok_or_else(|| format!("Date overflow for expiry: {}", expiry_str))
}

/// Parse FRA tenor string in "NxM" format (e.g., "3x6", "3X6M", "3Mx6M").
///
/// FRA tenors represent forward rate agreements with a start and end period.
/// Common formats include:
/// - "3x6" - 3 months to 6 months
/// - "6x12" - 6 months to 12 months
/// - "3Mx6M" - 3 months to 6 months (explicit month suffix)
///
/// # Arguments
///
/// * `tenor` - FRA tenor string
///
/// # Returns
///
/// `Some((start_years, end_years))` if successful, `None` otherwise.
///
/// # Example
///
/// ```
/// use adapter_loader::parse_fra_tenor;
///
/// let result = parse_fra_tenor("3x6");
/// assert!(result.is_some());
/// let (start, end) = result.unwrap();
/// assert!((start - 0.25).abs() < 1e-10); // 3M = 0.25Y
/// assert!((end - 0.5).abs() < 1e-10);    // 6M = 0.5Y
/// ```
pub fn parse_fra_tenor(tenor: &str) -> Option<(f64, f64)> {
    let tenor = tenor.trim().to_uppercase();

    // Find the 'X' separator
    let x_pos = tenor.find('X')?;
    if x_pos == 0 || x_pos == tenor.len() - 1 {
        return None;
    }

    let start_part = &tenor[..x_pos];
    let end_part = &tenor[x_pos + 1..];

    // Parse start period
    let start_months = parse_fra_period(start_part)?;

    // Parse end period
    let end_months = parse_fra_period(end_part)?;

    if end_months <= start_months {
        return None;
    }

    Some((start_months / 12.0, end_months / 12.0))
}

/// Parse a single FRA period part (e.g., "3", "3M", "12M").
///
/// # Arguments
///
/// * `s` - Period string (with or without 'M' suffix)
///
/// # Returns
///
/// Period in months, or `None` if parsing fails.
fn parse_fra_period(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // If ends with 'M', strip it and parse as months
    if s.ends_with('M') {
        s[..s.len() - 1].parse::<f64>().ok()
    } else {
        // Assume it's already in months
        s.parse::<f64>().ok()
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
    // Tenor Parsing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_tenor_years() {
        assert!((parse_tenor_string("1Y").unwrap() - 1.0).abs() < 1e-10);
        assert!((parse_tenor_string("5Y").unwrap() - 5.0).abs() < 1e-10);
        assert!((parse_tenor_string("10Y").unwrap() - 10.0).abs() < 1e-10);
        assert!((parse_tenor_string("0.5Y").unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_tenor_months() {
        assert!((parse_tenor_string("6M").unwrap() - 0.5).abs() < 1e-10);
        assert!((parse_tenor_string("3M").unwrap() - 0.25).abs() < 1e-10);
        assert!((parse_tenor_string("12M").unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_tenor_weeks() {
        assert!((parse_tenor_string("1W").unwrap() - 1.0 / 52.0).abs() < 1e-10);
        assert!((parse_tenor_string("4W").unwrap() - 4.0 / 52.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_tenor_days() {
        assert!((parse_tenor_string("30D").unwrap() - 30.0 / 365.0).abs() < 1e-10);
        assert!((parse_tenor_string("90D").unwrap() - 90.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_tenor_invalid() {
        assert!(parse_tenor_string("XYZ").is_err());
        assert!(parse_tenor_string("").is_err());
    }

    #[test]
    fn test_parse_tenor_case_insensitive() {
        assert!((parse_tenor_string("1y").unwrap() - 1.0).abs() < 1e-10);
        assert!((parse_tenor_string("6m").unwrap() - 0.5).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // FRA Tenor Parsing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_fra_tenor_3x6() {
        let result = parse_fra_tenor("3x6");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10); // 3M = 0.25Y
        assert!((end - 0.5).abs() < 1e-10); // 6M = 0.5Y
    }

    #[test]
    fn test_parse_fra_tenor_6x12() {
        let result = parse_fra_tenor("6x12");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.5).abs() < 1e-10); // 6M = 0.5Y
        assert!((end - 1.0).abs() < 1e-10); // 12M = 1.0Y
    }

    #[test]
    fn test_parse_fra_tenor_with_suffix() {
        // Test "3Mx6M" format
        let result = parse_fra_tenor("3Mx6M");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);

        // Test "1Mx4M" format
        let result = parse_fra_tenor("1Mx4M");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 1.0 / 12.0).abs() < 1e-10);
        assert!((end - 4.0 / 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_fra_tenor_case_insensitive() {
        assert!(parse_fra_tenor("3X6").is_some());
        assert!(parse_fra_tenor("3x6").is_some());
        assert!(parse_fra_tenor("3X6M").is_some());
    }

    #[test]
    fn test_parse_fra_tenor_invalid() {
        // Not FRA format
        assert!(parse_fra_tenor("6M").is_none());
        assert!(parse_fra_tenor("1Y").is_none());

        // Invalid: end <= start
        assert!(parse_fra_tenor("6x3").is_none());
        assert!(parse_fra_tenor("12x6").is_none());

        // Invalid: empty parts
        assert!(parse_fra_tenor("x6").is_none());
        assert!(parse_fra_tenor("3x").is_none());
        assert!(parse_fra_tenor("").is_none());
    }

    // -------------------------------------------------------------------------
    // Expiry Parsing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_expiry_iso_date() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = parse_expiry_string("2027-01-25", as_of).unwrap();
        assert_eq!(expiry, NaiveDate::from_ymd_opt(2027, 1, 25).unwrap());
    }

    #[test]
    fn test_parse_expiry_tenor() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = parse_expiry_string("1Y", as_of).unwrap();
        // Approximately 1 year later
        let days_diff = (expiry - as_of).num_days();
        assert!(days_diff >= 364 && days_diff <= 366);
    }

    // -------------------------------------------------------------------------
    // CSV Loading Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_load_swaption_csv_valid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "expiry,tenor,strike,bid,ask,mid,quote_type,strike_type"
        )
        .unwrap();
        writeln!(file, "1Y,5Y,0.03,0.195,0.205,0.20,lognormal,absolute").unwrap();
        writeln!(file, "2Y,10Y,0.035,0.180,0.190,0.185,lognormal,absolute").unwrap();

        let rows = VolSurfaceLoader::load_swaption_csv(file.path()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].expiry, "1Y");
        assert_eq!(rows[0].tenor, "5Y");
        assert!((rows[0].mid - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_load_swaption_csv_minimal_columns() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "expiry,tenor,strike,mid").unwrap();
        writeln!(file, "1Y,5Y,0.03,0.20").unwrap();

        let rows = VolSurfaceLoader::load_swaption_csv(file.path()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].bid.is_none());
        assert!(rows[0].ask.is_none());
        assert_eq!(rows[0].quote_type, "lognormal"); // default
        assert_eq!(rows[0].strike_type, "absolute"); // default
    }

    #[test]
    fn test_load_swaption_csv_invalid_mid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "expiry,tenor,strike,mid").unwrap();
        writeln!(file, "1Y,5Y,0.03,-0.20").unwrap();

        let result = VolSurfaceLoader::load_swaption_csv(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Mid volatility must be positive"));
    }

    #[test]
    fn test_load_swaption_csv_invalid_expiry() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "expiry,tenor,strike,mid").unwrap();
        writeln!(file, "INVALID,5Y,0.03,0.20").unwrap();

        let result = VolSurfaceLoader::load_swaption_csv(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid expiry"));
    }

    #[test]
    fn test_load_swaption_csv_file_not_found() {
        let result = VolSurfaceLoader::load_swaption_csv("nonexistent.csv");
        assert!(matches!(result, Err(LoaderError::FileNotFound(_))));
    }

    #[test]
    fn test_load_capfloor_csv_valid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "expiry,strike,mid,cap_floor").unwrap();
        writeln!(file, "5Y,0.03,0.25,cap").unwrap();
        writeln!(file, "10Y,0.04,0.22,floor").unwrap();

        let rows = VolSurfaceLoader::load_capfloor_csv(file.path()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].cap_floor, "cap");
        assert_eq!(rows[1].cap_floor, "floor");
    }

    // -------------------------------------------------------------------------
    // JSON Loading Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_load_json_valid() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "USD",
            "underlying_index": "SOFR",
            "as_of_date": "2026-01-25",
            "surface_type": "swaption",
            "quotes": [
                {
                    "instrument_id": "SWAPTION-1Y-5Y-ATM",
                    "expiry": "2027-01-25",
                    "tenor": 5.0,
                    "strike": 0.03,
                    "mid": 0.20,
                    "bid": 0.195,
                    "ask": 0.205
                }
            ]
        }"#;
        writeln!(file, "{}", json).unwrap();

        let quote_set = VolSurfaceLoader::load_json(file.path()).unwrap();
        assert_eq!(quote_set.currency, "USD");
        assert_eq!(quote_set.quotes.len(), 1);
        assert!((quote_set.quotes[0].mid - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_load_json_tenor_string() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "EUR",
            "as_of_date": "2026-01-25",
            "quotes": [
                {
                    "expiry": "1Y",
                    "tenor": "5Y",
                    "strike": 0.02,
                    "mid": 0.18
                }
            ]
        }"#;
        writeln!(file, "{}", json).unwrap();

        let quote_set = VolSurfaceLoader::load_json(file.path()).unwrap();
        assert_eq!(quote_set.quotes.len(), 1);

        let tenor_years = quote_set.quotes[0].tenor.to_years().unwrap();
        assert!((tenor_years - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_load_json_structured_strike() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "USD",
            "as_of_date": "2026-01-25",
            "quotes": [
                {
                    "expiry": "1Y",
                    "tenor": 5.0,
                    "strike": { "value": 50.0, "type": "relative" },
                    "mid": 0.20
                }
            ]
        }"#;
        writeln!(file, "{}", json).unwrap();

        let quote_set = VolSurfaceLoader::load_json(file.path()).unwrap();
        let (value, strike_type) = quote_set.quotes[0].strike.value_and_type();
        assert!((value - 50.0).abs() < 1e-10);
        assert_eq!(strike_type, "relative");
    }

    #[test]
    fn test_load_json_invalid_currency() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "XYZ",
            "as_of_date": "2026-01-25",
            "quotes": []
        }"#;
        writeln!(file, "{}", json).unwrap();

        let result = VolSurfaceLoader::load_json(file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported currency"));
    }

    #[test]
    fn test_load_json_invalid_date() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "USD",
            "as_of_date": "invalid-date",
            "quotes": []
        }"#;
        writeln!(file, "{}", json).unwrap();

        let result = VolSurfaceLoader::load_json(file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid date format"));
    }

    #[test]
    fn test_load_json_invalid_mid() {
        let mut file = NamedTempFile::new().unwrap();
        let json = r#"{
            "currency": "USD",
            "as_of_date": "2026-01-25",
            "quotes": [
                {
                    "expiry": "1Y",
                    "tenor": 5.0,
                    "strike": 0.03,
                    "mid": -0.20
                }
            ]
        }"#;
        writeln!(file, "{}", json).unwrap();

        let result = VolSurfaceLoader::load_json(file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Mid volatility must be positive"));
    }

    // -------------------------------------------------------------------------
    // TenorValue Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_tenor_value_years() {
        let tv = TenorValue::Years(5.0);
        assert!((tv.to_years().unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_value_string() {
        let tv = TenorValue::String("6M".to_string());
        assert!((tv.to_years().unwrap() - 0.5).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // StrikeValue Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_strike_value_absolute() {
        let sv = StrikeValue::Absolute(0.03);
        let (v, t) = sv.value_and_type();
        assert!((v - 0.03).abs() < 1e-10);
        assert_eq!(t, "absolute");
    }

    #[test]
    fn test_strike_value_structured() {
        let sv = StrikeValue::Structured {
            value: 50.0,
            strike_type: "relative".to_string(),
        };
        let (v, t) = sv.value_and_type();
        assert!((v - 50.0).abs() < 1e-10);
        assert_eq!(t, "relative");
    }
}

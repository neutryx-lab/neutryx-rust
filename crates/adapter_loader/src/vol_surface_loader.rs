//! Volatility Surface Loaders for Swaption and CapFloor vol quotes.

use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{error::LoaderError, JsonLoader};

/// CSV row for swaption volatility quotes.
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

/// JSON structure for a single volatility quote.
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

/// Volatility surface data loader.
pub struct VolSurfaceLoader;

impl VolSurfaceLoader {
    /// Load swaption volatility quotes from a CSV file.
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
                row: idx + 2,
                message: format!("Failed to parse swaption vol row: {}", e),
            })?;

            Self::validate_swaption_row(&row, idx + 2, &path_str)?;
            rows.push(row);
        }

        Ok(rows)
    }

    /// Load capfloor volatility quotes from a CSV file.
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

            Self::validate_capfloor_row(&row, idx + 2, &path_str)?;
            rows.push(row);
        }

        Ok(rows)
    }

    /// Load volatility quote set from a JSON file.
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<VolQuoteSetJson, LoaderError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        let quote_set: VolQuoteSetJson = JsonLoader::load(path)?;

        Self::validate_quote_set_json(&quote_set, &path_str)?;

        Ok(quote_set)
    }

    /// Load multiple volatility quote sets from JSON files matching a glob.
    pub fn load_json_glob(pattern: &str) -> Result<Vec<VolQuoteSetJson>, LoaderError> {
        JsonLoader::load_glob_ok(pattern)
    }

    fn validate_swaption_row(
        row: &SwaptionVolCsvRow,
        row_num: usize,
        path: &str,
    ) -> Result<(), LoaderError> {
        if row.mid <= 0.0 {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("mid (row {})", row_num),
                reason: "Mid volatility must be positive".to_string(),
            });
        }

        if parse_tenor_string(&row.expiry).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("expiry (row {})", row_num),
                reason: format!("Invalid expiry tenor format: {}", row.expiry),
            });
        }

        if parse_tenor_string(&row.tenor).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("tenor (row {})", row_num),
                reason: format!("Invalid tenor format: {}", row.tenor),
            });
        }

        let valid_quote_types = ["normal", "lognormal", "shifted_lognormal"];
        if !valid_quote_types.contains(&row.quote_type.to_lowercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("quote_type (row {})", row_num),
                reason: format!("Invalid quote type: {}", row.quote_type),
            });
        }

        let valid_strike_types = ["absolute", "relative", "moneyness", "log_moneyness"];
        if !valid_strike_types.contains(&row.strike_type.to_lowercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("strike_type (row {})", row_num),
                reason: format!("Invalid strike type: {}", row.strike_type),
            });
        }

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
        if row.mid <= 0.0 {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("mid (row {})", row_num),
                reason: "Mid volatility must be positive".to_string(),
            });
        }

        if parse_tenor_string(&row.expiry).is_err() {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: format!("expiry (row {})", row_num),
                reason: format!("Invalid expiry tenor format: {}", row.expiry),
            });
        }

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
        let valid_currencies = ["USD", "EUR", "JPY", "GBP", "CHF", "AUD", "CAD"];
        if !valid_currencies.contains(&quote_set.currency.to_uppercase().as_str()) {
            return Err(LoaderError::ValidationError {
                path: path.to_string(),
                field: "currency".to_string(),
                reason: format!("Unsupported currency: {}", quote_set.currency),
            });
        }

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

        for (i, quote) in quote_set.quotes.iter().enumerate() {
            if quote.mid <= 0.0 {
                return Err(LoaderError::ValidationError {
                    path: path.to_string(),
                    field: format!("quotes[{}].mid", i),
                    reason: "Mid volatility must be positive".to_string(),
                });
            }

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

/// Parse a tenor string (e.g., "1Y", "6M", "3M") to years.
pub fn parse_tenor_string(s: &str) -> Result<f64, String> {
    infra_domain::time::parse_tenor_to_years(s)
}

/// Convert expiry string to NaiveDate.
pub fn parse_expiry_string(expiry_str: &str, as_of_date: NaiveDate) -> Result<NaiveDate, String> {
    let as_of_date = infra_domain::time::Date::from(as_of_date);
    infra_domain::time::parse_expiry_to_date(expiry_str, as_of_date).map(|d| d.into_inner())
}

/// Parse FRA tenor string in "NxM" format (e.g., "3x6", "3X6M", "3Mx6M").
pub fn parse_fra_tenor(tenor: &str) -> Option<(f64, f64)> {
    infra_domain::time::parse_fra_tenor(tenor)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

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
        assert!((parse_tenor_string("1W").unwrap() - 7.0 / 365.0).abs() < 1e-10);
        assert!((parse_tenor_string("4W").unwrap() - 28.0 / 365.0).abs() < 1e-10);
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

    #[test]
    fn test_parse_fra_tenor_3x6() {
        let result = parse_fra_tenor("3x6");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_fra_tenor_6x12() {
        let result = parse_fra_tenor("6x12");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.5).abs() < 1e-10);
        assert!((end - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_fra_tenor_with_suffix() {
        let result = parse_fra_tenor("3Mx6M");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);

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
        assert!(parse_fra_tenor("6M").is_none());
        assert!(parse_fra_tenor("1Y").is_none());

        assert!(parse_fra_tenor("6x3").is_none());
        assert!(parse_fra_tenor("12x6").is_none());

        assert!(parse_fra_tenor("x6").is_none());
        assert!(parse_fra_tenor("3x").is_none());
        assert!(parse_fra_tenor("").is_none());
    }

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
        let days_diff = (expiry - as_of).num_days();
        assert!(days_diff >= 364 && days_diff <= 366);
    }

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
        assert_eq!(rows[0].quote_type, "lognormal");
        assert_eq!(rows[0].strike_type, "absolute");
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

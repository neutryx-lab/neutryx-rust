//! Ticker mapping loader for external data sources.
//!
//! This module provides loading of ticker mappings from JSON and CSV files,
//! converting external data provider tickers (Bloomberg, Reuters) to internal
//! [`QuoteId`] identifiers.
//!
//! # JSON Format
//!
//! ```json
//! [
//!   {
//!     "ticker": "USD3MD=",
//!     "currency": "USD",
//!     "tenor": "3M",
//!     "rate_type": "Deposit"
//!   },
//!   {
//!     "ticker": "USSW5 Curncy",
//!     "currency": "USD",
//!     "tenor": "5Y",
//!     "rate_type": "Swap"
//!   }
//! ]
//! ```
//!
//! # CSV Format
//!
//! ```csv
//! ticker,currency,tenor,rate_type
//! USD3MD=,USD,3M,Deposit
//! USSW5 Curncy,USD,5Y,Swap
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use adapter_loader::TickerMappingLoader;
//!
//! // Load from JSON
//! let mapping = TickerMappingLoader::load_json("tickers.json")?;
//!
//! // Load from CSV
//! let mapping = TickerMappingLoader::load_csv("tickers.csv")?;
//!
//! // Lookup a rate
//! if let Some(quote_id) = mapping.lookup("USD3MD=") {
//!     println!("Found: {}", quote_id);
//! }
//! ```

use std::{fs::File, io::BufReader, path::Path};

use serde::Deserialize;

use crate::{error::LoaderError, JsonLoader};
use infra_domain::{
    market::{core::RateType, quote::QuoteId, Currency, TickerMapping},
    time::Tenor,
};

/// A single ticker mapping entry for deserialisation.
///
/// This struct represents one row in a ticker mapping file (JSON or CSV).
#[derive(Debug, Clone, Deserialize)]
pub struct TickerMappingEntry {
    /// External ticker string (e.g., "USD3MD=", "USSW5 Curncy")
    pub ticker: String,
    /// Currency code
    pub currency: Currency,
    /// Tenor (e.g., "3M", "5Y")
    pub tenor: Tenor,
    /// Rate type
    pub rate_type: RateType,
}

impl TickerMappingEntry {
    /// Converts this entry to a [`QuoteId`].
    #[must_use]
    pub fn to_quote_id(&self) -> QuoteId {
        QuoteId::new(self.currency, self.tenor, self.rate_type)
    }
}

/// Loader for ticker mapping files.
///
/// Loads ticker mappings from JSON or CSV files and converts them to
/// [`TickerMapping`] instances.
///
/// # Examples
///
/// ```rust,ignore
/// use adapter_loader::TickerMappingLoader;
///
/// // Load from JSON file
/// let mapping = TickerMappingLoader::load_json("config/tickers.json")?;
///
/// // Load from CSV file
/// let mapping = TickerMappingLoader::load_csv("config/tickers.csv")?;
///
/// // Merge with defaults
/// let mapping = TickerMappingLoader::load_json_with_defaults("config/custom_tickers.json")?;
/// ```
pub struct TickerMappingLoader;

impl TickerMappingLoader {
    /// Loads ticker mappings from a JSON file.
    ///
    /// The JSON file should contain an array of ticker mapping entries.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Returns
    ///
    /// A [`TickerMapping`] containing all entries from the file.
    ///
    /// # Errors
    ///
    /// Returns [`LoaderError`] if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mapping = TickerMappingLoader::load_json("tickers.json")?;
    /// assert!(mapping.contains("USD3MD="));
    /// ```
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let entries: Vec<TickerMappingEntry> = JsonLoader::load(path)?;
        Ok(Self::entries_to_mapping(entries))
    }

    /// Loads ticker mappings from a JSON file and merges with defaults.
    ///
    /// Default mappings are loaded first, then custom mappings are added
    /// (overwriting any duplicates).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file with custom mappings
    ///
    /// # Returns
    ///
    /// A [`TickerMapping`] containing default mappings plus custom entries.
    pub fn load_json_with_defaults<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let mut mapping = TickerMapping::with_defaults();
        let entries: Vec<TickerMappingEntry> = JsonLoader::load(path)?;

        for entry in entries {
            mapping.register(&entry.ticker, entry.to_quote_id());
        }

        Ok(mapping)
    }

    /// Loads ticker mappings from a CSV file.
    ///
    /// The CSV file should have columns: `ticker`, `currency`, `tenor`, `rate_type`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    ///
    /// # Returns
    ///
    /// A [`TickerMapping`] containing all entries from the file.
    ///
    /// # Errors
    ///
    /// Returns [`LoaderError`] if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mapping = TickerMappingLoader::load_csv("tickers.csv")?;
    /// ```
    pub fn load_csv<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(LoaderError::file_not_found(path.display().to_string()));
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut csv_reader = csv::Reader::from_reader(reader);

        let mut mapping = TickerMapping::new();
        for result in csv_reader.deserialize() {
            let entry: TickerMappingEntry = result?;
            mapping.register(&entry.ticker, entry.to_quote_id());
        }

        Ok(mapping)
    }

    /// Loads ticker mappings from a CSV file and merges with defaults.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file with custom mappings
    ///
    /// # Returns
    ///
    /// A [`TickerMapping`] containing default mappings plus custom entries.
    pub fn load_csv_with_defaults<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(LoaderError::file_not_found(path.display().to_string()));
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut csv_reader = csv::Reader::from_reader(reader);

        let mut mapping = TickerMapping::with_defaults();
        for result in csv_reader.deserialize() {
            let entry: TickerMappingEntry = result?;
            mapping.register(&entry.ticker, entry.to_quote_id());
        }

        Ok(mapping)
    }

    /// Loads ticker mappings from multiple JSON files matching a glob pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern (e.g., "config/tickers/*.json")
    ///
    /// # Returns
    ///
    /// A [`TickerMapping`] containing entries from all matching files.
    pub fn load_json_glob(pattern: &str) -> Result<TickerMapping, LoaderError> {
        let results = JsonLoader::load_glob::<Vec<TickerMappingEntry>>(pattern)?;
        let mut mapping = TickerMapping::new();

        for result in results.into_iter().flatten() {
            let (_, entries) = result;
            for entry in entries {
                mapping.register(&entry.ticker, entry.to_quote_id());
            }
        }

        Ok(mapping)
    }

    /// Converts a vector of entries to a [`TickerMapping`].
    fn entries_to_mapping(entries: Vec<TickerMappingEntry>) -> TickerMapping {
        let mut mapping = TickerMapping::new();
        for entry in entries {
            mapping.register(&entry.ticker, entry.to_quote_id());
        }
        mapping
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_load_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[
                {{
                    "ticker": "USD3MD=",
                    "currency": "USD",
                    "tenor": "3M",
                    "rate_type": "Deposit"
                }},
                {{
                    "ticker": "USSW5 Curncy",
                    "currency": "USD",
                    "tenor": "5Y",
                    "rate_type": "Swap"
                }}
            ]"#
        )
        .unwrap();

        let mapping = TickerMappingLoader::load_json(file.path()).unwrap();

        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains("USD3MD="));
        assert!(mapping.contains("USSW5 Curncy"));

        let quote_id = mapping.lookup("USD3MD=").unwrap();
        assert_eq!(quote_id.currency, Currency::USD);
        assert_eq!(quote_id.tenor, Tenor::ThreeMonths);
        assert_eq!(quote_id.rate_type, RateType::Deposit);
    }

    #[test]
    fn test_load_json_with_defaults() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[
                {{
                    "ticker": "CUSTOM_TICKER",
                    "currency": "EUR",
                    "tenor": "10Y",
                    "rate_type": "Swap"
                }}
            ]"#
        )
        .unwrap();

        let mapping = TickerMappingLoader::load_json_with_defaults(file.path()).unwrap();

        // Should have defaults
        assert!(mapping.contains("USD3MD="));
        assert!(mapping.contains("EUR3MD="));

        // Plus custom
        assert!(mapping.contains("CUSTOM_TICKER"));

        let quote_id = mapping.lookup("CUSTOM_TICKER").unwrap();
        assert_eq!(quote_id.currency, Currency::EUR);
        assert_eq!(quote_id.tenor, Tenor::TenYears);
    }

    #[test]
    fn test_load_csv() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(file, "ticker,currency,tenor,rate_type").unwrap();
        writeln!(file, "GBP6MD=,GBP,6M,Deposit").unwrap();
        writeln!(file, "BPSW10 Curncy,GBP,10Y,Swap").unwrap();

        let mapping = TickerMappingLoader::load_csv(file.path()).unwrap();

        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains("GBP6MD="));
        assert!(mapping.contains("BPSW10 Curncy"));

        let quote_id = mapping.lookup("GBP6MD=").unwrap();
        assert_eq!(quote_id.currency, Currency::GBP);
        assert_eq!(quote_id.tenor, Tenor::SixMonths);
        assert_eq!(quote_id.rate_type, RateType::Deposit);
    }

    #[test]
    fn test_load_csv_with_defaults() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(file, "ticker,currency,tenor,rate_type").unwrap();
        writeln!(file, "JYSW10 Curncy,JPY,10Y,Swap").unwrap();

        let mapping = TickerMappingLoader::load_csv_with_defaults(file.path()).unwrap();

        // Should have defaults
        assert!(mapping.contains("USD3MD="));

        // Plus custom
        assert!(mapping.contains("JYSW10 Curncy"));
    }

    #[test]
    fn test_ticker_mapping_entry_to_quote_id() {
        let entry = TickerMappingEntry {
            ticker: "TEST".to_string(),
            currency: Currency::CHF,
            tenor: Tenor::TwoYears,
            rate_type: RateType::Ois,
        };

        let quote_id = entry.to_quote_id();
        assert_eq!(quote_id.currency, Currency::CHF);
        assert_eq!(quote_id.tenor, Tenor::TwoYears);
        assert_eq!(quote_id.rate_type, RateType::Ois);
    }

    #[test]
    fn test_load_json_file_not_found() {
        let result = TickerMappingLoader::load_json("nonexistent.json");
        assert!(matches!(result, Err(LoaderError::FileNotFound(_))));
    }

    #[test]
    fn test_load_json_invalid_format() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{ "not": "an array" }}"#).unwrap();

        let result = TickerMappingLoader::load_json(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_overwrite_on_duplicate() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[
                {{
                    "ticker": "DUPLICATE",
                    "currency": "USD",
                    "tenor": "1M",
                    "rate_type": "Deposit"
                }},
                {{
                    "ticker": "DUPLICATE",
                    "currency": "EUR",
                    "tenor": "5Y",
                    "rate_type": "Swap"
                }}
            ]"#
        )
        .unwrap();

        let mapping = TickerMappingLoader::load_json(file.path()).unwrap();

        // Should only have one entry (last one wins)
        assert_eq!(mapping.len(), 1);

        let quote_id = mapping.lookup("DUPLICATE").unwrap();
        assert_eq!(quote_id.currency, Currency::EUR);
        assert_eq!(quote_id.tenor, Tenor::FiveYears);
    }
}

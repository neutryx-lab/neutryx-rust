//! Ticker mapping loader for external data sources.

use std::{fs::File, io::BufReader, path::Path};

use infra_domain::{
    market::{core::QuoteCategory, quote::QuoteId, Currency, TickerMapping},
    time::Tenor,
};
use serde::Deserialize;

use crate::{error::LoaderError, JsonLoader};

/// A single ticker mapping entry for deserialisation.
#[derive(Debug, Clone, Deserialize)]
pub struct TickerMappingEntry {
    /// External ticker string (e.g., "USD3MD=", "USSW5 Curncy").
    pub ticker: String,
    /// Currency code.
    pub currency: Currency,
    /// Tenor (e.g., "3M", "5Y").
    pub tenor: Tenor,
    /// Rate type.
    pub quote_category: QuoteCategory,
}

impl TickerMappingEntry {
    /// Converts this entry to a [`QuoteId`].
    #[must_use]
    pub fn to_quote_id(&self) -> QuoteId {
        QuoteId::new(self.currency, self.tenor, self.quote_category)
    }
}

/// Loader for ticker mapping files.
pub struct TickerMappingLoader;

impl TickerMappingLoader {
    /// Loads ticker mappings from a JSON file.
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let entries: Vec<TickerMappingEntry> = JsonLoader::load(path)?;
        Ok(Self::entries_to_mapping(entries))
    }

    /// Loads ticker mappings from a JSON file and merges with defaults.
    pub fn load_json_with_defaults<P: AsRef<Path>>(path: P) -> Result<TickerMapping, LoaderError> {
        let mut mapping = TickerMapping::with_defaults();
        let entries: Vec<TickerMappingEntry> = JsonLoader::load(path)?;

        for entry in entries {
            mapping.register(&entry.ticker, entry.to_quote_id());
        }

        Ok(mapping)
    }

    /// Loads ticker mappings from a CSV file.
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
                    "quote_category": "Deposit"
                }},
                {{
                    "ticker": "USSW5 Curncy",
                    "currency": "USD",
                    "tenor": "5Y",
                    "quote_category": "Swap"
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
        assert_eq!(quote_id.quote_category, QuoteCategory::Deposit);
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
                    "quote_category": "Swap"
                }}
            ]"#
        )
        .unwrap();

        let mapping = TickerMappingLoader::load_json_with_defaults(file.path()).unwrap();

        assert!(mapping.contains("USD3MD="));
        assert!(mapping.contains("EUR3MD="));

        assert!(mapping.contains("CUSTOM_TICKER"));

        let quote_id = mapping.lookup("CUSTOM_TICKER").unwrap();
        assert_eq!(quote_id.currency, Currency::EUR);
        assert_eq!(quote_id.tenor, Tenor::TenYears);
    }

    #[test]
    fn test_load_csv() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(file, "ticker,currency,tenor,quote_category").unwrap();
        writeln!(file, "GBP6MD=,GBP,6M,Deposit").unwrap();
        writeln!(file, "BPSW10 Curncy,GBP,10Y,Swap").unwrap();

        let mapping = TickerMappingLoader::load_csv(file.path()).unwrap();

        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains("GBP6MD="));
        assert!(mapping.contains("BPSW10 Curncy"));

        let quote_id = mapping.lookup("GBP6MD=").unwrap();
        assert_eq!(quote_id.currency, Currency::GBP);
        assert_eq!(quote_id.tenor, Tenor::SixMonths);
        assert_eq!(quote_id.quote_category, QuoteCategory::Deposit);
    }

    #[test]
    fn test_load_csv_with_defaults() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(file, "ticker,currency,tenor,quote_category").unwrap();
        writeln!(file, "JYSW10 Curncy,JPY,10Y,Swap").unwrap();

        let mapping = TickerMappingLoader::load_csv_with_defaults(file.path()).unwrap();

        assert!(mapping.contains("USD3MD="));

        assert!(mapping.contains("JYSW10 Curncy"));
    }

    #[test]
    fn test_ticker_mapping_entry_to_quote_id() {
        let entry = TickerMappingEntry {
            ticker: "TEST".to_string(),
            currency: Currency::CHF,
            tenor: Tenor::TwoYears,
            quote_category: QuoteCategory::Ois,
        };

        let quote_id = entry.to_quote_id();
        assert_eq!(quote_id.currency, Currency::CHF);
        assert_eq!(quote_id.tenor, Tenor::TwoYears);
        assert_eq!(quote_id.quote_category, QuoteCategory::Ois);
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
                    "quote_category": "Deposit"
                }},
                {{
                    "ticker": "DUPLICATE",
                    "currency": "EUR",
                    "tenor": "5Y",
                    "quote_category": "Swap"
                }}
            ]"#
        )
        .unwrap();

        let mapping = TickerMappingLoader::load_json(file.path()).unwrap();

        assert_eq!(mapping.len(), 1);

        let quote_id = mapping.lookup("DUPLICATE").unwrap();
        assert_eq!(quote_id.currency, Currency::EUR);
        assert_eq!(quote_id.tenor, Tenor::FiveYears);
    }
}

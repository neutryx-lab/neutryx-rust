//! CSV file loader.

use std::path::Path;

use crate::error::LoaderError;

/// CSV file loader for trade and market data.
pub struct CsvLoader;

impl CsvLoader {
    /// Load records from a CSV file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    ///
    /// # Returns
    ///
    /// A vector of parsed records, or an error if loading fails.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<CsvRecord>, LoaderError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(LoaderError::FileNotFound(path.display().to_string()));
        }

        let mut reader = csv::Reader::from_path(path)?;
        let mut records = Vec::new();

        for (idx, result) in reader.records().enumerate() {
            let record = result?;
            records.push(CsvRecord {
                row: idx + 1,
                fields: record.iter().map(|s| s.to_string()).collect(),
            });
        }

        Ok(records)
    }
}

/// A single CSV record.
#[derive(Debug, Clone)]
pub struct CsvRecord {
    /// Row number (1-indexed)
    pub row: usize,
    /// Field values
    pub fields: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found() {
        let result = CsvLoader::load("nonexistent.csv");
        assert!(matches!(result, Err(LoaderError::FileNotFound(_))));
    }
}

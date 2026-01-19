//! Loader errors.

use thiserror::Error;

/// Errors that can occur during file loading.
#[derive(Error, Debug)]
pub enum LoaderError {
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// CSV parsing error
    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),

    /// Missing required column
    #[error("Missing required column: {0}")]
    MissingColumn(String),

    /// Invalid data format
    #[error("Invalid data format in row {row}: {message}")]
    InvalidFormat {
        /// Row number where the error occurred
        row: usize,
        /// Description of the format error
        message: String,
    },

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),
}

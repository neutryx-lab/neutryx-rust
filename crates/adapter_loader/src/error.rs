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

    /// JSON parsing error
    #[error("JSON parse error in '{path}' at line {line}, column {column}: {message}")]
    JsonError {
        /// File path where the error occurred
        path: String,
        /// Line number (1-indexed)
        line: usize,
        /// Column number (1-indexed)
        column: usize,
        /// Description of the parse error
        message: String,
    },

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

    /// Glob pattern error
    #[error("Invalid glob pattern '{pattern}': {message}")]
    GlobPatternError {
        /// The invalid pattern
        pattern: String,
        /// Description of the error
        message: String,
    },

    /// Validation error
    #[error("Validation error in '{path}' for field '{field}': {reason}")]
    ValidationError {
        /// File path where validation failed
        path: String,
        /// Field that failed validation
        field: String,
        /// Reason for validation failure
        reason: String,
    },
}

impl LoaderError {
    /// Creates a JSON error with full context.
    #[must_use]
    pub fn json_error(path: impl Into<String>, err: &serde_json::Error) -> Self {
        Self::JsonError {
            path: path.into(),
            line: err.line(),
            column: err.column(),
            message: err.to_string(),
        }
    }

    /// Creates a file not found error.
    #[must_use]
    pub fn file_not_found(path: impl Into<String>) -> Self { Self::FileNotFound(path.into()) }

    /// Creates a glob pattern error.
    #[must_use]
    pub fn glob_pattern_error(pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self::GlobPatternError {
            pattern: pattern.into(),
            message: message.into(),
        }
    }

    /// Creates a validation error.
    #[must_use]
    pub fn validation_error(
        path: impl Into<String>,
        field: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::ValidationError {
            path: path.into(),
            field: field.into(),
            reason: reason.into(),
        }
    }
}

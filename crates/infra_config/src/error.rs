//! Configuration errors.

use thiserror::Error;

/// Errors that can occur during configuration loading.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// Configuration file not found.
    #[error("Configuration file not found: {path}")]
    FileNotFound {
        /// The file path that was not found.
        path: String,
    },

    /// Invalid configuration value.
    #[error("Invalid configuration value for '{key}': {message}")]
    InvalidValue {
        /// Configuration key that has invalid value.
        key: String,
        /// Description of validation failure.
        message: String,
    },

    /// Missing required configuration field.
    #[error("Missing required configuration: {field}")]
    MissingRequired {
        /// The name of the missing field.
        field: String,
    },

    /// Missing required field (static str variant).
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    /// Parse error with location details.
    #[error("Parse error in '{path}' at line {line}, column {column}: {message}")]
    ParseError {
        /// Path to the file with parse error.
        path: String,
        /// Line number (1-indexed).
        line: usize,
        /// Column number (1-indexed).
        column: usize,
        /// Error message.
        message: String,
    },

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvError(String),

    /// Underlying config crate error.
    #[error("Configuration error: {0}")]
    ConfigCrateError(String),
}

impl ConfigError {
    /// Creates a FileNotFound error.
    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Creates an InvalidValue error.
    pub fn invalid_value(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            key: key.into(),
            message: message.into(),
        }
    }

    /// Creates a MissingRequired error.
    pub fn missing_required(field: impl Into<String>) -> Self {
        Self::MissingRequired {
            field: field.into(),
        }
    }

    /// Creates a ParseError.
    pub fn parse_error(
        path: impl Into<String>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self::ParseError {
            path: path.into(),
            line,
            column,
            message: message.into(),
        }
    }

    /// Returns true if this is a file-related error.
    pub fn is_file_error(&self) -> bool {
        matches!(self, Self::FileNotFound { .. } | Self::ParseError { .. })
    }

    /// Returns true if this is a validation error.
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidValue { .. } | Self::MissingRequired { .. } | Self::MissingField(_)
        )
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(err: config::ConfigError) -> Self { Self::ConfigCrateError(err.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        // File errors
        assert!(ConfigError::file_not_found("x").is_file_error());
        assert!(ConfigError::parse_error("x", 1, 1, "y").is_file_error());
        assert!(!ConfigError::file_not_found("x").is_validation_error());

        // Validation errors
        assert!(ConfigError::invalid_value("k", "v").is_validation_error());
        assert!(ConfigError::missing_required("f").is_validation_error());
        assert!(ConfigError::MissingField("f").is_validation_error());
        assert!(!ConfigError::invalid_value("k", "v").is_file_error());
    }
}

//! Error types for the FrictionalBank demo.

use thiserror::Error;

/// Demo error type
#[derive(Debug, Error)]
pub enum DemoError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    /// Data loading error
    #[error("Data loading error: {0}")]
    DataLoad(String),

    /// Workflow execution error
    #[error("Workflow execution error: {0}")]
    Workflow(String),

    /// API communication error
    #[error("API communication error: {0}")]
    Api(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl DemoError {
    /// Create a data loading error
    pub fn data_load(msg: impl Into<String>) -> Self {
        Self::DataLoad(msg.into())
    }

    /// Create a workflow error
    pub fn workflow(msg: impl Into<String>) -> Self {
        Self::Workflow(msg.into())
    }

    /// Create an API error
    pub fn api(msg: impl Into<String>) -> Self {
        Self::Api(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DemoError::workflow("test error");
        assert!(err.to_string().contains("test error"));
    }
}

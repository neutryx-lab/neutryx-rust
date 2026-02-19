//! # Graph Error Types

use serde::Serialize;
use thiserror::Error;

/// Error type for graph extraction and processing operations.
#[derive(Error, Debug, Clone, Serialize)]
#[serde(tag = "error_type", content = "message")]
pub enum GraphError {
    /// The specified trade ID was not found.
    #[error("Trade '{0}' not found")]
    TradeNotFound(String),

    /// Graph extraction failed due to an internal error.
    #[error("Graph extraction failed: {0}")]
    ExtractionFailed(String),

    /// Graph extraction timed out (exceeded 500ms limit).
    #[error("Graph extraction timed out (exceeded 500ms limit)")]
    Timeout,
}

impl GraphError {
    /// Get the HTTP status code for this error.
    pub fn http_status_code(&self) -> u16 {
        match self {
            GraphError::TradeNotFound(_) => 404,
            GraphError::ExtractionFailed(_) => 500,
            GraphError::Timeout => 500,
        }
    }

    /// Get a human-readable error message.
    pub fn message(&self) -> String { self.to_string() }
}

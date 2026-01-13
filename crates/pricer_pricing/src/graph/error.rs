//! # Graph Error Types
//!
//! Error types for computation graph extraction and processing.
//!
//! ## HTTP Status Code Mapping
//!
//! Each error variant maps to an appropriate HTTP status code:
//! - `TradeNotFound` -> 404 Not Found
//! - `ExtractionFailed` -> 500 Internal Server Error
//! - `Timeout` -> 500 Internal Server Error (or 504 Gateway Timeout)

#[cfg(feature = "serde")]
use serde::Serialize;

// =============================================================================
// GraphError Enumeration (Task 1.3)
// =============================================================================

/// Error type for graph extraction and processing operations.
///
/// # HTTP Status Codes
///
/// Each variant maps to an HTTP status code for REST API responses:
/// - `TradeNotFound(String)` -> 404 Not Found
/// - `ExtractionFailed(String)` -> 500 Internal Server Error
/// - `Timeout` -> 500 Internal Server Error
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::GraphError;
///
/// let error = GraphError::TradeNotFound("T001".to_string());
/// assert_eq!(error.http_status_code(), 404);
///
/// let error = GraphError::ExtractionFailed("Memory limit exceeded".to_string());
/// assert_eq!(error.http_status_code(), 500);
///
/// let error = GraphError::Timeout;
/// assert_eq!(error.http_status_code(), 500);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "error_type", content = "message"))]
pub enum GraphError {
    /// The specified trade ID was not found.
    ///
    /// Maps to HTTP 404 Not Found.
    TradeNotFound(String),

    /// Graph extraction failed due to an internal error.
    ///
    /// Maps to HTTP 500 Internal Server Error.
    ExtractionFailed(String),

    /// Graph extraction timed out (exceeded 500ms limit).
    ///
    /// Maps to HTTP 500 Internal Server Error.
    Timeout,
}

impl GraphError {
    /// Get the HTTP status code for this error.
    ///
    /// # Returns
    ///
    /// - 404 for `TradeNotFound`
    /// - 500 for `ExtractionFailed` and `Timeout`
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::graph::GraphError;
    ///
    /// let error = GraphError::TradeNotFound("T001".to_string());
    /// assert_eq!(error.http_status_code(), 404);
    /// ```
    pub fn http_status_code(&self) -> u16 {
        match self {
            GraphError::TradeNotFound(_) => 404,
            GraphError::ExtractionFailed(_) => 500,
            GraphError::Timeout => 500,
        }
    }

    /// Get a human-readable error message.
    ///
    /// # Returns
    ///
    /// A descriptive error message suitable for API responses.
    pub fn message(&self) -> String {
        match self {
            GraphError::TradeNotFound(trade_id) => {
                format!("Trade '{}' not found", trade_id)
            }
            GraphError::ExtractionFailed(reason) => {
                format!("Graph extraction failed: {}", reason)
            }
            GraphError::Timeout => {
                "Graph extraction timed out (exceeded 500ms limit)".to_string()
            }
        }
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for GraphError {}

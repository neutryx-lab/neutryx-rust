//! Store errors.

use thiserror::Error;

/// Errors that can occur during storage operations.
#[derive(Error, Debug)]
pub enum StoreError {
    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Query error
    #[error("Query error: {0}")]
    QueryError(String),

    /// Serialisation error
    #[error("Serialisation error: {0}")]
    SerialisationError(String),

    /// Record not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Duplicate record
    #[error("Duplicate record: {0}")]
    Duplicate(String),

    /// Database error
    #[cfg(feature = "postgres")]
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

//! FpML parsing errors.

use thiserror::Error;

/// Errors that can occur during FpML parsing.
#[derive(Error, Debug)]
pub enum FpmlError {
    /// XML parsing error
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),

    /// Missing required element
    #[error("Missing required element: {0}")]
    MissingElement(String),

    /// Invalid element value
    #[error("Invalid value for element '{element}': {message}")]
    InvalidValue {
        /// Name of the element with invalid value
        element: String,
        /// Description of the validation failure
        message: String,
    },

    /// Unsupported FpML product type
    #[error("Unsupported FpML product type: {0}")]
    UnsupportedProduct(String),

    /// Date parsing error
    #[error("Date parsing error: {0}")]
    DateError(String),
}

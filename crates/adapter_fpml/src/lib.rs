//! # adapter_fpml
//!
//! FpML and XML trade definition parsers for Neutryx.
//!
//! This crate parses complex XML/FpML trade structures and maps FpML elements
//! to `pricer_models::Instrument` enums.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends on `pricer_core` (for types), `pricer_models` (for instrument
//! definitions), and `infra_master` (for identifiers).
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_fpml::FpmlParser;
//!
//! let xml = r#"<trade>...</trade>"#;
//! let instrument = FpmlParser::parse(xml)?;
//! ```

mod error;
mod parser;

pub use error::FpmlError;
pub use parser::FpmlParser;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{FpmlError, FpmlParser};
}

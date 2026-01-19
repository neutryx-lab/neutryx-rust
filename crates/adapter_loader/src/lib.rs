// Clippy configuration for adapter_loader
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! # adapter_loader
//!
//! Flat file loaders (CSV/Parquet) and CSA details for Neutryx.
//!
//! This crate handles bulk loading of CSV, JSON, or Parquet files,
//! and manages CSA (Credit Support Annex) terms, counterparty details,
//! and netting set configurations.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends only on `pricer_core` (for types) and `infra_master` (for
//! identifiers).
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_loader::CsvLoader;
//!
//! let trades = CsvLoader::load("trades.csv")?;
//! ```

mod csa;
mod csv_loader;
mod error;

pub use csa::{CsaTerms, NettingSetConfig};
pub use csv_loader::CsvLoader;
pub use error::LoaderError;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{CsaTerms, CsvLoader, LoaderError, NettingSetConfig};
}

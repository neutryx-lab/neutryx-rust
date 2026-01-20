// Clippy configuration for adapter_loader
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! # adapter_loader
//!
//! Flat file loaders (CSV/Parquet) for Neutryx.
//!
//! This crate handles bulk loading of CSV, JSON, or Parquet files.
//! CSA and netting set types are re-exported from `infra_master` for
//! backward compatibility.
//!
//! ## Architecture Position
//!
//! Part of the **A**dapter layer in the A-I-P-S architecture.
//! Depends only on `infra_master` (for master data types).
//!
//! ## Example
//!
//! ```rust,ignore
//! use adapter_loader::CsvLoader;
//!
//! let records = CsvLoader::load("trades.csv")?;
//! ```

mod csa;
mod csv_loader;
mod error;

pub use csa::{CsaTerms, NettingSet};
pub use csv_loader::CsvLoader;
pub use error::LoaderError;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{CsaTerms, CsvLoader, LoaderError, NettingSet};
}

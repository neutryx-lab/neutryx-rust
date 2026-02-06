//! Rate index definitions.
//!
//! This module provides benchmark rate index types:
//!
//! - [`RateIndex`]: Benchmark interest rate indices (SOFR, EURIBOR, etc.)
//! - [`IndexMetadata`]: Metadata for rate indices
//!
//! # Examples
//!
//! ```
//! use infra_master::market::index::RateIndex;
//! use infra_master::market::core::Currency;
//!
//! let sofr = RateIndex::Sofr;
//! assert_eq!(sofr.currency(), Currency::USD);
//! assert_eq!(sofr.code(), "SOFR");
//! ```

mod rate_index;

pub use rate_index::{IndexMetadata, RateIndex};

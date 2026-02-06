//! Financial index definitions.
//!
//! This module provides market index types for financial instruments:
//!
//! - [`RateIndex`]: Benchmark interest rate indices (SOFR, EURIBOR, etc.)
//! - [`FxIndex`]: FX fixing indices (ECB, WM/Reuters, BOJ, etc.)
//! - [`SwapIndex`]: Swap rate indices for CMS (Constant Maturity Swap)
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::index::{RateIndex, FxIndex, SwapIndex};
//! use infra_domain::market::core::Currency;
//! use infra_domain::time::Tenor;
//!
//! // Rate index
//! let sofr = RateIndex::Sofr;
//! assert_eq!(sofr.currency(), Currency::USD);
//!
//! // FX index
//! let ecb = FxIndex::EcbEurUsd;
//! assert_eq!(ecb.base_currency(), Currency::EUR);
//!
//! // Swap index (CMS)
//! let cms = SwapIndex::UsdCms10Y;
//! assert_eq!(cms.tenor(), Tenor::TenYears);
//! ```

mod fx_index;
mod rate_index;
mod swap_index;

pub use fx_index::{FxFixingSource, FxIndex, FxIndexMetadata};
pub use rate_index::{IndexMetadata, RateIndex};
pub use swap_index::{SwapIndex, SwapIndexMetadata};

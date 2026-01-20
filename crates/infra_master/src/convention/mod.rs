//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_master::convention::{SwapConvention, FxConvention};
//!
//! // Get USD SOFR swap convention
//! let usd_sofr = SwapConvention::usd_sofr();
//!
//! // Get EUR/USD FX convention
//! let eur_usd = FxConvention::eur_usd();
//! ```

mod swap;
mod fra;
mod futures;
mod capfloor;
mod fx;
mod bond;
mod cds;

pub use swap::{SwapConvention, SwapLegConvention};
pub use fra::FraConvention;
pub use futures::FuturesConvention;
pub use capfloor::CapFloorConvention;
pub use fx::FxConvention;
pub use bond::BondConvention;
pub use cds::CdsConvention;

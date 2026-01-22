//! Convention module for market conventions.
//!
//! This module provides types for representing market conventions
//! used in standardised financial instruments.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_master::trade::convention::{SwapConvention, FxConvention};
//!
//! // Get USD SOFR swap convention
//! let usd_sofr = SwapConvention::usd_sofr();
//!
//! // Get EUR/USD FX convention
//! let eur_usd = FxConvention::eur_usd();
//! ```

mod bond;
mod capfloor;
mod cds;
mod convention_set;
mod fra;
mod futures;
mod fx;
mod swap;

pub use bond::BondConvention;
pub use capfloor::CapFloorConvention;
pub use cds::CdsConvention;
pub use convention_set::ConventionSet;
pub use fra::FraConvention;
pub use futures::FuturesConvention;
pub use fx::FxConvention;
pub use swap::{SwapConvention, SwapLegConvention};

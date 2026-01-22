//! Convention module for market conventions.
//!
//! **Deprecated**: This module is deprecated. Use [`crate::trade::convention`] instead.
//!
//! This module re-exports types from [`crate::trade::convention`] for backward compatibility.
//! All convention types are now part of the trade module to better reflect their relationship
//! with trade structures.
//!
//! # Migration
//!
//! Update your imports from:
//! ```rust,ignore
//! use infra_master::convention::{SwapConvention, FxConvention};
//! ```
//!
//! To:
//! ```rust,ignore
//! use infra_master::trade::convention::{SwapConvention, FxConvention};
//! ```
//!
//! # Example (deprecated usage)
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

#![deprecated(
    since = "0.8.0",
    note = "Use `infra_master::trade::convention` instead. This module will be removed in 0.9.0."
)]

// Re-export all types from trade::convention for backward compatibility
pub use crate::trade::convention::{
    BondConvention, CapFloorConvention, CdsConvention, FraConvention, FuturesConvention,
    FxConvention, SwapConvention, SwapLegConvention,
};

//! Analytical (closed-form) solutions for option pricing.
//!
//! This module provides analytical formulas for path-dependent options
//! to verify Monte Carlo pricing accuracy.
//!
//! # Available Solutions
//!
//! - **Geometric Average Asian Options**: Kemna-Vorst (1990) closed-form
//! - **Barrier Options**: Merton (1973) / Rubinstein-Reiner (1991) formulas
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_pricing::analytical::{geometric_asian_call, geometric_asian_put};
//! use pricer_pricing::analytical::{down_out_call, up_in_put};
//!
//! // Geometric Asian option
//! let asian_price = geometric_asian_call(
//!     100.0,  // spot
//!     100.0,  // strike
//!     0.05,   // risk-free rate
//!     0.0,    // dividend yield
//!     0.2,    // volatility
//!     1.0,    // time to maturity
//! );
//!
//! // Barrier option
//! let barrier_price = down_out_call(
//!     100.0,  // spot
//!     100.0,  // strike
//!     90.0,   // barrier
//!     0.05,   // rate
//!     0.0,    // dividend
//!     0.2,    // volatility
//!     1.0,    // maturity
//! );
//! ```

pub mod asian;
pub mod barrier;

pub use asian::{
    geometric_asian_call, geometric_asian_put, GeometricAsianParams, GeometricAsianResult,
};

pub use barrier::{
    barrier_price, down_in_call, down_in_put, down_out_call, down_out_put, up_in_call, up_in_put,
    up_out_call, up_out_put, BarrierDirection, BarrierParams, BarrierResult, BarrierType,
    KnockType, OptionType,
};

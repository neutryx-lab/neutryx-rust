//! Analytical (closed-form) solutions for option pricing.
//!
//! This module provides analytical formulas for path-dependent options
//! to verify Monte Carlo pricing accuracy.
//!
//! # Available Solutions
//!
//! - **Geometric Average Asian Options**: Kemna-Vorst (1990) closed-form
//! - **Barrier Options**: Rubinstein-Reiner formulas (planned)
//!
//! # Usage
//!
//! ```rust,ignore
//! use pricer_kernel::analytical::{geometric_asian_call, geometric_asian_put};
//!
//! let price = geometric_asian_call(
//!     100.0,  // spot
//!     100.0,  // strike
//!     0.05,   // risk-free rate
//!     0.0,    // dividend yield
//!     0.2,    // volatility
//!     1.0,    // time to maturity
//! );
//! ```

pub mod asian;

pub use asian::{
    geometric_asian_call, geometric_asian_put, GeometricAsianParams, GeometricAsianResult,
};

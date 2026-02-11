//! Mathematical utilities for derivatives pricing.
//!
//! ## Submodules
//! - `smoothing`: Smooth approximations using LogSumExp and sigmoid functions
//! - `normal_dist`: Standard normal distribution (CDF, PDF, inverse CDF)
//! - `formulas`: Closed-form pricing formulas (GeneralisedBSM, Black-Scholes,
//!   Bachelier, SABR)
//! - `solvers`: Root-finding algorithms for numerical solving
//! - `numeric`: Numeric conversion utilities (`from_f64`)
//! - `linalg`: Linear algebra operations (requires `linalg` feature)
//! - `rng`: Random number generation (requires `rng` feature)

// Allow standard mathematical single-letter variable names (a, b, c, x, y, etc.)
// which are conventional in numerical computing and interpolation algorithms.
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]

pub mod formulas;
pub mod normal_dist;
pub mod numeric;
pub mod smoothing;
pub mod solvers;

#[cfg(feature = "linalg")]
pub mod linalg;

#[cfg(feature = "rng")]
pub mod rng;

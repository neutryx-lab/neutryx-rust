//! Mathematical utilities for derivatives pricing.
//!
//! ## Submodules
//! - `smoothing`: Smooth approximations using LogSumExp and sigmoid functions
//! - `normal_dist`: Standard normal distribution (CDF, PDF, inverse CDF)
//! - `formulas`: Closed-form pricing formulas (GeneralisedBSM, Black-Scholes,
//!   Bachelier, SABR)
//! - `solvers`: Root-finding algorithms for numerical solving
//! - `numeric`: Numeric conversion utilities (`from_f64`)
//! - `linalg`: Linear algebra operations
//! - `quadrature`: Numerical integration via Gaussian quadrature
//! - `rng`: Random number generation

// Allow standard mathematical single-letter variable names (a, b, c, x, y, etc.)
// which are conventional in numerical computing and interpolation algorithms.
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]

pub mod formulas;
pub mod interpolation;
pub mod normal_dist;
pub mod numeric;
pub mod quadrature;
pub mod smoothing;
pub mod solvers;

pub mod linalg;

pub mod rng;

//! Mathematical utilities and smooth approximations.
//!
//! This module provides differentiable smoothing functions that replace
//! discontinuous operations (max, min, abs, indicator) with smooth
//! approximations compatible with Enzyme automatic differentiation.
//!
//! ## Submodules
//! - `smoothing`: Smooth approximations using LogSumExp and sigmoid functions
//! - `interpolators`: Interpolation methods for curve and surface fitting
//! - `solvers`: Root-finding algorithms for numerical solving
//! - `distributions`: Probability distributions for financial calculations
//! - `calculus`: Numerical differentiation using finite differences
//! - `utilities`: Common mathematical utility functions
//! - `integrators`: Numerical integration methods (quadrature, ODE solvers)
//! - `optimisers`: Optimisation algorithms (Nelder-Mead, L-BFGS)
//! - `fitting`: Curve fitting and regression analysis
//! - `mesh`: Grid generation for numerical methods
//! - `linalg`: Linear algebra operations (requires `linalg` feature)
//! - `rng`: Random number generation (requires `rng` feature)

// Allow standard mathematical single-letter variable names (a, b, c, x, y, etc.)
// which are conventional in numerical computing and interpolation algorithms.
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]

pub mod calculus;
pub mod distributions;
pub mod fitting;
pub mod integrators;
pub mod interpolators;
pub mod mesh;
pub mod numeric;
pub mod optimisers;
pub mod smoothing;
pub mod solvers;
pub mod utilities;

#[cfg(feature = "linalg")]
pub mod linalg;

#[cfg(feature = "rng")]
pub mod rng;

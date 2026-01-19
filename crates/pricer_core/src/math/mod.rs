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

// Allow standard mathematical single-letter variable names (a, b, c, x, y, etc.)
// which are conventional in numerical computing and interpolation algorithms.
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]

pub mod interpolators;
pub mod numeric;
pub mod smoothing;
pub mod solvers;

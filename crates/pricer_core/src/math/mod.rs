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

pub mod interpolators;
pub mod numeric;
pub mod smoothing;
pub mod solvers;

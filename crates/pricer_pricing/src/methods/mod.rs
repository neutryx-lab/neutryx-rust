//! Pricing methods for financial derivatives.
//!
//! This module contains the core pricing implementations:
//! - [`mc`] - Monte Carlo simulation with AD integration
//! - [`tree`] - Binomial/Trinomial tree methods
//! - [`path_dependent`] - Path-dependent option infrastructure

/// Monte Carlo pricing with Enzyme AD integration.
pub mod mc;

/// Tree-based pricing methods (Binomial/Trinomial).
pub mod tree;

/// Path-dependent option infrastructure (Asian, Barrier, Lookback).
pub mod path_dependent;

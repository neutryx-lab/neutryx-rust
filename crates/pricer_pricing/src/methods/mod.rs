//! Pricing methods for financial derivatives.

/// Monte Carlo pricing with Enzyme AD integration.
pub mod mc;

/// Path-dependent option payoffs (Asian, barrier, lookback).
pub mod path_dependent;

/// Tree-based pricing methods (Binomial/Trinomial).
pub mod tree;

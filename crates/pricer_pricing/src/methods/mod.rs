//! Pricing methods for financial derivatives.

/// Monte Carlo pricing with Enzyme AD integration.
pub mod mc;

/// Tree-based pricing methods (Binomial/Trinomial).
pub mod tree;

/// Path-dependent option infrastructure (Asian, Barrier, Lookback).
pub mod path_dependent;

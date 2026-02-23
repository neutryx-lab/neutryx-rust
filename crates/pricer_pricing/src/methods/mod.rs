//! Pricing methods for financial derivatives.

/// Monte Carlo pricing with Enzyme AD integration.
pub mod mc;

/// Monte Carlo XVA simulation infrastructure.
pub mod mc_xva;

/// Tree-based pricing methods (Binomial/Trinomial).
pub mod tree;

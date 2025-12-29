//! # Pricer Models (L2: Business Logic)
//!
//! Financial instruments, payoff functions, and stochastic models.
//!
//! This crate provides:
//! - Instrument definitions (vanilla options, barriers, swaps, etc.)
//! - Payoff functions with smooth approximations
//! - Stochastic models (GBM, Heston, etc.)
//! - Market data structures (curves, surfaces)
//! - Analytical formulas for validation
//!
//! ## Design Principles
//!
//! - **Enum-based instruments** for static dispatch (Enzyme-friendly)
//! - **Per-instrument smoothing epsilon** for configurable differentiability
//! - **Builder pattern** for ergonomic API with sensible defaults

#![warn(missing_docs)]

pub mod analytical;
pub mod instruments;
pub mod models;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

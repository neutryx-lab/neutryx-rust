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
//! - Model calibration to market data
//!
//! ## Design Principles
//!
//! - **Enum-based instruments** for static dispatch (Enzyme-friendly)
//! - **Per-instrument smoothing epsilon** for configurable differentiability
//! - **Builder pattern** for ergonomic API with sensible defaults

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod analytical;
pub mod calibration;
pub mod instruments;
pub mod models;
pub mod schedules;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

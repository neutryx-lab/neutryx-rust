//! # Pricer Core (L1: Foundation)
//!
//! Core mathematical types, traits, and utilities for the Neutryx pricing library.
//!
//! This crate provides:
//! - Dual-mode numeric types (Enzyme vs num-dual)
//! - Smooth approximations for discontinuous functions
//! - Date/time handling with day count conventions
//! - Core traits for priceable instruments
//!
//! ## Feature Flags
//!
//! - `num-dual-mode` (default): Use num-dual for automatic differentiation (verification mode)
//! - `enzyme-mode`: Use f64 directly (Enzyme handles AD at LLVM level)

#![warn(missing_docs)]

pub mod math;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

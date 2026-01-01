//! # pricer_core: Mathematical Foundation for XVA Pricing Library
//!
//! ## Layer 1 (Foundation) Role
//!
//! pricer_core serves as the bottom layer of the 4-layer architecture, providing:
//! - Differentiable smoothing functions (`math::smoothing`)
//! - Dual number type integration (`types::dual`)
//! - Traits for pricing and differentiability (`traits`)
//! - Time types: `Date`, `DayCountConvention` (`types::time`)
//! - Currency types: `Currency` (`types::currency`)
//! - Error types: `PricingError`, `DateError`, `CurrencyError` (`types::error`)
//!
//! ## Zero Dependency Principle
//!
//! Layer 1 has no dependencies on other pricer_* crates, with minimal external dependencies:
//! - num-traits: Traits for generic numerical computation
//! - num-dual: Dual number types and automatic differentiation (optional)
//! - chrono: Date arithmetic
//! - serde: Serialisation support (optional)
//!
//! ## Stable Rust Toolchain
//!
//! Layer 1 can be built with stable Rust only (nightly not required).
//! The Enzyme AD engine is isolated in Layer 3.
//!
//! ## Usage Examples
//!
//! ```rust
//! use pricer_core::math::smoothing::smooth_max;
//! use pricer_core::types::{Date, DayCountConvention, Currency};
//!
//! // Date operations
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//! let year_fraction = DayCountConvention::ActualActual365.year_fraction_dates(start, end);
//!
//! // Currency information
//! let usd = Currency::USD;
//! assert_eq!(usd.code(), "USD");
//! assert_eq!(usd.decimal_places(), 2);
//!
//! // Computation with f64
//! let result = smooth_max(3.0_f64, 5.0_f64, 1e-6_f64);
//! # assert!((result - 5.0_f64).abs() < 1e-3);
//! ```
//!
//! ## Feature Flags
//!
//! - `num-dual-mode` (default): Use num-dual for automatic differentiation (verification mode)
//! - `enzyme-mode`: Use f64 directly (Enzyme handles AD at LLVM level)
//! - `serde` (default): Enable serialisation for Date, Currency, DayCountConvention

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod market_data;
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

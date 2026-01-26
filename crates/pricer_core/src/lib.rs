// Clippy configuration for pricer_core
// Pedantic lints that are too strict for mathematical/financial libraries:
// - doc_markdown: Type names in docs are common
// - missing_errors_doc: Error conditions are often obvious from Result types
// - must_use_candidate: Many simple getters don't need #[must_use]
// - return_self_not_must_use: Builder methods are common
// - cast_precision_loss: Controlled in numerical code
// - redundant_closure_for_method_calls: Clarity vs brevity
// - explicit_iter_loop: Explicit iteration is sometimes clearer
// - manual_let_else: Readability preference
// - single_match_else: Readability preference
// - inconsistent_struct_constructor: Field order doesn't affect correctness
// - match_same_arms: Sometimes kept separate for documentation
// - unused_self: Sometimes needed for API consistency
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::single_match_else)]
#![allow(clippy::inconsistent_struct_constructor)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::unused_self)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::no_effect_underscore_binding)]

//! # pricer_core: Mathematical Foundation for XVA Pricing Library
//!
//! ## Layer 1 (Foundation) Role
//!
//! pricer_core serves as the bottom layer of the 4-layer architecture,
//! providing:
//! - Differentiable smoothing functions (`math::smoothing`)
//! - Dual number type integration (`types::dual`)
//! - Traits for pricing and differentiability (`traits`)
//! - Time types: `Date`, `DayCounter`, `BusinessDayConvention` (re-exported
//!   from `infra_master`)
//! - Currency types: `Currency` (re-exported from `infra_master`)
//! - Convenience: `DayCountConvention` wrapper for common day count conventions
//! - Error types: `PricingError`, `DateError`, `CurrencyError` (`types::error`)
//!
//! ## Zero Dependency Principle
//!
//! Layer 1 has no dependencies on other pricer_* crates, with minimal external
//! dependencies:
//! - num-traits: Traits for generic numerical computation
//! - num-dual: Dual number types and automatic differentiation (optional)
//! - infra_master: Authoritative source for Date, Currency, DayCounter
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
//! use infra_master::{Date, Currency, DayCounter};
//!
//! // Date operations (from infra_master)
//! let start = Date::from_ymd(2024, 1, 1).unwrap();
//! let end = Date::from_ymd(2024, 7, 1).unwrap();
//! let year_fraction = DayCounter::Actual365Fixed.year_fraction(start, end);
//!
//! // Currency information (from infra_master)
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
//! - `num-dual-mode` (default): Use num-dual for automatic differentiation
//!   (verification mode)
//! - `enzyme-mode`: Use f64 directly (Enzyme handles AD at LLVM level)
//! - `serde` (default): Enable serialisation for Date, Currency, DayCounter

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod ir;
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

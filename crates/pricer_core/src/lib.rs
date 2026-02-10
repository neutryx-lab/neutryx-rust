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

//! # pricer_core: Mathematical Foundation for Derivatives Pricing
//!
//! ## Layer 1 (Foundation) Role
//!
//! pricer_core serves as the bottom layer of the 4-layer architecture,
//! providing:
//! - Closed-form pricing formulas (`math::formulas`): GeneralisedBSM,
//!   Black-Scholes, Garman-Kohlhagen, Bachelier, SABR, forward pricing
//! - Standard normal distribution (`math::normal_dist`): CDF, PDF, inverse CDF
//! - Differentiable smoothing functions (`math::smoothing`)
//! - Numerical solvers (`math::solvers`): Newton-Raphson, Levenberg-Marquardt
//! - SoA kernel IR (`kernel`): aligned buffers, cashflow/callable kernels
//! - Traits for pricing and differentiability (`traits`)
//! - Error types: `PricingError`, `CalibrationError`, `SolverError`
//!   (`types::error`)
//!
//! ## Zero Dependency Principle
//!
//! Layer 1 has no dependencies on other pricer_* crates, with minimal external
//! dependencies:
//! - num-traits: Traits for generic numerical computation
//! - infra_domain: Authoritative source for Date, Currency, DayCounter
//! - serde: Serialisation support (optional)
//!
//! ## Usage Examples
//!
//! ```rust
//! use pricer_core::math::formulas::GeneralisedBSM;
//!
//! // Black-Scholes case: b = r
//! let bsm = GeneralisedBSM::new(100.0_f64, 100.0, 0.05, 0.05, 0.2, 1.0).unwrap();
//! let call = bsm.price(true);
//! let put = bsm.price(false);
//! assert!(call > 0.0 && put > 0.0);
//! ```
//!
//! ## Feature Flags
//!
//! - `serde` (default): Enable serialisation
//! - `equity` (default): Enable equity-specific kernel types
//! - `parallel` (default): Enable parallel computation via rayon
//! - `rng`: Random number generation (Sobol, Mersenne Twister)
//! - `linalg`: Linear algebra operations via nalgebra

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod kernel;
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

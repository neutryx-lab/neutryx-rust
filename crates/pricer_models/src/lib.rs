// Clippy configuration for pricer_models
// - unreadable_literal: Mathematical constants like 0.254829592 are from papers
// - similar_names: sorted_xs/sorted_ys are standard patterns
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::similar_names)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_else)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::unused_self)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::if_not_else)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::struct_field_names)]

//! # Pricer Models (L2: Business Logic)
//!
//! Financial instruments, payoff functions, and stochastic models.
//!
//! This crate provides:
//! - Instrument definitions (vanilla options, barriers, swaps, etc.)
//! - Payoff functions with smooth approximations
//! - Stochastic models (GBM, Heston, etc.)
//! - Yield curve bootstrapping and calibration
//! - Analytical pricing with instrument integration
//! - Model calibration to market data
//!
//! ## Module Organisation
//!
//! - [`stochastic`] - Stochastic process models (GBM, Heston, Hull-White, CIR)
//! - [`analytic`] - Analytical pricing with instrument wrappers
//! - [`calibration`] - Yield curve bootstrapping and market data calibration
//! - [`compiler`] - Instrument compilation for pricing engines
//!
//! ## Analytical Formulas
//!
//! Pure mathematical formulas are in `pricer_core::math::formulas`.
//! The [`analytic`] module re-exports these and adds instrument integration.
//!
//! ## Design Principles
//!
//! - **Enum-based instruments** for static dispatch (Enzyme-friendly)
//! - **Per-instrument smoothing epsilon** for configurable differentiability
//! - **Builder pattern** for ergonomic API with sensible defaults

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod analytic;
pub mod compiler;
mod direction_ext;
pub mod calibration;
pub mod stochastic;

// Re-export instrument types from infra_master for backwards compatibility
// This replaces the former pricer_models::instruments module
pub mod instruments {
    //! Lightweight instrument definitions for analytical pricing models.
    //!
    //! This module re-exports types from `infra_master::trade` for backwards
    //! compatibility. For full instrument definitions, see
    //! `infra_master::trade`.

    pub use infra_master::trade::{
        ExerciseStyle, Forward, ForwardDirection as Direction, FxOptionType, InstrumentParams,
        PayoffType, PricingInstrument as Instrument, VanillaOption,
    };
}

// Re-export extension traits for direction types (infra_master types not
// re-exported)
pub use direction_ext::{SwapDirectionExt, TradeDirectionExt};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

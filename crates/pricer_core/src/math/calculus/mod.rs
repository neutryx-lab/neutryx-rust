//! Numerical calculus operations.
//!
//! This module provides numerical differentiation using finite difference
//! methods. These are useful when automatic differentiation is not available
//! or as a fallback for verification.
//!
//! ## Available Methods
//!
//! - **Forward difference**: O(h) accuracy
//! - **Backward difference**: O(h) accuracy
//! - **Central difference**: O(h²) accuracy
//! - **Second derivative**: O(h²) accuracy
//! - **Partial derivatives**: For multivariate functions
//!
//! ## Usage
//!
//! ```
//! use pricer_core::math::calculus::{finite_diff, DifferenceType};
//!
//! let f = |x: f64| x * x;
//! let x = 2.0;
//! let h = 1e-5;
//!
//! let derivative = finite_diff(&f, x, h, DifferenceType::Central);
//! assert!((derivative - 4.0).abs() < 1e-8);
//! ```
//!
//! ## Precision Considerations
//!
//! The optimal step size balances truncation error (large h) against
//! round-off error (small h). Use [`suggest_bump_size`] for automatic
//! step size selection.

mod bump_selection;
mod finite_difference;

pub use bump_selection::{suggest_bump_size, suggest_bump_size_second};
pub use finite_difference::{
    finite_diff, finite_diff_second, partial_diff, partial_diff_second, DifferenceType,
};

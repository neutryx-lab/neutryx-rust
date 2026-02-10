//! Global curve bootstrapping with simultaneous discount factor calibration.
//!
//! This module provides `GlobalBootstrapper<T>` which calibrates all discount
//! factors simultaneously by solving a system of nonlinear equations where
//! each equation represents the pricing error for a market instrument.
//!
//! ## Key Features
//!
//! - Simultaneous calibration of all discount factors
//! - Jacobian inverse storage for AAD sensitivity computation
//! - Supports all instrument types via `CalibrationInstrument` trait
//! - Configurable convergence tolerances
//!
//! ## Comparison with Sequential Bootstrapping
//!
//! | Aspect | Sequential | Global |
//! |--------|-----------|--------|
//! | Speed | O(n) solves | O(1) solve, O(n³) per iteration |
//! | AAD | Requires implicit function theorem per pillar | Single J⁻¹ captures all sensitivities |
//! | Flexibility | Forward-starting only | Any instrument structure |
//! | Stability | Stable for well-ordered instruments | May require damping for ill-conditioned |

mod bootstrapper;
mod config;
mod result;

#[cfg(test)]
#[allow(non_snake_case)]
mod tests;

pub use bootstrapper::GlobalBootstrapper;
pub use config::GlobalBootstrapConfig;
pub use result::GlobalBootstrapResult;

use num_traits::Float;

/// Compute the Euclidean norm of a vector.
fn vector_norm<T: Float>(v: &[T]) -> T {
    let sum_sq = v.iter().fold(T::zero(), |acc, &x| acc + x * x);
    sum_sq.sqrt()
}

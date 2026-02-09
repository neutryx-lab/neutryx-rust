//! Curve calibration module.
//!
//! This module provides yield curve calibration algorithms:
//!
//! - **Sequential bootstrapping** ([`CurveBootstrapper`]): Solve one pillar at
//!   a time
//! - **Global calibration** (`GlobalBootstrapper`): Solve all pillars
//!   simultaneously (feature-gated)
//!
//! ## Choosing a Method
//!
//! | Aspect | Sequential | Global |
//! |--------|------------|--------|
//! | Speed | O(n) solves | O(1) solve, O(n³) per iteration |
//! | Robustness | May fail with overlapping instruments | Handles any structure |
//! | AAD | Per-pillar implicit differentiation | Full J⁻¹ available |
//! | Use case | Simple curves, real-time | Complex curves, risk systems |

mod bootstrap;

#[cfg(feature = "global-bootstrap")]
mod global;

// =============================================================================
// Re-exports
// =============================================================================

pub use bootstrap::{BootstrapConfig, CurveBootstrapper, InterpolationMethod, JacobianMatrix};
#[cfg(feature = "global-bootstrap")]
pub use global::{GlobalBootstrapConfig, GlobalBootstrapResult, GlobalBootstrapper};

// Re-export JacobianMethod from problem module for convenience
#[cfg(feature = "global-bootstrap")]
pub use super::problem::JacobianMethod;

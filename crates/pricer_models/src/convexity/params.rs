//! Configuration parameters and enums for convexity adjustment.

use pricer_core::math::numeric::from_f64;
use pricer_core::traits::Float;
use serde::{Deserialize, Serialize};

/// CMS convexity adjustment calculation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConvexityAdjustCalcMethod {
    /// Full numerical integration (Gauss-Kronrod adaptive quadrature).
    NumericalIntegration,
    /// Closed-form under Normal (Bachelier) dynamics.
    NormalAnalytic,
    /// Closed-form under Shifted Log-Normal dynamics.
    SlnAnalytic,
}

/// Configuration parameters for the convexity adjuster.
#[derive(Debug, Clone)]
pub struct ConvexityAdjusterParams<T: Float> {
    /// Upper integration bound in sigma multiples (default 5.0).
    pub upper_limit_sigma: T,
    /// Lower integration bound in sigma multiples (default -5.0).
    pub lower_limit_sigma: T,
    /// Grace period in days for deciding whether adjustment applies (default 14).
    pub grace_period_days: i32,
    /// Convergence tolerance for adaptive integration (default 1e-7).
    pub integral_tolerance: T,
    /// Time-value tolerance for early termination (default 1e-10).
    pub time_value_tolerance: T,
}

impl<T: Float> Default for ConvexityAdjusterParams<T> {
    fn default() -> Self {
        Self {
            upper_limit_sigma: from_f64(5.0),
            lower_limit_sigma: from_f64(-5.0),
            grace_period_days: 14,
            integral_tolerance: from_f64(1e-7),
            time_value_tolerance: from_f64(1e-10),
        }
    }
}

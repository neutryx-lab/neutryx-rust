//! Interpolation algorithms for curve construction.
//!
//! ## Submodules
//! - `cubic_spline`: Natural cubic spline interpolation
//! - `monotone_convex`: Hagan-West (2006) monotone convex interpolation
//! - `tension_spline`: Tension spline with sinh/cosh basis functions

pub mod cubic_spline;
pub mod monotone_convex;
pub mod tension_spline;

pub use cubic_spline::CubicSpline;
pub use monotone_convex::MonotoneConvexInterpolator;
pub use tension_spline::TensionSpline;

use std::fmt;

/// Error type for interpolation operations.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationError {
    /// Insufficient data points for interpolation.
    InsufficientData { required: usize, provided: usize },
    /// Knot x-values are not strictly increasing.
    NonIncreasingKnots,
    /// Query point is outside the interpolation domain.
    OutOfRange { x: f64, min: f64, max: f64 },
    /// Numerical issue during coefficient computation.
    NumericalFailure(String),
}

impl fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData { required, provided } => {
                write!(f, "need at least {required} points, got {provided}")
            }
            Self::NonIncreasingKnots => write!(f, "knot x-values must be strictly increasing"),
            Self::OutOfRange { x, min, max } => {
                write!(f, "x = {x} outside [{min}, {max}]")
            }
            Self::NumericalFailure(msg) => write!(f, "numerical failure: {msg}"),
        }
    }
}

impl std::error::Error for InterpolationError {}

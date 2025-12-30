//! Interpolation methods for numerical computation.
//!
//! This module provides a collection of interpolation algorithms designed for
//! financial applications, with full support for automatic differentiation
//! through generic `T: Float` type parameters.
//!
//! ## Available Interpolators
//!
//! - [`LinearInterpolator`]: Piecewise linear interpolation between data points
//! - [`CubicSplineInterpolator`]: Natural cubic spline with C² continuity
//! - [`MonotonicInterpolator`]: Fritsch-Carlson monotonicity-preserving interpolation
//! - [`BilinearInterpolator`]: 2D grid interpolation for surfaces
//! - [`smooth_interp`]: Branch-free smooth interpolation for Enzyme AD compatibility
//!
//! ## Core Trait
//!
//! All 1D interpolators implement the [`Interpolator`] trait, which defines:
//! - `interpolate(x: T) -> Result<T, InterpolationError>`: Compute interpolated value
//! - `domain() -> (T, T)`: Return valid interpolation range
//!
//! ## AD Compatibility
//!
//! All interpolators are generic over `T: num_traits::Float`, enabling use with:
//! - `f64`: Standard floating-point computation
//! - `Dual64`: Automatic differentiation via num-dual
//!
//! ## Example
//!
//! ```
//! use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
//!
//! let xs = [0.0, 1.0, 2.0, 3.0];
//! let ys = [0.0, 1.0, 4.0, 9.0];
//!
//! let interp = LinearInterpolator::new(&xs, &ys).unwrap();
//! let (x_min, x_max) = interp.domain();
//! assert_eq!(x_min, 0.0);
//! assert_eq!(x_max, 3.0);
//!
//! // Interpolate at x = 1.5 (between y=1.0 and y=4.0)
//! let y = interp.interpolate(1.5).unwrap();
//! assert!((y - 2.5).abs() < 1e-10);
//! ```

mod bilinear;
mod cubic_spline;
mod linear;
mod monotonic;
mod smooth_interp;
mod traits;

// Re-export public types at module level
pub use bilinear::BilinearInterpolator;
pub use cubic_spline::CubicSplineInterpolator;
pub use linear::LinearInterpolator;
pub use monotonic::MonotonicInterpolator;
pub use smooth_interp::smooth_interp;
pub use traits::Interpolator;

//! Dual number type integration for automatic differentiation.
//!
//! This module provides a type alias for num-dual's Dual64 type,
//! enabling forward-mode automatic differentiation for verification
//! against Enzyme's LLVM-level AD (implemented in Layer 3).
//!
//! ## Usage
//!
//! ```ignore
//! use pricer_core::types::dual::DualNumber;
//! use pricer_core::math::smoothing::smooth_max;
//!
//! // Create dual numbers
//! let a = DualNumber::from(3.0).derivative();  // da/da = 1.0
//! let b = DualNumber::from(5.0);                // db/da = 0.0
//!
//! // Compute smooth_max with automatic differentiation
//! let result = smooth_max(a, b, 1e-6);
//!
//! // Extract value and gradient
//! let value = result.re();      // ≈ 5.0
//! let gradient = result.eps;    // ∂smooth_max/∂a
//! ```

/// Type alias for num-dual's Dual64 (f64-based dual numbers).
///
/// This type supports first-order automatic differentiation with:
/// - `re()`: Real part (function value)
/// - `eps`: Dual part (derivative/gradient)
///
/// # Integration with Smoothing Functions
///
/// NOTE: `DualNumber` (`Dual64`) does NOT implement `num_traits::Float`.
/// To use smoothing functions with AD, they need to be refactored to use
/// a more permissive trait bound (e.g., `DualNum<f64>` or a custom `Scalar` trait).
///
/// Example of intended usage (requires trait bound refactoring):
///
/// ```ignore
/// # #[cfg(feature = "num-dual-mode")]
/// # {
/// use pricer_core::types::dual::DualNumber;
/// use pricer_core::math::smoothing::{smooth_max, smooth_min, smooth_abs, smooth_indicator};
///
/// let x = DualNumber::from(2.0).derivative();
/// let y = DualNumber::from(3.0);
///
/// // All smoothing functions propagate gradients
/// let max_result = smooth_max(x, y, 1e-6);
/// let min_result = smooth_min(x, y, 1e-6);
/// let abs_result = smooth_abs(x, 1e-6);
/// let ind_result = smooth_indicator(x, 1e-6);
/// # }
/// ```
#[cfg(feature = "num-dual-mode")]
pub type DualNumber = num_dual::Dual64;

//! Enzyme autodiff bindings for pricer_kernel.
//!
//! This module provides the interface for Enzyme LLVM-level automatic differentiation.
//! Enzyme operates at the LLVM IR level, enabling high-performance gradient computation
//! for financial derivative pricing.
//!
//! # Phase 3.0 Status
//!
//! This is the **Phase 3.0 placeholder implementation**. The module structure and API
//! are defined, but actual Enzyme integration will be completed in Phase 4.
//!
//! Current implementation uses finite difference approximation for gradient computation.
//! Phase 4 will replace this with actual Enzyme `#[autodiff]` macros.
//!
//! # Phase 4 Integration
//!
//! Phase 4 will enable:
//! ```rust,ignore
//! #![feature(autodiff)]
//!
//! #[autodiff_reverse(df, Active)]
//! pub fn f(x: f64) -> f64 {
//!     x * x
//! }
//! ```
//!
//! # Usage
//!
//! ```rust
//! use pricer_kernel::enzyme::{Activity, gradient};
//!
//! // Define a function to differentiate
//! fn square(x: f64) -> f64 {
//!     x * x
//! }
//!
//! // Compute gradient (Phase 3.0: finite difference approximation)
//! let grad = gradient(square, 3.0);
//! assert!((grad - 6.0).abs() < 1e-6);
//! ```

/// Activity annotations for autodiff parameters.
///
/// These annotations specify how each parameter participates in differentiation:
///
/// - `Const`: The parameter is not differentiated (treated as constant)
/// - `Dual`: Forward mode - carries tangent value alongside primal
/// - `Active`: Reverse mode - accumulates gradients during backward pass
/// - `Duplicated`: Reverse mode - parameter has a separate shadow for gradients
///
/// # Phase 4 Usage
///
/// ```rust,ignore
/// #[autodiff_forward(df, Dual, Const, Dual)]
/// pub fn f(x: &[f64], y: f64) -> f64 { ... }
///
/// #[autodiff_reverse(df, Duplicated, Const, Active)]
/// pub fn g(x: &mut [f64], y: f64) -> f64 { ... }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    /// Parameter is constant (not differentiated).
    ///
    /// Use for parameters that do not affect the derivative,
    /// such as configuration values or fixed constants.
    Const,

    /// Parameter carries dual/tangent value (forward mode).
    ///
    /// In forward mode AD, each variable carries both its primal value
    /// and its tangent (derivative with respect to input).
    Dual,

    /// Parameter is active (reverse mode accumulation).
    ///
    /// In reverse mode AD, active parameters accumulate gradients
    /// during the backward pass. Use for scalar outputs.
    Active,

    /// Parameter is duplicated with shadow (reverse mode).
    ///
    /// The parameter has a separate shadow buffer where gradients
    /// are accumulated. Use for arrays or mutable buffers.
    Duplicated,

    /// Shadow only, no primal value (reverse mode).
    ///
    /// Only the shadow gradient buffer is used; primal value is ignored.
    /// Useful for output-only gradient buffers.
    DuplicatedOnly,
}

impl Activity {
    /// Returns true if this activity participates in differentiation.
    #[inline]
    pub fn is_active(&self) -> bool {
        !matches!(self, Activity::Const)
    }

    /// Returns true if this is a reverse mode activity.
    #[inline]
    pub fn is_reverse_mode(&self) -> bool {
        matches!(
            self,
            Activity::Active | Activity::Duplicated | Activity::DuplicatedOnly
        )
    }

    /// Returns true if this is a forward mode activity.
    #[inline]
    pub fn is_forward_mode(&self) -> bool {
        matches!(self, Activity::Dual)
    }
}

/// Compute gradient of function `f` at point `x` using Enzyme.
///
/// # Phase 3.0 Implementation
///
/// This is a **placeholder implementation** using central finite difference:
/// ```text
/// f'(x) ≈ (f(x + h) - f(x - h)) / (2h)
/// ```
///
/// The finite difference step size `h` is set to `1e-8` which provides
/// a good balance between truncation and rounding errors for most functions.
///
/// # Phase 4 Implementation
///
/// Phase 4 will replace this with actual Enzyme autodiff:
/// ```rust,ignore
/// #[autodiff_reverse(gradient_impl, Active)]
/// pub fn gradient<F>(f: F, x: f64) -> f64
/// where
///     F: Fn(f64) -> f64;
/// ```
///
/// # Arguments
///
/// * `f` - The function to differentiate. Must be a closure or function
///   that takes `f64` and returns `f64`.
/// * `x` - The point at which to evaluate the gradient.
///
/// # Returns
///
/// The gradient (derivative) of `f` at point `x`.
///
/// # Examples
///
/// ```rust
/// use pricer_kernel::enzyme::gradient;
///
/// // Gradient of x^2 is 2x
/// let grad = gradient(|x| x * x, 3.0);
/// assert!((grad - 6.0).abs() < 1e-6);
///
/// // Gradient of sin(x) is cos(x)
/// let grad = gradient(|x| x.sin(), 0.0);
/// assert!((grad - 1.0).abs() < 1e-6);
/// ```
#[inline]
pub fn gradient<F>(f: F, x: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    // Phase 3.0: Finite difference approximation
    // Phase 4: Replace with Enzyme autodiff
    const H: f64 = 1e-8;
    (f(x + H) - f(x - H)) / (2.0 * H)
}

/// Compute gradient of function `f` at point `x` with custom step size.
///
/// Similar to [`gradient`], but allows specifying the finite difference step size.
/// This is useful for functions with very large or small values where the
/// default step size may not be appropriate.
///
/// # Arguments
///
/// * `f` - The function to differentiate.
/// * `x` - The point at which to evaluate the gradient.
/// * `h` - The finite difference step size.
///
/// # Returns
///
/// The gradient (derivative) of `f` at point `x`.
#[inline]
pub fn gradient_with_step<F>(f: F, x: f64, h: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    (f(x + h) - f(x - h)) / (2.0 * h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_activity_is_active() {
        assert!(!Activity::Const.is_active());
        assert!(Activity::Dual.is_active());
        assert!(Activity::Active.is_active());
        assert!(Activity::Duplicated.is_active());
        assert!(Activity::DuplicatedOnly.is_active());
    }

    #[test]
    fn test_activity_mode_detection() {
        assert!(Activity::Dual.is_forward_mode());
        assert!(!Activity::Dual.is_reverse_mode());

        assert!(!Activity::Active.is_forward_mode());
        assert!(Activity::Active.is_reverse_mode());

        assert!(Activity::Duplicated.is_reverse_mode());
        assert!(Activity::DuplicatedOnly.is_reverse_mode());
    }

    #[test]
    fn test_gradient_square() {
        // f(x) = x^2, f'(x) = 2x
        let grad = gradient(|x| x * x, 3.0);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_cubic() {
        // f(x) = x^3, f'(x) = 3x^2
        let grad = gradient(|x| x * x * x, 2.0);
        assert_relative_eq!(grad, 12.0, epsilon = 1e-5);
    }

    #[test]
    fn test_gradient_sin() {
        // f(x) = sin(x), f'(x) = cos(x)
        let grad = gradient(|x| x.sin(), 0.0);
        assert_relative_eq!(grad, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_with_custom_step() {
        let grad = gradient_with_step(|x| x * x, 3.0, 1e-6);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-4);
    }
}

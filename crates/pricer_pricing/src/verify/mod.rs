//! Enzyme gradient verification utilities.
//!
//! This module provides simple functions to verify that Enzyme automatic differentiation
//! works correctly. Phase 3.0 uses placeholder implementations; actual Enzyme integration
//! will be added in Phase 4.
//!
//! ## Purpose
//!
//! - Verify Enzyme LLVM-level AD infrastructure
//! - Validate gradient calculations against analytical derivatives
//! - Provide extensible framework for future verification functions
//!
//! ## Usage
//!
//! ```rust
//! use pricer_pricing::verify::{square, square_gradient};
//!
//! let x = 3.0;
//! let value = square(x);
//! let gradient = square_gradient(x);
//!
//! assert_eq!(value, 9.0);
//! assert!((gradient - 6.0).abs() < 1e-10); // d(x²)/dx = 2x
//! ```

/// Simple square function for Enzyme verification.
///
/// # Mathematical Definition
///
/// ```text
/// f(x) = x²
/// ```
///
/// # Arguments
///
/// * `x` - Input value
///
/// # Returns
///
/// The square of the input: x²
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::verify::square;
///
/// assert_eq!(square(3.0), 9.0);
/// assert_eq!(square(-2.5), 6.25);
/// assert_eq!(square(0.0), 0.0);
/// ```
#[inline]
pub fn square(x: f64) -> f64 {
    x * x
}

/// Calculate gradient of square function.
///
/// # Mathematical Definition
///
/// ```text
/// f(x) = x²
/// f'(x) = 2x
/// ```
///
/// # Phase 3.0 Implementation
///
/// This is a **placeholder implementation** using analytical derivative (2x).
/// Actual Enzyme AD integration will be added in Phase 4.
///
/// # Arguments
///
/// * `x` - Input value
///
/// # Returns
///
/// The gradient of square function at x: 2x
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::verify::square_gradient;
///
/// assert_eq!(square_gradient(3.0), 6.0);
/// assert_eq!(square_gradient(-2.5), -5.0);
/// assert_eq!(square_gradient(0.0), 0.0);
/// ```
///
/// # Future: Enzyme Integration
///
/// In Phase 4, this will be replaced with actual Enzyme AD:
///
/// ```rust,ignore
/// #[enzyme::autodiff]
/// pub fn square_gradient(x: f64) -> f64 {
///     enzyme::gradient(square, x)
/// }
/// ```
#[inline]
pub fn square_gradient(x: f64) -> f64 {
    // Placeholder: analytical derivative
    // Phase 4: Replace with Enzyme AD
    2.0 * x
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Requirement 4: Enzyme gradient verification tests

    #[test]
    fn test_square_value() {
        // Verify square function returns x²
        assert_eq!(square(3.0), 9.0);
        assert_eq!(square(-2.5), 6.25);
        assert_eq!(square(0.0), 0.0);
        assert_eq!(square(1.0), 1.0);
    }

    #[test]
    fn test_square_gradient() {
        // Verify gradient is 2x (analytical derivative)
        let x = 3.0;
        let gradient = square_gradient(x);
        let expected = 2.0 * x; // d(x²)/dx = 2x

        assert_relative_eq!(gradient, expected, epsilon = 1e-10);
        assert_relative_eq!(gradient, 6.0, epsilon = 1e-10);
    }

    #[test]
    fn test_square_gradient_at_zero() {
        // Verify gradient at x=0 is 0
        let gradient = square_gradient(0.0);
        assert_eq!(gradient, 0.0);
    }

    #[test]
    fn test_square_gradient_negative() {
        // Verify gradient works for negative values
        let x = -2.5;
        let gradient = square_gradient(x);
        let expected = 2.0 * x; // d(x²)/dx = 2x = -5.0

        assert_relative_eq!(gradient, expected, epsilon = 1e-10);
        assert_relative_eq!(gradient, -5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_square_gradient_positive_large() {
        // Verify gradient for larger values
        let x = 100.0;
        let gradient = square_gradient(x);
        let expected = 2.0 * x;

        assert_relative_eq!(gradient, expected, epsilon = 1e-10);
        assert_eq!(gradient, 200.0);
    }

    #[test]
    fn test_square_gradient_small_values() {
        // Verify gradient for small values
        let x = 0.001;
        let gradient = square_gradient(x);
        let expected = 2.0 * x;

        assert_relative_eq!(gradient, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_finite_difference_approximation() {
        // Verify placeholder matches finite difference approximation
        let x = 3.0;
        let h = 1e-8;

        // Finite difference: (f(x+h) - f(x-h)) / (2h)
        let fd_gradient = (square(x + h) - square(x - h)) / (2.0 * h);
        let analytical_gradient = square_gradient(x);

        assert_relative_eq!(analytical_gradient, fd_gradient, epsilon = 1e-6);
    }
}

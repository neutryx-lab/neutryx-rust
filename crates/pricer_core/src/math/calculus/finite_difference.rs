//! Finite difference methods for numerical differentiation.
//!
//! This module provides functions for approximating derivatives using
//! finite differences. These methods are useful when analytical derivatives
//! are unavailable or for verifying automatic differentiation results.

use num_traits::Float;

/// Type of finite difference approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceType {
    /// Forward difference: `(f(x+h) - f(x)) / h`
    ///
    /// Accuracy: O(h)
    Forward,

    /// Backward difference: `(f(x) - f(x-h)) / h`
    ///
    /// Accuracy: O(h)
    Backward,

    /// Central difference: `(f(x+h) - f(x-h)) / (2h)`
    ///
    /// Accuracy: O(h²)
    Central,
}

/// Computes the first derivative using finite differences.
///
/// # Arguments
///
/// * `f` - The function to differentiate
/// * `x` - The point at which to compute the derivative
/// * `h` - The step size (bump size)
/// * `diff_type` - The type of difference to use
///
/// # Returns
///
/// The approximated derivative f'(x)
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::{finite_diff, DifferenceType};
///
/// // Derivative of x² is 2x
/// let f = |x: f64| x * x;
/// let deriv = finite_diff(&f, 3.0, 1e-5, DifferenceType::Central);
/// assert!((deriv - 6.0).abs() < 1e-8);
/// ```
#[inline]
pub fn finite_diff<T, F>(f: &F, x: T, h: T, diff_type: DifferenceType) -> T
where
    T: Float,
    F: Fn(T) -> T,
{
    let two = T::from(2.0).unwrap();

    match diff_type {
        DifferenceType::Forward => (f(x + h) - f(x)) / h,
        DifferenceType::Backward => (f(x) - f(x - h)) / h,
        DifferenceType::Central => (f(x + h) - f(x - h)) / (two * h),
    }
}

/// Computes the second derivative using central finite differences.
///
/// Uses the formula: `(f(x+h) - 2*f(x) + f(x-h)) / h²`
///
/// Accuracy: O(h²)
///
/// # Arguments
///
/// * `f` - The function to differentiate
/// * `x` - The point at which to compute the second derivative
/// * `h` - The step size (bump size)
///
/// # Returns
///
/// The approximated second derivative f''(x)
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::finite_diff_second;
///
/// // Second derivative of x³ is 6x
/// let f = |x: f64| x.powi(3);
/// let deriv2 = finite_diff_second(&f, 2.0, 1e-4);
/// assert!((deriv2 - 12.0).abs() < 1e-4);
/// ```
#[inline]
pub fn finite_diff_second<T, F>(f: &F, x: T, h: T) -> T
where
    T: Float,
    F: Fn(T) -> T,
{
    let two = T::from(2.0).unwrap();
    (f(x + h) - two * f(x) + f(x - h)) / (h * h)
}

/// Computes a partial derivative of a multivariate function.
///
/// # Arguments
///
/// * `f` - The multivariate function
/// * `x` - The point (as a slice) at which to compute the derivative
/// * `index` - The index of the variable to differentiate with respect to
/// * `h` - The step size (bump size)
/// * `diff_type` - The type of difference to use
///
/// # Returns
///
/// The approximated partial derivative ∂f/∂x_i
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::{partial_diff, DifferenceType};
///
/// // f(x, y) = x² + y³, ∂f/∂x = 2x, ∂f/∂y = 3y²
/// let f = |x: &[f64]| x[0] * x[0] + x[1].powi(3);
/// let point = [2.0, 3.0];
///
/// let df_dx = partial_diff(&f, &point, 0, 1e-5, DifferenceType::Central);
/// let df_dy = partial_diff(&f, &point, 1, 1e-5, DifferenceType::Central);
///
/// assert!((df_dx - 4.0).abs() < 1e-6);   // 2 * 2 = 4
/// assert!((df_dy - 27.0).abs() < 1e-4);  // 3 * 3² = 27
/// ```
#[inline]
pub fn partial_diff<T, F>(f: &F, x: &[T], index: usize, h: T, diff_type: DifferenceType) -> T
where
    T: Float + Copy,
    F: Fn(&[T]) -> T,
{
    let two = T::from(2.0).unwrap();
    let n = x.len();

    // Create working buffers
    let mut x_plus = x.to_vec();
    let mut x_minus = x.to_vec();

    match diff_type {
        DifferenceType::Forward => {
            x_plus[index] = x[index] + h;
            (f(&x_plus) - f(x)) / h
        }
        DifferenceType::Backward => {
            x_minus[index] = x[index] - h;
            (f(x) - f(&x_minus)) / h
        }
        DifferenceType::Central => {
            x_plus[index] = x[index] + h;
            x_minus[index] = x[index] - h;
            (f(&x_plus) - f(&x_minus)) / (two * h)
        }
    }
}

/// Computes the second partial derivative of a multivariate function.
///
/// # Arguments
///
/// * `f` - The multivariate function
/// * `x` - The point (as a slice) at which to compute the derivative
/// * `index` - The index of the variable to differentiate with respect to
/// * `h` - The step size (bump size)
///
/// # Returns
///
/// The approximated second partial derivative ∂²f/∂x_i²
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::partial_diff_second;
///
/// // f(x, y) = x³ + y², ∂²f/∂x² = 6x, ∂²f/∂y² = 2
/// let f = |x: &[f64]| x[0].powi(3) + x[1] * x[1];
/// let point = [2.0, 3.0];
///
/// let d2f_dx2 = partial_diff_second(&f, &point, 0, 1e-4);
/// let d2f_dy2 = partial_diff_second(&f, &point, 1, 1e-4);
///
/// assert!((d2f_dx2 - 12.0).abs() < 1e-3);  // 6 * 2 = 12
/// assert!((d2f_dy2 - 2.0).abs() < 1e-6);   // 2
/// ```
#[inline]
pub fn partial_diff_second<T, F>(f: &F, x: &[T], index: usize, h: T) -> T
where
    T: Float + Copy,
    F: Fn(&[T]) -> T,
{
    let two = T::from(2.0).unwrap();

    // Create working buffers
    let mut x_plus = x.to_vec();
    let mut x_minus = x.to_vec();

    x_plus[index] = x[index] + h;
    x_minus[index] = x[index] - h;

    (f(&x_plus) - two * f(x) + f(&x_minus)) / (h * h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ==========================================================================
    // First derivative tests
    // ==========================================================================

    #[test]
    fn test_forward_diff_linear() {
        // f(x) = 3x + 2, f'(x) = 3
        let f = |x: f64| 3.0 * x + 2.0;
        let deriv = finite_diff(&f, 5.0, 1e-6, DifferenceType::Forward);
        assert_relative_eq!(deriv, 3.0, epsilon = 1e-6);
    }

    #[test]
    fn test_backward_diff_linear() {
        let f = |x: f64| 3.0 * x + 2.0;
        let deriv = finite_diff(&f, 5.0, 1e-6, DifferenceType::Backward);
        assert_relative_eq!(deriv, 3.0, epsilon = 1e-6);
    }

    #[test]
    fn test_central_diff_linear() {
        let f = |x: f64| 3.0 * x + 2.0;
        let deriv = finite_diff(&f, 5.0, 1e-6, DifferenceType::Central);
        assert_relative_eq!(deriv, 3.0, epsilon = 1e-8);
    }

    #[test]
    fn test_central_diff_quadratic() {
        // f(x) = x², f'(x) = 2x
        let f = |x: f64| x * x;
        let deriv = finite_diff(&f, 3.0, 1e-5, DifferenceType::Central);
        assert_relative_eq!(deriv, 6.0, epsilon = 1e-8);
    }

    #[test]
    fn test_central_diff_cubic() {
        // f(x) = x³, f'(x) = 3x²
        let f = |x: f64| x.powi(3);
        let deriv = finite_diff(&f, 2.0, 1e-5, DifferenceType::Central);
        assert_relative_eq!(deriv, 12.0, epsilon = 1e-6);
    }

    #[test]
    fn test_central_diff_sin() {
        // f(x) = sin(x), f'(x) = cos(x)
        let f = |x: f64| x.sin();
        let x = core::f64::consts::PI / 4.0;
        let deriv = finite_diff(&f, x, 1e-6, DifferenceType::Central);
        let expected = x.cos();
        assert_relative_eq!(deriv, expected, epsilon = 1e-8);
    }

    #[test]
    fn test_central_diff_exp() {
        // f(x) = exp(x), f'(x) = exp(x)
        let f = |x: f64| x.exp();
        let x = 1.0;
        let deriv = finite_diff(&f, x, 1e-6, DifferenceType::Central);
        let expected = x.exp();
        assert_relative_eq!(deriv, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_central_is_more_accurate_than_forward() {
        // f(x) = x³, f'(x) = 3x²
        let f = |x: f64| x.powi(3);
        let x = 2.0;
        let h = 1e-3; // Larger h to show difference

        let forward = finite_diff(&f, x, h, DifferenceType::Forward);
        let central = finite_diff(&f, x, h, DifferenceType::Central);
        let expected = 12.0;

        let forward_err = (forward - expected).abs();
        let central_err = (central - expected).abs();

        // Central should be more accurate
        assert!(
            central_err < forward_err,
            "Central error {central_err} should be less than forward error {forward_err}"
        );
    }

    // ==========================================================================
    // Second derivative tests
    // ==========================================================================

    #[test]
    fn test_second_diff_quadratic() {
        // f(x) = x², f''(x) = 2
        let f = |x: f64| x * x;
        let deriv2 = finite_diff_second(&f, 3.0, 1e-4);
        assert_relative_eq!(deriv2, 2.0, epsilon = 1e-6);
    }

    #[test]
    fn test_second_diff_cubic() {
        // f(x) = x³, f''(x) = 6x
        let f = |x: f64| x.powi(3);
        let deriv2 = finite_diff_second(&f, 2.0, 1e-4);
        assert_relative_eq!(deriv2, 12.0, epsilon = 1e-4);
    }

    #[test]
    fn test_second_diff_quartic() {
        // f(x) = x⁴, f''(x) = 12x²
        let f = |x: f64| x.powi(4);
        let x = 2.0;
        let deriv2 = finite_diff_second(&f, x, 1e-4);
        let expected = 12.0 * x * x;
        assert_relative_eq!(deriv2, expected, epsilon = 1e-3);
    }

    #[test]
    fn test_second_diff_sin() {
        // f(x) = sin(x), f''(x) = -sin(x)
        let f = |x: f64| x.sin();
        let x = core::f64::consts::PI / 3.0;
        let deriv2 = finite_diff_second(&f, x, 1e-4);
        let expected = -x.sin();
        assert_relative_eq!(deriv2, expected, epsilon = 1e-6);
    }

    // ==========================================================================
    // Partial derivative tests
    // ==========================================================================

    #[test]
    fn test_partial_diff_linear() {
        // f(x, y) = 2x + 3y
        let f = |x: &[f64]| 2.0 * x[0] + 3.0 * x[1];
        let point = [1.0, 2.0];

        let df_dx = partial_diff(&f, &point, 0, 1e-6, DifferenceType::Central);
        let df_dy = partial_diff(&f, &point, 1, 1e-6, DifferenceType::Central);

        assert_relative_eq!(df_dx, 2.0, epsilon = 1e-8);
        assert_relative_eq!(df_dy, 3.0, epsilon = 1e-8);
    }

    #[test]
    fn test_partial_diff_quadratic() {
        // f(x, y) = x² + xy + y²
        // ∂f/∂x = 2x + y
        // ∂f/∂y = x + 2y
        let f = |x: &[f64]| x[0] * x[0] + x[0] * x[1] + x[1] * x[1];
        let point = [2.0, 3.0];

        let df_dx = partial_diff(&f, &point, 0, 1e-5, DifferenceType::Central);
        let df_dy = partial_diff(&f, &point, 1, 1e-5, DifferenceType::Central);

        assert_relative_eq!(df_dx, 7.0, epsilon = 1e-6); // 2*2 + 3 = 7
        assert_relative_eq!(df_dy, 8.0, epsilon = 1e-6); // 2 + 2*3 = 8
    }

    #[test]
    fn test_partial_diff_three_vars() {
        // f(x, y, z) = x*y*z
        // ∂f/∂x = y*z
        // ∂f/∂y = x*z
        // ∂f/∂z = x*y
        let f = |x: &[f64]| x[0] * x[1] * x[2];
        let point = [2.0, 3.0, 4.0];

        let df_dx = partial_diff(&f, &point, 0, 1e-5, DifferenceType::Central);
        let df_dy = partial_diff(&f, &point, 1, 1e-5, DifferenceType::Central);
        let df_dz = partial_diff(&f, &point, 2, 1e-5, DifferenceType::Central);

        assert_relative_eq!(df_dx, 12.0, epsilon = 1e-6); // 3*4 = 12
        assert_relative_eq!(df_dy, 8.0, epsilon = 1e-6);  // 2*4 = 8
        assert_relative_eq!(df_dz, 6.0, epsilon = 1e-6);  // 2*3 = 6
    }

    // ==========================================================================
    // Second partial derivative tests
    // ==========================================================================

    #[test]
    fn test_partial_diff_second_quadratic() {
        // f(x, y) = x² + 2xy + 3y²
        // ∂²f/∂x² = 2
        // ∂²f/∂y² = 6
        let f = |x: &[f64]| x[0] * x[0] + 2.0 * x[0] * x[1] + 3.0 * x[1] * x[1];
        let point = [2.0, 3.0];

        let d2f_dx2 = partial_diff_second(&f, &point, 0, 1e-4);
        let d2f_dy2 = partial_diff_second(&f, &point, 1, 1e-4);

        assert_relative_eq!(d2f_dx2, 2.0, epsilon = 1e-6);
        assert_relative_eq!(d2f_dy2, 6.0, epsilon = 1e-6);
    }

    #[test]
    fn test_partial_diff_second_cubic() {
        // f(x, y) = x³ + y³
        // ∂²f/∂x² = 6x
        // ∂²f/∂y² = 6y
        let f = |x: &[f64]| x[0].powi(3) + x[1].powi(3);
        let point = [2.0, 3.0];

        let d2f_dx2 = partial_diff_second(&f, &point, 0, 1e-4);
        let d2f_dy2 = partial_diff_second(&f, &point, 1, 1e-4);

        assert_relative_eq!(d2f_dx2, 12.0, epsilon = 1e-3); // 6*2 = 12
        assert_relative_eq!(d2f_dy2, 18.0, epsilon = 1e-3); // 6*3 = 18
    }

    // ==========================================================================
    // DifferenceType tests
    // ==========================================================================

    #[test]
    fn test_difference_type_equality() {
        assert_eq!(DifferenceType::Forward, DifferenceType::Forward);
        assert_ne!(DifferenceType::Forward, DifferenceType::Backward);
        assert_ne!(DifferenceType::Central, DifferenceType::Forward);
    }

    #[test]
    fn test_difference_type_clone() {
        let dt = DifferenceType::Central;
        let dt_clone = dt;
        assert_eq!(dt, dt_clone);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_central_diff_polynomial(x in -10.0_f64..10.0, a in -5.0_f64..5.0, b in -5.0_f64..5.0) {
            // f(x) = a*x² + b*x, f'(x) = 2ax + b
            let f = |t: f64| a * t * t + b * t;
            let deriv = finite_diff(&f, x, 1e-5, DifferenceType::Central);
            let expected = 2.0 * a * x + b;

            // Allow for numerical error
            prop_assert!((deriv - expected).abs() < 1e-4);
        }

        #[test]
        fn prop_second_diff_polynomial(x in -10.0_f64..10.0, a in -5.0_f64..5.0) {
            // f(x) = a*x², f''(x) = 2a
            let f = |t: f64| a * t * t;
            let deriv2 = finite_diff_second(&f, x, 1e-3);
            let expected = 2.0 * a;

            prop_assert!((deriv2 - expected).abs() < 1e-4);
        }
    }
}

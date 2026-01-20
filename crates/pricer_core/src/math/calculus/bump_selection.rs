//! Automatic bump size selection for finite differences.
//!
//! Optimal step size balances truncation error (from finite difference
//! approximation) and round-off error (from floating-point arithmetic).
//!
//! ## Theoretical Background
//!
//! For central differences, the optimal step size is approximately:
//! - First derivative: h ≈ ε^(1/3) * |x| where ε is machine epsilon
//! - Second derivative: h ≈ ε^(1/4) * |x|
//!
//! This module provides functions to compute these optimal step sizes.

use num_traits::Float;

/// Suggests an optimal bump size for first derivative computation.
///
/// Uses the rule of thumb: h ≈ ε^(1/3) * max(|x|, 1)
///
/// This balances:
/// - Truncation error O(h²) for central differences
/// - Round-off error O(ε/h)
///
/// # Arguments
///
/// * `x` - The point at which the derivative will be computed
///
/// # Returns
///
/// The suggested step size h
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::suggest_bump_size;
///
/// let h = suggest_bump_size(1.0_f64);
/// assert!(h > 1e-10 && h < 1e-4);
/// ```
#[inline]
pub fn suggest_bump_size<T: Float>(x: T) -> T {
    // Machine epsilon
    let eps = T::epsilon();

    // Cube root of epsilon (optimal for central differences)
    let cbrt_eps = eps.powf(T::from(1.0 / 3.0).unwrap());

    // Scale by magnitude of x, with minimum of 1
    let scale = x.abs().max(T::one());

    cbrt_eps * scale
}

/// Suggests an optimal bump size for second derivative computation.
///
/// Uses the rule of thumb: h ≈ ε^(1/4) * max(|x|, 1)
///
/// This balances:
/// - Truncation error O(h²) for second central differences
/// - Round-off error O(ε/h²)
///
/// # Arguments
///
/// * `x` - The point at which the derivative will be computed
///
/// # Returns
///
/// The suggested step size h
///
/// # Example
///
/// ```
/// use pricer_core::math::calculus::suggest_bump_size_second;
///
/// let h = suggest_bump_size_second(1.0_f64);
/// assert!(h > 1e-8 && h < 1e-2);
/// ```
#[inline]
pub fn suggest_bump_size_second<T: Float>(x: T) -> T {
    // Machine epsilon
    let eps = T::epsilon();

    // Fourth root of epsilon (optimal for second derivatives)
    let root4_eps = eps.powf(T::from(0.25).unwrap());

    // Scale by magnitude of x, with minimum of 1
    let scale = x.abs().max(T::one());

    root4_eps * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_suggest_bump_size_typical() {
        let h = suggest_bump_size(1.0_f64);

        // For f64, epsilon ≈ 2.2e-16, so h ≈ 6e-6
        assert!(h > 1e-8);
        assert!(h < 1e-4);
    }

    #[test]
    fn test_suggest_bump_size_scales_with_x() {
        let h1 = suggest_bump_size(1.0_f64);
        let h2 = suggest_bump_size(1000.0_f64);

        // h should scale linearly with x
        assert_relative_eq!(h2 / h1, 1000.0, epsilon = 1e-10);
    }

    #[test]
    fn test_suggest_bump_size_minimum_scale() {
        let h_small = suggest_bump_size(1e-10_f64);
        let h_one = suggest_bump_size(1.0_f64);

        // For very small x, scale defaults to 1
        assert_relative_eq!(h_small, h_one, epsilon = 1e-15);
    }

    #[test]
    fn test_suggest_bump_size_negative_x() {
        let h_pos = suggest_bump_size(5.0_f64);
        let h_neg = suggest_bump_size(-5.0_f64);

        // Should be the same for positive and negative x
        assert_relative_eq!(h_pos, h_neg, epsilon = 1e-15);
    }

    #[test]
    fn test_suggest_bump_size_second_typical() {
        let h = suggest_bump_size_second(1.0_f64);

        // For f64, epsilon ≈ 2.2e-16, so h ≈ 1.2e-4
        assert!(h > 1e-6);
        assert!(h < 1e-2);
    }

    #[test]
    fn test_suggest_bump_size_second_larger_than_first() {
        // Second derivative needs larger step size due to h² in denominator
        let h1 = suggest_bump_size(1.0_f64);
        let h2 = suggest_bump_size_second(1.0_f64);

        assert!(h2 > h1);
    }

    #[test]
    fn test_suggest_bump_size_second_scales_with_x() {
        let h1 = suggest_bump_size_second(1.0_f64);
        let h2 = suggest_bump_size_second(100.0_f64);

        assert_relative_eq!(h2 / h1, 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_bump_sizes_improve_accuracy() {
        // Test that suggested bump sizes give good accuracy
        use crate::math::calculus::{finite_diff, finite_diff_second, DifferenceType};

        // f(x) = x³, f'(x) = 3x², f''(x) = 6x
        let f = |x: f64| x.powi(3);
        let x = 2.0;

        let h1 = suggest_bump_size(x);
        let h2 = suggest_bump_size_second(x);

        let deriv1 = finite_diff(&f, x, h1, DifferenceType::Central);
        let deriv2 = finite_diff_second(&f, x, h2);

        // Should achieve good accuracy
        assert_relative_eq!(deriv1, 12.0, epsilon = 1e-8); // 3 * 2² = 12
        assert_relative_eq!(deriv2, 12.0, epsilon = 1e-6); // 6 * 2 = 12
    }
}

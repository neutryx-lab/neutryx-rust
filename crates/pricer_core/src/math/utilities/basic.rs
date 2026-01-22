//! Basic mathematical utility functions.
//!
//! This module provides simple but commonly used mathematical functions
//! that are generic over `T: Float` for AD compatibility.

use num_traits::Float;

/// Returns the sign of a number.
///
/// Returns:
/// - `1` if x > 0
/// - `-1` if x < 0
/// - `0` if x == 0
///
/// # Arguments
///
/// * `x` - The input value
///
/// # Returns
///
/// The sign of x as a float (-1, 0, or 1)
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::sign;
///
/// assert_eq!(sign(5.0_f64), 1.0);
/// assert_eq!(sign(-3.0_f64), -1.0);
/// assert_eq!(sign(0.0_f64), 0.0);
/// ```
#[inline]
pub fn sign<T: Float>(x: T) -> T {
    if x > T::zero() {
        T::one()
    } else if x < T::zero() {
        -T::one()
    } else {
        T::zero()
    }
}

/// Clamps a value to a specified range.
///
/// If `x < min`, returns `min`.
/// If `x > max`, returns `max`.
/// Otherwise, returns `x`.
///
/// # Arguments
///
/// * `x` - The value to clamp
/// * `min` - The minimum bound
/// * `max` - The maximum bound
///
/// # Returns
///
/// The clamped value
///
/// # Panics
///
/// Debug assertion if `min > max`.
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::clamp;
///
/// assert_eq!(clamp(5.0_f64, 0.0, 10.0), 5.0);
/// assert_eq!(clamp(-5.0_f64, 0.0, 10.0), 0.0);
/// assert_eq!(clamp(15.0_f64, 0.0, 10.0), 10.0);
/// ```
#[inline]
pub fn clamp<T: Float>(x: T, min: T, max: T) -> T {
    debug_assert!(min <= max, "clamp: min must be <= max");

    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// Linear interpolation between two values.
///
/// Computes `a + t * (b - a)` which equals:
/// - `a` when `t = 0`
/// - `b` when `t = 1`
/// - Linear blend for `0 < t < 1`
///
/// # Arguments
///
/// * `a` - Start value (at t=0)
/// * `b` - End value (at t=1)
/// * `t` - Interpolation parameter (typically in [0, 1])
///
/// # Returns
///
/// The interpolated value
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::lerp;
///
/// assert_eq!(lerp(0.0_f64, 10.0, 0.0), 0.0);
/// assert_eq!(lerp(0.0_f64, 10.0, 1.0), 10.0);
/// assert_eq!(lerp(0.0_f64, 10.0, 0.5), 5.0);
/// assert_eq!(lerp(2.0_f64, 8.0, 0.25), 3.5);
/// ```
#[inline]
pub fn lerp<T: Float>(a: T, b: T, t: T) -> T { a + t * (b - a) }

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // sign tests
    // ==========================================================================

    #[test]
    fn test_sign_positive() {
        assert_eq!(sign(1.0_f64), 1.0);
        assert_eq!(sign(100.0_f64), 1.0);
        assert_eq!(sign(0.001_f64), 1.0);
    }

    #[test]
    fn test_sign_negative() {
        assert_eq!(sign(-1.0_f64), -1.0);
        assert_eq!(sign(-100.0_f64), -1.0);
        assert_eq!(sign(-0.001_f64), -1.0);
    }

    #[test]
    fn test_sign_zero() {
        assert_eq!(sign(0.0_f64), 0.0);
        assert_eq!(sign(-0.0_f64), 0.0);
    }

    // ==========================================================================
    // clamp tests
    // ==========================================================================

    #[test]
    fn test_clamp_within_bounds() {
        assert_eq!(clamp(5.0_f64, 0.0, 10.0), 5.0);
        assert_eq!(clamp(0.0_f64, -5.0, 5.0), 0.0);
    }

    #[test]
    fn test_clamp_below_min() {
        assert_eq!(clamp(-5.0_f64, 0.0, 10.0), 0.0);
        assert_eq!(clamp(-100.0_f64, -10.0, 10.0), -10.0);
    }

    #[test]
    fn test_clamp_above_max() {
        assert_eq!(clamp(15.0_f64, 0.0, 10.0), 10.0);
        assert_eq!(clamp(100.0_f64, -10.0, 10.0), 10.0);
    }

    #[test]
    fn test_clamp_at_bounds() {
        assert_eq!(clamp(0.0_f64, 0.0, 10.0), 0.0);
        assert_eq!(clamp(10.0_f64, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_clamp_equal_bounds() {
        assert_eq!(clamp(5.0_f64, 5.0, 5.0), 5.0);
        assert_eq!(clamp(0.0_f64, 5.0, 5.0), 5.0);
        assert_eq!(clamp(10.0_f64, 5.0, 5.0), 5.0);
    }

    // ==========================================================================
    // lerp tests
    // ==========================================================================

    #[test]
    fn test_lerp_endpoints() {
        assert_eq!(lerp(0.0_f64, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0_f64, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_lerp_midpoint() {
        assert_eq!(lerp(0.0_f64, 10.0, 0.5), 5.0);
        assert_eq!(lerp(-10.0_f64, 10.0, 0.5), 0.0);
    }

    #[test]
    fn test_lerp_quarter_points() {
        assert_eq!(lerp(0.0_f64, 100.0, 0.25), 25.0);
        assert_eq!(lerp(0.0_f64, 100.0, 0.75), 75.0);
    }

    #[test]
    fn test_lerp_negative_range() {
        assert_eq!(lerp(-10.0_f64, -5.0, 0.0), -10.0);
        assert_eq!(lerp(-10.0_f64, -5.0, 1.0), -5.0);
        assert_eq!(lerp(-10.0_f64, -5.0, 0.5), -7.5);
    }

    #[test]
    fn test_lerp_extrapolation() {
        // lerp allows t outside [0, 1]
        assert_eq!(lerp(0.0_f64, 10.0, -0.5), -5.0);
        assert_eq!(lerp(0.0_f64, 10.0, 1.5), 15.0);
    }

    #[test]
    fn test_lerp_same_values() {
        assert_relative_eq!(lerp(5.0_f64, 5.0, 0.0), 5.0);
        assert_relative_eq!(lerp(5.0_f64, 5.0, 0.5), 5.0);
        assert_relative_eq!(lerp(5.0_f64, 5.0, 1.0), 5.0);
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prop_sign_magnitude_is_one_or_zero(x in -1000.0_f64..1000.0) {
            let s = sign(x);
            prop_assert!(s == 1.0 || s == -1.0 || s == 0.0);
        }

        #[test]
        fn prop_sign_product_is_abs(x in -1000.0_f64..1000.0) {
            // sign(x) * |x| should equal x (except for x=0)
            if x != 0.0 {
                let result = sign(x) * x.abs();
                prop_assert!((result - x).abs() < 1e-10);
            }
        }

        #[test]
        fn prop_clamp_within_bounds(
            x in -100.0_f64..100.0,
            min in -100.0_f64..0.0,
            max in 0.0_f64..100.0
        ) {
            let result = clamp(x, min, max);
            prop_assert!(result >= min);
            prop_assert!(result <= max);
        }

        #[test]
        fn prop_lerp_at_zero_is_a(a in -100.0_f64..100.0, b in -100.0_f64..100.0) {
            let result = lerp(a, b, 0.0);
            prop_assert!((result - a).abs() < 1e-10);
        }

        #[test]
        fn prop_lerp_at_one_is_b(a in -100.0_f64..100.0, b in -100.0_f64..100.0) {
            let result = lerp(a, b, 1.0);
            prop_assert!((result - b).abs() < 1e-10);
        }

        #[test]
        fn prop_lerp_monotonic(
            a in -100.0_f64..100.0,
            b in -100.0_f64..100.0,
            t1 in 0.0_f64..0.5,
            t2 in 0.5_f64..1.0
        ) {
            let r1 = lerp(a, b, t1);
            let r2 = lerp(a, b, t2);

            if a < b {
                prop_assert!(r1 <= r2);
            } else if a > b {
                prop_assert!(r1 >= r2);
            } else {
                prop_assert!((r1 - r2).abs() < 1e-10);
            }
        }
    }
}

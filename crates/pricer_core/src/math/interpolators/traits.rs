//! Core traits for interpolation.

use num_traits::Float;

use crate::types::InterpolationError;

/// Generic trait for 1D interpolation.
///
/// Implementations of this trait provide interpolation over a set of data
/// points, supporting both standard floating-point types and dual numbers for
/// automatic differentiation.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Contract
///
/// - `interpolate(x)` returns `Ok(y)` if `x` is within `domain()`
/// - `interpolate(x)` returns `Err(OutOfBounds)` if `x` is outside `domain()`
/// - `domain()` returns `(x_min, x_max)` where `x_min <= x_max`
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::interpolators::Interpolator;
///
/// fn use_interpolator<T: Float, I: Interpolator<T>>(interp: &I, x: T) -> T {
///     let (x_min, x_max) = interp.domain();
///     interp.interpolate(x).expect("x within domain")
/// }
/// ```
pub trait Interpolator<T: Float> {
    /// Interpolate value at point `x`.
    ///
    /// # Arguments
    ///
    /// * `x` - The point at which to interpolate
    ///
    /// # Returns
    ///
    /// * `Ok(y)` - The interpolated value at `x`
    /// * `Err(InterpolationError::OutOfBounds)` - If `x` is outside the valid
    ///   domain
    ///
    /// # Example
    ///
    /// ```ignore
    /// let y = interpolator.interpolate(1.5)?;
    /// ```
    fn interpolate(&self, x: T) -> Result<T, InterpolationError>;

    /// Return the valid interpolation domain.
    ///
    /// # Returns
    ///
    /// A tuple `(x_min, x_max)` representing the range of valid `x` values
    /// for interpolation. Queries outside this range will return
    /// `InterpolationError::OutOfBounds`.
    ///
    /// # Invariant
    ///
    /// `x_min <= x_max` is always true.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (x_min, x_max) = interpolator.domain();
    /// assert!(x_min <= x_max);
    /// ```
    fn domain(&self) -> (T, T);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that the trait is object-safe and can be used with trait objects
    #[test]
    fn test_trait_is_object_safe() {
        // This test compiles if the trait is object-safe
        fn _accept_dyn_interpolator(_: &dyn Interpolator<f64>) {}
    }

    // Mock implementation for testing trait definition
    struct MockInterpolator {
        x_min: f64,
        x_max: f64,
    }

    impl Interpolator<f64> for MockInterpolator {
        fn interpolate(&self, x: f64) -> Result<f64, InterpolationError> {
            if x < self.x_min || x > self.x_max {
                return Err(InterpolationError::OutOfBounds {
                    x,
                    min: self.x_min,
                    max: self.x_max,
                });
            }
            Ok(x * 2.0) // Simple linear mock
        }

        fn domain(&self) -> (f64, f64) { (self.x_min, self.x_max) }
    }

    #[test]
    fn test_mock_interpolator_domain() {
        let interp = MockInterpolator {
            x_min: 0.0,
            x_max: 10.0,
        };
        let (x_min, x_max) = interp.domain();
        assert_eq!(x_min, 0.0);
        assert_eq!(x_max, 10.0);
    }

    #[test]
    fn test_mock_interpolator_in_bounds() {
        let interp = MockInterpolator {
            x_min: 0.0,
            x_max: 10.0,
        };
        let result = interp.interpolate(5.0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10.0);
    }

    #[test]
    fn test_mock_interpolator_out_of_bounds_low() {
        let interp = MockInterpolator {
            x_min: 0.0,
            x_max: 10.0,
        };
        let result = interp.interpolate(-1.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { x, min, max } => {
                assert_eq!(x, -1.0);
                assert_eq!(min, 0.0);
                assert_eq!(max, 10.0);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_mock_interpolator_out_of_bounds_high() {
        let interp = MockInterpolator {
            x_min: 0.0,
            x_max: 10.0,
        };
        let result = interp.interpolate(11.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { x, min, max } => {
                assert_eq!(x, 11.0);
                assert_eq!(min, 0.0);
                assert_eq!(max, 10.0);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_mock_interpolator_boundary_values() {
        let interp = MockInterpolator {
            x_min: 0.0,
            x_max: 10.0,
        };
        // Exactly at boundaries should be valid
        assert!(interp.interpolate(0.0).is_ok());
        assert!(interp.interpolate(10.0).is_ok());
    }
}

//! Linear interpolation implementation.

use super::Interpolator;
use crate::types::InterpolationError;
use num_traits::Float;

/// Piecewise linear interpolator.
///
/// Stores sorted (x, y) data points and performs linear interpolation
/// between adjacent points. Supports automatic differentiation through
/// generic `T: Float` type parameter.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Construction
///
/// Data points are automatically sorted by x-coordinate during construction.
/// At least 2 data points are required.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
///
/// let xs = [0.0, 1.0, 2.0, 3.0];
/// let ys = [0.0, 2.0, 4.0, 6.0];
///
/// let interp = LinearInterpolator::new(&xs, &ys).unwrap();
/// assert_eq!(interp.domain(), (0.0, 3.0));
/// ```
#[derive(Debug, Clone)]
pub struct LinearInterpolator<T: Float> {
    /// Sorted x-coordinates
    xs: Vec<T>,
    /// Corresponding y-values (in same order as xs after sorting)
    ys: Vec<T>,
}

impl<T: Float> LinearInterpolator<T> {
    /// Construct a linear interpolator from x and y data points.
    ///
    /// Data points are automatically sorted by x-coordinate if not already sorted.
    /// Requires at least 2 data points.
    ///
    /// # Arguments
    ///
    /// * `xs` - Slice of x-coordinates
    /// * `ys` - Slice of corresponding y-values
    ///
    /// # Returns
    ///
    /// * `Ok(LinearInterpolator)` - Successfully constructed interpolator
    /// * `Err(InterpolationError::InsufficientData)` - Fewer than 2 data points
    /// * `Err(InterpolationError::InvalidInput)` - Mismatched array lengths
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::LinearInterpolator;
    ///
    /// // Valid construction
    /// let interp = LinearInterpolator::new(&[0.0, 1.0], &[0.0, 1.0]).unwrap();
    ///
    /// // Insufficient data
    /// let result = LinearInterpolator::new(&[0.0], &[0.0]);
    /// assert!(result.is_err());
    /// ```
    pub fn new(xs: &[T], ys: &[T]) -> Result<Self, InterpolationError> {
        // Validate array lengths match
        if xs.len() != ys.len() {
            return Err(InterpolationError::InvalidInput(format!(
                "xs and ys must have same length: got {} and {}",
                xs.len(),
                ys.len()
            )));
        }

        // Validate minimum data points
        if xs.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                got: xs.len(),
                need: 2,
            });
        }

        // Create paired data and sort by x
        let mut pairs: Vec<(T, T)> = xs.iter().copied().zip(ys.iter().copied()).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Unzip back into separate vectors
        let (sorted_xs, sorted_ys): (Vec<T>, Vec<T>) = pairs.into_iter().unzip();

        Ok(Self {
            xs: sorted_xs,
            ys: sorted_ys,
        })
    }

    /// Returns a reference to the sorted x-coordinates.
    #[inline]
    pub fn xs(&self) -> &[T] {
        &self.xs
    }

    /// Returns a reference to the y-values (in sorted x order).
    #[inline]
    pub fn ys(&self) -> &[T] {
        &self.ys
    }

    /// Returns the number of data points.
    #[inline]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    /// Returns true if the interpolator has no data points.
    /// Note: This should never be true for a valid interpolator.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }

    /// Find the segment index for interpolation using binary search.
    ///
    /// Returns the index `i` such that `xs[i] <= x < xs[i+1]`,
    /// clamped to valid segment range [0, n-2].
    #[inline]
    fn find_segment(&self, x: T) -> usize {
        // Use partition_point for O(log n) binary search
        // partition_point returns index where predicate becomes false
        let pos = self.xs.partition_point(|&xi| xi <= x);

        // Clamp to valid segment index [0, n-2]
        // pos can be 0 (x < xs[0]) or n (x >= xs[n-1])
        if pos == 0 {
            0
        } else if pos >= self.xs.len() {
            self.xs.len() - 2
        } else {
            pos - 1
        }
    }
}

impl<T: Float> Interpolator<T> for LinearInterpolator<T> {
    /// Interpolate value at point `x` using piecewise linear interpolation.
    ///
    /// Uses binary search (O(log n)) to find the appropriate segment,
    /// then applies the linear interpolation formula.
    ///
    /// # Formula
    ///
    /// ```text
    /// y = y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `x` - The point at which to interpolate
    ///
    /// # Returns
    ///
    /// * `Ok(y)` - The interpolated value
    /// * `Err(InterpolationError::OutOfBounds)` - If `x` is outside the domain
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
    ///
    /// let interp = LinearInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 2.0, 4.0]).unwrap();
    ///
    /// // Interpolate at midpoint
    /// let y = interp.interpolate(0.5).unwrap();
    /// assert!((y - 1.0).abs() < 1e-10);
    ///
    /// // Interpolate at knot point
    /// let y = interp.interpolate(1.0).unwrap();
    /// assert!((y - 2.0).abs() < 1e-10);
    /// ```
    fn interpolate(&self, x: T) -> Result<T, InterpolationError> {
        let x_min = self.xs[0];
        let x_max = self.xs[self.xs.len() - 1];

        // Check bounds
        if x < x_min || x > x_max {
            return Err(InterpolationError::OutOfBounds {
                x: x.to_f64().unwrap_or(f64::NAN),
                min: x_min.to_f64().unwrap_or(f64::NAN),
                max: x_max.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Find the segment containing x
        let i = self.find_segment(x);

        // Get segment endpoints
        let x0 = self.xs[i];
        let x1 = self.xs[i + 1];
        let y0 = self.ys[i];
        let y1 = self.ys[i + 1];

        // Linear interpolation formula: y = y0 + (y1 - y0) * (x - x0) / (x1 - x0)
        let t = (x - x0) / (x1 - x0);
        let y = y0 + (y1 - y0) * t;

        Ok(y)
    }

    /// Return the valid interpolation domain.
    ///
    /// # Returns
    ///
    /// A tuple `(x_min, x_max)` representing the range of valid x values.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::{Interpolator, LinearInterpolator};
    ///
    /// let interp = LinearInterpolator::new(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0]).unwrap();
    /// assert_eq!(interp.domain(), (1.0, 3.0));
    /// ```
    #[inline]
    fn domain(&self) -> (T, T) {
        (self.xs[0], self.xs[self.xs.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests (Task 3.1)
    // ========================================

    #[test]
    fn test_new_with_minimum_points() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 2);
    }

    #[test]
    fn test_new_with_multiple_points() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 4.0, 9.0, 16.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 5);
    }

    #[test]
    fn test_new_insufficient_data_zero_points() {
        let xs: [f64; 0] = [];
        let ys: [f64; 0] = [];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InsufficientData { got, need } => {
                assert_eq!(got, 0);
                assert_eq!(need, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_insufficient_data_one_point() {
        let xs = [1.0];
        let ys = [2.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InsufficientData { got, need } => {
                assert_eq!(got, 1);
                assert_eq!(need, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_mismatched_lengths() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InvalidInput(msg) => {
                assert!(msg.contains("same length"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_new_auto_sorts_unsorted_data() {
        // Provide unsorted x data
        let xs = [3.0, 1.0, 2.0, 0.0];
        let ys = [9.0, 1.0, 4.0, 0.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();

        // Verify xs are sorted
        assert_eq!(interp.xs(), &[0.0, 1.0, 2.0, 3.0]);
        // Verify ys correspond to sorted xs
        assert_eq!(interp.ys(), &[0.0, 1.0, 4.0, 9.0]);
    }

    #[test]
    fn test_new_preserves_already_sorted_data() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 4.0, 6.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.xs(), &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(interp.ys(), &[0.0, 2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_new_with_negative_values() {
        let xs = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let ys = [4.0, 1.0, 0.0, 1.0, 4.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.xs(), &[-2.0, -1.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_len_and_is_empty() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        assert_eq!(interp.len(), 3);
        assert!(!interp.is_empty());
    }

    #[test]
    fn test_clone() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        let cloned = interp.clone();
        assert_eq!(interp.xs(), cloned.xs());
        assert_eq!(interp.ys(), cloned.ys());
    }

    #[test]
    fn test_debug() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        let debug_str = format!("{:?}", interp);
        assert!(debug_str.contains("LinearInterpolator"));
    }

    #[test]
    fn test_with_f32() {
        let xs: [f32; 3] = [0.0, 1.0, 2.0];
        let ys: [f32; 3] = [0.0, 1.0, 4.0];
        let result = LinearInterpolator::new(&xs, &ys);
        assert!(result.is_ok());
    }

    // ========================================
    // Interpolation Tests (Task 3.2)
    // ========================================

    #[test]
    fn test_domain() {
        let interp = LinearInterpolator::new(&[1.0, 2.0, 3.0, 4.0], &[1.0, 4.0, 9.0, 16.0]).unwrap();
        let (x_min, x_max) = interp.domain();
        assert_eq!(x_min, 1.0);
        assert_eq!(x_max, 4.0);
    }

    #[test]
    fn test_domain_with_negative_values() {
        let interp = LinearInterpolator::new(&[-2.0, 0.0, 2.0], &[4.0, 0.0, 4.0]).unwrap();
        let (x_min, x_max) = interp.domain();
        assert_eq!(x_min, -2.0);
        assert_eq!(x_max, 2.0);
    }

    #[test]
    fn test_interpolate_at_knot_points() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 4.0, 6.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // Interpolating at knot points should return exact y values
        assert!((interp.interpolate(0.0).unwrap() - 0.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(2.0).unwrap() - 4.0).abs() < 1e-10);
        assert!((interp.interpolate(3.0).unwrap() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_midpoints() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 4.0, 6.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // Midpoints should interpolate linearly
        assert!((interp.interpolate(0.5).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - 3.0).abs() < 1e-10);
        assert!((interp.interpolate(2.5).unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_arbitrary_points() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // Test at x = 0.25: interpolate between (0,0) and (1,1)
        // Expected: 0 + (1-0) * (0.25-0)/(1-0) = 0.25
        assert!((interp.interpolate(0.25).unwrap() - 0.25).abs() < 1e-10);

        // Test at x = 1.5: interpolate between (1,1) and (2,4)
        // Expected: 1 + (4-1) * (1.5-1)/(2-1) = 1 + 3*0.5 = 2.5
        assert!((interp.interpolate(1.5).unwrap() - 2.5).abs() < 1e-10);

        // Test at x = 1.75: interpolate between (1,1) and (2,4)
        // Expected: 1 + (4-1) * (1.75-1)/(2-1) = 1 + 3*0.75 = 3.25
        assert!((interp.interpolate(1.75).unwrap() - 3.25).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_out_of_bounds_low() {
        let interp = LinearInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();
        let result = interp.interpolate(-0.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { x, min, max } => {
                assert!((x - (-0.1)).abs() < 1e-10);
                assert!((min - 0.0).abs() < 1e-10);
                assert!((max - 2.0).abs() < 1e-10);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_high() {
        let interp = LinearInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();
        let result = interp.interpolate(2.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { x, min, max } => {
                assert!((x - 2.1).abs() < 1e-10);
                assert!((min - 0.0).abs() < 1e-10);
                assert!((max - 2.0).abs() < 1e-10);
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_at_boundaries() {
        let interp = LinearInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();

        // Exactly at boundaries should work
        assert!(interp.interpolate(0.0).is_ok());
        assert!(interp.interpolate(2.0).is_ok());
    }

    #[test]
    fn test_interpolate_with_two_points() {
        // Minimum case: 2 points
        let interp = LinearInterpolator::new(&[0.0, 1.0], &[0.0, 2.0]).unwrap();

        assert!((interp.interpolate(0.0).unwrap() - 0.0).abs() < 1e-10);
        assert!((interp.interpolate(0.5).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_with_negative_y_values() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [-1.0, 0.0, -1.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        assert!((interp.interpolate(0.0).unwrap() - (-1.0)).abs() < 1e-10);
        assert!((interp.interpolate(0.5).unwrap() - (-0.5)).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 0.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - (-0.5)).abs() < 1e-10);
        assert!((interp.interpolate(2.0).unwrap() - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_constant_function() {
        // y = 5 everywhere
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [5.0, 5.0, 5.0, 5.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        assert!((interp.interpolate(0.0).unwrap() - 5.0).abs() < 1e-10);
        assert!((interp.interpolate(0.5).unwrap() - 5.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - 5.0).abs() < 1e-10);
        assert!((interp.interpolate(3.0).unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_non_uniform_spacing() {
        // Non-uniform x spacing
        let xs = [0.0, 0.1, 1.0, 10.0];
        let ys = [0.0, 1.0, 2.0, 3.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        // At x = 0.05: between (0,0) and (0.1,1)
        // t = 0.05/0.1 = 0.5, y = 0 + 1*0.5 = 0.5
        assert!((interp.interpolate(0.05).unwrap() - 0.5).abs() < 1e-10);

        // At x = 0.55: between (0.1,1) and (1,2)
        // t = (0.55-0.1)/(1-0.1) = 0.45/0.9 = 0.5, y = 1 + 1*0.5 = 1.5
        assert!((interp.interpolate(0.55).unwrap() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolate_f32() {
        let xs: [f32; 3] = [0.0, 1.0, 2.0];
        let ys: [f32; 3] = [0.0, 2.0, 4.0];
        let interp = LinearInterpolator::new(&xs, &ys).unwrap();

        let y = interp.interpolate(0.5_f32).unwrap();
        assert!((y - 1.0_f32).abs() < 1e-6);
    }
}

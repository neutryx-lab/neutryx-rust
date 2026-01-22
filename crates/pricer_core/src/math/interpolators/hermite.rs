//! Hermite cubic interpolation with user-specified derivatives.
//!
//! Hermite interpolation constructs a cubic polynomial between each pair
//! of data points, using both the function values and their derivatives
//! at each point. This provides C¹ continuity (continuous first derivatives).

use num_traits::Float;

use super::Interpolator;
use crate::types::InterpolationError;

/// Hermite cubic interpolator with specified derivatives.
///
/// Given points (x_i, y_i) and derivatives m_i = dy/dx at each point,
/// constructs a piecewise cubic polynomial that passes through all points
/// with the specified slopes.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{HermiteInterpolator, Interpolator};
///
/// let xs = [0.0_f64, 1.0, 2.0];
/// let ys = [0.0_f64, 1.0, 0.0];
/// let ms = [1.0_f64, 0.0, -1.0]; // Derivatives at each point
///
/// let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();
///
/// // At x = 0, y = 0 with slope 1
/// assert!((interp.interpolate(0.0).unwrap() - 0.0).abs() < 1e-10);
/// // At x = 1, y = 1 with slope 0 (local maximum)
/// assert!((interp.interpolate(1.0).unwrap() - 1.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct HermiteInterpolator<T: Float> {
    xs: Vec<T>,
    ys: Vec<T>,
    ms: Vec<T>, // Derivatives at each point
}

impl<T: Float> HermiteInterpolator<T> {
    /// Creates a new Hermite interpolator.
    ///
    /// # Arguments
    ///
    /// * `xs` - X coordinates (must be sorted in ascending order)
    /// * `ys` - Y values at each x
    /// * `ms` - Derivatives (slopes) at each x
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Fewer than 2 points provided
    /// - Arrays have different lengths
    /// - xs are not sorted
    pub fn new(xs: &[T], ys: &[T], ms: &[T]) -> Result<Self, InterpolationError> {
        if xs.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                got: xs.len(),
                need: 2,
            });
        }

        if xs.len() != ys.len() || xs.len() != ms.len() {
            return Err(InterpolationError::InsufficientData {
                got: ys.len().min(ms.len()),
                need: xs.len(),
            });
        }

        // Check sorted
        for i in 1..xs.len() {
            if xs[i] <= xs[i - 1] {
                return Err(InterpolationError::NonMonotonicData { index: i });
            }
        }

        Ok(Self {
            xs: xs.to_vec(),
            ys: ys.to_vec(),
            ms: ms.to_vec(),
        })
    }

    /// Finds the index i such that xs[i] <= x < xs[i+1].
    fn find_interval(&self, x: T) -> usize {
        let n = self.xs.len();
        let mut lo = 0;
        let mut hi = n - 1;

        while hi - lo > 1 {
            let mid = usize::midpoint(lo, hi);
            if x >= self.xs[mid] {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        lo
    }
}

impl<T: Float> Interpolator<T> for HermiteInterpolator<T> {
    fn interpolate(&self, x: T) -> Result<T, InterpolationError> {
        let (x_min, x_max) = self.domain();
        let x_f64 = x.to_f64().unwrap_or(0.0);
        let min_f64 = x_min.to_f64().unwrap_or(0.0);
        let max_f64 = x_max.to_f64().unwrap_or(0.0);

        if x < x_min || x > x_max {
            return Err(InterpolationError::OutOfBounds {
                x: x_f64,
                min: min_f64,
                max: max_f64,
            });
        }

        // Handle exact right boundary
        if x == x_max {
            return Ok(self.ys[self.ys.len() - 1]);
        }

        let i = self.find_interval(x);

        // Hermite basis functions
        let x0 = self.xs[i];
        let x1 = self.xs[i + 1];
        let y0 = self.ys[i];
        let y1 = self.ys[i + 1];
        let m0 = self.ms[i];
        let m1 = self.ms[i + 1];

        let h = x1 - x0;
        let t = (x - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions:
        // h00(t) = 2t³ - 3t² + 1
        // h10(t) = t³ - 2t² + t
        // h01(t) = -2t³ + 3t²
        // h11(t) = t³ - t²
        let two = T::from(2.0).unwrap();
        let three = T::from(3.0).unwrap();

        let h00 = two * t3 - three * t2 + T::one();
        let h10 = t3 - two * t2 + t;
        let h01 = -two * t3 + three * t2;
        let h11 = t3 - t2;

        // p(x) = h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1
        Ok(h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1)
    }

    fn domain(&self) -> (T, T) { (self.xs[0], self.xs[self.xs.len() - 1]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermite_at_data_points() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0, 4.0];
        let ms = [1.0_f64, 2.0, 3.0];

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();

        assert!((interp.interpolate(0.0).unwrap() - 0.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(2.0).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_hermite_linear_case() {
        // When derivatives match linear slope, should get linear interpolation
        let xs = [0.0_f64, 2.0];
        let ys = [0.0_f64, 4.0];
        let ms = [2.0_f64, 2.0]; // Constant slope = 2

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();

        // Should be exactly linear: y = 2x
        assert!((interp.interpolate(0.5).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_hermite_parabola() {
        // y = x², with dy/dx = 2x
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0, 4.0];
        let ms = [0.0_f64, 2.0, 4.0];

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();

        // Test intermediate points
        let y_half = interp.interpolate(0.5).unwrap();
        assert!((y_half - 0.25).abs() < 1e-10); // 0.5² = 0.25

        let y_1_5 = interp.interpolate(1.5).unwrap();
        assert!((y_1_5 - 2.25).abs() < 1e-10); // 1.5² = 2.25
    }

    #[test]
    fn test_hermite_domain() {
        let xs = [1.0_f64, 3.0, 5.0];
        let ys = [0.0_f64, 1.0, 0.0];
        let ms = [0.5_f64, 0.0, -0.5];

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();
        let (x_min, x_max) = interp.domain();

        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_hermite_out_of_bounds() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0, 0.0];
        let ms = [1.0_f64, 0.0, -1.0];

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();

        assert!(interp.interpolate(-0.1).is_err());
        assert!(interp.interpolate(2.1).is_err());
    }

    #[test]
    fn test_hermite_insufficient_points() {
        let xs = [1.0_f64];
        let ys = [2.0_f64];
        let ms = [0.0_f64];

        let result = HermiteInterpolator::new(&xs, &ys, &ms);
        assert!(result.is_err());
    }

    #[test]
    fn test_hermite_mismatched_arrays() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0]; // Wrong length
        let ms = [0.0_f64, 0.0, 0.0];

        let result = HermiteInterpolator::new(&xs, &ys, &ms);
        assert!(result.is_err());
    }

    #[test]
    fn test_hermite_unsorted_input() {
        let xs = [2.0_f64, 1.0, 3.0];
        let ys = [0.0_f64, 1.0, 0.0];
        let ms = [0.0_f64, 0.0, 0.0];

        let result = HermiteInterpolator::new(&xs, &ys, &ms);
        assert!(result.is_err());
    }

    #[test]
    fn test_hermite_symmetric_bump() {
        // Symmetric bump: y = 0 at x=0,2 with y=1 at x=1
        // Zero slopes at endpoints, zero slope at peak
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [0.0_f64, 1.0, 0.0];
        let ms = [0.0_f64, 0.0, 0.0];

        let interp = HermiteInterpolator::new(&xs, &ys, &ms).unwrap();

        // Should be symmetric around x=1
        let y_0_5 = interp.interpolate(0.5).unwrap();
        let y_1_5 = interp.interpolate(1.5).unwrap();

        assert!((y_0_5 - y_1_5).abs() < 1e-10);
    }
}

//! Flat (piecewise constant) interpolation.
//!
//! Flat interpolation returns the value of the nearest data point,
//! either to the left (floor) or right (ceiling) of the query point.
//! This is useful for step functions and certain financial conventions.

use num_traits::Float;

use super::Interpolator;
use crate::types::InterpolationError;

/// Mode for flat interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatMode {
    /// Use the value at the left (floor) data point.
    Left,
    /// Use the value at the right (ceiling) data point.
    Right,
}

/// Piecewise constant (flat) interpolator.
///
/// For a query point x between data points x\[i\] and x\[i+1\], returns:
/// - `Left` mode: y\[i\] (the value at the left boundary)
/// - `Right` mode: y\[i+1\] (the value at the right boundary)
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{FlatInterpolator, FlatMode, Interpolator};
///
/// let xs = [0.0_f64, 1.0, 2.0, 3.0];
/// let ys = [1.0_f64, 2.0, 4.0, 8.0];
///
/// let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Left).unwrap();
///
/// // At x = 0.5, returns y[0] = 1.0 (left mode)
/// assert!((interp.interpolate(0.5).unwrap() - 1.0).abs() < 1e-10);
///
/// // At x = 1.5, returns y[1] = 2.0 (left mode)
/// assert!((interp.interpolate(1.5).unwrap() - 2.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct FlatInterpolator<T: Float> {
    xs: Vec<T>,
    ys: Vec<T>,
    mode: FlatMode,
}

impl<T: Float> FlatInterpolator<T> {
    /// Creates a new flat interpolator.
    ///
    /// # Arguments
    ///
    /// * `xs` - X coordinates (must be sorted in ascending order)
    /// * `ys` - Y values corresponding to each x
    /// * `mode` - Whether to use left (floor) or right (ceiling) values
    ///
    /// # Errors
    ///
    /// Returns `InterpolationError::InsufficientPoints` if fewer than 2 points
    /// provided. Returns `InterpolationError::UnsortedInput` if xs are not
    /// sorted.
    pub fn new(xs: &[T], ys: &[T], mode: FlatMode) -> Result<Self, InterpolationError> {
        if xs.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                got: xs.len(),
                need: 2,
            });
        }

        if xs.len() != ys.len() {
            return Err(InterpolationError::InsufficientData {
                got: ys.len(),
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
            mode,
        })
    }

    /// Returns the interpolation mode.
    #[must_use]
    pub const fn mode(&self) -> FlatMode { self.mode }

    /// Finds the index i such that xs[i] <= x < xs[i+1].
    fn find_interval(&self, x: T) -> usize {
        let n = self.xs.len();

        // Binary search
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

impl<T: Float> Interpolator<T> for FlatInterpolator<T> {
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

        // Handle exact boundary
        if x == x_max {
            return Ok(self.ys[self.ys.len() - 1]);
        }

        let i = self.find_interval(x);

        match self.mode {
            FlatMode::Left => Ok(self.ys[i]),
            FlatMode::Right => Ok(self.ys[(i + 1).min(self.ys.len() - 1)]),
        }
    }

    fn domain(&self) -> (T, T) { (self.xs[0], self.xs[self.xs.len() - 1]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_left_mode() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];
        let ys = [1.0_f64, 2.0, 4.0, 8.0];

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Left).unwrap();

        // At exact points
        assert!((interp.interpolate(0.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(2.0).unwrap() - 4.0).abs() < 1e-10);
        assert!((interp.interpolate(3.0).unwrap() - 8.0).abs() < 1e-10);

        // Between points - should return left value
        assert!((interp.interpolate(0.5).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(2.9).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_right_mode() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];
        let ys = [1.0_f64, 2.0, 4.0, 8.0];

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Right).unwrap();

        // Between points - should return right value
        assert!((interp.interpolate(0.5).unwrap() - 2.0).abs() < 1e-10);
        assert!((interp.interpolate(1.5).unwrap() - 4.0).abs() < 1e-10);
        assert!((interp.interpolate(2.1).unwrap() - 8.0).abs() < 1e-10);

        // At exact right boundary
        assert!((interp.interpolate(3.0).unwrap() - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_domain() {
        let xs = [1.0_f64, 5.0, 10.0];
        let ys = [2.0_f64, 3.0, 4.0];

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Left).unwrap();
        let (x_min, x_max) = interp.domain();

        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_out_of_bounds() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [1.0_f64, 2.0, 3.0];

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Left).unwrap();

        assert!(interp.interpolate(-0.1).is_err());
        assert!(interp.interpolate(2.1).is_err());
    }

    #[test]
    fn test_flat_insufficient_points() {
        let xs = [1.0_f64];
        let ys = [2.0_f64];

        let result = FlatInterpolator::new(&xs, &ys, FlatMode::Left);
        assert!(result.is_err());
    }

    #[test]
    fn test_flat_unsorted_input() {
        let xs = [2.0_f64, 1.0, 3.0];
        let ys = [1.0_f64, 2.0, 3.0];

        let result = FlatInterpolator::new(&xs, &ys, FlatMode::Left);
        assert!(result.is_err());
    }

    #[test]
    fn test_flat_mode_accessor() {
        let xs = [0.0_f64, 1.0];
        let ys = [1.0_f64, 2.0];

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Left).unwrap();
        assert_eq!(interp.mode(), FlatMode::Left);

        let interp = FlatInterpolator::new(&xs, &ys, FlatMode::Right).unwrap();
        assert_eq!(interp.mode(), FlatMode::Right);
    }
}

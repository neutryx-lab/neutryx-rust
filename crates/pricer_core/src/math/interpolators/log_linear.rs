//! Log-linear interpolation.
//!
//! Log-linear interpolation performs linear interpolation in log-space,
//! which is particularly useful for discount factors where exponential
//! decay is expected: df(t) = exp(-r * t).
//!
//! For y values representing discount factors, this ensures that
//! interpolated values maintain the expected exponential structure.

use super::Interpolator;
use crate::types::InterpolationError;
use num_traits::Float;

/// Log-linear interpolator for discount factors.
///
/// Performs linear interpolation on ln(y), which is equivalent to
/// assuming exponential interpolation between points:
///
/// y(x) = exp(ln(y_i) + (ln(y_{i+1}) - ln(y_i)) * (x - x_i) / (x_{i+1} - x_i))
///
/// This is the standard interpolation method for discount factors in
/// yield curve construction.
///
/// # Requirements
///
/// All y values must be strictly positive (> 0).
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{LogLinearInterpolator, Interpolator};
///
/// // Discount factors: df(0) = 1.0, df(1) = 0.95, df(2) = 0.90
/// let xs = [0.0_f64, 1.0, 2.0];
/// let ys = [1.0_f64, 0.95, 0.90];
///
/// let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();
///
/// // Interpolated df at t = 0.5
/// let df = interp.interpolate(0.5).unwrap();
/// assert!(df > 0.95 && df < 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct LogLinearInterpolator<T: Float> {
    xs: Vec<T>,
    log_ys: Vec<T>, // Store ln(y) for efficiency
}

impl<T: Float> LogLinearInterpolator<T> {
    /// Creates a new log-linear interpolator.
    ///
    /// # Arguments
    ///
    /// * `xs` - X coordinates (must be sorted in ascending order)
    /// * `ys` - Y values (must all be strictly positive)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Fewer than 2 points provided
    /// - xs are not sorted
    /// - Any y value is not positive
    pub fn new(xs: &[T], ys: &[T]) -> Result<Self, InterpolationError> {
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

        // Check positive and compute log(y)
        let mut log_ys = Vec::with_capacity(ys.len());
        for &y in ys {
            if y <= T::zero() {
                return Err(InterpolationError::InvalidInput(
                    "Log-linear interpolation requires positive y values".to_string(),
                ));
            }
            log_ys.push(y.ln());
        }

        Ok(Self {
            xs: xs.to_vec(),
            log_ys,
        })
    }

    /// Finds the index i such that xs[i] <= x < xs[i+1].
    fn find_interval(&self, x: T) -> usize {
        let n = self.xs.len();

        // Binary search
        let mut lo = 0;
        let mut hi = n - 1;

        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if x >= self.xs[mid] {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        lo
    }
}

impl<T: Float> Interpolator<T> for LogLinearInterpolator<T> {
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
            return Ok(self.log_ys[self.log_ys.len() - 1].exp());
        }

        let i = self.find_interval(x);

        // Linear interpolation in log-space
        let x0 = self.xs[i];
        let x1 = self.xs[i + 1];
        let log_y0 = self.log_ys[i];
        let log_y1 = self.log_ys[i + 1];

        let t = (x - x0) / (x1 - x0);
        let log_y = log_y0 + t * (log_y1 - log_y0);

        Ok(log_y.exp())
    }

    fn domain(&self) -> (T, T) {
        (self.xs[0], self.xs[self.xs.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_linear_at_data_points() {
        let xs = [0.0_f64, 1.0, 2.0, 3.0];
        let ys = [1.0_f64, 0.95, 0.90, 0.85];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        // Should return exact values at data points
        assert!((interp.interpolate(0.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((interp.interpolate(1.0).unwrap() - 0.95).abs() < 1e-10);
        assert!((interp.interpolate(2.0).unwrap() - 0.90).abs() < 1e-10);
        assert!((interp.interpolate(3.0).unwrap() - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_log_linear_interpolation() {
        let xs = [0.0_f64, 1.0];
        let ys = [1.0_f64, 0.9];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        // At x = 0.5, the log-linear interpolation gives:
        // ln(y) = 0.5 * ln(1.0) + 0.5 * ln(0.9) = 0.5 * ln(0.9)
        // y = exp(0.5 * ln(0.9)) = 0.9^0.5 = sqrt(0.9)
        let expected = 0.9_f64.sqrt();
        let result = interp.interpolate(0.5).unwrap();

        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_log_linear_geometric_mean() {
        // Log-linear interpolation at midpoint equals geometric mean
        let xs = [0.0_f64, 1.0];
        let ys = [4.0_f64, 9.0];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        // Geometric mean of 4 and 9 is sqrt(36) = 6
        let expected = (4.0_f64 * 9.0).sqrt();
        let result = interp.interpolate(0.5).unwrap();

        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_log_linear_domain() {
        let xs = [1.0_f64, 5.0, 10.0];
        let ys = [0.99_f64, 0.95, 0.90];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();
        let (x_min, x_max) = interp.domain();

        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_log_linear_out_of_bounds() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [1.0_f64, 0.95, 0.90];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        assert!(interp.interpolate(-0.1).is_err());
        assert!(interp.interpolate(2.1).is_err());
    }

    #[test]
    fn test_log_linear_non_positive_values() {
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [1.0_f64, 0.0, 0.5]; // Zero value not allowed

        let result = LogLinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        let ys_neg = [1.0_f64, -0.5, 0.5]; // Negative value not allowed
        let result = LogLinearInterpolator::new(&xs, &ys_neg);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_linear_insufficient_points() {
        let xs = [1.0_f64];
        let ys = [0.95_f64];

        let result = LogLinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_linear_unsorted_input() {
        let xs = [2.0_f64, 1.0, 3.0];
        let ys = [0.90_f64, 0.95, 0.85];

        let result = LogLinearInterpolator::new(&xs, &ys);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_linear_discount_factor_scenario() {
        // Typical discount factor scenario
        // df(0) = 1.0, df(1) = exp(-0.05) ≈ 0.9512, df(2) = exp(-0.10) ≈ 0.9048
        let r = 0.05_f64;
        let xs = [0.0_f64, 1.0, 2.0];
        let ys = [1.0, (-r).exp(), (-2.0 * r).exp()];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        // For constant rate, log-linear should give exact discount factors
        let df_half = interp.interpolate(0.5).unwrap();
        let expected = (-0.5 * r).exp();

        assert!((df_half - expected).abs() < 1e-10);
    }

    #[test]
    fn test_log_linear_monotonicity() {
        // Decreasing discount factors should result in decreasing interpolated values
        let xs = [0.0_f64, 1.0, 2.0, 3.0];
        let ys = [1.0_f64, 0.95, 0.90, 0.85];

        let interp = LogLinearInterpolator::new(&xs, &ys).unwrap();

        let y0 = interp.interpolate(0.5).unwrap();
        let y1 = interp.interpolate(1.5).unwrap();
        let y2 = interp.interpolate(2.5).unwrap();

        assert!(y0 > y1);
        assert!(y1 > y2);
    }
}

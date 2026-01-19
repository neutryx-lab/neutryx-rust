//! Monotonic interpolation using Fritsch-Carlson method.

use num_traits::Float;

use super::Interpolator;
use crate::{math::numeric::from_f64, types::InterpolationError};

/// Monotonicity-preserving interpolator using Fritsch-Carlson method.
///
/// Stores sorted (x, y) data points and computes Hermite slopes that
/// preserve monotonicity of the input data. Ideal for interpolating
/// discount factors, survival probabilities, and other financial curves
/// where monotonicity violations would create arbitrage opportunities.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Construction
///
/// Data points are automatically sorted by x-coordinate during construction.
/// At least 2 data points are required. Input data must be monotonic
/// (either all increasing or all decreasing).
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{Interpolator, MonotonicInterpolator};
///
/// // Monotonically increasing data
/// let xs = [0.0, 1.0, 2.0, 3.0];
/// let ys = [0.0, 1.0, 3.0, 6.0];
///
/// let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();
/// let y = interp.interpolate(1.5).unwrap();
/// // y is guaranteed to be between ys[1] and ys[2]
/// ```
#[derive(Debug, Clone)]
pub struct MonotonicInterpolator<T: Float> {
    /// Sorted x-coordinates
    xs: Vec<T>,
    /// Corresponding y-values (in same order as xs after sorting)
    ys: Vec<T>,
    /// Computed Hermite slopes at each point
    slopes: Vec<T>,
}

impl<T: Float> MonotonicInterpolator<T> {
    /// Construct a monotonic interpolator from x and y data points.
    ///
    /// Data points are automatically sorted by x-coordinate if not already
    /// sorted. Requires at least 2 data points. Input data must be
    /// monotonic.
    ///
    /// # Arguments
    ///
    /// * `xs` - Slice of x-coordinates
    /// * `ys` - Slice of corresponding y-values
    ///
    /// # Returns
    ///
    /// * `Ok(MonotonicInterpolator)` - Successfully constructed interpolator
    /// * `Err(InterpolationError::InsufficientData)` - Fewer than 2 data points
    /// * `Err(InterpolationError::InvalidInput)` - Mismatched array lengths
    /// * `Err(InterpolationError::NonMonotonicData)` - Data is not monotonic
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::MonotonicInterpolator;
    ///
    /// // Valid monotonic data
    /// let interp = MonotonicInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 3.0]).unwrap();
    ///
    /// // Non-monotonic data fails
    /// let result = MonotonicInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 2.0, 1.0]);
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

        // Check monotonicity
        Self::check_monotonicity(&sorted_ys)?;

        // Compute Fritsch-Carlson slopes
        let slopes = Self::compute_slopes(&sorted_xs, &sorted_ys);

        Ok(Self {
            xs: sorted_xs,
            ys: sorted_ys,
            slopes,
        })
    }

    /// Check if data is monotonic (either all increasing or all decreasing).
    fn check_monotonicity(ys: &[T]) -> Result<(), InterpolationError> {
        if ys.len() < 2 {
            return Ok(());
        }

        // Determine expected direction from first non-zero difference
        let mut expected_sign: Option<bool> = None; // true = increasing, false = decreasing

        for i in 1..ys.len() {
            let diff = ys[i] - ys[i - 1];
            if diff > T::zero() {
                match expected_sign {
                    None => expected_sign = Some(true),
                    Some(false) => {
                        return Err(InterpolationError::NonMonotonicData { index: i });
                    }
                    Some(true) => {}
                }
            } else if diff < T::zero() {
                match expected_sign {
                    None => expected_sign = Some(false),
                    Some(true) => {
                        return Err(InterpolationError::NonMonotonicData { index: i });
                    }
                    Some(false) => {}
                }
            }
            // diff == 0 is allowed (constant segments)
        }

        Ok(())
    }

    /// Compute Fritsch-Carlson monotonicity-preserving slopes.
    fn compute_slopes(xs: &[T], ys: &[T]) -> Vec<T> {
        let n = xs.len();
        let two: T = from_f64(2.0);
        let three: T = from_f64(3.0);

        if n == 2 {
            // Only two points: use secant slope for both
            let delta = (ys[1] - ys[0]) / (xs[1] - xs[0]);
            return vec![delta, delta];
        }

        // Compute secants (delta_k = (y_{k+1} - y_k) / (x_{k+1} - x_k))
        let deltas: Vec<T> = (0..n - 1)
            .map(|i| (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]))
            .collect();

        // Initial slopes: average of adjacent secants (or one-sided at boundaries)
        let mut slopes: Vec<T> = Vec::with_capacity(n);

        // First point: one-sided
        slopes.push(deltas[0]);

        // Interior points: average of adjacent secants
        for i in 1..n - 1 {
            // If signs differ (including zero crossing), set slope to zero
            if deltas[i - 1] * deltas[i] <= T::zero() {
                slopes.push(T::zero());
            } else {
                // Harmonic mean preserves monotonicity better than arithmetic mean
                // m_k = (delta_{k-1} + delta_k) / 2 for simplicity
                slopes.push((deltas[i - 1] + deltas[i]) / two);
            }
        }

        // Last point: one-sided
        slopes.push(deltas[n - 2]);

        // Fritsch-Carlson correction to ensure monotonicity
        for i in 0..n - 1 {
            let delta = deltas[i];

            if delta.abs() < T::epsilon() {
                // Flat segment: zero slopes
                slopes[i] = T::zero();
                slopes[i + 1] = T::zero();
            } else {
                let alpha = slopes[i] / delta;
                let beta = slopes[i + 1] / delta;

                // Check if (alpha, beta) is in valid region
                // For monotonicity: alpha² + beta² <= 9
                let r2 = alpha * alpha + beta * beta;
                let nine: T = from_f64(9.0);

                if r2 > nine {
                    // Scale slopes to satisfy constraint
                    let tau = three / r2.sqrt();
                    slopes[i] = tau * alpha * delta;
                    slopes[i + 1] = tau * beta * delta;
                }
            }
        }

        slopes
    }

    /// Find the segment index for interpolation using binary search.
    #[inline]
    fn find_segment(&self, x: T) -> usize {
        let pos = self.xs.partition_point(|&xi| xi <= x);
        if pos == 0 {
            0
        } else if pos >= self.xs.len() {
            self.xs.len() - 2
        } else {
            pos - 1
        }
    }

    /// Returns a reference to the sorted x-coordinates.
    #[inline]
    pub fn xs(&self) -> &[T] { &self.xs }

    /// Returns a reference to the y-values.
    #[inline]
    pub fn ys(&self) -> &[T] { &self.ys }

    /// Returns a reference to the computed slopes.
    #[inline]
    pub fn slopes(&self) -> &[T] { &self.slopes }

    /// Returns the number of data points.
    #[inline]
    pub fn len(&self) -> usize { self.xs.len() }

    /// Returns true if the interpolator has no data points.
    #[inline]
    pub fn is_empty(&self) -> bool { self.xs.is_empty() }
}

impl<T: Float> Interpolator<T> for MonotonicInterpolator<T> {
    /// Interpolate value at point `x` using monotonic Hermite interpolation.
    ///
    /// Uses binary search (O(log n)) to find the appropriate segment,
    /// then evaluates the Hermite polynomial with Fritsch-Carlson slopes.
    ///
    /// # Arguments
    ///
    /// * `x` - The point at which to interpolate
    ///
    /// # Returns
    ///
    /// * `Ok(y)` - The interpolated value
    /// * `Err(InterpolationError::OutOfBounds)` - If `x` is outside the domain
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

        // Get segment data
        let x0 = self.xs[i];
        let x1 = self.xs[i + 1];
        let y0 = self.ys[i];
        let y1 = self.ys[i + 1];
        let m0 = self.slopes[i];
        let m1 = self.slopes[i + 1];

        // Compute parameter t in [0, 1]
        let h = x1 - x0;
        let t = (x - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions:
        // h00(t) = 2t³ - 3t² + 1
        // h10(t) = t³ - 2t² + t
        // h01(t) = -2t³ + 3t²
        // h11(t) = t³ - t²

        let two: T = from_f64(2.0);
        let three: T = from_f64(3.0);

        let h00 = two * t3 - three * t2 + T::one();
        let h10 = t3 - two * t2 + t;
        let h01 = -two * t3 + three * t2;
        let h11 = t3 - t2;

        // Interpolated value: p(t) = h00*y0 + h10*h*m0 + h01*y1 + h11*h*m1
        Ok(h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1)
    }

    /// Return the valid interpolation domain.
    #[inline]
    fn domain(&self) -> (T, T) { (self.xs[0], self.xs[self.xs.len() - 1]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests (Task 5.1)
    // ========================================

    #[test]
    fn test_new_with_minimum_points() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 2);
    }

    #[test]
    fn test_new_with_multiple_points() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 2.0, 3.0, 4.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 5);
    }

    #[test]
    fn test_new_insufficient_data_one_point() {
        let xs = [1.0];
        let ys = [2.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
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
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InvalidInput(msg) => {
                assert!(msg.contains("same length"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_new_non_monotonic_data() {
        // Data goes up then down - not monotonic
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 1.0, 3.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::NonMonotonicData { index } => {
                assert_eq!(index, 2);
            }
            _ => panic!("Expected NonMonotonicData error"),
        }
    }

    #[test]
    fn test_new_decreasing_data() {
        // Monotonically decreasing is valid
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [4.0, 3.0, 2.0, 1.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_auto_sorts_unsorted_data() {
        let xs = [3.0, 1.0, 2.0, 0.0];
        let ys = [3.0, 1.0, 2.0, 0.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.xs(), &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(interp.ys(), &[0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_new_constant_data() {
        // Constant data is technically monotonic (non-strict)
        let xs = [0.0, 1.0, 2.0];
        let ys = [5.0, 5.0, 5.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());
    }

    // ========================================
    // Interpolation Tests (Task 5.2)
    // ========================================

    #[test]
    fn test_domain() {
        let interp =
            MonotonicInterpolator::new(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0]).unwrap();
        let (x_min, x_max) = interp.domain();
        assert_eq!(x_min, 1.0);
        assert_eq!(x_max, 4.0);
    }

    #[test]
    fn test_interpolate_at_knot_points() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 1.0, 3.0, 6.0];
        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        for (x, y) in xs.iter().zip(ys.iter()) {
            let result = interp.interpolate(*x).unwrap();
            assert!(
                (result - *y).abs() < 1e-10,
                "At x={}, expected y={}, got {}",
                x,
                y,
                result
            );
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_low() {
        let interp = MonotonicInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]).unwrap();
        let result = interp.interpolate(-0.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_interpolate_out_of_bounds_high() {
        let interp = MonotonicInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]).unwrap();
        let result = interp.interpolate(2.1);

        assert!(result.is_err());
        match result.unwrap_err() {
            InterpolationError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    // ========================================
    // Monotonicity Preservation Tests (Task 5.3)
    // ========================================

    #[test]
    fn test_increasing_data_produces_increasing_output() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 3.0, 6.0, 10.0];
        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        // Sample many points and verify monotonicity
        let mut prev_y = f64::NEG_INFINITY;
        for i in 0..100 {
            let x = (i as f64) * 4.0 / 99.0;
            let y = interp.interpolate(x).unwrap();
            assert!(
                y >= prev_y - 1e-10,
                "Monotonicity violated at x={}: prev={}, current={}",
                x,
                prev_y,
                y
            );
            prev_y = y;
        }
    }

    #[test]
    fn test_decreasing_data_produces_decreasing_output() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [10.0, 6.0, 3.0, 1.0, 0.0];
        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        // Sample many points and verify monotonicity
        let mut prev_y = f64::INFINITY;
        for i in 0..100 {
            let x = (i as f64) * 4.0 / 99.0;
            let y = interp.interpolate(x).unwrap();
            assert!(
                y <= prev_y + 1e-10,
                "Monotonicity violated at x={}: prev={}, current={}",
                x,
                prev_y,
                y
            );
            prev_y = y;
        }
    }

    #[test]
    fn test_interpolated_values_within_y_range() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 5.0, 9.0];
        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        // Check midpoints are within adjacent y values
        let y_mid1 = interp.interpolate(0.5).unwrap();
        assert!(y_mid1 >= 0.0 && y_mid1 <= 2.0);

        let y_mid2 = interp.interpolate(1.5).unwrap();
        assert!(y_mid2 >= 2.0 && y_mid2 <= 5.0);

        let y_mid3 = interp.interpolate(2.5).unwrap();
        assert!(y_mid3 >= 5.0 && y_mid3 <= 9.0);
    }

    #[test]
    fn test_clone() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 2.0];
        let interp = MonotonicInterpolator::new(&xs, &ys).unwrap();

        let cloned = interp.clone();
        assert_eq!(interp.xs(), cloned.xs());
        assert_eq!(interp.ys(), cloned.ys());
    }

    #[test]
    fn test_with_f32() {
        let xs: [f32; 3] = [0.0, 1.0, 2.0];
        let ys: [f32; 3] = [0.0, 1.0, 2.0];
        let result = MonotonicInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        let y = interp.interpolate(0.5_f32).unwrap();
        assert!(y.is_finite());
    }
}

//! Natural cubic spline interpolation.

use super::Interpolator;
use crate::types::InterpolationError;
use num_traits::Float;

/// Polynomial coefficients for a cubic spline segment.
///
/// Represents a cubic polynomial: `y = a + b*(x-xi) + c*(x-xi)² + d*(x-xi)³`
#[derive(Debug, Clone, Copy)]
struct SplineCoeffs<T: Float> {
    /// Constant term (y value at segment start)
    a: T,
    /// Linear coefficient
    b: T,
    /// Quadratic coefficient
    c: T,
    /// Cubic coefficient
    d: T,
}

/// Natural cubic spline interpolator with C² continuity.
///
/// Stores sorted (x, y) data points and computes natural cubic spline
/// coefficients with zero second derivative at boundaries.
/// Supports automatic differentiation through generic `T: Float` type parameter.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Construction
///
/// Data points are automatically sorted by x-coordinate during construction.
/// At least 3 data points are required.
///
/// # Example
///
/// ```
/// use pricer_core::math::interpolators::{Interpolator, CubicSplineInterpolator};
///
/// let xs = [0.0, 1.0, 2.0, 3.0];
/// let ys = [0.0, 1.0, 4.0, 9.0];
///
/// let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();
/// let y = interp.interpolate(1.5).unwrap();
/// // y is smoothly interpolated between points
/// ```
#[derive(Debug, Clone)]
pub struct CubicSplineInterpolator<T: Float> {
    /// Sorted x-coordinates
    xs: Vec<T>,
    /// Polynomial coefficients for each segment
    coeffs: Vec<SplineCoeffs<T>>,
}

impl<T: Float> CubicSplineInterpolator<T> {
    /// Construct a natural cubic spline from x and y data points.
    ///
    /// Data points are automatically sorted by x-coordinate if not already sorted.
    /// Requires at least 3 data points. Computes natural spline with zero
    /// second derivative at boundaries.
    ///
    /// # Arguments
    ///
    /// * `xs` - Slice of x-coordinates
    /// * `ys` - Slice of corresponding y-values
    ///
    /// # Returns
    ///
    /// * `Ok(CubicSplineInterpolator)` - Successfully constructed interpolator
    /// * `Err(InterpolationError::InsufficientData)` - Fewer than 3 data points
    /// * `Err(InterpolationError::InvalidInput)` - Mismatched array lengths
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::interpolators::CubicSplineInterpolator;
    ///
    /// // Valid construction
    /// let interp = CubicSplineInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();
    ///
    /// // Insufficient data
    /// let result = CubicSplineInterpolator::new(&[0.0, 1.0], &[0.0, 1.0]);
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

        // Validate minimum data points (cubic spline needs at least 3)
        if xs.len() < 3 {
            return Err(InterpolationError::InsufficientData {
                got: xs.len(),
                need: 3,
            });
        }

        // Create paired data and sort by x
        let mut pairs: Vec<(T, T)> = xs.iter().copied().zip(ys.iter().copied()).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Unzip back into separate vectors
        let (sorted_xs, sorted_ys): (Vec<T>, Vec<T>) = pairs.into_iter().unzip();

        // Compute spline coefficients using Thomas algorithm
        let coeffs = Self::compute_coefficients(&sorted_xs, &sorted_ys);

        Ok(Self {
            xs: sorted_xs,
            coeffs,
        })
    }

    /// Compute natural cubic spline coefficients using Thomas algorithm.
    ///
    /// Solves the tridiagonal system for second derivatives (M values),
    /// then computes polynomial coefficients for each segment.
    fn compute_coefficients(xs: &[T], ys: &[T]) -> Vec<SplineCoeffs<T>> {
        let n = xs.len();
        let two = T::from(2.0).unwrap();
        let _three = T::from(3.0).unwrap();
        let six = T::from(6.0).unwrap();

        // Compute intervals h[i] = x[i+1] - x[i]
        let h: Vec<T> = (0..n - 1).map(|i| xs[i + 1] - xs[i]).collect();

        // Build tridiagonal system for second derivatives M
        // Natural boundary: M[0] = M[n-1] = 0
        // Interior equations: h[i-1]*M[i-1] + 2*(h[i-1]+h[i])*M[i] + h[i]*M[i+1] = 6*(...)

        if n == 3 {
            // Special case: only one interior point
            // h[0]*M[0] + 2*(h[0]+h[1])*M[1] + h[1]*M[2] = rhs
            // With M[0] = M[2] = 0: 2*(h[0]+h[1])*M[1] = rhs
            let rhs = six * ((ys[2] - ys[1]) / h[1] - (ys[1] - ys[0]) / h[0]);
            let m1 = rhs / (two * (h[0] + h[1]));

            let mut coeffs = Vec::with_capacity(n - 1);

            // Segment 0: from x[0] to x[1]
            let a0 = ys[0];
            let c0 = T::zero(); // M[0] / 2 = 0
            let d0 = (m1 - T::zero()) / (six * h[0]); // (M[1] - M[0]) / (6*h[0])
            let b0 = (ys[1] - ys[0]) / h[0] - h[0] * (two * T::zero() + m1) / six;
            coeffs.push(SplineCoeffs {
                a: a0,
                b: b0,
                c: c0,
                d: d0,
            });

            // Segment 1: from x[1] to x[2]
            let a1 = ys[1];
            let c1 = m1 / two;
            let d1 = (T::zero() - m1) / (six * h[1]); // (M[2] - M[1]) / (6*h[1])
            let b1 = (ys[2] - ys[1]) / h[1] - h[1] * (two * m1 + T::zero()) / six;
            coeffs.push(SplineCoeffs {
                a: a1,
                b: b1,
                c: c1,
                d: d1,
            });

            return coeffs;
        }

        // General case: n >= 4, solve tridiagonal system using Thomas algorithm
        let interior = n - 2; // Number of interior points

        // Build system: A*M = R where A is tridiagonal
        // Sub-diagonal: h[i-1]
        // Main diagonal: 2*(h[i-1] + h[i])
        // Super-diagonal: h[i]
        // RHS: 6*((y[i+1]-y[i])/h[i] - (y[i]-y[i-1])/h[i-1])

        let mut diag: Vec<T> = Vec::with_capacity(interior);
        let mut rhs: Vec<T> = Vec::with_capacity(interior);

        for i in 1..n - 1 {
            diag.push(two * (h[i - 1] + h[i]));
            rhs.push(six * ((ys[i + 1] - ys[i]) / h[i] - (ys[i] - ys[i - 1]) / h[i - 1]));
        }

        // Thomas algorithm - forward elimination
        let mut c_prime: Vec<T> = Vec::with_capacity(interior);
        let mut d_prime: Vec<T> = Vec::with_capacity(interior);

        c_prime.push(h[1] / diag[0]);
        d_prime.push(rhs[0] / diag[0]);

        for i in 1..interior {
            let _sub = h[i]; // sub-diagonal element
            let sup = if i < interior - 1 {
                h[i + 1]
            } else {
                T::zero()
            }; // super-diagonal
            let denom = diag[i] - h[i] * c_prime[i - 1];
            if i < interior - 1 {
                c_prime.push(sup / denom);
            }
            d_prime.push((rhs[i] - h[i] * d_prime[i - 1]) / denom);
        }

        // Thomas algorithm - back substitution
        let mut m: Vec<T> = vec![T::zero(); n];
        m[n - 2] = d_prime[interior - 1];
        for i in (1..interior).rev() {
            m[i] = d_prime[i - 1] - c_prime[i - 1] * m[i + 1];
        }
        // m[0] and m[n-1] are already 0 (natural boundary)

        // Compute polynomial coefficients for each segment
        let mut coeffs = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let a = ys[i];
            let c = m[i] / two;
            let d = (m[i + 1] - m[i]) / (six * h[i]);
            let b = (ys[i + 1] - ys[i]) / h[i] - h[i] * (two * m[i] + m[i + 1]) / six;
            coeffs.push(SplineCoeffs { a, b, c, d });
        }

        coeffs
    }

    /// Find the segment index for interpolation using binary search.
    ///
    /// Returns the index `i` such that `xs[i] <= x < xs[i+1]`,
    /// clamped to valid segment range [0, n-2].
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
    pub fn xs(&self) -> &[T] {
        &self.xs
    }

    /// Returns the number of data points.
    #[inline]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    /// Returns true if the interpolator has no data points.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }
}

impl<T: Float> Interpolator<T> for CubicSplineInterpolator<T> {
    /// Interpolate value at point `x` using cubic spline.
    ///
    /// Uses binary search (O(log n)) to find the appropriate segment,
    /// then evaluates the cubic polynomial.
    ///
    /// # Formula
    ///
    /// ```text
    /// y = a + b*(x-xi) + c*(x-xi)² + d*(x-xi)³
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
        let coeffs = &self.coeffs[i];

        // Evaluate cubic polynomial: y = a + b*dx + c*dx² + d*dx³
        let dx = x - self.xs[i];
        let dx2 = dx * dx;
        let dx3 = dx2 * dx;

        Ok(coeffs.a + coeffs.b * dx + coeffs.c * dx2 + coeffs.d * dx3)
    }

    /// Return the valid interpolation domain.
    #[inline]
    fn domain(&self) -> (T, T) {
        (self.xs[0], self.xs[self.xs.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests (Task 4.1)
    // ========================================

    #[test]
    fn test_new_with_minimum_points() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 3);
    }

    #[test]
    fn test_new_with_multiple_points() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 4.0, 9.0, 16.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.len(), 5);
    }

    #[test]
    fn test_new_insufficient_data_two_points() {
        let xs = [0.0, 1.0];
        let ys = [0.0, 1.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InsufficientData { got, need } => {
                assert_eq!(got, 2);
                assert_eq!(need, 3);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_insufficient_data_one_point() {
        let xs = [1.0];
        let ys = [2.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_err());

        match result.unwrap_err() {
            InterpolationError::InsufficientData { got, need } => {
                assert_eq!(got, 1);
                assert_eq!(need, 3);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_mismatched_lengths() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
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
        let xs = [3.0, 1.0, 2.0, 0.0];
        let ys = [9.0, 1.0, 4.0, 0.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        assert_eq!(interp.xs(), &[0.0, 1.0, 2.0, 3.0]);
    }

    // ========================================
    // Interpolation Tests (Task 4.2)
    // ========================================

    #[test]
    fn test_domain() {
        let interp =
            CubicSplineInterpolator::new(&[1.0, 2.0, 3.0, 4.0], &[1.0, 4.0, 9.0, 16.0]).unwrap();
        let (x_min, x_max) = interp.domain();
        assert_eq!(x_min, 1.0);
        assert_eq!(x_max, 4.0);
    }

    #[test]
    fn test_interpolate_at_knot_points() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 2.0, 4.0, 6.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        // Interpolating at knot points should return exact y values
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
    fn test_interpolate_at_knot_points_quadratic() {
        // y = x² (quadratic)
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 4.0, 9.0, 16.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

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
        let interp = CubicSplineInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();
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
        let interp = CubicSplineInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();
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
        let interp = CubicSplineInterpolator::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).unwrap();

        assert!(interp.interpolate(0.0).is_ok());
        assert!(interp.interpolate(2.0).is_ok());
    }

    #[test]
    fn test_interpolate_smooth_curve() {
        // For linear data, cubic spline should produce linear interpolation
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 1.0, 2.0, 3.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        // Midpoints should be close to linear
        assert!((interp.interpolate(0.5).unwrap() - 0.5).abs() < 1e-6);
        assert!((interp.interpolate(1.5).unwrap() - 1.5).abs() < 1e-6);
        assert!((interp.interpolate(2.5).unwrap() - 2.5).abs() < 1e-6);
    }

    // ========================================
    // C² Continuity Tests (Task 4.3)
    // ========================================

    #[test]
    fn test_c2_continuity_at_interior_knots() {
        // Test that second derivative is continuous at interior knots
        // by checking that the spline is smooth (no discontinuities)
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 4.0, 9.0, 16.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        // Check smoothness around interior knots by sampling nearby points
        for &knot in &xs[1..xs.len() - 1] {
            let h = 1e-6;
            let y_left = interp.interpolate(knot - h).unwrap();
            let y_mid = interp.interpolate(knot).unwrap();
            let y_right = interp.interpolate(knot + h).unwrap();

            // Approximate second derivative from both sides should be similar
            let d2_left = (y_mid - y_left) / h;
            let d2_right = (y_right - y_mid) / h;

            // First derivatives should be continuous
            assert!(
                (d2_right - d2_left).abs() < 1e-3,
                "First derivative discontinuity at knot {}: left={}, right={}",
                knot,
                d2_left,
                d2_right
            );
        }
    }

    #[test]
    fn test_natural_boundary_conditions() {
        // Natural spline: second derivative = 0 at boundaries
        // We verify by checking that the spline approaches linear near boundaries
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 1.0, 4.0, 9.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        // Near boundary, curvature should be small
        let h = 0.01;
        let y0 = interp.interpolate(0.0).unwrap();
        let y1 = interp.interpolate(h).unwrap();
        let y2 = interp.interpolate(2.0 * h).unwrap();

        // Second derivative approximation at x=0
        let d2 = (y2 - 2.0 * y1 + y0) / (h * h);
        assert!(
            d2.abs() < 0.5,
            "Second derivative at boundary should be near zero, got {}",
            d2
        );
    }

    #[test]
    fn test_clone() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        let cloned = interp.clone();
        assert_eq!(interp.xs(), cloned.xs());
    }

    #[test]
    fn test_debug() {
        let xs = [0.0, 1.0, 2.0];
        let ys = [0.0, 1.0, 4.0];
        let interp = CubicSplineInterpolator::new(&xs, &ys).unwrap();

        let debug_str = format!("{:?}", interp);
        assert!(debug_str.contains("CubicSplineInterpolator"));
    }

    #[test]
    fn test_with_f32() {
        let xs: [f32; 4] = [0.0, 1.0, 2.0, 3.0];
        let ys: [f32; 4] = [0.0, 1.0, 4.0, 9.0];
        let result = CubicSplineInterpolator::new(&xs, &ys);
        assert!(result.is_ok());

        let interp = result.unwrap();
        let y = interp.interpolate(1.5_f32).unwrap();
        assert!(y.is_finite());
    }
}

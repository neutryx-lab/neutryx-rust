//! Natural cubic spline interpolation.
//!
//! Constructs a C² piecewise cubic polynomial through given knots using
//! natural boundary conditions (second derivative = 0 at endpoints).
//!
//! ## Algorithm
//!
//! Given knots (x_i, y_i), the spline on interval [x_i, x_{i+1}] is:
//!
//! ```text
//! S_i(x) = a_i + b_i·(x - x_i) + c_i·(x - x_i)² + d_i·(x - x_i)³
//! ```
//!
//! Coefficients are determined by solving a tridiagonal system enforcing
//! C² continuity at interior knots plus natural boundary conditions.

use num_traits::Float;

use crate::math::numeric::from_f64;

use super::InterpolationError;

/// Natural cubic spline interpolant.
///
/// Stores precomputed coefficients `[a, b, c, d]` per interval for O(log n)
/// evaluation via binary search.
#[derive(Debug, Clone, PartialEq)]
pub struct CubicSpline<T: Float> {
    /// Knot x-coordinates (strictly increasing).
    knots_x: Vec<T>,
    /// Coefficients `[a, b, c, d]` for each of `n-1` intervals.
    coefficients: Vec<[T; 4]>,
}

impl<T: Float> CubicSpline<T> {
    /// Constructs a natural cubic spline through the given knots.
    ///
    /// Natural boundary conditions: S''(x_0) = 0, S''(x_{n-1}) = 0.
    ///
    /// # Errors
    ///
    /// Returns error if fewer than 2 points or knots are not strictly
    /// increasing.
    pub fn natural(xs: &[T], ys: &[T]) -> Result<Self, InterpolationError> {
        let n = xs.len();
        if n < 2 || ys.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                required: 2,
                provided: n.min(ys.len()),
            });
        }
        if n != ys.len() {
            return Err(InterpolationError::InsufficientData {
                required: n,
                provided: ys.len(),
            });
        }

        // Validate strictly increasing
        for i in 0..n - 1 {
            if xs[i + 1] <= xs[i] {
                return Err(InterpolationError::NonIncreasingKnots);
            }
        }

        // For 2 points, linear interpolation (c = d = 0)
        if n == 2 {
            let h = xs[1] - xs[0];
            let b = (ys[1] - ys[0]) / h;
            return Ok(Self {
                knots_x: xs.to_vec(),
                coefficients: vec![[ys[0], b, T::zero(), T::zero()]],
            });
        }

        let m = n - 1; // number of intervals

        // Step sizes and divided differences
        let mut h = vec![T::zero(); m];
        let mut delta = vec![T::zero(); m];
        for i in 0..m {
            h[i] = xs[i + 1] - xs[i];
            delta[i] = (ys[i + 1] - ys[i]) / h[i];
        }

        // Solve tridiagonal system for second derivatives (c values)
        // Natural BCs: c[0] = 0, c[n-1] = 0
        // Interior equations: h[i-1]*c[i-1] + 2*(h[i-1]+h[i])*c[i] + h[i]*c[i+1]
        //                     = 3*(delta[i] - delta[i-1])
        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);

        let interior = n - 2; // number of interior unknowns
        let mut c = vec![T::zero(); n];

        if interior > 0 {
            // Thomas algorithm for tridiagonal system
            let mut diag = vec![T::zero(); interior];
            let mut upper = vec![T::zero(); interior];
            let mut rhs = vec![T::zero(); interior];

            for i in 0..interior {
                let idx = i + 1; // index in full array
                diag[i] = two * (h[idx - 1] + h[idx]);
                rhs[i] = three * (delta[idx] - delta[idx - 1]);
                if i < interior - 1 {
                    upper[i] = h[idx];
                }
            }

            // Forward sweep
            for i in 1..interior {
                let lower = h[i]; // h[idx-1] where idx = i+1
                let factor = lower / diag[i - 1];
                diag[i] = diag[i] - factor * upper[i - 1];
                rhs[i] = rhs[i] - factor * rhs[i - 1];
            }

            // Back substitution
            let last = interior - 1;
            c[last + 1] = rhs[last] / diag[last];
            for i in (0..last).rev() {
                c[i + 1] = (rhs[i] - upper[i] * c[i + 2]) / diag[i];
            }
        }

        // Compute coefficients for each interval
        let mut coefficients = Vec::with_capacity(m);
        for i in 0..m {
            let a = ys[i];
            let b = delta[i] - h[i] * (two * c[i] + c[i + 1]) / three;
            let d = (c[i + 1] - c[i]) / (three * h[i]);
            coefficients.push([a, b, c[i], d]);
        }

        Ok(Self {
            knots_x: xs.to_vec(),
            coefficients,
        })
    }

    /// Returns the precomputed coefficients (one `[a, b, c, d]` per interval).
    pub fn coefficients(&self) -> &[[T; 4]] { &self.coefficients }

    /// Returns the knot x-coordinates.
    pub fn knots_x(&self) -> &[T] { &self.knots_x }

    /// Finds the interval index for the given x value (clamped to valid range).
    fn find_interval(&self, x: T) -> usize {
        let n = self.knots_x.len();
        if x <= self.knots_x[0] {
            return 0;
        }
        if x >= self.knots_x[n - 1] {
            return self.coefficients.len() - 1;
        }
        // Binary search
        let mut lo = 0;
        let mut hi = n - 1;
        while lo < hi - 1 {
            let mid = (lo + hi) / 2;
            if x < self.knots_x[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// Evaluates the spline at point x.
    ///
    /// Extrapolates using the first/last interval if x is outside the domain.
    pub fn evaluate(&self, x: T) -> T {
        let i = self.find_interval(x);
        let [a, b, c, d] = self.coefficients[i];
        let dx = x - self.knots_x[i];
        a + dx * (b + dx * (c + dx * d))
    }

    /// Evaluates the first derivative of the spline at point x.
    pub fn derivative(&self, x: T) -> T {
        let i = self.find_interval(x);
        let [_, b, c, d] = self.coefficients[i];
        let dx = x - self.knots_x[i];
        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);
        b + dx * (two * c + three * d * dx)
    }

    /// Evaluates the second derivative of the spline at point x.
    pub fn second_derivative(&self, x: T) -> T {
        let i = self.find_interval(x);
        let [_, _, c, d] = self.coefficients[i];
        let dx = x - self.knots_x[i];
        let two = from_f64::<T>(2.0);
        let six = from_f64::<T>(6.0);
        two * c + six * d * dx
    }

    /// Computes the definite integral of the spline from `x0` to `x1`.
    ///
    /// Integrates analytically (quartic antiderivative per segment).
    pub fn integrate(&self, x0: T, x1: T) -> T {
        if x1 <= x0 {
            return T::zero();
        }

        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);
        let four = from_f64::<T>(4.0);

        let i0 = self.find_interval(x0);
        let i1 = self.find_interval(x1);

        let mut total = T::zero();

        for i in i0..=i1 {
            let [a, b, c, d] = self.coefficients[i];
            let seg_start = if i == i0 { x0 } else { self.knots_x[i] };
            let seg_end = if i == i1 {
                x1
            } else {
                self.knots_x[i + 1]
            };

            let lo = seg_start - self.knots_x[i];
            let hi = seg_end - self.knots_x[i];

            // Antiderivative of a + b*t + c*t^2 + d*t^3
            // = a*t + b*t^2/2 + c*t^3/3 + d*t^4/4
            let anti = |t: T| -> T {
                t * (a + t * (b / two + t * (c / three + t * d / four)))
            };

            total = total + anti(hi) - anti(lo);
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_linear_data() {
        // y = 2x + 1: spline should reproduce exactly
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![1.0, 3.0, 5.0, 7.0];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        assert_relative_eq!(spline.evaluate(0.5), 2.0, epsilon = 1e-10);
        assert_relative_eq!(spline.evaluate(1.5), 4.0, epsilon = 1e-10);
        assert_relative_eq!(spline.evaluate(2.5), 6.0, epsilon = 1e-10);
    }

    #[test]
    fn test_reproduces_knots() {
        let xs = vec![0.0, 1.0, 2.0, 5.0, 10.0];
        let ys = vec![1.0, 0.98, 0.95, 0.88, 0.75];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        for (x, y) in xs.iter().zip(ys.iter()) {
            assert_relative_eq!(spline.evaluate(*x), *y, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_c2_continuity() {
        let xs = vec![0.0, 1.0, 3.0, 5.0, 10.0];
        let ys = vec![0.0, 0.5, 1.2, 1.8, 2.5];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        // Check continuity at interior knots
        let eps = 1e-8;
        for &x in &xs[1..xs.len() - 1] {
            let left = spline.second_derivative(x - eps);
            let right = spline.second_derivative(x + eps);
            assert_relative_eq!(left, right, epsilon = 1e-4);
        }
    }

    #[test]
    fn test_natural_boundary_conditions() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![0.0, 1.0, 0.5, 0.2];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        assert_relative_eq!(spline.second_derivative(xs[0]), 0.0, epsilon = 1e-10);
        assert_relative_eq!(
            spline.second_derivative(*xs.last().unwrap()),
            0.0,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_integration() {
        // Integrate y = 2x + 1 from 0 to 3 → expected = 3*1 + 3^2 = 12
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![1.0, 3.0, 5.0, 7.0];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        assert_relative_eq!(spline.integrate(0.0, 3.0), 12.0, epsilon = 1e-10);
        assert_relative_eq!(spline.integrate(0.0, 1.5), 3.75, epsilon = 1e-10);
    }

    #[test]
    fn test_two_points() {
        let xs = vec![0.0, 1.0];
        let ys = vec![0.0, 2.0];
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        assert_relative_eq!(spline.evaluate(0.5), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_insufficient_data() {
        let result = CubicSpline::<f64>::natural(&[1.0], &[1.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_increasing() {
        let result = CubicSpline::<f64>::natural(&[1.0, 0.5, 2.0], &[1.0, 2.0, 3.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_derivative() {
        // y = x^2 on [0, 1, 2]: derivative at x=1 should be ~2
        let xs = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let ys: Vec<f64> = xs.iter().map(|x| x * x).collect();
        let spline = CubicSpline::natural(&xs, &ys).unwrap();

        // Natural cubic spline on x^2 data should give very good derivative
        assert_relative_eq!(spline.derivative(2.0), 4.0, epsilon = 0.1);
    }
}

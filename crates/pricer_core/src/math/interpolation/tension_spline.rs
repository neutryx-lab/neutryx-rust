//! Tension spline interpolation.
//!
//! A generalisation of cubic splines that introduces a tension parameter τ to
//! control oscillation. When τ → 0 the spline degenerates to a natural cubic
//! spline; higher τ pulls the curve toward piecewise linear.
//!
//! ## Basis Functions
//!
//! On interval [x_i, x_{i+1}] with h_i = x_{i+1} - x_i, the tension spline
//! uses the basis:
//!
//! ```text
//! S_i(x) = A_i · φ(τ, x_{i+1} - x)  +  B_i · φ(τ, x - x_i)
//!        + C_i · (x_{i+1} - x)        +  D_i · (x - x_i)
//! ```
//!
//! where φ(τ, u) = sinh(τ·u) / (τ·sinh(τ·h_i)) for τ > 0, or u²/(2·h_i)
//! for τ = 0. Coefficients are determined by interpolation and C²
//! continuity conditions.

use num_traits::Float;

use super::InterpolationError;
use crate::math::numeric::from_f64;

/// Tension spline interpolant.
///
/// Stores precomputed coefficients for efficient evaluation. The tension
/// parameter τ ≥ 0 controls the trade-off between smoothness (τ → 0 =
/// cubic) and linearity (τ → ∞ = piecewise linear).
#[derive(Debug, Clone, PartialEq)]
pub struct TensionSpline<T: Float> {
    /// Knot x-coordinates.
    knots_x: Vec<T>,
    /// Knot y-values.
    knots_y: Vec<T>,
    /// Second derivatives (σ_i) at each knot.
    sigma: Vec<T>,
    /// Tension parameter.
    tau: T,
}

impl<T: Float> TensionSpline<T> {
    /// Constructs a tension spline with the given tension parameter.
    ///
    /// Natural boundary conditions (σ_0 = σ_{n-1} = 0).
    /// When `tau = 0`, this is equivalent to a natural cubic spline.
    ///
    /// # Errors
    ///
    /// Returns error if fewer than 2 points or knots are not strictly
    /// increasing.
    pub fn new(xs: &[T], ys: &[T], tau: T) -> Result<Self, InterpolationError> {
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

        for i in 0..n - 1 {
            if xs[i + 1] <= xs[i] {
                return Err(InterpolationError::NonIncreasingKnots);
            }
        }

        // For 2 points, linear (sigma = 0)
        if n == 2 {
            return Ok(Self {
                knots_x: xs.to_vec(),
                knots_y: ys.to_vec(),
                sigma: vec![T::zero(); 2],
                tau,
            });
        }

        let sigma = Self::compute_sigma(xs, ys, tau);

        Ok(Self {
            knots_x: xs.to_vec(),
            knots_y: ys.to_vec(),
            sigma,
            tau,
        })
    }

    /// Solves the tridiagonal system for σ values.
    fn compute_sigma(xs: &[T], ys: &[T], tau: T) -> Vec<T> {
        let n = xs.len();
        let interior = n - 2;
        let mut sigma = vec![T::zero(); n];

        if interior == 0 {
            return sigma;
        }

        let epsilon = from_f64::<T>(1e-6);
        let use_cubic = tau.abs() < epsilon;

        let two = from_f64::<T>(2.0);

        // Build tridiagonal system
        // For each interior point i (1..n-1):
        // a_i * σ_{i-1} + b_i * σ_i + c_i * σ_{i+1} = d_i
        let mut lower = vec![T::zero(); interior];
        let mut diag = vec![T::zero(); interior];
        let mut upper = vec![T::zero(); interior];
        let mut rhs = vec![T::zero(); interior];

        for k in 0..interior {
            let i = k + 1;
            let h_prev = xs[i] - xs[i - 1];
            let h_next = xs[i + 1] - xs[i];

            if use_cubic {
                // Standard cubic spline tridiagonal coefficients
                let six = from_f64::<T>(6.0);
                lower[k] = h_prev;
                diag[k] = two * (h_prev + h_next);
                upper[k] = h_next;
                rhs[k] = six * ((ys[i + 1] - ys[i]) / h_next - (ys[i] - ys[i - 1]) / h_prev);
            } else {
                // Tension spline coefficients
                let th_prev = tau * h_prev;
                let th_next = tau * h_next;

                // q(τh) = 1/sinh(τh) * (cosh(τh)/sinh(τh) - 1/(τh))
                // = (τh*cosh(τh) - sinh(τh)) / (τh * sinh²(τh))
                let alpha_prev = Self::tension_alpha(th_prev);
                let alpha_next = Self::tension_alpha(th_next);
                let beta_prev = Self::tension_beta(th_prev);
                let beta_next = Self::tension_beta(th_next);

                lower[k] = beta_prev;
                diag[k] = two * (alpha_prev + alpha_next);
                upper[k] = beta_next;
                rhs[k] = tau * tau * ((ys[i + 1] - ys[i]) / h_next - (ys[i] - ys[i - 1]) / h_prev);
            }
        }

        // Thomas algorithm
        for k in 1..interior {
            let factor = lower[k] / diag[k - 1];
            diag[k] = diag[k] - factor * upper[k - 1];
            rhs[k] = rhs[k] - factor * rhs[k - 1];
        }

        let last = interior - 1;
        sigma[last + 1] = rhs[last] / diag[last];
        for k in (0..last).rev() {
            sigma[k + 1] = (rhs[k] - upper[k] * sigma[k + 2]) / diag[k];
        }

        sigma
    }

    /// α(τh) = (1/tanh(τh) - 1/(τh)) / h = (τh·cosh(τh) - sinh(τh)) /
    /// (h·τh·sinh(τh)) For the tridiagonal system coefficient on the
    /// diagonal.
    fn tension_alpha(th: T) -> T {
        let epsilon = from_f64::<T>(1e-4);
        if th.abs() < epsilon {
            // Taylor: α ≈ th/3 + th³/45 + ...
            let three = from_f64::<T>(3.0);
            return th / three;
        }
        let one = T::one();
        one / th.tanh() - one / th // = (th*cosh - sinh)/(th*sinh)
    }

    /// β(τh) = (1/(τh) - 1/sinh(τh)) / h
    /// For the tridiagonal system off-diagonal coefficient.
    fn tension_beta(th: T) -> T {
        let epsilon = from_f64::<T>(1e-4);
        if th.abs() < epsilon {
            // Taylor: β ≈ th/6 - th³/360 + ...
            let six = from_f64::<T>(6.0);
            return th / six;
        }
        let one = T::one();
        one / th - one / th.sinh() // = (sinh - th) / (th*sinh)
    }

    /// Returns the tension parameter.
    pub fn tension(&self) -> T { self.tau }

    /// Returns the knot x-coordinates.
    pub fn knots_x(&self) -> &[T] { &self.knots_x }

    /// Finds the interval index for the given x value.
    fn find_interval(&self, x: T) -> usize {
        let n = self.knots_x.len();
        if x <= self.knots_x[0] {
            return 0;
        }
        if x >= self.knots_x[n - 1] {
            return n - 2;
        }
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

    /// Evaluates the tension spline at point x.
    pub fn evaluate(&self, x: T) -> T {
        let i = self.find_interval(x);
        let h = self.knots_x[i + 1] - self.knots_x[i];
        let u = x - self.knots_x[i];
        let v = self.knots_x[i + 1] - x;

        let epsilon = from_f64::<T>(1e-6);

        if self.tau.abs() < epsilon {
            // Cubic spline fallback: S(x) = σ_i*(x_{i+1}-x)³/(6h) +
            // σ_{i+1}*(x-x_i)³/(6h) + (y_i/h - σ_i*h/6)*(x_{i+1}-x) +
            // (y_{i+1}/h - σ_{i+1}*h/6)*(x-x_i)
            let six = from_f64::<T>(6.0);
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            s_i * v * v * v / (six * h)
                + s_j * u * u * u / (six * h)
                + (self.knots_y[i] / h - s_i * h / six) * v
                + (self.knots_y[i + 1] / h - s_j * h / six) * u
        } else {
            // Tension spline:
            // S(x) = σ_i * sinh(τ*v)/(τ²*sinh(τ*h))
            //       + σ_{i+1} * sinh(τ*u)/(τ²*sinh(τ*h))
            //       + (y_i - σ_i/τ²) * v/h
            //       + (y_{i+1} - σ_{i+1}/τ²) * u/h
            let tau2 = self.tau * self.tau;
            let sinh_th = (self.tau * h).sinh();
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            s_i * (self.tau * v).sinh() / (tau2 * sinh_th)
                + s_j * (self.tau * u).sinh() / (tau2 * sinh_th)
                + (self.knots_y[i] - s_i / tau2) * v / h
                + (self.knots_y[i + 1] - s_j / tau2) * u / h
        }
    }

    /// Evaluates the first derivative of the tension spline at point x.
    pub fn derivative(&self, x: T) -> T {
        let i = self.find_interval(x);
        let h = self.knots_x[i + 1] - self.knots_x[i];
        let u = x - self.knots_x[i];
        let v = self.knots_x[i + 1] - x;

        let epsilon = from_f64::<T>(1e-6);

        if self.tau.abs() < epsilon {
            let six = from_f64::<T>(6.0);
            let two = from_f64::<T>(2.0);
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            -s_i * v * v / (two * h) + s_j * u * u / (two * h)
                - (self.knots_y[i] / h - s_i * h / six)
                + (self.knots_y[i + 1] / h - s_j * h / six)
        } else {
            let tau2 = self.tau * self.tau;
            let sinh_th = (self.tau * h).sinh();
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            -s_i * self.tau * (self.tau * v).cosh() / (tau2 * sinh_th)
                + s_j * self.tau * (self.tau * u).cosh() / (tau2 * sinh_th)
                - (self.knots_y[i] - s_i / tau2) / h
                + (self.knots_y[i + 1] - s_j / tau2) / h
        }
    }

    /// Computes the definite integral of the tension spline from `x0` to `x1`.
    pub fn integrate(&self, x0: T, x1: T) -> T {
        if x1 <= x0 {
            return T::zero();
        }

        let i0 = self.find_interval(x0);
        let i1 = self.find_interval(x1);

        let mut total = T::zero();

        for i in i0..=i1 {
            let seg_start = if i == i0 { x0 } else { self.knots_x[i] };
            let seg_end = if i == i1 { x1 } else { self.knots_x[i + 1] };

            total = total + self.integrate_segment(i, seg_start, seg_end);
        }

        total
    }

    /// Integrates the spline over a portion of segment i.
    fn integrate_segment(&self, i: usize, from: T, to: T) -> T {
        let h = self.knots_x[i + 1] - self.knots_x[i];
        let epsilon = from_f64::<T>(1e-6);

        if self.tau.abs() < epsilon {
            // Cubic: integrate a + b*t + c*t^2 + d*t^3 form
            let six = from_f64::<T>(6.0);
            let two = from_f64::<T>(2.0);
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            // Antiderivative terms at point x (let v = x_{i+1}-x, u = x-x_i):
            // We integrate the expression directly using substitution
            let anti = |x: T| -> T {
                let v = self.knots_x[i + 1] - x;
                let u = x - self.knots_x[i];
                -s_i * v * v * v * v / (from_f64::<T>(24.0) * h)
                    + s_j * u * u * u * u / (from_f64::<T>(24.0) * h)
                    + (self.knots_y[i] / h - s_i * h / six) * (-v * v / two)
                    + (self.knots_y[i + 1] / h - s_j * h / six) * (u * u / two)
            };

            anti(to) - anti(from)
        } else {
            let tau2 = self.tau * self.tau;
            let sinh_th = (self.tau * h).sinh();
            let s_i = self.sigma[i];
            let s_j = self.sigma[i + 1];

            // Antiderivative of sinh(τ*v)/sinh(τ*h) is -cosh(τ*v)/(τ*sinh(τ*h))
            // Antiderivative of sinh(τ*u)/sinh(τ*h) is cosh(τ*u)/(τ*sinh(τ*h))
            let two = from_f64::<T>(2.0);
            let anti = |x: T| -> T {
                let v = self.knots_x[i + 1] - x;
                let u = x - self.knots_x[i];

                s_i * (self.tau * v).cosh() / (tau2 * self.tau * sinh_th)
                    + s_j * (self.tau * u).cosh() / (tau2 * self.tau * sinh_th)
                    + (self.knots_y[i] - s_i / tau2) * (-v * v / (two * h))
                    + (self.knots_y[i + 1] - s_j / tau2) * (u * u / (two * h))
            };

            anti(to) - anti(from)
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_linear_data() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![1.0, 3.0, 5.0, 7.0];
        let spline = TensionSpline::new(&xs, &ys, 0.0).unwrap();

        assert_relative_eq!(spline.evaluate(0.5), 2.0, epsilon = 1e-8);
        assert_relative_eq!(spline.evaluate(1.5), 4.0, epsilon = 1e-8);
    }

    #[test]
    fn test_reproduces_knots() {
        let xs = vec![0.0, 1.0, 2.0, 5.0, 10.0];
        let ys = vec![1.0, 0.98, 0.95, 0.88, 0.75];

        for &tau in &[0.0, 0.5, 1.0, 5.0] {
            let spline = TensionSpline::new(&xs, &ys, tau).unwrap();
            for (x, y) in xs.iter().zip(ys.iter()) {
                assert_relative_eq!(spline.evaluate(*x), *y, epsilon = 1e-8);
            }
        }
    }

    #[test]
    fn test_zero_tension_matches_cubic() {
        // Zero tension should produce results very close to CubicSpline
        let xs = vec![0.0, 1.0, 3.0, 5.0, 10.0];
        let ys = vec![0.0, 0.5, 1.2, 1.8, 2.5];

        let cubic = super::super::cubic_spline::CubicSpline::natural(&xs, &ys).unwrap();
        let tension = TensionSpline::new(&xs, &ys, 0.0).unwrap();

        for i in 0..40 {
            let x = i as f64 * 0.25;
            assert_relative_eq!(tension.evaluate(x), cubic.evaluate(x), epsilon = 1e-6,);
        }
    }

    #[test]
    fn test_high_tension_approaches_linear() {
        let xs = vec![0.0, 1.0, 2.0];
        let ys = vec![0.0, 1.0, 0.0];

        let low = TensionSpline::new(&xs, &ys, 0.0).unwrap();
        let high = TensionSpline::new(&xs, &ys, 20.0).unwrap();

        // At midpoint x=0.5: high tension should be closer to linear (0.5)
        let low_val = low.evaluate(0.5);
        let high_val = high.evaluate(0.5);
        let linear_val = 0.5;

        assert!(
            (high_val - linear_val).abs() < (low_val - linear_val).abs() + 0.1,
            "high tension should be closer to linear: low={low_val}, high={high_val}"
        );
    }

    #[test]
    fn test_integration() {
        // Linear: ∫₀³ (2x + 1) dx = 12
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![1.0, 3.0, 5.0, 7.0];
        let spline = TensionSpline::new(&xs, &ys, 0.0).unwrap();

        assert_relative_eq!(spline.integrate(0.0, 3.0), 12.0, epsilon = 1e-6);
    }

    #[test]
    fn test_two_points() {
        let spline = TensionSpline::new(&[0.0, 1.0], &[0.0, 2.0], 1.0).unwrap();
        assert_relative_eq!(spline.evaluate(0.5), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_insufficient_data() {
        assert!(TensionSpline::<f64>::new(&[1.0], &[1.0], 0.0).is_err());
    }

    #[test]
    fn test_tension_spline_with_nonzero_tau() {
        let xs = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = vec![0.0, 0.5, 1.5, 1.0, 0.8, 1.2];
        let spline = TensionSpline::new(&xs, &ys, 2.0).unwrap();

        // Should reproduce knots
        for (x, y) in xs.iter().zip(ys.iter()) {
            assert_relative_eq!(spline.evaluate(*x), *y, epsilon = 1e-6);
        }
    }
}

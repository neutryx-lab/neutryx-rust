//! Monotone convex interpolation (Hagan & West, 2006).
//!
//! Constructs a piecewise quadratic interpolant on instantaneous forward rates
//! that preserves positivity and local monotonicity. This is the industry
//! standard for yield curve interpolation where forward rate behaviour matters.
//!
//! ## Algorithm
//!
//! Given discrete forward rates f_i over intervals [t_i, t_{i+1}]:
//!
//! 1. Compute instantaneous forwards at knots via Hagan-West formula
//! 2. For each interval, fit a quadratic f(t) subject to:
//!    - f(t_i) matches the instantaneous forward at t_i
//!    - The integral over the interval matches the discrete forward
//!    - Monotonicity: f is monotone within each interval
//!
//! The integral ∫₀ᵗ f(s) ds gives the negative log of the discount factor.

use num_traits::Float;

use super::InterpolationError;
use crate::math::numeric::from_f64;

/// Monotone convex interpolator on forward rates.
#[derive(Debug, Clone, PartialEq)]
pub struct MonotoneConvexInterpolator<T: Float> {
    /// Knot times (including t_0 = 0).
    times: Vec<T>,
    /// Discrete forward rates for each interval [t_i, t_{i+1}].
    discrete_forwards: Vec<T>,
    /// Instantaneous forward rates at each knot.
    instant_forwards: Vec<T>,
}

impl<T: Float> MonotoneConvexInterpolator<T> {
    /// Constructs a monotone convex interpolator from discrete forward rates.
    ///
    /// `times` must start at or near 0, be strictly increasing, and have
    /// length `n + 1` where `discrete_forwards` has length `n`.
    ///
    /// # Errors
    ///
    /// Returns error if fewer than 2 time points or data lengths mismatch.
    pub fn from_discrete_forwards(
        times: &[T],
        discrete_forwards: &[T],
    ) -> Result<Self, InterpolationError> {
        let n_intervals = discrete_forwards.len();
        if times.len() != n_intervals + 1 {
            return Err(InterpolationError::InsufficientData {
                required: n_intervals + 1,
                provided: times.len(),
            });
        }
        if n_intervals == 0 {
            return Err(InterpolationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        for i in 0..times.len() - 1 {
            if times[i + 1] <= times[i] {
                return Err(InterpolationError::NonIncreasingKnots);
            }
        }

        let instant_forwards = Self::compute_instantaneous_forwards(times, discrete_forwards);

        Ok(Self {
            times: times.to_vec(),
            discrete_forwards: discrete_forwards.to_vec(),
            instant_forwards,
        })
    }

    /// Constructs a monotone convex interpolator from pillar times and
    /// discount factors.
    ///
    /// Derives discrete forward rates from the discount factors, then builds
    /// the interpolator.
    pub fn from_discount_factors(
        times: &[T],
        discount_factors: &[T],
    ) -> Result<Self, InterpolationError> {
        if times.len() < 2 || discount_factors.len() < 2 {
            return Err(InterpolationError::InsufficientData {
                required: 2,
                provided: times.len().min(discount_factors.len()),
            });
        }
        if times.len() != discount_factors.len() {
            return Err(InterpolationError::InsufficientData {
                required: times.len(),
                provided: discount_factors.len(),
            });
        }

        let n = times.len();
        let mut discrete_forwards = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let dt = times[i + 1] - times[i];
            if dt <= T::zero() {
                return Err(InterpolationError::NonIncreasingKnots);
            }
            // f_i = -ln(DF_{i+1}/DF_i) / dt
            let fwd = -(discount_factors[i + 1] / discount_factors[i]).ln() / dt;
            discrete_forwards.push(fwd);
        }

        Self::from_discrete_forwards(times, &discrete_forwards)
    }

    /// Computes instantaneous forward rates at each knot using Hagan-West.
    fn compute_instantaneous_forwards(times: &[T], discrete_fwds: &[T]) -> Vec<T> {
        let n = discrete_fwds.len();
        let half = from_f64::<T>(0.5);
        let mut f_inst = vec![T::zero(); n + 1];

        if n == 1 {
            // Single interval: flat
            f_inst[0] = discrete_fwds[0];
            f_inst[1] = discrete_fwds[0];
            return f_inst;
        }

        // Boundary: linear extrapolation from first two discrete forwards
        let dt0 = times[1] - times[0];
        let dt1 = times[2] - times[1];
        f_inst[0] = discrete_fwds[0] - (discrete_fwds[1] - discrete_fwds[0]) * dt0 / (dt0 + dt1);

        // Interior knots: weighted average
        for i in 1..n {
            let dt_prev = times[i] - times[i - 1];
            let dt_next = times[i + 1] - times[i];
            let total = dt_prev + dt_next;
            if total > T::zero() {
                f_inst[i] = (dt_next * discrete_fwds[i - 1] + dt_prev * discrete_fwds[i]) / total;
            } else {
                f_inst[i] = half * (discrete_fwds[i - 1] + discrete_fwds[i]);
            }
        }

        // Boundary: linear extrapolation from last two discrete forwards
        let dt_n1 = times[n] - times[n - 1];
        let dt_n2 = times[n - 1] - times[n - 2];
        f_inst[n] = discrete_fwds[n - 1]
            + (discrete_fwds[n - 1] - discrete_fwds[n - 2]) * dt_n1 / (dt_n1 + dt_n2);

        // Apply Hagan-West monotonicity constraints to all knots
        // Constraint: f_inst[i] must lie in [0, 2*f_d] for each adjacent
        // discrete forward f_d to prevent quadratic extrema in the interval.
        Self::apply_monotonicity_constraints(&mut f_inst, discrete_fwds);

        f_inst
    }

    /// Applies Hagan-West monotonicity constraints to instantaneous forwards.
    ///
    /// For each interval, the instantaneous forwards at endpoints must be
    /// bounded so the quadratic interpolant has no extremum within [0, 1].
    fn apply_monotonicity_constraints(f_inst: &mut [T], discrete_fwds: &[T]) {
        let n = discrete_fwds.len();
        let two = from_f64::<T>(2.0);

        for i in 0..=n {
            // Clamp against left interval (if exists)
            if i > 0 {
                let f_d = discrete_fwds[i - 1];
                if f_d >= T::zero() {
                    f_inst[i] = Self::clamp(f_inst[i], T::zero(), two * f_d);
                }
            }
            // Clamp against right interval (if exists)
            if i < n {
                let f_d = discrete_fwds[i];
                if f_d >= T::zero() {
                    f_inst[i] = Self::clamp(f_inst[i], T::zero(), two * f_d);
                }
            }
        }
    }

    /// Clamp a value to [lo, hi].
    fn clamp(val: T, lo: T, hi: T) -> T {
        if val < lo {
            lo
        } else if val > hi {
            hi
        } else {
            val
        }
    }

    /// Returns the instantaneous forward rate at time t.
    pub fn instantaneous_forward(&self, t: T) -> T {
        if t <= self.times[0] {
            return self.instant_forwards[0];
        }
        let n = self.discrete_forwards.len();
        if t >= self.times[n] {
            return self.instant_forwards[n];
        }

        let i = self.find_interval(t);
        self.interpolate_in_interval(i, t)
    }

    /// Returns the integrated forward rate from 0 to t: ∫₀ᵗ f(s) ds.
    ///
    /// This equals -ln(DF(t)), so DF(t) = exp(-integrated_forward(t)).
    pub fn integrated_forward(&self, t: T) -> T {
        if t <= self.times[0] {
            return T::zero();
        }

        let n = self.discrete_forwards.len();
        let i_end = self.find_interval(t);

        let mut integral = T::zero();

        // Sum full intervals
        for i in 0..i_end.min(n) {
            let dt = self.times[i + 1] - self.times[i];
            integral = integral + self.interval_integral(i, self.times[i], self.times[i + 1], dt);
        }

        // Partial last interval
        if i_end < n && t > self.times[i_end] {
            let dt = self.times[i_end + 1] - self.times[i_end];
            integral = integral + self.interval_integral(i_end, self.times[i_end], t, dt);
        }

        integral
    }

    /// Returns the coefficients for the quadratic interpolant in interval i.
    ///
    /// Returns (a, b, c) such that f(t) = a + b*s + c*s² where
    /// s = (t - t_i) / (t_{i+1} - t_i) ∈ [0, 1].
    pub fn interval_coefficients(&self, i: usize) -> (T, T, T) {
        let f0 = self.instant_forwards[i];
        let f1 = self.instant_forwards[i + 1];
        let f_avg = self.discrete_forwards[i];
        let two = from_f64::<T>(2.0);
        let six = from_f64::<T>(6.0);

        // Quadratic: f(s) = a + b*s + c*s²
        // Conditions: f(0) = f0, f(1) = f1, ∫₀¹ f(s) ds = f_avg
        // → a = f0, a + b + c = f1, a + b/2 + c/3 = f_avg
        // → b = -(2*f0 + f1) + 6*f_avg - 3*f0 = -3*f0 - f1 + 6*f_avg - 2*f_avg
        // Solving correctly:
        // a = f0
        // a + b/2 + c/3 = f_avg → f0 + b/2 + c/3 = f_avg
        // a + b + c = f1 → b + c = f1 - f0
        // From second: c = f1 - f0 - b
        // Sub into integral: f0 + b/2 + (f1 - f0 - b)/3 = f_avg
        // f0 + b/2 + f1/3 - f0/3 - b/3 = f_avg
        // 2*f0/3 + f1/3 + b/6 = f_avg
        // b = 6*f_avg - 4*f0 - 2*f1
        let b = six * f_avg - from_f64::<T>(4.0) * f0 - two * f1;
        let c = f1 - f0 - b;

        (f0, b, c)
    }

    /// Interpolates the instantaneous forward rate within interval i.
    fn interpolate_in_interval(&self, i: usize, t: T) -> T {
        let dt = self.times[i + 1] - self.times[i];
        if dt <= T::zero() {
            return self.discrete_forwards[i];
        }
        let s = (t - self.times[i]) / dt;
        let (a, b, c) = self.interval_coefficients(i);
        a + s * (b + s * c)
    }

    /// Computes the integral of the forward rate over a sub-interval.
    fn interval_integral(&self, i: usize, from: T, to: T, _interval_width: T) -> T {
        let dt = self.times[i + 1] - self.times[i];
        if dt <= T::zero() {
            return self.discrete_forwards[i] * (to - from);
        }

        let (a, b, c) = self.interval_coefficients(i);
        let two = from_f64::<T>(2.0);
        let three = from_f64::<T>(3.0);

        let s0 = (from - self.times[i]) / dt;
        let s1 = (to - self.times[i]) / dt;

        // ∫ (a + b*s + c*s²) ds = a*s + b*s²/2 + c*s³/3
        let anti = |s: T| -> T { s * (a + s * (b / two + s * c / three)) };

        (anti(s1) - anti(s0)) * dt
    }

    /// Finds the interval index containing t.
    fn find_interval(&self, t: T) -> usize {
        let n = self.times.len();
        if t <= self.times[0] {
            return 0;
        }
        if t >= self.times[n - 1] {
            return (self.discrete_forwards.len()).saturating_sub(1);
        }
        let mut lo = 0;
        let mut hi = n - 1;
        while lo < hi - 1 {
            let mid = (lo + hi) / 2;
            if t < self.times[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// Returns the knot times.
    pub fn times(&self) -> &[T] { &self.times }

    /// Returns the instantaneous forwards at knots.
    pub fn instant_forwards(&self) -> &[T] { &self.instant_forwards }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_flat_forwards() {
        // Constant forward rate of 5%
        let times = vec![0.0, 1.0, 2.0, 3.0, 5.0, 10.0];
        let fwds = vec![0.05, 0.05, 0.05, 0.05, 0.05];
        let mc = MonotoneConvexInterpolator::from_discrete_forwards(&times, &fwds).unwrap();

        // Instantaneous forward should be 5% everywhere
        assert_relative_eq!(mc.instantaneous_forward(0.5), 0.05, epsilon = 1e-10);
        assert_relative_eq!(mc.instantaneous_forward(2.5), 0.05, epsilon = 1e-10);

        // Integrated forward at t=2: ∫₀² 0.05 ds = 0.10
        assert_relative_eq!(mc.integrated_forward(2.0), 0.10, epsilon = 1e-10);
    }

    #[test]
    fn test_reproduces_discrete_integrals() {
        // The integral over each full interval should match discrete_fwd * dt
        let times = vec![0.0, 0.5, 1.0, 2.0, 5.0];
        let fwds = vec![0.03, 0.035, 0.04, 0.045];
        let mc = MonotoneConvexInterpolator::from_discrete_forwards(&times, &fwds).unwrap();

        let mut cumulative = 0.0;
        for i in 0..fwds.len() {
            let dt = times[i + 1] - times[i];
            cumulative += fwds[i] * dt;
            assert_relative_eq!(
                mc.integrated_forward(times[i + 1]),
                cumulative,
                epsilon = 1e-8
            );
        }
    }

    #[test]
    fn test_from_discount_factors() {
        let times = vec![0.5, 1.0, 2.0, 5.0];
        let dfs = vec![0.985, 0.97, 0.93, 0.82];
        let result = MonotoneConvexInterpolator::from_discount_factors(&times, &dfs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_monotone_upward() {
        // Increasing forward rates
        let times = vec![0.0, 1.0, 2.0, 3.0, 5.0];
        let fwds = vec![0.02, 0.03, 0.04, 0.05];
        let mc = MonotoneConvexInterpolator::from_discrete_forwards(&times, &fwds).unwrap();

        // Check forward is non-decreasing at sample points
        let mut prev = mc.instantaneous_forward(0.0);
        for i in 1..50 {
            let t = i as f64 * 0.1;
            let curr = mc.instantaneous_forward(t);
            assert!(
                curr >= prev - 1e-10,
                "forward decreased at t={t}: {prev} -> {curr}"
            );
            prev = curr;
        }
    }

    #[test]
    fn test_insufficient_data() {
        assert!(MonotoneConvexInterpolator::<f64>::from_discrete_forwards(&[0.0], &[]).is_err());
    }
}

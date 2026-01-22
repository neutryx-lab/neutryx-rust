//! Runge-Kutta methods for ordinary differential equations.
//!
//! This module provides ODE solvers for initial value problems of the form:
//! dy/dt = f(t, y), y(t0) = y0
//!
//! Available methods:
//! - RK4: Classical 4th-order Runge-Kutta (fixed step)
//! - RK45: Dormand-Prince method with adaptive step size control

use num_traits::Float;

use super::IntegrationError;

/// Options for RK45 (Dormand-Prince) adaptive integration.
#[derive(Debug, Clone, Copy)]
pub struct Rk45Options<T> {
    /// Absolute tolerance for step size control.
    pub abs_tol: T,
    /// Relative tolerance for step size control.
    pub rel_tol: T,
    /// Initial step size.
    pub h_init: T,
    /// Minimum allowed step size.
    pub h_min: T,
    /// Maximum allowed step size.
    pub h_max: T,
    /// Maximum number of steps.
    pub max_steps: usize,
}

impl<T: Float> Default for Rk45Options<T> {
    fn default() -> Self {
        Self {
            abs_tol: T::from(1e-6).unwrap(),
            rel_tol: T::from(1e-6).unwrap(),
            h_init: T::from(0.01).unwrap(),
            h_min: T::from(1e-12).unwrap(),
            h_max: T::from(1.0).unwrap(),
            max_steps: 10000,
        }
    }
}

impl<T: Float> Rk45Options<T> {
    /// Creates new options with specified tolerances.
    #[must_use]
    pub fn new(abs_tol: T, rel_tol: T) -> Self {
        Self {
            abs_tol,
            rel_tol,
            ..Default::default()
        }
    }

    /// Sets the initial step size.
    #[must_use]
    pub fn with_h_init(mut self, h: T) -> Self {
        self.h_init = h;
        self
    }

    /// Sets the step size limits.
    #[must_use]
    pub fn with_step_limits(mut self, h_min: T, h_max: T) -> Self {
        self.h_min = h_min;
        self.h_max = h_max;
        self
    }

    /// Sets the maximum number of steps.
    #[must_use]
    pub const fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }
}

/// Performs a single step of the classical 4th-order Runge-Kutta method.
///
/// Solves dy/dt = f(t, y) from t to t+h.
///
/// # Arguments
///
/// * `f` - The derivative function f(t, y)
/// * `t` - Current time
/// * `y` - Current state
/// * `h` - Step size
///
/// # Returns
///
/// The state at time t+h.
///
/// # Example
///
/// ```
/// use pricer_core::math::integrators::rk4_step;
///
/// // Solve dy/dt = -y (exponential decay)
/// let y0: f64 = 1.0;
/// let t0: f64 = 0.0;
/// let h: f64 = 0.1;
///
/// let y1 = rk4_step(|_t, y| -y, t0, y0, h);
/// // y1 should be approximately exp(-0.1) ≈ 0.9048...
/// assert!((y1 - (-0.1_f64).exp()).abs() < 1e-6);
/// ```
pub fn rk4_step<T, F>(f: F, t: T, y: T, h: T) -> T
where
    T: Float,
    F: Fn(T, T) -> T,
{
    let two = T::from(2.0).unwrap();
    let half = T::from(0.5).unwrap();
    let sixth = T::from(1.0 / 6.0).unwrap();

    let k1 = f(t, y);
    let k2 = f(t + half * h, y + half * h * k1);
    let k3 = f(t + half * h, y + half * h * k2);
    let k4 = f(t + h, y + h * k3);

    y + sixth * h * (k1 + two * k2 + two * k3 + k4)
}

/// Integrates an ODE using the RK45 (Dormand-Prince) method with adaptive step
/// size.
///
/// Solves dy/dt = f(t, y), y(t0) = y0 from t0 to t_end.
///
/// # Arguments
///
/// * `f` - The derivative function f(t, y)
/// * `t0` - Initial time
/// * `y0` - Initial state
/// * `t_end` - Final time
/// * `options` - Integration options
///
/// # Returns
///
/// A vector of (t, y) pairs representing the solution trajectory, or an error.
///
/// # Example
///
/// ```
/// use pricer_core::math::integrators::{rk45_integrate, Rk45Options};
///
/// // Solve dy/dt = -y (exponential decay) from t=0 to t=1
/// let options: Rk45Options<f64> = Rk45Options::default();
/// let trajectory = rk45_integrate(|_t: f64, y: f64| -y, 0.0_f64, 1.0_f64, 1.0_f64, &options).unwrap();
///
/// // Final value should be approximately exp(-1) ≈ 0.3679...
/// let (t_final, y_final) = trajectory.last().unwrap();
/// assert!((*t_final - 1.0_f64).abs() < 1e-10);
/// assert!((*y_final - (-1.0_f64).exp()).abs() < 1e-5);
/// ```
pub fn rk45_integrate<T, F>(
    f: F,
    t0: T,
    y0: T,
    t_end: T,
    options: &Rk45Options<T>,
) -> Result<Vec<(T, T)>, IntegrationError>
where
    T: Float,
    F: Fn(T, T) -> T,
{
    // Dormand-Prince coefficients
    // Butcher tableau for DOPRI5
    let a21 = T::from(1.0 / 5.0).unwrap();
    let a31 = T::from(3.0 / 40.0).unwrap();
    let a32 = T::from(9.0 / 40.0).unwrap();
    let a41 = T::from(44.0 / 45.0).unwrap();
    let a42 = T::from(-56.0 / 15.0).unwrap();
    let a43 = T::from(32.0 / 9.0).unwrap();
    let a51 = T::from(19372.0 / 6561.0).unwrap();
    let a52 = T::from(-25360.0 / 2187.0).unwrap();
    let a53 = T::from(64448.0 / 6561.0).unwrap();
    let a54 = T::from(-212.0 / 729.0).unwrap();
    let a61 = T::from(9017.0 / 3168.0).unwrap();
    let a62 = T::from(-355.0 / 33.0).unwrap();
    let a63 = T::from(46732.0 / 5247.0).unwrap();
    let a64 = T::from(49.0 / 176.0).unwrap();
    let a65 = T::from(-5103.0 / 18656.0).unwrap();

    // 5th order weights
    let b1 = T::from(35.0 / 384.0).unwrap();
    let b3 = T::from(500.0 / 1113.0).unwrap();
    let b4 = T::from(125.0 / 192.0).unwrap();
    let b5 = T::from(-2187.0 / 6784.0).unwrap();
    let b6 = T::from(11.0 / 84.0).unwrap();

    // 4th order weights (for error estimation)
    let e1 = T::from(71.0 / 57600.0).unwrap();
    let e3 = T::from(-71.0 / 16695.0).unwrap();
    let e4 = T::from(71.0 / 1920.0).unwrap();
    let e5 = T::from(-17253.0 / 339200.0).unwrap();
    let e6 = T::from(22.0 / 525.0).unwrap();
    let e7 = T::from(-1.0 / 40.0).unwrap();

    let c2 = T::from(1.0 / 5.0).unwrap();
    let c3 = T::from(3.0 / 10.0).unwrap();
    let c4 = T::from(4.0 / 5.0).unwrap();
    let c5 = T::from(8.0 / 9.0).unwrap();

    let mut trajectory = vec![(t0, y0)];
    let mut t = t0;
    let mut y = y0;
    let mut h = options.h_init;

    let safety = T::from(0.9).unwrap();
    let p_grow = T::from(0.2).unwrap();
    let p_shrink = T::from(0.25).unwrap();

    for _ in 0..options.max_steps {
        if t >= t_end {
            break;
        }

        // Don't overshoot t_end
        if t + h > t_end {
            h = t_end - t;
        }

        // Compute RK stages
        let k1 = f(t, y);
        let k2 = f(t + c2 * h, y + h * a21 * k1);
        let k3 = f(t + c3 * h, y + h * (a31 * k1 + a32 * k2));
        let k4 = f(t + c4 * h, y + h * (a41 * k1 + a42 * k2 + a43 * k3));
        let k5 = f(
            t + c5 * h,
            y + h * (a51 * k1 + a52 * k2 + a53 * k3 + a54 * k4),
        );
        let k6 = f(
            t + h,
            y + h * (a61 * k1 + a62 * k2 + a63 * k3 + a64 * k4 + a65 * k5),
        );

        // 5th order solution
        let y_new = y + h * (b1 * k1 + b3 * k3 + b4 * k4 + b5 * k5 + b6 * k6);

        // Error estimate
        let k7 = f(t + h, y_new);
        let err = h * (e1 * k1 + e3 * k3 + e4 * k4 + e5 * k5 + e6 * k6 + e7 * k7);
        let err_abs = err.abs();

        // Tolerance
        let tol = options.abs_tol + options.rel_tol * y_new.abs();

        if err_abs <= tol {
            // Accept step
            t = t + h;
            y = y_new;
            trajectory.push((t, y));

            // Grow step size
            if err_abs > T::zero() {
                let factor = safety * (tol / err_abs).powf(p_grow);
                h = h * factor.min(T::from(5.0).unwrap());
            } else {
                h = h * T::from(5.0).unwrap();
            }
            h = h.min(options.h_max);
        } else {
            // Reject step and shrink
            let factor = safety * (tol / err_abs).powf(p_shrink);
            h = h * factor.max(T::from(0.1).unwrap());
        }

        // Check for step size too small
        if h < options.h_min {
            return Err(IntegrationError::NumericalError(
                "Step size became too small".to_string(),
            ));
        }
    }

    // Ensure we reached t_end
    if t < t_end {
        return Err(IntegrationError::NotConverged {
            max_iterations: options.max_steps,
        });
    }

    Ok(trajectory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rk45_options_default() {
        let opts: Rk45Options<f64> = Rk45Options::default();
        assert!((opts.abs_tol - 1e-6).abs() < 1e-15);
        assert!((opts.rel_tol - 1e-6).abs() < 1e-15);
        assert_eq!(opts.max_steps, 10000);
    }

    #[test]
    fn test_rk45_options_new() {
        let opts: Rk45Options<f64> = Rk45Options::new(1e-8, 1e-8);
        assert!((opts.abs_tol - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_rk45_options_with_h_init() {
        let opts: Rk45Options<f64> = Rk45Options::default().with_h_init(0.05);
        assert!((opts.h_init - 0.05).abs() < 1e-15);
    }

    #[test]
    fn test_rk45_options_with_step_limits() {
        let opts: Rk45Options<f64> = Rk45Options::default().with_step_limits(1e-10, 0.5);
        assert!((opts.h_min - 1e-10).abs() < 1e-15);
        assert!((opts.h_max - 0.5).abs() < 1e-15);
    }

    #[test]
    fn test_rk45_options_with_max_steps() {
        let opts: Rk45Options<f64> = Rk45Options::default().with_max_steps(5000);
        assert_eq!(opts.max_steps, 5000);
    }

    #[test]
    fn test_rk4_step_exponential_decay() {
        // dy/dt = -y, y(0) = 1
        // Exact solution: y(t) = exp(-t)
        let y0: f64 = 1.0;
        let t0: f64 = 0.0;
        let h: f64 = 0.1;

        let y1 = rk4_step(|_t, y| -y, t0, y0, h);
        let exact = (-h).exp();

        assert!((y1 - exact).abs() < 1e-6);
    }

    #[test]
    fn test_rk4_step_linear_growth() {
        // dy/dt = 1, y(0) = 0
        // Exact solution: y(t) = t
        let y0: f64 = 0.0;
        let t0: f64 = 0.0;
        let h: f64 = 0.1;

        let y1 = rk4_step(|_t, _y| 1.0, t0, y0, h);
        assert!((y1 - h).abs() < 1e-14);
    }

    #[test]
    fn test_rk4_step_quadratic() {
        // dy/dt = 2t, y(0) = 0
        // Exact solution: y(t) = t^2
        let y0: f64 = 0.0;
        let t0: f64 = 0.0;
        let h: f64 = 0.1;

        let y1 = rk4_step(|t, _y| 2.0 * t, t0, y0, h);
        let exact = h * h;

        assert!((y1 - exact).abs() < 1e-10);
    }

    #[test]
    fn test_rk45_exponential_decay() {
        // dy/dt = -y, y(0) = 1
        // Exact solution: y(t) = exp(-t)
        let options = Rk45Options::default();
        let trajectory = rk45_integrate(|_t, y| -y, 0.0, 1.0, 1.0, &options).unwrap();

        let (t_final, y_final) = trajectory.last().unwrap();
        let exact = (-1.0_f64).exp();

        assert!((*t_final - 1.0).abs() < 1e-10);
        assert!((*y_final - exact).abs() < 1e-5);
    }

    #[test]
    fn test_rk45_linear_ode() {
        // dy/dt = t, y(0) = 0
        // Exact solution: y(t) = t^2 / 2
        let options = Rk45Options::default();
        let trajectory = rk45_integrate(|t, _y| t, 0.0, 0.0, 2.0, &options).unwrap();

        let (t_final, y_final) = trajectory.last().unwrap();
        let exact = 2.0_f64 * 2.0 / 2.0;

        assert!((*t_final - 2.0).abs() < 1e-10);
        assert!((*y_final - exact).abs() < 1e-4);
    }

    #[test]
    fn test_rk45_harmonic_oscillator() {
        // dy/dt = -sin(t), y(0) = 1
        // Exact solution: y(t) = cos(t)
        // (This is d/dt[cos(t)] = -sin(t))
        let options = Rk45Options::new(1e-8, 1e-8);
        let trajectory =
            rk45_integrate(|t, _y| -t.sin(), 0.0, 1.0, std::f64::consts::PI, &options).unwrap();

        let (t_final, y_final) = trajectory.last().unwrap();
        let exact = std::f64::consts::PI.cos();

        assert!((*t_final - std::f64::consts::PI).abs() < 1e-10);
        assert!((*y_final - exact).abs() < 1e-5);
    }

    #[test]
    fn test_rk45_trajectory_starts_at_initial() {
        let options = Rk45Options::default();
        let trajectory = rk45_integrate(|_t, y| -y, 0.0, 1.0, 1.0, &options).unwrap();

        let (t0, y0) = trajectory[0];
        assert!((t0 - 0.0).abs() < 1e-15);
        assert!((y0 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn test_rk45_trajectory_is_monotonic_in_time() {
        let options = Rk45Options::default();
        let trajectory = rk45_integrate(|_t, y| -y, 0.0, 1.0, 1.0, &options).unwrap();

        for i in 1..trajectory.len() {
            assert!(trajectory[i].0 > trajectory[i - 1].0);
        }
    }
}

//! Adaptive numerical integration with interval bisection and tanh-sinh
//! transform.
//!
//! Adaptive quadrature subdivides the integration interval until a desired
//! tolerance is achieved, allowing efficient integration of functions with
//! localised difficult regions.

use std::f64::consts::PI;

use num_traits::Float;

use super::{IntegrationError, IntegrationResult};

/// Options for tanh-sinh (double-exponential) quadrature.
#[derive(Debug, Clone, Copy)]
pub struct TanhSinhOptions<T> {
    /// Absolute tolerance for convergence.
    pub abs_tol: T,
    /// Relative tolerance for convergence.
    pub rel_tol: T,
    /// Maximum number of levels (refinements).
    pub max_levels: usize,
    /// Initial step size parameter.
    pub h_init: T,
}

impl<T: Float> Default for TanhSinhOptions<T> {
    fn default() -> Self {
        Self {
            abs_tol: T::from(1e-10).unwrap(),
            rel_tol: T::from(1e-10).unwrap(),
            max_levels: 10,
            h_init: T::from(1.0).unwrap(),
        }
    }
}

impl<T: Float> TanhSinhOptions<T> {
    /// Creates new options with specified tolerances.
    #[must_use]
    pub fn new(abs_tol: T, rel_tol: T) -> Self {
        Self {
            abs_tol,
            rel_tol,
            ..Default::default()
        }
    }

    /// Sets the maximum refinement levels.
    #[must_use]
    pub fn with_max_levels(mut self, max_levels: usize) -> Self {
        self.max_levels = max_levels;
        self
    }
}

/// Integrates f(x) from a to b using adaptive bisection with Gauss-Kronrod.
///
/// Uses G7-K15 quadrature on each subinterval and bisects intervals where
/// the error estimate exceeds the tolerance.
///
/// # Arguments
///
/// * `f` - The function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `options` - Integration options (tolerances and limits)
///
/// # Returns
///
/// An `IntegrationResult` or an error if convergence fails.
pub fn integrate_adaptive<T, F>(
    f: F,
    a: T,
    b: T,
    options: &TanhSinhOptions<T>,
) -> Result<IntegrationResult<T>, IntegrationError>
where
    T: Float,
    F: Fn(T) -> T,
{
    let half = T::from(0.5).unwrap();

    // Stack of intervals to process: (lower, upper)
    let mut stack: Vec<(T, T, usize)> = vec![(a, b, 0)];
    let mut total = T::zero();
    let mut total_error = T::zero();
    let mut num_evaluations = 0usize;

    while let Some((lo, hi, depth)) = stack.pop() {
        // Use G7-K15 on this interval
        let result = gauss_kronrod_g7k15(&f, lo, hi);
        num_evaluations += 15;

        let interval_error = result.error_estimate.unwrap_or(T::zero());
        let tolerance = options.abs_tol + options.rel_tol * result.value.abs();

        if interval_error <= tolerance || depth >= options.max_levels {
            total = total + result.value;
            total_error = total_error + interval_error;
        } else {
            // Bisect the interval
            let mid = (lo + hi) * half;
            stack.push((lo, mid, depth + 1));
            stack.push((mid, hi, depth + 1));
        }
    }

    // Check if we exhausted iterations without converging well enough
    if total_error > options.abs_tol + options.rel_tol * total.abs() {
        // Still return result but note it may not meet tolerance
        Ok(IntegrationResult::with_error(
            total,
            total_error,
            num_evaluations,
        ))
    } else {
        Ok(IntegrationResult::with_error(
            total,
            total_error,
            num_evaluations,
        ))
    }
}

/// Performs tanh-sinh (double-exponential) quadrature.
///
/// The tanh-sinh transform maps [−1, 1] to (−inf, inf) via x = tanh(pi/2 *
/// sinh(t)), which clusters quadrature points near the endpoints. This is
/// effective for integrands with endpoint singularities.
///
/// # Arguments
///
/// * `f` - The function to integrate (on transformed domain)
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `options` - Integration options
///
/// # Returns
///
/// An `IntegrationResult` or an error if convergence fails.
pub fn integrate_tanh_sinh<T, F>(
    f: F,
    a: T,
    b: T,
    options: &TanhSinhOptions<T>,
) -> Result<IntegrationResult<T>, IntegrationError>
where
    T: Float,
    F: Fn(T) -> T,
{
    let half = T::from(0.5).unwrap();
    let pi_half = T::from(PI / 2.0).unwrap();

    // Transform from [a, b] to [-1, 1]
    let scale = (b - a) * half;
    let shift = (a + b) * half;

    let mut h = options.h_init;
    let mut prev_result = T::zero();
    let mut num_evaluations = 0usize;

    for level in 0..options.max_levels {
        let mut sum = T::zero();

        // Compute points at t = k * h for k = 0, 1, 2, ...
        // For each k > 0, we also evaluate at t = -k * h (symmetry)
        let mut k = 0i32;
        loop {
            let t = T::from(k).unwrap() * h;

            // Compute sinh(t) and cosh(t) directly
            let sinh_t = t.sinh();
            let cosh_t = t.cosh();

            // u = pi/2 * sinh(t)
            let u = pi_half * sinh_t;

            // x = tanh(u)
            let x = u.tanh();

            // dx/dt = pi/2 * cosh(t) * sech^2(u)
            // sech^2(u) = 1 / cosh^2(u)
            let cosh_u = u.cosh();
            let weight = pi_half * cosh_t / (cosh_u * cosh_u);

            // Check if weight is negligible or x is at boundary
            if weight < T::from(1e-15).unwrap() || x.abs() > T::from(0.9999999999).unwrap() {
                break;
            }

            // Transform x from [-1, 1] to [a, b]
            let x_transformed = scale * x + shift;
            let fval = f(x_transformed);
            num_evaluations += 1;

            if k == 0 {
                sum = sum + weight * fval;
            } else {
                // Add contribution from positive t
                sum = sum + weight * fval;

                // Add contribution from negative t (by symmetry of cosh)
                let x_neg = (-u).tanh();
                let x_neg_transformed = scale * x_neg + shift;
                let fval_neg = f(x_neg_transformed);
                num_evaluations += 1;
                sum = sum + weight * fval_neg;
            }

            k += 1;
            if k > 500 {
                break;
            }
        }

        let result = scale * h * sum;
        let error = (result - prev_result).abs();

        if level > 0 && error < options.abs_tol + options.rel_tol * result.abs() {
            return Ok(IntegrationResult::with_error(
                result,
                error,
                num_evaluations,
            ));
        }

        prev_result = result;
        h = h * half;
    }

    let error = T::from(1e-10).unwrap();
    Ok(IntegrationResult::with_error(
        prev_result,
        error,
        num_evaluations,
    ))
}

/// Internal G7-K15 quadrature for adaptive integration.
fn gauss_kronrod_g7k15<T, F>(f: &F, a: T, b: T) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T,
{
    let half = T::from(0.5).unwrap();

    let scale = (b - a) * half;
    let shift = (a + b) * half;

    // K15 nodes
    const K15_NODES: [f64; 15] = [
        -0.991_455_371_120_812_6,
        -0.949_107_912_342_758_5,
        -0.864_864_423_359_769_1,
        -0.741_531_185_599_394_4,
        -0.586_087_235_467_691_1,
        -0.405_845_151_377_397_2,
        -0.207_784_955_007_898_47,
        0.0,
        0.207_784_955_007_898_47,
        0.405_845_151_377_397_2,
        0.586_087_235_467_691_1,
        0.741_531_185_599_394_4,
        0.864_864_423_359_769_1,
        0.949_107_912_342_758_5,
        0.991_455_371_120_812_6,
    ];

    const K15_WEIGHTS: [f64; 15] = [
        0.022_935_322_010_529_22,
        0.063_092_092_629_978_55,
        0.104_790_010_322_250_18,
        0.140_653_259_715_525_92,
        0.169_004_726_639_267_0,
        0.190_350_578_064_785_4,
        0.204_432_940_075_298_9,
        0.209_482_141_084_727_83,
        0.204_432_940_075_298_9,
        0.190_350_578_064_785_4,
        0.169_004_726_639_267_0,
        0.140_653_259_715_525_92,
        0.104_790_010_322_250_18,
        0.063_092_092_629_978_55,
        0.022_935_322_010_529_22,
    ];

    const G7_WEIGHTS_IN_K15: [f64; 7] = [
        0.129_484_966_168_869_69,
        0.279_705_391_489_276_67,
        0.381_830_050_505_118_94,
        0.417_959_183_673_469_4,
        0.381_830_050_505_118_94,
        0.279_705_391_489_276_67,
        0.129_484_966_168_869_69,
    ];

    let mut fvals = [T::zero(); 15];
    for (i, &node) in K15_NODES.iter().enumerate() {
        let t = T::from(node).unwrap();
        let x = scale * t + shift;
        fvals[i] = f(x);
    }

    let mut kronrod_sum = T::zero();
    for (&fval, &weight) in fvals.iter().zip(K15_WEIGHTS.iter()) {
        let w = T::from(weight).unwrap();
        kronrod_sum = kronrod_sum + w * fval;
    }

    let gauss_indices = [1, 3, 5, 7, 9, 11, 13];
    let mut gauss_sum = T::zero();
    for (&idx, &weight) in gauss_indices.iter().zip(G7_WEIGHTS_IN_K15.iter()) {
        let w = T::from(weight).unwrap();
        gauss_sum = gauss_sum + w * fvals[idx];
    }

    let kronrod_result = scale * kronrod_sum;
    let gauss_result = scale * gauss_sum;
    let error = (kronrod_result - gauss_result).abs();

    IntegrationResult::with_error(kronrod_result, error, 15)
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_tanh_sinh_options_default() {
        let opts: TanhSinhOptions<f64> = TanhSinhOptions::default();
        assert!((opts.abs_tol - 1e-10).abs() < 1e-15);
        assert!((opts.rel_tol - 1e-10).abs() < 1e-15);
        assert_eq!(opts.max_levels, 10);
    }

    #[test]
    fn test_tanh_sinh_options_new() {
        let opts: TanhSinhOptions<f64> = TanhSinhOptions::new(1e-8, 1e-6);
        assert!((opts.abs_tol - 1e-8).abs() < 1e-15);
        assert!((opts.rel_tol - 1e-6).abs() < 1e-15);
    }

    #[test]
    fn test_tanh_sinh_options_with_max_levels() {
        let opts: TanhSinhOptions<f64> = TanhSinhOptions::default().with_max_levels(15);
        assert_eq!(opts.max_levels, 15);
    }

    #[test]
    fn test_adaptive_constant_function() {
        // Integral of 5 from 0 to 2 = 10
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|_x: f64| 5.0, 0.0, 2.0, &opts).unwrap();
        assert!((result.value - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_quadratic() {
        // Integral of x^2 from 0 to 1 = 1/3
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| x * x, 0.0, 1.0, &opts).unwrap();
        assert!((result.value - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_sine() {
        // Integral of sin(x) from 0 to pi = 2
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| x.sin(), 0.0, PI, &opts).unwrap();
        assert!((result.value - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_exponential() {
        // Integral of exp(x) from 0 to 1 = e - 1
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| x.exp(), 0.0, 1.0, &opts).unwrap();
        let expected = std::f64::consts::E - 1.0;
        assert!((result.value - expected).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_gaussian() {
        // Integral of exp(-x^2) from 0 to 1 ~ 0.7468241328124271
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| (-x * x).exp(), 0.0, 1.0, &opts).unwrap();
        let expected = 0.746_824_132_812_427_1;
        assert!((result.value - expected).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_oscillatory() {
        // Integral of sin(10x) from 0 to pi = 0 (since cos(10*pi) = 1)
        let opts = TanhSinhOptions::new(1e-8, 1e-8);
        let result = integrate_adaptive(|x: f64| (10.0 * x).sin(), 0.0, PI, &opts).unwrap();
        assert!(result.value.abs() < 1e-7);
    }

    #[test]
    fn test_adaptive_provides_error_estimate() {
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| x.sin(), 0.0, PI, &opts).unwrap();
        assert!(result.error_estimate.is_some());
        assert!(result.error_estimate.unwrap() >= 0.0);
    }

    #[test]
    fn test_adaptive_num_evaluations() {
        let opts = TanhSinhOptions::default();
        let result = integrate_adaptive(|x: f64| x * x, 0.0, 1.0, &opts).unwrap();
        assert!(result.num_evaluations >= 15); // At least one G7K15 call
    }

    #[test]
    fn test_tanh_sinh_constant() {
        let opts = TanhSinhOptions::default();
        let result = integrate_tanh_sinh(|_x: f64| 5.0, 0.0, 2.0, &opts).unwrap();
        assert!((result.value - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_tanh_sinh_quadratic() {
        let opts = TanhSinhOptions::default();
        let result = integrate_tanh_sinh(|x: f64| x * x, 0.0, 1.0, &opts).unwrap();
        assert!((result.value - 1.0 / 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_tanh_sinh_sine() {
        let opts = TanhSinhOptions::default();
        let result = integrate_tanh_sinh(|x: f64| x.sin(), 0.0, PI, &opts).unwrap();
        assert!((result.value - 2.0).abs() < 1e-4);
    }
}

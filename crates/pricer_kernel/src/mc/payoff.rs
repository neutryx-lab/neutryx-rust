//! Smooth payoff functions for Monte Carlo pricing.
//!
//! This module provides differentiable payoff functions using smooth
//! approximations to discontinuous max/min operations. Smoothing is
//! essential for Enzyme automatic differentiation.
//!
//! # Smooth Approximations
//!
//! The soft-plus function approximates `max(x, 0)`:
//! ```text
//! softplus(x, ε) = ε × ln(1 + exp(x/ε))
//! ```
//!
//! As ε → 0, softplus converges to max. The gradient is the sigmoid function.
//!
//! # Enzyme Compatibility
//!
//! All functions use smooth operations only (no `if`, `max`, `min` on floats).
//! This ensures Enzyme can compute gradients correctly.

use super::workspace::PathWorkspace;

/// Payoff type for option pricing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PayoffType {
    /// Call option: max(S - K, 0)
    #[default]
    Call,
    /// Put option: max(K - S, 0)
    Put,
}

/// Parameters for payoff computation.
///
/// # Examples
///
/// ```rust
/// use pricer_kernel::mc::{PayoffParams, PayoffType};
///
/// let params = PayoffParams {
///     strike: 100.0,
///     payoff_type: PayoffType::Call,
///     smoothing_epsilon: 1e-4,
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoffParams {
    /// Strike price.
    pub strike: f64,
    /// Payoff type (Call or Put).
    pub payoff_type: PayoffType,
    /// Smoothing epsilon for soft approximation.
    ///
    /// Smaller values give sharper payoff but may cause numerical issues.
    /// Recommended: 1e-4 for typical option pricing.
    pub smoothing_epsilon: f64,
}

impl Default for PayoffParams {
    fn default() -> Self {
        Self {
            strike: 100.0,
            payoff_type: PayoffType::Call,
            smoothing_epsilon: 1e-4,
        }
    }
}

impl PayoffParams {
    /// Creates call option payoff parameters.
    #[inline]
    pub fn call(strike: f64) -> Self {
        Self {
            strike,
            payoff_type: PayoffType::Call,
            smoothing_epsilon: 1e-4,
        }
    }

    /// Creates put option payoff parameters.
    #[inline]
    pub fn put(strike: f64) -> Self {
        Self {
            strike,
            payoff_type: PayoffType::Put,
            smoothing_epsilon: 1e-4,
        }
    }

    /// Sets the smoothing epsilon.
    #[inline]
    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.smoothing_epsilon = epsilon;
        self
    }
}

/// Soft-plus function: smooth approximation of max(x, 0).
///
/// ```text
/// softplus(x, ε) = ε × ln(1 + exp(x/ε))
/// ```
///
/// # Numerical Stability
///
/// For large positive `x/ε`, uses the approximation `x` to avoid overflow.
/// For large negative `x/ε`, uses the approximation `ε × exp(x/ε)` for accuracy.
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter (smaller = sharper)
///
/// # Returns
///
/// Smooth approximation of max(x, 0).
#[inline]
pub fn soft_plus(x: f64, epsilon: f64) -> f64 {
    let scaled = x / epsilon;
    if scaled > 20.0 {
        // Avoid overflow: ln(1 + exp(20)) ≈ 20
        x
    } else if scaled < -20.0 {
        // Avoid underflow: use exp directly
        epsilon * scaled.exp()
    } else {
        epsilon * (1.0 + scaled.exp()).ln()
    }
}

/// Derivative of soft-plus: the sigmoid function.
///
/// ```text
/// d/dx softplus(x, ε) = sigmoid(x/ε) = 1 / (1 + exp(-x/ε))
/// ```
///
/// # Arguments
///
/// * `x` - Input value
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Value in (0, 1), approximating the Heaviside step function.
#[inline]
pub fn soft_plus_derivative(x: f64, epsilon: f64) -> f64 {
    let scaled = x / epsilon;
    if scaled > 20.0 {
        1.0
    } else if scaled < -20.0 {
        0.0
    } else {
        1.0 / (1.0 + (-scaled).exp())
    }
}

/// Computes smooth European call payoff.
///
/// ```text
/// payoff = softplus(S - K, ε)
/// ```
///
/// As ε → 0, converges to max(S - K, 0).
///
/// # Arguments
///
/// * `terminal_price` - Asset price at expiry
/// * `strike` - Strike price
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Smooth call payoff (always >= 0).
#[inline]
pub fn european_call_smooth(terminal_price: f64, strike: f64, epsilon: f64) -> f64 {
    soft_plus(terminal_price - strike, epsilon)
}

/// Computes smooth European put payoff.
///
/// ```text
/// payoff = softplus(K - S, ε)
/// ```
///
/// As ε → 0, converges to max(K - S, 0).
///
/// # Arguments
///
/// * `terminal_price` - Asset price at expiry
/// * `strike` - Strike price
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Smooth put payoff (always >= 0).
#[inline]
pub fn european_put_smooth(terminal_price: f64, strike: f64, epsilon: f64) -> f64 {
    soft_plus(strike - terminal_price, epsilon)
}

/// Computes payoff for a single path.
///
/// # Arguments
///
/// * `terminal_price` - Asset price at expiry
/// * `params` - Payoff parameters
///
/// # Returns
///
/// Smooth payoff value.
#[inline]
pub fn compute_payoff(terminal_price: f64, params: PayoffParams) -> f64 {
    match params.payoff_type {
        PayoffType::Call => {
            european_call_smooth(terminal_price, params.strike, params.smoothing_epsilon)
        }
        PayoffType::Put => {
            european_put_smooth(terminal_price, params.strike, params.smoothing_epsilon)
        }
    }
}

/// Computes payoffs for all paths in workspace.
///
/// Reads terminal prices from paths and writes payoff values.
///
/// # Arguments
///
/// * `workspace` - Workspace with generated paths
/// * `params` - Payoff parameters
/// * `n_paths` - Number of paths
/// * `n_steps` - Number of steps (to locate terminal prices)
///
/// # Side Effects
///
/// Writes payoff values to `workspace.payoffs_mut()`.
pub fn compute_payoffs(
    workspace: &mut PathWorkspace,
    params: PayoffParams,
    n_paths: usize,
    n_steps: usize,
) {
    let (paths, payoffs) = workspace.paths_and_payoffs_mut();
    let n_steps_plus_1 = n_steps + 1;

    for path_idx in 0..n_paths {
        let terminal_price = paths[path_idx * n_steps_plus_1 + n_steps];
        payoffs[path_idx] = compute_payoff(terminal_price, params);
    }
}

/// Computes smooth Asian arithmetic average call payoff.
///
/// The arithmetic average is computed from all path points,
/// then the payoff is `softplus(avg - K, ε)`.
///
/// # Arguments
///
/// * `path` - Slice of price path (all time points)
/// * `strike` - Strike price
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Smooth Asian call payoff.
#[inline]
pub fn asian_arithmetic_call_smooth(path: &[f64], strike: f64, epsilon: f64) -> f64 {
    if path.is_empty() {
        return 0.0;
    }
    let avg = path.iter().sum::<f64>() / path.len() as f64;
    soft_plus(avg - strike, epsilon)
}

/// Computes smooth Asian arithmetic average put payoff.
///
/// # Arguments
///
/// * `path` - Slice of price path (all time points)
/// * `strike` - Strike price
/// * `epsilon` - Smoothing parameter
///
/// # Returns
///
/// Smooth Asian put payoff.
#[inline]
pub fn asian_arithmetic_put_smooth(path: &[f64], strike: f64, epsilon: f64) -> f64 {
    if path.is_empty() {
        return 0.0;
    }
    let avg = path.iter().sum::<f64>() / path.len() as f64;
    soft_plus(strike - avg, epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_soft_plus_positive() {
        // For large positive x, softplus(x) ≈ x
        let result = soft_plus(10.0, 0.01);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_soft_plus_negative() {
        // For large negative x, softplus(x) ≈ 0
        let result = soft_plus(-10.0, 0.01);
        assert!(result < 0.01);
        assert!(result > 0.0);
    }

    #[test]
    fn test_soft_plus_at_zero() {
        // softplus(0) = ε × ln(2) ≈ 0.693 × ε
        let epsilon = 1.0;
        let result = soft_plus(0.0, epsilon);
        assert_relative_eq!(result, 2.0_f64.ln(), epsilon = 1e-10);
    }

    #[test]
    fn test_soft_plus_derivative_positive() {
        // For large positive x, derivative ≈ 1
        let result = soft_plus_derivative(10.0, 0.01);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_soft_plus_derivative_negative() {
        // For large negative x, derivative ≈ 0
        let result = soft_plus_derivative(-10.0, 0.01);
        assert!(result < 1e-6);
    }

    #[test]
    fn test_soft_plus_derivative_at_zero() {
        // At zero, derivative = 0.5
        let result = soft_plus_derivative(0.0, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_european_call_itm() {
        // Deep ITM call: payoff ≈ S - K
        let payoff = european_call_smooth(110.0, 100.0, 1e-4);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_european_call_otm() {
        // Deep OTM call: payoff ≈ 0
        let payoff = european_call_smooth(90.0, 100.0, 1e-4);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_european_call_atm() {
        // ATM call: payoff ≈ 0.693 × ε
        let epsilon = 1e-4;
        let payoff = european_call_smooth(100.0, 100.0, epsilon);
        let expected = epsilon * 2.0_f64.ln();
        assert_relative_eq!(payoff, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_european_put_itm() {
        // Deep ITM put: payoff ≈ K - S
        let payoff = european_put_smooth(90.0, 100.0, 1e-4);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_european_put_otm() {
        // Deep OTM put: payoff ≈ 0
        let payoff = european_put_smooth(110.0, 100.0, 1e-4);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_put_call_parity_smooth() {
        // Smooth put-call parity: call - put ≈ S - K for deep ITM/OTM
        let strike = 100.0;
        let epsilon = 1e-6;

        // ITM case
        let s_itm = 120.0;
        let call_itm = european_call_smooth(s_itm, strike, epsilon);
        let put_itm = european_put_smooth(s_itm, strike, epsilon);
        assert_relative_eq!(call_itm - put_itm, s_itm - strike, epsilon = 0.01);

        // OTM case
        let s_otm = 80.0;
        let call_otm = european_call_smooth(s_otm, strike, epsilon);
        let put_otm = european_put_smooth(s_otm, strike, epsilon);
        assert_relative_eq!(call_otm - put_otm, s_otm - strike, epsilon = 0.01);
    }

    #[test]
    fn test_payoff_params_call() {
        let params = PayoffParams::call(105.0);
        assert_eq!(params.strike, 105.0);
        assert_eq!(params.payoff_type, PayoffType::Call);
    }

    #[test]
    fn test_payoff_params_put() {
        let params = PayoffParams::put(95.0);
        assert_eq!(params.strike, 95.0);
        assert_eq!(params.payoff_type, PayoffType::Put);
    }

    #[test]
    fn test_compute_payoff_call() {
        let params = PayoffParams::call(100.0).with_epsilon(1e-4);
        let payoff = compute_payoff(110.0, params);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_compute_payoff_put() {
        let params = PayoffParams::put(100.0).with_epsilon(1e-4);
        let payoff = compute_payoff(90.0, params);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_asian_call_smooth() {
        let path = vec![100.0, 105.0, 110.0, 115.0, 120.0];
        let avg = 110.0; // (100+105+110+115+120)/5
        let payoff = asian_arithmetic_call_smooth(&path, 100.0, 1e-4);
        assert_relative_eq!(payoff, avg - 100.0, epsilon = 0.01);
    }

    #[test]
    fn test_asian_put_smooth() {
        let path = vec![100.0, 95.0, 90.0, 85.0, 80.0];
        let avg = 90.0; // (100+95+90+85+80)/5
        let payoff = asian_arithmetic_put_smooth(&path, 100.0, 1e-4);
        assert_relative_eq!(payoff, 100.0 - avg, epsilon = 0.01);
    }

    #[test]
    fn test_asian_empty_path() {
        let payoff_call = asian_arithmetic_call_smooth(&[], 100.0, 1e-4);
        let payoff_put = asian_arithmetic_put_smooth(&[], 100.0, 1e-4);
        assert_eq!(payoff_call, 0.0);
        assert_eq!(payoff_put, 0.0);
    }
}

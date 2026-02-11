//! Smooth payoff functions for Monte Carlo pricing.

use pricer_core::math::smoothing::{smooth_indicator, smooth_max};

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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoffParams {
    /// Strike price.
    pub strike: f64,
    /// Payoff type (Call or Put).
    pub payoff_type: PayoffType,
    /// Smoothing epsilon for soft approximation.
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
#[inline]
pub fn soft_plus(x: f64, epsilon: f64) -> f64 {
    smooth_max(x, 0.0, epsilon)
}

/// Derivative of soft-plus: the sigmoid function.
#[inline]
pub fn soft_plus_derivative(x: f64, epsilon: f64) -> f64 {
    smooth_indicator(x, epsilon)
}

/// Computes smooth European call payoff.
#[inline]
pub fn european_call_smooth(terminal_price: f64, strike: f64, epsilon: f64) -> f64 {
    soft_plus(terminal_price - strike, epsilon)
}

/// Computes smooth European put payoff.
#[inline]
pub fn european_put_smooth(terminal_price: f64, strike: f64, epsilon: f64) -> f64 {
    soft_plus(strike - terminal_price, epsilon)
}

/// Computes payoff for a single path.
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
#[inline]
pub fn asian_arithmetic_call_smooth(path: &[f64], strike: f64, epsilon: f64) -> f64 {
    if path.is_empty() {
        return 0.0;
    }
    let avg = path.iter().sum::<f64>() / path.len() as f64;
    soft_plus(avg - strike, epsilon)
}

/// Computes smooth Asian arithmetic average put payoff.
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
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_soft_plus_positive() {
        let result = soft_plus(10.0, 0.01);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_soft_plus_negative() {
        let result = soft_plus(-10.0, 0.01);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_soft_plus_at_zero() {
        let epsilon = 1.0;
        let result = soft_plus(0.0, epsilon);
        assert_relative_eq!(result, 2.0_f64.ln(), epsilon = 1e-10);
    }

    #[test]
    fn test_soft_plus_derivative_positive() {
        let result = soft_plus_derivative(10.0, 0.01);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_soft_plus_derivative_negative() {
        let result = soft_plus_derivative(-10.0, 0.01);
        assert!(result < 1e-6);
    }

    #[test]
    fn test_soft_plus_derivative_at_zero() {
        let result = soft_plus_derivative(0.0, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_european_call_itm() {
        let payoff = european_call_smooth(110.0, 100.0, 1e-4);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_european_call_otm() {
        let payoff = european_call_smooth(90.0, 100.0, 1e-4);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_european_call_atm() {
        let epsilon = 1e-4;
        let payoff = european_call_smooth(100.0, 100.0, epsilon);
        let expected = epsilon * 2.0_f64.ln();
        assert_relative_eq!(payoff, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_european_put_itm() {
        let payoff = european_put_smooth(90.0, 100.0, 1e-4);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_european_put_otm() {
        let payoff = european_put_smooth(110.0, 100.0, 1e-4);
        assert!(payoff < 0.01);
        assert!(payoff >= 0.0);
    }

    #[test]
    fn test_put_call_parity_smooth() {
        let strike = 100.0;
        let epsilon = 1e-6;

        let s_itm = 120.0;
        let call_itm = european_call_smooth(s_itm, strike, epsilon);
        let put_itm = european_put_smooth(s_itm, strike, epsilon);
        assert_relative_eq!(call_itm - put_itm, s_itm - strike, epsilon = 0.01);

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
        let avg = 110.0;
        let payoff = asian_arithmetic_call_smooth(&path, 100.0, 1e-4);
        assert_relative_eq!(payoff, avg - 100.0, epsilon = 0.01);
    }

    #[test]
    fn test_asian_put_smooth() {
        let path = vec![100.0, 95.0, 90.0, 85.0, 80.0];
        let avg = 90.0;
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

    /// Tests for pricer_core integration (Phase 4, Task 1.2).
    mod core_integration_tests {
        use pricer_core::math::smoothing::{smooth_indicator, smooth_max};

        use super::*;

        /// Verify soft_plus delegates to pricer_core smooth_max correctly.
        #[test]
        fn test_soft_plus_delegates_to_smooth_max() {
            let test_cases = [
                (10.0, 0.01),
                (-10.0, 0.01),
                (0.0, 1.0),
                (1.0, 1e-4),
                (-1.0, 1e-4),
                (100.0, 1e-6),
                (0.5, 0.1),
            ];

            for (x, epsilon) in test_cases {
                let soft_plus_result = soft_plus(x, epsilon);
                let smooth_max_result = smooth_max(x, 0.0, epsilon);

                assert_relative_eq!(soft_plus_result, smooth_max_result, epsilon = 1e-10);
            }
        }

        /// Verify European call payoff uses pricer_core smoothing.
        #[test]
        fn test_european_call_uses_core_smoothing() {
            let strike = 100.0;
            let epsilon = 1e-4;

            for terminal in [80.0, 100.0, 120.0] {
                let payoff_result = european_call_smooth(terminal, strike, epsilon);
                let expected = smooth_max(terminal - strike, 0.0, epsilon);

                assert_relative_eq!(payoff_result, expected, epsilon = 1e-10);
            }
        }

        /// Verify European put payoff uses pricer_core smoothing.
        #[test]
        fn test_european_put_uses_core_smoothing() {
            let strike = 100.0;
            let epsilon = 1e-4;

            for terminal in [80.0, 100.0, 120.0] {
                let payoff_result = european_put_smooth(terminal, strike, epsilon);
                let expected = smooth_max(strike - terminal, 0.0, epsilon);

                assert_relative_eq!(payoff_result, expected, epsilon = 1e-10);
            }
        }

        /// Verify soft_plus_derivative delegates to smooth_indicator.
        #[test]
        fn test_soft_plus_derivative_delegates_to_smooth_indicator() {
            let test_cases = [
                (10.0, 0.01),
                (-10.0, 0.01),
                (0.0, 1.0),
            ];

            for (x, epsilon) in test_cases {
                let deriv_result = soft_plus_derivative(x, epsilon);
                let indicator_result = smooth_indicator(x, epsilon);

                assert_relative_eq!(deriv_result, indicator_result, epsilon = 1e-10);
            }
        }
    }
}

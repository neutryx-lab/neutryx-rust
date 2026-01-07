//! Lookback option payoff implementations.
//!
//! This module provides payoff implementations for lookback options:
//!
//! - **Fixed Strike Lookback Call**: max(S_max - K, 0)
//! - **Fixed Strike Lookback Put**: max(K - S_min, 0)
//! - **Floating Strike Lookback Call**: max(S_T - S_min, 0)
//! - **Floating Strike Lookback Put**: max(S_max - S_T, 0)
//!
//! Lookback options have payoffs based on path extrema (maximum or minimum).

use super::{ObservationType, PathDependentPayoff, PathObserver};
use num_traits::Float;

/// Lookback option type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookbackType {
    /// Fixed strike call: max(S_max - K, 0)
    FixedCall,
    /// Fixed strike put: max(K - S_min, 0)
    FixedPut,
    /// Floating strike call: max(S_T - S_min, 0) = S_T - S_min (always positive)
    FloatingCall,
    /// Floating strike put: max(S_max - S_T, 0) = S_max - S_T (always positive)
    FloatingPut,
}

impl LookbackType {
    /// Returns true if this uses fixed strike.
    #[inline]
    pub fn is_fixed(&self) -> bool {
        matches!(self, LookbackType::FixedCall | LookbackType::FixedPut)
    }

    /// Returns true if this is a call-type payoff.
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, LookbackType::FixedCall | LookbackType::FloatingCall)
    }
}

/// Parameters for lookback option payoffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookbackParams<T: Float> {
    /// Strike price (only used for fixed strike lookbacks)
    pub strike: T,
    /// Lookback type
    pub lookback_type: LookbackType,
    /// Smoothing epsilon for soft approximations
    pub smoothing_epsilon: T,
}

impl<T: Float> LookbackParams<T> {
    /// Creates new lookback parameters.
    #[inline]
    pub fn new(strike: T, lookback_type: LookbackType, epsilon: T) -> Self {
        Self {
            strike,
            lookback_type,
            smoothing_epsilon: epsilon,
        }
    }

    /// Creates fixed strike call parameters.
    #[inline]
    pub fn fixed_call(strike: T, epsilon: T) -> Self {
        Self::new(strike, LookbackType::FixedCall, epsilon)
    }

    /// Creates fixed strike put parameters.
    #[inline]
    pub fn fixed_put(strike: T, epsilon: T) -> Self {
        Self::new(strike, LookbackType::FixedPut, epsilon)
    }

    /// Creates floating strike call parameters.
    #[inline]
    pub fn floating_call(epsilon: T) -> Self {
        Self::new(T::zero(), LookbackType::FloatingCall, epsilon)
    }

    /// Creates floating strike put parameters.
    #[inline]
    pub fn floating_put(epsilon: T) -> Self {
        Self::new(T::zero(), LookbackType::FloatingPut, epsilon)
    }
}

/// Soft-plus function: smooth approximation of max(x, 0).
#[inline]
fn soft_plus<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();
    if scaled > twenty {
        x
    } else if scaled < -twenty {
        epsilon * scaled.exp()
    } else {
        epsilon * (T::one() + scaled.exp()).ln()
    }
}

/// Lookback option payoff.
///
/// Computes payoffs for all four lookback types using path extrema.
#[derive(Clone, Copy, Debug)]
pub struct LookbackPayoff<T: Float> {
    params: LookbackParams<T>,
}

impl<T: Float> LookbackPayoff<T> {
    /// Creates a new lookback payoff.
    #[inline]
    pub fn new(params: LookbackParams<T>) -> Self {
        Self { params }
    }

    /// Creates a fixed strike call.
    #[inline]
    pub fn fixed_call(strike: T, epsilon: T) -> Self {
        Self::new(LookbackParams::fixed_call(strike, epsilon))
    }

    /// Creates a fixed strike put.
    #[inline]
    pub fn fixed_put(strike: T, epsilon: T) -> Self {
        Self::new(LookbackParams::fixed_put(strike, epsilon))
    }

    /// Creates a floating strike call.
    #[inline]
    pub fn floating_call(epsilon: T) -> Self {
        Self::new(LookbackParams::floating_call(epsilon))
    }

    /// Creates a floating strike put.
    #[inline]
    pub fn floating_put(epsilon: T) -> Self {
        Self::new(LookbackParams::floating_put(epsilon))
    }
}

impl<T: Float + Send + Sync> PathDependentPayoff<T> for LookbackPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let epsilon = self.params.smoothing_epsilon;

        match self.params.lookback_type {
            LookbackType::FixedCall => {
                // max(S_max - K, 0)
                let intrinsic = observer.maximum() - self.params.strike;
                soft_plus(intrinsic, epsilon)
            }
            LookbackType::FixedPut => {
                // max(K - S_min, 0)
                let intrinsic = self.params.strike - observer.minimum();
                soft_plus(intrinsic, epsilon)
            }
            LookbackType::FloatingCall => {
                // S_T - S_min (always >= 0 for valid paths)
                let intrinsic = observer.terminal() - observer.minimum();
                soft_plus(intrinsic, epsilon)
            }
            LookbackType::FloatingPut => {
                // S_max - S_T (always >= 0 for valid paths)
                let intrinsic = observer.maximum() - observer.terminal();
                soft_plus(intrinsic, epsilon)
            }
        }
    }

    fn required_observations(&self) -> ObservationType {
        ObservationType::lookback()
    }

    fn smoothing_epsilon(&self) -> T {
        self.params.smoothing_epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // LookbackType Tests
    // ========================================================================

    #[test]
    fn test_lookback_type_is_fixed() {
        assert!(LookbackType::FixedCall.is_fixed());
        assert!(LookbackType::FixedPut.is_fixed());
        assert!(!LookbackType::FloatingCall.is_fixed());
        assert!(!LookbackType::FloatingPut.is_fixed());
    }

    #[test]
    fn test_lookback_type_is_call() {
        assert!(LookbackType::FixedCall.is_call());
        assert!(!LookbackType::FixedPut.is_call());
        assert!(LookbackType::FloatingCall.is_call());
        assert!(!LookbackType::FloatingPut.is_call());
    }

    // ========================================================================
    // Fixed Strike Call Tests
    // ========================================================================

    #[test]
    fn test_fixed_call_itm() {
        let payoff = LookbackPayoff::fixed_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 120, 110, 105] -> max = 120
        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.observe(105.0);
        observer.set_terminal(105.0);

        let result = payoff.compute(&[], &observer);
        // max - K = 120 - 100 = 20
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_fixed_call_otm() {
        let payoff = LookbackPayoff::fixed_call(130.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 120, 110, 105] -> max = 120 < 130
        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.observe(105.0);
        observer.set_terminal(105.0);

        let result = payoff.compute(&[], &observer);
        // max - K = 120 - 130 = -10 -> ~0
        assert!(result < 0.1);
    }

    // ========================================================================
    // Fixed Strike Put Tests
    // ========================================================================

    #[test]
    fn test_fixed_put_itm() {
        let payoff = LookbackPayoff::fixed_put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 95, 85, 90] -> min = 85
        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(90.0);
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        // K - min = 100 - 85 = 15
        assert_relative_eq!(result, 15.0, epsilon = 0.1);
    }

    #[test]
    fn test_fixed_put_otm() {
        let payoff = LookbackPayoff::fixed_put(80.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 95, 85, 90] -> min = 85 > 80
        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(90.0);
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        // K - min = 80 - 85 = -5 -> ~0
        assert!(result < 0.1);
    }

    // ========================================================================
    // Floating Strike Call Tests
    // ========================================================================

    #[test]
    fn test_floating_call() {
        let payoff = LookbackPayoff::floating_call(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 90, 95, 110] -> min = 90, terminal = 110
        observer.observe(100.0);
        observer.observe(90.0);
        observer.observe(95.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        // S_T - S_min = 110 - 90 = 20
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_floating_call_terminal_at_min() {
        let payoff = LookbackPayoff::floating_call(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 110, 95, 90] -> min = 90, terminal = 90
        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(95.0);
        observer.observe(90.0);
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        // S_T - S_min = 90 - 90 = 0
        assert!(result < 0.01);
    }

    // ========================================================================
    // Floating Strike Put Tests
    // ========================================================================

    #[test]
    fn test_floating_put() {
        let payoff = LookbackPayoff::floating_put(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 120, 110, 95] -> max = 120, terminal = 95
        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.observe(95.0);
        observer.set_terminal(95.0);

        let result = payoff.compute(&[], &observer);
        // S_max - S_T = 120 - 95 = 25
        assert_relative_eq!(result, 25.0, epsilon = 0.1);
    }

    #[test]
    fn test_floating_put_terminal_at_max() {
        let payoff = LookbackPayoff::floating_put(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Path: [100, 110, 115, 120] -> max = 120, terminal = 120
        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(115.0);
        observer.observe(120.0);
        observer.set_terminal(120.0);

        let result = payoff.compute(&[], &observer);
        // S_max - S_T = 120 - 120 = 0
        assert!(result < 0.01);
    }

    // ========================================================================
    // Required Observations Tests
    // ========================================================================

    #[test]
    fn test_lookback_requires_max_and_min() {
        let payoff = LookbackPayoff::fixed_call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_lookback_smoothing_epsilon() {
        let epsilon = 1e-4_f64;
        let payoff = LookbackPayoff::floating_call(epsilon);
        assert_eq!(payoff.smoothing_epsilon(), epsilon);
    }
}

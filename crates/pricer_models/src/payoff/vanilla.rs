//! Vanilla European and digital option payoffs for Monte Carlo pricing.

use num_traits::Float;

use super::{
    smooth_math::{smooth_indicator, soft_plus},
    structured::PathObserver,
    McPayoff, ObservationType,
};

/// Parameters for vanilla European option payoffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VanillaParams<T: Float> {
    /// Strike price.
    pub strike: T,
    /// Whether this is a call (true) or put (false).
    pub is_call: bool,
    /// Smoothing epsilon for soft approximations.
    pub smoothing_epsilon: T,
}

impl<T: Float> VanillaParams<T> {
    /// Creates parameters for a European call option.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: true,
            smoothing_epsilon: epsilon,
        }
    }

    /// Creates parameters for a European put option.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: false,
            smoothing_epsilon: epsilon,
        }
    }
}

/// Vanilla European option payoff (call or put).
///
/// Computes max(S_T - K, 0) for calls or max(K - S_T, 0) for puts,
/// using smooth soft-plus approximation for AD compatibility.
#[derive(Clone, Copy, Debug)]
pub struct VanillaPayoff<T: Float> {
    params: VanillaParams<T>,
}

impl<T: Float> VanillaPayoff<T> {
    /// Creates a new vanilla payoff.
    #[inline]
    pub fn new(params: VanillaParams<T>) -> Self { Self { params } }

    /// Creates a European call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self { Self::new(VanillaParams::call(strike, epsilon)) }

    /// Creates a European put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self { Self::new(VanillaParams::put(strike, epsilon)) }
}

impl<T: Float + Send + Sync> McPayoff<T> for VanillaPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let terminal = observer.terminal();
        let intrinsic = if self.params.is_call {
            terminal - self.params.strike
        } else {
            self.params.strike - terminal
        };
        soft_plus(intrinsic, self.params.smoothing_epsilon)
    }

    fn required_observations(&self) -> ObservationType { ObservationType::terminal_only() }

    fn smoothing_epsilon(&self) -> T { self.params.smoothing_epsilon }
}

/// Parameters for digital option payoffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DigitalParams<T: Float> {
    /// Strike price.
    pub strike: T,
    /// Whether this is a call (true) or put (false).
    pub is_call: bool,
    /// Smoothing epsilon for soft approximations.
    pub smoothing_epsilon: T,
}

impl<T: Float> DigitalParams<T> {
    /// Creates parameters for a digital call option.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: true,
            smoothing_epsilon: epsilon,
        }
    }

    /// Creates parameters for a digital put option.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self {
        Self {
            strike,
            is_call: false,
            smoothing_epsilon: epsilon,
        }
    }
}

/// Digital option payoff (binary call or put).
///
/// Pays 1 if S_T > K (call) or K > S_T (put), using smooth indicator
/// approximation for AD compatibility.
#[derive(Clone, Copy, Debug)]
pub struct DigitalPayoff<T: Float> {
    params: DigitalParams<T>,
}

impl<T: Float> DigitalPayoff<T> {
    /// Creates a new digital payoff.
    #[inline]
    pub fn new(params: DigitalParams<T>) -> Self { Self { params } }

    /// Creates a digital call option payoff.
    #[inline]
    pub fn call(strike: T, epsilon: T) -> Self { Self::new(DigitalParams::call(strike, epsilon)) }

    /// Creates a digital put option payoff.
    #[inline]
    pub fn put(strike: T, epsilon: T) -> Self { Self::new(DigitalParams::put(strike, epsilon)) }
}

impl<T: Float + Send + Sync> McPayoff<T> for DigitalPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let terminal = observer.terminal();
        let diff = if self.params.is_call {
            terminal - self.params.strike
        } else {
            self.params.strike - terminal
        };
        smooth_indicator(diff, self.params.smoothing_epsilon)
    }

    fn required_observations(&self) -> ObservationType { ObservationType::terminal_only() }

    fn smoothing_epsilon(&self) -> T { self.params.smoothing_epsilon }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_vanilla_call_itm() {
        let payoff = VanillaPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_vanilla_call_otm() {
        let payoff = VanillaPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_vanilla_put_itm() {
        let payoff = VanillaPayoff::put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_vanilla_put_otm() {
        let payoff = VanillaPayoff::put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.01);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_vanilla_put_call_parity() {
        let strike = 100.0_f64;
        let epsilon = 1e-8;
        let call = VanillaPayoff::call(strike, epsilon);
        let put = VanillaPayoff::put(strike, epsilon);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let c = call.compute(&[], &observer);
        let p = put.compute(&[], &observer);
        assert_relative_eq!(c - p, 110.0 - strike, epsilon = 0.01);
    }

    #[test]
    fn test_vanilla_required_observations() {
        let payoff = VanillaPayoff::call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_terminal);
        assert!(!obs.needs_average);
        assert!(!obs.needs_max);
        assert!(!obs.needs_min);
    }

    #[test]
    fn test_digital_call_itm() {
        let payoff = DigitalPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_digital_call_otm() {
        let payoff = DigitalPayoff::call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.01);
    }

    #[test]
    fn test_digital_put_itm() {
        let payoff = DigitalPayoff::put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_digital_put_otm() {
        let payoff = DigitalPayoff::put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.01);
    }

    #[test]
    fn test_digital_required_observations() {
        let payoff = DigitalPayoff::call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_terminal);
        assert!(!obs.needs_average);
        assert!(!obs.needs_max);
        assert!(!obs.needs_min);
    }

    #[test]
    fn test_vanilla_smoothing_epsilon() {
        let epsilon = 1e-4_f64;
        let payoff = VanillaPayoff::call(100.0, epsilon);
        assert_eq!(payoff.smoothing_epsilon(), epsilon);
    }

    #[test]
    fn test_digital_smoothing_epsilon() {
        let epsilon = 1e-4_f64;
        let payoff = DigitalPayoff::call(100.0, epsilon);
        assert_eq!(payoff.smoothing_epsilon(), epsilon);
    }
}

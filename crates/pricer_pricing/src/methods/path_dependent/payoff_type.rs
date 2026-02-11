//! PathPayoffType enum for static dispatch of path-dependent payoffs.

use enum_dispatch::enum_dispatch;
use num_traits::Float;

use super::{
    AsianArithmeticPayoff, AsianGeometricPayoff, AsianParams, BarrierPayoff, LookbackPayoff,
    ObservationType, PathDependentPayoff, PathObserver,
};

/// Enum encompassing all path-dependent option payoff types.
#[derive(Clone, Copy, Debug)]
#[enum_dispatch(PathDependentPayoff<T>)]
pub enum PathPayoffType<T: Float + Send + Sync> {
    /// Arithmetic average Asian option (call or put)
    AsianArithmetic(AsianArithmeticPayoff<T>),
    /// Geometric average Asian option (call or put)
    AsianGeometric(AsianGeometricPayoff<T>),
    /// Barrier option (Up/Down, In/Out, Call/Put)
    Barrier(BarrierPayoff<T>),
    /// Lookback option (Fixed/Floating, Call/Put)
    Lookback(LookbackPayoff<T>),
}

impl<T: Float + Send + Sync> PathPayoffType<T> {
    /// Creates an arithmetic average Asian call option.
    #[inline]
    pub fn asian_arithmetic_call(strike: T, epsilon: T) -> Self {
        Self::AsianArithmetic(AsianArithmeticPayoff::new(AsianParams::call(
            strike, epsilon,
        )))
    }

    /// Creates an arithmetic average Asian put option.
    #[inline]
    pub fn asian_arithmetic_put(strike: T, epsilon: T) -> Self {
        Self::AsianArithmetic(AsianArithmeticPayoff::new(AsianParams::put(
            strike, epsilon,
        )))
    }

    /// Creates a geometric average Asian call option.
    #[inline]
    pub fn asian_geometric_call(strike: T, epsilon: T) -> Self {
        Self::AsianGeometric(AsianGeometricPayoff::new(AsianParams::call(
            strike, epsilon,
        )))
    }

    /// Creates a geometric average Asian put option.
    #[inline]
    pub fn asian_geometric_put(strike: T, epsilon: T) -> Self {
        Self::AsianGeometric(AsianGeometricPayoff::new(AsianParams::put(strike, epsilon)))
    }

    /// Creates an up-and-in call barrier option.
    #[inline]
    pub fn barrier_up_in_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::Barrier(BarrierPayoff::up_in_call(strike, barrier, epsilon))
    }

    /// Creates an up-and-out call barrier option.
    #[inline]
    pub fn barrier_up_out_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::Barrier(BarrierPayoff::up_out_call(strike, barrier, epsilon))
    }

    /// Creates a down-and-in put barrier option.
    #[inline]
    pub fn barrier_down_in_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::Barrier(BarrierPayoff::down_in_put(strike, barrier, epsilon))
    }

    /// Creates a down-and-out put barrier option.
    #[inline]
    pub fn barrier_down_out_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::Barrier(BarrierPayoff::down_out_put(strike, barrier, epsilon))
    }

    /// Creates a fixed strike lookback call option.
    #[inline]
    pub fn lookback_fixed_call(strike: T, epsilon: T) -> Self {
        Self::Lookback(LookbackPayoff::fixed_call(strike, epsilon))
    }

    /// Creates a fixed strike lookback put option.
    #[inline]
    pub fn lookback_fixed_put(strike: T, epsilon: T) -> Self {
        Self::Lookback(LookbackPayoff::fixed_put(strike, epsilon))
    }

    /// Creates a floating strike lookback call option.
    #[inline]
    pub fn lookback_floating_call(epsilon: T) -> Self {
        Self::Lookback(LookbackPayoff::floating_call(epsilon))
    }

    /// Creates a floating strike lookback put option.
    #[inline]
    pub fn lookback_floating_put(epsilon: T) -> Self {
        Self::Lookback(LookbackPayoff::floating_put(epsilon))
    }

    /// Returns true if this is an Asian option.
    #[inline]
    pub fn is_asian(&self) -> bool {
        matches!(
            self,
            PathPayoffType::AsianArithmetic(_) | PathPayoffType::AsianGeometric(_)
        )
    }

    /// Returns true if this is a barrier option.
    #[inline]
    pub fn is_barrier(&self) -> bool { matches!(self, PathPayoffType::Barrier(_)) }

    /// Returns true if this is a lookback option.
    #[inline]
    pub fn is_lookback(&self) -> bool { matches!(self, PathPayoffType::Lookback(_)) }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_enum_asian_arithmetic_call() {
        let payoff = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);
        observer.set_terminal(120.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_asian_arithmetic_put() {
        let payoff = PathPayoffType::asian_arithmetic_put(110.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(100.0);
        observer.set_terminal(100.0);

        let result = payoff.compute(&[], &observer);
        assert!(result > 8.0 && result < 9.0);
    }

    #[test]
    fn test_enum_asian_geometric_call() {
        let payoff = PathPayoffType::asian_geometric_call(95.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(100.0);
        observer.observe(100.0);
        observer.set_terminal(100.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_asian_geometric_put() {
        let payoff = PathPayoffType::asian_geometric_put(105.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(100.0);
        observer.observe(100.0);
        observer.set_terminal(100.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_up_in_call_hit() {
        let payoff = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(115.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_up_out_call_not_hit() {
        let payoff = PathPayoffType::barrier_up_out_call(100.0_f64, 120.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(105.0);
        observer.set_terminal(105.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_down_in_put_hit() {
        let payoff = PathPayoffType::barrier_down_in_put(100.0_f64, 90.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 8.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_down_out_put_not_hit() {
        let payoff = PathPayoffType::barrier_down_out_put(100.0_f64, 80.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(90.0);
        observer.observe(95.0);
        observer.set_terminal(95.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_fixed_call() {
        let payoff = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_fixed_put() {
        let payoff = PathPayoffType::lookback_fixed_put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(90.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 15.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_floating_call() {
        let payoff = PathPayoffType::lookback_floating_call(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(90.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_floating_put() {
        let payoff = PathPayoffType::lookback_floating_put(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.set_terminal(100.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_required_observations_asian_arithmetic() {
        let payoff = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_average);
        assert!(!obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_enum_required_observations_asian_geometric() {
        let payoff = PathPayoffType::asian_geometric_call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(!obs.needs_average);
        assert!(obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_enum_required_observations_barrier_up() {
        let payoff = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_max);
        assert!(!obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_enum_required_observations_barrier_down() {
        let payoff = PathPayoffType::barrier_down_in_put(100.0_f64, 90.0, 1e-6);
        let obs = payoff.required_observations();
        assert!(!obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_enum_required_observations_lookback() {
        let payoff = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_enum_is_asian() {
        let asian1 = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let asian2 = PathPayoffType::asian_geometric_call(100.0_f64, 1e-6);
        let barrier = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let lookback = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);

        assert!(asian1.is_asian());
        assert!(asian2.is_asian());
        assert!(!barrier.is_asian());
        assert!(!lookback.is_asian());
    }

    #[test]
    fn test_enum_is_barrier() {
        let asian = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let barrier = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let lookback = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);

        assert!(!asian.is_barrier());
        assert!(barrier.is_barrier());
        assert!(!lookback.is_barrier());
    }

    #[test]
    fn test_enum_is_lookback() {
        let asian = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let barrier = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let lookback = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);

        assert!(!asian.is_lookback());
        assert!(!barrier.is_lookback());
        assert!(lookback.is_lookback());
    }

    #[test]
    fn test_enum_smoothing_epsilon() {
        let epsilon = 1e-4_f64;

        let asian = PathPayoffType::asian_arithmetic_call(100.0, epsilon);
        let geometric = PathPayoffType::asian_geometric_call(100.0, epsilon);
        let barrier = PathPayoffType::barrier_up_in_call(100.0, 110.0, epsilon);
        let lookback = PathPayoffType::lookback_fixed_call(100.0, epsilon);

        assert_eq!(asian.smoothing_epsilon(), epsilon);
        assert_eq!(geometric.smoothing_epsilon(), epsilon);
        assert_eq!(barrier.smoothing_epsilon(), epsilon);
        assert_eq!(lookback.smoothing_epsilon(), epsilon);
    }

    #[test]
    fn test_enum_clone() {
        let payoff = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let cloned = payoff.clone();

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result1 = payoff.compute(&[], &observer);
        let result2 = cloned.compute(&[], &observer);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_enum_copy() {
        let payoff = PathPayoffType::lookback_floating_call(1e-6_f64);
        let copied: PathPayoffType<f64> = payoff;

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(90.0);
        observer.set_terminal(110.0);

        let result1 = payoff.compute(&[], &observer);
        let result2 = copied.compute(&[], &observer);

        assert_eq!(result1, result2);
    }
}

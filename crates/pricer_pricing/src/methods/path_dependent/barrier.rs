//! Barrier option payoff implementations.

use num_traits::Float;

use super::{ObservationType, PathDependentPayoff, PathObserver};

/// Barrier type enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierType {
    /// Up-and-In: activates when price crosses barrier from below
    UpIn,
    /// Up-and-Out: deactivates when price crosses barrier from below
    UpOut,
    /// Down-and-In: activates when price crosses barrier from above
    DownIn,
    /// Down-and-Out: deactivates when price crosses barrier from above
    DownOut,
}

impl BarrierType {
    /// Returns true if this is an "up" barrier (uses path maximum).
    #[inline]
    pub fn is_up(&self) -> bool { matches!(self, BarrierType::UpIn | BarrierType::UpOut) }

    /// Returns true if this is an "in" barrier (knock-in).
    #[inline]
    pub fn is_in(&self) -> bool { matches!(self, BarrierType::UpIn | BarrierType::DownIn) }
}

/// Parameters for barrier option payoffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarrierParams<T: Float> {
    /// Strike price
    pub strike: T,
    /// Barrier level
    pub barrier: T,
    /// Barrier type (Up/Down, In/Out)
    pub barrier_type: BarrierType,
    /// Whether this is a call (true) or put (false)
    pub is_call: bool,
    /// Smoothing epsilon for soft approximations
    pub smoothing_epsilon: T,
}

impl<T: Float> BarrierParams<T> {
    /// Creates new barrier parameters.
    #[inline]
    pub fn new(
        strike: T,
        barrier: T,
        barrier_type: BarrierType,
        is_call: bool,
        epsilon: T,
    ) -> Self {
        Self {
            strike,
            barrier,
            barrier_type,
            is_call,
            smoothing_epsilon: epsilon,
        }
    }

    /// Creates Up-and-In call parameters.
    #[inline]
    pub fn up_in_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(strike, barrier, BarrierType::UpIn, true, epsilon)
    }

    /// Creates Up-and-Out call parameters.
    #[inline]
    pub fn up_out_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(strike, barrier, BarrierType::UpOut, true, epsilon)
    }

    /// Creates Down-and-In put parameters.
    #[inline]
    pub fn down_in_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(strike, barrier, BarrierType::DownIn, false, epsilon)
    }

    /// Creates Down-and-Out put parameters.
    #[inline]
    pub fn down_out_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(strike, barrier, BarrierType::DownOut, false, epsilon)
    }
}

/// Smooth indicator function: approximation of Heaviside step.
#[inline]
fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
    let scaled = x / epsilon;
    let twenty = T::from(20.0).unwrap();
    if scaled > twenty {
        T::one()
    } else if scaled < -twenty {
        T::zero()
    } else {
        T::one() / (T::one() + (-scaled).exp())
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

/// Barrier option payoff.
#[derive(Clone, Copy, Debug)]
pub struct BarrierPayoff<T: Float> {
    params: BarrierParams<T>,
}

impl<T: Float> BarrierPayoff<T> {
    /// Creates a new barrier payoff.
    #[inline]
    pub fn new(params: BarrierParams<T>) -> Self { Self { params } }

    /// Creates an Up-and-In call.
    #[inline]
    pub fn up_in_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(BarrierParams::up_in_call(strike, barrier, epsilon))
    }

    /// Creates an Up-and-Out call.
    #[inline]
    pub fn up_out_call(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(BarrierParams::up_out_call(strike, barrier, epsilon))
    }

    /// Creates a Down-and-In put.
    #[inline]
    pub fn down_in_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(BarrierParams::down_in_put(strike, barrier, epsilon))
    }

    /// Creates a Down-and-Out put.
    #[inline]
    pub fn down_out_put(strike: T, barrier: T, epsilon: T) -> Self {
        Self::new(BarrierParams::down_out_put(strike, barrier, epsilon))
    }

    /// Computes the barrier indicator based on path extremum.
    fn barrier_indicator(&self, observer: &PathObserver<T>) -> T {
        let epsilon = self.params.smoothing_epsilon;

        match self.params.barrier_type {
            BarrierType::UpIn => {
                let max_price = observer.maximum();
                smooth_indicator(max_price - self.params.barrier, epsilon)
            }
            BarrierType::UpOut => {
                let max_price = observer.maximum();
                T::one() - smooth_indicator(max_price - self.params.barrier, epsilon)
            }
            BarrierType::DownIn => {
                let min_price = observer.minimum();
                smooth_indicator(self.params.barrier - min_price, epsilon)
            }
            BarrierType::DownOut => {
                let min_price = observer.minimum();
                T::one() - smooth_indicator(self.params.barrier - min_price, epsilon)
            }
        }
    }

    /// Computes the vanilla payoff (without barrier condition).
    fn vanilla_payoff(&self, terminal: T) -> T {
        let epsilon = self.params.smoothing_epsilon;
        let intrinsic = if self.params.is_call {
            terminal - self.params.strike
        } else {
            self.params.strike - terminal
        };
        soft_plus(intrinsic, epsilon)
    }
}

impl<T: Float + Send + Sync> PathDependentPayoff<T> for BarrierPayoff<T> {
    fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
        let terminal = observer.terminal();
        let barrier_ind = self.barrier_indicator(observer);
        let vanilla = self.vanilla_payoff(terminal);

        barrier_ind * vanilla
    }

    fn required_observations(&self) -> ObservationType {
        ObservationType::barrier(self.params.barrier_type.is_up())
    }

    fn smoothing_epsilon(&self) -> T { self.params.smoothing_epsilon }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_barrier_type_is_up() {
        assert!(BarrierType::UpIn.is_up());
        assert!(BarrierType::UpOut.is_up());
        assert!(!BarrierType::DownIn.is_up());
        assert!(!BarrierType::DownOut.is_up());
    }

    #[test]
    fn test_barrier_type_is_in() {
        assert!(BarrierType::UpIn.is_in());
        assert!(!BarrierType::UpOut.is_in());
        assert!(BarrierType::DownIn.is_in());
        assert!(!BarrierType::DownOut.is_in());
    }

    #[test]
    fn test_smooth_indicator_positive() {
        let result = smooth_indicator(10.0_f64, 0.01);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_smooth_indicator_negative() {
        let result = smooth_indicator(-10.0_f64, 0.01);
        assert!(result < 1e-6);
    }

    #[test]
    fn test_smooth_indicator_at_zero() {
        let result = smooth_indicator(0.0_f64, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_up_in_call_barrier_hit() {
        let payoff = BarrierPayoff::up_in_call(100.0_f64, 110.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(115.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_up_in_call_barrier_not_hit() {
        let payoff = BarrierPayoff::up_in_call(100.0_f64, 120.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(110.0);
        observer.observe(108.0);
        observer.set_terminal(108.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.1);
    }

    #[test]
    fn test_up_out_call_barrier_not_hit() {
        let payoff = BarrierPayoff::up_out_call(100.0_f64, 120.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(115.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_up_out_call_barrier_hit() {
        let payoff = BarrierPayoff::up_out_call(100.0_f64, 110.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(115.0);
        observer.observe(112.0);
        observer.set_terminal(112.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.1);
    }

    #[test]
    fn test_down_in_put_barrier_hit() {
        let payoff = BarrierPayoff::down_in_put(100.0_f64, 90.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 8.0, epsilon = 0.1);
    }

    #[test]
    fn test_down_in_put_barrier_not_hit() {
        let payoff = BarrierPayoff::down_in_put(100.0_f64, 80.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.1);
    }

    #[test]
    fn test_down_out_put_barrier_not_hit() {
        let payoff = BarrierPayoff::down_out_put(100.0_f64, 80.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 8.0, epsilon = 0.1);
    }

    #[test]
    fn test_down_out_put_barrier_hit() {
        let payoff = BarrierPayoff::down_out_put(100.0_f64, 90.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(95.0);
        observer.observe(85.0);
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        assert!(result < 0.1);
    }

    #[test]
    fn test_up_barrier_requires_max() {
        let payoff = BarrierPayoff::up_in_call(100.0_f64, 110.0, 1e-6);
        let obs = payoff.required_observations();
        assert!(obs.needs_max);
        assert!(!obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_down_barrier_requires_min() {
        let payoff = BarrierPayoff::down_in_put(100.0_f64, 90.0, 1e-6);
        let obs = payoff.required_observations();
        assert!(!obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }
}

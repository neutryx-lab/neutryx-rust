//! Path-dependent option pricing infrastructure.

mod asian;
mod barrier;
mod lookback;
mod observer;
mod payoff;
mod payoff_type;
pub(crate) mod smooth_math;

pub use asian::{AsianArithmeticPayoff, AsianGeometricPayoff, AsianParams};
pub use barrier::{BarrierParams, BarrierPayoff, BarrierType};
pub use lookback::{LookbackParams, LookbackPayoff, LookbackType};
pub use observer::{PathObserver, PathObserverState};
pub use payoff::{ObservationType, PathDependentPayoff};
pub use payoff_type::PathPayoffType;

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_path_observer_new() {
        let observer: PathObserver<f64> = PathObserver::new();
        assert_eq!(observer.count(), 0);
    }

    #[test]
    fn test_path_observer_single_observation() {
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);

        assert_eq!(observer.count(), 1);
        assert_relative_eq!(observer.arithmetic_average(), 100.0, epsilon = 1e-10);
        assert_relative_eq!(observer.maximum(), 100.0, epsilon = 1e-10);
        assert_relative_eq!(observer.minimum(), 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_multiple_observations() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.observe(110.0);
        observer.observe(95.0);
        observer.observe(100.0);

        assert_eq!(observer.count(), 5);

        assert_relative_eq!(observer.arithmetic_average(), 102.0, epsilon = 1e-10);

        assert_relative_eq!(observer.maximum(), 110.0, epsilon = 1e-10);

        assert_relative_eq!(observer.minimum(), 95.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_geometric_average() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(100.0);
        observer.observe(100.0);

        assert_relative_eq!(observer.geometric_average(), 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_geometric_average_varied() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(2.0);
        observer.observe(8.0);

        assert_relative_eq!(observer.geometric_average(), 4.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_terminal() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(105.0);
        observer.set_terminal(110.0);

        assert_relative_eq!(observer.terminal(), 110.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_reset() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.set_terminal(120.0);

        assert_eq!(observer.count(), 2);

        observer.reset();

        assert_eq!(observer.count(), 0);
    }

    #[test]
    fn test_path_observer_state_snapshot() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(90.0);

        let state = observer.snapshot();

        assert_eq!(state.count, 3);
        assert_relative_eq!(state.running_sum, 300.0, epsilon = 1e-10);
        assert_relative_eq!(state.running_max, 110.0, epsilon = 1e-10);
        assert_relative_eq!(state.running_min, 90.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_restore_from_state() {
        let mut observer1: PathObserver<f64> = PathObserver::new();
        observer1.observe(100.0);
        observer1.observe(110.0);

        let state = observer1.snapshot();

        let mut observer2: PathObserver<f64> = PathObserver::new();
        observer2.restore(&state);

        assert_eq!(observer2.count(), 2);
        assert_relative_eq!(observer2.arithmetic_average(), 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_empty_average() {
        let observer: PathObserver<f64> = PathObserver::new();

        assert_eq!(observer.arithmetic_average(), 0.0);
    }

    #[test]
    fn test_path_observer_f32() {
        let mut observer: PathObserver<f32> = PathObserver::new();
        observer.observe(100.0_f32);
        observer.observe(200.0_f32);

        assert_eq!(observer.count(), 2);
        assert!((observer.arithmetic_average() - 150.0_f32).abs() < 1e-5);
    }
}

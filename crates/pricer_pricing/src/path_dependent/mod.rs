//! Path-dependent option pricing infrastructure.
//!
//! This module provides the core infrastructure for pricing path-dependent
//! derivatives such as Asian, barrier, and lookback options.
//!
//! # Key Components
//!
//! - [`PathObserver`]: Streaming statistics accumulation for path observations
//! - [`PathDependentPayoff`]: Trait for path-dependent payoff computation
//! - [`ObservationType`]: Flags specifying required path statistics
//!
//! # Design Philosophy
//!
//! - **Streaming accumulation**: Statistics are computed incrementally as prices
//!   are observed, avoiding full path storage when possible
//! - **Enzyme AD compatible**: All computations use smooth approximations and
//!   avoid branches on floating-point values
//! - **Static dispatch**: Enum-based dispatch for payoff types ensures
//!   LLVM-level optimization

mod asian;
mod barrier;
mod lookback;
mod observer;
mod payoff;
mod payoff_type;

pub use asian::{AsianArithmeticPayoff, AsianGeometricPayoff, AsianParams};
pub use barrier::{BarrierParams, BarrierPayoff, BarrierType};
pub use lookback::{LookbackParams, LookbackPayoff, LookbackType};
pub use observer::{PathObserver, PathObserverState};
pub use payoff::{ObservationType, PathDependentPayoff};
pub use payoff_type::PathPayoffType;

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // PathObserver Tests (TDD)
    // ========================================================================

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

        // Arithmetic average: (100 + 105 + 110 + 95 + 100) / 5 = 510 / 5 = 102
        assert_relative_eq!(observer.arithmetic_average(), 102.0, epsilon = 1e-10);

        // Maximum: 110
        assert_relative_eq!(observer.maximum(), 110.0, epsilon = 1e-10);

        // Minimum: 95
        assert_relative_eq!(observer.minimum(), 95.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_geometric_average() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Geometric mean of [100, 100, 100] = 100
        observer.observe(100.0);
        observer.observe(100.0);
        observer.observe(100.0);

        assert_relative_eq!(observer.geometric_average(), 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_geometric_average_varied() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Geometric mean of [2, 8] = sqrt(16) = 4
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
        // After reset, max/min should be initial values (inf/-inf)
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

        // Create a new observer and restore from state
        let mut observer2: PathObserver<f64> = PathObserver::new();
        observer2.restore(&state);

        assert_eq!(observer2.count(), 2);
        assert_relative_eq!(observer2.arithmetic_average(), 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_path_observer_empty_average() {
        let observer: PathObserver<f64> = PathObserver::new();

        // Empty observer should return 0 for arithmetic average
        assert_eq!(observer.arithmetic_average(), 0.0);
    }

    #[test]
    fn test_path_observer_f32() {
        // Test generic Float support
        let mut observer: PathObserver<f32> = PathObserver::new();
        observer.observe(100.0_f32);
        observer.observe(200.0_f32);

        assert_eq!(observer.count(), 2);
        assert!((observer.arithmetic_average() - 150.0_f32).abs() < 1e-5);
    }
}

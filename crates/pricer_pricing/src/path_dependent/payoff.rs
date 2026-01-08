//! PathDependentPayoff trait and observation types.
//!
//! This module defines the core trait for path-dependent payoff calculations
//! and the observation type flags that specify what statistics are needed.

use super::PathObserver;
use num_traits::Float;

/// Observation type flags for path-dependent payoffs.
///
/// Specifies which path statistics are required for payoff computation.
/// This allows the simulation engine to optimize by only computing
/// the necessary statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservationType {
    /// Whether arithmetic average is needed (Asian options)
    pub needs_average: bool,
    /// Whether geometric average is needed (Geometric Asian options)
    pub needs_geometric_average: bool,
    /// Whether path maximum is needed (Up barriers, lookbacks)
    pub needs_max: bool,
    /// Whether path minimum is needed (Down barriers, lookbacks)
    pub needs_min: bool,
    /// Whether terminal price is needed (most options)
    pub needs_terminal: bool,
}

impl ObservationType {
    /// Creates observation type that only needs the terminal price.
    #[inline]
    pub fn terminal_only() -> Self {
        Self {
            needs_terminal: true,
            ..Default::default()
        }
    }

    /// Creates observation type for arithmetic Asian options.
    #[inline]
    pub fn arithmetic_asian() -> Self {
        Self {
            needs_average: true,
            needs_terminal: true,
            ..Default::default()
        }
    }

    /// Creates observation type for geometric Asian options.
    #[inline]
    pub fn geometric_asian() -> Self {
        Self {
            needs_geometric_average: true,
            needs_terminal: true,
            ..Default::default()
        }
    }

    /// Creates observation type for barrier options.
    #[inline]
    pub fn barrier(is_up: bool) -> Self {
        Self {
            needs_max: is_up,
            needs_min: !is_up,
            needs_terminal: true,
            ..Default::default()
        }
    }

    /// Creates observation type for lookback options.
    #[inline]
    pub fn lookback() -> Self {
        Self {
            needs_max: true,
            needs_min: true,
            needs_terminal: true,
            ..Default::default()
        }
    }

    /// Creates observation type that needs all statistics.
    #[inline]
    pub fn all() -> Self {
        Self {
            needs_average: true,
            needs_geometric_average: true,
            needs_max: true,
            needs_min: true,
            needs_terminal: true,
        }
    }
}

/// Trait for path-dependent payoff calculations.
///
/// This trait provides a unified interface for computing payoffs that
/// depend on the entire price path, not just the terminal price.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` for parallel Monte Carlo.
///
/// # Example
///
/// ```ignore
/// use pricer_pricing::path_dependent::{PathDependentPayoff, PathObserver, ObservationType};
///
/// struct AsianCall<T: Float> {
///     strike: T,
///     epsilon: T,
/// }
///
/// impl<T: Float> PathDependentPayoff<T> for AsianCall<T> {
///     fn compute(&self, _path: &[T], observer: &PathObserver<T>) -> T {
///         let avg = observer.arithmetic_average();
///         smooth_max(avg - self.strike, T::zero(), self.epsilon)
///     }
///
///     fn required_observations(&self) -> ObservationType {
///         ObservationType::arithmetic_asian()
///     }
///
///     fn smoothing_epsilon(&self) -> T {
///         self.epsilon
///     }
/// }
/// ```
pub trait PathDependentPayoff<T: Float>: Send + Sync {
    /// Computes the payoff from path statistics.
    ///
    /// # Arguments
    ///
    /// * `path` - The full price path (may be empty if not needed)
    /// * `observer` - Observer containing streaming statistics
    ///
    /// # Returns
    ///
    /// The payoff value.
    fn compute(&self, path: &[T], observer: &PathObserver<T>) -> T;

    /// Returns the observation types required for this payoff.
    ///
    /// This allows the simulation engine to optimize by only computing
    /// the necessary statistics.
    fn required_observations(&self) -> ObservationType;

    /// Returns the smoothing epsilon used for smooth approximations.
    fn smoothing_epsilon(&self) -> T;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_type_terminal_only() {
        let obs = ObservationType::terminal_only();
        assert!(!obs.needs_average);
        assert!(!obs.needs_geometric_average);
        assert!(!obs.needs_max);
        assert!(!obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_arithmetic_asian() {
        let obs = ObservationType::arithmetic_asian();
        assert!(obs.needs_average);
        assert!(!obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_geometric_asian() {
        let obs = ObservationType::geometric_asian();
        assert!(!obs.needs_average);
        assert!(obs.needs_geometric_average);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_barrier_up() {
        let obs = ObservationType::barrier(true);
        assert!(obs.needs_max);
        assert!(!obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_barrier_down() {
        let obs = ObservationType::barrier(false);
        assert!(!obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_lookback() {
        let obs = ObservationType::lookback();
        assert!(obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_all() {
        let obs = ObservationType::all();
        assert!(obs.needs_average);
        assert!(obs.needs_geometric_average);
        assert!(obs.needs_max);
        assert!(obs.needs_min);
        assert!(obs.needs_terminal);
    }

    #[test]
    fn test_observation_type_default() {
        let obs = ObservationType::default();
        assert!(!obs.needs_average);
        assert!(!obs.needs_geometric_average);
        assert!(!obs.needs_max);
        assert!(!obs.needs_min);
        assert!(!obs.needs_terminal);
    }

    // Test that a simple implementation compiles and works
    struct MockPayoff {
        strike: f64,
        epsilon: f64,
    }

    impl PathDependentPayoff<f64> for MockPayoff {
        fn compute(&self, _path: &[f64], observer: &PathObserver<f64>) -> f64 {
            let avg = observer.arithmetic_average();
            if avg > self.strike {
                avg - self.strike
            } else {
                0.0
            }
        }

        fn required_observations(&self) -> ObservationType {
            ObservationType::arithmetic_asian()
        }

        fn smoothing_epsilon(&self) -> f64 {
            self.epsilon
        }
    }

    #[test]
    fn test_mock_payoff_implementation() {
        let payoff = MockPayoff {
            strike: 100.0,
            epsilon: 1e-6,
        };

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);

        // Average = 110, Strike = 100, Payoff = 10
        let result = payoff.compute(&[], &observer);
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_mock_payoff_otm() {
        let payoff = MockPayoff {
            strike: 120.0,
            epsilon: 1e-6,
        };

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(110.0);

        // Average = 105, Strike = 120, Payoff = 0
        let result = payoff.compute(&[], &observer);
        assert_eq!(result, 0.0);
    }
}

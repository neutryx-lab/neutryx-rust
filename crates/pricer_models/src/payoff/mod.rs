//! Monte Carlo payoff trait and unified payoff types.
//!
//! This module provides the [`McPayoff`] trait for all Monte Carlo payoff
//! computations, along with concrete implementations for both vanilla
//! (European) and path-dependent (Asian, Barrier, Lookback) options.
//!
//! ## Module Organisation
//!
//! - [`structured`] - Structured/path-dependent payoffs (Asian, Barrier,
//!   Lookback) and the [`PathObserver`] streaming statistics accumulator.
//! - `vanilla` - Vanilla European and digital payoffs.
//! - `smooth_math` - Shared smooth approximation functions (soft-plus, smooth
//!   indicator).
//!
//! ## Unified Dispatch
//!
//! [`PayoffKind`] is an enum-dispatch wrapper covering both vanilla and
//! structured payoffs, enabling static dispatch without trait objects.

pub(crate) mod smooth_math;
pub mod structured;
mod vanilla;

use enum_dispatch::enum_dispatch;
use num_traits::Float;
pub use structured::{
    AsianArithmeticPayoff, AsianGeometricPayoff, AsianParams, BarrierParams, BarrierPayoff,
    BarrierType, LookbackParams, LookbackPayoff, LookbackType, PathObserver, PathObserverState,
};
pub use vanilla::{DigitalParams, DigitalPayoff, VanillaParams, VanillaPayoff};

// ---------------------------------------------------------------------------
// ObservationType
// ---------------------------------------------------------------------------

/// Observation type flags for Monte Carlo payoffs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservationType {
    /// Whether arithmetic average is needed (Asian options).
    pub needs_average: bool,
    /// Whether geometric average is needed (Geometric Asian options).
    pub needs_geometric_average: bool,
    /// Whether path maximum is needed (Up barriers, lookbacks).
    pub needs_max: bool,
    /// Whether path minimum is needed (Down barriers, lookbacks).
    pub needs_min: bool,
    /// Whether terminal price is needed (most options).
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

// ---------------------------------------------------------------------------
// McPayoff trait
// ---------------------------------------------------------------------------

/// Unified trait for Monte Carlo payoff evaluation.
///
/// Covers both vanilla European options and path-dependent exotics.
/// All payoffs are evaluated against a [`PathObserver`] which accumulates
/// path statistics during simulation.
///
/// # Type Parameters
///
/// * `T` - Numeric type (`f64` or AD type). Must be `Float + Send + Sync`.
///
/// # Design Principles
///
/// - **Enum-based dispatch** via `enum_dispatch` for Enzyme AD compatibility.
/// - **Smooth approximations** via configurable epsilon for differentiability.
/// - **Generic over `T: Float`** to support both `f64` and automatic
///   differentiation types.
#[enum_dispatch]
pub trait McPayoff<T: Float>: Send + Sync {
    /// Computes the payoff from path statistics.
    fn compute(&self, path: &[T], observer: &PathObserver<T>) -> T;

    /// Returns the observation types required for this payoff.
    fn required_observations(&self) -> ObservationType;

    /// Returns the smoothing epsilon used for smooth approximations.
    fn smoothing_epsilon(&self) -> T;
}

// ---------------------------------------------------------------------------
// PayoffKind unified enum
// ---------------------------------------------------------------------------

/// Enum encompassing all Monte Carlo payoff types.
///
/// Provides static dispatch across vanilla and path-dependent payoffs
/// via `enum_dispatch`.
#[derive(Clone, Copy, Debug)]
#[enum_dispatch(McPayoff<T>)]
pub enum PayoffKind<T: Float + Send + Sync> {
    /// Vanilla European option (call or put).
    Vanilla(VanillaPayoff<T>),
    /// Digital option (binary call or put).
    Digital(DigitalPayoff<T>),
    /// Arithmetic average Asian option (call or put).
    AsianArithmetic(AsianArithmeticPayoff<T>),
    /// Geometric average Asian option (call or put).
    AsianGeometric(AsianGeometricPayoff<T>),
    /// Barrier option (Up/Down, In/Out, Call/Put).
    Barrier(BarrierPayoff<T>),
    /// Lookback option (Fixed/Floating, Call/Put).
    Lookback(LookbackPayoff<T>),
}

impl<T: Float + Send + Sync> PayoffKind<T> {
    // --- Vanilla constructors ---

    /// Creates a European call option.
    #[inline]
    pub fn european_call(strike: T, epsilon: T) -> Self {
        Self::Vanilla(VanillaPayoff::call(strike, epsilon))
    }

    /// Creates a European put option.
    #[inline]
    pub fn european_put(strike: T, epsilon: T) -> Self {
        Self::Vanilla(VanillaPayoff::put(strike, epsilon))
    }

    /// Creates a digital call option.
    #[inline]
    pub fn digital_call(strike: T, epsilon: T) -> Self {
        Self::Digital(DigitalPayoff::call(strike, epsilon))
    }

    /// Creates a digital put option.
    #[inline]
    pub fn digital_put(strike: T, epsilon: T) -> Self {
        Self::Digital(DigitalPayoff::put(strike, epsilon))
    }

    // --- Asian constructors ---

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

    // --- Barrier constructors ---

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

    // --- Lookback constructors ---

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

    // --- Classification helpers ---

    /// Returns true if this is a vanilla European option.
    #[inline]
    pub fn is_vanilla(&self) -> bool { matches!(self, PayoffKind::Vanilla(_)) }

    /// Returns true if this is a digital option.
    #[inline]
    pub fn is_digital(&self) -> bool { matches!(self, PayoffKind::Digital(_)) }

    /// Returns true if this is an Asian option.
    #[inline]
    pub fn is_asian(&self) -> bool {
        matches!(
            self,
            PayoffKind::AsianArithmetic(_) | PayoffKind::AsianGeometric(_)
        )
    }

    /// Returns true if this is a barrier option.
    #[inline]
    pub fn is_barrier(&self) -> bool { matches!(self, PayoffKind::Barrier(_)) }

    /// Returns true if this is a lookback option.
    #[inline]
    pub fn is_lookback(&self) -> bool { matches!(self, PayoffKind::Lookback(_)) }

    /// Returns true if this payoff is path-dependent.
    #[inline]
    pub fn is_path_dependent(&self) -> bool {
        matches!(
            self,
            PayoffKind::AsianArithmetic(_)
                | PayoffKind::AsianGeometric(_)
                | PayoffKind::Barrier(_)
                | PayoffKind::Lookback(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // --- ObservationType tests ---

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

    // --- PayoffKind vanilla tests ---

    #[test]
    fn test_european_call() {
        let payoff = PayoffKind::european_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_european_put() {
        let payoff = PayoffKind::european_put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_digital_call() {
        let payoff = PayoffKind::digital_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_digital_put() {
        let payoff = PayoffKind::digital_put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(90.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 1.0, epsilon = 0.01);
    }

    // --- PayoffKind path-dependent tests ---

    #[test]
    fn test_enum_asian_arithmetic_call() {
        let payoff = PayoffKind::asian_arithmetic_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);
        observer.set_terminal(120.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_up_in_call_hit() {
        let payoff = PayoffKind::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(115.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_fixed_call() {
        let payoff = PayoffKind::lookback_fixed_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(120.0);
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    // --- Classification tests ---

    #[test]
    fn test_is_vanilla() {
        assert!(PayoffKind::european_call(100.0_f64, 1e-6).is_vanilla());
        assert!(!PayoffKind::digital_call(100.0_f64, 1e-6).is_vanilla());
        assert!(!PayoffKind::asian_arithmetic_call(100.0_f64, 1e-6).is_vanilla());
    }

    #[test]
    fn test_is_digital() {
        assert!(PayoffKind::digital_call(100.0_f64, 1e-6).is_digital());
        assert!(!PayoffKind::european_call(100.0_f64, 1e-6).is_digital());
    }

    #[test]
    fn test_is_path_dependent() {
        assert!(!PayoffKind::european_call(100.0_f64, 1e-6).is_path_dependent());
        assert!(!PayoffKind::digital_call(100.0_f64, 1e-6).is_path_dependent());
        assert!(PayoffKind::asian_arithmetic_call(100.0_f64, 1e-6).is_path_dependent());
        assert!(PayoffKind::barrier_up_in_call(100.0_f64, 110.0, 1e-6).is_path_dependent());
        assert!(PayoffKind::lookback_fixed_call(100.0_f64, 1e-6).is_path_dependent());
    }

    #[test]
    fn test_is_asian() {
        assert!(PayoffKind::asian_arithmetic_call(100.0_f64, 1e-6).is_asian());
        assert!(PayoffKind::asian_geometric_call(100.0_f64, 1e-6).is_asian());
        assert!(!PayoffKind::barrier_up_in_call(100.0_f64, 110.0, 1e-6).is_asian());
    }

    #[test]
    fn test_is_barrier() {
        assert!(PayoffKind::barrier_up_in_call(100.0_f64, 110.0, 1e-6).is_barrier());
        assert!(!PayoffKind::lookback_fixed_call(100.0_f64, 1e-6).is_barrier());
    }

    #[test]
    fn test_is_lookback() {
        assert!(PayoffKind::lookback_fixed_call(100.0_f64, 1e-6).is_lookback());
        assert!(!PayoffKind::barrier_up_in_call(100.0_f64, 110.0, 1e-6).is_lookback());
    }

    // --- Smoothing epsilon tests ---

    #[test]
    fn test_enum_smoothing_epsilon() {
        let epsilon = 1e-4_f64;

        let vanilla = PayoffKind::european_call(100.0, epsilon);
        let digital = PayoffKind::digital_call(100.0, epsilon);
        let asian = PayoffKind::asian_arithmetic_call(100.0, epsilon);
        let barrier = PayoffKind::barrier_up_in_call(100.0, 110.0, epsilon);
        let lookback = PayoffKind::lookback_fixed_call(100.0, epsilon);

        assert_eq!(vanilla.smoothing_epsilon(), epsilon);
        assert_eq!(digital.smoothing_epsilon(), epsilon);
        assert_eq!(asian.smoothing_epsilon(), epsilon);
        assert_eq!(barrier.smoothing_epsilon(), epsilon);
        assert_eq!(lookback.smoothing_epsilon(), epsilon);
    }

    // --- Clone / Copy tests ---

    #[test]
    fn test_enum_clone() {
        let payoff = PayoffKind::european_call(100.0_f64, 1e-6);
        let cloned = payoff.clone();

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.set_terminal(110.0);

        let result1 = payoff.compute(&[], &observer);
        let result2 = cloned.compute(&[], &observer);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_enum_copy() {
        let payoff = PayoffKind::lookback_floating_call(1e-6_f64);
        let copied: PayoffKind<f64> = payoff;

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(90.0);
        observer.set_terminal(110.0);

        let result1 = payoff.compute(&[], &observer);
        let result2 = copied.compute(&[], &observer);
        assert_eq!(result1, result2);
    }

    // --- McPayoff trait object test ---

    #[test]
    fn test_mc_payoff_trait_exists() {
        fn assert_trait<T, C: McPayoff<T>>()
        where
            T: Float,
        {
        }

        assert_trait::<f64, VanillaPayoff<f64>>();
        assert_trait::<f64, DigitalPayoff<f64>>();
        assert_trait::<f64, AsianArithmeticPayoff<f64>>();
        assert_trait::<f64, BarrierPayoff<f64>>();
        assert_trait::<f64, LookbackPayoff<f64>>();
    }
}

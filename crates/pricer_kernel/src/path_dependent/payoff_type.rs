//! PathPayoffType enum for static dispatch of path-dependent payoffs.
//!
//! This module provides a unified enum for all path-dependent option types,
//! enabling static dispatch without trait objects for Enzyme AD compatibility.

use super::{
    AsianArithmeticPayoff, AsianGeometricPayoff, AsianParams, BarrierPayoff, LookbackPayoff,
    ObservationType, PathDependentPayoff, PathObserver,
};
use num_traits::Float;

/// Enum encompassing all path-dependent option payoff types.
///
/// This enum enables static dispatch for payoff computation, which is crucial
/// for Enzyme AD compatibility. Each variant wraps the corresponding payoff struct.
///
/// # Enzyme AD Compatibility
///
/// By using an enum instead of trait objects (`Box<dyn PathDependentPayoff>`),
/// we enable LLVM-level optimization and Enzyme autodifferentiation.
///
/// # Example
///
/// ```ignore
/// use pricer_kernel::path_dependent::{PathPayoffType, PathObserver, AsianParams};
///
/// let payoff = PathPayoffType::asian_arithmetic_call(100.0, 1e-6);
/// let mut observer = PathObserver::new();
/// observer.observe(100.0);
/// observer.observe(110.0);
/// observer.observe(105.0);
/// observer.set_terminal(105.0);
///
/// let result = payoff.compute(&[], &observer);
/// ```
#[derive(Clone, Copy, Debug)]
pub enum PathPayoffType<T: Float> {
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
    // ========================================================================
    // Asian Option Constructors
    // ========================================================================

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

    // ========================================================================
    // Barrier Option Constructors
    // ========================================================================

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

    // ========================================================================
    // Lookback Option Constructors
    // ========================================================================

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

    // ========================================================================
    // Static Dispatch Methods
    // ========================================================================

    /// Computes the payoff using static dispatch.
    ///
    /// This method dispatches to the appropriate payoff computation based on
    /// the enum variant, enabling Enzyme AD to trace through the computation.
    #[inline]
    pub fn compute(&self, path: &[T], observer: &PathObserver<T>) -> T {
        match self {
            PathPayoffType::AsianArithmetic(payoff) => payoff.compute(path, observer),
            PathPayoffType::AsianGeometric(payoff) => payoff.compute(path, observer),
            PathPayoffType::Barrier(payoff) => payoff.compute(path, observer),
            PathPayoffType::Lookback(payoff) => payoff.compute(path, observer),
        }
    }

    /// Returns the observation types required for this payoff.
    #[inline]
    pub fn required_observations(&self) -> ObservationType {
        match self {
            PathPayoffType::AsianArithmetic(payoff) => payoff.required_observations(),
            PathPayoffType::AsianGeometric(payoff) => payoff.required_observations(),
            PathPayoffType::Barrier(payoff) => payoff.required_observations(),
            PathPayoffType::Lookback(payoff) => payoff.required_observations(),
        }
    }

    /// Returns the smoothing epsilon used for this payoff.
    #[inline]
    pub fn smoothing_epsilon(&self) -> T {
        match self {
            PathPayoffType::AsianArithmetic(payoff) => payoff.smoothing_epsilon(),
            PathPayoffType::AsianGeometric(payoff) => payoff.smoothing_epsilon(),
            PathPayoffType::Barrier(payoff) => payoff.smoothing_epsilon(),
            PathPayoffType::Lookback(payoff) => payoff.smoothing_epsilon(),
        }
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
    pub fn is_barrier(&self) -> bool {
        matches!(self, PathPayoffType::Barrier(_))
    }

    /// Returns true if this is a lookback option.
    #[inline]
    pub fn is_lookback(&self) -> bool {
        matches!(self, PathPayoffType::Lookback(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // Asian Option Tests via PathPayoffType
    // ========================================================================

    #[test]
    fn test_enum_asian_arithmetic_call() {
        let payoff = PathPayoffType::asian_arithmetic_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(110.0);
        observer.observe(120.0);
        observer.set_terminal(120.0);

        let result = payoff.compute(&[], &observer);
        // Average = 110, Strike = 100, Payoff ≈ 10
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
        // Average ≈ 101.67, Strike = 110, Payoff ≈ 8.33
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
        // Geometric average = 100, Strike = 95, Payoff ≈ 5
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
        // Geometric average = 100, Strike = 105, Payoff ≈ 5
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    // ========================================================================
    // Barrier Option Tests via PathPayoffType
    // ========================================================================

    #[test]
    fn test_enum_barrier_up_in_call_hit() {
        let payoff = PathPayoffType::barrier_up_in_call(100.0_f64, 110.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(115.0); // Barrier hit
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        // Barrier hit, Terminal = 110, Strike = 100, Payoff ≈ 10
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
        // Barrier not hit, Terminal = 105, Strike = 100, Payoff ≈ 5
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_barrier_down_in_put_hit() {
        let payoff = PathPayoffType::barrier_down_in_put(100.0_f64, 90.0, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(85.0); // Barrier hit
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        // Barrier hit, Strike = 100, Terminal = 92, Put Payoff ≈ 8
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
        // Barrier not hit, Strike = 100, Terminal = 95, Put Payoff ≈ 5
        assert_relative_eq!(result, 5.0, epsilon = 0.1);
    }

    // ========================================================================
    // Lookback Option Tests via PathPayoffType
    // ========================================================================

    #[test]
    fn test_enum_lookback_fixed_call() {
        let payoff = PathPayoffType::lookback_fixed_call(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(120.0); // Max
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        // max(S_max - K, 0) = max(120 - 100, 0) = 20
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_fixed_put() {
        let payoff = PathPayoffType::lookback_fixed_put(100.0_f64, 1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(90.0);
        observer.observe(85.0); // Min
        observer.observe(92.0);
        observer.set_terminal(92.0);

        let result = payoff.compute(&[], &observer);
        // max(K - S_min, 0) = max(100 - 85, 0) = 15
        assert_relative_eq!(result, 15.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_floating_call() {
        let payoff = PathPayoffType::lookback_floating_call(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(90.0); // Min
        observer.observe(110.0);
        observer.set_terminal(110.0);

        let result = payoff.compute(&[], &observer);
        // S_T - S_min = 110 - 90 = 20
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    #[test]
    fn test_enum_lookback_floating_put() {
        let payoff = PathPayoffType::lookback_floating_put(1e-6);
        let mut observer: PathObserver<f64> = PathObserver::new();

        observer.observe(100.0);
        observer.observe(120.0); // Max
        observer.observe(110.0);
        observer.set_terminal(100.0);

        let result = payoff.compute(&[], &observer);
        // S_max - S_T = 120 - 100 = 20
        assert_relative_eq!(result, 20.0, epsilon = 0.1);
    }

    // ========================================================================
    // Required Observations Tests
    // ========================================================================

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

    // ========================================================================
    // Type Classification Tests
    // ========================================================================

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

    // ========================================================================
    // Smoothing Epsilon Tests
    // ========================================================================

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

    // ========================================================================
    // Clone/Copy Tests
    // ========================================================================

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
        let copied: PathPayoffType<f64> = payoff; // Copy

        let mut observer: PathObserver<f64> = PathObserver::new();
        observer.observe(100.0);
        observer.observe(90.0);
        observer.set_terminal(110.0);

        let result1 = payoff.compute(&[], &observer);
        let result2 = copied.compute(&[], &observer);

        assert_eq!(result1, result2);
    }
}

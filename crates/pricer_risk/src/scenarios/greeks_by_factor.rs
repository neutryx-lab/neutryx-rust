//! Greeks results aggregated by risk factor.
//!
//! This module provides [`GreeksResultByFactor`], a container for holding
//! Greeks calculations organised by their underlying risk factors.
//!
//! # Requirements
//!
//! - Requirement 1.1, 1.2: First and second-order Greeks by risk factor.
//! - Requirement 1.3: Risk factor identification for aggregation.
//! - Requirement 1.5: AD compatibility with `GreeksResult<T>`.

use super::RiskFactorId;
use pricer_core::traits::Float;
use pricer_pricing::greeks::{GreeksMode, GreeksResult};
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Greeks calculation results organised by risk factor.
///
/// Stores [`GreeksResult`] values keyed by [`RiskFactorId`], enabling
/// aggregation and analysis of sensitivities across different market
/// risk drivers.
///
/// # Type Parameters
///
/// * `T` - A floating-point type implementing `Float`. Enables AD compatibility.
///
/// # Examples
///
/// ```rust
/// use pricer_risk::scenarios::{GreeksResultByFactor, RiskFactorId};
/// use pricer_pricing::greeks::{GreeksMode, GreeksResult};
///
/// // Create results for different risk factors
/// let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
///
/// results.insert(
///     RiskFactorId::underlying("SPX"),
///     GreeksResult::new(100.0, 0.1).with_delta(0.5).with_gamma(0.02)
/// );
///
/// results.insert(
///     RiskFactorId::curve("USD-OIS"),
///     GreeksResult::new(0.0, 0.0).with_rho(15.0)
/// );
///
/// // Aggregate to total Greeks
/// let total = results.total();
/// assert!(total.delta.is_some());
/// ```
///
/// # Requirements
///
/// - Requirement 1.1, 1.2, 1.3, 1.5
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GreeksResultByFactor<T: Float> {
    /// Greeks results keyed by risk factor.
    by_factor: HashMap<RiskFactorId, GreeksResult<T>>,

    /// Computation mode used (AAD or Bump).
    mode: GreeksMode,

    /// Computation time in nanoseconds.
    computation_time_ns: u64,
}

impl<T: Float> GreeksResultByFactor<T> {
    /// Creates a new empty container with the specified computation mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The Greeks computation mode (AAD, Bump, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_risk::scenarios::GreeksResultByFactor;
    /// use pricer_pricing::greeks::GreeksMode;
    ///
    /// let results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
    /// assert!(results.is_empty());
    /// ```
    #[inline]
    pub fn new(mode: GreeksMode) -> Self {
        Self {
            by_factor: HashMap::new(),
            mode,
            computation_time_ns: 0,
        }
    }

    /// Creates a new container with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `mode` - The Greeks computation mode.
    /// * `capacity` - Initial capacity for the number of risk factors.
    #[inline]
    pub fn with_capacity(mode: GreeksMode, capacity: usize) -> Self {
        Self {
            by_factor: HashMap::with_capacity(capacity),
            mode,
            computation_time_ns: 0,
        }
    }

    /// Creates a container from an existing HashMap.
    ///
    /// # Arguments
    ///
    /// * `by_factor` - HashMap of risk factor to Greeks results.
    /// * `mode` - The computation mode used.
    /// * `computation_time_ns` - Time taken for computation in nanoseconds.
    #[inline]
    pub fn from_map(
        by_factor: HashMap<RiskFactorId, GreeksResult<T>>,
        mode: GreeksMode,
        computation_time_ns: u64,
    ) -> Self {
        Self {
            by_factor,
            mode,
            computation_time_ns,
        }
    }

    /// Returns the computation mode.
    #[inline]
    pub fn mode(&self) -> GreeksMode {
        self.mode
    }

    /// Returns the computation time in nanoseconds.
    #[inline]
    pub fn computation_time_ns(&self) -> u64 {
        self.computation_time_ns
    }

    /// Returns the computation time in milliseconds.
    #[inline]
    pub fn computation_time_ms(&self) -> f64 {
        self.computation_time_ns as f64 / 1_000_000.0
    }

    /// Sets the computation time in nanoseconds.
    #[inline]
    pub fn set_computation_time_ns(&mut self, time_ns: u64) {
        self.computation_time_ns = time_ns;
    }

    /// Inserts a Greeks result for a risk factor.
    ///
    /// # Arguments
    ///
    /// * `factor` - The risk factor identifier.
    /// * `result` - The Greeks calculation result.
    ///
    /// # Returns
    ///
    /// The previous value if the factor was already present.
    #[inline]
    pub fn insert(
        &mut self,
        factor: RiskFactorId,
        result: GreeksResult<T>,
    ) -> Option<GreeksResult<T>> {
        self.by_factor.insert(factor, result)
    }

    /// Gets the Greeks result for a specific risk factor.
    ///
    /// # Arguments
    ///
    /// * `factor` - The risk factor to look up.
    ///
    /// # Returns
    ///
    /// Reference to the Greeks result if found.
    #[inline]
    pub fn get(&self, factor: &RiskFactorId) -> Option<&GreeksResult<T>> {
        self.by_factor.get(factor)
    }

    /// Gets a mutable reference to Greeks result for a specific risk factor.
    #[inline]
    pub fn get_mut(&mut self, factor: &RiskFactorId) -> Option<&mut GreeksResult<T>> {
        self.by_factor.get_mut(factor)
    }

    /// Returns the number of risk factors.
    #[inline]
    pub fn len(&self) -> usize {
        self.by_factor.len()
    }

    /// Returns true if no risk factors are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.by_factor.is_empty()
    }

    /// Returns an iterator over risk factor and Greeks result pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&RiskFactorId, &GreeksResult<T>)> {
        self.by_factor.iter()
    }

    /// Returns an iterator over risk factors.
    #[inline]
    pub fn factors(&self) -> impl Iterator<Item = &RiskFactorId> {
        self.by_factor.keys()
    }

    /// Returns an iterator over Greeks results.
    #[inline]
    pub fn results(&self) -> impl Iterator<Item = &GreeksResult<T>> {
        self.by_factor.values()
    }

    /// Returns the underlying HashMap.
    #[inline]
    pub fn as_map(&self) -> &HashMap<RiskFactorId, GreeksResult<T>> {
        &self.by_factor
    }

    /// Consumes self and returns the underlying HashMap.
    #[inline]
    pub fn into_map(self) -> HashMap<RiskFactorId, GreeksResult<T>> {
        self.by_factor
    }

    /// Computes total Greeks by summing across all risk factors.
    ///
    /// Aggregates price, delta, gamma, vega, theta, rho, vanna, and volga
    /// by summing the values from all risk factors. Standard error is
    /// combined using root-sum-of-squares.
    ///
    /// # Returns
    ///
    /// A single [`GreeksResult`] representing the portfolio total.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_risk::scenarios::{GreeksResultByFactor, RiskFactorId};
    /// use pricer_pricing::greeks::{GreeksMode, GreeksResult};
    ///
    /// let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
    ///
    /// results.insert(
    ///     RiskFactorId::underlying("SPX"),
    ///     GreeksResult::new(100.0, 0.1).with_delta(0.5)
    /// );
    /// results.insert(
    ///     RiskFactorId::underlying("AAPL"),
    ///     GreeksResult::new(50.0, 0.05).with_delta(0.3)
    /// );
    ///
    /// let total = results.total();
    /// assert!((total.price - 150.0).abs() < 1e-10);
    /// assert!((total.delta.unwrap() - 0.8).abs() < 1e-10);
    /// ```
    pub fn total(&self) -> GreeksResult<T> {
        if self.by_factor.is_empty() {
            return GreeksResult::default();
        }

        let mut total_price = T::zero();
        let mut total_std_error_sq = T::zero();
        let mut total_delta = T::zero();
        let mut total_gamma = T::zero();
        let mut total_vega = T::zero();
        let mut total_theta = T::zero();
        let mut total_rho = T::zero();
        let mut total_vanna = T::zero();
        let mut total_volga = T::zero();

        let mut has_delta = false;
        let mut has_gamma = false;
        let mut has_vega = false;
        let mut has_theta = false;
        let mut has_rho = false;
        let mut has_vanna = false;
        let mut has_volga = false;

        for result in self.by_factor.values() {
            total_price = total_price + result.price;
            total_std_error_sq = total_std_error_sq + result.std_error * result.std_error;

            if let Some(delta) = result.delta {
                total_delta = total_delta + delta;
                has_delta = true;
            }
            if let Some(gamma) = result.gamma {
                total_gamma = total_gamma + gamma;
                has_gamma = true;
            }
            if let Some(vega) = result.vega {
                total_vega = total_vega + vega;
                has_vega = true;
            }
            if let Some(theta) = result.theta {
                total_theta = total_theta + theta;
                has_theta = true;
            }
            if let Some(rho) = result.rho {
                total_rho = total_rho + rho;
                has_rho = true;
            }
            if let Some(vanna) = result.vanna {
                total_vanna = total_vanna + vanna;
                has_vanna = true;
            }
            if let Some(volga) = result.volga {
                total_volga = total_volga + volga;
                has_volga = true;
            }
        }

        let total_std_error = total_std_error_sq.sqrt();

        let mut result = GreeksResult::new(total_price, total_std_error);

        if has_delta {
            result = result.with_delta(total_delta);
        }
        if has_gamma {
            result = result.with_gamma(total_gamma);
        }
        if has_vega {
            result = result.with_vega(total_vega);
        }
        if has_theta {
            result = result.with_theta(total_theta);
        }
        if has_rho {
            result = result.with_rho(total_rho);
        }
        if has_vanna {
            result = result.with_vanna(total_vanna);
        }
        if has_volga {
            result = result.with_volga(total_volga);
        }

        result
    }

    /// Filters results to only include underlying risk factors.
    pub fn filter_underlying(&self) -> Self {
        Self {
            by_factor: self
                .by_factor
                .iter()
                .filter(|(k, _)| k.is_underlying())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            mode: self.mode,
            computation_time_ns: self.computation_time_ns,
        }
    }

    /// Filters results to only include curve risk factors.
    pub fn filter_curves(&self) -> Self {
        Self {
            by_factor: self
                .by_factor
                .iter()
                .filter(|(k, _)| k.is_curve())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            mode: self.mode,
            computation_time_ns: self.computation_time_ns,
        }
    }

    /// Filters results to only include volatility surface risk factors.
    pub fn filter_vol_surfaces(&self) -> Self {
        Self {
            by_factor: self
                .by_factor
                .iter()
                .filter(|(k, _)| k.is_vol_surface())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            mode: self.mode,
            computation_time_ns: self.computation_time_ns,
        }
    }
}

impl<T: Float> Default for GreeksResultByFactor<T> {
    fn default() -> Self {
        Self::new(GreeksMode::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 1.2: GreeksResultByFactor tests (TDD)
    // ================================================================

    #[test]
    fn test_greeks_result_by_factor_new() {
        let results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        assert!(results.is_empty());
        assert_eq!(results.len(), 0);
        assert_eq!(results.mode(), GreeksMode::BumpRevalue);
        assert_eq!(results.computation_time_ns(), 0);
    }

    #[test]
    fn test_greeks_result_by_factor_with_capacity() {
        let results = GreeksResultByFactor::<f64>::with_capacity(GreeksMode::NumDual, 10);

        assert!(results.is_empty());
        assert_eq!(results.mode(), GreeksMode::NumDual);
    }

    #[test]
    fn test_greeks_result_by_factor_insert_and_get() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        let factor = RiskFactorId::underlying("SPX");
        let greeks = GreeksResult::new(100.0, 0.1).with_delta(0.5);

        assert!(results.insert(factor.clone(), greeks).is_none());
        assert_eq!(results.len(), 1);

        let retrieved = results.get(&factor).unwrap();
        assert!((retrieved.price - 100.0).abs() < 1e-10);
        assert!((retrieved.delta.unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_insert_replace() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        let factor = RiskFactorId::underlying("SPX");
        results.insert(factor.clone(), GreeksResult::new(100.0, 0.1));

        let old = results.insert(factor.clone(), GreeksResult::new(200.0, 0.2));

        assert!(old.is_some());
        assert!((old.unwrap().price - 100.0).abs() < 1e-10);
        assert!((results.get(&factor).unwrap().price - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_get_nonexistent() {
        let results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
        let factor = RiskFactorId::underlying("AAPL");

        assert!(results.get(&factor).is_none());
    }

    #[test]
    fn test_greeks_result_by_factor_total_empty() {
        let results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
        let total = results.total();

        assert!((total.price - 0.0).abs() < 1e-10);
        assert!((total.std_error - 0.0).abs() < 1e-10);
        assert!(total.delta.is_none());
    }

    #[test]
    fn test_greeks_result_by_factor_total_single() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1)
                .with_delta(0.5)
                .with_gamma(0.02),
        );

        let total = results.total();

        assert!((total.price - 100.0).abs() < 1e-10);
        assert!((total.std_error - 0.1).abs() < 1e-10);
        assert!((total.delta.unwrap() - 0.5).abs() < 1e-10);
        assert!((total.gamma.unwrap() - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_total_multiple() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1).with_delta(0.5),
        );
        results.insert(
            RiskFactorId::underlying("AAPL"),
            GreeksResult::new(50.0, 0.05).with_delta(0.3),
        );
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(0.0, 0.0).with_rho(15.0),
        );

        let total = results.total();

        // Price: 100 + 50 + 0 = 150
        assert!((total.price - 150.0).abs() < 1e-10);

        // Std error: sqrt(0.1^2 + 0.05^2 + 0^2) = sqrt(0.01 + 0.0025) = sqrt(0.0125)
        let expected_std_error = (0.01_f64 + 0.0025).sqrt();
        assert!((total.std_error - expected_std_error).abs() < 1e-10);

        // Delta: 0.5 + 0.3 = 0.8
        assert!((total.delta.unwrap() - 0.8).abs() < 1e-10);

        // Rho: 15.0
        assert!((total.rho.unwrap() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_total_second_order() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.0)
                .with_gamma(0.02)
                .with_vanna(0.01)
                .with_volga(0.005),
        );
        results.insert(
            RiskFactorId::vol_surface("SPX-Vol"),
            GreeksResult::new(0.0, 0.0)
                .with_vega(25.0)
                .with_volga(0.003),
        );

        let total = results.total();

        assert!((total.gamma.unwrap() - 0.02).abs() < 1e-10);
        assert!((total.vanna.unwrap() - 0.01).abs() < 1e-10);
        assert!((total.volga.unwrap() - 0.008).abs() < 1e-10);
        assert!((total.vega.unwrap() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_computation_time() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.set_computation_time_ns(1_500_000);

        assert_eq!(results.computation_time_ns(), 1_500_000);
        assert!((results.computation_time_ms() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_by_factor_from_map() {
        let mut map = HashMap::new();
        map.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );

        let results = GreeksResultByFactor::from_map(map, GreeksMode::NumDual, 2_000_000);

        assert_eq!(results.len(), 1);
        assert_eq!(results.mode(), GreeksMode::NumDual);
        assert_eq!(results.computation_time_ns(), 2_000_000);
    }

    #[test]
    fn test_greeks_result_by_factor_iter() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(50.0, 0.05),
        );

        let count = results.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_greeks_result_by_factor_factors() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(50.0, 0.05),
        );

        let factors: Vec<_> = results.factors().collect();
        assert_eq!(factors.len(), 2);
    }

    #[test]
    fn test_greeks_result_by_factor_filter_underlying() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.insert(
            RiskFactorId::underlying("AAPL"),
            GreeksResult::new(50.0, 0.05),
        );
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(25.0, 0.02),
        );
        results.insert(
            RiskFactorId::vol_surface("SPX-Vol"),
            GreeksResult::new(10.0, 0.01),
        );

        let filtered = results.filter_underlying();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.get(&RiskFactorId::underlying("SPX")).is_some());
        assert!(filtered.get(&RiskFactorId::underlying("AAPL")).is_some());
        assert!(filtered.get(&RiskFactorId::curve("USD-OIS")).is_none());
    }

    #[test]
    fn test_greeks_result_by_factor_filter_curves() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(25.0, 0.02),
        );
        results.insert(
            RiskFactorId::curve("EUR-EURIBOR"),
            GreeksResult::new(15.0, 0.01),
        );

        let filtered = results.filter_curves();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.get(&RiskFactorId::curve("USD-OIS")).is_some());
        assert!(filtered.get(&RiskFactorId::curve("EUR-EURIBOR")).is_some());
    }

    #[test]
    fn test_greeks_result_by_factor_filter_vol_surfaces() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.insert(
            RiskFactorId::vol_surface("SPX-Vol"),
            GreeksResult::new(10.0, 0.01),
        );
        results.insert(
            RiskFactorId::vol_surface("USDJPY-Vol"),
            GreeksResult::new(5.0, 0.005),
        );

        let filtered = results.filter_vol_surfaces();

        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .get(&RiskFactorId::vol_surface("SPX-Vol"))
            .is_some());
        assert!(filtered
            .get(&RiskFactorId::vol_surface("USDJPY-Vol"))
            .is_some());
    }

    #[test]
    fn test_greeks_result_by_factor_into_map() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );

        let map = results.into_map();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&RiskFactorId::underlying("SPX")));
    }

    #[test]
    fn test_greeks_result_by_factor_default() {
        let results = GreeksResultByFactor::<f64>::default();

        assert!(results.is_empty());
        assert_eq!(results.mode(), GreeksMode::default());
    }

    #[test]
    fn test_greeks_result_by_factor_clone() {
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1),
        );
        results.set_computation_time_ns(1_000_000);

        let cloned = results.clone();

        assert_eq!(cloned.len(), results.len());
        assert_eq!(cloned.mode(), results.mode());
        assert_eq!(cloned.computation_time_ns(), results.computation_time_ns());
    }

    #[test]
    fn test_greeks_result_by_factor_ad_compatibility() {
        // Test that the struct works with f32 as well (AD compatibility)
        let mut results = GreeksResultByFactor::<f32>::new(GreeksMode::BumpRevalue);

        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0_f32, 0.1_f32).with_delta(0.5_f32),
        );

        let total = results.total();
        assert!((total.price - 100.0_f32).abs() < 1e-6);
        assert!((total.delta.unwrap() - 0.5_f32).abs() < 1e-6);
    }

    #[test]
    fn test_greeks_result_by_factor_partial_greeks() {
        // Test aggregation when some factors have Greeks and others don't
        let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);

        // Factor 1: has delta and gamma
        results.insert(
            RiskFactorId::underlying("SPX"),
            GreeksResult::new(100.0, 0.1)
                .with_delta(0.5)
                .with_gamma(0.02),
        );

        // Factor 2: only has rho
        results.insert(
            RiskFactorId::curve("USD-OIS"),
            GreeksResult::new(0.0, 0.0).with_rho(15.0),
        );

        // Factor 3: only has vega
        results.insert(
            RiskFactorId::vol_surface("SPX-Vol"),
            GreeksResult::new(0.0, 0.0).with_vega(25.0),
        );

        let total = results.total();

        // Only delta from factor 1
        assert!((total.delta.unwrap() - 0.5).abs() < 1e-10);
        // Only gamma from factor 1
        assert!((total.gamma.unwrap() - 0.02).abs() < 1e-10);
        // Only rho from factor 2
        assert!((total.rho.unwrap() - 15.0).abs() < 1e-10);
        // Only vega from factor 3
        assert!((total.vega.unwrap() - 25.0).abs() < 1e-10);
        // Theta not present in any factor
        assert!(total.theta.is_none());
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_greeks_result_by_factor_serialize() {
            let mut results = GreeksResultByFactor::<f64>::new(GreeksMode::BumpRevalue);
            results.insert(
                RiskFactorId::underlying("SPX"),
                GreeksResult::new(100.0, 0.1),
            );
            results.set_computation_time_ns(1_000_000);

            let json = serde_json::to_string(&results).unwrap();
            assert!(json.contains("SPX"));
            assert!(json.contains("100.0"));
        }
    }
}

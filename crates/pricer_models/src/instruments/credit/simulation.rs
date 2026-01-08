//! Credit default simulation module.
//!
//! This module provides Monte Carlo simulation utilities for credit events,
//! including default time sampling from hazard rate curves.
//!
//! # Default Time Sampling
//!
//! Default times are sampled using the inverse transform method:
//! 1. Draw a uniform random U ~ Uniform(0, 1)
//! 2. Solve for τ such that S(τ) = U (survival probability equals U)
//!
//! For a flat hazard rate λ:
//! ```text
//! τ = -ln(U) / λ
//! ```
//!
//! # Example
//!
//! ```
//! use pricer_models::instruments::credit::simulation::DefaultTimeSimulator;
//! use pricer_core::market_data::curves::FlatHazardRateCurve;
//!
//! let credit_curve = FlatHazardRateCurve::new(0.02_f64); // 2% hazard rate
//! let simulator = DefaultTimeSimulator::new(&credit_curve);
//!
//! // Sample a default time using a uniform random number
//! let default_time = simulator.sample_default_time(0.5_f64).unwrap();
//!
//! // For flat hazard rate: τ = -ln(U) / λ = -ln(0.5) / 0.02 ≈ 34.66 years
//! assert!(default_time > 30.0 && default_time < 40.0);
//! ```

use num_traits::Float;
use pricer_core::market_data::curves::CreditCurve;
use pricer_core::market_data::error::MarketDataError;

/// Default time simulator using a credit curve.
///
/// Samples default times using inverse transform sampling from
/// the survival probability function.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float`
/// * `C` - Credit curve implementing `CreditCurve<T>`
pub struct DefaultTimeSimulator<'a, T: Float, C: CreditCurve<T>> {
    /// Credit curve for survival probability calculations.
    credit_curve: &'a C,
    /// Maximum time horizon for default search (years).
    max_horizon: T,
    /// Phantom data for type parameter T.
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: Float, C: CreditCurve<T>> DefaultTimeSimulator<'a, T, C> {
    /// Create a new default time simulator.
    ///
    /// # Arguments
    ///
    /// * `credit_curve` - Credit curve for survival probabilities
    pub fn new(credit_curve: &'a C) -> Self {
        Self::with_horizon(credit_curve, T::from(100.0).unwrap())
    }

    /// Create a new default time simulator with custom horizon.
    ///
    /// # Arguments
    ///
    /// * `credit_curve` - Credit curve for survival probabilities
    /// * `max_horizon` - Maximum time to search for default (years)
    pub fn with_horizon(credit_curve: &'a C, max_horizon: T) -> Self {
        Self {
            credit_curve,
            max_horizon,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sample a default time using inverse transform sampling.
    ///
    /// Given a uniform random number U ∈ (0, 1), returns the default time τ
    /// such that S(τ) = U, where S is the survival probability.
    ///
    /// # Arguments
    ///
    /// * `uniform` - Uniform random number in (0, 1)
    ///
    /// # Returns
    ///
    /// * `Ok(τ)` - Sampled default time in years
    /// * `Err(MarketDataError)` - If sampling fails
    ///
    /// # Mathematical Background
    ///
    /// For a survival probability S(t), the CDF of default time is:
    /// ```text
    /// F(t) = P(τ ≤ t) = 1 - S(t)
    /// ```
    ///
    /// To sample using inverse transform:
    /// ```text
    /// U = 1 - S(τ)  →  S(τ) = 1 - U  →  τ = S⁻¹(1 - U)
    /// ```
    ///
    /// For flat hazard rate λ:
    /// ```text
    /// S(t) = exp(-λt)  →  τ = -ln(U) / λ
    /// ```
    ///
    /// Note: We use U directly as the survival probability for numerical stability:
    /// ```text
    /// S(τ) = U  →  τ = S⁻¹(U) = -ln(U) / λ  (for flat curve)
    /// ```
    pub fn sample_default_time(&self, uniform: T) -> Result<T, MarketDataError> {
        if uniform <= T::zero() || uniform >= T::one() {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!(
                    "Uniform random must be in (0, 1), got {}",
                    uniform.to_f64().unwrap_or(0.0)
                ),
            });
        }

        // Use bisection to find τ such that S(τ) = uniform
        // This works for any credit curve shape
        self.find_default_time_bisection(uniform)
    }

    /// Find default time using bisection method.
    ///
    /// Finds τ such that S(τ) = target_survival using bisection search.
    fn find_default_time_bisection(&self, target_survival: T) -> Result<T, MarketDataError> {
        let tol = T::from(1e-6).unwrap();
        let max_iter = 100;

        let mut lo = T::zero();
        let mut hi = self.max_horizon;

        // Check if default occurs before max horizon
        let survival_at_horizon = self.credit_curve.survival_probability(hi)?;
        if target_survival <= survival_at_horizon {
            // Default occurs after max horizon
            return Ok(hi);
        }

        for _ in 0..max_iter {
            let mid = (lo + hi) / (T::one() + T::one());
            let survival_at_mid = self.credit_curve.survival_probability(mid)?;

            if (survival_at_mid - target_survival).abs() < tol {
                return Ok(mid);
            }

            if survival_at_mid > target_survival {
                // S(mid) > target means we need larger τ (lower survival)
                lo = mid;
            } else {
                // S(mid) < target means we need smaller τ (higher survival)
                hi = mid;
            }

            if (hi - lo) < tol {
                return Ok(mid);
            }
        }

        Ok((lo + hi) / (T::one() + T::one()))
    }

    /// Check if default occurs within a given time horizon.
    ///
    /// # Arguments
    ///
    /// * `uniform` - Uniform random number in (0, 1)
    /// * `horizon` - Time horizon to check (years)
    ///
    /// # Returns
    ///
    /// `true` if default occurs before horizon, `false` otherwise
    pub fn defaults_within(
        &self,
        uniform: T,
        horizon: T,
    ) -> Result<bool, MarketDataError> {
        let survival_at_horizon = self.credit_curve.survival_probability(horizon)?;
        // Default occurs if U < 1 - S(T), which is equivalent to U > S(T) for survival
        // When using U as survival target: default if target_survival > S(T)
        Ok(uniform > survival_at_horizon)
    }

    /// Sample multiple default times.
    ///
    /// # Arguments
    ///
    /// * `uniforms` - Slice of uniform random numbers in (0, 1)
    ///
    /// # Returns
    ///
    /// Vector of sampled default times
    pub fn sample_default_times(
        &self,
        uniforms: &[T],
    ) -> Result<Vec<T>, MarketDataError> {
        uniforms.iter()
            .map(|&u| self.sample_default_time(u))
            .collect()
    }
}

/// Indicator for whether default has occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultStatus {
    /// Entity has not defaulted.
    Survived,
    /// Entity has defaulted.
    Defaulted,
}

/// Result of a credit simulation for a single path.
#[derive(Debug, Clone, Copy)]
pub struct CreditPathResult<T: Float> {
    /// Default time (None if no default within horizon).
    pub default_time: Option<T>,
    /// Default status at horizon.
    pub status: DefaultStatus,
    /// Path horizon used.
    pub horizon: T,
}

impl<T: Float> CreditPathResult<T> {
    /// Create a new path result for a default.
    pub fn defaulted(default_time: T, horizon: T) -> Self {
        Self {
            default_time: Some(default_time),
            status: DefaultStatus::Defaulted,
            horizon,
        }
    }

    /// Create a new path result for survival.
    pub fn survived(horizon: T) -> Self {
        Self {
            default_time: None,
            status: DefaultStatus::Survived,
            horizon,
        }
    }

    /// Check if default occurred.
    #[inline]
    pub fn is_defaulted(&self) -> bool {
        self.status == DefaultStatus::Defaulted
    }

    /// Check if entity survived.
    #[inline]
    pub fn is_survived(&self) -> bool {
        self.status == DefaultStatus::Survived
    }
}

/// Monte Carlo simulator for credit default events.
///
/// Simulates multiple paths of credit events and computes statistics.
pub struct CreditMonteCarloSimulator<'a, T: Float, C: CreditCurve<T>> {
    /// Default time simulator.
    simulator: DefaultTimeSimulator<'a, T, C>,
    /// Simulation horizon (years).
    horizon: T,
}

impl<'a, T: Float, C: CreditCurve<T>> CreditMonteCarloSimulator<'a, T, C> {
    /// Create a new credit Monte Carlo simulator.
    ///
    /// # Arguments
    ///
    /// * `credit_curve` - Credit curve for survival probabilities
    /// * `horizon` - Simulation horizon (years)
    pub fn new(credit_curve: &'a C, horizon: T) -> Self {
        Self {
            simulator: DefaultTimeSimulator::with_horizon(credit_curve, horizon),
            horizon,
        }
    }

    /// Simulate a single path.
    ///
    /// # Arguments
    ///
    /// * `uniform` - Uniform random number in (0, 1)
    ///
    /// # Returns
    ///
    /// Credit path result indicating default status and timing
    pub fn simulate_path(&self, uniform: T) -> Result<CreditPathResult<T>, MarketDataError> {
        let default_time = self.simulator.sample_default_time(uniform)?;

        if default_time < self.horizon {
            Ok(CreditPathResult::defaulted(default_time, self.horizon))
        } else {
            Ok(CreditPathResult::survived(self.horizon))
        }
    }

    /// Simulate multiple paths.
    ///
    /// # Arguments
    ///
    /// * `uniforms` - Slice of uniform random numbers in (0, 1)
    ///
    /// # Returns
    ///
    /// Vector of credit path results
    pub fn simulate_paths(
        &self,
        uniforms: &[T],
    ) -> Result<Vec<CreditPathResult<T>>, MarketDataError> {
        uniforms.iter()
            .map(|&u| self.simulate_path(u))
            .collect()
    }

    /// Compute the default probability via Monte Carlo.
    ///
    /// # Arguments
    ///
    /// * `uniforms` - Slice of uniform random numbers in (0, 1)
    ///
    /// # Returns
    ///
    /// Estimated default probability
    pub fn default_probability(
        &self,
        uniforms: &[T],
    ) -> Result<T, MarketDataError> {
        let paths = self.simulate_paths(uniforms)?;
        let n_defaults = paths.iter()
            .filter(|p| p.is_defaulted())
            .count();

        let n_paths = T::from(uniforms.len()).unwrap();
        let n_def = T::from(n_defaults).unwrap();

        Ok(n_def / n_paths)
    }

    /// Compute expected default time (conditional on default).
    ///
    /// # Arguments
    ///
    /// * `uniforms` - Slice of uniform random numbers in (0, 1)
    ///
    /// # Returns
    ///
    /// Expected default time given default occurs (None if no defaults)
    pub fn expected_default_time(
        &self,
        uniforms: &[T],
    ) -> Result<Option<T>, MarketDataError> {
        let paths = self.simulate_paths(uniforms)?;

        let default_times: Vec<T> = paths.iter()
            .filter_map(|p| p.default_time)
            .collect();

        if default_times.is_empty() {
            return Ok(None);
        }

        let sum: T = default_times.iter().copied().fold(T::zero(), |a, b| a + b);
        let n = T::from(default_times.len()).unwrap();

        Ok(Some(sum / n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::market_data::curves::FlatHazardRateCurve;

    // ========================================
    // DefaultTimeSimulator Tests
    // ========================================

    #[test]
    fn test_simulator_new() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let _simulator = DefaultTimeSimulator::new(&credit_curve);
    }

    #[test]
    fn test_sample_default_time_flat_curve() {
        let hazard_rate = 0.02_f64; // 2% hazard rate
        let credit_curve = FlatHazardRateCurve::new(hazard_rate);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        // For flat curve: τ = -ln(U) / λ
        let uniform = 0.5_f64;
        let expected_time = -uniform.ln() / hazard_rate;

        let sampled_time = simulator.sample_default_time(uniform).unwrap();

        assert!(
            (sampled_time - expected_time).abs() < 1e-4,
            "Expected {}, got {}",
            expected_time,
            sampled_time
        );
    }

    #[test]
    fn test_sample_default_time_extreme_uniform() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        // Very low uniform = very early default (high U = early default for survival target)
        let early_time = simulator.sample_default_time(0.999).unwrap();
        assert!(early_time < 1.0, "Early default should happen quickly, got {}", early_time);

        // Very high uniform = very late default (low U = late default)
        // Default time is capped at max_horizon (100 years)
        let late_time = simulator.sample_default_time(0.001).unwrap();
        // For λ=0.02: τ = -ln(0.001)/0.02 ≈ 345 years, but capped at 100
        assert!(late_time >= 100.0 - 0.1, "Late default should be at max horizon, got {}", late_time);
    }

    #[test]
    fn test_sample_default_time_invalid_uniform() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        assert!(simulator.sample_default_time(0.0).is_err());
        assert!(simulator.sample_default_time(1.0).is_err());
        assert!(simulator.sample_default_time(-0.1).is_err());
        assert!(simulator.sample_default_time(1.1).is_err());
    }

    #[test]
    fn test_defaults_within() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        // Survival at 5 years with 2% hazard: exp(-0.02 * 5) ≈ 0.9048
        let horizon = 5.0_f64;
        let survival_at_horizon = (-0.02_f64 * horizon).exp();

        // U > S(T) means default within horizon
        assert!(simulator.defaults_within(0.95, horizon).unwrap()); // 0.95 > 0.9048
        assert!(!simulator.defaults_within(0.80, horizon).unwrap()); // 0.80 < 0.9048
    }

    #[test]
    fn test_sample_default_times() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        let uniforms = vec![0.1, 0.5, 0.9];
        let times = simulator.sample_default_times(&uniforms).unwrap();

        assert_eq!(times.len(), 3);
        // Times should be ordered inversely to uniforms
        assert!(times[0] > times[1]); // Lower U = later default
        assert!(times[1] > times[2]); // Higher U = earlier default
    }

    // ========================================
    // CreditPathResult Tests
    // ========================================

    #[test]
    fn test_path_result_defaulted() {
        let result: CreditPathResult<f64> = CreditPathResult::defaulted(3.5, 5.0);

        assert!(result.is_defaulted());
        assert!(!result.is_survived());
        assert_eq!(result.default_time, Some(3.5));
        assert_eq!(result.status, DefaultStatus::Defaulted);
    }

    #[test]
    fn test_path_result_survived() {
        let result: CreditPathResult<f64> = CreditPathResult::survived(5.0);

        assert!(!result.is_defaulted());
        assert!(result.is_survived());
        assert_eq!(result.default_time, None);
        assert_eq!(result.status, DefaultStatus::Survived);
    }

    // ========================================
    // CreditMonteCarloSimulator Tests
    // ========================================

    #[test]
    fn test_mc_simulator_new() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let _simulator = CreditMonteCarloSimulator::new(&credit_curve, 5.0);
    }

    #[test]
    fn test_mc_simulate_path() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = CreditMonteCarloSimulator::new(&credit_curve, 5.0_f64);

        // High uniform = early default (within horizon)
        let result = simulator.simulate_path(0.95).unwrap();
        assert!(result.is_defaulted());

        // Low uniform = late default (after horizon)
        let result = simulator.simulate_path(0.5).unwrap();
        assert!(result.is_survived());
    }

    #[test]
    fn test_mc_simulate_paths() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = CreditMonteCarloSimulator::new(&credit_curve, 5.0_f64);

        let uniforms: Vec<f64> = (0..100).map(|i| 0.01 + 0.98 * (i as f64 / 99.0)).collect();
        let results = simulator.simulate_paths(&uniforms).unwrap();

        assert_eq!(results.len(), 100);

        // Some should default, some should survive
        let n_defaults = results.iter().filter(|r| r.is_defaulted()).count();
        assert!(n_defaults > 0 && n_defaults < 100);
    }

    #[test]
    fn test_mc_default_probability() {
        let hazard_rate = 0.02_f64;
        let horizon = 5.0_f64;
        let credit_curve = FlatHazardRateCurve::new(hazard_rate);
        let simulator = CreditMonteCarloSimulator::new(&credit_curve, horizon);

        // Generate deterministic uniform grid for testing
        let n = 10000;
        let uniforms: Vec<f64> = (1..=n).map(|i| i as f64 / (n + 1) as f64).collect();

        let mc_prob = simulator.default_probability(&uniforms).unwrap();

        // Analytical default probability: 1 - exp(-λT)
        let analytical_prob = 1.0 - (-hazard_rate * horizon).exp();

        // MC should be close to analytical
        assert!(
            (mc_prob - analytical_prob).abs() < 0.01,
            "MC prob {} should be close to analytical {}",
            mc_prob,
            analytical_prob
        );
    }

    #[test]
    fn test_mc_expected_default_time() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f64);
        let simulator = CreditMonteCarloSimulator::new(&credit_curve, 10.0_f64);

        // Generate uniforms that will default (high values)
        let uniforms: Vec<f64> = (0..100).map(|i| 0.9 + 0.09 * (i as f64 / 99.0)).collect();

        let expected_time = simulator.expected_default_time(&uniforms).unwrap();

        // Should have expected default time since we used high uniforms
        assert!(expected_time.is_some());
        let avg_time = expected_time.unwrap();
        assert!(avg_time > 0.0 && avg_time < 10.0);
    }

    #[test]
    fn test_mc_no_defaults() {
        let credit_curve = FlatHazardRateCurve::new(0.0001_f64); // Very low hazard rate
        let simulator = CreditMonteCarloSimulator::new(&credit_curve, 1.0_f64);

        // Use low uniforms = late defaults
        let uniforms: Vec<f64> = vec![0.1, 0.2, 0.3];

        let expected_time = simulator.expected_default_time(&uniforms).unwrap();
        assert!(expected_time.is_none()); // No defaults within horizon
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let credit_curve = FlatHazardRateCurve::new(0.02_f32);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        let default_time = simulator.sample_default_time(0.5_f32).unwrap();
        let expected = -0.5_f32.ln() / 0.02_f32;

        assert!((default_time - expected).abs() < 1e-3);
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_default_time_distribution() {
        // Verify that sampled default times follow the expected distribution
        let hazard_rate = 0.05_f64;
        let credit_curve = FlatHazardRateCurve::new(hazard_rate);
        let simulator = DefaultTimeSimulator::new(&credit_curve);

        // Sample 1000 default times
        let n = 1000;
        let uniforms: Vec<f64> = (1..=n).map(|i| i as f64 / (n + 1) as f64).collect();
        let times = simulator.sample_default_times(&uniforms).unwrap();

        // Mean of exponential with rate λ is 1/λ
        let expected_mean = 1.0 / hazard_rate;
        let actual_mean: f64 = times.iter().sum::<f64>() / n as f64;

        // Check mean is close (within 5%)
        let rel_error = (actual_mean - expected_mean).abs() / expected_mean;
        assert!(
            rel_error < 0.05,
            "Mean default time {} should be close to {}",
            actual_mean,
            expected_mean
        );
    }
}

//! Brownian bridge resampling for XVA exposure interpolation.
//!
//! Provides [`BrownianBridgeResampler`] which resamples netted PV paths
//! at intermediate collateral call dates using Brownian bridge scaling.

use infra_domain::counterparty::CallFrequency;
use pricer_core::math::rng::PricerRng;

/// Resampler that uses Brownian bridge interpolation to generate intermediate
/// exposure samples between simulation time grid points.
///
/// This is used when the collateral call frequency is finer than the
/// simulation time grid frequency, allowing realistic modelling of
/// margin period of risk dynamics.
#[derive(Clone, Debug)]
pub struct BrownianBridgeResampler {
    /// Call frequency in business days.
    call_frequency_days: f64,
}

impl BrownianBridgeResampler {
    /// Creates a new resampler with the given call frequency in days.
    ///
    /// # Panics
    ///
    /// Panics if `call_frequency_days <= 0.0`.
    pub fn new(call_frequency_days: f64) -> Self {
        assert!(
            call_frequency_days > 0.0,
            "call_frequency_days must be positive, got {call_frequency_days}"
        );
        Self {
            call_frequency_days,
        }
    }

    /// Creates a resampler from a [`CallFrequency`] enum.
    ///
    /// Mapping: Daily = 1 day, Weekly = 5 days, Monthly = 20 days.
    pub fn from_call_frequency(freq: CallFrequency) -> Self {
        let days = match freq {
            CallFrequency::Daily => 1.0,
            CallFrequency::Weekly => 5.0,
            CallFrequency::Monthly => 20.0,
        };
        Self::new(days)
    }

    /// Returns the Brownian bridge scaling factor.
    ///
    /// `scaling = sqrt(min(call_frequency_days, days_from_val_date) /
    /// days_from_val_date)`
    ///
    /// When `days_from_val_date == 0.0`, returns `1.0` to avoid division by
    /// zero.
    pub fn scaling_factor(&self, days_from_val_date: f64) -> f64 {
        if days_from_val_date <= 0.0 {
            return 1.0;
        }
        let dt = self.call_frequency_days.min(days_from_val_date);
        (dt / days_from_val_date).sqrt()
    }

    /// Resamples the netted PV vector using Brownian bridge interpolation.
    ///
    /// For each path, the resampled value is:
    /// ```text
    /// resampled[i] = E[V] + scaling * (netted_pv_at_t[random_index] - E[V])
    /// ```
    /// where `E[V]` is the cross-path mean and `scaling` is the Brownian bridge
    /// scaling factor.
    ///
    /// # Arguments
    ///
    /// * `netted_pv_at_t` - The netted PV values across all paths at a given
    ///   time.
    /// * `days_from_val_date` - Number of business days from valuation date.
    /// * `rng` - Random number generator for selecting indices.
    pub fn resample(
        &self,
        netted_pv_at_t: &[f64],
        days_from_val_date: f64,
        rng: &mut PricerRng,
    ) -> Vec<f64> {
        if netted_pv_at_t.is_empty() {
            return Vec::new();
        }

        let n = netted_pv_at_t.len();
        let ev: f64 = netted_pv_at_t.iter().sum::<f64>() / n as f64;
        let scaling = self.scaling_factor(days_from_val_date);

        let mut resampled = Vec::with_capacity(n);
        for _ in 0..n {
            let random_idx = (rng.gen_uniform() * n as f64) as usize;
            // Clamp to valid range (gen_uniform returns [0, 1) so this is
            // almost always in range, but guard against edge cases).
            let idx = random_idx.min(n - 1);
            let val = ev + scaling * (netted_pv_at_t[idx] - ev);
            resampled.push(val);
        }

        resampled
    }

    /// Returns the call frequency in business days.
    #[inline]
    pub fn call_frequency_days(&self) -> f64 { self.call_frequency_days }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_new_valid() {
        let bb = BrownianBridgeResampler::new(5.0);
        assert_relative_eq!(bb.call_frequency_days(), 5.0);
    }

    #[test]
    #[should_panic(expected = "call_frequency_days must be positive")]
    fn test_new_zero_frequency_panics() { BrownianBridgeResampler::new(0.0); }

    #[test]
    #[should_panic(expected = "call_frequency_days must be positive")]
    fn test_new_negative_frequency_panics() { BrownianBridgeResampler::new(-1.0); }

    #[test]
    fn test_from_call_frequency_daily() {
        let bb = BrownianBridgeResampler::from_call_frequency(CallFrequency::Daily);
        assert_relative_eq!(bb.call_frequency_days(), 1.0);
    }

    #[test]
    fn test_from_call_frequency_weekly() {
        let bb = BrownianBridgeResampler::from_call_frequency(CallFrequency::Weekly);
        assert_relative_eq!(bb.call_frequency_days(), 5.0);
    }

    #[test]
    fn test_from_call_frequency_monthly() {
        let bb = BrownianBridgeResampler::from_call_frequency(CallFrequency::Monthly);
        assert_relative_eq!(bb.call_frequency_days(), 20.0);
    }

    #[test]
    fn test_scaling_factor_basic() {
        let bb = BrownianBridgeResampler::new(5.0);
        // dt = min(5, 20) = 5, scaling = sqrt(5/20) = sqrt(0.25) = 0.5
        assert_relative_eq!(bb.scaling_factor(20.0), 0.5, epsilon = 1e-12);
    }

    #[test]
    fn test_scaling_factor_when_days_less_than_frequency() {
        let bb = BrownianBridgeResampler::new(10.0);
        // dt = min(10, 3) = 3, scaling = sqrt(3/3) = 1.0
        assert_relative_eq!(bb.scaling_factor(3.0), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_scaling_factor_zero_days() {
        let bb = BrownianBridgeResampler::new(5.0);
        assert_relative_eq!(bb.scaling_factor(0.0), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_scaling_factor_negative_days() {
        let bb = BrownianBridgeResampler::new(5.0);
        assert_relative_eq!(bb.scaling_factor(-1.0), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_scaling_factor_equal_frequency_and_days() {
        let bb = BrownianBridgeResampler::new(5.0);
        // dt = min(5, 5) = 5, scaling = sqrt(5/5) = 1.0
        assert_relative_eq!(bb.scaling_factor(5.0), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_resample_preserves_mean_approximately() {
        let bb = BrownianBridgeResampler::new(5.0);
        let pvs = vec![100.0, 102.0, 98.0, 101.0, 99.0, 103.0, 97.0, 100.5];
        let expected_mean: f64 = pvs.iter().sum::<f64>() / pvs.len() as f64;

        let mut rng = PricerRng::from_seed(42);
        let resampled = bb.resample(&pvs, 20.0, &mut rng);

        assert_eq!(resampled.len(), pvs.len());

        let resampled_mean: f64 = resampled.iter().sum::<f64>() / resampled.len() as f64;
        // The mean should be close to the original mean (within a few units
        // for this small sample).
        assert_relative_eq!(resampled_mean, expected_mean, epsilon = 5.0);
    }

    #[test]
    fn test_resample_empty_input() {
        let bb = BrownianBridgeResampler::new(5.0);
        let mut rng = PricerRng::from_seed(42);
        let resampled = bb.resample(&[], 10.0, &mut rng);
        assert!(resampled.is_empty());
    }

    #[test]
    fn test_resample_single_path() {
        let bb = BrownianBridgeResampler::new(5.0);
        let pvs = vec![100.0];
        let mut rng = PricerRng::from_seed(42);
        let resampled = bb.resample(&pvs, 20.0, &mut rng);

        // With a single path, ev = 100.0 and the only index to sample is 0,
        // so resampled = ev + scaling * (100 - ev) = 100.
        assert_eq!(resampled.len(), 1);
        assert_relative_eq!(resampled[0], 100.0, epsilon = 1e-12);
    }

    #[test]
    fn test_resample_scaling_one_returns_shuffled_originals() {
        // When days_from_val_date <= call_frequency_days, scaling = 1.0,
        // so resampled[i] = ev + 1.0 * (pv[rand_idx] - ev) = pv[rand_idx].
        let bb = BrownianBridgeResampler::new(10.0);
        let pvs = vec![10.0, 20.0, 30.0, 40.0];
        let mut rng = PricerRng::from_seed(99);
        let resampled = bb.resample(&pvs, 5.0, &mut rng);

        assert_eq!(resampled.len(), 4);
        // Each resampled value should be one of the original PV values.
        for &val in &resampled {
            assert!(
                pvs.contains(&val),
                "expected {val} to be one of the original PVs"
            );
        }
    }

    #[test]
    fn test_resample_deterministic_with_seed() {
        let bb = BrownianBridgeResampler::new(5.0);
        let pvs = vec![100.0, 110.0, 90.0, 105.0];

        let mut rng1 = PricerRng::from_seed(123);
        let result1 = bb.resample(&pvs, 20.0, &mut rng1);

        let mut rng2 = PricerRng::from_seed(123);
        let result2 = bb.resample(&pvs, 20.0, &mut rng2);

        assert_eq!(result1, result2);
    }
}

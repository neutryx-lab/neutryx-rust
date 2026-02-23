//! Collateral-adjusted exposure using asymmetric VmCsa parameters.

use pricer_core::math::rng::PricerRng;

/// Variation-margin CSA parameters for collateral adjustment.
///
/// Defines asymmetric threshold, MTA, and independent-amount settings for
/// computing collateral-adjusted exposures.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VmCsa {
    /// Threshold below which no VM collateral is posted by the counterparty.
    pub threshold_ctpy: f64,
    /// Threshold below which no VM collateral is posted by self.
    pub threshold_self: f64,
    /// Minimum Transfer Amount for counterparty.
    pub mta_ctpy: f64,
    /// Minimum Transfer Amount for self.
    pub mta_self: f64,
    /// Independent Amount (net IA; positive = we receive).
    pub independent_amount: f64,
    /// Margin Period of Risk in business days.
    pub mpor_days: f64,
}

impl Default for VmCsa {
    fn default() -> Self {
        Self {
            threshold_ctpy: 0.0,
            threshold_self: 0.0,
            mta_ctpy: 0.0,
            mta_self: 0.0,
            independent_amount: 0.0,
            mpor_days: 10.0,
        }
    }
}

impl VmCsa {
    /// Computes the net independent-amount contribution for the given exposure.
    /// Positive IA reduces the exposure we face (counterparty posts to us).
    #[inline]
    pub fn initial_margin(&self, _exposure: f64) -> f64 { self.independent_amount }
}

/// Computes collateral-adjusted exposure from raw netted PV paths using
/// asymmetric VM CSA terms.
pub struct CollateralAdjuster;

impl CollateralAdjuster {
    /// Produces collateral-adjusted exposure values from raw netted PV paths.
    ///
    /// # Arguments
    /// * `netted_pv` - `netted_pv[t][p]` is the netted PV at time-step `t`,
    ///   path `p`.
    /// * `vm_csa` - Variation-margin CSA parameters.
    /// * `call_frequency_days` - Collateral call frequency in calendar days.
    /// * `rng` - PRNG for MPOR resampling.
    ///
    /// # Returns
    /// `cv[t][p]` collateral-adjusted exposure for each time step and path.
    pub fn compute_collateral_values(
        netted_pv: &[Vec<f64>],
        vm_csa: &VmCsa,
        call_frequency_days: f64,
        rng: &mut PricerRng,
    ) -> Vec<Vec<f64>> {
        let n_times = netted_pv.len();
        if n_times == 0 {
            return Vec::new();
        }

        let mut cv: Vec<Vec<f64>> = Vec::with_capacity(n_times);

        for (t, pv_paths) in netted_pv.iter().enumerate() {
            let n_paths = pv_paths.len();
            if n_paths == 0 {
                cv.push(Vec::new());
                continue;
            }

            // Expected value for this time step (mean across paths).
            let ev: f64 = pv_paths.iter().sum::<f64>() / n_paths as f64;

            // Time from valuation approximated via the time-step index (in
            // units of grid steps). For the MPOR Brownian bridge scaling we
            // clamp to at least 1 to avoid division by zero at t=0.
            let days_from_val_eff = (t as f64).max(1.0);
            let dt = call_frequency_days.min(days_from_val_eff);
            let scaling = if days_from_val_eff > 0.0 {
                (dt / days_from_val_eff).sqrt()
            } else {
                0.0
            };

            let mut cv_t = Vec::with_capacity(n_paths);

            for p in 0..n_paths {
                let raw_exposure = pv_paths[p];

                // Net independent-amount contribution.
                let ia_net = vm_csa.initial_margin(raw_exposure);

                // Apply asymmetric threshold and MTA.
                // Collateral reduces the residual exposure. When threshold is
                // high no collateral is posted and the full exposure remains.
                let exposure_after_collateral = if raw_exposure > 0.0 {
                    // We face the counterparty: ctpy posts collateral.
                    let collateral = (raw_exposure - ia_net - vm_csa.threshold_ctpy).max(0.0);
                    // MTA filter: if the collateral call is below MTA, nothing
                    // is posted.
                    let posted = if collateral >= vm_csa.mta_ctpy {
                        collateral
                    } else {
                        0.0
                    };
                    raw_exposure - ia_net - posted
                } else {
                    // Counterparty faces us: we post collateral.
                    let collateral = (-raw_exposure - ia_net - vm_csa.threshold_self).max(0.0);
                    let posted = if collateral >= vm_csa.mta_self {
                        collateral
                    } else {
                        0.0
                    };
                    raw_exposure + ia_net + posted
                };

                // MPOR resampling via Brownian bridge approximation.
                let random_path = (rng.gen_uniform() * n_paths as f64) as usize % n_paths;
                let resample = scaling * (pv_paths[random_path] - ev);

                cv_t.push(exposure_after_collateral + resample);
            }

            cv.push(cv_t);
        }

        cv
    }

    /// Computes E[max(cv, 0)] at each time step.
    ///
    /// This gives the positive adjusted exposure profile suitable for FCA
    /// calculation.
    pub fn positive_adjusted_exposure(cv: &[Vec<f64>]) -> Vec<f64> {
        cv.iter()
            .map(|paths| {
                if paths.is_empty() {
                    return 0.0;
                }
                let sum: f64 = paths.iter().map(|&v| v.max(0.0)).sum();
                sum / paths.len() as f64
            })
            .collect()
    }

    /// Computes E[max(-cv, 0)] at each time step.
    ///
    /// This gives the negative adjusted exposure profile suitable for FBA
    /// calculation.
    pub fn negative_adjusted_exposure(cv: &[Vec<f64>]) -> Vec<f64> {
        cv.iter()
            .map(|paths| {
                if paths.is_empty() {
                    return 0.0;
                }
                let sum: f64 = paths.iter().map(|&v| (-v).max(0.0)).sum();
                sum / paths.len() as f64
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rng() -> PricerRng { PricerRng::from_seed(42) }

    #[test]
    fn test_zero_threshold_zero_ia_fully_collateralised() {
        let vm_csa = VmCsa::default();
        let mut rng = create_test_rng();

        // 3 time steps, 100 paths with constant value 50.
        let netted_pv: Vec<Vec<f64>> = vec![vec![50.0; 100]; 3];

        let cv = CollateralAdjuster::compute_collateral_values(&netted_pv, &vm_csa, 1.0, &mut rng);

        assert_eq!(cv.len(), 3);
        // With zero threshold, zero IA, and zero MTA the full exposure is
        // collateralised. All paths are identical so MPOR resampling noise
        // is zero (pv[random_path] - ev == 0). Hence cv should be ~0.
        for t in 0..3 {
            let mean_cv: f64 = cv[t].iter().sum::<f64>() / cv[t].len() as f64;
            assert!(
                mean_cv.abs() < 1.0,
                "Mean CV at t={} should be ~0 (fully collateralised), got {}",
                t,
                mean_cv
            );
        }
    }

    #[test]
    fn test_very_high_threshold() {
        let vm_csa = VmCsa {
            threshold_ctpy: 1e12,
            threshold_self: 1e12,
            ..VmCsa::default()
        };
        let mut rng = create_test_rng();

        // Some varying paths.
        let netted_pv: Vec<Vec<f64>> = vec![vec![100.0, 100.0, 100.0], vec![120.0, 110.0, 130.0]];

        let cv = CollateralAdjuster::compute_collateral_values(&netted_pv, &vm_csa, 1.0, &mut rng);

        // With a huge threshold, no collateral is posted so the MTA branch
        // keeps the raw exposure; the result should be approximately the
        // original PV (plus MPOR noise).
        for (t, pv_t) in netted_pv.iter().enumerate() {
            let mean_pv: f64 = pv_t.iter().sum::<f64>() / pv_t.len() as f64;
            let mean_cv: f64 = cv[t].iter().sum::<f64>() / cv[t].len() as f64;
            assert!(
                (mean_cv - mean_pv).abs() < 50.0,
                "At t={}, mean CV {} should be close to mean PV {}",
                t,
                mean_cv,
                mean_pv
            );
        }
    }

    #[test]
    fn test_positive_adjusted_exposure_non_negative() {
        let cv = vec![
            vec![-10.0, 20.0, -5.0, 15.0],
            vec![30.0, -20.0, 10.0, -40.0],
        ];

        let pae = CollateralAdjuster::positive_adjusted_exposure(&cv);

        assert_eq!(pae.len(), 2);
        for val in &pae {
            assert!(*val >= 0.0, "Positive adjusted exposure must be >= 0");
        }
    }

    #[test]
    fn test_negative_adjusted_exposure_non_negative() {
        let cv = vec![
            vec![-10.0, 20.0, -5.0, 15.0],
            vec![30.0, -20.0, 10.0, -40.0],
        ];

        let nae = CollateralAdjuster::negative_adjusted_exposure(&cv);

        assert_eq!(nae.len(), 2);
        for val in &nae {
            assert!(*val >= 0.0, "Negative adjusted exposure must be >= 0");
        }
    }

    #[test]
    fn test_adjusted_exposure_values() {
        // cv[0] = [-10, 20, -5, 15]
        //   positive: max(0, -10)=0, max(0,20)=20, max(0,-5)=0, max(0,15)=15 ->
        // mean=8.75   negative: max(0, 10)=10, max(0,-20)=0, max(0,5)=5,
        // max(0,-15)=0 -> mean=3.75
        let cv = vec![vec![-10.0, 20.0, -5.0, 15.0]];

        let pae = CollateralAdjuster::positive_adjusted_exposure(&cv);
        let nae = CollateralAdjuster::negative_adjusted_exposure(&cv);

        assert!((pae[0] - 8.75).abs() < 1e-10);
        assert!((nae[0] - 3.75).abs() < 1e-10);
    }

    #[test]
    fn test_empty_input() {
        let vm_csa = VmCsa::default();
        let mut rng = create_test_rng();

        let cv = CollateralAdjuster::compute_collateral_values(&[], &vm_csa, 1.0, &mut rng);
        assert!(cv.is_empty());

        let pae = CollateralAdjuster::positive_adjusted_exposure(&[]);
        assert!(pae.is_empty());

        let nae = CollateralAdjuster::negative_adjusted_exposure(&[]);
        assert!(nae.is_empty());
    }

    #[test]
    fn test_high_threshold_mean_exposure_approximately_preserved() {
        // With a very high threshold no collateral is posted, so the mean
        // adjusted exposure should stay close to the original PV.
        let vm_csa = VmCsa {
            threshold_ctpy: 1e12,
            threshold_self: 1e12,
            ..VmCsa::default()
        };
        let mut rng = create_test_rng();

        let n_paths = 1000;
        let netted_pv: Vec<Vec<f64>> = vec![vec![100.0; n_paths]; 5];

        let cv = CollateralAdjuster::compute_collateral_values(&netted_pv, &vm_csa, 1.0, &mut rng);

        for t in 0..5 {
            let mean_cv: f64 = cv[t].iter().sum::<f64>() / cv[t].len() as f64;
            assert!(
                (mean_cv - 100.0).abs() < 5.0,
                "Mean CV at t={} should be approximately 100, got {}",
                t,
                mean_cv
            );
        }
    }
}

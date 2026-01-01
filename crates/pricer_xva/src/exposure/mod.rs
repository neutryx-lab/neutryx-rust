//! Exposure aggregation calculations.
//!
//! This module provides utilities for computing exposure metrics
//! used in XVA calculations:
//!
//! - Expected Exposure (EE)
//! - Expected Positive Exposure (EPE)
//! - Potential Future Exposure (PFE)
//! - Netting benefit analysis

use rayon::prelude::*;

/// Exposure calculation utilities.
///
/// Provides methods for computing standard exposure metrics
/// from simulated portfolio values.
pub struct ExposureCalculator;

impl ExposureCalculator {
    /// Computes Expected Exposure at each time point.
    ///
    /// EE(t) = E[max(V(t), 0)]
    ///
    /// # Arguments
    ///
    /// * `values` - Simulated values `[scenario_idx][time_idx]`
    ///
    /// # Returns
    ///
    /// Expected exposure at each time point.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::exposure::ExposureCalculator;
    ///
    /// let values = vec![
    ///     vec![10.0, 20.0, 15.0],   // Scenario 1
    ///     vec![5.0, -10.0, 25.0],   // Scenario 2
    ///     vec![-5.0, 15.0, 10.0],   // Scenario 3
    /// ];
    ///
    /// let ee = ExposureCalculator::expected_exposure(&values);
    /// // At t=0: mean(max(10,0), max(5,0), max(-5,0)) = (10+5+0)/3 = 5
    /// ```
    pub fn expected_exposure(values: &[Vec<f64>]) -> Vec<f64> {
        if values.is_empty() {
            return Vec::new();
        }

        let n_times = values[0].len();
        let n_scenarios = values.len();

        if n_scenarios == 0 {
            return vec![0.0; n_times];
        }

        (0..n_times)
            .into_par_iter()
            .map(|t| {
                let sum: f64 = values.iter().map(|path| path[t].max(0.0)).sum();
                sum / n_scenarios as f64
            })
            .collect()
    }

    /// Computes time-weighted Expected Positive Exposure (EPE).
    ///
    /// EPE = (1/T) ∫₀ᵀ EE(t) dt
    ///
    /// Uses trapezoidal integration over the time grid.
    ///
    /// # Arguments
    ///
    /// * `ee` - Expected exposure profile
    /// * `time_grid` - Time points in years
    ///
    /// # Returns
    ///
    /// Time-averaged EPE (scalar).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::exposure::ExposureCalculator;
    ///
    /// let ee = vec![0.0, 10.0, 20.0, 15.0, 5.0];
    /// let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    ///
    /// let epe = ExposureCalculator::expected_positive_exposure(&ee, &time_grid);
    /// // EPE is the time-weighted average
    /// ```
    pub fn expected_positive_exposure(ee: &[f64], time_grid: &[f64]) -> f64 {
        if time_grid.len() < 2 || ee.len() != time_grid.len() {
            return ee.first().copied().unwrap_or(0.0);
        }

        // Trapezoidal integration
        let mut integral = 0.0;
        for i in 0..time_grid.len() - 1 {
            let dt = time_grid[i + 1] - time_grid[i];
            integral += 0.5 * (ee[i] + ee[i + 1]) * dt;
        }

        let total_time = time_grid.last().unwrap() - time_grid.first().unwrap();
        if total_time > 0.0 {
            integral / total_time
        } else {
            ee.first().copied().unwrap_or(0.0)
        }
    }

    /// Computes Potential Future Exposure at specified confidence level.
    ///
    /// PFE(t, α) = Quantile_α(max(V(t), 0))
    ///
    /// # Arguments
    ///
    /// * `values` - Simulated values `[scenario_idx][time_idx]`
    /// * `confidence` - Confidence level (e.g., 0.95 or 0.99)
    ///
    /// # Returns
    ///
    /// PFE at each time point.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::exposure::ExposureCalculator;
    ///
    /// let values = vec![
    ///     vec![10.0, 20.0],
    ///     vec![5.0, 30.0],
    ///     vec![15.0, 10.0],
    ///     vec![8.0, 25.0],
    /// ];
    ///
    /// let pfe_95 = ExposureCalculator::potential_future_exposure(&values, 0.95);
    /// ```
    pub fn potential_future_exposure(values: &[Vec<f64>], confidence: f64) -> Vec<f64> {
        if values.is_empty() {
            return Vec::new();
        }

        let n_times = values[0].len();
        let n_scenarios = values.len();

        if n_scenarios == 0 {
            return vec![0.0; n_times];
        }

        // Clamp confidence to valid range
        let confidence = confidence.clamp(0.0, 1.0);
        let quantile_idx = ((n_scenarios as f64 - 1.0) * confidence).round() as usize;
        let quantile_idx = quantile_idx.min(n_scenarios - 1);

        (0..n_times)
            .into_par_iter()
            .map(|t| {
                let mut exposures: Vec<f64> = values.iter().map(|path| path[t].max(0.0)).collect();
                exposures.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                exposures[quantile_idx]
            })
            .collect()
    }

    /// Computes peak PFE across all time points.
    ///
    /// # Arguments
    ///
    /// * `pfe` - PFE profile across time
    ///
    /// # Returns
    ///
    /// Maximum PFE value.
    #[inline]
    pub fn peak_pfe(pfe: &[f64]) -> f64 {
        pfe.iter().copied().fold(0.0_f64, |max, val| max.max(val))
    }

    /// Computes gross and net exposure from trade values.
    ///
    /// Gross exposure is the sum of absolute values (no netting).
    /// Net exposure applies netting: max(sum of values, 0).
    ///
    /// # Arguments
    ///
    /// * `trade_values` - MTM values for each trade
    ///
    /// # Returns
    ///
    /// Tuple of (gross_exposure, net_exposure).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_xva::exposure::ExposureCalculator;
    ///
    /// let trade_values = vec![10.0, -5.0, 3.0];
    /// let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);
    ///
    /// // Gross = |10| + |-5| + |3| = 18
    /// // Net = max(10 - 5 + 3, 0) = 8
    /// assert_eq!(gross, 18.0);
    /// assert_eq!(net, 8.0);
    /// ```
    pub fn netting_benefit(trade_values: &[f64]) -> (f64, f64) {
        let gross: f64 = trade_values.iter().map(|v| v.abs()).sum();
        let net: f64 = trade_values.iter().sum::<f64>().max(0.0);
        (gross, net)
    }

    /// Computes the netting benefit ratio.
    ///
    /// Ratio = 1 - (net_exposure / gross_exposure)
    ///
    /// A ratio of 0 means no benefit (all same sign).
    /// A ratio approaching 1 means significant netting benefit.
    ///
    /// # Arguments
    ///
    /// * `trade_values` - MTM values for each trade
    ///
    /// # Returns
    ///
    /// Netting benefit ratio in [0, 1].
    pub fn netting_benefit_ratio(trade_values: &[f64]) -> f64 {
        let (gross, net) = Self::netting_benefit(trade_values);
        if gross > 0.0 {
            1.0 - (net / gross)
        } else {
            0.0
        }
    }

    /// Computes Effective Expected Positive Exposure (EEPE).
    ///
    /// EEPE is the time-weighted average of non-decreasing EE,
    /// used in regulatory capital calculations.
    ///
    /// # Arguments
    ///
    /// * `ee` - Expected exposure profile
    /// * `time_grid` - Time points in years
    /// * `maturity_time` - Time horizon (typically 1 year for regulatory)
    ///
    /// # Returns
    ///
    /// Effective EPE (scalar).
    pub fn effective_epe(ee: &[f64], time_grid: &[f64], maturity_time: f64) -> f64 {
        if time_grid.is_empty() || ee.is_empty() {
            return 0.0;
        }

        // Compute Effective EE (non-decreasing)
        let mut effective_ee = vec![0.0; ee.len()];
        let mut running_max = 0.0_f64;
        for (i, &val) in ee.iter().enumerate() {
            running_max = running_max.max(val);
            effective_ee[i] = running_max;
        }

        // Integrate up to maturity_time
        let mut integral = 0.0;
        let mut t_max = 0.0;

        for i in 0..time_grid.len() - 1 {
            let t0 = time_grid[i];
            let t1 = time_grid[i + 1].min(maturity_time);

            if t0 >= maturity_time {
                break;
            }

            let dt = t1 - t0;
            if dt > 0.0 {
                // Linear interpolation for effective_ee at boundaries
                integral += 0.5 * (effective_ee[i] + effective_ee[i + 1]) * dt;
                t_max = t1;
            }
        }

        if t_max > 0.0 {
            integral / t_max
        } else {
            effective_ee.first().copied().unwrap_or(0.0)
        }
    }

    /// Computes Expected Negative Exposure (ENE) at each time point.
    ///
    /// ENE(t) = E[max(-V(t), 0)] = E[min(V(t), 0).abs()]
    ///
    /// Used for DVA calculations (exposure to counterparty if we default).
    ///
    /// # Arguments
    ///
    /// * `values` - Simulated values `[scenario_idx][time_idx]`
    ///
    /// # Returns
    ///
    /// Expected negative exposure at each time point.
    pub fn expected_negative_exposure(values: &[Vec<f64>]) -> Vec<f64> {
        if values.is_empty() {
            return Vec::new();
        }

        let n_times = values[0].len();
        let n_scenarios = values.len();

        if n_scenarios == 0 {
            return vec![0.0; n_times];
        }

        (0..n_times)
            .into_par_iter()
            .map(|t| {
                let sum: f64 = values.iter().map(|path| (-path[t]).max(0.0)).sum();
                sum / n_scenarios as f64
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_expected_exposure() {
        let values = vec![
            vec![10.0, 20.0, 15.0],
            vec![5.0, -10.0, 25.0],
            vec![-5.0, 15.0, 10.0],
        ];

        let ee = ExposureCalculator::expected_exposure(&values);

        // t=0: (max(10,0) + max(5,0) + max(-5,0)) / 3 = (10 + 5 + 0) / 3 = 5
        assert_relative_eq!(ee[0], 5.0, epsilon = 1e-10);
        // t=1: (max(20,0) + max(-10,0) + max(15,0)) / 3 = (20 + 0 + 15) / 3 ≈ 11.67
        assert_relative_eq!(ee[1], 35.0 / 3.0, epsilon = 1e-10);
        // t=2: (max(15,0) + max(25,0) + max(10,0)) / 3 = 50/3 ≈ 16.67
        assert_relative_eq!(ee[2], 50.0 / 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_expected_exposure_empty() {
        let values: Vec<Vec<f64>> = vec![];
        let ee = ExposureCalculator::expected_exposure(&values);
        assert!(ee.is_empty());
    }

    #[test]
    fn test_expected_positive_exposure() {
        let ee = vec![0.0, 10.0, 20.0, 15.0, 5.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let epe = ExposureCalculator::expected_positive_exposure(&ee, &time_grid);

        // Trapezoidal: 0.25 * (0+10)/2 + 0.25 * (10+20)/2 + 0.25 * (20+15)/2 + 0.25 * (15+5)/2
        //           = 1.25 + 3.75 + 4.375 + 2.5 = 11.875
        // EPE = 11.875 / 1.0 = 11.875
        assert_relative_eq!(epe, 11.875, epsilon = 1e-10);
    }

    #[test]
    fn test_potential_future_exposure() {
        let values = vec![vec![10.0], vec![5.0], vec![15.0], vec![20.0], vec![25.0]];

        let pfe_80 = ExposureCalculator::potential_future_exposure(&values, 0.80);

        // Sorted: [5, 10, 15, 20, 25]
        // 80% quantile index = round(4 * 0.8) = round(3.2) = 3 → value = 20
        assert_relative_eq!(pfe_80[0], 20.0, epsilon = 1e-10);
    }

    #[test]
    fn test_peak_pfe() {
        let pfe = vec![10.0, 25.0, 15.0, 30.0, 20.0];
        assert_eq!(ExposureCalculator::peak_pfe(&pfe), 30.0);
    }

    #[test]
    fn test_netting_benefit() {
        let trade_values = vec![10.0, -5.0, 3.0];
        let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);

        assert_eq!(gross, 18.0); // |10| + |-5| + |3|
        assert_eq!(net, 8.0); // max(10 - 5 + 3, 0)
    }

    #[test]
    fn test_netting_benefit_all_positive() {
        let trade_values = vec![10.0, 5.0, 3.0];
        let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);

        assert_eq!(gross, 18.0);
        assert_eq!(net, 18.0); // No netting benefit
    }

    #[test]
    fn test_netting_benefit_all_negative() {
        let trade_values = vec![-10.0, -5.0, -3.0];
        let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);

        assert_eq!(gross, 18.0);
        assert_eq!(net, 0.0); // Full netting benefit
    }

    #[test]
    fn test_netting_benefit_ratio() {
        let trade_values = vec![10.0, -5.0, 3.0];
        let ratio = ExposureCalculator::netting_benefit_ratio(&trade_values);

        // gross = 18, net = 8, ratio = 1 - 8/18 ≈ 0.556
        assert_relative_eq!(ratio, 1.0 - 8.0 / 18.0, epsilon = 1e-10);
    }

    #[test]
    fn test_netting_benefit_ratio_no_benefit() {
        let trade_values = vec![10.0, 5.0, 3.0];
        let ratio = ExposureCalculator::netting_benefit_ratio(&trade_values);
        assert_relative_eq!(ratio, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_expected_negative_exposure() {
        let values = vec![vec![10.0, -20.0], vec![-5.0, -10.0], vec![15.0, 5.0]];

        let ene = ExposureCalculator::expected_negative_exposure(&values);

        // t=0: (max(-10,0) + max(5,0) + max(-15,0)) / 3 = (0 + 5 + 0) / 3 ≈ 1.67
        assert_relative_eq!(ene[0], 5.0 / 3.0, epsilon = 1e-10);
        // t=1: (max(20,0) + max(10,0) + max(-5,0)) / 3 = (20 + 10 + 0) / 3 = 10
        assert_relative_eq!(ene[1], 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_epe() {
        // EE that decreases then increases
        let ee = vec![10.0, 8.0, 12.0, 15.0, 10.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let eepe = ExposureCalculator::effective_epe(&ee, &time_grid, 1.0);

        // Effective EE (non-decreasing): [10, 10, 12, 15, 15]
        // Should be higher than standard EPE due to non-decreasing constraint
        assert!(eepe > 0.0);
    }
}

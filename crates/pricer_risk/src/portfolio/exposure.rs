//! Exposure aggregation calculations for XVA (EE, EPE, PFE, netting benefit).

use rayon::prelude::*;

/// Exposure calculation utilities for computing standard exposure metrics from simulated portfolio values.
pub struct ExposureCalculator;

impl ExposureCalculator {
    /// Computes Expected Exposure at each time point: EE(t) = E[max(V(t), 0)].
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

    /// Computes time-weighted Expected Positive Exposure using trapezoidal integration: EPE = (1/T) * integral(EE(t) dt).
    pub fn expected_positive_exposure(ee: &[f64], time_grid: &[f64]) -> f64 {
        if time_grid.len() < 2 || ee.len() != time_grid.len() {
            return ee.first().copied().unwrap_or(0.0);
        }

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

    /// Computes Potential Future Exposure at specified confidence level: PFE(t, alpha) = Quantile_alpha(max(V(t), 0)).
    pub fn potential_future_exposure(values: &[Vec<f64>], confidence: f64) -> Vec<f64> {
        if values.is_empty() {
            return Vec::new();
        }

        let n_times = values[0].len();
        let n_scenarios = values.len();

        if n_scenarios == 0 {
            return vec![0.0; n_times];
        }

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
    #[inline]
    pub fn peak_pfe(pfe: &[f64]) -> f64 {
        pfe.iter().copied().fold(0.0_f64, |max, val| max.max(val))
    }

    /// Computes gross and net exposure from trade values, returning (gross_exposure, net_exposure).
    pub fn netting_benefit(trade_values: &[f64]) -> (f64, f64) {
        let gross: f64 = trade_values.iter().map(|v| v.abs()).sum();
        let net: f64 = trade_values.iter().sum::<f64>().max(0.0);
        (gross, net)
    }

    /// Computes the netting benefit ratio: 1 - (net/gross), ranging from 0 (no benefit) to 1 (full benefit).
    pub fn netting_benefit_ratio(trade_values: &[f64]) -> f64 {
        let (gross, net) = Self::netting_benefit(trade_values);
        if gross > 0.0 {
            1.0 - (net / gross)
        } else {
            0.0
        }
    }

    /// Computes Effective Expected Positive Exposure (EEPE) using non-decreasing EE for regulatory capital.
    pub fn effective_epe(ee: &[f64], time_grid: &[f64], maturity_time: f64) -> f64 {
        if time_grid.is_empty() || ee.is_empty() {
            return 0.0;
        }

        let mut effective_ee = vec![0.0; ee.len()];
        let mut running_max = 0.0_f64;
        for (i, &val) in ee.iter().enumerate() {
            running_max = running_max.max(val);
            effective_ee[i] = running_max;
        }

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

    /// Computes Expected Negative Exposure at each time point: ENE(t) = E[max(-V(t), 0)], used for DVA.
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
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_expected_exposure() {
        let values = vec![
            vec![10.0, 20.0, 15.0],
            vec![5.0, -10.0, 25.0],
            vec![-5.0, 15.0, 10.0],
        ];

        let ee = ExposureCalculator::expected_exposure(&values);

        assert_relative_eq!(ee[0], 5.0, epsilon = 1e-10);
        assert_relative_eq!(ee[1], 35.0 / 3.0, epsilon = 1e-10);
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

        assert_relative_eq!(epe, 11.875, epsilon = 1e-10);
    }

    #[test]
    fn test_potential_future_exposure() {
        let values = vec![vec![10.0], vec![5.0], vec![15.0], vec![20.0], vec![25.0]];

        let pfe_80 = ExposureCalculator::potential_future_exposure(&values, 0.80);

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

        assert_eq!(gross, 18.0);
        assert_eq!(net, 8.0);
    }

    #[test]
    fn test_netting_benefit_all_positive() {
        let trade_values = vec![10.0, 5.0, 3.0];
        let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);

        assert_eq!(gross, 18.0);
        assert_eq!(net, 18.0);
    }

    #[test]
    fn test_netting_benefit_all_negative() {
        let trade_values = vec![-10.0, -5.0, -3.0];
        let (gross, net) = ExposureCalculator::netting_benefit(&trade_values);

        assert_eq!(gross, 18.0);
        assert_eq!(net, 0.0);
    }

    #[test]
    fn test_netting_benefit_ratio() {
        let trade_values = vec![10.0, -5.0, 3.0];
        let ratio = ExposureCalculator::netting_benefit_ratio(&trade_values);

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

        assert_relative_eq!(ene[0], 5.0 / 3.0, epsilon = 1e-10);
        assert_relative_eq!(ene[1], 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_epe() {
        let ee = vec![10.0, 8.0, 12.0, 15.0, 10.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let eepe = ExposureCalculator::effective_epe(&ee, &time_grid, 1.0);

        assert!(eepe > 0.0);
    }
}

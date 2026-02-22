//! Exposure aggregation with Radon-Nikodym weighting for XVA.
//!
//! Provides methods for computing Expected Positive Exposure (EPE),
//! Expected Negative Exposure (ENE), netting of trade values, and
//! merging of external exposure paths.

use super::hierarchy::OtherExposurePaths;

/// Exposure aggregation engine for XVA computations.
///
/// Aggregates Monte Carlo trade values into exposure profiles
/// with optional numeraire (Radon-Nikodym) weighting.
pub struct ExposureAggregator;

impl ExposureAggregator {
    /// Computes the Expected Positive Exposure profile with numeraire
    /// weighting.
    ///
    /// EPE[t] = mean over paths of max(values[t][p], 0) *
    /// numeraire_ratios[t][p]
    ///
    /// `values` and `numeraire_ratios` are indexed as [time_idx][path_idx].
    pub fn compute_epe_weighted(values: &[Vec<f64>], numeraire_ratios: &[Vec<f64>]) -> Vec<f64> {
        values
            .iter()
            .zip(numeraire_ratios.iter())
            .map(|(v_t, nr_t)| {
                let n = v_t.len() as f64;
                if n == 0.0 {
                    return 0.0;
                }
                let sum: f64 = v_t
                    .iter()
                    .zip(nr_t.iter())
                    .map(|(&v, &nr)| v.max(0.0) * nr)
                    .sum();
                sum / n
            })
            .collect()
    }

    /// Computes the Expected Negative Exposure profile with numeraire
    /// weighting.
    ///
    /// ENE[t] = mean over paths of max(-values[t][p], 0) *
    /// numeraire_ratios[t][p]
    ///
    /// `values` and `numeraire_ratios` are indexed as [time_idx][path_idx].
    pub fn compute_ene_weighted(values: &[Vec<f64>], numeraire_ratios: &[Vec<f64>]) -> Vec<f64> {
        values
            .iter()
            .zip(numeraire_ratios.iter())
            .map(|(v_t, nr_t)| {
                let n = v_t.len() as f64;
                if n == 0.0 {
                    return 0.0;
                }
                let sum: f64 = v_t
                    .iter()
                    .zip(nr_t.iter())
                    .map(|(&v, &nr)| (-v).max(0.0) * nr)
                    .sum();
                sum / n
            })
            .collect()
    }

    /// Computes the Expected Positive Exposure profile (unweighted).
    ///
    /// EPE[t] = mean over paths of max(values[t][p], 0)
    pub fn compute_epe(values: &[Vec<f64>]) -> Vec<f64> {
        values
            .iter()
            .map(|v_t| {
                let n = v_t.len() as f64;
                if n == 0.0 {
                    return 0.0;
                }
                let sum: f64 = v_t.iter().map(|&v| v.max(0.0)).sum();
                sum / n
            })
            .collect()
    }

    /// Computes the Expected Negative Exposure profile (unweighted).
    ///
    /// ENE[t] = mean over paths of max(-values[t][p], 0)
    pub fn compute_ene(values: &[Vec<f64>]) -> Vec<f64> {
        values
            .iter()
            .map(|v_t| {
                let n = v_t.len() as f64;
                if n == 0.0 {
                    return 0.0;
                }
                let sum: f64 = v_t.iter().map(|&v| (-v).max(0.0)).sum();
                sum / n
            })
            .collect()
    }

    /// Nets trade values within a netting set.
    ///
    /// `trade_values` is indexed as [trade_idx][time_idx][path_idx].
    /// Returns netted values indexed as [time_idx][path_idx].
    ///
    /// netted[t][p] = sum_i(trade_values[i][t][p])
    pub fn net_exposures(trade_values: &[Vec<Vec<f64>>]) -> Vec<Vec<f64>> {
        if trade_values.is_empty() {
            return Vec::new();
        }

        let n_times = trade_values[0].len();
        let n_paths = if n_times > 0 {
            trade_values[0][0].len()
        } else {
            0
        };

        let mut netted = vec![vec![0.0; n_paths]; n_times];

        for trade in trade_values {
            for (t, trade_t) in trade.iter().enumerate() {
                for (p, &val) in trade_t.iter().enumerate() {
                    netted[t][p] += val;
                }
            }
        }

        netted
    }

    /// Merges external exposure paths into netted values.
    ///
    /// netted[t][p] += other.paths[t][p]
    pub fn merge_other_exposure(netted: &mut [Vec<f64>], other: &OtherExposurePaths) {
        other.add_to_exposure(netted);
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_epe_simple() {
        // 2 time points, 4 paths
        let values = vec![
            vec![1.0, -2.0, 3.0, -4.0], // t=0
            vec![5.0, -1.0, 2.0, -3.0], // t=1
        ];

        let epe = ExposureAggregator::compute_epe(&values);
        assert_eq!(epe.len(), 2);
        // EPE[0] = mean(max(1,0), max(-2,0), max(3,0), max(-4,0)) = (1+0+3+0)/4 = 1.0
        assert_relative_eq!(epe[0], 1.0, epsilon = 1e-10);
        // EPE[1] = mean(5, 0, 2, 0) / 4 = 7/4 = 1.75
        assert_relative_eq!(epe[1], 1.75, epsilon = 1e-10);
    }

    #[test]
    fn test_ene_simple() {
        let values = vec![vec![1.0, -2.0, 3.0, -4.0], vec![5.0, -1.0, 2.0, -3.0]];

        let ene = ExposureAggregator::compute_ene(&values);
        assert_eq!(ene.len(), 2);
        // ENE[0] = mean(max(-1,0), max(2,0), max(-3,0), max(4,0)) = (0+2+0+4)/4 = 1.5
        assert_relative_eq!(ene[0], 1.5, epsilon = 1e-10);
        // ENE[1] = (0+1+0+3)/4 = 1.0
        assert_relative_eq!(ene[1], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_flat_numeraire_matches_unweighted() {
        let values = vec![vec![1.0, -2.0, 3.0, -4.0], vec![5.0, -1.0, 2.0, -3.0]];
        // Flat numeraire = 1.0 everywhere
        let numeraire = vec![vec![1.0; 4]; 2];

        let epe_weighted = ExposureAggregator::compute_epe_weighted(&values, &numeraire);
        let epe_unweighted = ExposureAggregator::compute_epe(&values);

        for (w, u) in epe_weighted.iter().zip(epe_unweighted.iter()) {
            assert_relative_eq!(w, u, epsilon = 1e-10);
        }

        let ene_weighted = ExposureAggregator::compute_ene_weighted(&values, &numeraire);
        let ene_unweighted = ExposureAggregator::compute_ene(&values);

        for (w, u) in ene_weighted.iter().zip(ene_unweighted.iter()) {
            assert_relative_eq!(w, u, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_weighted_epe_with_discount() {
        // values = [[2.0, -1.0]], numeraire = [[0.5, 0.5]]
        let values = vec![vec![2.0, -1.0]];
        let numeraire = vec![vec![0.5, 0.5]];

        let epe = ExposureAggregator::compute_epe_weighted(&values, &numeraire);
        // EPE[0] = (max(2,0)*0.5 + max(-1,0)*0.5) / 2 = (1.0 + 0.0) / 2 = 0.5
        assert_relative_eq!(epe[0], 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_net_exposures() {
        // 2 trades, 2 time points, 3 paths
        let trade_values = vec![
            vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]], // trade 0
            vec![vec![-1.0, -1.0, -1.0], vec![1.0, 1.0, 1.0]], // trade 1
        ];

        let netted = ExposureAggregator::net_exposures(&trade_values);
        assert_eq!(netted.len(), 2);
        assert_eq!(netted[0].len(), 3);

        // netted[0] = [1-1, 2-1, 3-1] = [0, 1, 2]
        assert_relative_eq!(netted[0][0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(netted[0][1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(netted[0][2], 2.0, epsilon = 1e-10);

        // netted[1] = [4+1, 5+1, 6+1] = [5, 6, 7]
        assert_relative_eq!(netted[1][0], 5.0, epsilon = 1e-10);
        assert_relative_eq!(netted[1][1], 6.0, epsilon = 1e-10);
        assert_relative_eq!(netted[1][2], 7.0, epsilon = 1e-10);
    }

    #[test]
    fn test_net_exposures_empty() {
        let trade_values: Vec<Vec<Vec<f64>>> = vec![];
        let netted = ExposureAggregator::net_exposures(&trade_values);
        assert!(netted.is_empty());
    }

    #[test]
    fn test_merge_other_exposure() {
        let n_times = 2;
        let n_paths = 3;

        let mut netted = vec![vec![10.0; n_paths]; n_times];
        let mut other = OtherExposurePaths::new(n_times, n_paths);
        other.set(0, 0, 5.0);
        other.set(1, 2, 7.0);

        ExposureAggregator::merge_other_exposure(&mut netted, &other);

        assert_relative_eq!(netted[0][0], 15.0, epsilon = 1e-10);
        assert_relative_eq!(netted[0][1], 10.0, epsilon = 1e-10);
        assert_relative_eq!(netted[1][2], 17.0, epsilon = 1e-10);
    }

    #[test]
    fn test_epe_all_positive() {
        let values = vec![vec![1.0, 2.0, 3.0, 4.0]];
        let epe = ExposureAggregator::compute_epe(&values);
        assert_relative_eq!(epe[0], 2.5, epsilon = 1e-10);
    }

    #[test]
    fn test_epe_all_negative() {
        let values = vec![vec![-1.0, -2.0, -3.0, -4.0]];
        let epe = ExposureAggregator::compute_epe(&values);
        assert_relative_eq!(epe[0], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_ene_all_negative() {
        let values = vec![vec![-1.0, -2.0, -3.0, -4.0]];
        let ene = ExposureAggregator::compute_ene(&values);
        assert_relative_eq!(ene[0], 2.5, epsilon = 1e-10);
    }

    #[test]
    fn test_empty_paths() {
        let values: Vec<Vec<f64>> = vec![vec![]];
        let epe = ExposureAggregator::compute_epe(&values);
        assert_relative_eq!(epe[0], 0.0, epsilon = 1e-10);
    }
}

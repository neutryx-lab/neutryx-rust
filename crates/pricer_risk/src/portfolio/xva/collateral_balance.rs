//! Expected Collateral Balance (ECB) computation.

/// Computes the Expected Collateral Balance at each time step.
///
/// ECB[t] = mean over paths of |collateral_values[t][p]|
///
/// # Arguments
/// * `collateral_values` - `collateral_values[t][p]` is the collateral-adjusted
///   exposure at time `t`, path `p`.
///
/// # Returns
/// A vector of ECB values, one per time step.
pub fn compute_ecb(collateral_values: &[Vec<f64>]) -> Vec<f64> {
    collateral_values
        .iter()
        .map(|paths| {
            if paths.is_empty() {
                return 0.0;
            }
            let sum: f64 = paths.iter().map(|&v| v.abs()).sum();
            sum / paths.len() as f64
        })
        .collect()
}

/// Computes the Radon-Nikodym weighted Expected Collateral Balance at each
/// time step.
///
/// ECB[t] = mean over paths of |collateral_values[t][p]| *
/// numeraire_ratios[t][p]
///
/// This is the measure-change-weighted version, useful when switching between
/// pricing measures (e.g. risk-neutral to T-forward).
///
/// # Arguments
/// * `collateral_values` - `collateral_values[t][p]` is the collateral-adjusted
///   exposure at time `t`, path `p`.
/// * `numeraire_ratios` - `numeraire_ratios[t][p]` is the Radon-Nikodym weight
///   at time `t`, path `p`.
///
/// # Returns
/// A vector of weighted ECB values, one per time step.
pub fn compute_ecb_weighted(
    collateral_values: &[Vec<f64>],
    numeraire_ratios: &[Vec<f64>],
) -> Vec<f64> {
    collateral_values
        .iter()
        .zip(numeraire_ratios.iter())
        .map(|(cv_paths, nr_paths)| {
            if cv_paths.is_empty() || nr_paths.is_empty() {
                return 0.0;
            }
            let n = cv_paths.len().min(nr_paths.len());
            let sum: f64 = cv_paths
                .iter()
                .zip(nr_paths.iter())
                .take(n)
                .map(|(&cv, &nr)| cv.abs() * nr)
                .sum();
            sum / n as f64
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecb_zero_collateral() {
        let cv = vec![vec![0.0; 10]; 5];
        let ecb = compute_ecb(&cv);

        assert_eq!(ecb.len(), 5);
        for val in &ecb {
            assert_eq!(*val, 0.0);
        }
    }

    #[test]
    fn test_ecb_constant_collateral() {
        let cv = vec![vec![42.0; 100]; 3];
        let ecb = compute_ecb(&cv);

        assert_eq!(ecb.len(), 3);
        for val in &ecb {
            assert!((val - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_ecb_negative_values_taken_as_abs() {
        // ECB should use absolute value.
        let cv = vec![vec![-10.0, -20.0, -30.0]];
        let ecb = compute_ecb(&cv);

        assert_eq!(ecb.len(), 1);
        assert!((ecb[0] - 20.0).abs() < 1e-10); // mean(10, 20, 30) = 20
    }

    #[test]
    fn test_ecb_mixed_signs() {
        let cv = vec![vec![-10.0, 20.0, -30.0, 40.0]];
        let ecb = compute_ecb(&cv);

        // |values| = [10, 20, 30, 40], mean = 25
        assert!((ecb[0] - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecb_empty_input() {
        let ecb = compute_ecb(&[]);
        assert!(ecb.is_empty());
    }

    #[test]
    fn test_ecb_empty_paths() {
        let cv: Vec<Vec<f64>> = vec![vec![]];
        let ecb = compute_ecb(&cv);

        assert_eq!(ecb.len(), 1);
        assert_eq!(ecb[0], 0.0);
    }

    #[test]
    fn test_ecb_weighted_flat_numeraire_matches_unweighted() {
        let cv = vec![
            vec![-10.0, 20.0, -30.0, 40.0],
            vec![5.0, -15.0, 25.0, -35.0],
        ];
        let nr = vec![vec![1.0; 4]; 2]; // flat numeraire = 1

        let ecb = compute_ecb(&cv);
        let ecb_w = compute_ecb_weighted(&cv, &nr);

        assert_eq!(ecb.len(), ecb_w.len());
        for (a, b) in ecb.iter().zip(ecb_w.iter()) {
            assert!(
                (a - b).abs() < 1e-10,
                "Weighted ECB with flat numeraire should match unweighted"
            );
        }
    }

    #[test]
    fn test_ecb_weighted_scaling() {
        // If numeraire_ratio = 2.0 for all paths, weighted ECB = 2 * unweighted.
        let cv = vec![vec![10.0, 20.0, 30.0]];
        let nr = vec![vec![2.0; 3]];

        let ecb = compute_ecb(&cv);
        let ecb_w = compute_ecb_weighted(&cv, &nr);

        assert!((ecb_w[0] - 2.0 * ecb[0]).abs() < 1e-10);
    }

    #[test]
    fn test_ecb_weighted_empty_input() {
        let ecb_w = compute_ecb_weighted(&[], &[]);
        assert!(ecb_w.is_empty());
    }

    #[test]
    fn test_ecb_weighted_empty_paths() {
        let cv: Vec<Vec<f64>> = vec![vec![]];
        let nr: Vec<Vec<f64>> = vec![vec![]];
        let ecb_w = compute_ecb_weighted(&cv, &nr);

        assert_eq!(ecb_w.len(), 1);
        assert_eq!(ecb_w[0], 0.0);
    }
}

//! XVA risk indicator profiles and CSV output.
//!
//! Contains computed risk profiles (EPE, ENE, ECB, PFE) along a time grid,
//! with support for CSV serialization.

use std::{collections::HashMap, fmt::Write as _};

/// Computed XVA risk indicator profiles along a time grid.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XvaRiskIndicators {
    /// Time grid in year fractions.
    pub time_grid: Vec<f64>,
    /// Expected Positive Exposure profile.
    pub epe: Vec<f64>,
    /// Expected Negative Exposure profile.
    pub ene: Vec<f64>,
    /// Expected Collateral Balance profile.
    pub ecb: Vec<f64>,
    /// Potential Future Exposure profiles keyed by percentile (e.g., "95.0").
    pub pfe: HashMap<String, Vec<f64>>,
}

impl XvaRiskIndicators {
    /// Creates a new risk indicators struct with empty profiles for the given
    /// time grid.
    pub fn new(time_grid: Vec<f64>) -> Self {
        let n = time_grid.len();
        Self {
            time_grid,
            epe: vec![0.0; n],
            ene: vec![0.0; n],
            ecb: vec![0.0; n],
            pfe: HashMap::new(),
        }
    }

    /// Computes the Potential Future Exposure at a given percentile.
    ///
    /// PFE[t] = quantile(max(values[t][p], 0) for all p, percentile)
    ///
    /// `values` is indexed as [time_idx][path_idx].
    /// `percentile` is in (0, 1), e.g., 0.95 for the 95th percentile.
    pub fn compute_pfe(values: &[Vec<f64>], percentile: f64) -> Vec<f64> {
        values
            .iter()
            .map(|v_t| {
                if v_t.is_empty() {
                    return 0.0;
                }

                let mut positive: Vec<f64> = v_t.iter().map(|&v| v.max(0.0)).collect();
                positive.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let idx = ((percentile * positive.len() as f64).ceil() as usize)
                    .saturating_sub(1)
                    .min(positive.len() - 1);
                positive[idx]
            })
            .collect()
    }

    /// Writes the risk indicators to CSV format.
    ///
    /// Header format: "Time,EPE,ENE,ECB,PFE_95.0,PFE_97.5,PFE_99.0"
    /// (PFE columns depend on available percentiles, sorted by key).
    pub fn to_csv(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        // Build header
        let mut header = "Time,EPE,ENE,ECB".to_string();
        let mut pfe_keys: Vec<&String> = self.pfe.keys().collect();
        pfe_keys.sort();
        for key in &pfe_keys {
            let _ = write!(header, ",PFE_{}", key);
        }
        writeln!(writer, "{}", header)?;

        // Write rows
        for (i, &t) in self.time_grid.iter().enumerate() {
            let epe_val = self.epe.get(i).copied().unwrap_or(0.0);
            let ene_val = self.ene.get(i).copied().unwrap_or(0.0);
            let ecb_val = self.ecb.get(i).copied().unwrap_or(0.0);

            let mut row = format!("{},{},{},{}", t, epe_val, ene_val, ecb_val);
            for key in &pfe_keys {
                let pfe_val = self
                    .pfe
                    .get(*key)
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.0);
                let _ = write!(row, ",{}", pfe_val);
            }
            writeln!(writer, "{}", row)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_new_risk_indicators() {
        let time_grid = vec![0.25, 0.5, 0.75, 1.0];
        let ri = XvaRiskIndicators::new(time_grid.clone());

        assert_eq!(ri.time_grid, time_grid);
        assert_eq!(ri.epe.len(), 4);
        assert_eq!(ri.ene.len(), 4);
        assert_eq!(ri.ecb.len(), 4);
        assert!(ri.pfe.is_empty());

        for i in 0..4 {
            assert_relative_eq!(ri.epe[i], 0.0, epsilon = 1e-10);
            assert_relative_eq!(ri.ene[i], 0.0, epsilon = 1e-10);
            assert_relative_eq!(ri.ecb[i], 0.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_compute_pfe_95() {
        // 1 time point, 100 paths with values [1, 2, ..., 100]
        let values = vec![(1..=100).map(|i| i as f64).collect::<Vec<f64>>()];

        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.95);
        assert_eq!(pfe.len(), 1);
        // 95th percentile of [1..100]: index = ceil(0.95 * 100) - 1 = 94
        // value = 95.0
        assert_relative_eq!(pfe[0], 95.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_pfe_99() {
        let values = vec![(1..=100).map(|i| i as f64).collect::<Vec<f64>>()];

        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.99);
        assert_eq!(pfe.len(), 1);
        // 99th percentile: index = ceil(0.99 * 100) - 1 = 98, value = 99.0
        assert_relative_eq!(pfe[0], 99.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_pfe_with_negatives() {
        // Negative values should be floored to 0 before computing quantile
        let values = vec![vec![-5.0, -3.0, -1.0, 1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0]];

        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.5);
        // After flooring: [0, 0, 0, 1, 3, 5, 7, 9, 11, 13]
        // Sorted: [0, 0, 0, 1, 3, 5, 7, 9, 11, 13]
        // 50th percentile: index = ceil(0.5 * 10) - 1 = 4, value = 3.0
        assert_relative_eq!(pfe[0], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_pfe_empty_paths() {
        let values: Vec<Vec<f64>> = vec![vec![]];
        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.95);
        assert_relative_eq!(pfe[0], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_pfe_single_path() {
        let values = vec![vec![42.0]];
        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.95);
        assert_relative_eq!(pfe[0], 42.0, epsilon = 1e-10);
    }

    #[test]
    fn test_csv_output_basic() {
        let time_grid = vec![0.25, 0.5];
        let mut ri = XvaRiskIndicators::new(time_grid);
        ri.epe = vec![1.0, 2.0];
        ri.ene = vec![0.5, 1.5];
        ri.ecb = vec![0.1, 0.2];
        ri.pfe.insert("95.0".to_string(), vec![3.0, 4.0]);
        ri.pfe.insert("99.0".to_string(), vec![5.0, 6.0]);

        let mut buf = Vec::new();
        ri.to_csv(&mut buf).unwrap();
        let csv_str = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = csv_str.trim().split('\n').collect();
        assert_eq!(lines.len(), 3); // header + 2 data rows

        // Header should contain PFE columns sorted
        assert_eq!(lines[0], "Time,EPE,ENE,ECB,PFE_95.0,PFE_99.0");

        // First data row
        assert_eq!(lines[1], "0.25,1,0.5,0.1,3,5");

        // Second data row
        assert_eq!(lines[2], "0.5,2,1.5,0.2,4,6");
    }

    #[test]
    fn test_csv_output_no_pfe() {
        let time_grid = vec![1.0];
        let mut ri = XvaRiskIndicators::new(time_grid);
        ri.epe = vec![10.0];
        ri.ene = vec![5.0];
        ri.ecb = vec![2.0];

        let mut buf = Vec::new();
        ri.to_csv(&mut buf).unwrap();
        let csv_str = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = csv_str.trim().split('\n').collect();
        assert_eq!(lines[0], "Time,EPE,ENE,ECB");
        assert_eq!(lines[1], "1,10,5,2");
    }

    #[test]
    fn test_risk_indicators_clone() {
        let mut ri = XvaRiskIndicators::new(vec![0.25, 0.5]);
        ri.epe = vec![1.0, 2.0];
        ri.pfe.insert("95.0".to_string(), vec![3.0, 4.0]);

        let cloned = ri.clone();
        assert_eq!(ri.epe, cloned.epe);
        assert_eq!(ri.pfe.get("95.0"), cloned.pfe.get("95.0"));
    }

    #[test]
    fn test_risk_indicators_serde_roundtrip() {
        let mut ri = XvaRiskIndicators::new(vec![0.25, 0.5]);
        ri.epe = vec![1.0, 2.0];
        ri.pfe.insert("95.0".to_string(), vec![3.0, 4.0]);

        let json = serde_json::to_string(&ri).unwrap();
        let deserialized: XvaRiskIndicators = serde_json::from_str(&json).unwrap();
        assert_eq!(ri.epe, deserialized.epe);
        assert_eq!(ri.time_grid, deserialized.time_grid);
    }

    #[test]
    fn test_compute_pfe_multiple_time_points() {
        let values = vec![
            (1..=10).map(|i| i as f64).collect::<Vec<f64>>(),
            (11..=20).map(|i| i as f64).collect::<Vec<f64>>(),
        ];

        let pfe = XvaRiskIndicators::compute_pfe(&values, 0.90);
        assert_eq!(pfe.len(), 2);
        // First time: 90th percentile of [1..10], index = ceil(0.9*10)-1 = 8, value =
        // 9.0
        assert_relative_eq!(pfe[0], 9.0, epsilon = 1e-10);
        // Second time: 90th percentile of [11..20], index = ceil(0.9*10)-1 = 8, value =
        // 19.0
        assert_relative_eq!(pfe[1], 19.0, epsilon = 1e-10);
    }
}

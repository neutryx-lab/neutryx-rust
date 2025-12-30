//! Structure of Arrays for exposure profiles.
//!
//! Provides storage for exposure profiles across time and scenarios.

use crate::portfolio::NettingSetId;

/// SoA representation of exposure profiles.
///
/// Stores exposure values across time and netting sets for
/// efficient XVA calculations.
///
/// # Memory Layout
///
/// ```text
/// time_grid:       [t0, t1, t2, ..., tn]
/// exposures[ns0]:  [e00, e01, e02, ..., e0n]
/// exposures[ns1]:  [e10, e11, e12, ..., e1n]
/// ...
/// ```
#[derive(Debug, Clone)]
pub struct ExposureSoA {
    /// Time grid shared across all netting sets
    time_grid: Vec<f64>,
    /// Exposure profiles indexed by [netting_set_idx][time_idx]
    exposures: Vec<Vec<f64>>,
    /// Netting set IDs for reference
    netting_set_ids: Vec<NettingSetId>,
}

impl ExposureSoA {
    /// Creates a new exposure SoA with the given time grid.
    ///
    /// # Arguments
    ///
    /// * `time_grid` - Time points in years
    /// * `netting_set_ids` - IDs of netting sets to track
    pub fn new(time_grid: Vec<f64>, netting_set_ids: Vec<NettingSetId>) -> Self {
        let n_times = time_grid.len();
        let n_sets = netting_set_ids.len();

        Self {
            time_grid,
            exposures: vec![vec![0.0; n_times]; n_sets],
            netting_set_ids,
        }
    }

    /// Creates an exposure SoA with preallocated capacity.
    pub fn with_capacity(n_times: usize, n_netting_sets: usize) -> Self {
        Self {
            time_grid: Vec::with_capacity(n_times),
            exposures: Vec::with_capacity(n_netting_sets),
            netting_set_ids: Vec::with_capacity(n_netting_sets),
        }
    }

    /// Returns the number of time points.
    #[inline]
    pub fn n_times(&self) -> usize {
        self.time_grid.len()
    }

    /// Returns the number of netting sets.
    #[inline]
    pub fn n_netting_sets(&self) -> usize {
        self.netting_set_ids.len()
    }

    /// Returns the time grid.
    #[inline]
    pub fn time_grid(&self) -> &[f64] {
        &self.time_grid
    }

    /// Returns the netting set IDs.
    #[inline]
    pub fn netting_set_ids(&self) -> &[NettingSetId] {
        &self.netting_set_ids
    }

    /// Gets the exposure profile for a netting set by index.
    ///
    /// # Arguments
    ///
    /// * `ns_idx` - Netting set index
    ///
    /// # Returns
    ///
    /// Slice of exposure values across time.
    #[inline]
    pub fn exposure_profile(&self, ns_idx: usize) -> &[f64] {
        &self.exposures[ns_idx]
    }

    /// Gets mutable exposure profile for a netting set by index.
    #[inline]
    pub fn exposure_profile_mut(&mut self, ns_idx: usize) -> &mut [f64] {
        &mut self.exposures[ns_idx]
    }

    /// Sets the exposure value at a specific time for a netting set.
    ///
    /// # Arguments
    ///
    /// * `ns_idx` - Netting set index
    /// * `time_idx` - Time index
    /// * `value` - Exposure value
    #[inline]
    pub fn set_exposure(&mut self, ns_idx: usize, time_idx: usize, value: f64) {
        self.exposures[ns_idx][time_idx] = value;
    }

    /// Gets the exposure value at a specific time for a netting set.
    #[inline]
    pub fn get_exposure(&self, ns_idx: usize, time_idx: usize) -> f64 {
        self.exposures[ns_idx][time_idx]
    }

    /// Returns exposures at a specific time for all netting sets.
    ///
    /// # Arguments
    ///
    /// * `time_idx` - Time index
    ///
    /// # Returns
    ///
    /// Vector of exposures for each netting set at that time.
    pub fn exposures_at_time(&self, time_idx: usize) -> Vec<f64> {
        self.exposures.iter().map(|e| e[time_idx]).collect()
    }

    /// Returns the peak exposure for each netting set.
    pub fn peak_exposures(&self) -> Vec<f64> {
        self.exposures
            .iter()
            .map(|profile| {
                profile
                    .iter()
                    .copied()
                    .fold(0.0_f64, |max, val| max.max(val))
            })
            .collect()
    }

    /// Returns the total exposure across all netting sets at each time.
    pub fn total_exposure_profile(&self) -> Vec<f64> {
        let n_times = self.n_times();
        (0..n_times)
            .map(|t| self.exposures.iter().map(|e| e[t]).sum())
            .collect()
    }

    /// Finds the netting set index by ID.
    pub fn find_netting_set(&self, id: &NettingSetId) -> Option<usize> {
        self.netting_set_ids.iter().position(|ns_id| ns_id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_exposure_soa() -> ExposureSoA {
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let ns_ids = vec![NettingSetId::new("NS001"), NettingSetId::new("NS002")];
        let mut soa = ExposureSoA::new(time_grid, ns_ids);

        // Set some exposure values
        // NS001: increasing exposure
        for (t, &val) in [0.0, 10.0, 20.0, 15.0, 5.0].iter().enumerate() {
            soa.set_exposure(0, t, val);
        }
        // NS002: different pattern
        for (t, &val) in [5.0, 8.0, 12.0, 10.0, 3.0].iter().enumerate() {
            soa.set_exposure(1, t, val);
        }

        soa
    }

    #[test]
    fn test_exposure_soa_creation() {
        let soa = create_test_exposure_soa();
        assert_eq!(soa.n_times(), 5);
        assert_eq!(soa.n_netting_sets(), 2);
    }

    #[test]
    fn test_time_grid() {
        let soa = create_test_exposure_soa();
        assert_eq!(soa.time_grid(), &[0.0, 0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn test_exposure_profile() {
        let soa = create_test_exposure_soa();
        let profile = soa.exposure_profile(0);
        assert_eq!(profile, &[0.0, 10.0, 20.0, 15.0, 5.0]);
    }

    #[test]
    fn test_get_set_exposure() {
        let mut soa = create_test_exposure_soa();
        soa.set_exposure(0, 2, 100.0);
        assert_eq!(soa.get_exposure(0, 2), 100.0);
    }

    #[test]
    fn test_exposures_at_time() {
        let soa = create_test_exposure_soa();
        let exposures = soa.exposures_at_time(2); // t=0.5
        assert_eq!(exposures, vec![20.0, 12.0]);
    }

    #[test]
    fn test_peak_exposures() {
        let soa = create_test_exposure_soa();
        let peaks = soa.peak_exposures();
        assert_eq!(peaks, vec![20.0, 12.0]); // Max for each NS
    }

    #[test]
    fn test_total_exposure_profile() {
        let soa = create_test_exposure_soa();
        let total = soa.total_exposure_profile();
        assert_eq!(total, vec![5.0, 18.0, 32.0, 25.0, 8.0]);
    }

    #[test]
    fn test_find_netting_set() {
        let soa = create_test_exposure_soa();
        assert_eq!(soa.find_netting_set(&NettingSetId::new("NS001")), Some(0));
        assert_eq!(soa.find_netting_set(&NettingSetId::new("NS002")), Some(1));
        assert_eq!(soa.find_netting_set(&NettingSetId::new("NS999")), None);
    }

    #[test]
    fn test_with_capacity() {
        let soa = ExposureSoA::with_capacity(100, 10);
        assert_eq!(soa.n_times(), 0);
        assert_eq!(soa.n_netting_sets(), 0);
    }

    #[test]
    fn test_netting_set_ids() {
        let soa = create_test_exposure_soa();
        assert_eq!(soa.netting_set_ids().len(), 2);
        assert_eq!(soa.netting_set_ids()[0].as_str(), "NS001");
    }
}

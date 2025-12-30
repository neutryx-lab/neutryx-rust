//! Funding Valuation Adjustment (FVA) calculation.
//!
//! FVA represents the cost/benefit of funding positive/negative exposure.
//!
//! # Formulas
//!
//! FCA = ∫₀ᵀ EE(t) × s_borrow × df(t) dt
//! FBA = ∫₀ᵀ ENE(t) × s_lend × df(t) dt
//! FVA = FCA - FBA
//!
//! Where:
//! - EE(t) = Expected Exposure (funding cost on positive exposure)
//! - ENE(t) = Expected Negative Exposure (funding benefit)
//! - s_borrow = Borrowing spread
//! - s_lend = Lending spread
//! - df(t) = Discount factor

/// Computes Funding Cost Adjustment (FCA).
///
/// FCA represents the cost of funding positive exposure.
///
/// # Arguments
///
/// * `ee` - Expected Exposure profile
/// * `time_grid` - Time points in years
/// * `funding_spread` - Borrowing spread (annualised decimal)
/// * `discount_factors` - Risk-free discount factors at each time point
///
/// # Returns
///
/// FCA value (always non-negative).
///
/// # Examples
///
/// ```
/// use pricer_xva::xva::compute_fca;
///
/// let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
/// let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
/// let funding_spread = 0.005; // 50bp
/// let df = vec![1.0, 0.99, 0.98, 0.97, 0.96];
///
/// let fca = compute_fca(&ee, &time_grid, funding_spread, &df);
/// assert!(fca > 0.0);
/// ```
pub fn compute_fca(
    ee: &[f64],
    time_grid: &[f64],
    funding_spread: f64,
    discount_factors: &[f64],
) -> f64 {
    if time_grid.len() < 2
        || ee.len() != time_grid.len()
        || discount_factors.len() != time_grid.len()
    {
        return 0.0;
    }

    let mut fca = 0.0;

    for i in 0..time_grid.len() - 1 {
        let dt = time_grid[i + 1] - time_grid[i];
        let avg_ee = 0.5 * (ee[i] + ee[i + 1]);
        let avg_df = 0.5 * (discount_factors[i] + discount_factors[i + 1]);

        fca += avg_ee * funding_spread * avg_df * dt;
    }

    fca.max(0.0)
}

/// Computes Funding Benefit Adjustment (FBA).
///
/// FBA represents the benefit of investing negative exposure.
///
/// # Arguments
///
/// * `ene` - Expected Negative Exposure profile
/// * `time_grid` - Time points in years
/// * `lending_spread` - Lending spread (annualised decimal)
/// * `discount_factors` - Risk-free discount factors at each time point
///
/// # Returns
///
/// FBA value (always non-negative).
pub fn compute_fba(
    ene: &[f64],
    time_grid: &[f64],
    lending_spread: f64,
    discount_factors: &[f64],
) -> f64 {
    if time_grid.len() < 2
        || ene.len() != time_grid.len()
        || discount_factors.len() != time_grid.len()
    {
        return 0.0;
    }

    let mut fba = 0.0;

    for i in 0..time_grid.len() - 1 {
        let dt = time_grid[i + 1] - time_grid[i];
        let avg_ene = 0.5 * (ene[i] + ene[i + 1]);
        let avg_df = 0.5 * (discount_factors[i] + discount_factors[i + 1]);

        fba += avg_ene * lending_spread * avg_df * dt;
    }

    fba.max(0.0)
}

/// Computes combined FVA (FCA - FBA).
///
/// Positive FVA = net funding cost
/// Negative FVA = net funding benefit
///
/// # Arguments
///
/// * `ee` - Expected Exposure profile
/// * `ene` - Expected Negative Exposure profile
/// * `time_grid` - Time points
/// * `spread_borrow` - Borrowing spread
/// * `spread_lend` - Lending spread
/// * `discount_factors` - Discount factors
///
/// # Returns
///
/// Tuple of (FCA, FBA, FVA).
pub fn compute_fva(
    ee: &[f64],
    ene: &[f64],
    time_grid: &[f64],
    spread_borrow: f64,
    spread_lend: f64,
    discount_factors: &[f64],
) -> (f64, f64, f64) {
    let fca = compute_fca(ee, time_grid, spread_borrow, discount_factors);
    let fba = compute_fba(ene, time_grid, spread_lend, discount_factors);
    let fva = fca - fba;
    (fca, fba, fva)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn create_flat_df(rate: f64, times: &[f64]) -> Vec<f64> {
        times.iter().map(|&t| (-rate * t).exp()).collect()
    }

    #[test]
    fn test_fca_basic() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fca = compute_fca(&ee, &time_grid, 0.005, &df);
        assert!(fca > 0.0);
    }

    #[test]
    fn test_fca_zero_exposure() {
        let ee = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fca = compute_fca(&ee, &time_grid, 0.005, &df);
        assert_eq!(fca, 0.0);
    }

    #[test]
    fn test_fca_zero_spread() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fca = compute_fca(&ee, &time_grid, 0.0, &df);
        assert_eq!(fca, 0.0);
    }

    #[test]
    fn test_fca_proportional_to_spread() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fca_50bp = compute_fca(&ee, &time_grid, 0.005, &df);
        let fca_100bp = compute_fca(&ee, &time_grid, 0.01, &df);

        assert_relative_eq!(fca_100bp / fca_50bp, 2.0, max_relative = 0.01);
    }

    #[test]
    fn test_fba_basic() {
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fba = compute_fba(&ene, &time_grid, 0.003, &df);
        assert!(fba > 0.0);
    }

    #[test]
    fn test_fba_zero_ene() {
        let ene = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let fba = compute_fba(&ene, &time_grid, 0.003, &df);
        assert_eq!(fba, 0.0);
    }

    #[test]
    fn test_fva_net_cost() {
        // More positive exposure than negative → net funding cost
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let ene = vec![0.0, 30.0, 30.0, 30.0, 30.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let (fca, fba, fva) = compute_fva(&ee, &ene, &time_grid, 0.005, 0.003, &df);

        assert!(fca > 0.0);
        assert!(fba > 0.0);
        assert!(fva > 0.0); // Net cost
        assert_relative_eq!(fva, fca - fba, epsilon = 1e-10);
    }

    #[test]
    fn test_fva_net_benefit() {
        // More negative exposure than positive → net funding benefit
        let ee = vec![0.0, 30.0, 30.0, 30.0, 30.0];
        let ene = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        let (_fca, _fba, fva) = compute_fva(&ee, &ene, &time_grid, 0.005, 0.005, &df);

        assert!(fva < 0.0); // Net benefit
    }

    #[test]
    fn test_fva_symmetric_spreads() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let ene = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let df = create_flat_df(0.05, &time_grid);

        // With symmetric spreads and equal EE/ENE, FVA should be ~0
        let (fca, fba, fva) = compute_fva(&ee, &ene, &time_grid, 0.005, 0.005, &df);

        assert_relative_eq!(fva, 0.0, epsilon = 1e-10);
        assert_relative_eq!(fca, fba, epsilon = 1e-10);
    }

    #[test]
    fn test_fva_empty_inputs() {
        let (fca, fba, fva) = compute_fva(&[], &[], &[], 0.005, 0.003, &[]);
        assert_eq!(fca, 0.0);
        assert_eq!(fba, 0.0);
        assert_eq!(fva, 0.0);
    }

    #[test]
    fn test_fca_non_negative() {
        let ee = vec![0.0, -10.0, -20.0]; // Negative EE (unusual)
        let time_grid = vec![0.0, 0.5, 1.0];
        let df = vec![1.0, 0.98, 0.96];

        let fca = compute_fca(&ee, &time_grid, 0.005, &df);
        assert!(fca >= 0.0);
    }
}

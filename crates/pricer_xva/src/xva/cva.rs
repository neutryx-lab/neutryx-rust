//! Credit Valuation Adjustment (CVA) calculation.
//!
//! CVA represents the expected loss due to counterparty default.
//!
//! # Formula
//!
//! CVA = LGD × ∫₀ᵀ EE(t) × dPD(t)
//!
//! Where:
//! - LGD = Loss Given Default
//! - EE(t) = Expected Exposure at time t
//! - dPD(t) = Marginal default probability

use crate::portfolio::CreditParams;

/// Computes unilateral CVA for a netting set.
///
/// Uses trapezoidal integration over the time grid.
///
/// # Arguments
///
/// * `ee` - Expected Exposure profile at each time point
/// * `time_grid` - Time points in years
/// * `credit_params` - Counterparty credit parameters (hazard rate, LGD)
///
/// # Returns
///
/// CVA value (always non-negative).
///
/// # Examples
///
/// ```
/// use pricer_xva::xva::compute_cva;
/// use pricer_xva::portfolio::CreditParams;
///
/// let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
/// let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
/// let credit = CreditParams::new(0.02, 0.4).unwrap();
///
/// let cva = compute_cva(&ee, &time_grid, &credit);
/// assert!(cva > 0.0);
/// ```
pub fn compute_cva(ee: &[f64], time_grid: &[f64], credit_params: &CreditParams) -> f64 {
    if time_grid.len() < 2 || ee.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = credit_params.lgd();
    let mut cva = 0.0;

    // Numerical integration using trapezoidal rule
    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        // Marginal default probability over interval [t1, t2]
        let marginal_pd = credit_params.marginal_default_prob(t1, t2);

        // Average EE over interval (trapezoidal)
        let avg_ee = 0.5 * (ee[i] + ee[i + 1]);

        cva += lgd * avg_ee * marginal_pd;
    }

    cva.max(0.0) // Ensure non-negative
}

/// Computes CVA with survival probability weighting.
///
/// This version explicitly weights by the counterparty's survival probability,
/// useful for bilateral CVA calculations.
///
/// # Arguments
///
/// * `ee` - Expected Exposure profile
/// * `time_grid` - Time points
/// * `credit_params` - Counterparty credit parameters
/// * `own_survival` - Own survival probabilities at each time point
pub fn compute_cva_with_survival(
    ee: &[f64],
    time_grid: &[f64],
    credit_params: &CreditParams,
    own_survival: &[f64],
) -> f64 {
    if time_grid.len() < 2 || ee.len() != time_grid.len() || own_survival.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = credit_params.lgd();
    let mut cva = 0.0;

    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        let marginal_pd = credit_params.marginal_default_prob(t1, t2);
        let avg_ee = 0.5 * (ee[i] + ee[i + 1]);

        // Weight by own survival (we must survive to experience the loss)
        let avg_own_survival = 0.5 * (own_survival[i] + own_survival[i + 1]);

        cva += lgd * avg_ee * marginal_pd * avg_own_survival;
    }

    cva.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn create_test_credit_params() -> CreditParams {
        CreditParams::new(0.02, 0.4).unwrap()
    }

    #[test]
    fn test_cva_basic() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let credit = create_test_credit_params();

        let cva = compute_cva(&ee, &time_grid, &credit);

        // CVA should be positive for positive exposure
        assert!(cva > 0.0);
    }

    #[test]
    fn test_cva_zero_exposure() {
        let ee = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let credit = create_test_credit_params();

        let cva = compute_cva(&ee, &time_grid, &credit);
        assert_eq!(cva, 0.0);
    }

    #[test]
    fn test_cva_higher_pd_higher_cva() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let low_risk = CreditParams::new(0.01, 0.4).unwrap();
        let high_risk = CreditParams::new(0.05, 0.4).unwrap();

        let cva_low = compute_cva(&ee, &time_grid, &low_risk);
        let cva_high = compute_cva(&ee, &time_grid, &high_risk);

        assert!(cva_high > cva_low);
    }

    #[test]
    fn test_cva_higher_lgd_higher_cva() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let low_lgd = CreditParams::new(0.02, 0.3).unwrap();
        let high_lgd = CreditParams::new(0.02, 0.6).unwrap();

        let cva_low = compute_cva(&ee, &time_grid, &low_lgd);
        let cva_high = compute_cva(&ee, &time_grid, &high_lgd);

        assert!(cva_high > cva_low);
    }

    #[test]
    fn test_cva_proportional_to_lgd() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let lgd_40 = CreditParams::new(0.02, 0.4).unwrap();
        let lgd_80 = CreditParams::new(0.02, 0.8).unwrap();

        let cva_40 = compute_cva(&ee, &time_grid, &lgd_40);
        let cva_80 = compute_cva(&ee, &time_grid, &lgd_80);

        // CVA should scale approximately linearly with LGD
        assert_relative_eq!(cva_80 / cva_40, 2.0, max_relative = 0.01);
    }

    #[test]
    fn test_cva_non_negative() {
        let ee = vec![0.0, -10.0, -20.0, -10.0, 0.0]; // Negative exposure
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let credit = create_test_credit_params();

        let cva = compute_cva(&ee, &time_grid, &credit);
        assert!(cva >= 0.0);
    }

    #[test]
    fn test_cva_empty_inputs() {
        let credit = create_test_credit_params();

        // Empty arrays
        assert_eq!(compute_cva(&[], &[], &credit), 0.0);

        // Mismatched lengths
        let ee = vec![100.0, 100.0];
        let time_grid = vec![0.0, 0.5, 1.0];
        assert_eq!(compute_cva(&ee, &time_grid, &credit), 0.0);
    }

    #[test]
    fn test_cva_single_point() {
        let ee = vec![100.0];
        let time_grid = vec![0.0];
        let credit = create_test_credit_params();

        // Single point - no integration possible
        assert_eq!(compute_cva(&ee, &time_grid, &credit), 0.0);
    }

    #[test]
    fn test_cva_with_survival() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let credit = create_test_credit_params();

        // Full survival
        let full_survival = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let cva_full = compute_cva_with_survival(&ee, &time_grid, &credit, &full_survival);
        let cva_basic = compute_cva(&ee, &time_grid, &credit);

        // Should be approximately equal
        assert_relative_eq!(cva_full, cva_basic, max_relative = 0.01);

        // Reduced survival should reduce CVA
        let partial_survival = vec![1.0, 0.9, 0.8, 0.7, 0.6];
        let cva_partial = compute_cva_with_survival(&ee, &time_grid, &credit, &partial_survival);

        assert!(cva_partial < cva_basic);
    }
}

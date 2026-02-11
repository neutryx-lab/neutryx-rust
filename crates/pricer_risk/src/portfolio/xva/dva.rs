//! Debit Valuation Adjustment (DVA) calculation.

use super::params::OwnCreditParams;

/// Computes DVA for a netting set using trapezoidal integration: DVA = LGD_own
/// * integral(ENE(t) * dPD_own(t)).
pub fn compute_dva(ene: &[f64], time_grid: &[f64], own_credit: &OwnCreditParams) -> f64 {
    if time_grid.len() < 2 || ene.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = own_credit.lgd();
    let mut dva = 0.0;

    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        let marginal_pd = own_credit.marginal_pd(t1, t2);
        let avg_ene = 0.5 * (ene[i] + ene[i + 1]);

        dva += lgd * avg_ene * marginal_pd;
    }

    dva.max(0.0)
}

/// Computes DVA with counterparty survival weighting for bilateral DVA
/// calculations.
pub fn compute_dva_with_survival(
    ene: &[f64],
    time_grid: &[f64],
    own_credit: &OwnCreditParams,
    cp_survival: &[f64],
) -> f64 {
    if time_grid.len() < 2 || ene.len() != time_grid.len() || cp_survival.len() != time_grid.len() {
        return 0.0;
    }

    let lgd = own_credit.lgd();
    let mut dva = 0.0;

    for i in 0..time_grid.len() - 1 {
        let t1 = time_grid[i];
        let t2 = time_grid[i + 1];

        let marginal_pd = own_credit.marginal_pd(t1, t2);
        let avg_ene = 0.5 * (ene[i] + ene[i + 1]);
        let avg_cp_survival = 0.5 * (cp_survival[i] + cp_survival[i + 1]);

        dva += lgd * avg_ene * marginal_pd * avg_cp_survival;
    }

    dva.max(0.0)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn create_test_own_credit() -> OwnCreditParams { OwnCreditParams::new(0.03, 0.4).unwrap() }

    #[test]
    fn test_dva_basic() {
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let own_credit = create_test_own_credit();

        let dva = compute_dva(&ene, &time_grid, &own_credit);

        assert!(dva > 0.0);
    }

    #[test]
    fn test_dva_zero_ene() {
        let ene = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let own_credit = create_test_own_credit();

        let dva = compute_dva(&ene, &time_grid, &own_credit);
        assert_eq!(dva, 0.0);
    }

    #[test]
    fn test_dva_higher_own_pd_higher_dva() {
        let ene = vec![0.0, 50.0, 50.0, 50.0, 50.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let low_risk = OwnCreditParams::new(0.01, 0.4).unwrap();
        let high_risk = OwnCreditParams::new(0.05, 0.4).unwrap();

        let dva_low = compute_dva(&ene, &time_grid, &low_risk);
        let dva_high = compute_dva(&ene, &time_grid, &high_risk);

        assert!(dva_high > dva_low);
    }

    #[test]
    fn test_dva_proportional_to_lgd() {
        let ene = vec![0.0, 50.0, 50.0, 50.0, 50.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let lgd_30 = OwnCreditParams::new(0.03, 0.3).unwrap();
        let lgd_60 = OwnCreditParams::new(0.03, 0.6).unwrap();

        let dva_30 = compute_dva(&ene, &time_grid, &lgd_30);
        let dva_60 = compute_dva(&ene, &time_grid, &lgd_60);

        assert_relative_eq!(dva_60 / dva_30, 2.0, max_relative = 0.01);
    }

    #[test]
    fn test_dva_non_negative() {
        let ene = vec![0.0, -10.0, -20.0, -10.0, 0.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let own_credit = create_test_own_credit();

        let dva = compute_dva(&ene, &time_grid, &own_credit);
        assert!(dva >= 0.0);
    }

    #[test]
    fn test_dva_empty_inputs() {
        let own_credit = create_test_own_credit();

        assert_eq!(compute_dva(&[], &[], &own_credit), 0.0);

        let ene = vec![50.0, 50.0];
        let time_grid = vec![0.0, 0.5, 1.0];
        assert_eq!(compute_dva(&ene, &time_grid, &own_credit), 0.0);
    }

    #[test]
    fn test_dva_with_survival() {
        let ene = vec![0.0, 50.0, 50.0, 50.0, 50.0];
        let time_grid = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let own_credit = create_test_own_credit();

        let full_survival = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let dva_full = compute_dva_with_survival(&ene, &time_grid, &own_credit, &full_survival);
        let dva_basic = compute_dva(&ene, &time_grid, &own_credit);

        assert_relative_eq!(dva_full, dva_basic, max_relative = 0.01);

        let partial_survival = vec![1.0, 0.9, 0.8, 0.7, 0.6];
        let dva_partial =
            compute_dva_with_survival(&ene, &time_grid, &own_credit, &partial_survival);

        assert!(dva_partial < dva_basic);
    }
}

//! Bilateral CVA/DVA with First-to-Default and cross-currency FVA.

use super::integrate::{trapezoidal_xva, trapezoidal_xva_with_survival};
use super::params::OwnCreditParams;
use crate::portfolio::CreditParams;

/// Result of bilateral CVA/DVA calculation including both unilateral and
/// bilateral (first-to-default) measures.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BilateralCvaResult {
    /// Unilateral CVA (no own-survival weighting).
    pub ucva: f64,
    /// Unilateral DVA (no counterparty-survival weighting).
    pub udva: f64,
    /// Bilateral CVA (weighted by own survival probability).
    pub bcva: f64,
    /// Bilateral DVA (weighted by counterparty survival probability).
    pub bdva: f64,
}

/// Result of FVA calculation with optional cross-currency basis spread.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FvaWithBasisResult {
    /// Funding Cost Adjustment.
    pub fca: f64,
    /// Funding Benefit Adjustment.
    pub fba: f64,
    /// Net FVA = FCA - FBA.
    pub fva: f64,
}

/// Calculator for bilateral CVA/DVA with first-to-default logic and FVA with
/// cross-currency basis.
pub struct BilateralXvaCalculator;

impl BilateralXvaCalculator {
    /// Computes unilateral CVA: UCVA = LGD_ctpy * integral(EE(t) * dPD_ctpy(t)).
    ///
    /// Delegates to the existing `compute_cva` logic.
    pub fn compute_ucva(ee: &[f64], time_grid: &[f64], credit_params: &CreditParams) -> f64 {
        let lgd = credit_params.lgd();
        trapezoidal_xva(ee, time_grid, |_i, t1, t2| {
            lgd * credit_params.marginal_default_prob(t1, t2)
        })
    }

    /// Computes unilateral DVA: UDVA = LGD_own * integral(ENE(t) * dPD_own(t)).
    ///
    /// Delegates to the existing `compute_dva` logic.
    pub fn compute_udva(ene: &[f64], time_grid: &[f64], own_credit: &OwnCreditParams) -> f64 {
        let lgd = own_credit.lgd();
        trapezoidal_xva(ene, time_grid, |_i, t1, t2| {
            lgd * own_credit.marginal_pd(t1, t2)
        })
    }

    /// Computes bilateral CVA and DVA under first-to-default logic.
    ///
    /// - BCVA = LGD_ctpy * integral(EE(t) * Q_own(t) * dPD_ctpy(t))
    /// - BDVA = LGD_own  * integral(ENE(t) * Q_ctpy(t) * dPD_own(t))
    ///
    /// The bilateral measures are smaller than unilateral because they account
    /// for the probability that the weighting party is still alive when the
    /// other defaults.
    pub fn compute_bilateral_cva(
        ee: &[f64],
        ene: &[f64],
        time_grid: &[f64],
        credit_params: &CreditParams,
        own_credit: &OwnCreditParams,
    ) -> BilateralCvaResult {
        let ucva = Self::compute_ucva(ee, time_grid, credit_params);
        let udva = Self::compute_udva(ene, time_grid, own_credit);

        // Build survival probability arrays.
        let own_survival: Vec<f64> = time_grid
            .iter()
            .map(|&t| own_credit.survival_prob(t))
            .collect();
        let ctpy_survival: Vec<f64> = time_grid
            .iter()
            .map(|&t| credit_params.survival_prob(t))
            .collect();

        // BCVA: EE weighted by own survival, with ctpy default probability.
        let lgd_ctpy = credit_params.lgd();
        let bcva = trapezoidal_xva_with_survival(ee, time_grid, &own_survival, |_i, t1, t2| {
            lgd_ctpy * credit_params.marginal_default_prob(t1, t2)
        });

        // BDVA: ENE weighted by ctpy survival, with own default probability.
        let lgd_own = own_credit.lgd();
        let bdva = trapezoidal_xva_with_survival(ene, time_grid, &ctpy_survival, |_i, t1, t2| {
            lgd_own * own_credit.marginal_pd(t1, t2)
        });

        BilateralCvaResult {
            ucva,
            udva,
            bcva,
            bdva,
        }
    }

    /// Computes FVA with optional cross-currency basis spread.
    ///
    /// - FCA = integral(cv_positive(t) * (spread_borrow + xccy_basis(t)) * Q_both(t) * df(t) * dt)
    /// - FBA = integral(cv_negative(t) * (spread_lend + xccy_basis(t)) * Q_both(t) * df(t) * dt)
    /// - FVA = FCA - FBA
    ///
    /// `survival_both[t]` = Q_own(t) * Q_ctpy(t) (joint survival).
    /// `xccy_basis` is optional; if `None` it is treated as zero.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_fva_with_basis(
        cv_positive: &[f64],
        cv_negative: &[f64],
        time_grid: &[f64],
        spread_borrow: f64,
        spread_lend: f64,
        discount_factors: &[f64],
        survival_both: &[f64],
        xccy_basis: Option<&[f64]>,
    ) -> FvaWithBasisResult {
        let n = time_grid.len();
        if n < 2
            || cv_positive.len() != n
            || cv_negative.len() != n
            || discount_factors.len() != n
            || survival_both.len() != n
        {
            return FvaWithBasisResult::default();
        }

        if let Some(basis) = xccy_basis {
            if basis.len() != n {
                return FvaWithBasisResult::default();
            }
        }

        let mut fca = 0.0;
        let mut fba = 0.0;

        for i in 0..n - 1 {
            let dt = time_grid[i + 1] - time_grid[i];
            let avg_df = 0.5 * (discount_factors[i] + discount_factors[i + 1]);
            let avg_surv = 0.5 * (survival_both[i] + survival_both[i + 1]);

            let basis_i = xccy_basis.map_or(0.0, |b| 0.5 * (b[i] + b[i + 1]));

            let avg_pos = 0.5 * (cv_positive[i] + cv_positive[i + 1]);
            let avg_neg = 0.5 * (cv_negative[i] + cv_negative[i + 1]);

            fca += avg_pos * (spread_borrow + basis_i) * avg_surv * avg_df * dt;
            fba += avg_neg * (spread_lend + basis_i) * avg_surv * avg_df * dt;
        }

        fca = fca.max(0.0);
        fba = fba.max(0.0);

        FvaWithBasisResult {
            fca,
            fba,
            fva: fca - fba,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn create_test_time_grid() -> Vec<f64> {
        vec![0.0, 0.25, 0.5, 0.75, 1.0]
    }

    fn create_test_credit_params() -> CreditParams {
        CreditParams::new(0.02, 0.4).unwrap()
    }

    fn create_test_own_credit() -> OwnCreditParams {
        OwnCreditParams::new(0.03, 0.4).unwrap()
    }

    fn create_flat_df(rate: f64, times: &[f64]) -> Vec<f64> {
        times.iter().map(|&t| (-rate * t).exp()).collect()
    }

    // -- UCVA tests --

    #[test]
    fn test_ucva_matches_compute_cva() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let time_grid = create_test_time_grid();
        let credit = create_test_credit_params();

        let ucva = BilateralXvaCalculator::compute_ucva(&ee, &time_grid, &credit);
        let cva = super::super::cva::compute_cva(&ee, &time_grid, &credit);

        assert_relative_eq!(ucva, cva, epsilon = 1e-12);
    }

    #[test]
    fn test_ucva_positive_for_positive_ee() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let time_grid = create_test_time_grid();
        let credit = create_test_credit_params();

        let ucva = BilateralXvaCalculator::compute_ucva(&ee, &time_grid, &credit);
        assert!(ucva > 0.0);
    }

    // -- UDVA tests --

    #[test]
    fn test_udva_matches_compute_dva() {
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let own_credit = create_test_own_credit();

        let udva = BilateralXvaCalculator::compute_udva(&ene, &time_grid, &own_credit);
        let dva = super::super::dva::compute_dva(&ene, &time_grid, &own_credit);

        assert_relative_eq!(udva, dva, epsilon = 1e-12);
    }

    #[test]
    fn test_udva_positive_for_positive_ene() {
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let own_credit = create_test_own_credit();

        let udva = BilateralXvaCalculator::compute_udva(&ene, &time_grid, &own_credit);
        assert!(udva > 0.0);
    }

    // -- Bilateral CVA tests --

    #[test]
    fn test_bcva_less_than_ucva() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let credit = create_test_credit_params();
        let own_credit = create_test_own_credit();

        let result = BilateralXvaCalculator::compute_bilateral_cva(
            &ee,
            &ene,
            &time_grid,
            &credit,
            &own_credit,
        );

        assert!(
            result.bcva < result.ucva,
            "BCVA ({}) should be less than UCVA ({})",
            result.bcva,
            result.ucva
        );
    }

    #[test]
    fn test_bdva_less_than_udva() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let credit = create_test_credit_params();
        let own_credit = create_test_own_credit();

        let result = BilateralXvaCalculator::compute_bilateral_cva(
            &ee,
            &ene,
            &time_grid,
            &credit,
            &own_credit,
        );

        assert!(
            result.bdva < result.udva,
            "BDVA ({}) should be less than UDVA ({})",
            result.bdva,
            result.udva
        );
    }

    #[test]
    fn test_bilateral_symmetry_equal_params_and_profiles() {
        // When own_credit params == ctpy params and EE == ENE, BCVA == BDVA.
        let profile = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let time_grid = create_test_time_grid();

        // Use identical hazard rate and LGD for both parties.
        let credit = CreditParams::new(0.02, 0.4).unwrap();
        let own_credit = OwnCreditParams::new(0.02, 0.4).unwrap();

        let result = BilateralXvaCalculator::compute_bilateral_cva(
            &profile,
            &profile,
            &time_grid,
            &credit,
            &own_credit,
        );

        assert_relative_eq!(result.bcva, result.bdva, epsilon = 1e-10);
    }

    #[test]
    fn test_bilateral_cva_empty_inputs() {
        let credit = create_test_credit_params();
        let own_credit = create_test_own_credit();

        let result =
            BilateralXvaCalculator::compute_bilateral_cva(&[], &[], &[], &credit, &own_credit);

        assert_eq!(result.ucva, 0.0);
        assert_eq!(result.udva, 0.0);
        assert_eq!(result.bcva, 0.0);
        assert_eq!(result.bdva, 0.0);
    }

    #[test]
    fn test_bilateral_cva_zero_exposure() {
        let ee = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let ene = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let time_grid = create_test_time_grid();
        let credit = create_test_credit_params();
        let own_credit = create_test_own_credit();

        let result = BilateralXvaCalculator::compute_bilateral_cva(
            &ee,
            &ene,
            &time_grid,
            &credit,
            &own_credit,
        );

        assert_eq!(result.ucva, 0.0);
        assert_eq!(result.udva, 0.0);
        assert_eq!(result.bcva, 0.0);
        assert_eq!(result.bdva, 0.0);
    }

    // -- FVA with basis tests --

    #[test]
    fn test_fva_with_zero_basis_matches_plain_fva() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let df = create_flat_df(0.05, &time_grid);
        let full_survival = vec![1.0; time_grid.len()];

        let result = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &full_survival,
            None,
        );

        // Compare with existing FVA implementation (survival = 1 everywhere).
        let (fca_ref, fba_ref, _fva_ref) =
            super::super::fva::compute_fva(&ee, &ene, &time_grid, 0.005, 0.003, &df);

        assert_relative_eq!(result.fca, fca_ref, epsilon = 1e-10);
        assert_relative_eq!(result.fba, fba_ref, epsilon = 1e-10);
    }

    #[test]
    fn test_fva_with_positive_basis_increases_fca() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let df = create_flat_df(0.05, &time_grid);
        let full_survival = vec![1.0; time_grid.len()];

        let result_no_basis = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &full_survival,
            None,
        );

        // Positive cross-currency basis (e.g. 20bps).
        let xccy_basis = vec![0.002; time_grid.len()];
        let result_with_basis = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &full_survival,
            Some(&xccy_basis),
        );

        assert!(
            result_with_basis.fca > result_no_basis.fca,
            "FCA with positive basis ({}) should exceed FCA without ({})",
            result_with_basis.fca,
            result_no_basis.fca
        );
    }

    #[test]
    fn test_fva_with_basis_empty_inputs() {
        let result = BilateralXvaCalculator::compute_fva_with_basis(
            &[],
            &[],
            &[],
            0.005,
            0.003,
            &[],
            &[],
            None,
        );

        assert_eq!(result.fca, 0.0);
        assert_eq!(result.fba, 0.0);
        assert_eq!(result.fva, 0.0);
    }

    #[test]
    fn test_fva_with_basis_zero_spreads() {
        let ee = vec![0.0, 100.0, 100.0, 100.0, 100.0];
        let ene = vec![0.0, 50.0, 50.0, 50.0, 50.0];
        let time_grid = create_test_time_grid();
        let df = create_flat_df(0.05, &time_grid);
        let full_survival = vec![1.0; time_grid.len()];

        let result = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.0,
            0.0,
            &df,
            &full_survival,
            None,
        );

        assert_eq!(result.fca, 0.0);
        assert_eq!(result.fba, 0.0);
        assert_eq!(result.fva, 0.0);
    }

    #[test]
    fn test_fva_with_basis_mismatched_lengths() {
        let ee = vec![0.0, 100.0, 100.0];
        let ene = vec![0.0, 50.0, 50.0];
        let time_grid = vec![0.0, 0.5, 1.0];
        let df = create_flat_df(0.05, &time_grid);
        let full_survival = vec![1.0; time_grid.len()];

        // xccy_basis has wrong length.
        let bad_basis = vec![0.001; 5];
        let result = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &full_survival,
            Some(&bad_basis),
        );

        assert_eq!(result.fca, 0.0);
        assert_eq!(result.fba, 0.0);
        assert_eq!(result.fva, 0.0);
    }

    #[test]
    fn test_fva_survival_weighting_reduces_values() {
        let ee = vec![0.0, 100.0, 150.0, 100.0, 50.0];
        let ene = vec![0.0, 50.0, 80.0, 60.0, 30.0];
        let time_grid = create_test_time_grid();
        let df = create_flat_df(0.05, &time_grid);

        let full_survival = vec![1.0; time_grid.len()];
        let partial_survival = vec![1.0, 0.9, 0.8, 0.7, 0.6];

        let result_full = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &full_survival,
            None,
        );

        let result_partial = BilateralXvaCalculator::compute_fva_with_basis(
            &ee,
            &ene,
            &time_grid,
            0.005,
            0.003,
            &df,
            &partial_survival,
            None,
        );

        assert!(
            result_partial.fca < result_full.fca,
            "FCA with partial survival should be less than with full survival"
        );
    }
}

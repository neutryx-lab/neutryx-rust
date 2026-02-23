//! VM-CSA (Variation Margin Credit Support Annex) with asymmetric terms.
//!
//! Models bilateral CSA agreements where self and counterparty have different
//! thresholds, minimum transfer amounts, haircuts, and PV-linked initial
//! margins.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use bon::Builder;

use super::{csa::CallFrequency, CounterPartyError};
use crate::market::Currency;

/// Asymmetric VM-CSA terms for bilateral collateral agreements.
///
/// Unlike [`super::CsaTerms`] which uses symmetric parameters, `VmCsa` models
/// real-world CSAs where each party may have different thresholds, MTAs,
/// haircuts, and PV-linked initial margin schedules.
///
/// # PV-Linked Initial Margin
///
/// The independent amount for each party is computed as:
/// - IA_ctpy = `ia_base_ctpy` + `ia_pv_factor_ctpy` * max(PV, 0)
/// - IA_self = `ia_base_self` + `ia_pv_factor_self` * max(-PV, 0)
///
/// where PV is quoted from *our* perspective (positive = counterparty owes us).
///
/// # Collateral Value
///
/// For a given PV and posting direction:
/// - exposure = max(|PV| - threshold - IA, 0)
/// - collateral = exposure * (1 - haircut), subject to MTA filter
#[derive(Clone, Debug, Builder, serde::Serialize, serde::Deserialize)]
pub struct VmCsa {
    // --- Asymmetric threshold parameters ---
    /// Threshold below which self does not need to post collateral.
    #[builder(default)]
    threshold_self: f64,
    /// Threshold below which counterparty does not need to post collateral.
    #[builder(default)]
    threshold_ctpy: f64,

    // --- Asymmetric MTA parameters ---
    /// Minimum Transfer Amount for self.
    #[builder(default)]
    mta_self: f64,
    /// Minimum Transfer Amount for counterparty.
    #[builder(default)]
    mta_ctpy: f64,

    // --- Asymmetric haircut parameters ---
    /// Haircut applied to collateral posted by self (in [0, 1]).
    #[builder(default)]
    haircut_self: f64,
    /// Haircut applied to collateral posted by counterparty (in [0, 1]).
    #[builder(default)]
    haircut_ctpy: f64,

    // --- PV-linked Initial Margin parameters ---
    /// Base independent amount for self.
    #[builder(default)]
    ia_base_self: f64,
    /// Base independent amount for counterparty.
    #[builder(default)]
    ia_base_ctpy: f64,
    /// PV-linked IA factor for self: IA_self += k * max(-PV, 0).
    #[builder(default)]
    ia_pv_factor_self: f64,
    /// PV-linked IA factor for counterparty: IA_ctpy += k * max(PV, 0).
    #[builder(default)]
    ia_pv_factor_ctpy: f64,

    // --- MPOR / call frequency parameters ---
    /// Margin Period of Risk in business days.
    #[builder(default = 10)]
    mpor_days: u32,
    /// Margin call frequency.
    #[builder(default)]
    call_frequency: CallFrequency,
    /// Margin currency.
    #[builder(default = Currency::USD)]
    margin_currency: Currency,
}

impl VmCsa {
    /// Returns the threshold for self.
    pub fn threshold_self(&self) -> f64 { self.threshold_self }

    /// Returns the threshold for counterparty.
    pub fn threshold_ctpy(&self) -> f64 { self.threshold_ctpy }

    /// Returns the MTA for self.
    pub fn mta_self(&self) -> f64 { self.mta_self }

    /// Returns the MTA for counterparty.
    pub fn mta_ctpy(&self) -> f64 { self.mta_ctpy }

    /// Returns the haircut for collateral posted by self.
    pub fn haircut_self(&self) -> f64 { self.haircut_self }

    /// Returns the haircut for collateral posted by counterparty.
    pub fn haircut_ctpy(&self) -> f64 { self.haircut_ctpy }

    /// Returns the base independent amount for self.
    pub fn ia_base_self(&self) -> f64 { self.ia_base_self }

    /// Returns the base independent amount for counterparty.
    pub fn ia_base_ctpy(&self) -> f64 { self.ia_base_ctpy }

    /// Returns the PV-linked IA factor for self.
    pub fn ia_pv_factor_self(&self) -> f64 { self.ia_pv_factor_self }

    /// Returns the PV-linked IA factor for counterparty.
    pub fn ia_pv_factor_ctpy(&self) -> f64 { self.ia_pv_factor_ctpy }

    /// Returns the Margin Period of Risk in business days.
    pub fn mpor_days(&self) -> u32 { self.mpor_days }

    /// Returns the margin call frequency.
    pub fn call_frequency(&self) -> CallFrequency { self.call_frequency }

    /// Returns the margin currency.
    pub fn margin_currency(&self) -> Currency { self.margin_currency }

    /// Computes the PV-linked initial margin for both parties.
    ///
    /// PV is from our perspective: positive means counterparty owes us.
    ///
    /// Returns `(ia_self, ia_ctpy)` where:
    /// - `ia_self`  = `ia_base_self`  + `ia_pv_factor_self`  * max(-PV, 0)
    /// - `ia_ctpy`  = `ia_base_ctpy`  + `ia_pv_factor_ctpy`  * max( PV, 0)
    pub fn initial_margin(&self, pv: f64) -> (f64, f64) {
        let ia_self = self.ia_base_self + self.ia_pv_factor_self * (-pv).max(0.0);
        let ia_ctpy = self.ia_base_ctpy + self.ia_pv_factor_ctpy * pv.max(0.0);
        (ia_self, ia_ctpy)
    }

    /// Computes the collateral value for a given PV and posting direction.
    ///
    /// When `is_self_posting` is `true`, self posts collateral (self is the
    /// poster). Self posts when PV < 0 (we owe counterparty), using self's
    /// parameters.
    ///
    /// When `is_self_posting` is `false`, counterparty posts collateral.
    /// Counterparty posts when PV > 0 (counterparty owes us), using
    /// counterparty's parameters.
    ///
    /// The calculation is:
    /// 1. Determine the unsigned exposure = |PV|
    /// 2. Subtract the poster's threshold and IA: net = max(exposure -
    ///    threshold - IA, 0)
    /// 3. Apply haircut: collateral = net * (1 - haircut)
    /// 4. Apply MTA filter: if collateral < MTA, return 0
    pub fn collateral_value(&self, pv: f64, is_self_posting: bool) -> f64 {
        let (ia_self, ia_ctpy) = self.initial_margin(pv);

        let (threshold, mta, haircut, ia, exposure) = if is_self_posting {
            // Self posts when PV < 0 (we owe counterparty).
            (
                self.threshold_self,
                self.mta_self,
                self.haircut_self,
                ia_self,
                (-pv).max(0.0),
            )
        } else {
            // Counterparty posts when PV > 0 (counterparty owes us).
            (
                self.threshold_ctpy,
                self.mta_ctpy,
                self.haircut_ctpy,
                ia_ctpy,
                pv.max(0.0),
            )
        };

        let net = (exposure - threshold - ia).max(0.0);
        let after_haircut = net * (1.0 - haircut);

        if after_haircut >= mta {
            after_haircut
        } else {
            0.0
        }
    }

    /// Converts the [`CallFrequency`] to business days.
    ///
    /// - `Daily`   -> 1 business day
    /// - `Weekly`  -> 5 business days
    /// - `Monthly` -> 20 business days
    pub fn call_frequency_days(&self) -> f64 {
        match self.call_frequency {
            CallFrequency::Daily => 1.0,
            CallFrequency::Weekly => 5.0,
            CallFrequency::Monthly => 20.0,
        }
    }

    /// Computes the MPOR Brownian-bridge scaling factor.
    ///
    /// For close-out risk modelling, the scaling is:
    ///
    /// ```text
    /// sqrt(min(call_freq_days, days_from_val_date) / days_from_val_date)
    /// ```
    ///
    /// This captures the idea that for short horizons (less than one call
    /// frequency period), the full time interval matters, while for longer
    /// horizons the call frequency caps the relevant diffusion period.
    ///
    /// Returns 1.0 when `days_from_val_date <= 0`.
    pub fn mpor_scaling(&self, days_from_val_date: f64) -> f64 {
        if days_from_val_date <= 0.0 {
            return 1.0;
        }
        let cf_days = self.call_frequency_days();
        (cf_days.min(days_from_val_date) / days_from_val_date).sqrt()
    }

    /// Validates the CSA parameters.
    ///
    /// Checks that:
    /// - Thresholds are non-negative
    /// - MTAs are non-negative
    /// - Haircuts are in [0, 1]
    /// - IA base amounts are non-negative
    /// - PV factors are non-negative
    /// - MPOR is positive
    pub fn validate(&self) -> Result<(), CounterPartyError> {
        if self.threshold_self < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "threshold_self must be non-negative, got {}",
                self.threshold_self
            )));
        }
        if self.threshold_ctpy < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "threshold_ctpy must be non-negative, got {}",
                self.threshold_ctpy
            )));
        }
        if self.mta_self < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "mta_self must be non-negative, got {}",
                self.mta_self
            )));
        }
        if self.mta_ctpy < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "mta_ctpy must be non-negative, got {}",
                self.mta_ctpy
            )));
        }
        if !(0.0..=1.0).contains(&self.haircut_self) {
            return Err(CounterPartyError::InvalidHaircut(self.haircut_self));
        }
        if !(0.0..=1.0).contains(&self.haircut_ctpy) {
            return Err(CounterPartyError::InvalidHaircut(self.haircut_ctpy));
        }
        if self.ia_base_self < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "ia_base_self must be non-negative, got {}",
                self.ia_base_self
            )));
        }
        if self.ia_base_ctpy < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "ia_base_ctpy must be non-negative, got {}",
                self.ia_base_ctpy
            )));
        }
        if self.ia_pv_factor_self < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "ia_pv_factor_self must be non-negative, got {}",
                self.ia_pv_factor_self
            )));
        }
        if self.ia_pv_factor_ctpy < 0.0 {
            return Err(CounterPartyError::InvalidMarginTerms(format!(
                "ia_pv_factor_ctpy must be non-negative, got {}",
                self.ia_pv_factor_ctpy
            )));
        }
        if self.mpor_days == 0 {
            return Err(CounterPartyError::InvalidMarginTerms(
                "mpor_days must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for VmCsa {
    fn default() -> Self { Self::builder().build() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_csa_defaults() {
        let csa = VmCsa::builder().build();
        assert!((csa.threshold_self()).abs() < f64::EPSILON);
        assert!((csa.threshold_ctpy()).abs() < f64::EPSILON);
        assert!((csa.mta_self()).abs() < f64::EPSILON);
        assert!((csa.mta_ctpy()).abs() < f64::EPSILON);
        assert!((csa.haircut_self()).abs() < f64::EPSILON);
        assert!((csa.haircut_ctpy()).abs() < f64::EPSILON);
        assert_eq!(csa.mpor_days(), 10);
        assert_eq!(csa.call_frequency(), CallFrequency::Daily);
        assert_eq!(csa.margin_currency(), Currency::USD);
    }

    #[test]
    fn test_vm_csa_builder_custom() {
        let csa = VmCsa::builder()
            .threshold_self(500_000.0)
            .threshold_ctpy(1_000_000.0)
            .mta_self(25_000.0)
            .mta_ctpy(50_000.0)
            .haircut_self(0.02)
            .haircut_ctpy(0.05)
            .ia_base_self(100_000.0)
            .ia_base_ctpy(200_000.0)
            .ia_pv_factor_self(0.01)
            .ia_pv_factor_ctpy(0.02)
            .mpor_days(14)
            .call_frequency(CallFrequency::Weekly)
            .margin_currency(Currency::EUR)
            .build();

        assert!((csa.threshold_self() - 500_000.0).abs() < f64::EPSILON);
        assert!((csa.threshold_ctpy() - 1_000_000.0).abs() < f64::EPSILON);
        assert!((csa.mta_self() - 25_000.0).abs() < f64::EPSILON);
        assert!((csa.mta_ctpy() - 50_000.0).abs() < f64::EPSILON);
        assert!((csa.haircut_self() - 0.02).abs() < f64::EPSILON);
        assert!((csa.haircut_ctpy() - 0.05).abs() < f64::EPSILON);
        assert!((csa.ia_base_self() - 100_000.0).abs() < f64::EPSILON);
        assert!((csa.ia_base_ctpy() - 200_000.0).abs() < f64::EPSILON);
        assert!((csa.ia_pv_factor_self() - 0.01).abs() < f64::EPSILON);
        assert!((csa.ia_pv_factor_ctpy() - 0.02).abs() < f64::EPSILON);
        assert_eq!(csa.mpor_days(), 14);
        assert_eq!(csa.call_frequency(), CallFrequency::Weekly);
        assert_eq!(csa.margin_currency(), Currency::EUR);
    }

    #[test]
    fn test_initial_margin_positive_pv() {
        // PV > 0: counterparty owes us.
        // ia_ctpy = 200_000 + 0.02 * max(1_000_000, 0) = 220_000
        // ia_self = 100_000 + 0.01 * max(-1_000_000, 0) = 100_000
        let csa = VmCsa::builder()
            .ia_base_self(100_000.0)
            .ia_base_ctpy(200_000.0)
            .ia_pv_factor_self(0.01)
            .ia_pv_factor_ctpy(0.02)
            .build();

        let (ia_self, ia_ctpy) = csa.initial_margin(1_000_000.0);
        assert!((ia_self - 100_000.0).abs() < f64::EPSILON);
        assert!((ia_ctpy - 220_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_initial_margin_negative_pv() {
        // PV < 0: we owe counterparty.
        // ia_ctpy = 200_000 + 0.02 * max(-500_000, 0) = 200_000
        // ia_self = 100_000 + 0.01 * max(500_000, 0) = 105_000
        let csa = VmCsa::builder()
            .ia_base_self(100_000.0)
            .ia_base_ctpy(200_000.0)
            .ia_pv_factor_self(0.01)
            .ia_pv_factor_ctpy(0.02)
            .build();

        let (ia_self, ia_ctpy) = csa.initial_margin(-500_000.0);
        assert!((ia_self - 105_000.0).abs() < f64::EPSILON);
        assert!((ia_ctpy - 200_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_initial_margin_zero_pv() {
        let csa = VmCsa::builder()
            .ia_base_self(100_000.0)
            .ia_base_ctpy(200_000.0)
            .ia_pv_factor_self(0.01)
            .ia_pv_factor_ctpy(0.02)
            .build();

        let (ia_self, ia_ctpy) = csa.initial_margin(0.0);
        assert!((ia_self - 100_000.0).abs() < f64::EPSILON);
        assert!((ia_ctpy - 200_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_collateral_value_ctpy_posts() {
        // PV = 2_000_000 (counterparty owes us), counterparty posts.
        // threshold_ctpy = 500_000, ia_ctpy = 0
        // exposure = max(2_000_000, 0) = 2_000_000
        // net = max(2_000_000 - 500_000 - 0, 0) = 1_500_000
        // after haircut (5%) = 1_500_000 * 0.95 = 1_425_000
        // mta_ctpy = 50_000 -> 1_425_000 >= 50_000, return 1_425_000
        let csa = VmCsa::builder()
            .threshold_ctpy(500_000.0)
            .mta_ctpy(50_000.0)
            .haircut_ctpy(0.05)
            .build();

        let cv = csa.collateral_value(2_000_000.0, false);
        assert!((cv - 1_425_000.0).abs() < 0.01);
    }

    #[test]
    fn test_collateral_value_self_posts() {
        // PV = -1_000_000 (we owe counterparty), self posts.
        // threshold_self = 200_000, ia_self = 0
        // exposure = max(1_000_000, 0) = 1_000_000
        // net = max(1_000_000 - 200_000 - 0, 0) = 800_000
        // after haircut (2%) = 800_000 * 0.98 = 784_000
        // mta_self = 25_000 -> 784_000 >= 25_000, return 784_000
        let csa = VmCsa::builder()
            .threshold_self(200_000.0)
            .mta_self(25_000.0)
            .haircut_self(0.02)
            .build();

        let cv = csa.collateral_value(-1_000_000.0, true);
        assert!((cv - 784_000.0).abs() < 0.01);
    }

    #[test]
    fn test_collateral_value_below_threshold() {
        // PV = 300_000, threshold_ctpy = 500_000
        // exposure = 300_000, net = max(300_000 - 500_000, 0) = 0
        let csa = VmCsa::builder().threshold_ctpy(500_000.0).build();

        let cv = csa.collateral_value(300_000.0, false);
        assert!(cv.abs() < f64::EPSILON);
    }

    #[test]
    fn test_collateral_value_below_mta() {
        // PV = 510_000, threshold_ctpy = 500_000, mta_ctpy = 50_000
        // exposure = 510_000, net = max(510_000 - 500_000, 0) = 10_000
        // 10_000 < 50_000 (MTA) -> return 0
        let csa = VmCsa::builder()
            .threshold_ctpy(500_000.0)
            .mta_ctpy(50_000.0)
            .build();

        let cv = csa.collateral_value(510_000.0, false);
        assert!(cv.abs() < f64::EPSILON);
    }

    #[test]
    fn test_collateral_value_with_pv_linked_ia() {
        // PV = 2_000_000, counterparty posts.
        // ia_ctpy = 100_000 + 0.05 * max(2_000_000, 0) = 200_000
        // threshold_ctpy = 500_000
        // exposure = 2_000_000, net = max(2_000_000 - 500_000 - 200_000, 0) = 1_300_000
        // no haircut: collateral = 1_300_000
        let csa = VmCsa::builder()
            .threshold_ctpy(500_000.0)
            .ia_base_ctpy(100_000.0)
            .ia_pv_factor_ctpy(0.05)
            .build();

        let cv = csa.collateral_value(2_000_000.0, false);
        assert!((cv - 1_300_000.0).abs() < 0.01);
    }

    #[test]
    fn test_collateral_value_wrong_direction() {
        // PV = 1_000_000 (positive), self posting -> exposure = max(-1_000_000, 0) = 0
        let csa = VmCsa::builder().build();
        let cv = csa.collateral_value(1_000_000.0, true);
        assert!(cv.abs() < f64::EPSILON);

        // PV = -1_000_000 (negative), ctpy posting -> exposure = max(-1_000_000, 0) = 0
        let cv = csa.collateral_value(-1_000_000.0, false);
        assert!(cv.abs() < f64::EPSILON);
    }

    #[test]
    fn test_call_frequency_days() {
        let daily = VmCsa::builder()
            .call_frequency(CallFrequency::Daily)
            .build();
        assert!((daily.call_frequency_days() - 1.0).abs() < f64::EPSILON);

        let weekly = VmCsa::builder()
            .call_frequency(CallFrequency::Weekly)
            .build();
        assert!((weekly.call_frequency_days() - 5.0).abs() < f64::EPSILON);

        let monthly = VmCsa::builder()
            .call_frequency(CallFrequency::Monthly)
            .build();
        assert!((monthly.call_frequency_days() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mpor_scaling_short_horizon() {
        // days_from_val_date < call_freq_days -> scaling = sqrt(days / days) = 1.0
        let csa = VmCsa::builder()
            .call_frequency(CallFrequency::Weekly)
            .build();
        let s = csa.mpor_scaling(3.0);
        // min(5, 3) / 3 = 1.0 -> sqrt(1.0) = 1.0
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mpor_scaling_long_horizon() {
        // days_from_val_date > call_freq_days -> scaling = sqrt(call_freq / days)
        let csa = VmCsa::builder()
            .call_frequency(CallFrequency::Weekly)
            .build();
        let s = csa.mpor_scaling(20.0);
        // min(5, 20) / 20 = 0.25 -> sqrt(0.25) = 0.5
        assert!((s - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mpor_scaling_exact_call_frequency() {
        let csa = VmCsa::builder()
            .call_frequency(CallFrequency::Weekly)
            .build();
        let s = csa.mpor_scaling(5.0);
        // min(5, 5) / 5 = 1.0 -> sqrt(1.0) = 1.0
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mpor_scaling_zero_days() {
        let csa = VmCsa::builder().build();
        let s = csa.mpor_scaling(0.0);
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mpor_scaling_negative_days() {
        let csa = VmCsa::builder().build();
        let s = csa.mpor_scaling(-5.0);
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_valid_csa() {
        let csa = VmCsa::builder()
            .threshold_self(500_000.0)
            .threshold_ctpy(1_000_000.0)
            .mta_self(25_000.0)
            .mta_ctpy(50_000.0)
            .haircut_self(0.02)
            .haircut_ctpy(0.05)
            .ia_base_self(100_000.0)
            .ia_base_ctpy(200_000.0)
            .ia_pv_factor_self(0.01)
            .ia_pv_factor_ctpy(0.02)
            .mpor_days(14)
            .build();

        assert!(csa.validate().is_ok());
    }

    #[test]
    fn test_validate_default_is_valid() {
        let csa = VmCsa::default();
        assert!(csa.validate().is_ok());
    }

    #[test]
    fn test_validate_negative_threshold_self() {
        let csa = VmCsa::builder().threshold_self(-1.0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_negative_threshold_ctpy() {
        let csa = VmCsa::builder().threshold_ctpy(-1.0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_negative_mta_self() {
        let csa = VmCsa::builder().mta_self(-1.0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_negative_mta_ctpy() {
        let csa = VmCsa::builder().mta_ctpy(-1.0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_haircut_out_of_range() {
        let csa = VmCsa::builder().haircut_self(1.5).build();
        assert!(csa.validate().is_err());

        let csa = VmCsa::builder().haircut_ctpy(-0.1).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_haircut_boundaries() {
        let csa = VmCsa::builder().haircut_self(0.0).haircut_ctpy(1.0).build();
        assert!(csa.validate().is_ok());
    }

    #[test]
    fn test_validate_negative_ia_base() {
        let csa = VmCsa::builder().ia_base_self(-100.0).build();
        assert!(csa.validate().is_err());

        let csa = VmCsa::builder().ia_base_ctpy(-100.0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_negative_pv_factor() {
        let csa = VmCsa::builder().ia_pv_factor_self(-0.01).build();
        assert!(csa.validate().is_err());

        let csa = VmCsa::builder().ia_pv_factor_ctpy(-0.01).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_validate_zero_mpor() {
        let csa = VmCsa::builder().mpor_days(0).build();
        assert!(csa.validate().is_err());
    }

    #[test]
    fn test_asymmetric_collateral_scenario() {
        // Realistic scenario: large bank vs small counterparty.
        // Bank (self) has high threshold, counterparty has low threshold.
        let csa = VmCsa::builder()
            .threshold_self(10_000_000.0) // bank barely posts
            .threshold_ctpy(500_000.0)    // counterparty posts more
            .mta_self(100_000.0)
            .mta_ctpy(50_000.0)
            .haircut_self(0.01)
            .haircut_ctpy(0.05)
            .build();

        // PV = 5_000_000 (counterparty owes us)
        // ctpy posts: net = max(5M - 500k, 0) = 4.5M, after 5% haircut = 4_275_000
        let cv_ctpy = csa.collateral_value(5_000_000.0, false);
        assert!((cv_ctpy - 4_275_000.0).abs() < 0.01);

        // PV = -5_000_000 (we owe counterparty)
        // self posts: net = max(5M - 10M, 0) = 0 -> below threshold
        let cv_self = csa.collateral_value(-5_000_000.0, true);
        assert!(cv_self.abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_impl() {
        let csa = VmCsa::default();
        assert_eq!(csa.mpor_days(), 10);
        assert_eq!(csa.margin_currency(), Currency::USD);
        assert_eq!(csa.call_frequency(), CallFrequency::Daily);
    }
}

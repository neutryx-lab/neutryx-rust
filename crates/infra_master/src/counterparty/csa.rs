//! CSA (Credit Support Annex) terms and collateral settings.
//!
//! This module defines types for CSA contractual terms governing collateral
//! exchange between counterparties.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::CounterPartyError;
use crate::Currency;
use std::collections::HashMap;

// ============================================================================
// Enums
// ============================================================================

/// Eligible collateral types for CSA agreements.
///
/// Defines the types of assets that can be posted as collateral.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EligibleCollateral {
    /// Cash in various currencies
    Cash,
    /// Government bonds (e.g., US Treasuries, Bunds, JGBs)
    GovernmentBonds,
    /// Corporate bonds (investment grade)
    CorporateBonds,
    /// Equity securities
    Equity,
    /// Gold bullion
    Gold,
}

/// Collateral segregation type.
///
/// Determines how posted collateral is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SegregationType {
    /// Collateral held in segregated account (protected from bankruptcy)
    #[default]
    Segregated,
    /// Collateral commingled with other assets (may be rehypothecated)
    Commingled,
}

/// Margin call frequency.
///
/// Defines how often margin calls are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CallFrequency {
    /// Daily margin calls (standard for most CSAs)
    #[default]
    Daily,
    /// Weekly margin calls
    Weekly,
    /// Monthly margin calls
    Monthly,
}

// ============================================================================
// CollateralHaircut
// ============================================================================

/// Collateral haircut settings.
///
/// Defines the haircut (discount) applied to collateral value for a specific
/// collateral type and optional currency.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{CollateralHaircut, EligibleCollateral};
/// use infra_master::Currency;
///
/// // 2% haircut on government bonds
/// let haircut = CollateralHaircut::new(EligibleCollateral::GovernmentBonds, 0.02).unwrap();
/// assert!((haircut.haircut_rate() - 0.02).abs() < f64::EPSILON);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollateralHaircut {
    collateral_type: EligibleCollateral,
    currency: Option<Currency>,
    haircut_rate: f64,
}

impl CollateralHaircut {
    /// Creates a new collateral haircut.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidHaircut`] if haircut_rate is not in [0, 1].
    pub fn new(
        collateral_type: EligibleCollateral,
        haircut_rate: f64,
    ) -> Result<Self, CounterPartyError> {
        if !(0.0..=1.0).contains(&haircut_rate) {
            return Err(CounterPartyError::InvalidHaircut(haircut_rate));
        }
        Ok(Self {
            collateral_type,
            currency: None,
            haircut_rate,
        })
    }

    /// Sets the currency for this haircut (for currency-specific haircuts).
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }

    /// Returns the collateral type.
    pub fn collateral_type(&self) -> EligibleCollateral {
        self.collateral_type
    }

    /// Returns the currency if set.
    pub fn currency(&self) -> Option<Currency> {
        self.currency
    }

    /// Returns the haircut rate.
    pub fn haircut_rate(&self) -> f64 {
        self.haircut_rate
    }

    /// Calculates the collateral value after applying haircut.
    pub fn apply_haircut(&self, value: f64) -> f64 {
        value * (1.0 - self.haircut_rate)
    }
}

// ============================================================================
// CsaTerms
// ============================================================================

/// CSA (Credit Support Annex) terms.
///
/// Defines the collateral agreement between counterparties, including thresholds,
/// minimum transfer amounts, margin period of risk, and eligible collateral.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{CsaTerms, EligibleCollateral, CallFrequency};
/// use infra_master::Currency;
///
/// let csa = CsaTerms::builder()
///     .threshold(1_000_000.0)
///     .mta(50_000.0)
///     .mpor_days(10)
///     .margin_currency(Currency::USD)
///     .call_frequency(CallFrequency::Daily)
///     .build();
///
/// assert_eq!(csa.threshold(), 1_000_000.0);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CsaTerms {
    /// Threshold amount (below which no collateral is posted).
    threshold: f64,
    /// Minimum Transfer Amount.
    mta: f64,
    /// Independent Amount (initial margin-like).
    independent_amount: f64,
    /// Margin Period of Risk in business days.
    mpor_days: u32,
    /// Margin currency.
    margin_currency: Currency,
    /// Currency-specific thresholds (overrides base threshold).
    currency_thresholds: HashMap<Currency, f64>,
    /// Eligible collateral types.
    eligible_collateral: Vec<EligibleCollateral>,
    /// Collateral haircuts.
    haircuts: Vec<CollateralHaircut>,
    /// Rehypothecation allowed.
    rehypothecation: bool,
    /// Segregation type.
    segregation: SegregationType,
    /// Margin call frequency.
    call_frequency: CallFrequency,
    /// Dispute threshold.
    dispute_threshold: f64,
}

impl CsaTerms {
    /// Creates a new CSA terms builder.
    pub fn builder() -> CsaTermsBuilder {
        CsaTermsBuilder::default()
    }

    /// Returns the base threshold amount.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Returns the threshold for a specific currency.
    ///
    /// If a currency-specific threshold is set, returns that; otherwise
    /// returns the base threshold.
    pub fn threshold_for_currency(&self, ccy: &Currency) -> f64 {
        self.currency_thresholds
            .get(ccy)
            .copied()
            .unwrap_or(self.threshold)
    }

    /// Returns the Minimum Transfer Amount.
    pub fn mta(&self) -> f64 {
        self.mta
    }

    /// Returns the Independent Amount.
    pub fn independent_amount(&self) -> f64 {
        self.independent_amount
    }

    /// Returns the Margin Period of Risk in business days.
    pub fn mpor_days(&self) -> u32 {
        self.mpor_days
    }

    /// Returns the margin currency.
    pub fn margin_currency(&self) -> Currency {
        self.margin_currency
    }

    /// Returns the currency-specific thresholds.
    pub fn currency_thresholds(&self) -> &HashMap<Currency, f64> {
        &self.currency_thresholds
    }

    /// Returns the eligible collateral types.
    pub fn eligible_collateral(&self) -> &[EligibleCollateral] {
        &self.eligible_collateral
    }

    /// Returns the collateral haircuts.
    pub fn haircuts(&self) -> &[CollateralHaircut] {
        &self.haircuts
    }

    /// Returns whether rehypothecation is allowed.
    pub fn is_rehypothecation_allowed(&self) -> bool {
        self.rehypothecation
    }

    /// Returns the segregation type.
    pub fn segregation(&self) -> SegregationType {
        self.segregation
    }

    /// Returns the margin call frequency.
    pub fn call_frequency(&self) -> CallFrequency {
        self.call_frequency
    }

    /// Returns the dispute threshold.
    pub fn dispute_threshold(&self) -> f64 {
        self.dispute_threshold
    }

    /// Calculates the required margin amount given an exposure.
    ///
    /// Takes into account threshold and MTA.
    pub fn required_margin(&self, exposure: f64, currency: &Currency) -> f64 {
        let threshold = self.threshold_for_currency(currency);
        let excess = (exposure - threshold).max(0.0);
        if excess >= self.mta {
            excess
        } else {
            0.0
        }
    }
}

impl Default for CsaTerms {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            mta: 0.0,
            independent_amount: 0.0,
            mpor_days: 10,
            margin_currency: Currency::USD,
            currency_thresholds: HashMap::new(),
            eligible_collateral: vec![EligibleCollateral::Cash],
            haircuts: Vec::new(),
            rehypothecation: false,
            segregation: SegregationType::default(),
            call_frequency: CallFrequency::default(),
            dispute_threshold: 0.0,
        }
    }
}

// ============================================================================
// CsaTermsBuilder
// ============================================================================

/// Builder for [`CsaTerms`].
#[derive(Clone, Debug, Default)]
pub struct CsaTermsBuilder {
    threshold: f64,
    mta: f64,
    independent_amount: f64,
    mpor_days: u32,
    margin_currency: Option<Currency>,
    currency_thresholds: HashMap<Currency, f64>,
    eligible_collateral: Vec<EligibleCollateral>,
    haircuts: Vec<CollateralHaircut>,
    rehypothecation: bool,
    segregation: SegregationType,
    call_frequency: CallFrequency,
    dispute_threshold: f64,
}

impl CsaTermsBuilder {
    /// Sets the base threshold amount.
    pub fn threshold(mut self, v: f64) -> Self {
        self.threshold = v;
        self
    }

    /// Sets the Minimum Transfer Amount.
    pub fn mta(mut self, v: f64) -> Self {
        self.mta = v;
        self
    }

    /// Sets the Independent Amount.
    pub fn independent_amount(mut self, v: f64) -> Self {
        self.independent_amount = v;
        self
    }

    /// Sets the Margin Period of Risk in business days.
    pub fn mpor_days(mut self, v: u32) -> Self {
        self.mpor_days = v;
        self
    }

    /// Sets the margin currency.
    pub fn margin_currency(mut self, v: Currency) -> Self {
        self.margin_currency = Some(v);
        self
    }

    /// Adds a currency-specific threshold.
    pub fn currency_threshold(mut self, ccy: Currency, threshold: f64) -> Self {
        self.currency_thresholds.insert(ccy, threshold);
        self
    }

    /// Sets the eligible collateral types.
    pub fn eligible_collateral(mut self, v: Vec<EligibleCollateral>) -> Self {
        self.eligible_collateral = v;
        self
    }

    /// Adds a collateral haircut.
    pub fn haircut(mut self, h: CollateralHaircut) -> Self {
        self.haircuts.push(h);
        self
    }

    /// Sets whether rehypothecation is allowed.
    pub fn rehypothecation(mut self, v: bool) -> Self {
        self.rehypothecation = v;
        self
    }

    /// Sets the segregation type.
    pub fn segregation(mut self, v: SegregationType) -> Self {
        self.segregation = v;
        self
    }

    /// Sets the margin call frequency.
    pub fn call_frequency(mut self, v: CallFrequency) -> Self {
        self.call_frequency = v;
        self
    }

    /// Sets the dispute threshold.
    pub fn dispute_threshold(mut self, v: f64) -> Self {
        self.dispute_threshold = v;
        self
    }

    /// Builds the CSA terms.
    pub fn build(self) -> CsaTerms {
        let eligible_collateral = if self.eligible_collateral.is_empty() {
            vec![EligibleCollateral::Cash]
        } else {
            self.eligible_collateral
        };

        CsaTerms {
            threshold: self.threshold,
            mta: self.mta,
            independent_amount: self.independent_amount,
            mpor_days: if self.mpor_days == 0 {
                10
            } else {
                self.mpor_days
            },
            margin_currency: self.margin_currency.unwrap_or(Currency::USD),
            currency_thresholds: self.currency_thresholds,
            eligible_collateral,
            haircuts: self.haircuts,
            rehypothecation: self.rehypothecation,
            segregation: self.segregation,
            call_frequency: self.call_frequency,
            dispute_threshold: self.dispute_threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Enum tests
    // ========================================================================

    #[test]
    fn test_segregation_type_default() {
        assert_eq!(SegregationType::default(), SegregationType::Segregated);
    }

    #[test]
    fn test_call_frequency_default() {
        assert_eq!(CallFrequency::default(), CallFrequency::Daily);
    }

    // ========================================================================
    // CollateralHaircut tests
    // ========================================================================

    #[test]
    fn test_collateral_haircut_valid() {
        let haircut = CollateralHaircut::new(EligibleCollateral::GovernmentBonds, 0.02).unwrap();
        assert_eq!(haircut.collateral_type(), EligibleCollateral::GovernmentBonds);
        assert!((haircut.haircut_rate() - 0.02).abs() < f64::EPSILON);
        assert!(haircut.currency().is_none());
    }

    #[test]
    fn test_collateral_haircut_with_currency() {
        let haircut = CollateralHaircut::new(EligibleCollateral::Cash, 0.0)
            .unwrap()
            .with_currency(Currency::EUR);
        assert_eq!(haircut.currency(), Some(Currency::EUR));
    }

    #[test]
    fn test_collateral_haircut_invalid_negative() {
        let result = CollateralHaircut::new(EligibleCollateral::Cash, -0.1);
        assert!(result.is_err());
        match result {
            Err(CounterPartyError::InvalidHaircut(v)) => assert!((v - (-0.1)).abs() < f64::EPSILON),
            _ => panic!("Expected InvalidHaircut error"),
        }
    }

    #[test]
    fn test_collateral_haircut_invalid_above_one() {
        let result = CollateralHaircut::new(EligibleCollateral::Cash, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_collateral_haircut_boundary_values() {
        // 0% haircut is valid (cash)
        assert!(CollateralHaircut::new(EligibleCollateral::Cash, 0.0).is_ok());
        // 100% haircut is valid (though unusual)
        assert!(CollateralHaircut::new(EligibleCollateral::Equity, 1.0).is_ok());
    }

    #[test]
    fn test_collateral_haircut_apply() {
        let haircut = CollateralHaircut::new(EligibleCollateral::GovernmentBonds, 0.05).unwrap();
        let value = haircut.apply_haircut(1_000_000.0);
        assert!((value - 950_000.0).abs() < 0.01);
    }

    // ========================================================================
    // CsaTerms tests
    // ========================================================================

    #[test]
    fn test_csa_terms_default() {
        let csa = CsaTerms::default();
        assert!((csa.threshold() - 0.0).abs() < f64::EPSILON);
        assert!((csa.mta() - 0.0).abs() < f64::EPSILON);
        assert_eq!(csa.mpor_days(), 10);
        assert_eq!(csa.margin_currency(), Currency::USD);
        assert!(!csa.is_rehypothecation_allowed());
        assert_eq!(csa.segregation(), SegregationType::Segregated);
        assert_eq!(csa.call_frequency(), CallFrequency::Daily);
    }

    #[test]
    fn test_csa_terms_builder() {
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mta(50_000.0)
            .independent_amount(100_000.0)
            .mpor_days(14)
            .margin_currency(Currency::EUR)
            .rehypothecation(true)
            .segregation(SegregationType::Commingled)
            .call_frequency(CallFrequency::Weekly)
            .dispute_threshold(25_000.0)
            .build();

        assert!((csa.threshold() - 1_000_000.0).abs() < f64::EPSILON);
        assert!((csa.mta() - 50_000.0).abs() < f64::EPSILON);
        assert!((csa.independent_amount() - 100_000.0).abs() < f64::EPSILON);
        assert_eq!(csa.mpor_days(), 14);
        assert_eq!(csa.margin_currency(), Currency::EUR);
        assert!(csa.is_rehypothecation_allowed());
        assert_eq!(csa.segregation(), SegregationType::Commingled);
        assert_eq!(csa.call_frequency(), CallFrequency::Weekly);
        assert!((csa.dispute_threshold() - 25_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_csa_terms_currency_threshold() {
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .currency_threshold(Currency::EUR, 500_000.0)
            .currency_threshold(Currency::JPY, 100_000_000.0)
            .build();

        assert!((csa.threshold_for_currency(&Currency::USD) - 1_000_000.0).abs() < f64::EPSILON);
        assert!((csa.threshold_for_currency(&Currency::EUR) - 500_000.0).abs() < f64::EPSILON);
        assert!((csa.threshold_for_currency(&Currency::JPY) - 100_000_000.0).abs() < f64::EPSILON);
        // GBP uses base threshold
        assert!((csa.threshold_for_currency(&Currency::GBP) - 1_000_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_csa_terms_eligible_collateral() {
        let csa = CsaTerms::builder()
            .eligible_collateral(vec![
                EligibleCollateral::Cash,
                EligibleCollateral::GovernmentBonds,
            ])
            .build();

        assert_eq!(csa.eligible_collateral().len(), 2);
        assert!(csa
            .eligible_collateral()
            .contains(&EligibleCollateral::Cash));
        assert!(csa
            .eligible_collateral()
            .contains(&EligibleCollateral::GovernmentBonds));
    }

    #[test]
    fn test_csa_terms_haircuts() {
        let csa = CsaTerms::builder()
            .haircut(CollateralHaircut::new(EligibleCollateral::GovernmentBonds, 0.02).unwrap())
            .haircut(CollateralHaircut::new(EligibleCollateral::CorporateBonds, 0.10).unwrap())
            .build();

        assert_eq!(csa.haircuts().len(), 2);
    }

    #[test]
    fn test_csa_terms_required_margin() {
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mta(50_000.0)
            .build();

        // Exposure below threshold
        assert!((csa.required_margin(500_000.0, &Currency::USD) - 0.0).abs() < f64::EPSILON);

        // Exposure above threshold but excess below MTA
        assert!((csa.required_margin(1_040_000.0, &Currency::USD) - 0.0).abs() < f64::EPSILON);

        // Exposure above threshold with excess above MTA
        let margin = csa.required_margin(1_100_000.0, &Currency::USD);
        assert!((margin - 100_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_csa_terms_builder_defaults() {
        // Empty builder should produce sensible defaults
        let csa = CsaTerms::builder().build();
        assert_eq!(csa.mpor_days(), 10); // Default MPOR
        assert_eq!(csa.margin_currency(), Currency::USD);
        assert_eq!(csa.eligible_collateral().len(), 1);
        assert_eq!(csa.eligible_collateral()[0], EligibleCollateral::Cash);
    }
}

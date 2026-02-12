//! CSA (Credit Support Annex) terms and collateral settings.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use std::collections::HashMap;

use bon::Builder;

use super::CounterPartyError;
use crate::market::Currency;

/// Eligible collateral types for CSA agreements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EligibleCollateral {
    /// Cash in various currencies.
    Cash,
    /// Government bonds (e.g., US Treasuries, Bunds, JGBs).
    GovernmentBonds,
    /// Corporate bonds (investment grade).
    CorporateBonds,
    /// Equity securities.
    Equity,
    /// Gold bullion.
    Gold,
}

/// Collateral segregation type.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum SegregationType {
    /// Collateral held in segregated account (protected from bankruptcy).
    #[default]
    Segregated,
    /// Collateral commingled with other assets (may be rehypothecated).
    Commingled,
}

/// Margin call frequency.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum CallFrequency {
    /// Daily margin calls (standard for most CSAs).
    #[default]
    Daily,
    /// Weekly margin calls.
    Weekly,
    /// Monthly margin calls.
    Monthly,
}

/// Collateral haircut settings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CollateralHaircut {
    collateral_type: EligibleCollateral,
    currency: Option<Currency>,
    haircut_rate: f64,
}

impl CollateralHaircut {
    /// Creates a new collateral haircut.
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
    pub fn collateral_type(&self) -> EligibleCollateral { self.collateral_type }

    /// Returns the currency if set.
    pub fn currency(&self) -> Option<Currency> { self.currency }

    /// Returns the haircut rate.
    pub fn haircut_rate(&self) -> f64 { self.haircut_rate }

    /// Calculates the collateral value after applying haircut.
    pub fn apply_haircut(&self, value: f64) -> f64 { value * (1.0 - self.haircut_rate) }
}

/// CSA (Credit Support Annex) terms.
#[derive(Clone, Debug, Builder, serde::Serialize, serde::Deserialize)]
pub struct CsaTerms {
    /// Threshold amount (below which no collateral is posted).
    #[builder(default)]
    threshold: f64,
    /// Minimum Transfer Amount.
    #[builder(default)]
    mta: f64,
    /// Independent Amount (initial margin-like).
    #[builder(default)]
    independent_amount: f64,
    /// Margin Period of Risk in business days.
    #[builder(default = 10)]
    mpor_days: u32,
    /// Margin currency.
    #[builder(default = Currency::USD)]
    margin_currency: Currency,
    /// Currency-specific thresholds (overrides base threshold).
    #[builder(default)]
    currency_thresholds: HashMap<Currency, f64>,
    /// Eligible collateral types.
    #[builder(default = vec![EligibleCollateral::Cash])]
    eligible_collateral: Vec<EligibleCollateral>,
    /// Collateral haircuts.
    #[builder(default)]
    haircuts: Vec<CollateralHaircut>,
    /// Rehypothecation allowed.
    #[builder(default)]
    rehypothecation: bool,
    /// Segregation type.
    #[builder(default)]
    segregation: SegregationType,
    /// Margin call frequency.
    #[builder(default)]
    call_frequency: CallFrequency,
    /// Dispute threshold.
    #[builder(default)]
    dispute_threshold: f64,
}

impl CsaTerms {
    /// Returns the base threshold amount.
    pub fn threshold(&self) -> f64 { self.threshold }

    /// Returns the threshold for a specific currency.
    pub fn threshold_for_currency(&self, ccy: &Currency) -> f64 {
        self.currency_thresholds
            .get(ccy)
            .copied()
            .unwrap_or(self.threshold)
    }

    /// Returns the Minimum Transfer Amount.
    pub fn mta(&self) -> f64 { self.mta }

    /// Returns the Independent Amount.
    pub fn independent_amount(&self) -> f64 { self.independent_amount }

    /// Returns the Margin Period of Risk in business days.
    pub fn mpor_days(&self) -> u32 { self.mpor_days }

    /// Returns the margin currency.
    pub fn margin_currency(&self) -> Currency { self.margin_currency }

    /// Returns the currency-specific thresholds.
    pub fn currency_thresholds(&self) -> &HashMap<Currency, f64> { &self.currency_thresholds }

    /// Returns the eligible collateral types.
    pub fn eligible_collateral(&self) -> &[EligibleCollateral] { &self.eligible_collateral }

    /// Returns the collateral haircuts.
    pub fn haircuts(&self) -> &[CollateralHaircut] { &self.haircuts }

    /// Returns whether rehypothecation is allowed.
    pub fn is_rehypothecation_allowed(&self) -> bool { self.rehypothecation }

    /// Returns the segregation type.
    pub fn segregation(&self) -> SegregationType { self.segregation }

    /// Returns the margin call frequency.
    pub fn call_frequency(&self) -> CallFrequency { self.call_frequency }

    /// Returns the dispute threshold.
    pub fn dispute_threshold(&self) -> f64 { self.dispute_threshold }

    /// Calculates the required margin amount given an exposure.
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
    fn default() -> Self { Self::builder().build() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collateral_haircut() {
        let haircut = CollateralHaircut::new(EligibleCollateral::GovernmentBonds, 0.05).unwrap();
        assert!((haircut.apply_haircut(1_000_000.0) - 950_000.0).abs() < 0.01);

        assert!(CollateralHaircut::new(EligibleCollateral::Cash, -0.1).is_err());
        assert!(CollateralHaircut::new(EligibleCollateral::Cash, 1.5).is_err());
        assert!(CollateralHaircut::new(EligibleCollateral::Cash, 0.0).is_ok());
        assert!(CollateralHaircut::new(EligibleCollateral::Equity, 1.0).is_ok());
    }

    #[test]
    fn test_csa_terms_builder() {
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mta(50_000.0)
            .mpor_days(14)
            .margin_currency(Currency::EUR)
            .call_frequency(CallFrequency::Weekly)
            .build();

        assert!((csa.threshold() - 1_000_000.0).abs() < f64::EPSILON);
        assert_eq!(csa.mpor_days(), 14);
        assert_eq!(csa.margin_currency(), Currency::EUR);
    }

    #[test]
    fn test_csa_terms_required_margin() {
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mta(50_000.0)
            .build();

        assert!((csa.required_margin(500_000.0, &Currency::USD)).abs() < f64::EPSILON);
        assert!((csa.required_margin(1_040_000.0, &Currency::USD)).abs() < f64::EPSILON);
        assert!(
            (csa.required_margin(1_100_000.0, &Currency::USD) - 100_000.0).abs() < f64::EPSILON
        );
    }

    #[test]
    fn test_csa_terms_defaults() {
        let csa = CsaTerms::builder().build();
        assert_eq!(csa.mpor_days(), 10);
        assert_eq!(csa.margin_currency(), Currency::USD);
        assert_eq!(csa.eligible_collateral(), &[EligibleCollateral::Cash]);
    }
}

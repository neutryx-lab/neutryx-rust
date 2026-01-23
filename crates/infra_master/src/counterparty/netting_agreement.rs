//! Netting agreement structures for legal netting configuration.
//!
//! This module provides comprehensive types for netting agreements including
//! ISDA, GMRA, GMSLA, CSA, and custom agreement types.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use crate::time::Date;
use super::{LegalEntityId, NettingSetId};

// ============================================================================
// NettingAgreementType
// ============================================================================

/// Netting agreement type classification.
///
/// Standard master agreement types used in OTC derivatives markets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NettingAgreementType {
    /// ISDA Master Agreement (derivatives).
    #[default]
    Isda,
    /// Global Master Repurchase Agreement (repos).
    Gmra,
    /// Global Master Securities Lending Agreement.
    Gmsla,
    /// Credit Support Annex (collateral).
    Csa,
    /// Custom or regional agreement.
    Custom,
}

impl std::fmt::Display for NettingAgreementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NettingAgreementType::Isda => "ISDA",
            NettingAgreementType::Gmra => "GMRA",
            NettingAgreementType::Gmsla => "GMSLA",
            NettingAgreementType::Csa => "CSA",
            NettingAgreementType::Custom => "Custom",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// NettingJurisdiction
// ============================================================================

/// Netting enforceability by jurisdiction.
///
/// Tracks whether close-out netting is legally enforceable in various
/// jurisdictions.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingJurisdiction {
    /// Jurisdiction code (ISO 3166-1 alpha-2).
    jurisdiction_code: String,
    /// Jurisdiction name.
    jurisdiction_name: String,
    /// Whether close-out netting is enforceable.
    closeout_enforceable: bool,
    /// Whether payment netting is enforceable.
    payment_netting_enforceable: bool,
    /// Whether multi-branch netting is allowed.
    multi_branch_netting: bool,
    /// Legal opinion reference if available.
    legal_opinion_ref: Option<String>,
}

impl NettingJurisdiction {
    /// Creates a new NettingJurisdiction.
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        closeout_enforceable: bool,
    ) -> Self {
        Self {
            jurisdiction_code: code.into(),
            jurisdiction_name: name.into(),
            closeout_enforceable,
            payment_netting_enforceable: closeout_enforceable,
            multi_branch_netting: false,
            legal_opinion_ref: None,
        }
    }

    /// Sets payment netting enforceability.
    pub fn with_payment_netting(mut self, enforceable: bool) -> Self {
        self.payment_netting_enforceable = enforceable;
        self
    }

    /// Sets multi-branch netting allowance.
    pub fn with_multi_branch_netting(mut self, allowed: bool) -> Self {
        self.multi_branch_netting = allowed;
        self
    }

    /// Sets legal opinion reference.
    pub fn with_legal_opinion(mut self, reference: impl Into<String>) -> Self {
        self.legal_opinion_ref = Some(reference.into());
        self
    }

    /// Returns the jurisdiction code.
    pub fn jurisdiction_code(&self) -> &str { &self.jurisdiction_code }

    /// Returns the jurisdiction name.
    pub fn jurisdiction_name(&self) -> &str { &self.jurisdiction_name }

    /// Returns whether close-out netting is enforceable.
    pub fn is_closeout_enforceable(&self) -> bool { self.closeout_enforceable }

    /// Returns whether payment netting is enforceable.
    pub fn is_payment_netting_enforceable(&self) -> bool { self.payment_netting_enforceable }

    /// Returns whether multi-branch netting is allowed.
    pub fn allows_multi_branch_netting(&self) -> bool { self.multi_branch_netting }

    /// Returns the legal opinion reference.
    pub fn legal_opinion_ref(&self) -> Option<&str> { self.legal_opinion_ref.as_deref() }
}

// ============================================================================
// NettingAgreement
// ============================================================================

/// Netting agreement structure.
///
/// Represents a legal netting agreement between two entities.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingAgreement {
    /// Agreement identifier.
    agreement_id: String,
    /// Agreement type.
    agreement_type: NettingAgreementType,
    /// First party (our entity).
    party_a: LegalEntityId,
    /// Second party (counterparty).
    party_b: LegalEntityId,
    /// Effective date.
    effective_date: Option<Date>,
    /// Termination date.
    termination_date: Option<Date>,
    /// Governing law jurisdiction.
    governing_law: String,
    /// Enforceability in party A's jurisdiction.
    party_a_jurisdiction: Option<NettingJurisdiction>,
    /// Enforceability in party B's jurisdiction.
    party_b_jurisdiction: Option<NettingJurisdiction>,
    /// Associated netting sets.
    netting_set_ids: Vec<NettingSetId>,
}

impl NettingAgreement {
    /// Creates a new builder.
    pub fn builder(
        agreement_id: impl Into<String>,
        party_a: LegalEntityId,
        party_b: LegalEntityId,
    ) -> NettingAgreementBuilder {
        NettingAgreementBuilder::new(agreement_id, party_a, party_b)
    }

    /// Returns the agreement ID.
    pub fn agreement_id(&self) -> &str { &self.agreement_id }

    /// Returns the agreement type.
    pub fn agreement_type(&self) -> NettingAgreementType { self.agreement_type }

    /// Returns party A (our entity).
    pub fn party_a(&self) -> &LegalEntityId { &self.party_a }

    /// Returns party B (counterparty).
    pub fn party_b(&self) -> &LegalEntityId { &self.party_b }

    /// Returns the effective date.
    pub fn effective_date(&self) -> Option<Date> { self.effective_date }

    /// Returns the termination date.
    pub fn termination_date(&self) -> Option<Date> { self.termination_date }

    /// Returns the governing law jurisdiction.
    pub fn governing_law(&self) -> &str { &self.governing_law }

    /// Returns party A's jurisdiction info.
    pub fn party_a_jurisdiction(&self) -> Option<&NettingJurisdiction> {
        self.party_a_jurisdiction.as_ref()
    }

    /// Returns party B's jurisdiction info.
    pub fn party_b_jurisdiction(&self) -> Option<&NettingJurisdiction> {
        self.party_b_jurisdiction.as_ref()
    }

    /// Returns the associated netting sets.
    pub fn netting_set_ids(&self) -> &[NettingSetId] { &self.netting_set_ids }

    /// Returns whether close-out netting is enforceable.
    ///
    /// Requires enforceability in both jurisdictions.
    pub fn is_closeout_enforceable(&self) -> bool {
        let a_ok = self
            .party_a_jurisdiction
            .as_ref()
            .map(|j| j.is_closeout_enforceable())
            .unwrap_or(true);
        let b_ok = self
            .party_b_jurisdiction
            .as_ref()
            .map(|j| j.is_closeout_enforceable())
            .unwrap_or(true);
        a_ok && b_ok
    }
}

// ============================================================================
// NettingAgreementBuilder
// ============================================================================

/// Builder for [`NettingAgreement`].
pub struct NettingAgreementBuilder {
    agreement_id: String,
    agreement_type: NettingAgreementType,
    party_a: LegalEntityId,
    party_b: LegalEntityId,
    effective_date: Option<Date>,
    termination_date: Option<Date>,
    governing_law: String,
    party_a_jurisdiction: Option<NettingJurisdiction>,
    party_b_jurisdiction: Option<NettingJurisdiction>,
    netting_set_ids: Vec<NettingSetId>,
}

impl NettingAgreementBuilder {
    /// Creates a new builder.
    pub fn new(
        agreement_id: impl Into<String>,
        party_a: LegalEntityId,
        party_b: LegalEntityId,
    ) -> Self {
        Self {
            agreement_id: agreement_id.into(),
            agreement_type: NettingAgreementType::default(),
            party_a,
            party_b,
            effective_date: None,
            termination_date: None,
            governing_law: String::from("English"),
            party_a_jurisdiction: None,
            party_b_jurisdiction: None,
            netting_set_ids: Vec::new(),
        }
    }

    /// Sets the agreement type.
    pub fn agreement_type(mut self, t: NettingAgreementType) -> Self {
        self.agreement_type = t;
        self
    }

    /// Sets the effective date.
    pub fn effective_date(mut self, date: Date) -> Self {
        self.effective_date = Some(date);
        self
    }

    /// Sets the termination date.
    pub fn termination_date(mut self, date: Date) -> Self {
        self.termination_date = Some(date);
        self
    }

    /// Sets the governing law.
    pub fn governing_law(mut self, law: impl Into<String>) -> Self {
        self.governing_law = law.into();
        self
    }

    /// Sets party A's jurisdiction info.
    pub fn party_a_jurisdiction(mut self, j: NettingJurisdiction) -> Self {
        self.party_a_jurisdiction = Some(j);
        self
    }

    /// Sets party B's jurisdiction info.
    pub fn party_b_jurisdiction(mut self, j: NettingJurisdiction) -> Self {
        self.party_b_jurisdiction = Some(j);
        self
    }

    /// Adds a netting set ID.
    pub fn add_netting_set(mut self, id: impl Into<NettingSetId>) -> Self {
        let nsi = id.into();
        if !self.netting_set_ids.contains(&nsi) {
            self.netting_set_ids.push(nsi);
        }
        self
    }

    /// Builds the NettingAgreement.
    pub fn build(self) -> NettingAgreement {
        NettingAgreement {
            agreement_id: self.agreement_id,
            agreement_type: self.agreement_type,
            party_a: self.party_a,
            party_b: self.party_b,
            effective_date: self.effective_date,
            termination_date: self.termination_date,
            governing_law: self.governing_law,
            party_a_jurisdiction: self.party_a_jurisdiction,
            party_b_jurisdiction: self.party_b_jurisdiction,
            netting_set_ids: self.netting_set_ids,
        }
    }
}

// ============================================================================
// CloseoutCalculationMethod
// ============================================================================

/// Close-out calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CloseoutCalculationMethod {
    /// Market quotation (2002 ISDA).
    #[default]
    MarketQuotation,
    /// Loss (1992 ISDA).
    Loss,
    /// Close-out amount (2002 ISDA).
    CloseoutAmount,
}

// ============================================================================
// CloseoutNetting
// ============================================================================

/// Close-out netting configuration.
///
/// Defines parameters for close-out netting in case of default.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseoutNetting {
    /// Close-out calculation method.
    calculation_method: CloseoutCalculationMethod,
    /// Notice period in business days.
    notice_period_days: u32,
    /// Cure period in business days.
    cure_period_days: u32,
    /// Grace period in business days.
    grace_period_days: u32,
    /// Whether automatic early termination applies.
    automatic_early_termination: bool,
}

impl CloseoutNetting {
    /// Creates a new CloseoutNetting with default values.
    pub fn new() -> Self { Self::default() }

    /// Sets the calculation method.
    pub fn with_calculation_method(mut self, method: CloseoutCalculationMethod) -> Self {
        self.calculation_method = method;
        self
    }

    /// Sets the notice period.
    pub fn with_notice_period(mut self, days: u32) -> Self {
        self.notice_period_days = days;
        self
    }

    /// Sets the cure period.
    pub fn with_cure_period(mut self, days: u32) -> Self {
        self.cure_period_days = days;
        self
    }

    /// Sets the grace period.
    pub fn with_grace_period(mut self, days: u32) -> Self {
        self.grace_period_days = days;
        self
    }

    /// Sets automatic early termination.
    pub fn with_automatic_early_termination(mut self, enabled: bool) -> Self {
        self.automatic_early_termination = enabled;
        self
    }

    /// Returns the calculation method.
    pub fn calculation_method(&self) -> CloseoutCalculationMethod { self.calculation_method }

    /// Returns the notice period in business days.
    pub fn notice_period_days(&self) -> u32 { self.notice_period_days }

    /// Returns the cure period in business days.
    pub fn cure_period_days(&self) -> u32 { self.cure_period_days }

    /// Returns the grace period in business days.
    pub fn grace_period_days(&self) -> u32 { self.grace_period_days }

    /// Returns whether automatic early termination is enabled.
    pub fn automatic_early_termination(&self) -> bool { self.automatic_early_termination }

    /// Returns the total close-out timeline in business days.
    pub fn total_timeline_days(&self) -> u32 {
        self.notice_period_days + self.cure_period_days + self.grace_period_days
    }
}

impl Default for CloseoutNetting {
    fn default() -> Self {
        Self {
            calculation_method: CloseoutCalculationMethod::default(),
            notice_period_days: 0,
            cure_period_days: 3,
            grace_period_days: 2,
            automatic_early_termination: false,
        }
    }
}

// ============================================================================
// PaymentNettingFrequency
// ============================================================================

/// Payment netting frequency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaymentNettingFrequency {
    /// Daily netting.
    Daily,
    /// Weekly netting.
    #[default]
    Weekly,
    /// Monthly netting.
    Monthly,
    /// On demand.
    OnDemand,
}

// ============================================================================
// PaymentNetting
// ============================================================================

/// Payment netting (operational netting) configuration.
///
/// Defines parameters for operational payment netting to reduce
/// settlement risk and costs.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaymentNetting {
    /// Whether payment netting is enabled.
    enabled: bool,
    /// Netting frequency.
    frequency: PaymentNettingFrequency,
    /// Minimum net amount for settlement.
    minimum_net_amount: f64,
    /// Whether cross-currency netting is allowed.
    cross_currency_netting: bool,
}

impl PaymentNetting {
    /// Creates a new PaymentNetting with default values.
    pub fn new() -> Self { Self::default() }

    /// Enables or disables payment netting.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the netting frequency.
    pub fn with_frequency(mut self, freq: PaymentNettingFrequency) -> Self {
        self.frequency = freq;
        self
    }

    /// Sets the minimum net amount for settlement.
    pub fn with_minimum_net_amount(mut self, amount: f64) -> Self {
        self.minimum_net_amount = amount;
        self
    }

    /// Sets cross-currency netting flag.
    pub fn with_cross_currency_netting(mut self, allowed: bool) -> Self {
        self.cross_currency_netting = allowed;
        self
    }

    /// Returns whether payment netting is enabled.
    pub fn is_enabled(&self) -> bool { self.enabled }

    /// Returns the netting frequency.
    pub fn frequency(&self) -> PaymentNettingFrequency { self.frequency }

    /// Returns the minimum net amount for settlement.
    pub fn minimum_net_amount(&self) -> f64 { self.minimum_net_amount }

    /// Returns whether cross-currency netting is allowed.
    pub fn allows_cross_currency_netting(&self) -> bool { self.cross_currency_netting }
}

impl Default for PaymentNetting {
    fn default() -> Self {
        Self {
            enabled: true,
            frequency: PaymentNettingFrequency::default(),
            minimum_net_amount: 0.0,
            cross_currency_netting: false,
        }
    }
}

// ============================================================================
// CrossProductNettingEligibility
// ============================================================================

/// Cross-product netting eligibility configuration.
///
/// Defines which product types can be netted together within
/// a netting agreement.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrossProductNettingEligibility {
    /// Interest rate products eligible.
    interest_rate: bool,
    /// FX products eligible.
    fx: bool,
    /// Credit products eligible.
    credit: bool,
    /// Equity products eligible.
    equity: bool,
    /// Commodity products eligible.
    commodity: bool,
    /// Custom eligible product types.
    custom_products: Vec<String>,
}

impl CrossProductNettingEligibility {
    /// Creates a new configuration with all products eligible.
    pub fn all() -> Self {
        Self {
            interest_rate: true,
            fx: true,
            credit: true,
            equity: true,
            commodity: true,
            custom_products: Vec::new(),
        }
    }

    /// Creates a new configuration with only specified products.
    pub fn none() -> Self {
        Self {
            interest_rate: false,
            fx: false,
            credit: false,
            equity: false,
            commodity: false,
            custom_products: Vec::new(),
        }
    }

    /// Creates a configuration for interest rate products only.
    pub fn interest_rate_only() -> Self {
        Self {
            interest_rate: true,
            ..Self::none()
        }
    }

    /// Sets interest rate eligibility.
    pub fn with_interest_rate(mut self, eligible: bool) -> Self {
        self.interest_rate = eligible;
        self
    }

    /// Sets FX eligibility.
    pub fn with_fx(mut self, eligible: bool) -> Self {
        self.fx = eligible;
        self
    }

    /// Sets credit eligibility.
    pub fn with_credit(mut self, eligible: bool) -> Self {
        self.credit = eligible;
        self
    }

    /// Sets equity eligibility.
    pub fn with_equity(mut self, eligible: bool) -> Self {
        self.equity = eligible;
        self
    }

    /// Sets commodity eligibility.
    pub fn with_commodity(mut self, eligible: bool) -> Self {
        self.commodity = eligible;
        self
    }

    /// Adds a custom product type.
    pub fn add_custom_product(mut self, product: impl Into<String>) -> Self {
        let p = product.into();
        if !self.custom_products.contains(&p) {
            self.custom_products.push(p);
        }
        self
    }

    /// Returns whether interest rate products are eligible.
    pub fn is_interest_rate_eligible(&self) -> bool { self.interest_rate }

    /// Returns whether FX products are eligible.
    pub fn is_fx_eligible(&self) -> bool { self.fx }

    /// Returns whether credit products are eligible.
    pub fn is_credit_eligible(&self) -> bool { self.credit }

    /// Returns whether equity products are eligible.
    pub fn is_equity_eligible(&self) -> bool { self.equity }

    /// Returns whether commodity products are eligible.
    pub fn is_commodity_eligible(&self) -> bool { self.commodity }

    /// Returns the custom eligible products.
    pub fn custom_products(&self) -> &[String] { &self.custom_products }

    /// Checks if a product type is eligible.
    pub fn is_product_eligible(&self, product_type: &str) -> bool {
        match product_type.to_lowercase().as_str() {
            "irs" | "swap" | "interest_rate" | "rates" => self.interest_rate,
            "fx" | "fxswap" | "fxforward" => self.fx,
            "cds" | "credit" => self.credit,
            "equity" | "eq" => self.equity,
            "commodity" | "comm" => self.commodity,
            other => self.custom_products.iter().any(|p| p.eq_ignore_ascii_case(other)),
        }
    }
}

impl Default for CrossProductNettingEligibility {
    fn default() -> Self { Self::all() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // NettingAgreementType tests
    // ========================================================================

    #[test]
    fn test_netting_agreement_type_default() {
        assert_eq!(NettingAgreementType::default(), NettingAgreementType::Isda);
    }

    #[test]
    fn test_netting_agreement_type_display() {
        assert_eq!(format!("{}", NettingAgreementType::Isda), "ISDA");
        assert_eq!(format!("{}", NettingAgreementType::Gmra), "GMRA");
        assert_eq!(format!("{}", NettingAgreementType::Gmsla), "GMSLA");
        assert_eq!(format!("{}", NettingAgreementType::Csa), "CSA");
        assert_eq!(format!("{}", NettingAgreementType::Custom), "Custom");
    }

    // ========================================================================
    // NettingJurisdiction tests
    // ========================================================================

    #[test]
    fn test_netting_jurisdiction_basic() {
        let j = NettingJurisdiction::new("GB", "United Kingdom", true);
        assert_eq!(j.jurisdiction_code(), "GB");
        assert_eq!(j.jurisdiction_name(), "United Kingdom");
        assert!(j.is_closeout_enforceable());
    }

    #[test]
    fn test_netting_jurisdiction_builder() {
        let j = NettingJurisdiction::new("DE", "Germany", true)
            .with_payment_netting(true)
            .with_multi_branch_netting(true)
            .with_legal_opinion("ISDA-2020-DE");

        assert!(j.is_payment_netting_enforceable());
        assert!(j.allows_multi_branch_netting());
        assert_eq!(j.legal_opinion_ref(), Some("ISDA-2020-DE"));
    }

    // ========================================================================
    // NettingAgreement tests
    // ========================================================================

    #[test]
    fn test_netting_agreement_builder() {
        let party_a = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let party_b = LegalEntityId::new_unchecked("213800NCKZAPQ56PAB66");

        let agreement = NettingAgreement::builder("ISDA-001", party_a.clone(), party_b.clone())
            .agreement_type(NettingAgreementType::Isda)
            .governing_law("English")
            .add_netting_set("NS001")
            .add_netting_set("NS002")
            .build();

        assert_eq!(agreement.agreement_id(), "ISDA-001");
        assert_eq!(agreement.agreement_type(), NettingAgreementType::Isda);
        assert_eq!(agreement.governing_law(), "English");
        assert_eq!(agreement.netting_set_ids().len(), 2);
    }

    #[test]
    fn test_netting_agreement_enforceability() {
        let party_a = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let party_b = LegalEntityId::new_unchecked("213800NCKZAPQ56PAB66");
        let j_gb = NettingJurisdiction::new("GB", "UK", true);
        let j_no = NettingJurisdiction::new("XX", "Non-enforceable", false);

        // Both enforceable
        let agreement1 = NettingAgreement::builder("A1", party_a.clone(), party_b.clone())
            .party_a_jurisdiction(j_gb.clone())
            .party_b_jurisdiction(j_gb.clone())
            .build();
        assert!(agreement1.is_closeout_enforceable());

        // One not enforceable
        let agreement2 = NettingAgreement::builder("A2", party_a, party_b)
            .party_a_jurisdiction(j_gb)
            .party_b_jurisdiction(j_no)
            .build();
        assert!(!agreement2.is_closeout_enforceable());
    }

    // ========================================================================
    // CloseoutNetting tests
    // ========================================================================

    #[test]
    fn test_closeout_netting_default() {
        let c = CloseoutNetting::default();
        assert_eq!(c.calculation_method(), CloseoutCalculationMethod::MarketQuotation);
        assert_eq!(c.notice_period_days(), 0);
        assert_eq!(c.cure_period_days(), 3);
        assert_eq!(c.grace_period_days(), 2);
        assert!(!c.automatic_early_termination());
    }

    #[test]
    fn test_closeout_netting_builder() {
        let c = CloseoutNetting::new()
            .with_calculation_method(CloseoutCalculationMethod::CloseoutAmount)
            .with_notice_period(1)
            .with_cure_period(5)
            .with_grace_period(3)
            .with_automatic_early_termination(true);

        assert_eq!(c.calculation_method(), CloseoutCalculationMethod::CloseoutAmount);
        assert_eq!(c.total_timeline_days(), 9); // 1 + 5 + 3
        assert!(c.automatic_early_termination());
    }

    // ========================================================================
    // PaymentNetting tests
    // ========================================================================

    #[test]
    fn test_payment_netting_default() {
        let p = PaymentNetting::default();
        assert!(p.is_enabled());
        assert_eq!(p.frequency(), PaymentNettingFrequency::Weekly);
        assert!(p.minimum_net_amount().abs() < f64::EPSILON);
        assert!(!p.allows_cross_currency_netting());
    }

    #[test]
    fn test_payment_netting_builder() {
        let p = PaymentNetting::new()
            .with_enabled(true)
            .with_frequency(PaymentNettingFrequency::Daily)
            .with_minimum_net_amount(10_000.0)
            .with_cross_currency_netting(true);

        assert!(p.is_enabled());
        assert_eq!(p.frequency(), PaymentNettingFrequency::Daily);
        assert!((p.minimum_net_amount() - 10_000.0).abs() < f64::EPSILON);
        assert!(p.allows_cross_currency_netting());
    }

    // ========================================================================
    // CrossProductNettingEligibility tests
    // ========================================================================

    #[test]
    fn test_cross_product_eligibility_all() {
        let e = CrossProductNettingEligibility::all();
        assert!(e.is_interest_rate_eligible());
        assert!(e.is_fx_eligible());
        assert!(e.is_credit_eligible());
        assert!(e.is_equity_eligible());
        assert!(e.is_commodity_eligible());
    }

    #[test]
    fn test_cross_product_eligibility_none() {
        let e = CrossProductNettingEligibility::none();
        assert!(!e.is_interest_rate_eligible());
        assert!(!e.is_fx_eligible());
        assert!(!e.is_credit_eligible());
        assert!(!e.is_equity_eligible());
        assert!(!e.is_commodity_eligible());
    }

    #[test]
    fn test_cross_product_eligibility_interest_rate_only() {
        let e = CrossProductNettingEligibility::interest_rate_only();
        assert!(e.is_interest_rate_eligible());
        assert!(!e.is_fx_eligible());
    }

    #[test]
    fn test_cross_product_eligibility_check() {
        let e = CrossProductNettingEligibility::none()
            .with_interest_rate(true)
            .with_fx(true);

        assert!(e.is_product_eligible("IRS"));
        assert!(e.is_product_eligible("swap"));
        assert!(e.is_product_eligible("FX"));
        assert!(!e.is_product_eligible("CDS"));
        assert!(!e.is_product_eligible("Equity"));
    }

    #[test]
    fn test_cross_product_eligibility_custom() {
        let e = CrossProductNettingEligibility::none()
            .add_custom_product("EXOTIC");

        assert!(!e.is_product_eligible("IRS"));
        assert!(e.is_product_eligible("EXOTIC"));
        assert!(e.is_product_eligible("exotic")); // Case-insensitive
    }
}

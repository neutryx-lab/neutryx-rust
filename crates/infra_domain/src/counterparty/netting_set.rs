//! NettingSet and ExposureConfig definitions.
//!
//! This module defines the NettingSet entity for grouping trades for netting
//! purposes, and ExposureConfig for exposure calculation parameters.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::{
    Ccp, CcpId, CounterPartyError, CounterPartyId, CrossBookNettingAgreementId, CsaTerms,
    LegalEntityId, MarginTerms,
};
use crate::{ids::BookId, time::Date};

// ============================================================================
// NettingType
// ============================================================================

/// Netting type classification.
///
/// Defines how a netting set is settled and its risk treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NettingType {
    /// Bilateral (OTC) - direct with counterparty
    #[default]
    Bilateral,
    /// Cleared via CCP as a clearing member
    ClearedCcp,
    /// Cleared via CCP as a client of a clearing member
    ClearedClient,
}

impl NettingType {
    /// Returns whether this is a cleared transaction.
    pub fn is_cleared(&self) -> bool {
        matches!(self, NettingType::ClearedCcp | NettingType::ClearedClient)
    }
}

impl std::fmt::Display for NettingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NettingType::Bilateral => "Bilateral",
            NettingType::ClearedCcp => "Cleared (CCP)",
            NettingType::ClearedClient => "Cleared (Client)",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// ExposureConfig
// ============================================================================

/// Exposure calculation configuration.
///
/// Defines parameters for exposure calculations including time grids,
/// confidence levels, and netting/collateral flags.
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::ExposureConfig;
///
/// let config = ExposureConfig::new()
///     .with_time_grid(vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0])
///     .with_pfe_confidence(0.99);
///
/// assert_eq!(config.pfe_confidence(), 0.99);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExposureConfig {
    /// Time grid for exposure calculation (in years)
    time_grid_years: Vec<f64>,
    /// PFE (Potential Future Exposure) confidence level (e.g., 0.95 or 0.99)
    pfe_confidence: f64,
    /// Regulatory maturity for EEPE calculation (typically 1 year)
    regulatory_maturity: f64,
    /// Whether to apply netting
    apply_netting: bool,
    /// Whether to apply collateral effects
    apply_collateral: bool,
}

impl ExposureConfig {
    /// Creates a new ExposureConfig with default values.
    pub fn new() -> Self { Self::default() }

    /// Sets the time grid for exposure calculation.
    pub fn with_time_grid(mut self, grid: Vec<f64>) -> Self {
        self.time_grid_years = grid;
        self
    }

    /// Sets the PFE confidence level.
    ///
    /// Value is clamped to [0, 1].
    pub fn with_pfe_confidence(mut self, confidence: f64) -> Self {
        self.pfe_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Sets the regulatory maturity for EEPE calculation.
    pub fn with_regulatory_maturity(mut self, maturity: f64) -> Self {
        self.regulatory_maturity = maturity;
        self
    }

    /// Sets whether to apply netting.
    pub fn with_apply_netting(mut self, apply: bool) -> Self {
        self.apply_netting = apply;
        self
    }

    /// Sets whether to apply collateral effects.
    pub fn with_apply_collateral(mut self, apply: bool) -> Self {
        self.apply_collateral = apply;
        self
    }

    /// Returns the time grid (in years).
    pub fn time_grid(&self) -> &[f64] { &self.time_grid_years }

    /// Returns the PFE confidence level.
    pub fn pfe_confidence(&self) -> f64 { self.pfe_confidence }

    /// Returns the regulatory maturity (in years).
    pub fn regulatory_maturity(&self) -> f64 { self.regulatory_maturity }

    /// Returns whether netting should be applied.
    pub fn apply_netting(&self) -> bool { self.apply_netting }

    /// Returns whether collateral effects should be applied.
    pub fn apply_collateral(&self) -> bool { self.apply_collateral }
}

impl Default for ExposureConfig {
    fn default() -> Self {
        Self {
            time_grid_years: vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0],
            pfe_confidence: 0.95,
            regulatory_maturity: 1.0,
            apply_netting: true,
            apply_collateral: true,
        }
    }
}

// ============================================================================
// PfeConfidenceLevel
// ============================================================================

/// PFE (Potential Future Exposure) confidence level.
///
/// Standard confidence levels used in regulatory and risk management contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PfeConfidenceLevel {
    /// 95% confidence level (regulatory standard).
    #[default]
    Q95,
    /// 97.5% confidence level.
    Q97_5,
    /// 99% confidence level.
    Q99,
    /// Custom confidence level with explicit value.
    Custom(u8),
}

impl PfeConfidenceLevel {
    /// Returns the confidence level as a decimal (e.g., 0.95 for Q95).
    pub fn as_f64(&self) -> f64 {
        match self {
            PfeConfidenceLevel::Q95 => 0.95,
            PfeConfidenceLevel::Q97_5 => 0.975,
            PfeConfidenceLevel::Q99 => 0.99,
            PfeConfidenceLevel::Custom(p) => f64::from(*p) / 100.0,
        }
    }

    /// Creates a PfeConfidenceLevel from a decimal value.
    ///
    /// Values are rounded to the nearest standard level if within 0.5%,
    /// otherwise a Custom level is returned.
    pub fn from_f64(value: f64) -> Self {
        let clamped = value.clamp(0.0, 1.0);
        if (clamped - 0.95).abs() < 0.005 {
            PfeConfidenceLevel::Q95
        } else if (clamped - 0.975).abs() < 0.005 {
            PfeConfidenceLevel::Q97_5
        } else if (clamped - 0.99).abs() < 0.005 {
            PfeConfidenceLevel::Q99
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            PfeConfidenceLevel::Custom((clamped * 100.0).round() as u8)
        }
    }
}

// ============================================================================
// ExposureAggregation
// ============================================================================

/// Exposure aggregation method.
///
/// Defines how exposure is aggregated across trades, netting sets, and
/// counterparties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExposureAggregation {
    /// Gross exposure (no netting applied).
    Gross,
    /// Net exposure within each netting set (standard).
    #[default]
    NetWithinNettingSet,
    /// Net exposure within each counterparty (cross-netting set).
    NetWithinCounterparty,
}

impl ExposureAggregation {
    /// Returns true if netting is applied at any level.
    pub fn applies_netting(&self) -> bool { !matches!(self, ExposureAggregation::Gross) }
}

// ============================================================================
// MporConfig
// ============================================================================

/// Margin Period of Risk (MPOR) configuration.
///
/// Configures the MPOR calculation parameters based on regulatory requirements.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MporConfig {
    /// Base MPOR in business days (default: 10 for bilateral OTC).
    pub base_mpor_days: u32,
    /// Grace period for disputes in business days.
    pub dispute_grace_days: u32,
    /// Whether to apply extended MPOR for illiquid trades.
    pub apply_illiquidity_extension: bool,
    /// Illiquidity threshold for MPOR extension (days).
    pub illiquidity_threshold_days: u32,
}

impl Default for MporConfig {
    fn default() -> Self {
        Self {
            base_mpor_days: 10,
            dispute_grace_days: 0,
            apply_illiquidity_extension: false,
            illiquidity_threshold_days: 20,
        }
    }
}

// ============================================================================
// CollateralizedExposureConfig
// ============================================================================

/// Configuration for collateralized exposure calculations.
///
/// Defines parameters for EEPE (Effective Expected Positive Exposure) and
/// collateral-adjusted exposure calculations.
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::CollateralizedExposureConfig;
///
/// let config = CollateralizedExposureConfig::new()
///     .with_eepe_horizon_years(5.0)
///     .with_effective_maturity_years(1.0)
///     .with_collateral_volatility(0.15);
///
/// assert!((config.eepe_horizon_years() - 5.0).abs() < f64::EPSILON);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollateralizedExposureConfig {
    /// EEPE calculation horizon in years (regulatory: 5 years).
    eepe_horizon_years: f64,
    /// Effective maturity for regulatory purposes in years (default: 1 year).
    effective_maturity_years: f64,
    /// Collateral value volatility for haircut calculation.
    collateral_volatility: f64,
    /// Whether to apply close-out risk adjustment.
    apply_closeout_risk: bool,
    /// MPOR configuration.
    mpor_config: MporConfig,
}

impl CollateralizedExposureConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self { Self::default() }

    /// Sets the EEPE horizon in years.
    pub fn with_eepe_horizon_years(mut self, years: f64) -> Self {
        self.eepe_horizon_years = years;
        self
    }

    /// Sets the effective maturity in years.
    pub fn with_effective_maturity_years(mut self, years: f64) -> Self {
        self.effective_maturity_years = years;
        self
    }

    /// Sets the collateral volatility.
    pub fn with_collateral_volatility(mut self, vol: f64) -> Self {
        self.collateral_volatility = vol.clamp(0.0, 1.0);
        self
    }

    /// Sets whether to apply close-out risk adjustment.
    pub fn with_apply_closeout_risk(mut self, apply: bool) -> Self {
        self.apply_closeout_risk = apply;
        self
    }

    /// Sets the MPOR configuration.
    pub fn with_mpor_config(mut self, config: MporConfig) -> Self {
        self.mpor_config = config;
        self
    }

    /// Returns the EEPE horizon in years.
    pub fn eepe_horizon_years(&self) -> f64 { self.eepe_horizon_years }

    /// Returns the effective maturity in years.
    pub fn effective_maturity_years(&self) -> f64 { self.effective_maturity_years }

    /// Returns the collateral volatility.
    pub fn collateral_volatility(&self) -> f64 { self.collateral_volatility }

    /// Returns whether close-out risk is applied.
    pub fn apply_closeout_risk(&self) -> bool { self.apply_closeout_risk }

    /// Returns the MPOR configuration.
    pub fn mpor_config(&self) -> &MporConfig { &self.mpor_config }
}

impl Default for CollateralizedExposureConfig {
    fn default() -> Self {
        Self {
            eepe_horizon_years: 5.0,
            effective_maturity_years: 1.0,
            collateral_volatility: 0.0,
            apply_closeout_risk: true,
            mpor_config: MporConfig::default(),
        }
    }
}

// ============================================================================
// NettingSet
// ============================================================================

/// NettingSet entity.
///
/// Represents a collection of trades that can be legally netted against each
/// other in the event of default. Each netting set belongs to a single
/// counterparty and may have associated CSA and margin terms.
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::{NettingSet, NettingType, CsaTerms, MarginTerms};
///
/// let ns = NettingSet::builder("NS001", "CP001")
///     .netting_type(NettingType::Bilateral)
///     .closeout_netting(true)
///     .csa_terms(CsaTerms::builder().threshold(1_000_000.0).build())
///     .build()
///     .unwrap();
///
/// assert_eq!(ns.id().as_str(), "NS001");
/// assert!(ns.has_closeout_netting());
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::struct_field_names)]
pub struct NettingSet {
    netting_set_id: NettingSetId,
    counterparty_id: CounterPartyId,
    legal_entity_id: Option<LegalEntityId>,
    netting_type: NettingType,
    closeout_netting: bool,
    csa_terms: Option<CsaTerms>,
    margin_terms: Option<MarginTerms>,
    ccp_id: Option<CcpId>,
    exposure_config: Option<ExposureConfig>,
    /// Book IDs allowed for cross-book netting within this set.
    /// If empty, all books are allowed (single-book netting assumed).
    book_ids: Vec<BookId>,
}

impl NettingSet {
    /// Creates a new NettingSet builder.
    pub fn builder(
        id: impl Into<NettingSetId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> NettingSetBuilder {
        NettingSetBuilder::new(id, counterparty_id)
    }

    /// Returns the netting set ID.
    pub fn id(&self) -> &NettingSetId { &self.netting_set_id }

    /// Returns the counterparty ID.
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the legal entity ID if set.
    pub fn legal_entity_id(&self) -> Option<&LegalEntityId> { self.legal_entity_id.as_ref() }

    /// Returns the netting type.
    pub fn netting_type(&self) -> NettingType { self.netting_type }

    /// Returns whether close-out netting applies.
    pub fn has_closeout_netting(&self) -> bool { self.closeout_netting }

    /// Returns the CSA terms if set.
    pub fn csa_terms(&self) -> Option<&CsaTerms> { self.csa_terms.as_ref() }

    /// Returns the margin terms if set.
    pub fn margin_terms(&self) -> Option<&MarginTerms> { self.margin_terms.as_ref() }

    /// Returns the CCP ID if this is a cleared transaction.
    pub fn ccp_id(&self) -> Option<&CcpId> { self.ccp_id.as_ref() }

    /// Returns the exposure configuration if set.
    pub fn exposure_config(&self) -> Option<&ExposureConfig> { self.exposure_config.as_ref() }

    /// Returns the book IDs allowed for cross-book netting.
    ///
    /// If empty, all books are allowed (single-book netting assumed).
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns true if cross-book netting is enabled (multiple books
    /// specified).
    pub fn allows_cross_book_netting(&self) -> bool { self.book_ids.len() > 1 }

    /// Returns true if the specified book is allowed in this netting set.
    ///
    /// If no books are explicitly configured, returns true (all books allowed).
    pub fn allows_book(&self, book_id: &BookId) -> bool {
        self.book_ids.is_empty() || self.book_ids.contains(book_id)
    }

    /// Returns whether this is a cleared transaction.
    pub fn is_cleared(&self) -> bool { self.netting_type.is_cleared() }

    /// Returns whether this has collateral (CSA terms set).
    pub fn is_collateralised(&self) -> bool { self.csa_terms.is_some() }

    /// Returns the MPOR (Margin Period of Risk) in business days.
    ///
    /// For cleared transactions, returns the CCP default (5 days).
    /// For bilateral transactions, returns the CSA MPOR or 10 days default.
    pub fn mpor_days(&self) -> u32 {
        match self.netting_type {
            NettingType::ClearedCcp | NettingType::ClearedClient => Ccp::CLEARED_MPOR_DAYS,
            NettingType::Bilateral => self.csa_terms.as_ref().map(|c| c.mpor_days()).unwrap_or(10),
        }
    }
}

// ============================================================================
// NettingSetId
// ============================================================================

/// Type-safe NettingSet identifier.
///
/// Re-exported from ids module for convenience.
pub use super::ids::NettingSetId;

// ============================================================================
// NettingSetBuilder
// ============================================================================

/// Builder for [`NettingSet`].
pub struct NettingSetBuilder {
    netting_set_id: NettingSetId,
    counterparty_id: CounterPartyId,
    legal_entity_id: Option<LegalEntityId>,
    netting_type: NettingType,
    closeout_netting: bool,
    csa_terms: Option<CsaTerms>,
    margin_terms: Option<MarginTerms>,
    ccp_id: Option<CcpId>,
    exposure_config: Option<ExposureConfig>,
    book_ids: Vec<BookId>,
}

impl NettingSetBuilder {
    /// Creates a new builder with required fields.
    pub fn new(id: impl Into<NettingSetId>, counterparty_id: impl Into<CounterPartyId>) -> Self {
        Self {
            netting_set_id: id.into(),
            counterparty_id: counterparty_id.into(),
            legal_entity_id: None,
            netting_type: NettingType::default(),
            closeout_netting: true,
            csa_terms: None,
            margin_terms: None,
            ccp_id: None,
            exposure_config: None,
            book_ids: Vec::new(),
        }
    }

    /// Sets the legal entity ID.
    pub fn legal_entity_id(mut self, lei: LegalEntityId) -> Self {
        self.legal_entity_id = Some(lei);
        self
    }

    /// Sets the netting type.
    pub fn netting_type(mut self, t: NettingType) -> Self {
        self.netting_type = t;
        self
    }

    /// Sets whether close-out netting applies.
    pub fn closeout_netting(mut self, v: bool) -> Self {
        self.closeout_netting = v;
        self
    }

    /// Sets the CSA terms.
    pub fn csa_terms(mut self, terms: CsaTerms) -> Self {
        self.csa_terms = Some(terms);
        self
    }

    /// Sets the margin terms.
    pub fn margin_terms(mut self, terms: MarginTerms) -> Self {
        self.margin_terms = Some(terms);
        self
    }

    /// Sets the CCP ID (for cleared transactions).
    pub fn ccp_id(mut self, id: impl Into<CcpId>) -> Self {
        self.ccp_id = Some(id.into());
        self
    }

    /// Sets the exposure configuration.
    pub fn exposure_config(mut self, config: ExposureConfig) -> Self {
        self.exposure_config = Some(config);
        self
    }

    /// Adds a book ID to the allowed books list.
    pub fn add_book(mut self, book_id: impl Into<BookId>) -> Self {
        let id = book_id.into();
        if !self.book_ids.contains(&id) {
            self.book_ids.push(id);
        }
        self
    }

    /// Sets the allowed book IDs for cross-book netting.
    pub fn book_ids(mut self, book_ids: impl IntoIterator<Item = impl Into<BookId>>) -> Self {
        self.book_ids = book_ids.into_iter().map(Into::into).collect();
        self
    }

    /// Builds the NettingSet.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError`] if validation fails.
    pub fn build(self) -> Result<NettingSet, CounterPartyError> {
        // Validation: cleared transactions should have CCP ID
        // (warning only - not enforced as error for flexibility)

        Ok(NettingSet {
            netting_set_id: self.netting_set_id,
            counterparty_id: self.counterparty_id,
            legal_entity_id: self.legal_entity_id,
            netting_type: self.netting_type,
            closeout_netting: self.closeout_netting,
            csa_terms: self.csa_terms,
            margin_terms: self.margin_terms,
            ccp_id: self.ccp_id,
            exposure_config: self.exposure_config,
            book_ids: self.book_ids,
        })
    }
}

// ============================================================================
// CrossBookNettingAgreement
// ============================================================================

/// Cross-book netting agreement.
///
/// Defines an explicit agreement that allows netting of trades across
/// multiple books. This is required when cross-book netting is enabled
/// for a netting set.
///
/// # Requirements
///
/// According to the design requirements (4.3, 4.5):
/// - Cross-book netting requires explicit configuration
/// - Must specify at least 2 books
/// - Optionally restricts eligible product types
///
/// # Examples
///
/// ```
/// use infra_domain::counterparty::{CrossBookNettingAgreement, CounterPartyError};
/// use infra_domain::ids::BookId;
///
/// let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
///     .add_book("B001")
///     .add_book("B002")
///     .add_eligible_product("IRS")
///     .build()
///     .unwrap();
///
/// assert!(agreement.is_book_eligible(&BookId::new("B001")));
/// assert!(agreement.is_product_eligible("IRS"));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrossBookNettingAgreement {
    /// Unique identifier for this agreement.
    id: CrossBookNettingAgreementId,
    /// Counterparty associated with this agreement.
    counterparty_id: CounterPartyId,
    /// Books included in this cross-book netting agreement.
    book_ids: Vec<BookId>,
    /// Eligible product types (empty = all products eligible).
    eligible_products: Vec<String>,
    /// Effective date of the agreement.
    effective_date: Option<Date>,
    /// Termination date of the agreement.
    termination_date: Option<Date>,
    /// Description of the agreement.
    description: Option<String>,
}

impl CrossBookNettingAgreement {
    /// Creates a new builder.
    pub fn builder(
        id: impl Into<CrossBookNettingAgreementId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> CrossBookNettingAgreementBuilder {
        CrossBookNettingAgreementBuilder::new(id, counterparty_id)
    }

    /// Returns the agreement ID.
    #[inline]
    pub fn id(&self) -> &CrossBookNettingAgreementId { &self.id }

    /// Returns the counterparty ID.
    #[inline]
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the book IDs in this agreement.
    #[inline]
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns the eligible products (empty = all products eligible).
    #[inline]
    pub fn eligible_products(&self) -> &[String] { &self.eligible_products }

    /// Returns the effective date.
    #[inline]
    pub fn effective_date(&self) -> Option<Date> { self.effective_date }

    /// Returns the termination date.
    #[inline]
    pub fn termination_date(&self) -> Option<Date> { self.termination_date }

    /// Returns the description.
    #[inline]
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }

    /// Returns true if the specified book is included in this agreement.
    pub fn is_book_eligible(&self, book_id: &BookId) -> bool { self.book_ids.contains(book_id) }

    /// Returns true if the specified product is eligible for cross-book
    /// netting.
    ///
    /// If no products are specified, all products are eligible.
    pub fn is_product_eligible(&self, product: &str) -> bool {
        self.eligible_products.is_empty() || self.eligible_products.iter().any(|p| p == product)
    }

    /// Returns the number of books in this agreement.
    #[inline]
    pub fn book_count(&self) -> usize { self.book_ids.len() }
}

// ============================================================================
// CrossBookNettingAgreementBuilder
// ============================================================================

/// Builder for [`CrossBookNettingAgreement`].
pub struct CrossBookNettingAgreementBuilder {
    id: CrossBookNettingAgreementId,
    counterparty_id: CounterPartyId,
    book_ids: Vec<BookId>,
    eligible_products: Vec<String>,
    effective_date: Option<Date>,
    termination_date: Option<Date>,
    description: Option<String>,
}

impl CrossBookNettingAgreementBuilder {
    /// Creates a new builder with required fields.
    pub fn new(
        id: impl Into<CrossBookNettingAgreementId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> Self {
        Self {
            id: id.into(),
            counterparty_id: counterparty_id.into(),
            book_ids: Vec::new(),
            eligible_products: Vec::new(),
            effective_date: None,
            termination_date: None,
            description: None,
        }
    }

    /// Adds a book to the agreement.
    pub fn add_book(mut self, book_id: impl Into<BookId>) -> Self {
        let id = book_id.into();
        if !self.book_ids.contains(&id) {
            self.book_ids.push(id);
        }
        self
    }

    /// Sets the book IDs.
    pub fn book_ids(mut self, book_ids: impl IntoIterator<Item = impl Into<BookId>>) -> Self {
        self.book_ids = book_ids.into_iter().map(Into::into).collect();
        self
    }

    /// Adds an eligible product type.
    pub fn add_eligible_product(mut self, product: impl Into<String>) -> Self {
        let p = product.into();
        if !self.eligible_products.contains(&p) {
            self.eligible_products.push(p);
        }
        self
    }

    /// Sets the eligible products.
    pub fn eligible_products(
        mut self,
        products: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.eligible_products = products.into_iter().map(Into::into).collect();
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

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builds the CrossBookNettingAgreement.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidNettingSetId`] if fewer than 2 books
    /// are specified, since cross-book netting requires at least 2 books.
    pub fn build(self) -> Result<CrossBookNettingAgreement, CounterPartyError> {
        // Cross-book netting requires at least 2 books
        if self.book_ids.len() < 2 {
            return Err(CounterPartyError::InvalidNettingSetId(format!(
                "Cross-book netting agreement requires at least 2 books, got {}",
                self.book_ids.len()
            )));
        }

        Ok(CrossBookNettingAgreement {
            id: self.id,
            counterparty_id: self.counterparty_id,
            book_ids: self.book_ids,
            eligible_products: self.eligible_products,
            effective_date: self.effective_date,
            termination_date: self.termination_date,
            description: self.description,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Currency;

    // ========================================================================
    // NettingType tests
    // ========================================================================

    #[test]
    fn test_netting_type_default() {
        assert_eq!(NettingType::default(), NettingType::Bilateral);
    }

    #[test]
    fn test_netting_type_is_cleared() {
        assert!(!NettingType::Bilateral.is_cleared());
        assert!(NettingType::ClearedCcp.is_cleared());
        assert!(NettingType::ClearedClient.is_cleared());
    }

    #[test]
    fn test_netting_type_display() {
        assert_eq!(format!("{}", NettingType::Bilateral), "Bilateral");
        assert_eq!(format!("{}", NettingType::ClearedCcp), "Cleared (CCP)");
        assert_eq!(
            format!("{}", NettingType::ClearedClient),
            "Cleared (Client)"
        );
    }

    // ========================================================================
    // ExposureConfig tests
    // ========================================================================

    #[test]
    fn test_exposure_config_default() {
        let config = ExposureConfig::default();
        assert!((config.pfe_confidence() - 0.95).abs() < f64::EPSILON);
        assert!((config.regulatory_maturity() - 1.0).abs() < f64::EPSILON);
        assert!(config.apply_netting());
        assert!(config.apply_collateral());
        assert!(!config.time_grid().is_empty());
    }

    #[test]
    fn test_exposure_config_new() {
        let config = ExposureConfig::new();
        assert_eq!(
            config.pfe_confidence(),
            ExposureConfig::default().pfe_confidence()
        );
    }

    #[test]
    fn test_exposure_config_with_time_grid() {
        let grid = vec![0.5, 1.0, 2.0, 5.0];
        let config = ExposureConfig::new().with_time_grid(grid.clone());
        assert_eq!(config.time_grid(), grid.as_slice());
    }

    #[test]
    fn test_exposure_config_with_pfe_confidence() {
        let config = ExposureConfig::new().with_pfe_confidence(0.99);
        assert!((config.pfe_confidence() - 0.99).abs() < f64::EPSILON);
    }

    #[test]
    fn test_exposure_config_pfe_confidence_clamped() {
        let config1 = ExposureConfig::new().with_pfe_confidence(1.5);
        assert!((config1.pfe_confidence() - 1.0).abs() < f64::EPSILON);

        let config2 = ExposureConfig::new().with_pfe_confidence(-0.5);
        assert!(config2.pfe_confidence().abs() < f64::EPSILON);
    }

    #[test]
    fn test_exposure_config_with_flags() {
        let config = ExposureConfig::new()
            .with_apply_netting(false)
            .with_apply_collateral(false);

        assert!(!config.apply_netting());
        assert!(!config.apply_collateral());
    }

    // ========================================================================
    // NettingSet tests
    // ========================================================================

    #[test]
    fn test_netting_set_builder_minimal() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();

        assert_eq!(ns.id().as_str(), "NS001");
        assert_eq!(ns.counterparty_id().as_str(), "CP001");
        assert!(ns.legal_entity_id().is_none());
        assert_eq!(ns.netting_type(), NettingType::Bilateral);
        assert!(ns.has_closeout_netting());
        assert!(ns.csa_terms().is_none());
        assert!(ns.margin_terms().is_none());
        assert!(ns.ccp_id().is_none());
        assert!(ns.exposure_config().is_none());
    }

    #[test]
    fn test_netting_set_builder_full() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mpor_days(14)
            .margin_currency(Currency::EUR)
            .build();
        let margin = MarginTerms::vm_only(super::super::VmTerms::default());
        let exposure = ExposureConfig::new().with_pfe_confidence(0.99);

        let ns = NettingSet::builder("NS001", "CP001")
            .legal_entity_id(lei)
            .netting_type(NettingType::Bilateral)
            .closeout_netting(true)
            .csa_terms(csa)
            .margin_terms(margin)
            .exposure_config(exposure)
            .build()
            .unwrap();

        assert!(ns.legal_entity_id().is_some());
        assert!(ns.csa_terms().is_some());
        assert!(ns.margin_terms().is_some());
        assert!(ns.exposure_config().is_some());
    }

    #[test]
    fn test_netting_set_cleared_ccp() {
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::ClearedCcp)
            .ccp_id("LCH")
            .build()
            .unwrap();

        assert!(ns.is_cleared());
        assert_eq!(ns.ccp_id().map(|c| c.as_str()), Some("LCH"));
    }

    #[test]
    fn test_netting_set_is_collateralised() {
        let ns_no_csa = NettingSet::builder("NS001", "CP001").build().unwrap();
        assert!(!ns_no_csa.is_collateralised());

        let ns_with_csa = NettingSet::builder("NS002", "CP001")
            .csa_terms(CsaTerms::default())
            .build()
            .unwrap();
        assert!(ns_with_csa.is_collateralised());
    }

    #[test]
    fn test_netting_set_mpor_days_bilateral_no_csa() {
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::Bilateral)
            .build()
            .unwrap();

        // Default MPOR for bilateral without CSA is 10
        assert_eq!(ns.mpor_days(), 10);
    }

    #[test]
    fn test_netting_set_mpor_days_bilateral_with_csa() {
        let csa = CsaTerms::builder().mpor_days(14).build();
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::Bilateral)
            .csa_terms(csa)
            .build()
            .unwrap();

        assert_eq!(ns.mpor_days(), 14);
    }

    #[test]
    fn test_netting_set_mpor_days_cleared() {
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::ClearedCcp)
            .build()
            .unwrap();

        // Cleared MPOR is always 5
        assert_eq!(ns.mpor_days(), 5);
    }

    #[test]
    fn test_netting_set_mpor_days_cleared_ignores_csa() {
        // Even with CSA specifying different MPOR, cleared uses 5
        let csa = CsaTerms::builder().mpor_days(14).build();
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::ClearedCcp)
            .csa_terms(csa)
            .build()
            .unwrap();

        assert_eq!(ns.mpor_days(), 5);
    }

    // ========================================================================
    // Book IDs tests
    // ========================================================================

    #[test]
    fn test_netting_set_book_ids_empty_by_default() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();
        assert!(ns.book_ids().is_empty());
        assert!(!ns.allows_cross_book_netting());
    }

    #[test]
    fn test_netting_set_add_book() {
        let ns = NettingSet::builder("NS001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .build()
            .unwrap();

        assert_eq!(ns.book_ids().len(), 2);
        assert!(ns.allows_cross_book_netting());
    }

    #[test]
    fn test_netting_set_add_book_dedup() {
        let ns = NettingSet::builder("NS001", "CP001")
            .add_book("B001")
            .add_book("B001") // Duplicate
            .add_book("B002")
            .build()
            .unwrap();

        assert_eq!(ns.book_ids().len(), 2);
    }

    #[test]
    fn test_netting_set_book_ids_bulk() {
        let ns = NettingSet::builder("NS001", "CP001")
            .book_ids(["B001", "B002", "B003"])
            .build()
            .unwrap();

        assert_eq!(ns.book_ids().len(), 3);
    }

    #[test]
    fn test_netting_set_allows_book_empty() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();

        // Empty book_ids means all books allowed
        assert!(ns.allows_book(&BookId::new("B001")));
        assert!(ns.allows_book(&BookId::new("B999")));
    }

    #[test]
    fn test_netting_set_allows_book_with_list() {
        let ns = NettingSet::builder("NS001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .build()
            .unwrap();

        assert!(ns.allows_book(&BookId::new("B001")));
        assert!(ns.allows_book(&BookId::new("B002")));
        assert!(!ns.allows_book(&BookId::new("B003")));
    }

    #[test]
    fn test_netting_set_single_book_no_cross_book() {
        let ns = NettingSet::builder("NS001", "CP001")
            .add_book("B001")
            .build()
            .unwrap();

        assert!(!ns.allows_cross_book_netting());
    }

    // ========================================================================
    // CrossBookNettingAgreement tests
    // ========================================================================

    #[test]
    fn test_cross_book_netting_agreement_builder() {
        let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .add_book("B003")
            .effective_date(Date::from_ymd(2025, 1, 1).unwrap())
            .build()
            .unwrap();

        assert_eq!(agreement.id().as_str(), "CBNA001");
        assert_eq!(agreement.counterparty_id().as_str(), "CP001");
        assert_eq!(agreement.book_ids().len(), 3);
        assert!(agreement.is_book_eligible(&BookId::new("B001")));
        assert!(agreement.is_book_eligible(&BookId::new("B002")));
        assert!(!agreement.is_book_eligible(&BookId::new("B999")));
    }

    #[test]
    fn test_cross_book_netting_agreement_requires_multiple_books() {
        let result = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .build();

        // Cross-book netting requires at least 2 books
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_book_netting_agreement_dedup_books() {
        let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .add_book("B001") // Duplicate
            .build()
            .unwrap();

        assert_eq!(agreement.book_ids().len(), 2);
    }

    #[test]
    fn test_cross_book_netting_agreement_product_eligibility() {
        let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .add_eligible_product("IRS")
            .add_eligible_product("CCS")
            .build()
            .unwrap();

        assert!(agreement.is_product_eligible("IRS"));
        assert!(agreement.is_product_eligible("CCS"));
        assert!(!agreement.is_product_eligible("FX"));
    }

    #[test]
    fn test_cross_book_netting_agreement_all_products_eligible_when_empty() {
        let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .build()
            .unwrap();

        // When no products specified, all products are eligible
        assert!(agreement.is_product_eligible("IRS"));
        assert!(agreement.is_product_eligible("FX"));
    }

    // ========================================================================
    // PfeConfidenceLevel tests
    // ========================================================================

    #[test]
    fn test_pfe_confidence_level_default() {
        assert_eq!(PfeConfidenceLevel::default(), PfeConfidenceLevel::Q95);
    }

    #[test]
    fn test_pfe_confidence_level_as_f64() {
        assert!((PfeConfidenceLevel::Q95.as_f64() - 0.95).abs() < f64::EPSILON);
        assert!((PfeConfidenceLevel::Q97_5.as_f64() - 0.975).abs() < f64::EPSILON);
        assert!((PfeConfidenceLevel::Q99.as_f64() - 0.99).abs() < f64::EPSILON);
        assert!((PfeConfidenceLevel::Custom(90).as_f64() - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pfe_confidence_level_from_f64() {
        assert_eq!(PfeConfidenceLevel::from_f64(0.95), PfeConfidenceLevel::Q95);
        assert_eq!(
            PfeConfidenceLevel::from_f64(0.975),
            PfeConfidenceLevel::Q97_5
        );
        assert_eq!(PfeConfidenceLevel::from_f64(0.99), PfeConfidenceLevel::Q99);
        assert_eq!(
            PfeConfidenceLevel::from_f64(0.90),
            PfeConfidenceLevel::Custom(90)
        );
    }

    #[test]
    fn test_pfe_confidence_level_from_f64_rounding() {
        // Values close to standard levels should snap
        assert_eq!(PfeConfidenceLevel::from_f64(0.951), PfeConfidenceLevel::Q95);
        assert_eq!(PfeConfidenceLevel::from_f64(0.949), PfeConfidenceLevel::Q95);
    }

    // ========================================================================
    // ExposureAggregation tests
    // ========================================================================

    #[test]
    fn test_exposure_aggregation_default() {
        assert_eq!(
            ExposureAggregation::default(),
            ExposureAggregation::NetWithinNettingSet
        );
    }

    #[test]
    fn test_exposure_aggregation_applies_netting() {
        assert!(!ExposureAggregation::Gross.applies_netting());
        assert!(ExposureAggregation::NetWithinNettingSet.applies_netting());
        assert!(ExposureAggregation::NetWithinCounterparty.applies_netting());
    }

    // ========================================================================
    // MporConfig tests
    // ========================================================================

    #[test]
    fn test_mpor_config_default() {
        let config = MporConfig::default();
        assert_eq!(config.base_mpor_days, 10);
        assert_eq!(config.dispute_grace_days, 0);
        assert!(!config.apply_illiquidity_extension);
        assert_eq!(config.illiquidity_threshold_days, 20);
    }

    // ========================================================================
    // CollateralizedExposureConfig tests
    // ========================================================================

    #[test]
    fn test_collateralized_exposure_config_default() {
        let config = CollateralizedExposureConfig::default();
        assert!((config.eepe_horizon_years() - 5.0).abs() < f64::EPSILON);
        assert!((config.effective_maturity_years() - 1.0).abs() < f64::EPSILON);
        assert!(config.collateral_volatility().abs() < f64::EPSILON);
        assert!(config.apply_closeout_risk());
    }

    #[test]
    fn test_collateralized_exposure_config_builder() {
        let config = CollateralizedExposureConfig::new()
            .with_eepe_horizon_years(7.0)
            .with_effective_maturity_years(2.0)
            .with_collateral_volatility(0.15)
            .with_apply_closeout_risk(false);

        assert!((config.eepe_horizon_years() - 7.0).abs() < f64::EPSILON);
        assert!((config.effective_maturity_years() - 2.0).abs() < f64::EPSILON);
        assert!((config.collateral_volatility() - 0.15).abs() < f64::EPSILON);
        assert!(!config.apply_closeout_risk());
    }

    #[test]
    fn test_collateralized_exposure_config_volatility_clamped() {
        let config = CollateralizedExposureConfig::new().with_collateral_volatility(1.5);

        assert!((config.collateral_volatility() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_collateralized_exposure_config_with_mpor() {
        let mpor = MporConfig {
            base_mpor_days: 14,
            dispute_grace_days: 3,
            apply_illiquidity_extension: true,
            illiquidity_threshold_days: 30,
        };

        let config = CollateralizedExposureConfig::new().with_mpor_config(mpor);

        assert_eq!(config.mpor_config().base_mpor_days, 14);
        assert_eq!(config.mpor_config().dispute_grace_days, 3);
        assert!(config.mpor_config().apply_illiquidity_extension);
    }
}

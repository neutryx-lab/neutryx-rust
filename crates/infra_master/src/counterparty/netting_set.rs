//! NettingSet and ExposureConfig definitions.
//!
//! This module defines the NettingSet entity for grouping trades for netting
//! purposes, and ExposureConfig for exposure calculation parameters.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::{
    Ccp, CcpId, CounterPartyError, CounterPartyId, CsaTerms, LegalEntityId, MarginTerms,
};

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
/// use infra_master::counterparty::ExposureConfig;
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
    pub fn new() -> Self {
        Self::default()
    }

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
    pub fn time_grid(&self) -> &[f64] {
        &self.time_grid_years
    }

    /// Returns the PFE confidence level.
    pub fn pfe_confidence(&self) -> f64 {
        self.pfe_confidence
    }

    /// Returns the regulatory maturity (in years).
    pub fn regulatory_maturity(&self) -> f64 {
        self.regulatory_maturity
    }

    /// Returns whether netting should be applied.
    pub fn apply_netting(&self) -> bool {
        self.apply_netting
    }

    /// Returns whether collateral effects should be applied.
    pub fn apply_collateral(&self) -> bool {
        self.apply_collateral
    }
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
/// use infra_master::counterparty::{NettingSet, NettingType, CsaTerms, MarginTerms};
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
    pub fn id(&self) -> &NettingSetId {
        &self.netting_set_id
    }

    /// Returns the counterparty ID.
    pub fn counterparty_id(&self) -> &CounterPartyId {
        &self.counterparty_id
    }

    /// Returns the legal entity ID if set.
    pub fn legal_entity_id(&self) -> Option<&LegalEntityId> {
        self.legal_entity_id.as_ref()
    }

    /// Returns the netting type.
    pub fn netting_type(&self) -> NettingType {
        self.netting_type
    }

    /// Returns whether close-out netting applies.
    pub fn has_closeout_netting(&self) -> bool {
        self.closeout_netting
    }

    /// Returns the CSA terms if set.
    pub fn csa_terms(&self) -> Option<&CsaTerms> {
        self.csa_terms.as_ref()
    }

    /// Returns the margin terms if set.
    pub fn margin_terms(&self) -> Option<&MarginTerms> {
        self.margin_terms.as_ref()
    }

    /// Returns the CCP ID if this is a cleared transaction.
    pub fn ccp_id(&self) -> Option<&CcpId> {
        self.ccp_id.as_ref()
    }

    /// Returns the exposure configuration if set.
    pub fn exposure_config(&self) -> Option<&ExposureConfig> {
        self.exposure_config.as_ref()
    }

    /// Returns whether this is a cleared transaction.
    pub fn is_cleared(&self) -> bool {
        self.netting_type.is_cleared()
    }

    /// Returns whether this has collateral (CSA terms set).
    pub fn is_collateralised(&self) -> bool {
        self.csa_terms.is_some()
    }

    /// Returns the MPOR (Margin Period of Risk) in business days.
    ///
    /// For cleared transactions, returns the CCP default (5 days).
    /// For bilateral transactions, returns the CSA MPOR or 10 days default.
    pub fn mpor_days(&self) -> u32 {
        match self.netting_type {
            NettingType::ClearedCcp | NettingType::ClearedClient => Ccp::CLEARED_MPOR_DAYS,
            NettingType::Bilateral => self
                .csa_terms
                .as_ref()
                .map(|c| c.mpor_days())
                .unwrap_or(10),
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
}

impl NettingSetBuilder {
    /// Creates a new builder with required fields.
    pub fn new(
        id: impl Into<NettingSetId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> Self {
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
        assert_eq!(format!("{}", NettingType::ClearedClient), "Cleared (Client)");
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
        assert_eq!(config.pfe_confidence(), ExposureConfig::default().pfe_confidence());
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
}

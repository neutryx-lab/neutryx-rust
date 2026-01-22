//! VM/IM margin terms and SIMM configuration.
//!
//! This module defines types for Variation Margin (VM) and Initial Margin (IM)
//! terms, including SIMM (Standard Initial Margin Model) configuration.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::CallFrequency;
use crate::Currency;

// ============================================================================
// Enums
// ============================================================================

/// Margin type classification.
///
/// Defines the type of margin requirements for a netting set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarginType {
    /// No margin requirements (uncollateralised).
    #[default]
    NoMargin,
    /// Variation Margin only (legacy bilateral CSAs).
    VmOnly,
    /// Both VM and IM (UMR-compliant).
    VmAndIm,
}

/// Initial Margin model type.
///
/// Defines the methodology used to calculate Initial Margin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ImModel {
    /// ISDA SIMM (Standard Initial Margin Model)
    Simm,
    /// Regulatory schedule-based approach
    Schedule,
    /// Grid/table-based approach (CCP-specific)
    Grid,
    /// Internal model (regulator-approved)
    Internal,
}

impl Default for ImModel {
    fn default() -> Self {
        Self::Simm
    }
}

/// SIMM version.
///
/// Defines which version of the ISDA SIMM methodology to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SimmVersion {
    /// SIMM v2.5 (December 2021)
    V2_5,
    /// SIMM v2.6 (December 2023)
    V2_6,
    /// SIMM v2.7 (December 2024)
    V2_7,
}

impl Default for SimmVersion {
    fn default() -> Self {
        Self::V2_6
    }
}

impl std::fmt::Display for SimmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SimmVersion::V2_5 => "2.5",
            SimmVersion::V2_6 => "2.6",
            SimmVersion::V2_7 => "2.7",
        };
        write!(f, "SIMM v{}", s)
    }
}

/// Rounding direction for margin amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RoundingDirection {
    /// Round to nearest (half up)
    #[default]
    Nearest,
    /// Always round up (ceiling)
    Up,
    /// Always round down (floor)
    Down,
}

// ============================================================================
// RoundingRule
// ============================================================================

/// Rounding rule for margin amounts.
///
/// Defines how margin amounts are rounded to specific increments.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{RoundingRule, RoundingDirection};
///
/// let rule = RoundingRule::new(1000.0, RoundingDirection::Up);
/// assert_eq!(rule.apply(1001.0), 2000.0);
/// assert_eq!(rule.apply(1000.0), 1000.0);
/// ```
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoundingRule {
    amount: f64,
    direction: RoundingDirection,
}

impl RoundingRule {
    /// Creates a new rounding rule.
    ///
    /// # Arguments
    ///
    /// * `amount` - The rounding increment (e.g., 1000 for rounding to thousands)
    /// * `direction` - The rounding direction
    pub fn new(amount: f64, direction: RoundingDirection) -> Self {
        Self { amount, direction }
    }

    /// Returns the rounding amount.
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// Returns the rounding direction.
    pub fn direction(&self) -> RoundingDirection {
        self.direction
    }

    /// Applies the rounding rule to a value.
    pub fn apply(&self, value: f64) -> f64 {
        if self.amount <= 0.0 {
            return value;
        }
        match self.direction {
            RoundingDirection::Nearest => (value / self.amount).round() * self.amount,
            RoundingDirection::Up => (value / self.amount).ceil() * self.amount,
            RoundingDirection::Down => (value / self.amount).floor() * self.amount,
        }
    }
}

impl Default for RoundingRule {
    fn default() -> Self {
        Self {
            amount: 0.0,
            direction: RoundingDirection::default(),
        }
    }
}

// ============================================================================
// VmTerms
// ============================================================================

/// Variation Margin terms.
///
/// Defines the terms for VM calculation and settlement.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{VmTerms, CallFrequency, RoundingRule, RoundingDirection};
///
/// let vm = VmTerms::new(CallFrequency::Daily, 1)
///     .with_rounding(RoundingRule::new(1000.0, RoundingDirection::Up));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VmTerms {
    frequency: CallFrequency,
    settlement_lag: u32,
    rounding: Option<RoundingRule>,
}

impl VmTerms {
    /// Creates new VM terms.
    ///
    /// # Arguments
    ///
    /// * `frequency` - How often VM is calculated/exchanged
    /// * `settlement_lag` - Days between calculation and settlement
    pub fn new(frequency: CallFrequency, settlement_lag: u32) -> Self {
        Self {
            frequency,
            settlement_lag,
            rounding: None,
        }
    }

    /// Sets the rounding rule.
    pub fn with_rounding(mut self, rule: RoundingRule) -> Self {
        self.rounding = Some(rule);
        self
    }

    /// Returns the call frequency.
    pub fn frequency(&self) -> CallFrequency {
        self.frequency
    }

    /// Returns the settlement lag in days.
    pub fn settlement_lag(&self) -> u32 {
        self.settlement_lag
    }

    /// Returns the rounding rule if set.
    pub fn rounding(&self) -> Option<&RoundingRule> {
        self.rounding.as_ref()
    }

    /// Applies rounding to a margin amount if a rule is set.
    pub fn apply_rounding(&self, amount: f64) -> f64 {
        match &self.rounding {
            Some(rule) => rule.apply(amount),
            None => amount,
        }
    }
}

impl Default for VmTerms {
    fn default() -> Self {
        Self::new(CallFrequency::Daily, 1)
    }
}

// ============================================================================
// ImTerms
// ============================================================================

/// Initial Margin terms.
///
/// Defines the terms for IM calculation, including the model used
/// and posting requirements.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{ImTerms, ImModel, SimmVersion, CallFrequency};
/// use infra_master::Currency;
///
/// let im = ImTerms::new(ImModel::Simm, Currency::USD)
///     .with_simm_version(SimmVersion::V2_7)
///     .with_calculation_frequency(CallFrequency::Daily);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImTerms {
    model: ImModel,
    simm_version: Option<SimmVersion>,
    calculation_frequency: CallFrequency,
    posting_currency: Currency,
}

impl ImTerms {
    /// Creates new IM terms.
    ///
    /// # Arguments
    ///
    /// * `model` - The IM calculation model
    /// * `posting_currency` - Currency for posting IM
    pub fn new(model: ImModel, posting_currency: Currency) -> Self {
        let simm_version = if model == ImModel::Simm {
            Some(SimmVersion::default())
        } else {
            None
        };
        Self {
            model,
            simm_version,
            calculation_frequency: CallFrequency::Daily,
            posting_currency,
        }
    }

    /// Sets the SIMM version (only applicable for SIMM model).
    pub fn with_simm_version(mut self, version: SimmVersion) -> Self {
        if self.model == ImModel::Simm {
            self.simm_version = Some(version);
        }
        self
    }

    /// Sets the calculation frequency.
    pub fn with_calculation_frequency(mut self, frequency: CallFrequency) -> Self {
        self.calculation_frequency = frequency;
        self
    }

    /// Returns the IM model.
    pub fn model(&self) -> ImModel {
        self.model
    }

    /// Returns the SIMM version if applicable.
    pub fn simm_version(&self) -> Option<SimmVersion> {
        self.simm_version
    }

    /// Returns the calculation frequency.
    pub fn calculation_frequency(&self) -> CallFrequency {
        self.calculation_frequency
    }

    /// Returns the posting currency.
    pub fn posting_currency(&self) -> Currency {
        self.posting_currency
    }
}

// ============================================================================
// SimmRiskClassMapping
// ============================================================================

/// SIMM risk class mapping (placeholder for future integration).
///
/// This structure will hold SIMM risk class definitions and mappings
/// for calculating SIMM IM requirements.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimmRiskClassMapping {
    // Placeholder for SIMM risk class definitions
    // Will be expanded in future iterations
}

// ============================================================================
// MarginTerms
// ============================================================================

/// Combined margin terms (VM + IM).
///
/// Represents the complete margin requirements for a netting set,
/// combining VM and IM terms as applicable.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{MarginTerms, MarginType, VmTerms, ImTerms, ImModel, CallFrequency};
/// use infra_master::Currency;
///
/// // No margin
/// let no_margin = MarginTerms::no_margin();
/// assert_eq!(no_margin.margin_type(), MarginType::NoMargin);
///
/// // VM only
/// let vm_only = MarginTerms::vm_only(VmTerms::default());
/// assert_eq!(vm_only.margin_type(), MarginType::VmOnly);
///
/// // VM and IM (UMR-compliant)
/// let umr = MarginTerms::vm_and_im(
///     VmTerms::new(CallFrequency::Daily, 1),
///     ImTerms::new(ImModel::Simm, Currency::USD),
/// );
/// assert_eq!(umr.margin_type(), MarginType::VmAndIm);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarginTerms {
    margin_type: MarginType,
    vm_terms: Option<VmTerms>,
    im_terms: Option<ImTerms>,
}

impl MarginTerms {
    /// Creates margin terms with no margin requirements.
    pub fn no_margin() -> Self {
        Self {
            margin_type: MarginType::NoMargin,
            vm_terms: None,
            im_terms: None,
        }
    }

    /// Creates margin terms with VM only.
    pub fn vm_only(vm: VmTerms) -> Self {
        Self {
            margin_type: MarginType::VmOnly,
            vm_terms: Some(vm),
            im_terms: None,
        }
    }

    /// Creates margin terms with both VM and IM.
    pub fn vm_and_im(vm: VmTerms, im: ImTerms) -> Self {
        Self {
            margin_type: MarginType::VmAndIm,
            vm_terms: Some(vm),
            im_terms: Some(im),
        }
    }

    /// Returns the margin type.
    pub fn margin_type(&self) -> MarginType {
        self.margin_type
    }

    /// Returns the VM terms if applicable.
    pub fn vm_terms(&self) -> Option<&VmTerms> {
        self.vm_terms.as_ref()
    }

    /// Returns the IM terms if applicable.
    pub fn im_terms(&self) -> Option<&ImTerms> {
        self.im_terms.as_ref()
    }

    /// Returns whether this requires VM.
    pub fn requires_vm(&self) -> bool {
        matches!(self.margin_type, MarginType::VmOnly | MarginType::VmAndIm)
    }

    /// Returns whether this requires IM.
    pub fn requires_im(&self) -> bool {
        matches!(self.margin_type, MarginType::VmAndIm)
    }
}

impl Default for MarginTerms {
    fn default() -> Self {
        Self::no_margin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Enum tests
    // ========================================================================

    #[test]
    fn test_margin_type_default() {
        assert_eq!(MarginType::default(), MarginType::NoMargin);
    }

    #[test]
    fn test_im_model_default() {
        assert_eq!(ImModel::default(), ImModel::Simm);
    }

    #[test]
    fn test_simm_version_default() {
        assert_eq!(SimmVersion::default(), SimmVersion::V2_6);
    }

    #[test]
    fn test_simm_version_display() {
        assert_eq!(format!("{}", SimmVersion::V2_5), "SIMM v2.5");
        assert_eq!(format!("{}", SimmVersion::V2_6), "SIMM v2.6");
        assert_eq!(format!("{}", SimmVersion::V2_7), "SIMM v2.7");
    }

    #[test]
    fn test_rounding_direction_default() {
        assert_eq!(RoundingDirection::default(), RoundingDirection::Nearest);
    }

    // ========================================================================
    // RoundingRule tests
    // ========================================================================

    #[test]
    fn test_rounding_rule_nearest() {
        let rule = RoundingRule::new(1000.0, RoundingDirection::Nearest);

        // Exactly on boundary
        assert!((rule.apply(1000.0) - 1000.0).abs() < f64::EPSILON);

        // Round down (below half)
        assert!((rule.apply(1400.0) - 1000.0).abs() < f64::EPSILON);

        // Round up (above half)
        assert!((rule.apply(1600.0) - 2000.0).abs() < f64::EPSILON);

        // Half rounds up (banker's rounding may vary)
        assert!(rule.apply(1500.0) >= 1500.0);
    }

    #[test]
    fn test_rounding_rule_up() {
        let rule = RoundingRule::new(1000.0, RoundingDirection::Up);

        assert!((rule.apply(1000.0) - 1000.0).abs() < f64::EPSILON);
        assert!((rule.apply(1001.0) - 2000.0).abs() < f64::EPSILON);
        assert!((rule.apply(1999.0) - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding_rule_down() {
        let rule = RoundingRule::new(1000.0, RoundingDirection::Down);

        assert!((rule.apply(1000.0) - 1000.0).abs() < f64::EPSILON);
        assert!((rule.apply(1999.0) - 1000.0).abs() < f64::EPSILON);
        assert!((rule.apply(2000.0) - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding_rule_zero_amount() {
        let rule = RoundingRule::new(0.0, RoundingDirection::Up);
        let value = 1234.567;
        assert!((rule.apply(value) - value).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding_rule_negative_amount() {
        let rule = RoundingRule::new(-100.0, RoundingDirection::Up);
        let value = 1234.567;
        assert!((rule.apply(value) - value).abs() < f64::EPSILON);
    }

    // ========================================================================
    // VmTerms tests
    // ========================================================================

    #[test]
    fn test_vm_terms_new() {
        let vm = VmTerms::new(CallFrequency::Daily, 1);
        assert_eq!(vm.frequency(), CallFrequency::Daily);
        assert_eq!(vm.settlement_lag(), 1);
        assert!(vm.rounding().is_none());
    }

    #[test]
    fn test_vm_terms_with_rounding() {
        let rule = RoundingRule::new(1000.0, RoundingDirection::Up);
        let vm = VmTerms::new(CallFrequency::Daily, 1).with_rounding(rule);

        assert!(vm.rounding().is_some());
        assert!((vm.apply_rounding(1500.0) - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vm_terms_apply_rounding_none() {
        let vm = VmTerms::new(CallFrequency::Daily, 1);
        let value = 1234.567;
        assert!((vm.apply_rounding(value) - value).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vm_terms_default() {
        let vm = VmTerms::default();
        assert_eq!(vm.frequency(), CallFrequency::Daily);
        assert_eq!(vm.settlement_lag(), 1);
    }

    // ========================================================================
    // ImTerms tests
    // ========================================================================

    #[test]
    fn test_im_terms_simm() {
        let im = ImTerms::new(ImModel::Simm, Currency::USD);
        assert_eq!(im.model(), ImModel::Simm);
        assert_eq!(im.simm_version(), Some(SimmVersion::V2_6));
        assert_eq!(im.posting_currency(), Currency::USD);
        assert_eq!(im.calculation_frequency(), CallFrequency::Daily);
    }

    #[test]
    fn test_im_terms_schedule() {
        let im = ImTerms::new(ImModel::Schedule, Currency::EUR);
        assert_eq!(im.model(), ImModel::Schedule);
        assert!(im.simm_version().is_none()); // SIMM version not applicable
    }

    #[test]
    fn test_im_terms_with_simm_version() {
        let im = ImTerms::new(ImModel::Simm, Currency::USD).with_simm_version(SimmVersion::V2_7);
        assert_eq!(im.simm_version(), Some(SimmVersion::V2_7));
    }

    #[test]
    fn test_im_terms_simm_version_ignored_for_non_simm() {
        let im = ImTerms::new(ImModel::Schedule, Currency::USD).with_simm_version(SimmVersion::V2_7);
        // SIMM version should remain None for non-SIMM models
        assert!(im.simm_version().is_none());
    }

    #[test]
    fn test_im_terms_with_calculation_frequency() {
        let im = ImTerms::new(ImModel::Simm, Currency::USD)
            .with_calculation_frequency(CallFrequency::Weekly);
        assert_eq!(im.calculation_frequency(), CallFrequency::Weekly);
    }

    // ========================================================================
    // MarginTerms tests
    // ========================================================================

    #[test]
    fn test_margin_terms_no_margin() {
        let terms = MarginTerms::no_margin();
        assert_eq!(terms.margin_type(), MarginType::NoMargin);
        assert!(terms.vm_terms().is_none());
        assert!(terms.im_terms().is_none());
        assert!(!terms.requires_vm());
        assert!(!terms.requires_im());
    }

    #[test]
    fn test_margin_terms_vm_only() {
        let terms = MarginTerms::vm_only(VmTerms::default());
        assert_eq!(terms.margin_type(), MarginType::VmOnly);
        assert!(terms.vm_terms().is_some());
        assert!(terms.im_terms().is_none());
        assert!(terms.requires_vm());
        assert!(!terms.requires_im());
    }

    #[test]
    fn test_margin_terms_vm_and_im() {
        let terms = MarginTerms::vm_and_im(
            VmTerms::new(CallFrequency::Daily, 1),
            ImTerms::new(ImModel::Simm, Currency::USD),
        );
        assert_eq!(terms.margin_type(), MarginType::VmAndIm);
        assert!(terms.vm_terms().is_some());
        assert!(terms.im_terms().is_some());
        assert!(terms.requires_vm());
        assert!(terms.requires_im());
    }

    #[test]
    fn test_margin_terms_default() {
        let terms = MarginTerms::default();
        assert_eq!(terms.margin_type(), MarginType::NoMargin);
    }
}

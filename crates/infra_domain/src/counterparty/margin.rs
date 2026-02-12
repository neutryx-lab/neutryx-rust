//! VM/IM margin terms and SIMM configuration.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::CallFrequency;
use crate::market::Currency;

/// Margin type classification.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
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
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ImModel {
    /// ISDA SIMM (Standard Initial Margin Model).
    #[default]
    Simm,
    /// Regulatory schedule-based approach.
    Schedule,
    /// Grid/table-based approach (CCP-specific).
    Grid,
    /// Internal model (regulator-approved).
    Internal,
}

/// SIMM version.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum SimmVersion {
    /// SIMM v2.5 (December 2021).
    #[strum(serialize = "SIMM v2.5")]
    V2_5,
    /// SIMM v2.6 (December 2023).
    #[default]
    #[strum(serialize = "SIMM v2.6")]
    V2_6,
    /// SIMM v2.7 (December 2024).
    #[strum(serialize = "SIMM v2.7")]
    V2_7,
}

/// Rounding direction for margin amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum RoundingDirection {
    /// Round to nearest (half up).
    #[default]
    Nearest,
    /// Always round up (ceiling).
    Up,
    /// Always round down (floor).
    Down,
}

/// Rounding rule for margin amounts.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoundingRule {
    amount: f64,
    direction: RoundingDirection,
}

impl RoundingRule {
    /// Creates a new rounding rule.
    pub fn new(amount: f64, direction: RoundingDirection) -> Self { Self { amount, direction } }

    /// Returns the rounding amount.
    pub fn amount(&self) -> f64 { self.amount }

    /// Returns the rounding direction.
    pub fn direction(&self) -> RoundingDirection { self.direction }

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

/// Variation Margin terms.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VmTerms {
    frequency: CallFrequency,
    settlement_lag: u32,
    rounding: Option<RoundingRule>,
}

impl VmTerms {
    /// Creates new VM terms.
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
    pub fn frequency(&self) -> CallFrequency { self.frequency }

    /// Returns the settlement lag in days.
    pub fn settlement_lag(&self) -> u32 { self.settlement_lag }

    /// Returns the rounding rule if set.
    pub fn rounding(&self) -> Option<&RoundingRule> { self.rounding.as_ref() }

    /// Applies rounding to a margin amount if a rule is set.
    pub fn apply_rounding(&self, amount: f64) -> f64 {
        match &self.rounding {
            Some(rule) => rule.apply(amount),
            None => amount,
        }
    }
}

impl Default for VmTerms {
    fn default() -> Self { Self::new(CallFrequency::Daily, 1) }
}

/// Initial Margin terms.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ImTerms {
    model: ImModel,
    simm_version: Option<SimmVersion>,
    calculation_frequency: CallFrequency,
    posting_currency: Currency,
}

impl ImTerms {
    /// Creates new IM terms.
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
    pub fn model(&self) -> ImModel { self.model }

    /// Returns the SIMM version if applicable.
    pub fn simm_version(&self) -> Option<SimmVersion> { self.simm_version }

    /// Returns the calculation frequency.
    pub fn calculation_frequency(&self) -> CallFrequency { self.calculation_frequency }

    /// Returns the posting currency.
    pub fn posting_currency(&self) -> Currency { self.posting_currency }
}

/// Combined margin terms (VM + IM).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    pub fn margin_type(&self) -> MarginType { self.margin_type }

    /// Returns the VM terms if applicable.
    pub fn vm_terms(&self) -> Option<&VmTerms> { self.vm_terms.as_ref() }

    /// Returns the IM terms if applicable.
    pub fn im_terms(&self) -> Option<&ImTerms> { self.im_terms.as_ref() }

    /// Returns whether this requires VM.
    pub fn requires_vm(&self) -> bool {
        matches!(self.margin_type, MarginType::VmOnly | MarginType::VmAndIm)
    }

    /// Returns whether this requires IM.
    pub fn requires_im(&self) -> bool { matches!(self.margin_type, MarginType::VmAndIm) }
}

impl Default for MarginTerms {
    fn default() -> Self { Self::no_margin() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rounding_rule() {
        let up = RoundingRule::new(1000.0, RoundingDirection::Up);
        assert!((up.apply(1001.0) - 2000.0).abs() < f64::EPSILON);

        let down = RoundingRule::new(1000.0, RoundingDirection::Down);
        assert!((down.apply(1999.0) - 1000.0).abs() < f64::EPSILON);

        let zero = RoundingRule::new(0.0, RoundingDirection::Up);
        assert!((zero.apply(1234.567) - 1234.567).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vm_terms() {
        let rule = RoundingRule::new(1000.0, RoundingDirection::Up);
        let vm = VmTerms::new(CallFrequency::Daily, 1).with_rounding(rule);
        assert_eq!(vm.frequency(), CallFrequency::Daily);
        assert!((vm.apply_rounding(1500.0) - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_im_terms() {
        let im = ImTerms::new(ImModel::Simm, Currency::USD);
        assert_eq!(im.simm_version(), Some(SimmVersion::V2_6));

        let im2 =
            ImTerms::new(ImModel::Schedule, Currency::EUR).with_simm_version(SimmVersion::V2_7);
        assert!(im2.simm_version().is_none());
    }

    #[test]
    fn test_margin_terms() {
        let no_margin = MarginTerms::no_margin();
        assert!(!no_margin.requires_vm());
        assert!(!no_margin.requires_im());

        let vm_only = MarginTerms::vm_only(VmTerms::default());
        assert!(vm_only.requires_vm());
        assert!(!vm_only.requires_im());

        let vm_and_im = MarginTerms::vm_and_im(
            VmTerms::new(CallFrequency::Daily, 1),
            ImTerms::new(ImModel::Simm, Currency::USD),
        );
        assert!(vm_and_im.requires_vm());
        assert!(vm_and_im.requires_im());
    }
}

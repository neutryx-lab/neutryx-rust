//! Counterparty portfolio hierarchy (stub for future XVA integration).
//!
//! This module will provide the complete hierarchy for XVA calculation:
//! `CounterpartyPortfolio` -> `IsdaMasterAgreement` ->
//! `VariationMarginAgreement` -> Trade
//!
//! Currently provides minimal enum types. Full implementation will be added
//! when the XVA engine is integrated.

// ============================================================================
// IsdaPaymentMethod
// ============================================================================

/// ISDA payment method for collateral.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IsdaPaymentMethod {
    /// Full bilateral exchange.
    #[default]
    Full,
    /// Limited recourse.
    Limited,
    /// One-way posting to counterparty.
    OnewayToCpty,
    /// One-way posting to self.
    OnewayToSelf,
}

// ============================================================================
// CollateralCallFrequency
// ============================================================================

/// Collateral call frequency for VM agreements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CollateralCallFrequency {
    /// Daily margin calls (standard).
    #[default]
    Daily,
    /// Weekly margin calls.
    Weekly,
    /// Bi-weekly margin calls.
    Biweekly,
    /// Monthly margin calls.
    Monthly,
}

impl CollateralCallFrequency {
    /// Returns the default MPOR (Margin Period of Risk) in business days.
    #[must_use]
    pub fn default_mpor_days(&self) -> u32 {
        match self {
            CollateralCallFrequency::Daily => 10,
            CollateralCallFrequency::Weekly => 10,
            CollateralCallFrequency::Biweekly => 14,
            CollateralCallFrequency::Monthly => 20,
        }
    }
}

// ============================================================================
// NettingEligibility
// ============================================================================

/// Netting eligibility classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NettingEligibility {
    /// Full netting with CSA collateral.
    FullNetting,
    /// ISDA netting only (no CSA).
    IsdaOnly,
    /// Non-nettable (gross exposure).
    NonNettable,
}

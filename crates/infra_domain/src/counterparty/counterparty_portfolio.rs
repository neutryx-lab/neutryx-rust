//! Counterparty portfolio hierarchy (stub for future XVA integration).

/// ISDA payment method for collateral.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
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

/// Collateral call frequency for VM agreements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
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

/// Netting eligibility classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NettingEligibility {
    /// Full netting with CSA collateral.
    FullNetting,
    /// ISDA netting only (no CSA).
    IsdaOnly,
    /// Non-nettable (gross exposure).
    NonNettable,
}

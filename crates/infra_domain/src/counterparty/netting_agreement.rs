//! Netting agreement types (stub for future XVA integration).
//!
//! Provides enum types for netting agreement classification. Full agreement
//! structures will be added when the XVA engine is integrated.

/// Netting agreement type classification.
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

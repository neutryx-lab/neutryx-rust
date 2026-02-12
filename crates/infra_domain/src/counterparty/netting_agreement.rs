//! Netting agreement types (stub for future XVA integration).

/// Netting agreement type classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, strum::Display, serde::Serialize, serde::Deserialize)]
pub enum NettingAgreementType {
    /// ISDA Master Agreement (derivatives).
    #[default]
    #[strum(serialize = "ISDA")]
    Isda,
    /// Global Master Repurchase Agreement (repos).
    #[strum(serialize = "GMRA")]
    Gmra,
    /// Global Master Securities Lending Agreement.
    #[strum(serialize = "GMSLA")]
    Gmsla,
    /// Credit Support Annex (collateral).
    #[strum(serialize = "CSA")]
    Csa,
    /// Custom or regional agreement.
    Custom,
}

/// Close-out calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
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

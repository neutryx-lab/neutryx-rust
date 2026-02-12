//! XVA calculation configuration types (stub for future XVA integration).

/// XVA calculation aggregation level.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum XvaCalculationLevel {
    /// Trade-level calculation (no aggregation).
    Trade,
    /// Netting set level (standard for CVA).
    #[default]
    NettingSet,
    /// Counterparty level (aggregates netting sets).
    Counterparty,
    /// Book level (aggregates by trading book).
    Book,
    /// Portfolio level (full aggregation).
    Portfolio,
}

/// Regulatory capital calculation method.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum RegulatoryCapitalMethod {
    /// Standardised Approach for Counterparty Credit Risk.
    #[default]
    SaCcr,
    /// Internal Model Method.
    Imm,
}

/// Wrong-Way Risk model type.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum WwrModelType {
    /// No WWR adjustment.
    #[default]
    None,
    /// Constant correlation model.
    ConstantCorrelation,
    /// Hull-White correlation model.
    HullWhite,
    /// Jump-to-default model.
    JumpToDefault,
}

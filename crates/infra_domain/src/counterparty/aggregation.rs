//! Aggregation hierarchy types (stub for future XVA integration).
//!
//! Provides enum types for hierarchical aggregation of exposure and XVA
//! calculations. Full aggregation engine will be added when XVA is integrated.

/// Aggregation hierarchy level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AggregationHierarchy {
    /// Individual trade level (no aggregation).
    Trade,
    /// Netting set level.
    #[default]
    NettingSet,
    /// Trading book level.
    Book,
    /// Counterparty level.
    Counterparty,
    /// Legal entity level.
    LegalEntity,
    /// Full portfolio level.
    Portfolio,
}

/// Aggregation method for combining values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AggregationMethod {
    /// Sum all values.
    #[default]
    Sum,
    /// Take the average.
    Average,
    /// Take the maximum.
    Max,
    /// Take the minimum.
    Min,
    /// Weighted average (requires weights).
    WeightedAverage,
}

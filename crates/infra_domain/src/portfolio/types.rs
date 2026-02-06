//! Portfolio-related types.
//!
//! This module provides type definitions for portfolios including
//! scope classifications, metadata, and book mappings.

use chrono::{DateTime, Utc};

use crate::{
    book::BookOwnership,
    ids::{BookId, PortfolioId},
    market::Currency,
};

// ============================================================================
// PortfolioScope
// ============================================================================

/// Scope of a portfolio.
///
/// Classifies the purpose and usage of a portfolio within the organisation.
///
/// # Examples
///
/// ```
/// use infra_master::portfolio::PortfolioScope;
///
/// let scope = PortfolioScope::Internal;
/// assert_eq!(scope, PortfolioScope::default());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum PortfolioScope {
    /// Internal management portfolio.
    #[default]
    Internal,
    /// Legal entity-level portfolio.
    Legal,
    /// Regulatory reporting portfolio.
    Regulatory,
    /// Consolidated portfolio (aggregated across entities).
    Consolidated,
}

impl PortfolioScope {
    /// Returns true if this is an internal scope.
    #[inline]
    #[must_use]
    pub fn is_internal(&self) -> bool { matches!(self, PortfolioScope::Internal) }

    /// Returns true if this is a legal scope.
    #[inline]
    #[must_use]
    pub fn is_legal(&self) -> bool { matches!(self, PortfolioScope::Legal) }

    /// Returns true if this is a regulatory scope.
    #[inline]
    #[must_use]
    pub fn is_regulatory(&self) -> bool { matches!(self, PortfolioScope::Regulatory) }

    /// Returns true if this is a consolidated scope.
    #[inline]
    #[must_use]
    pub fn is_consolidated(&self) -> bool { matches!(self, PortfolioScope::Consolidated) }
}

// ============================================================================
// PortfolioMetadata
// ============================================================================

/// Metadata for a portfolio.
///
/// Captures ownership, scope, reporting currency, and audit information.
///
/// # Examples
///
/// ```
/// use infra_master::portfolio::PortfolioMetadata;
/// use infra_master::market::Currency;
///
/// let metadata = PortfolioMetadata::new(Currency::USD);
/// assert_eq!(metadata.reporting_currency(), Currency::USD);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PortfolioMetadata {
    ownership: Option<BookOwnership>,
    scope: PortfolioScope,
    reporting_currency: Currency,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PortfolioMetadata {
    /// Creates new metadata with the specified reporting currency.
    #[must_use]
    pub fn new(reporting_currency: Currency) -> Self {
        let now = Utc::now();
        Self {
            ownership: None,
            scope: PortfolioScope::default(),
            reporting_currency,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the ownership information.
    #[must_use]
    pub fn with_ownership(mut self, ownership: BookOwnership) -> Self {
        self.ownership = Some(ownership);
        self
    }

    /// Sets the portfolio scope.
    #[must_use]
    pub fn with_scope(mut self, scope: PortfolioScope) -> Self {
        self.scope = scope;
        self
    }

    /// Returns the ownership information.
    #[inline]
    #[must_use]
    pub fn ownership(&self) -> Option<&BookOwnership> { self.ownership.as_ref() }

    /// Returns the portfolio scope.
    #[inline]
    #[must_use]
    pub fn scope(&self) -> PortfolioScope { self.scope }

    /// Returns the reporting currency.
    #[inline]
    #[must_use]
    pub fn reporting_currency(&self) -> Currency { self.reporting_currency }

    /// Returns the creation timestamp.
    #[inline]
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }

    /// Returns the last update timestamp.
    #[inline]
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}

// ============================================================================
// PortfolioBookMapping
// ============================================================================

/// Mapping between a portfolio and a book.
///
/// Represents the many-to-many relationship between portfolios and books,
/// with an optional weight for weighted aggregation.
///
/// # Examples
///
/// ```
/// use infra_master::portfolio::PortfolioBookMapping;
/// use infra_master::ids::{PortfolioId, BookId};
///
/// let mapping = PortfolioBookMapping::new("P001", "B001");
/// assert_eq!(mapping.portfolio_id().as_str(), "P001");
/// assert_eq!(mapping.book_id().as_str(), "B001");
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PortfolioBookMapping {
    portfolio_id: PortfolioId,
    book_id: BookId,
    weight: Option<f64>,
}

impl PortfolioBookMapping {
    /// Creates a new portfolio-book mapping.
    #[must_use]
    pub fn new(portfolio_id: impl Into<PortfolioId>, book_id: impl Into<BookId>) -> Self {
        Self {
            portfolio_id: portfolio_id.into(),
            book_id: book_id.into(),
            weight: None,
        }
    }

    /// Creates a new portfolio-book mapping with weight.
    #[must_use]
    pub fn with_weight(
        portfolio_id: impl Into<PortfolioId>,
        book_id: impl Into<BookId>,
        weight: f64,
    ) -> Self {
        Self {
            portfolio_id: portfolio_id.into(),
            book_id: book_id.into(),
            weight: Some(weight),
        }
    }

    /// Returns the portfolio ID.
    #[inline]
    #[must_use]
    pub fn portfolio_id(&self) -> &PortfolioId { &self.portfolio_id }

    /// Returns the book ID.
    #[inline]
    #[must_use]
    pub fn book_id(&self) -> &BookId { &self.book_id }

    /// Returns the weight, if any.
    #[inline]
    #[must_use]
    pub fn weight(&self) -> Option<f64> { self.weight }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PortfolioScope tests
    // ========================================================================

    #[test]
    fn test_portfolio_scope_default() {
        let scope = PortfolioScope::default();
        assert_eq!(scope, PortfolioScope::Internal);
    }

    #[test]
    fn test_portfolio_scope_is_internal() {
        assert!(PortfolioScope::Internal.is_internal());
        assert!(!PortfolioScope::Legal.is_internal());
    }

    #[test]
    fn test_portfolio_scope_is_legal() {
        assert!(PortfolioScope::Legal.is_legal());
        assert!(!PortfolioScope::Internal.is_legal());
    }

    #[test]
    fn test_portfolio_scope_is_regulatory() {
        assert!(PortfolioScope::Regulatory.is_regulatory());
        assert!(!PortfolioScope::Internal.is_regulatory());
    }

    #[test]
    fn test_portfolio_scope_is_consolidated() {
        assert!(PortfolioScope::Consolidated.is_consolidated());
        assert!(!PortfolioScope::Internal.is_consolidated());
    }

    #[test]
    fn test_portfolio_scope_clone_and_equality() {
        let s1 = PortfolioScope::Legal;
        let s2 = s1;
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_portfolio_scope_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PortfolioScope::Internal);
        set.insert(PortfolioScope::Legal);
        set.insert(PortfolioScope::Internal); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ========================================================================
    // PortfolioMetadata tests
    // ========================================================================

    #[test]
    fn test_portfolio_metadata_new() {
        let metadata = PortfolioMetadata::new(Currency::USD);
        assert_eq!(metadata.reporting_currency(), Currency::USD);
        assert_eq!(metadata.scope(), PortfolioScope::Internal);
        assert!(metadata.ownership().is_none());
    }

    #[test]
    fn test_portfolio_metadata_with_scope() {
        let metadata = PortfolioMetadata::new(Currency::EUR).with_scope(PortfolioScope::Regulatory);
        assert_eq!(metadata.scope(), PortfolioScope::Regulatory);
    }

    #[test]
    fn test_portfolio_metadata_with_ownership() {
        let ownership = BookOwnership::new().with_desk("Trading");
        let metadata = PortfolioMetadata::new(Currency::GBP).with_ownership(ownership);
        assert!(metadata.ownership().is_some());
        assert_eq!(metadata.ownership().unwrap().desk(), Some("Trading"));
    }

    #[test]
    fn test_portfolio_metadata_timestamps() {
        let metadata = PortfolioMetadata::new(Currency::USD);
        assert!(metadata.created_at() <= metadata.updated_at());
    }

    #[test]
    fn test_portfolio_metadata_clone() {
        let metadata = PortfolioMetadata::new(Currency::JPY).with_scope(PortfolioScope::Legal);
        let cloned = metadata.clone();
        assert_eq!(cloned.scope(), PortfolioScope::Legal);
        assert_eq!(cloned.reporting_currency(), Currency::JPY);
    }

    // ========================================================================
    // PortfolioBookMapping tests
    // ========================================================================

    #[test]
    fn test_portfolio_book_mapping_new() {
        let mapping = PortfolioBookMapping::new("P001", "B001");
        assert_eq!(mapping.portfolio_id().as_str(), "P001");
        assert_eq!(mapping.book_id().as_str(), "B001");
        assert!(mapping.weight().is_none());
    }

    #[test]
    fn test_portfolio_book_mapping_with_weight() {
        let mapping = PortfolioBookMapping::with_weight("P001", "B001", 0.5);
        assert_eq!(mapping.portfolio_id().as_str(), "P001");
        assert_eq!(mapping.book_id().as_str(), "B001");
        assert_eq!(mapping.weight(), Some(0.5));
    }

    #[test]
    fn test_portfolio_book_mapping_from_ids() {
        let portfolio_id = PortfolioId::new("P002");
        let book_id = BookId::new("B002");
        let mapping = PortfolioBookMapping::new(portfolio_id, book_id);
        assert_eq!(mapping.portfolio_id().as_str(), "P002");
        assert_eq!(mapping.book_id().as_str(), "B002");
    }

    #[test]
    fn test_portfolio_book_mapping_clone() {
        let mapping = PortfolioBookMapping::with_weight("P001", "B001", 0.75);
        let cloned = mapping.clone();
        assert_eq!(cloned.portfolio_id().as_str(), "P001");
        assert_eq!(cloned.weight(), Some(0.75));
    }
}

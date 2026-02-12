//! Portfolio-related types.

use chrono::{DateTime, Utc};

use crate::{
    book::BookOwnership,
    ids::{BookId, PortfolioId},
    market::Currency,
};

/// Scope of a portfolio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Metadata for a portfolio.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Mapping between a portfolio and a book.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

    #[test]
    fn test_portfolio_metadata_builder() {
        let o = BookOwnership::new().with_desk("Trading");
        let m = PortfolioMetadata::new(Currency::GBP)
            .with_scope(PortfolioScope::Regulatory)
            .with_ownership(o);
        assert_eq!(m.reporting_currency(), Currency::GBP);
        assert_eq!(m.scope(), PortfolioScope::Regulatory);
        assert_eq!(m.ownership().unwrap().desk(), Some("Trading"));
    }

    #[test]
    fn test_portfolio_book_mapping() {
        let m = PortfolioBookMapping::new("P001", "B001");
        assert_eq!(m.portfolio_id().as_str(), "P001");
        assert_eq!(m.book_id().as_str(), "B001");
        assert!(m.weight().is_none());

        let mw = PortfolioBookMapping::with_weight("P001", "B001", 0.5);
        assert_eq!(mw.weight(), Some(0.5));
    }
}

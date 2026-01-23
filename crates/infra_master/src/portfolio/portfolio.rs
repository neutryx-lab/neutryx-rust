//! Portfolio definition and builder.
//!
//! This module provides the PortfolioDefinition struct representing a portfolio
//! and its builder for fluent construction with validation.

use std::collections::HashSet;

use crate::error::PortfolioError;
use crate::ids::{BookId, PortfolioId};
use crate::market::Currency;

use super::{PortfolioBookMapping, PortfolioMetadata, PortfolioScope};

// ============================================================================
// PortfolioDefinition
// ============================================================================

/// A portfolio definition.
///
/// Represents a logical grouping of books for risk management and
/// regulatory reporting purposes. Supports hierarchical portfolios
/// through parent-child relationships.
///
/// # Examples
///
/// ```
/// use infra_master::portfolio::{PortfolioDefinition, PortfolioScope};
/// use infra_master::market::Currency;
///
/// let portfolio = PortfolioDefinition::builder("P001", "Main Portfolio", Currency::USD)
///     .description("Primary trading portfolio")
///     .scope(PortfolioScope::Regulatory)
///     .build();
///
/// assert_eq!(portfolio.portfolio_id().as_str(), "P001");
/// assert_eq!(portfolio.name(), "Main Portfolio");
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PortfolioDefinition {
    portfolio_id: PortfolioId,
    name: String,
    description: Option<String>,
    parent_portfolio_id: Option<PortfolioId>,
    book_ids: Vec<BookId>,
    metadata: PortfolioMetadata,
}

impl PortfolioDefinition {
    /// Creates a new portfolio builder.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the portfolio
    /// * `name` - Human-readable name for the portfolio
    /// * `reporting_currency` - Currency for portfolio-level reporting
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::portfolio::PortfolioDefinition;
    /// use infra_master::market::Currency;
    ///
    /// let portfolio = PortfolioDefinition::builder("P001", "Main Portfolio", Currency::USD)
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(
        id: impl Into<PortfolioId>,
        name: impl Into<String>,
        reporting_currency: Currency,
    ) -> PortfolioBuilder {
        PortfolioBuilder::new(id, name, reporting_currency)
    }

    /// Returns the portfolio's unique identifier.
    #[inline]
    #[must_use]
    pub fn portfolio_id(&self) -> &PortfolioId { &self.portfolio_id }

    /// Returns the portfolio's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    /// Returns the portfolio's description.
    #[inline]
    #[must_use]
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }

    /// Returns the parent portfolio ID, if any.
    #[inline]
    #[must_use]
    pub fn parent_portfolio_id(&self) -> Option<&PortfolioId> {
        self.parent_portfolio_id.as_ref()
    }

    /// Returns the list of book IDs in this portfolio.
    #[inline]
    #[must_use]
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns the portfolio's metadata.
    #[inline]
    #[must_use]
    pub fn metadata(&self) -> &PortfolioMetadata { &self.metadata }

    /// Returns the reporting currency.
    #[inline]
    #[must_use]
    pub fn reporting_currency(&self) -> Currency { self.metadata.reporting_currency() }

    /// Returns the portfolio scope.
    #[inline]
    #[must_use]
    pub fn scope(&self) -> PortfolioScope { self.metadata.scope() }

    /// Returns true if this portfolio has a parent.
    #[inline]
    #[must_use]
    pub fn has_parent(&self) -> bool { self.parent_portfolio_id.is_some() }

    /// Returns true if this portfolio contains the specified book.
    #[must_use]
    pub fn contains_book(&self, book_id: &BookId) -> bool {
        self.book_ids.contains(book_id)
    }

    /// Creates book mappings for this portfolio.
    #[must_use]
    pub fn book_mappings(&self) -> Vec<PortfolioBookMapping> {
        self.book_ids
            .iter()
            .map(|book_id| {
                PortfolioBookMapping::new(self.portfolio_id.clone(), book_id.clone())
            })
            .collect()
    }
}

// ============================================================================
// PortfolioBuilder
// ============================================================================

/// Builder for constructing PortfolioDefinition instances.
///
/// Provides a fluent API for setting portfolio properties before construction.
/// Includes validation for referential integrity.
///
/// # Examples
///
/// ```
/// use infra_master::portfolio::{PortfolioDefinition, PortfolioScope};
/// use infra_master::market::Currency;
///
/// let portfolio = PortfolioDefinition::builder("P001", "Main Portfolio", Currency::USD)
///     .description("Primary trading portfolio")
///     .scope(PortfolioScope::Regulatory)
///     .add_book("B001")
///     .add_book("B002")
///     .build();
///
/// assert_eq!(portfolio.book_ids().len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct PortfolioBuilder {
    portfolio_id: PortfolioId,
    name: String,
    description: Option<String>,
    parent_portfolio_id: Option<PortfolioId>,
    book_ids: Vec<BookId>,
    metadata: PortfolioMetadata,
}

impl PortfolioBuilder {
    /// Creates a new portfolio builder with required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the portfolio
    /// * `name` - Human-readable name for the portfolio
    /// * `reporting_currency` - Currency for portfolio-level reporting
    #[must_use]
    pub fn new(
        id: impl Into<PortfolioId>,
        name: impl Into<String>,
        reporting_currency: Currency,
    ) -> Self {
        Self {
            portfolio_id: id.into(),
            name: name.into(),
            description: None,
            parent_portfolio_id: None,
            book_ids: Vec::new(),
            metadata: PortfolioMetadata::new(reporting_currency),
        }
    }

    /// Sets the portfolio's description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the parent portfolio ID.
    #[must_use]
    pub fn parent(mut self, parent_id: impl Into<PortfolioId>) -> Self {
        self.parent_portfolio_id = Some(parent_id.into());
        self
    }

    /// Sets the portfolio scope.
    #[must_use]
    pub fn scope(mut self, scope: PortfolioScope) -> Self {
        self.metadata = self.metadata.with_scope(scope);
        self
    }

    /// Adds a book to the portfolio.
    #[must_use]
    pub fn add_book(mut self, book_id: impl Into<BookId>) -> Self {
        let id = book_id.into();
        if !self.book_ids.contains(&id) {
            self.book_ids.push(id);
        }
        self
    }

    /// Adds multiple books to the portfolio.
    #[must_use]
    pub fn add_books(mut self, book_ids: impl IntoIterator<Item = impl Into<BookId>>) -> Self {
        for book_id in book_ids {
            let id = book_id.into();
            if !self.book_ids.contains(&id) {
                self.book_ids.push(id);
            }
        }
        self
    }

    /// Sets the portfolio's metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PortfolioMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builds the PortfolioDefinition instance.
    ///
    /// This method consumes the builder and returns a fully constructed portfolio.
    #[must_use]
    pub fn build(self) -> PortfolioDefinition {
        PortfolioDefinition {
            portfolio_id: self.portfolio_id,
            name: self.name,
            description: self.description,
            parent_portfolio_id: self.parent_portfolio_id,
            book_ids: self.book_ids,
            metadata: self.metadata,
        }
    }

    /// Builds with validation against a set of known book IDs.
    ///
    /// Returns an error if any referenced book ID is not in the known set.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError::InvalidBookReference`] if a book ID is not found.
    pub fn build_validated(
        self,
        known_book_ids: &HashSet<BookId>,
    ) -> Result<PortfolioDefinition, PortfolioError> {
        for book_id in &self.book_ids {
            if !known_book_ids.contains(book_id) {
                return Err(PortfolioError::InvalidBookReference(book_id.to_string()));
            }
        }
        Ok(self.build())
    }

    /// Builds with circular reference detection.
    ///
    /// Validates that setting a parent portfolio does not create a cycle.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioError::CircularReference`] if a cycle is detected.
    pub fn build_with_hierarchy_validation<F>(
        self,
        get_parent: F,
    ) -> Result<PortfolioDefinition, PortfolioError>
    where
        F: Fn(&PortfolioId) -> Option<PortfolioId>,
    {
        if let Some(ref parent_id) = self.parent_portfolio_id {
            let mut visited = HashSet::new();
            visited.insert(self.portfolio_id.clone());

            let mut current = Some(parent_id.clone());
            while let Some(id) = current {
                if visited.contains(&id) {
                    return Err(PortfolioError::CircularReference(
                        self.portfolio_id.to_string(),
                        id.to_string(),
                    ));
                }
                visited.insert(id.clone());
                current = get_parent(&id);
            }
        }
        Ok(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PortfolioDefinition tests
    // ========================================================================

    #[test]
    fn test_portfolio_builder_minimal() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::USD)
            .build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
        assert_eq!(portfolio.name(), "Test Portfolio");
        assert_eq!(portfolio.reporting_currency(), Currency::USD);
        assert!(portfolio.description().is_none());
        assert!(portfolio.parent_portfolio_id().is_none());
        assert!(portfolio.book_ids().is_empty());
    }

    #[test]
    fn test_portfolio_builder_with_description() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::EUR)
            .description("A test portfolio")
            .build();
        assert_eq!(portfolio.description(), Some("A test portfolio"));
    }

    #[test]
    fn test_portfolio_builder_with_parent() {
        let portfolio = PortfolioDefinition::builder("P002", "Child Portfolio", Currency::GBP)
            .parent("P001")
            .build();
        assert!(portfolio.has_parent());
        assert_eq!(portfolio.parent_portfolio_id().unwrap().as_str(), "P001");
    }

    #[test]
    fn test_portfolio_builder_with_scope() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::USD)
            .scope(PortfolioScope::Regulatory)
            .build();
        assert_eq!(portfolio.scope(), PortfolioScope::Regulatory);
    }

    #[test]
    fn test_portfolio_builder_add_book() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::USD)
            .add_book("B001")
            .add_book("B002")
            .build();
        assert_eq!(portfolio.book_ids().len(), 2);
        assert!(portfolio.contains_book(&BookId::new("B001")));
        assert!(portfolio.contains_book(&BookId::new("B002")));
    }

    #[test]
    fn test_portfolio_builder_add_book_dedup() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::USD)
            .add_book("B001")
            .add_book("B001") // Duplicate
            .add_book("B002")
            .build();
        assert_eq!(portfolio.book_ids().len(), 2);
    }

    #[test]
    fn test_portfolio_builder_add_books() {
        let portfolio = PortfolioDefinition::builder("P001", "Test Portfolio", Currency::USD)
            .add_books(["B001", "B002", "B003"])
            .build();
        assert_eq!(portfolio.book_ids().len(), 3);
    }

    #[test]
    fn test_portfolio_builder_full_chain() {
        let portfolio = PortfolioDefinition::builder("P001", "Main Portfolio", Currency::USD)
            .description("Primary trading portfolio")
            .scope(PortfolioScope::Legal)
            .add_book("B001")
            .add_book("B002")
            .build();

        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
        assert_eq!(portfolio.name(), "Main Portfolio");
        assert_eq!(portfolio.description(), Some("Primary trading portfolio"));
        assert_eq!(portfolio.scope(), PortfolioScope::Legal);
        assert_eq!(portfolio.book_ids().len(), 2);
    }

    #[test]
    fn test_portfolio_has_parent() {
        let with_parent = PortfolioDefinition::builder("P002", "Child", Currency::USD)
            .parent("P001")
            .build();
        let without_parent = PortfolioDefinition::builder("P001", "Parent", Currency::USD)
            .build();

        assert!(with_parent.has_parent());
        assert!(!without_parent.has_parent());
    }

    #[test]
    fn test_portfolio_contains_book() {
        let portfolio = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .add_book("B001")
            .build();

        assert!(portfolio.contains_book(&BookId::new("B001")));
        assert!(!portfolio.contains_book(&BookId::new("B999")));
    }

    #[test]
    fn test_portfolio_book_mappings() {
        let portfolio = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .add_book("B001")
            .add_book("B002")
            .build();

        let mappings = portfolio.book_mappings();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].portfolio_id().as_str(), "P001");
        assert_eq!(mappings[0].book_id().as_str(), "B001");
        assert_eq!(mappings[1].book_id().as_str(), "B002");
    }

    #[test]
    fn test_portfolio_clone() {
        let portfolio = PortfolioDefinition::builder("P001", "Test", Currency::EUR)
            .description("Test desc")
            .add_book("B001")
            .build();
        let cloned = portfolio.clone();
        assert_eq!(cloned.portfolio_id().as_str(), "P001");
        assert_eq!(cloned.description(), Some("Test desc"));
    }

    // ========================================================================
    // Validation tests
    // ========================================================================

    #[test]
    fn test_portfolio_build_validated_success() {
        let mut known_books = HashSet::new();
        known_books.insert(BookId::new("B001"));
        known_books.insert(BookId::new("B002"));

        let result = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .add_book("B001")
            .add_book("B002")
            .build_validated(&known_books);

        assert!(result.is_ok());
    }

    #[test]
    fn test_portfolio_build_validated_invalid_book() {
        let mut known_books = HashSet::new();
        known_books.insert(BookId::new("B001"));

        let result = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .add_book("B001")
            .add_book("B999") // Unknown book
            .build_validated(&known_books);

        assert!(result.is_err());
        match result {
            Err(PortfolioError::InvalidBookReference(id)) => assert_eq!(id, "B999"),
            _ => panic!("Expected InvalidBookReference error"),
        }
    }

    #[test]
    fn test_portfolio_build_with_hierarchy_validation_success() {
        // No parent - should succeed
        let result = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .build_with_hierarchy_validation(|_| None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_portfolio_build_with_hierarchy_validation_valid_parent() {
        // P001 has no parent, P002 has P001 as parent - should succeed
        let result = PortfolioDefinition::builder("P002", "Child", Currency::USD)
            .parent("P001")
            .build_with_hierarchy_validation(|id| {
                if id.as_str() == "P001" {
                    None // P001 has no parent
                } else {
                    None
                }
            });

        assert!(result.is_ok());
    }

    #[test]
    fn test_portfolio_build_with_hierarchy_validation_circular() {
        // P001 -> P002 -> P001 would create a cycle
        let result = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .parent("P002")
            .build_with_hierarchy_validation(|id| {
                if id.as_str() == "P002" {
                    Some(PortfolioId::new("P001")) // P002's parent is P001
                } else {
                    None
                }
            });

        assert!(result.is_err());
        match result {
            Err(PortfolioError::CircularReference(from, to)) => {
                assert_eq!(from, "P001");
                assert_eq!(to, "P001");
            }
            _ => panic!("Expected CircularReference error"),
        }
    }

    // ========================================================================
    // PortfolioBuilder tests
    // ========================================================================

    #[test]
    fn test_portfolio_builder_new() {
        let builder = PortfolioBuilder::new("P001", "Test", Currency::USD);
        let portfolio = builder.build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
    }

    #[test]
    fn test_portfolio_builder_clone() {
        let builder = PortfolioDefinition::builder("P001", "Test", Currency::USD)
            .description("Test desc")
            .add_book("B001");
        let cloned = builder.clone();
        let portfolio = cloned.build();
        assert_eq!(portfolio.description(), Some("Test desc"));
    }

    #[test]
    fn test_portfolio_id_from_string() {
        let portfolio = PortfolioDefinition::builder(
            "P001".to_string(),
            "Test Portfolio",
            Currency::USD,
        ).build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
    }

    #[test]
    fn test_portfolio_id_from_portfolio_id() {
        let portfolio_id = PortfolioId::new("P001");
        let portfolio = PortfolioDefinition::builder(
            portfolio_id,
            "Test Portfolio",
            Currency::USD,
        ).build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
    }
}

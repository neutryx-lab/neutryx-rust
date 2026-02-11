//! Portfolio definition and builder.

use std::collections::HashSet;

use bon::Builder;

use super::{PortfolioBookMapping, PortfolioScope};
use crate::{
    error::PortfolioError,
    ids::{BookId, PortfolioId},
    market::Currency,
};

/// A portfolio definition.
#[derive(Clone, Debug, Builder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PortfolioDefinition {
    /// Unique identifier for this portfolio.
    #[builder(into)]
    portfolio_id: PortfolioId,
    /// Human-readable name for this portfolio.
    #[builder(into)]
    name: String,
    /// Reporting currency for portfolio-level calculations.
    reporting_currency: Currency,
    /// Optional description of the portfolio's purpose.
    #[builder(into)]
    description: Option<String>,
    /// Parent portfolio ID for hierarchical structures.
    #[builder(into)]
    parent_portfolio_id: Option<PortfolioId>,
    /// Books assigned to this portfolio.
    #[builder(default)]
    book_ids: Vec<BookId>,
    /// Portfolio scope classification.
    #[builder(default)]
    scope: PortfolioScope,
}

impl PortfolioDefinition {
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
    pub fn parent_portfolio_id(&self) -> Option<&PortfolioId> { self.parent_portfolio_id.as_ref() }

    /// Returns the list of book IDs in this portfolio.
    #[inline]
    #[must_use]
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns the reporting currency.
    #[inline]
    #[must_use]
    pub fn reporting_currency(&self) -> Currency { self.reporting_currency }

    /// Returns the portfolio scope.
    #[inline]
    #[must_use]
    pub fn scope(&self) -> PortfolioScope { self.scope }

    /// Returns true if this portfolio has a parent.
    #[inline]
    #[must_use]
    pub fn has_parent(&self) -> bool { self.parent_portfolio_id.is_some() }

    /// Returns true if this portfolio contains the specified book.
    #[must_use]
    pub fn contains_book(&self, book_id: &BookId) -> bool { self.book_ids.contains(book_id) }

    /// Creates book mappings for this portfolio.
    #[must_use]
    pub fn book_mappings(&self) -> Vec<PortfolioBookMapping> {
        self.book_ids
            .iter()
            .map(|book_id| PortfolioBookMapping::new(self.portfolio_id.clone(), book_id.clone()))
            .collect()
    }

    /// Validates book references against a known set of book IDs.
    pub fn validate_books(&self, known_book_ids: &HashSet<BookId>) -> Result<(), PortfolioError> {
        for book_id in &self.book_ids {
            if !known_book_ids.contains(book_id) {
                return Err(PortfolioError::InvalidBookReference(book_id.to_string()));
            }
        }
        Ok(())
    }

    /// Validates hierarchy to detect circular references.
    pub fn validate_hierarchy<F>(&self, get_parent: F) -> Result<(), PortfolioError>
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_builder_minimal() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test Portfolio")
            .reporting_currency(Currency::USD)
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
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test Portfolio")
            .reporting_currency(Currency::EUR)
            .description("A test portfolio")
            .build();
        assert_eq!(portfolio.description(), Some("A test portfolio"));
    }

    #[test]
    fn test_portfolio_builder_with_parent() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P002")
            .name("Child Portfolio")
            .reporting_currency(Currency::GBP)
            .parent_portfolio_id("P001")
            .build();
        assert!(portfolio.has_parent());
        assert_eq!(portfolio.parent_portfolio_id().unwrap().as_str(), "P001");
    }

    #[test]
    fn test_portfolio_builder_with_scope() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test Portfolio")
            .reporting_currency(Currency::USD)
            .scope(PortfolioScope::Regulatory)
            .build();
        assert_eq!(portfolio.scope(), PortfolioScope::Regulatory);
    }

    #[test]
    fn test_portfolio_builder_with_books() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test Portfolio")
            .reporting_currency(Currency::USD)
            .book_ids(vec![BookId::new("B001"), BookId::new("B002")])
            .build();
        assert_eq!(portfolio.book_ids().len(), 2);
        assert!(portfolio.contains_book(&BookId::new("B001")));
        assert!(portfolio.contains_book(&BookId::new("B002")));
    }

    #[test]
    fn test_portfolio_builder_full_chain() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Main Portfolio")
            .reporting_currency(Currency::USD)
            .description("Primary trading portfolio")
            .scope(PortfolioScope::Legal)
            .book_ids(vec![BookId::new("B001"), BookId::new("B002")])
            .build();

        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
        assert_eq!(portfolio.name(), "Main Portfolio");
        assert_eq!(portfolio.description(), Some("Primary trading portfolio"));
        assert_eq!(portfolio.scope(), PortfolioScope::Legal);
        assert_eq!(portfolio.book_ids().len(), 2);
    }

    #[test]
    fn test_portfolio_has_parent() {
        let with_parent = PortfolioDefinition::builder()
            .portfolio_id("P002")
            .name("Child")
            .reporting_currency(Currency::USD)
            .parent_portfolio_id("P001")
            .build();
        let without_parent = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Parent")
            .reporting_currency(Currency::USD)
            .build();

        assert!(with_parent.has_parent());
        assert!(!without_parent.has_parent());
    }

    #[test]
    fn test_portfolio_contains_book() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .book_ids(vec![BookId::new("B001")])
            .build();

        assert!(portfolio.contains_book(&BookId::new("B001")));
        assert!(!portfolio.contains_book(&BookId::new("B999")));
    }

    #[test]
    fn test_portfolio_book_mappings() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .book_ids(vec![BookId::new("B001"), BookId::new("B002")])
            .build();

        let mappings = portfolio.book_mappings();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].portfolio_id().as_str(), "P001");
        assert_eq!(mappings[0].book_id().as_str(), "B001");
        assert_eq!(mappings[1].book_id().as_str(), "B002");
    }

    #[test]
    fn test_portfolio_clone() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::EUR)
            .description("Test desc")
            .book_ids(vec![BookId::new("B001")])
            .build();
        let cloned = portfolio.clone();
        assert_eq!(cloned.portfolio_id().as_str(), "P001");
        assert_eq!(cloned.description(), Some("Test desc"));
    }

    #[test]
    fn test_portfolio_validate_books_success() {
        let mut known_books = HashSet::new();
        known_books.insert(BookId::new("B001"));
        known_books.insert(BookId::new("B002"));

        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .book_ids(vec![BookId::new("B001"), BookId::new("B002")])
            .build();

        assert!(portfolio.validate_books(&known_books).is_ok());
    }

    #[test]
    fn test_portfolio_validate_books_invalid() {
        let mut known_books = HashSet::new();
        known_books.insert(BookId::new("B001"));

        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .book_ids(vec![BookId::new("B001"), BookId::new("B999")])
            .build();

        let result = portfolio.validate_books(&known_books);
        assert!(result.is_err());
        match result {
            Err(PortfolioError::InvalidBookReference(id)) => assert_eq!(id, "B999"),
            _ => panic!("Expected InvalidBookReference error"),
        }
    }

    #[test]
    fn test_portfolio_validate_hierarchy_success() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .build();

        assert!(portfolio.validate_hierarchy(|_| None).is_ok());
    }

    #[test]
    fn test_portfolio_validate_hierarchy_valid_parent() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P002")
            .name("Child")
            .reporting_currency(Currency::USD)
            .parent_portfolio_id("P001")
            .build();

        let result =
            portfolio.validate_hierarchy(|id| if id.as_str() == "P001" { None } else { None });

        assert!(result.is_ok());
    }

    #[test]
    fn test_portfolio_validate_hierarchy_circular() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001")
            .name("Test")
            .reporting_currency(Currency::USD)
            .parent_portfolio_id("P002")
            .build();

        let result = portfolio.validate_hierarchy(|id| {
            if id.as_str() == "P002" {
                Some(PortfolioId::new("P001"))
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

    #[test]
    fn test_portfolio_id_from_string() {
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id("P001".to_string())
            .name("Test Portfolio")
            .reporting_currency(Currency::USD)
            .build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
    }

    #[test]
    fn test_portfolio_id_from_portfolio_id() {
        let portfolio_id = PortfolioId::new("P001");
        let portfolio = PortfolioDefinition::builder()
            .portfolio_id(portfolio_id)
            .name("Test Portfolio")
            .reporting_currency(Currency::USD)
            .build();
        assert_eq!(portfolio.portfolio_id().as_str(), "P001");
    }
}

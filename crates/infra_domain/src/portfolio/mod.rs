//! Portfolio management module.
//!
//! This module provides comprehensive types for managing portfolios,
//! including portfolio definitions, book mappings, and hierarchy management.
//!
//! Uses `bon::Builder` for fluent construction with compile-time safety.
//!
//! # Module Structure
//!
//! - `portfolio`: PortfolioDefinition entity and builder
//! - `types`: PortfolioScope, PortfolioMetadata, PortfolioBookMapping
//!
//! # Example
//!
//! ```
//! use infra_domain::portfolio::{PortfolioDefinition, PortfolioScope};
//! use infra_domain::market::Currency;
//! use infra_domain::ids::BookId;
//!
//! let portfolio = PortfolioDefinition::builder()
//!     .portfolio_id("P001")
//!     .name("Main Portfolio")
//!     .reporting_currency(Currency::USD)
//!     .description("Primary trading portfolio")
//!     .scope(PortfolioScope::Regulatory)
//!     .book_ids(vec![BookId::new("B001"), BookId::new("B002")])
//!     .build();
//!
//! assert_eq!(portfolio.name(), "Main Portfolio");
//! assert_eq!(portfolio.book_ids().len(), 2);
//! ```

mod portfolio;
mod types;

pub use portfolio::*;
pub use types::*;

/// Prelude for commonly used portfolio types.
pub mod prelude {
    pub use super::{
        PortfolioBookMapping, PortfolioDefinition, PortfolioDefinitionBuilder, PortfolioMetadata,
        PortfolioScope,
    };
}

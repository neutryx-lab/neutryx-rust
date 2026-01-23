//! Portfolio management module.
//!
//! This module provides comprehensive types for managing portfolios,
//! including portfolio definitions, book mappings, and hierarchy management.
//!
//! # Module Structure
//!
//! - `portfolio`: PortfolioDefinition entity and builder
//! - `types`: PortfolioScope, PortfolioMetadata, PortfolioBookMapping
//!
//! # Example
//!
//! ```
//! use infra_master::portfolio::{PortfolioDefinition, PortfolioScope};
//! use infra_master::market::Currency;
//!
//! let portfolio = PortfolioDefinition::builder("P001", "Main Portfolio", Currency::USD)
//!     .description("Primary trading portfolio")
//!     .scope(PortfolioScope::Regulatory)
//!     .add_book("B001")
//!     .add_book("B002")
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
        PortfolioBookMapping, PortfolioBuilder, PortfolioDefinition, PortfolioMetadata,
        PortfolioScope,
    };
}

//! Portfolio management module.

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

//! Data Transfer Objects for REST API
//!
//! Contains request and response types for all API endpoints.
//! Domain-specific DTOs are feature-gated.

mod curves;
mod pricing;

// Feature-gated DTO modules
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "risk")]
pub mod portfolio;
#[cfg(feature = "risk")]
pub mod risk;
#[cfg(feature = "volatility")]
pub mod volatility;

// Re-export common DTOs
pub use curves::*;
#[cfg(feature = "models")]
pub use models::*;
// Feature-gated re-exports
#[cfg(feature = "risk")]
pub use portfolio::*;
pub use pricing::*;
#[cfg(feature = "risk")]
pub use risk::*;
#[cfg(feature = "volatility")]
pub use volatility::*;

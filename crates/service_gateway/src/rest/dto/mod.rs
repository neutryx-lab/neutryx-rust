//! Data Transfer Objects for REST API
//!
//! Contains request and response types for all API endpoints.
//! Domain-specific DTOs are feature-gated.

mod curves;
mod pricing;

// Feature-gated DTO modules
#[cfg(feature = "demo")]
pub mod demo;
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
pub use pricing::*;

// Feature-gated re-exports (public API, may not be used internally)
#[cfg(feature = "demo")]
#[allow(unused_imports)]
pub use demo::*;
#[cfg(feature = "models")]
#[allow(unused_imports)]
pub use models::*;
#[cfg(feature = "risk")]
#[allow(unused_imports)]
pub use portfolio::*;
#[cfg(feature = "risk")]
#[allow(unused_imports)]
pub use risk::*;
#[cfg(feature = "volatility")]
#[allow(unused_imports)]
pub use volatility::*;

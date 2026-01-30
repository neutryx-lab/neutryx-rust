//! REST API handlers
//!
//! Thin handlers that delegate business logic to services.

mod curves;
mod health;
mod pricing;

// Feature-gated handlers
#[cfg(feature = "risk")]
mod portfolio;
#[cfg(feature = "risk")]
mod risk;
#[cfg(feature = "models")]
mod models;
#[cfg(feature = "volatility")]
mod volatility;

pub use curves::*;
pub use health::*;
pub use pricing::*;

// Feature-gated re-exports
#[cfg(feature = "risk")]
pub use portfolio::*;
#[cfg(feature = "risk")]
pub use risk::*;
#[cfg(feature = "models")]
pub use models::*;
#[cfg(feature = "volatility")]
pub use volatility::*;

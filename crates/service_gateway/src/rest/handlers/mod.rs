//! REST API handlers
//!
//! Thin handlers that delegate business logic to services.

mod config;
mod curves;
mod health;
mod pricing;

// Feature-gated handlers
#[cfg(feature = "demo")]
pub mod demo;
#[cfg(feature = "models")]
mod models;
#[cfg(feature = "risk")]
mod portfolio;
#[cfg(feature = "risk")]
mod risk;
#[cfg(feature = "volatility")]
mod volatility;

pub use config::*;
pub use curves::*;
pub use health::*;
pub use pricing::*;

// Feature-gated re-exports
#[cfg(feature = "demo")]
pub use demo::*;
#[cfg(feature = "models")]
pub use models::*;
#[cfg(feature = "risk")]
pub use portfolio::*;
#[cfg(feature = "risk")]
pub use risk::*;
#[cfg(feature = "volatility")]
pub use volatility::*;

//! REST API handlers.

mod config;
mod curves;
mod health;
mod pricing;

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
#[cfg(feature = "models")]
pub use models::*;
#[cfg(feature = "risk")]
pub use portfolio::*;
pub use pricing::*;
#[cfg(feature = "risk")]
pub use risk::*;
#[cfg(feature = "volatility")]
pub use volatility::*;

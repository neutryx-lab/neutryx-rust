//! Data Transfer Objects for REST API.

mod curves;
mod pricing;

#[cfg(feature = "demo")]
pub mod demo;
#[cfg(feature = "demo")]
pub mod exotic;
#[cfg(feature = "demo")]
pub mod incremental_xva;
#[cfg(feature = "demo")]
pub mod jy_inflation;
#[cfg(feature = "demo")]
pub mod mfm;
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "risk")]
pub mod portfolio;
#[cfg(feature = "risk")]
pub mod risk;
#[cfg(feature = "volatility")]
pub mod volatility;
#[cfg(feature = "demo")]
pub mod xva;

pub use curves::*;
#[cfg(feature = "demo")]
#[allow(unused_imports)]
pub use demo::*;
#[cfg(feature = "models")]
#[allow(unused_imports)]
pub use models::*;
#[cfg(feature = "risk")]
#[allow(unused_imports)]
pub use portfolio::*;
pub use pricing::*;
#[cfg(feature = "risk")]
#[allow(unused_imports)]
pub use risk::*;
#[cfg(feature = "volatility")]
#[allow(unused_imports)]
pub use volatility::*;

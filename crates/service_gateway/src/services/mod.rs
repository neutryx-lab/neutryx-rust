//! Service wrappers for facade APIs
//!
//! These services wrap the pricer_* facade APIs and provide
//! convenient methods for use by REST handlers.

mod curve_service;
mod pricing_service;

// Feature-gated services
#[cfg(feature = "demo")]
mod demo_service;
#[cfg(feature = "models")]
mod model_service;
#[cfg(feature = "risk")]
mod portfolio_service;
#[cfg(feature = "risk")]
mod risk_service;
#[cfg(feature = "demo")]
mod volcube_service;
#[cfg(feature = "volatility")]
mod volatility_service;

pub use curve_service::CurveService;
// Feature-gated re-exports
#[cfg(feature = "demo")]
pub use demo_service::DemoService;
#[cfg(feature = "models")]
pub use model_service::ModelService;
#[cfg(feature = "risk")]
pub use portfolio_service::PortfolioService;
pub use pricing_service::PricingService;
#[cfg(feature = "risk")]
pub use risk_service::RiskService;
#[cfg(feature = "demo")]
pub use volcube_service::VolcubeService;
#[cfg(feature = "volatility")]
pub use volatility_service::VolatilityService;

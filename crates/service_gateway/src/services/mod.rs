//! Service wrappers for facade APIs.

mod chart_grid;
mod curve_service;
pub mod helpers;
mod pricing_service;

#[cfg(feature = "demo")]
mod demo;
#[cfg(feature = "models")]
mod model_service;
#[cfg(feature = "risk")]
mod portfolio_service;
#[cfg(feature = "risk")]
mod risk_service;
#[cfg(feature = "volatility")]
mod volatility_service;
#[cfg(feature = "demo")]
mod volcube_service;

pub use curve_service::CurveService;
#[cfg(feature = "demo")]
pub use demo::DemoService;
#[cfg(feature = "models")]
pub use model_service::ModelService;
#[cfg(feature = "risk")]
pub use portfolio_service::PortfolioService;
pub use pricing_service::PricingService;
#[cfg(feature = "risk")]
pub use risk_service::RiskService;
#[cfg(feature = "volatility")]
pub use volatility_service::VolatilityService;
#[cfg(feature = "demo")]
pub use volcube_service::VolcubeService;

//! Service wrappers for facade APIs.

mod chart_grid;
mod curve_service;
pub mod helpers;
mod pricing_service;

#[cfg(feature = "demo")]
mod demo;
#[cfg(feature = "demo")]
mod exotic_service;
#[cfg(feature = "demo")]
mod mfm_service;
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
#[cfg(feature = "demo")]
mod incremental_xva_service;
#[cfg(feature = "demo")]
mod xva_service;

pub use curve_service::CurveService;
#[cfg(feature = "demo")]
pub use demo::DemoService;
#[cfg(feature = "demo")]
pub use exotic_service::ExoticService;
#[cfg(feature = "demo")]
pub use mfm_service::MfmService;
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
#[cfg(feature = "demo")]
pub use incremental_xva_service::IncrementalXvaService;
#[cfg(feature = "demo")]
pub use xva_service::XvaService;

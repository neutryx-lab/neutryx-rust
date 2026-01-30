//! Application state management for service_gateway
//!
//! Provides shared state including caches, pricers, and risk engines.
//! Feature-gated caches are included based on enabled features.

mod cache;

use std::sync::Arc;

// Re-export common cache types
pub use cache::{CurveCache, FxVolCache, InstrumentInput, SabrParams};

// Feature-gated re-exports
#[cfg(feature = "risk")]
pub use cache::{PortfolioCache, PortfolioEntry};
#[cfg(feature = "models")]
pub use cache::{ModelCache, ModelEntry, ModelType};
#[cfg(feature = "volatility")]
pub use cache::{VolSurfaceCache, VolSurfaceEntry, VolSurfaceType};

use pricer_pricing::generic_pricer::{GenericPricer, ModelConfig, PricerConfig};

/// Configuration for AppState cache sizes
#[derive(Debug, Clone)]
pub struct AppStateConfig {
    /// Curve cache capacity
    pub curve_cache_size: usize,
    /// FX vol cache capacity
    pub fxvol_cache_size: usize,
    /// Portfolio cache capacity (risk feature)
    #[cfg(feature = "risk")]
    pub portfolio_cache_size: usize,
    /// Model cache capacity (models feature)
    #[cfg(feature = "models")]
    pub model_cache_size: usize,
    /// Vol surface cache capacity (volatility feature)
    #[cfg(feature = "volatility")]
    pub vol_surface_cache_size: usize,
}

impl Default for AppStateConfig {
    fn default() -> Self {
        Self {
            curve_cache_size: 100,
            fxvol_cache_size: 20,
            #[cfg(feature = "risk")]
            portfolio_cache_size: 50,
            #[cfg(feature = "models")]
            model_cache_size: 20,
            #[cfg(feature = "volatility")]
            vol_surface_cache_size: 20,
        }
    }
}

/// Application state shared across all handlers
pub struct AppState {
    /// Cache for bootstrapped curves
    pub curve_cache: CurveCache,
    /// Cache for FX volatility surfaces
    pub fxvol_cache: FxVolCache,
    /// Pre-configured generic pricer for standalone pricing
    pub pricer: Arc<GenericPricer>,

    // Feature-gated caches (Requirement 11)

    /// Cache for portfolios (risk feature)
    #[cfg(feature = "risk")]
    pub portfolio_cache: PortfolioCache,
    /// Cache for stochastic models (models feature)
    #[cfg(feature = "models")]
    pub model_cache: ModelCache,
    /// Cache for volatility surfaces/cubes (volatility feature)
    #[cfg(feature = "volatility")]
    pub vol_surface_cache: VolSurfaceCache,
}

impl AppState {
    /// Create a new application state with default configuration
    pub fn new() -> Self {
        Self::with_config(AppStateConfig::default())
    }

    /// Create application state with custom configuration
    pub fn with_config(config: AppStateConfig) -> Self {
        let model_config = ModelConfig::builder()
            .num_paths(10_000)
            .num_steps(100)
            .build()
            .expect("valid model config");

        let pricer_config = PricerConfig::builder()
            .build()
            .expect("valid pricer config");

        let pricer = GenericPricer::new_standalone(model_config, pricer_config);

        Self {
            curve_cache: CurveCache::new(config.curve_cache_size),
            fxvol_cache: FxVolCache::new(config.fxvol_cache_size),
            pricer: Arc::new(pricer),
            #[cfg(feature = "risk")]
            portfolio_cache: PortfolioCache::new(config.portfolio_cache_size),
            #[cfg(feature = "models")]
            model_cache: ModelCache::new(config.model_cache_size),
            #[cfg(feature = "volatility")]
            vol_surface_cache: VolSurfaceCache::new(config.vol_surface_cache_size),
        }
    }

    /// Create application state with custom cache sizes (legacy API)
    pub fn with_cache_sizes(curve_cache_size: usize, fxvol_cache_size: usize) -> Self {
        let mut config = AppStateConfig::default();
        config.curve_cache_size = curve_cache_size;
        config.fxvol_cache_size = fxvol_cache_size;
        Self::with_config(config)
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        let state = AppState::new();
        assert!(state.curve_cache.is_empty());
        assert!(state.fxvol_cache.is_empty());
    }

    #[test]
    fn test_app_state_with_config() {
        let config = AppStateConfig {
            curve_cache_size: 50,
            fxvol_cache_size: 10,
            #[cfg(feature = "risk")]
            portfolio_cache_size: 25,
            #[cfg(feature = "models")]
            model_cache_size: 15,
            #[cfg(feature = "volatility")]
            vol_surface_cache_size: 15,
        };
        let state = AppState::with_config(config);
        assert!(state.curve_cache.is_empty());
    }

    #[cfg(feature = "risk")]
    #[test]
    fn test_app_state_portfolio_cache() {
        let state = AppState::new();
        assert!(state.portfolio_cache.is_empty());
    }

    #[cfg(feature = "models")]
    #[test]
    fn test_app_state_model_cache() {
        let state = AppState::new();
        assert!(state.model_cache.is_empty());
    }

    #[cfg(feature = "volatility")]
    #[test]
    fn test_app_state_vol_surface_cache() {
        let state = AppState::new();
        assert!(state.vol_surface_cache.is_empty());
    }
}

//! Application state management for service_gateway
//!
//! Provides shared state including caches, pricers, and risk engines.

mod cache;

use std::sync::Arc;

pub use cache::{CurveCache, CurveEntry, FxVolCache, FxVolEntry, InstrumentInput, SabrParams};
use pricer_pricing::generic_pricer::{GenericPricer, ModelConfig, PricerConfig};

/// Application state shared across all handlers
pub struct AppState {
    /// Cache for bootstrapped curves
    pub curve_cache: CurveCache,
    /// Cache for FX volatility surfaces
    pub fxvol_cache: FxVolCache,
    /// Pre-configured generic pricer for standalone pricing
    pub pricer: Arc<GenericPricer>,
}

impl AppState {
    /// Create a new application state with default configuration
    pub fn new() -> Self {
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
            curve_cache: CurveCache::new(100),
            fxvol_cache: FxVolCache::new(20),
            pricer: Arc::new(pricer),
        }
    }

    /// Create application state with custom cache sizes
    pub fn with_cache_sizes(curve_cache_size: usize, fxvol_cache_size: usize) -> Self {
        let mut state = Self::new();
        state.curve_cache = CurveCache::new(curve_cache_size);
        state.fxvol_cache = FxVolCache::new(fxvol_cache_size);
        state
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

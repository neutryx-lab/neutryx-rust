//! Curve construction engine.
//!
//! This module provides `CurveEngine`, the main orchestration layer for
//! constructing yield curves from market data, with integrated caching
//! and sensitivity calculation support.
//!
//! ## Architecture
//!
//! ```text
//! CurveEngine
//! ├── InstrumentAdapter: CurveDefinition + rates → BootstrapInstrument[]
//! ├── SequentialBootstrapper: BootstrapInstrument[] → BootstrappedCurve
//! ├── CurveResultCache: Optional caching of built curves
//! └── SensitivityBootstrapper: Optional sensitivity computation
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use pricer_models::market::calibration::bootstrapping::{
//!     CurveEngine, CurveDefinition, CurveConfig, InstrumentTenor,
//! };
//!
//! let engine = CurveEngine::new();
//! let definition = CurveDefinition::default_usd_sofr();
//! let rates = vec![
//!     (InstrumentTenor::ThreeMonths, 0.03),
//!     (InstrumentTenor::OneYear, 0.035),
//! ];
//!
//! let result = engine.build_curve(&definition, &rates)?;
//! ```

use num_traits::Float;

use super::{
    adapter::InstrumentAdapter,
    config::GenericBootstrapConfig,
    curve::BootstrappedCurve,
    curve_config::CurveConfig,
    definition::{CurveDefinition, InstrumentTenor},
    engine::{GenericBootstrapResult, SequentialBootstrapper},
    engine_error::CurveEngineError,
    result_cache::{CurveKey, CurveResultCache},
    sensitivity::{BootstrapResultWithSensitivities, SensitivityBootstrapper},
};

/// Result of curve construction including diagnostics.
#[derive(Debug, Clone)]
pub struct CurveConstructionResult<T: Float> {
    /// The constructed curve.
    pub curve: BootstrappedCurve<T>,
    /// Pillar maturities in years.
    pub pillars: Vec<T>,
    /// Discount factors at each pillar.
    pub discount_factors: Vec<T>,
    /// Residual at each pillar (pricing error).
    pub residuals: Vec<T>,
    /// Number of solver iterations per pillar.
    pub iterations: Vec<usize>,
    /// Whether the result was retrieved from cache.
    pub from_cache: bool,
}

impl<T: Float> From<GenericBootstrapResult<T>> for CurveConstructionResult<T> {
    fn from(result: GenericBootstrapResult<T>) -> Self {
        Self {
            curve: result.curve,
            pillars: result.pillars,
            discount_factors: result.discount_factors,
            residuals: result.residuals,
            iterations: result.iterations,
            from_cache: false,
        }
    }
}

/// Curve construction engine with integrated caching and sensitivity support.
///
/// `CurveEngine` provides a high-level API for constructing yield curves from
/// curve definitions and market rates. It orchestrates:
///
/// - Instrument conversion via `InstrumentAdapter`
/// - Curve bootstrapping via `SequentialBootstrapper`
/// - Optional result caching via `CurveResultCache`
/// - Optional sensitivity calculation via `SensitivityBootstrapper`
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ## Basic Usage (No Caching)
///
/// ```ignore
/// let engine = CurveEngine::new();
/// let result = engine.build_curve(&definition, &rates)?;
/// ```
///
/// ## With Caching
///
/// ```ignore
/// let engine = CurveEngine::with_cache(100);
/// let result1 = engine.build_curve(&definition, &rates)?;
/// let result2 = engine.build_curve(&definition, &rates)?; // Cache hit
/// assert!(result2.from_cache);
/// ```
///
/// ## With Sensitivities
///
/// ```ignore
/// let engine = CurveEngine::new();
/// let result = engine.build_curve_with_sensitivities(&definition, &rates)?;
/// let jacobian = result.sensitivities; // dDF/dRate matrix
/// ```
#[derive(Debug)]
pub struct CurveEngine<T: Float> {
    /// Bootstrap configuration.
    bootstrap_config: GenericBootstrapConfig<T>,
    /// Extended curve configuration.
    curve_config: CurveConfig<T>,
    /// Optional result cache.
    cache: Option<CurveResultCache<T>>,
}

impl<T: Float> Clone for CurveEngine<T> {
    fn clone(&self) -> Self {
        Self {
            bootstrap_config: self.bootstrap_config.clone(),
            curve_config: self.curve_config.clone(),
            cache: None, // Do not clone cache
        }
    }
}

impl<T: Float> Default for CurveEngine<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float> CurveEngine<T> {
    /// Create a new curve engine with default configuration.
    pub fn new() -> Self {
        Self {
            bootstrap_config: GenericBootstrapConfig::default(),
            curve_config: CurveConfig::default(),
            cache: None,
        }
    }

    /// Create a curve engine with custom bootstrap configuration.
    pub fn with_config(bootstrap_config: GenericBootstrapConfig<T>) -> Self {
        Self {
            bootstrap_config,
            curve_config: CurveConfig::default(),
            cache: None,
        }
    }

    /// Create a curve engine with caching enabled.
    ///
    /// # Arguments
    ///
    /// * `cache_size` - Maximum number of curves to cache
    pub fn with_cache(cache_size: usize) -> Self {
        Self {
            bootstrap_config: GenericBootstrapConfig::default(),
            curve_config: CurveConfig::default(),
            cache: Some(CurveResultCache::new(cache_size)),
        }
    }

    /// Create a curve engine with both custom configuration and caching.
    pub fn with_config_and_cache(
        bootstrap_config: GenericBootstrapConfig<T>,
        cache_size: usize,
    ) -> Self {
        Self {
            bootstrap_config,
            curve_config: CurveConfig::default(),
            cache: Some(CurveResultCache::new(cache_size)),
        }
    }

    /// Set the extended curve configuration.
    pub fn with_curve_config(mut self, curve_config: CurveConfig<T>) -> Self {
        self.curve_config = curve_config;
        self
    }

    /// Get the bootstrap configuration.
    pub fn bootstrap_config(&self) -> &GenericBootstrapConfig<T> { &self.bootstrap_config }

    /// Get the extended curve configuration.
    pub fn curve_config(&self) -> &CurveConfig<T> { &self.curve_config }

    /// Check if caching is enabled.
    pub fn has_cache(&self) -> bool { self.cache.is_some() }

    /// Get cache statistics if caching is enabled.
    pub fn cache_stats(&self) -> Option<super::result_cache::CacheStats> {
        self.cache.as_ref().map(|c| c.stats())
    }

    /// Clear the cache if caching is enabled.
    pub fn clear_cache(&self) {
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
    }

    /// Build a yield curve from a curve definition and market rates.
    ///
    /// This is the main entry point for curve construction. It:
    /// 1. Checks the cache for an existing curve (if caching enabled)
    /// 2. Converts the definition and rates to bootstrap instruments
    /// 3. Performs sequential bootstrapping
    /// 4. Caches the result (if caching enabled)
    ///
    /// # Arguments
    ///
    /// * `definition` - Curve definition specifying instruments and conventions
    /// * `rates` - Market rates for each instrument tenor
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Successfully constructed curve with diagnostics
    /// * `Err(e)` - If construction fails (validation, conversion, or bootstrap
    ///   error)
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError` if:
    /// - Rate count doesn't match instrument count
    /// - Instrument conversion fails
    /// - Bootstrap fails to converge
    pub fn build_curve(
        &self,
        definition: &CurveDefinition,
        rates: &[(InstrumentTenor, T)],
    ) -> Result<CurveConstructionResult<T>, CurveEngineError> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            let key = self.create_cache_key(definition, rates);
            if let Some(cached_curve) = cache.lookup(&key) {
                let pillar_count = cached_curve.pillar_count();
                let pillars = cached_curve.pillars().to_vec();
                let discount_factors = cached_curve.discount_factors_at_pillars().to_vec();
                return Ok(CurveConstructionResult {
                    curve: cached_curve.clone(),
                    pillars,
                    discount_factors,
                    residuals: vec![T::zero(); pillar_count],
                    iterations: vec![0; pillar_count],
                    from_cache: true,
                });
            }
        }

        // Convert definition and rates to bootstrap instruments
        let instruments = InstrumentAdapter::convert(definition, rates)?;

        // Perform bootstrap
        let bootstrapper = SequentialBootstrapper::new(self.bootstrap_config.clone());
        let result = bootstrapper
            .bootstrap(&instruments)
            .map_err(CurveEngineError::Bootstrap)?;

        // Cache the result
        if let Some(ref cache) = self.cache {
            let key = self.create_cache_key(definition, rates);
            cache.insert(key, result.curve.clone());
        }

        Ok(CurveConstructionResult::from(result))
    }

    /// Build a curve directly from bootstrap instruments.
    ///
    /// Use this method when you have already converted instruments,
    /// bypassing the definition-based conversion.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Pre-converted bootstrap instruments
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Successfully constructed curve
    /// * `Err(e)` - If bootstrap fails
    pub fn build_curve_from_instruments(
        &self,
        instruments: &[super::instrument::BootstrapInstrument<T>],
    ) -> Result<CurveConstructionResult<T>, CurveEngineError> {
        let bootstrapper = SequentialBootstrapper::new(self.bootstrap_config.clone());
        let result = bootstrapper
            .bootstrap(instruments)
            .map_err(CurveEngineError::Bootstrap)?;

        Ok(CurveConstructionResult::from(result))
    }

    /// Create a cache key from definition and rates.
    fn create_cache_key(
        &self,
        definition: &CurveDefinition,
        rates: &[(InstrumentTenor, T)],
    ) -> CurveKey {
        // Extract f64 rates for hashing
        let f64_rates: Vec<f64> = rates
            .iter()
            .map(|(_, r)| r.to_f64().unwrap_or(0.0))
            .collect();

        // Simple config hash based on interpolation and tolerance
        let config_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", self.bootstrap_config.interpolation).hash(&mut hasher);
            hasher.finish()
        };

        CurveKey::from_rates(definition.rate_index(), &f64_rates, config_hash)
    }
}

// f64-specific implementations for sensitivity calculations
impl CurveEngine<f64> {
    /// Build a curve with sensitivity calculation.
    ///
    /// Computes the Jacobian matrix d(DF_i)/d(rate_j) for all pillars
    /// and input rates using bump-and-revalue methodology.
    ///
    /// # Arguments
    ///
    /// * `definition` - Curve definition
    /// * `rates` - Market rates
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Curve with sensitivity matrix
    /// * `Err(e)` - If construction fails
    ///
    /// # Note
    ///
    /// This method does not use caching since sensitivities may vary
    /// with configuration changes.
    pub fn build_curve_with_sensitivities(
        &self,
        definition: &CurveDefinition,
        rates: &[(InstrumentTenor, f64)],
    ) -> Result<BootstrapResultWithSensitivities, CurveEngineError> {
        // Convert instruments
        let instruments = InstrumentAdapter::convert(definition, rates)?;

        // Build with sensitivities
        let bootstrapper = SensitivityBootstrapper::new(self.bootstrap_config.clone());
        let result = bootstrapper
            .bootstrap_with_bump_and_revalue(&instruments)
            .map_err(CurveEngineError::Bootstrap)?;

        Ok(result)
    }
}

/// Builder for `CurveEngine` with fluent API.
#[derive(Debug, Clone)]
pub struct CurveEngineBuilder<T: Float> {
    bootstrap_config: GenericBootstrapConfig<T>,
    curve_config: CurveConfig<T>,
    cache_size: Option<usize>,
}

impl<T: Float> Default for CurveEngineBuilder<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float> CurveEngineBuilder<T> {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            bootstrap_config: GenericBootstrapConfig::default(),
            curve_config: CurveConfig::default(),
            cache_size: None,
        }
    }

    /// Set the bootstrap configuration.
    pub fn bootstrap_config(mut self, config: GenericBootstrapConfig<T>) -> Self {
        self.bootstrap_config = config;
        self
    }

    /// Set the extended curve configuration.
    pub fn curve_config(mut self, config: CurveConfig<T>) -> Self {
        self.curve_config = config;
        self
    }

    /// Enable caching with the specified size.
    pub fn with_cache(mut self, size: usize) -> Self {
        self.cache_size = Some(size);
        self
    }

    /// Disable caching.
    pub fn without_cache(mut self) -> Self {
        self.cache_size = None;
        self
    }

    /// Set the tolerance for bootstrap convergence.
    pub fn tolerance(mut self, tol: T) -> Self {
        self.bootstrap_config.tolerance = tol;
        self
    }

    /// Set the maximum iterations for bootstrap solver.
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.bootstrap_config.max_iterations = max;
        self
    }

    /// Build the curve engine.
    pub fn build(self) -> CurveEngine<T> {
        CurveEngine {
            bootstrap_config: self.bootstrap_config,
            curve_config: self.curve_config,
            cache: self.cache_size.map(CurveResultCache::new),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::curves::YieldCurve;

    // ========================================
    // Helper Functions
    // ========================================

    fn create_simple_definition() -> CurveDefinition {
        // Use the default USD-SOFR definition with only the first 3 instruments
        CurveDefinition::default_usd_sofr()
    }

    fn create_simple_rates() -> Vec<(InstrumentTenor, f64)> {
        // Must match the instruments in default_usd_sofr (12 instruments)
        vec![
            (InstrumentTenor::OneMonth, 0.025),
            (InstrumentTenor::ThreeMonths, 0.028),
            (InstrumentTenor::SixMonths, 0.030),
            (InstrumentTenor::OneYear, 0.032),
            (InstrumentTenor::TwoYears, 0.034),
            (InstrumentTenor::ThreeYears, 0.036),
            (InstrumentTenor::FiveYears, 0.038),
            (InstrumentTenor::SevenYears, 0.040),
            (InstrumentTenor::TenYears, 0.042),
            (InstrumentTenor::FifteenYears, 0.044),
            (InstrumentTenor::TwentyYears, 0.045),
            (InstrumentTenor::ThirtyYears, 0.046),
        ]
    }

    // ========================================
    // Basic Construction Tests
    // ========================================

    #[test]
    fn test_engine_new() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        assert!(!engine.has_cache());
    }

    #[test]
    fn test_engine_with_cache() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        assert!(engine.has_cache());
    }

    #[test]
    fn test_engine_default() {
        let engine: CurveEngine<f64> = CurveEngine::default();
        assert!(!engine.has_cache());
    }

    #[test]
    fn test_engine_clone() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let cloned = engine.clone();
        // Cloned engine should not have cache
        assert!(!cloned.has_cache());
    }

    // ========================================
    // Curve Building Tests
    // ========================================

    #[test]
    fn test_build_curve_basic() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve(&definition, &rates);
        assert!(result.is_ok(), "Build should succeed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 12);
        assert!(!result.from_cache);
    }

    #[test]
    fn test_build_curve_discount_factors_decreasing() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve(&definition, &rates).unwrap();

        // Discount factors should be decreasing
        for i in 1..result.discount_factors.len() {
            assert!(
                result.discount_factors[i] < result.discount_factors[i - 1],
                "DF[{}] = {} should be < DF[{}] = {}",
                i,
                result.discount_factors[i],
                i - 1,
                result.discount_factors[i - 1]
            );
        }
    }

    #[test]
    fn test_build_curve_residuals_small() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve(&definition, &rates).unwrap();

        // Residuals should be very small
        for (i, residual) in result.residuals.iter().enumerate() {
            assert!(
                residual.abs() < 1e-8,
                "Residual[{}] = {} should be near zero",
                i,
                residual
            );
        }
    }

    #[test]
    fn test_build_curve_uses_yield_curve_trait() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve(&definition, &rates).unwrap();

        // Test YieldCurve trait methods
        let df_1y = result.curve.discount_factor(1.0).unwrap();
        let df_2y = result.curve.discount_factor(2.0).unwrap();
        assert!(df_1y > df_2y);

        // Zero rate at 1Y should be approximately 3%
        let zero_rate = result.curve.zero_rate(1.0).unwrap();
        assert!(
            (zero_rate - 0.03).abs() < 0.005,
            "Zero rate at 1Y should be ~3%, got {}",
            zero_rate
        );
    }

    // ========================================
    // Caching Tests
    // ========================================

    #[test]
    fn test_build_curve_cache_miss() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve(&definition, &rates).unwrap();
        assert!(!result.from_cache, "First build should be a cache miss");
    }

    #[test]
    fn test_build_curve_cache_hit() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        // First build
        let result1 = engine.build_curve(&definition, &rates).unwrap();
        assert!(!result1.from_cache);

        // Second build should hit cache
        let result2 = engine.build_curve(&definition, &rates).unwrap();
        assert!(result2.from_cache, "Second build should be a cache hit");
    }

    #[test]
    fn test_cache_stats() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        // Initial stats
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // Build once (miss)
        let _ = engine.build_curve(&definition, &rates);
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);

        // Build again (hit)
        let _ = engine.build_curve(&definition, &rates);
        let stats = engine.cache_stats().unwrap();
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_clear_cache() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        // Build and cache
        let _ = engine.build_curve(&definition, &rates);
        assert_eq!(engine.cache_stats().unwrap().entries, 1);

        // Clear cache
        engine.clear_cache();
        assert_eq!(engine.cache_stats().unwrap().entries, 0);

        // Build again should miss
        let result = engine.build_curve(&definition, &rates).unwrap();
        assert!(!result.from_cache);
    }

    #[test]
    fn test_different_rates_different_cache_entries() {
        let engine: CurveEngine<f64> = CurveEngine::with_cache(100);
        let definition = create_simple_definition();

        // First set of rates
        let rates1 = create_simple_rates();

        // Second set with different values
        let rates2: Vec<(InstrumentTenor, f64)> = vec![
            (InstrumentTenor::OneMonth, 0.020),
            (InstrumentTenor::ThreeMonths, 0.023),
            (InstrumentTenor::SixMonths, 0.025),
            (InstrumentTenor::OneYear, 0.027),
            (InstrumentTenor::TwoYears, 0.029),
            (InstrumentTenor::ThreeYears, 0.031),
            (InstrumentTenor::FiveYears, 0.033),
            (InstrumentTenor::SevenYears, 0.035),
            (InstrumentTenor::TenYears, 0.037),
            (InstrumentTenor::FifteenYears, 0.039),
            (InstrumentTenor::TwentyYears, 0.040),
            (InstrumentTenor::ThirtyYears, 0.041),
        ];

        // Build with first rates
        let _ = engine.build_curve(&definition, &rates1);
        let _ = engine.build_curve(&definition, &rates2);

        // Should have 2 cache entries
        assert_eq!(engine.cache_stats().unwrap().entries, 2);
    }

    // ========================================
    // Builder Pattern Tests
    // ========================================

    #[test]
    fn test_engine_builder_basic() {
        let engine: CurveEngine<f64> = CurveEngineBuilder::new().build();
        assert!(!engine.has_cache());
    }

    #[test]
    fn test_engine_builder_with_cache() {
        let engine: CurveEngine<f64> = CurveEngineBuilder::new().with_cache(50).build();
        assert!(engine.has_cache());
    }

    #[test]
    fn test_engine_builder_tolerance() {
        let engine: CurveEngine<f64> = CurveEngineBuilder::new().tolerance(1e-14).build();
        assert!((engine.bootstrap_config().tolerance - 1e-14).abs() < 1e-20);
    }

    #[test]
    fn test_engine_builder_max_iterations() {
        let engine: CurveEngine<f64> = CurveEngineBuilder::new().max_iterations(200).build();
        assert_eq!(engine.bootstrap_config().max_iterations, 200);
    }

    // ========================================
    // Sensitivity Tests
    // ========================================

    #[test]
    fn test_build_curve_with_sensitivities() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine.build_curve_with_sensitivities(&definition, &rates);
        assert!(
            result.is_ok(),
            "Sensitivity build should succeed: {:?}",
            result.err()
        );

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 12);
        assert_eq!(result.sensitivities.len(), 12);

        // Diagonal elements should be negative (higher rate -> lower DF)
        for i in 0..result.sensitivities.len() {
            assert!(
                result.sensitivities[i][i] < 0.0,
                "Diagonal sensitivity [{i}][{i}] = {} should be negative",
                result.sensitivities[i][i]
            );
        }
    }

    #[test]
    fn test_sensitivities_triangular_structure() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();
        let rates = create_simple_rates();

        let result = engine
            .build_curve_with_sensitivities(&definition, &rates)
            .unwrap();

        // Sequential bootstrap creates lower-triangular sensitivity structure
        // DF_0 shouldn't depend on rates at later indices
        for j in 1..result.sensitivities[0].len() {
            assert!(
                result.sensitivities[0][j].abs() < 1e-10,
                "DF_0 shouldn't depend on rate_{j}"
            );
        }
    }

    // ========================================
    // Error Handling Tests
    // ========================================

    #[test]
    fn test_build_curve_mismatched_rates() {
        let engine: CurveEngine<f64> = CurveEngine::new();
        let definition = create_simple_definition();

        // Provide only 2 rates for 12 instruments (definition has 12)
        let rates = vec![
            (InstrumentTenor::OneMonth, 0.03),
            (InstrumentTenor::ThreeMonths, 0.035),
        ];

        let result = engine.build_curve(&definition, &rates);
        assert!(result.is_err());
    }

    // ========================================
    // Direct Instrument Building Tests
    // ========================================

    #[test]
    fn test_build_curve_from_instruments() {
        use crate::market::calibration::bootstrapping::BootstrapInstrument;

        let engine: CurveEngine<f64> = CurveEngine::new();
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.035),
            BootstrapInstrument::ois(3.0, 0.04),
        ];

        let result = engine.build_curve_from_instruments(&instruments);
        assert!(result.is_ok(), "Build should succeed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.pillars.len(), 3);
    }
}

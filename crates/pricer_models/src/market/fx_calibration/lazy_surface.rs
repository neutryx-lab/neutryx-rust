//! Lazy FX Volatility Surface with deferred calibration.
//!
//! This module provides a lazy wrapper around [`CalibratedFxVolSurface`] that
//! defers calibration until the first volatility query, with thread-safe caching.

use std::sync::{Arc, RwLock};

use num_traits::Float;

use crate::market::surfaces::traits::VolatilitySurface;

use super::config::FxVolSurfaceConfig;
use super::surface::{CalibratedFxVolSurface, VolSmile, VolSurfaceError};
use super::types::Strike;
use super::vol_builder::{CalibrationDiagnostics, CalibrationError, FxVolSurfaceBuilder};

/// Statistics for cache usage.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: usize,
    /// Number of cache misses (including first access).
    pub misses: usize,
    /// Number of explicit invalidations.
    pub invalidations: usize,
}

impl CacheStats {
    /// Creates new empty cache statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// State of the lazy surface.
enum LazyState<T: Float> {
    /// Surface has not been calibrated yet. Builder is ready.
    Pending(FxVolSurfaceBuilder<T>),
    /// Surface has been calibrated.
    Calibrated {
        /// The calibrated surface.
        surface: CalibratedFxVolSurface<T>,
        /// Calibration diagnostics.
        diagnostics: CalibrationDiagnostics,
    },
    /// Calibration failed.
    Failed(CalibrationError),
}

/// Thread-safe interior state wrapper.
struct LazyInner<T: Float> {
    /// Current state.
    state: LazyState<T>,
    /// Cache statistics.
    stats: CacheStats,
}

/// Lazy FX volatility surface with deferred calibration.
///
/// This wrapper delays the actual calibration until the first volatility
/// query is made. Once calibrated, the results are cached for subsequent
/// queries. The surface supports thread-safe access via interior mutability.
///
/// # Example
///
/// ```ignore
/// let lazy_surface = LazyFxVolSurface::new(builder);
///
/// // Calibration happens on first vol() call
/// let vol = lazy_surface.vol(strike, expiry)?;
///
/// // Subsequent calls use cached surface
/// let vol2 = lazy_surface.vol(strike2, expiry)?;
///
/// // Check cache statistics
/// let stats = lazy_surface.cache_stats();
/// ```
pub struct LazyFxVolSurface<T: Float> {
    /// Interior state with thread-safe access.
    inner: Arc<RwLock<LazyInner<T>>>,
}

impl<T: Float + Send + Sync + 'static> LazyFxVolSurface<T> {
    /// Creates a new lazy surface from a builder.
    ///
    /// The builder will be used to calibrate the surface on the first
    /// volatility query.
    pub fn new(builder: FxVolSurfaceBuilder<T>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LazyInner {
                state: LazyState::Pending(builder),
                stats: CacheStats::new(),
            })),
        }
    }

    /// Returns whether the surface has been calibrated.
    pub fn is_calibrated(&self) -> bool {
        let inner = self.inner.read().expect("Lock poisoned");
        matches!(inner.state, LazyState::Calibrated { .. })
    }

    /// Returns whether calibration has failed.
    pub fn has_failed(&self) -> bool {
        let inner = self.inner.read().expect("Lock poisoned");
        matches!(inner.state, LazyState::Failed(_))
    }

    /// Returns a clone of the cache statistics.
    pub fn cache_stats(&self) -> CacheStats {
        let inner = self.inner.read().expect("Lock poisoned");
        inner.stats.clone()
    }

    /// Returns calibration diagnostics if calibration has completed.
    pub fn diagnostics(&self) -> Option<CalibrationDiagnostics> {
        let inner = self.inner.read().expect("Lock poisoned");
        if let LazyState::Calibrated { diagnostics, .. } = &inner.state {
            Some(diagnostics.clone())
        } else {
            None
        }
    }

    /// Forces calibration and returns the result.
    ///
    /// If already calibrated, returns the cached result.
    /// If calibration fails, stores and returns the error.
    pub fn force_calibrate(&self) -> Result<(), CalibrationError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)?;
        Ok(())
    }

    /// Invalidates the cache and resets to pending state.
    ///
    /// The next volatility query will trigger re-calibration.
    ///
    /// # Arguments
    ///
    /// * `new_builder` - Optional new builder to use. If None, re-calibration
    ///   will fail unless the original builder is still valid.
    pub fn invalidate(&self, new_builder: Option<FxVolSurfaceBuilder<T>>) {
        let mut inner = self.inner.write().expect("Lock poisoned");
        inner.stats.invalidations += 1;

        if let Some(builder) = new_builder {
            inner.state = LazyState::Pending(builder);
        } else {
            // Mark as failed if no new builder provided
            inner.state = LazyState::Failed(CalibrationError::NoInstruments);
        }
    }

    /// Invalidates the cache with a new builder.
    ///
    /// This is the preferred method for re-calibration after quote changes.
    pub fn invalidate_with_builder(&self, builder: FxVolSurfaceBuilder<T>) {
        self.invalidate(Some(builder));
    }

    /// Gets the configuration if available.
    pub fn config(&self) -> Option<FxVolSurfaceConfig> {
        let inner = self.inner.read().expect("Lock poisoned");
        if let LazyState::Calibrated { surface, .. } = &inner.state {
            Some(surface.config().clone())
        } else {
            None
        }
    }

    /// Queries volatility by strike.
    ///
    /// Triggers calibration on first call.
    pub fn vol(&self, strike: Strike<T>, expiry: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::CalibrationFailed(e.to_string()))?;

        if let LazyState::Calibrated { surface, .. } = &inner.state {
            inner.stats.hits += 1;
            surface.vol(strike, expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Queries volatility by delta.
    ///
    /// Triggers calibration on first call.
    pub fn vol_by_delta(&self, delta: T, expiry: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::CalibrationFailed(e.to_string()))?;

        if let LazyState::Calibrated { surface, .. } = &inner.state {
            inner.stats.hits += 1;
            surface.vol_by_delta(delta, expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Extracts a smile at a given expiry.
    ///
    /// Triggers calibration on first call.
    pub fn smile(&self, expiry: T) -> Result<VolSmile<T>, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::CalibrationFailed(e.to_string()))?;

        if let LazyState::Calibrated { surface, .. } = &inner.state {
            inner.stats.hits += 1;
            surface.smile(expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Returns the ATM volatility at a given expiry.
    ///
    /// Triggers calibration on first call.
    pub fn atm_vol(&self, expiry: T) -> Result<T, VolSurfaceError> {
        let mut inner = self.inner.write().expect("Lock poisoned");
        self.ensure_calibrated(&mut inner)
            .map_err(|e| VolSurfaceError::CalibrationFailed(e.to_string()))?;

        if let LazyState::Calibrated { surface, .. } = &inner.state {
            inner.stats.hits += 1;
            surface.atm_vol(expiry)
        } else {
            unreachable!("ensure_calibrated succeeded but state is not Calibrated")
        }
    }

    /// Ensures calibration has been performed.
    fn ensure_calibrated(
        &self,
        inner: &mut LazyInner<T>,
    ) -> Result<(), CalibrationError> {
        match &inner.state {
            LazyState::Calibrated { .. } => Ok(()),
            LazyState::Failed(e) => Err(e.clone()),
            LazyState::Pending(_) => {
                // Need to take ownership of builder
                let state = std::mem::replace(
                    &mut inner.state,
                    LazyState::Failed(CalibrationError::NoInstruments),
                );

                if let LazyState::Pending(builder) = state {
                    inner.stats.misses += 1;
                    match builder.build() {
                        Ok((surface, diagnostics)) => {
                            inner.state = LazyState::Calibrated { surface, diagnostics };
                            Ok(())
                        }
                        Err(e) => {
                            inner.state = LazyState::Failed(e.clone());
                            Err(e)
                        }
                    }
                } else {
                    unreachable!("State was Pending")
                }
            }
        }
    }
}

impl<T: Float + Send + Sync + 'static> Clone for LazyFxVolSurface<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Note: Cannot implement VolatilitySurface directly because the trait
// uses &self and we need &mut self for lazy calibration.
// Users should use the vol() method directly.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::fx_calibration::curve::SimpleFxCurve;
    use crate::market::fx_calibration::vol_builder::VolQuoteType;
    use crate::market::fx_calibration::FxCurve;
    use chrono::NaiveDate;
    use infra_master::data::instruments::fx::CurrencyPair;
    use std::sync::Arc;

    fn make_test_builder() -> FxVolSurfaceBuilder<f64> {
        let curve = make_test_fx_curve();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let expiry_1m = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();

        FxVolSurfaceBuilder::new(CurrencyPair::eurusd())
            .with_reference_date(ref_date)
            .with_fx_curve(Arc::new(curve))
            .with_quote(expiry_1m, VolQuoteType::Atm, 0.08)
    }

    fn make_test_fx_curve() -> SimpleFxCurve<f64> {
        use crate::market::curves::interpolator::InterpolationMethod;
        use crate::market::curves::traits::YieldCurve;
        use crate::market::curves::BootstrappedCurve;
        use chrono::NaiveDate;

        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let domestic_curve: Arc<dyn YieldCurve<f64> + Send + Sync> = Arc::new(
            BootstrappedCurve::new(
                ref_date,
                vec![0.0, 0.25, 0.5, 1.0],
                vec![0.045, 0.045, 0.046, 0.047],
                InterpolationMethod::Linear,
            )
            .unwrap(),
        );

        let foreign_curve: Arc<dyn YieldCurve<f64> + Send + Sync> = Arc::new(
            BootstrappedCurve::new(
                ref_date,
                vec![0.0, 0.25, 0.5, 1.0],
                vec![0.035, 0.035, 0.036, 0.037],
                InterpolationMethod::Linear,
            )
            .unwrap(),
        );

        SimpleFxCurve::new(
            CurrencyPair::eurusd(),
            1.0850,
            domestic_curve,
            foreign_curve,
        )
    }

    #[test]
    fn test_lazy_surface_creation() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        assert!(!lazy_surface.is_calibrated());
        assert!(!lazy_surface.has_failed());
    }

    #[test]
    fn test_lazy_calibration_on_first_vol() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        // Not yet calibrated
        assert!(!lazy_surface.is_calibrated());

        // Query triggers calibration
        let strike = Strike::new(1.0850).unwrap();
        let vol = lazy_surface.vol(strike, 0.0833);
        assert!(vol.is_ok());

        // Now calibrated
        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_cache_stats() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        let stats_before = lazy_surface.cache_stats();
        assert_eq!(stats_before.hits, 0);
        assert_eq!(stats_before.misses, 0);

        // First query triggers calibration (miss)
        let strike = Strike::new(1.0850).unwrap();
        let _ = lazy_surface.vol(strike, 0.0833);

        let stats_after_first = lazy_surface.cache_stats();
        assert_eq!(stats_after_first.misses, 1);
        assert_eq!(stats_after_first.hits, 1); // The vol call after calibration

        // Second query uses cache (hit)
        let _ = lazy_surface.vol(strike, 0.0833);

        let stats_after_second = lazy_surface.cache_stats();
        assert_eq!(stats_after_second.hits, 2);
    }

    #[test]
    fn test_force_calibrate() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        assert!(!lazy_surface.is_calibrated());

        let result = lazy_surface.force_calibrate();
        assert!(result.is_ok());
        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_invalidate() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        // Calibrate first
        let _ = lazy_surface.force_calibrate();
        assert!(lazy_surface.is_calibrated());

        // Invalidate without new builder
        lazy_surface.invalidate(None);
        assert!(!lazy_surface.is_calibrated());
        assert!(lazy_surface.has_failed()); // No builder means failed state

        let stats = lazy_surface.cache_stats();
        assert_eq!(stats.invalidations, 1);
    }

    #[test]
    fn test_invalidate_with_builder() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        // Calibrate first
        let _ = lazy_surface.force_calibrate();
        assert!(lazy_surface.is_calibrated());

        // Invalidate with new builder
        let new_builder = make_test_builder();
        lazy_surface.invalidate_with_builder(new_builder);

        assert!(!lazy_surface.is_calibrated());
        assert!(!lazy_surface.has_failed());

        // Re-calibrate
        let result = lazy_surface.force_calibrate();
        assert!(result.is_ok());
        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_diagnostics() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        // No diagnostics before calibration
        assert!(lazy_surface.diagnostics().is_none());

        // Calibrate
        let _ = lazy_surface.force_calibrate();

        // Diagnostics available after calibration
        let diag = lazy_surface.diagnostics();
        assert!(diag.is_some());
    }

    #[test]
    fn test_clone_shares_state() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        let cloned = lazy_surface.clone();

        // Calibrate original
        let _ = lazy_surface.force_calibrate();

        // Clone should also be calibrated (shared state)
        assert!(cloned.is_calibrated());
    }

    #[test]
    fn test_hit_rate() {
        let stats = CacheStats {
            hits: 8,
            misses: 2,
            invalidations: 0,
        };
        assert!((stats.hit_rate() - 0.8).abs() < 0.001);

        let empty_stats = CacheStats::default();
        assert_eq!(empty_stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_vol_by_delta() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        let vol = lazy_surface.vol_by_delta(0.25, 0.0833);
        assert!(vol.is_ok());
        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_atm_vol() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        let vol = lazy_surface.atm_vol(0.0833);
        assert!(vol.is_ok());
        assert!(lazy_surface.is_calibrated());
    }

    #[test]
    fn test_smile_extraction() {
        let builder = make_test_builder();
        let lazy_surface = LazyFxVolSurface::new(builder);

        let smile = lazy_surface.smile(0.0833);
        assert!(smile.is_ok());
        assert!(lazy_surface.is_calibrated());
    }
}

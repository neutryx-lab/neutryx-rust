//! Multi-curve framework for yield curve bootstrapping.
//!
//! This module provides `MultiCurveBuilder<T>` for constructing OIS discount
//! curves and tenor-specific forward curves (3M, 6M, etc.) in a single operation.
//!
//! ## Architecture
//!
//! In modern interest rate markets, different tenors (overnight, 3M, 6M) trade
//! at different spreads. This module supports:
//!
//! - Single-curve mode (self-discounting)
//! - Multi-curve mode (OIS discount + tenor forward curves)
//!
//! ## Example
//!
//! ```rust,ignore
//! use pricer_optimiser::bootstrapping::{
//!     MultiCurveBuilder, BootstrapInstrument, Tenor, GenericBootstrapConfig,
//! };
//!
//! // Build OIS discount curve and 3M forward curve
//! let builder = MultiCurveBuilder::<f64>::new(GenericBootstrapConfig::default());
//!
//! let ois_instruments = vec![
//!     BootstrapInstrument::ois(1.0, 0.03),
//!     BootstrapInstrument::ois(2.0, 0.032),
//! ];
//!
//! let forward_instruments = vec![
//!     (Tenor::ThreeMonth, vec![
//!         BootstrapInstrument::irs(1.0, 0.035),
//!         BootstrapInstrument::irs(2.0, 0.037),
//!     ]),
//! ];
//!
//! let curve_set = builder.build(&ois_instruments, &forward_instruments).unwrap();
//! ```

use super::config::GenericBootstrapConfig;
use super::curve::BootstrappedCurve;
use super::engine::SequentialBootstrapper;
use super::error::BootstrapError;
use super::instrument::BootstrapInstrument;
use num_traits::Float;
use std::collections::HashMap;
#[cfg(feature = "parallel")]
use std::sync::Arc;

/// Tenor definitions for forward curves.
///
/// Represents the standard interest rate tenors used in the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tenor {
    /// Overnight rate (e.g., SOFR, ESTR)
    Overnight,
    /// 1-month tenor
    OneMonth,
    /// 3-month tenor (e.g., 3M LIBOR equivalent)
    #[default]
    ThreeMonth,
    /// 6-month tenor (e.g., 6M EURIBOR)
    SixMonth,
    /// 12-month tenor
    TwelveMonth,
}

impl Tenor {
    /// Get the period length in years.
    pub fn period_years<T: Float>(&self) -> T {
        match self {
            Tenor::Overnight => T::from(1.0 / 365.0).unwrap(),
            Tenor::OneMonth => T::from(1.0 / 12.0).unwrap(),
            Tenor::ThreeMonth => T::from(0.25).unwrap(),
            Tenor::SixMonth => T::from(0.5).unwrap(),
            Tenor::TwelveMonth => T::from(1.0).unwrap(),
        }
    }

    /// Get the tenor name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Tenor::Overnight => "ON",
            Tenor::OneMonth => "1M",
            Tenor::ThreeMonth => "3M",
            Tenor::SixMonth => "6M",
            Tenor::TwelveMonth => "12M",
        }
    }

    /// Get the number of periods per year.
    pub fn periods_per_year(&self) -> f64 {
        match self {
            Tenor::Overnight => 365.0,
            Tenor::OneMonth => 12.0,
            Tenor::ThreeMonth => 4.0,
            Tenor::SixMonth => 2.0,
            Tenor::TwelveMonth => 1.0,
        }
    }
}

impl std::fmt::Display for Tenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A set of curves for multi-curve discounting.
///
/// Contains an OIS discount curve and optional tenor-specific forward curves.
/// This structure is immutable once created.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
#[derive(Debug, Clone)]
pub struct CurveSet<T: Float> {
    /// OIS discount curve for discounting cash flows
    discount_curve: BootstrappedCurve<T>,
    /// Tenor-specific forward curves for projection
    forward_curves: HashMap<Tenor, BootstrappedCurve<T>>,
}

impl<T: Float> CurveSet<T> {
    /// Create a new curve set.
    ///
    /// # Arguments
    ///
    /// * `discount_curve` - The OIS discount curve
    /// * `forward_curves` - Tenor-specific forward curves
    pub fn new(
        discount_curve: BootstrappedCurve<T>,
        forward_curves: HashMap<Tenor, BootstrappedCurve<T>>,
    ) -> Self {
        Self {
            discount_curve,
            forward_curves,
        }
    }

    /// Create a single-curve set (self-discounting).
    ///
    /// The discount curve is also used for forward projection.
    pub fn single_curve(curve: BootstrappedCurve<T>) -> Self {
        Self {
            discount_curve: curve,
            forward_curves: HashMap::new(),
        }
    }

    /// Get the discount curve.
    pub fn discount_curve(&self) -> &BootstrappedCurve<T> {
        &self.discount_curve
    }

    /// Get a forward curve for a specific tenor.
    ///
    /// If no tenor-specific curve exists, returns the discount curve.
    pub fn forward_curve(&self, tenor: Tenor) -> &BootstrappedCurve<T> {
        self.forward_curves
            .get(&tenor)
            .unwrap_or(&self.discount_curve)
    }

    /// Check if a tenor-specific forward curve exists.
    pub fn has_forward_curve(&self, tenor: Tenor) -> bool {
        self.forward_curves.contains_key(&tenor)
    }

    /// Get all available tenors.
    pub fn tenors(&self) -> Vec<Tenor> {
        self.forward_curves.keys().copied().collect()
    }

    /// Get the number of forward curves.
    pub fn forward_curve_count(&self) -> usize {
        self.forward_curves.len()
    }

    /// Check if this is a single-curve setup.
    pub fn is_single_curve(&self) -> bool {
        self.forward_curves.is_empty()
    }
}

/// Builder for multi-curve construction.
///
/// Orchestrates the construction of OIS discount curves and tenor-specific
/// forward curves using sequential bootstrapping.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Example
///
/// ```rust,ignore
/// use pricer_optimiser::bootstrapping::{MultiCurveBuilder, GenericBootstrapConfig};
///
/// let builder = MultiCurveBuilder::<f64>::new(GenericBootstrapConfig::default());
/// ```
#[derive(Debug, Clone)]
pub struct MultiCurveBuilder<T: Float> {
    /// Internal bootstrapper
    bootstrapper: SequentialBootstrapper<T>,
    /// Configuration
    config: GenericBootstrapConfig<T>,
}

impl<T: Float> MultiCurveBuilder<T> {
    /// Create a new multi-curve builder.
    pub fn new(config: GenericBootstrapConfig<T>) -> Self {
        Self {
            bootstrapper: SequentialBootstrapper::new(config.clone()),
            config,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GenericBootstrapConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &GenericBootstrapConfig<T> {
        &self.config
    }

    /// Build a multi-curve set.
    ///
    /// # Arguments
    ///
    /// * `discount_instruments` - Instruments for OIS discount curve (typically OIS swaps)
    /// * `forward_instruments` - Tenor-specific instruments for forward curves
    ///
    /// # Returns
    ///
    /// * `Ok(curve_set)` - Successfully built curve set
    /// * `Err(e)` - If bootstrapping fails
    ///
    /// # Algorithm
    ///
    /// 1. First, bootstrap the OIS discount curve from `discount_instruments`
    /// 2. For each tenor in `forward_instruments`:
    ///    - Bootstrap the forward curve using the OIS discount curve for discounting
    pub fn build(
        &self,
        discount_instruments: &[BootstrapInstrument<T>],
        forward_instruments: &[(Tenor, Vec<BootstrapInstrument<T>>)],
    ) -> Result<CurveSet<T>, BootstrapError> {
        // Step 1: Bootstrap discount curve
        let discount_result = self.bootstrapper.bootstrap(discount_instruments)?;
        let discount_curve = discount_result.curve;

        // Step 2: Bootstrap forward curves
        let mut forward_curves = HashMap::new();

        for (tenor, instruments) in forward_instruments {
            if instruments.is_empty() {
                continue;
            }

            // Bootstrap forward curve with discount curve for valuation
            // Note: For now, we use the same bootstrapper. In a more advanced
            // implementation, we would use the discount curve for NPV calculations.
            let forward_result = self.bootstrapper.bootstrap(instruments)?;
            forward_curves.insert(*tenor, forward_result.curve);
        }

        Ok(CurveSet::new(discount_curve, forward_curves))
    }

    /// Build a single-curve (self-discounting).
    ///
    /// Convenience method when only one curve is needed.
    pub fn build_single_curve(
        &self,
        instruments: &[BootstrapInstrument<T>],
    ) -> Result<CurveSet<T>, BootstrapError> {
        let result = self.bootstrapper.bootstrap(instruments)?;
        Ok(CurveSet::single_curve(result.curve))
    }

    /// Build OIS discount curve only.
    ///
    /// Returns just the bootstrapped curve without the CurveSet wrapper.
    pub fn build_discount_curve(
        &self,
        instruments: &[BootstrapInstrument<T>],
    ) -> Result<BootstrappedCurve<T>, BootstrapError> {
        let result = self.bootstrapper.bootstrap(instruments)?;
        Ok(result.curve)
    }

    /// Build a multi-curve set with parallel forward curve construction.
    ///
    /// This method builds the discount curve first, then constructs all
    /// forward curves in parallel using Rayon's work-stealing scheduler.
    ///
    /// # Arguments
    ///
    /// * `discount_instruments` - Instruments for OIS discount curve
    /// * `forward_instruments` - Tenor-specific instruments for forward curves
    ///
    /// # Returns
    ///
    /// * `Ok(curve_set)` - Successfully built curve set
    /// * `Err(e)` - If any bootstrapping fails
    ///
    /// # Performance
    ///
    /// When the `parallel` feature is enabled, forward curves are built
    /// in parallel. Otherwise, falls back to sequential construction.
    #[cfg(feature = "parallel")]
    pub fn build_parallel(
        &self,
        discount_instruments: &[BootstrapInstrument<T>],
        forward_instruments: &[(Tenor, Vec<BootstrapInstrument<T>>)],
    ) -> Result<CurveSet<T>, BootstrapError>
    where
        T: Send + Sync,
    {
        use rayon::prelude::*;

        // Step 1: Bootstrap discount curve (must be done first)
        let discount_result = self.bootstrapper.bootstrap(discount_instruments)?;
        let discount_curve = discount_result.curve;

        // Step 2: Bootstrap forward curves in parallel
        let config = self.config.clone();
        let forward_results: Result<Vec<_>, BootstrapError> = forward_instruments
            .par_iter()
            .filter(|(_, instruments)| !instruments.is_empty())
            .map(|(tenor, instruments)| {
                let bootstrapper = SequentialBootstrapper::new(config.clone());
                let result = bootstrapper.bootstrap(instruments)?;
                Ok((*tenor, result.curve))
            })
            .collect();

        let forward_curves: HashMap<Tenor, BootstrappedCurve<T>> =
            forward_results?.into_iter().collect();

        Ok(CurveSet::new(discount_curve, forward_curves))
    }

    /// Fallback to sequential when parallel feature is disabled.
    #[cfg(not(feature = "parallel"))]
    pub fn build_parallel(
        &self,
        discount_instruments: &[BootstrapInstrument<T>],
        forward_instruments: &[(Tenor, Vec<BootstrapInstrument<T>>)],
    ) -> Result<CurveSet<T>, BootstrapError> {
        self.build(discount_instruments, forward_instruments)
    }
}

/// Parallel curve set builder for batch processing.
///
/// Enables construction of multiple independent curve sets in parallel,
/// using Rayon's work-stealing scheduler for optimal load balancing.
///
/// # Example
///
/// ```rust,ignore
/// use pricer_optimiser::bootstrapping::{ParallelCurveSetBuilder, GenericBootstrapConfig};
///
/// let builder = ParallelCurveSetBuilder::<f64>::new(GenericBootstrapConfig::default());
/// let curve_sets = builder.build_batch(&curve_set_inputs)?;
/// ```
#[derive(Debug, Clone)]
pub struct ParallelCurveSetBuilder<T: Float> {
    /// Configuration shared across all curve builds
    config: GenericBootstrapConfig<T>,
}

impl<T: Float> ParallelCurveSetBuilder<T> {
    /// Create a new parallel curve set builder.
    pub fn new(config: GenericBootstrapConfig<T>) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GenericBootstrapConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &GenericBootstrapConfig<T> {
        &self.config
    }

    /// Build multiple curve sets in parallel.
    ///
    /// Each input set is processed independently, enabling true parallelism.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Vector of (discount_instruments, forward_instruments) pairs
    ///
    /// # Returns
    ///
    /// * `Ok(curve_sets)` - Successfully built curve sets
    /// * `Err(e)` - If any curve set fails to build
    #[cfg(feature = "parallel")]
    #[allow(clippy::type_complexity)]
    pub fn build_batch(
        &self,
        inputs: &[(
            Vec<BootstrapInstrument<T>>,
            Vec<(Tenor, Vec<BootstrapInstrument<T>>)>,
        )],
    ) -> Result<Vec<CurveSet<T>>, BootstrapError>
    where
        T: Send + Sync,
    {
        use rayon::prelude::*;

        let config = Arc::new(self.config.clone());

        inputs
            .par_iter()
            .map(|(discount_instruments, forward_instruments)| {
                let builder = MultiCurveBuilder::new((*config).clone());
                builder.build(discount_instruments, forward_instruments)
            })
            .collect()
    }

    /// Sequential fallback when parallel feature is disabled.
    #[cfg(not(feature = "parallel"))]
    pub fn build_batch(
        &self,
        inputs: &[(
            Vec<BootstrapInstrument<T>>,
            Vec<(Tenor, Vec<BootstrapInstrument<T>>)>,
        )],
    ) -> Result<Vec<CurveSet<T>>, BootstrapError> {
        inputs
            .iter()
            .map(|(discount_instruments, forward_instruments)| {
                let builder = MultiCurveBuilder::new(self.config.clone());
                builder.build(discount_instruments, forward_instruments)
            })
            .collect()
    }

    /// Build multiple single-curve sets in parallel.
    ///
    /// Convenience method for building multiple self-discounting curves.
    #[cfg(feature = "parallel")]
    pub fn build_single_curves_batch(
        &self,
        inputs: &[Vec<BootstrapInstrument<T>>],
    ) -> Result<Vec<CurveSet<T>>, BootstrapError>
    where
        T: Send + Sync,
    {
        use rayon::prelude::*;

        let config = Arc::new(self.config.clone());

        inputs
            .par_iter()
            .map(|instruments| {
                let builder = MultiCurveBuilder::new((*config).clone());
                builder.build_single_curve(instruments)
            })
            .collect()
    }

    /// Sequential fallback for single curves.
    #[cfg(not(feature = "parallel"))]
    pub fn build_single_curves_batch(
        &self,
        inputs: &[Vec<BootstrapInstrument<T>>],
    ) -> Result<Vec<CurveSet<T>>, BootstrapError> {
        inputs
            .iter()
            .map(|instruments| {
                let builder = MultiCurveBuilder::new(self.config.clone());
                builder.build_single_curve(instruments)
            })
            .collect()
    }

    /// Build discount curves only in parallel.
    ///
    /// Returns raw `BootstrappedCurve` instances without the CurveSet wrapper.
    #[cfg(feature = "parallel")]
    pub fn build_discount_curves_batch(
        &self,
        inputs: &[Vec<BootstrapInstrument<T>>],
    ) -> Result<Vec<BootstrappedCurve<T>>, BootstrapError>
    where
        T: Send + Sync,
    {
        use rayon::prelude::*;

        let config = Arc::new(self.config.clone());

        inputs
            .par_iter()
            .map(|instruments| {
                let bootstrapper = SequentialBootstrapper::new((*config).clone());
                let result = bootstrapper.bootstrap(instruments)?;
                Ok(result.curve)
            })
            .collect()
    }

    /// Sequential fallback for discount curves.
    #[cfg(not(feature = "parallel"))]
    pub fn build_discount_curves_batch(
        &self,
        inputs: &[Vec<BootstrapInstrument<T>>],
    ) -> Result<Vec<BootstrappedCurve<T>>, BootstrapError> {
        inputs
            .iter()
            .map(|instruments| {
                let bootstrapper = SequentialBootstrapper::new(self.config.clone());
                let result = bootstrapper.bootstrap(instruments)?;
                Ok(result.curve)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::market_data::curves::YieldCurve;

    // ========================================
    // Tenor Tests
    // ========================================

    #[test]
    fn test_tenor_period_years() {
        assert!((Tenor::Overnight.period_years::<f64>() - 1.0 / 365.0).abs() < 1e-10);
        assert!((Tenor::OneMonth.period_years::<f64>() - 1.0 / 12.0).abs() < 1e-10);
        assert!((Tenor::ThreeMonth.period_years::<f64>() - 0.25).abs() < 1e-10);
        assert!((Tenor::SixMonth.period_years::<f64>() - 0.5).abs() < 1e-10);
        assert!((Tenor::TwelveMonth.period_years::<f64>() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_name() {
        assert_eq!(Tenor::Overnight.name(), "ON");
        assert_eq!(Tenor::OneMonth.name(), "1M");
        assert_eq!(Tenor::ThreeMonth.name(), "3M");
        assert_eq!(Tenor::SixMonth.name(), "6M");
        assert_eq!(Tenor::TwelveMonth.name(), "12M");
    }

    #[test]
    fn test_tenor_periods_per_year() {
        assert!((Tenor::Overnight.periods_per_year() - 365.0).abs() < 1e-10);
        assert!((Tenor::ThreeMonth.periods_per_year() - 4.0).abs() < 1e-10);
        assert!((Tenor::SixMonth.periods_per_year() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_default() {
        let tenor: Tenor = Default::default();
        assert_eq!(tenor, Tenor::ThreeMonth);
    }

    #[test]
    fn test_tenor_display() {
        assert_eq!(format!("{}", Tenor::ThreeMonth), "3M");
        assert_eq!(format!("{}", Tenor::SixMonth), "6M");
    }

    #[test]
    fn test_tenor_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Tenor::ThreeMonth);
        set.insert(Tenor::SixMonth);
        assert!(set.contains(&Tenor::ThreeMonth));
        assert!(set.contains(&Tenor::SixMonth));
        assert!(!set.contains(&Tenor::OneMonth));
    }

    #[test]
    fn test_tenor_clone_copy() {
        let t1 = Tenor::ThreeMonth;
        let t2 = t1; // Copy
        let t3 = t1.clone();
        assert_eq!(t1, t2);
        assert_eq!(t1, t3);
    }

    // ========================================
    // CurveSet Tests
    // ========================================

    fn create_test_curve() -> BootstrappedCurve<f64> {
        use super::super::config::BootstrapInterpolation;
        BootstrappedCurve::new(
            vec![1.0, 2.0, 3.0],
            vec![0.97, 0.94, 0.91],
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap()
    }

    #[test]
    fn test_curve_set_single_curve() {
        let curve = create_test_curve();
        let curve_set = CurveSet::single_curve(curve);

        assert!(curve_set.is_single_curve());
        assert_eq!(curve_set.forward_curve_count(), 0);
        assert!(curve_set.tenors().is_empty());
    }

    #[test]
    fn test_curve_set_with_forward_curves() {
        let discount_curve = create_test_curve();
        let forward_curve = create_test_curve();

        let mut forward_curves = HashMap::new();
        forward_curves.insert(Tenor::ThreeMonth, forward_curve);

        let curve_set = CurveSet::new(discount_curve, forward_curves);

        assert!(!curve_set.is_single_curve());
        assert_eq!(curve_set.forward_curve_count(), 1);
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
        assert!(!curve_set.has_forward_curve(Tenor::SixMonth));
    }

    #[test]
    fn test_curve_set_get_discount_curve() {
        let curve = create_test_curve();
        let curve_set = CurveSet::single_curve(curve);

        let df = curve_set.discount_curve().discount_factor(1.0).unwrap();
        assert!((df - 0.97).abs() < 1e-10);
    }

    #[test]
    fn test_curve_set_get_forward_curve_fallback() {
        let curve = create_test_curve();
        let curve_set = CurveSet::single_curve(curve);

        // Should return discount curve when no forward curve exists
        let forward = curve_set.forward_curve(Tenor::ThreeMonth);
        let df = forward.discount_factor(1.0).unwrap();
        assert!((df - 0.97).abs() < 1e-10);
    }

    #[test]
    fn test_curve_set_tenors() {
        let discount_curve = create_test_curve();
        let mut forward_curves = HashMap::new();
        forward_curves.insert(Tenor::ThreeMonth, create_test_curve());
        forward_curves.insert(Tenor::SixMonth, create_test_curve());

        let curve_set = CurveSet::new(discount_curve, forward_curves);

        let tenors = curve_set.tenors();
        assert_eq!(tenors.len(), 2);
        assert!(tenors.contains(&Tenor::ThreeMonth));
        assert!(tenors.contains(&Tenor::SixMonth));
    }

    #[test]
    fn test_curve_set_clone() {
        let curve = create_test_curve();
        let curve_set1 = CurveSet::single_curve(curve);
        let curve_set2 = curve_set1.clone();

        assert_eq!(curve_set1.is_single_curve(), curve_set2.is_single_curve());
    }

    // ========================================
    // MultiCurveBuilder Tests
    // ========================================

    #[test]
    fn test_builder_with_defaults() {
        let builder = MultiCurveBuilder::<f64>::with_defaults();
        assert!(builder.config().tolerance < 1e-10);
    }

    #[test]
    fn test_builder_clone() {
        let builder1 = MultiCurveBuilder::<f64>::with_defaults();
        let builder2 = builder1.clone();
        assert_eq!(
            builder1.config().max_iterations,
            builder2.config().max_iterations
        );
    }

    #[test]
    fn test_build_single_curve() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_single_curve(&instruments).unwrap();

        assert!(curve_set.is_single_curve());
        assert_eq!(curve_set.discount_curve().pillar_count(), 3);
    }

    #[test]
    fn test_build_discount_curve() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve = builder.build_discount_curve(&instruments).unwrap();

        assert_eq!(curve.pillar_count(), 2);
    }

    #[test]
    fn test_build_multi_curve() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![(
            Tenor::ThreeMonth,
            vec![
                BootstrapInstrument::irs(1.0, 0.035),
                BootstrapInstrument::irs(2.0, 0.037),
            ],
        )];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build(&discount_instruments, &forward_instruments)
            .unwrap();

        assert!(!curve_set.is_single_curve());
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
        assert_eq!(curve_set.forward_curve_count(), 1);
    }

    #[test]
    fn test_build_multi_curve_multiple_tenors() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![
            (
                Tenor::ThreeMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            ),
            (
                Tenor::SixMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.036),
                    BootstrapInstrument::irs(2.0, 0.038),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build(&discount_instruments, &forward_instruments)
            .unwrap();

        assert_eq!(curve_set.forward_curve_count(), 2);
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
        assert!(curve_set.has_forward_curve(Tenor::SixMonth));
    }

    #[test]
    fn test_build_multi_curve_empty_forward() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> =
            vec![(Tenor::ThreeMonth, vec![])]; // Empty

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build(&discount_instruments, &forward_instruments)
            .unwrap();

        // Empty forward instruments should be skipped
        assert!(curve_set.is_single_curve());
    }

    #[test]
    fn test_build_error_empty_discount() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let result = builder.build(&discount_instruments, &[]);

        assert!(result.is_err());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_multi_curve_discount_factor_consistency() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![(
            Tenor::ThreeMonth,
            vec![
                BootstrapInstrument::irs(1.0, 0.035),
                BootstrapInstrument::irs(2.0, 0.037),
            ],
        )];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build(&discount_instruments, &forward_instruments)
            .unwrap();

        // Discount curve should give valid discount factors
        let df_discount = curve_set.discount_curve().discount_factor(1.0).unwrap();
        assert!(df_discount > 0.0 && df_discount < 1.0);

        // Forward curve should also give valid discount factors
        let df_forward = curve_set
            .forward_curve(Tenor::ThreeMonth)
            .discount_factor(1.0)
            .unwrap();
        assert!(df_forward > 0.0 && df_forward < 1.0);

        // Forward curve rates are typically higher than OIS
        // so forward DF should be lower (more discounting)
        // This is a typical market relationship
        assert!(df_forward <= df_discount + 0.01); // Allow some tolerance
    }

    // ========================================
    // Parallel Bootstrap Tests
    // ========================================

    #[test]
    fn test_build_parallel_single_tenor() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![(
            Tenor::ThreeMonth,
            vec![
                BootstrapInstrument::irs(1.0, 0.035),
                BootstrapInstrument::irs(2.0, 0.037),
            ],
        )];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build_parallel(&discount_instruments, &forward_instruments)
            .unwrap();

        assert!(!curve_set.is_single_curve());
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
    }

    #[test]
    fn test_build_parallel_multiple_tenors() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![
            (
                Tenor::ThreeMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            ),
            (
                Tenor::SixMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.036),
                    BootstrapInstrument::irs(2.0, 0.038),
                ],
            ),
            (
                Tenor::TwelveMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.037),
                    BootstrapInstrument::irs(2.0, 0.039),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build_parallel(&discount_instruments, &forward_instruments)
            .unwrap();

        assert_eq!(curve_set.forward_curve_count(), 3);
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
        assert!(curve_set.has_forward_curve(Tenor::SixMonth));
        assert!(curve_set.has_forward_curve(Tenor::TwelveMonth));
    }

    #[test]
    fn test_build_parallel_equals_sequential() {
        let discount_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![(
            Tenor::ThreeMonth,
            vec![
                BootstrapInstrument::irs(1.0, 0.035),
                BootstrapInstrument::irs(2.0, 0.037),
            ],
        )];

        let builder = MultiCurveBuilder::<f64>::with_defaults();

        let sequential_set = builder
            .build(&discount_instruments, &forward_instruments)
            .unwrap();
        let parallel_set = builder
            .build_parallel(&discount_instruments, &forward_instruments)
            .unwrap();

        // Results should be identical
        let df_seq = sequential_set
            .discount_curve()
            .discount_factor(1.5)
            .unwrap();
        let df_par = parallel_set.discount_curve().discount_factor(1.5).unwrap();
        assert!((df_seq - df_par).abs() < 1e-12);

        let df_fwd_seq = sequential_set
            .forward_curve(Tenor::ThreeMonth)
            .discount_factor(1.5)
            .unwrap();
        let df_fwd_par = parallel_set
            .forward_curve(Tenor::ThreeMonth)
            .discount_factor(1.5)
            .unwrap();
        assert!((df_fwd_seq - df_fwd_par).abs() < 1e-12);
    }

    // ========================================
    // ParallelCurveSetBuilder Tests
    // ========================================

    #[test]
    fn test_parallel_builder_with_defaults() {
        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        assert!(builder.config().tolerance < 1e-10);
    }

    #[test]
    fn test_parallel_builder_clone() {
        let builder1 = ParallelCurveSetBuilder::<f64>::with_defaults();
        let builder2 = builder1.clone();
        assert_eq!(
            builder1.config().max_iterations,
            builder2.config().max_iterations
        );
    }

    #[test]
    fn test_build_batch_single() {
        let input = vec![(
            vec![
                BootstrapInstrument::ois(1.0, 0.03),
                BootstrapInstrument::ois(2.0, 0.032),
            ],
            vec![(
                Tenor::ThreeMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            )],
        )];

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let curve_sets = builder.build_batch(&input).unwrap();

        assert_eq!(curve_sets.len(), 1);
        assert!(curve_sets[0].has_forward_curve(Tenor::ThreeMonth));
    }

    #[test]
    fn test_build_batch_multiple() {
        let inputs = vec![
            (
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
                vec![(
                    Tenor::ThreeMonth,
                    vec![BootstrapInstrument::irs(1.0, 0.035)],
                )],
            ),
            (
                vec![
                    BootstrapInstrument::ois(1.0, 0.025),
                    BootstrapInstrument::ois(2.0, 0.028),
                ],
                vec![(Tenor::SixMonth, vec![BootstrapInstrument::irs(1.0, 0.032)])],
            ),
            (
                vec![
                    BootstrapInstrument::ois(1.0, 0.02),
                    BootstrapInstrument::ois(2.0, 0.024),
                ],
                vec![],
            ),
        ];

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let curve_sets = builder.build_batch(&inputs).unwrap();

        assert_eq!(curve_sets.len(), 3);
        assert!(curve_sets[0].has_forward_curve(Tenor::ThreeMonth));
        assert!(curve_sets[1].has_forward_curve(Tenor::SixMonth));
        assert!(curve_sets[2].is_single_curve()); // No forward curves
    }

    #[test]
    fn test_build_single_curves_batch() {
        let inputs = vec![
            vec![
                BootstrapInstrument::ois(1.0, 0.03),
                BootstrapInstrument::ois(2.0, 0.032),
            ],
            vec![
                BootstrapInstrument::ois(1.0, 0.025),
                BootstrapInstrument::ois(2.0, 0.028),
            ],
        ];

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let curve_sets = builder.build_single_curves_batch(&inputs).unwrap();

        assert_eq!(curve_sets.len(), 2);
        assert!(curve_sets[0].is_single_curve());
        assert!(curve_sets[1].is_single_curve());
    }

    #[test]
    fn test_build_discount_curves_batch() {
        let inputs = vec![
            vec![
                BootstrapInstrument::ois(1.0, 0.03),
                BootstrapInstrument::ois(2.0, 0.032),
            ],
            vec![
                BootstrapInstrument::ois(1.0, 0.025),
                BootstrapInstrument::ois(2.0, 0.028),
            ],
        ];

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let curves = builder.build_discount_curves_batch(&inputs).unwrap();

        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0].pillar_count(), 2);
        assert_eq!(curves[1].pillar_count(), 2);
    }

    #[test]
    fn test_batch_error_propagation() {
        let inputs = vec![
            (
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
                vec![],
            ),
            (
                vec![], // Empty - will cause error
                vec![],
            ),
        ];

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let result = builder.build_batch(&inputs);

        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_thread_safety() {
        // Construct many curve sets in parallel to test thread safety
        let base_rate = 0.03;
        let inputs: Vec<_> = (0..10)
            .map(|i| {
                let rate = base_rate + (i as f64) * 0.001;
                (
                    vec![
                        BootstrapInstrument::ois(1.0, rate),
                        BootstrapInstrument::ois(2.0, rate + 0.002),
                        BootstrapInstrument::ois(3.0, rate + 0.004),
                    ],
                    vec![(
                        Tenor::ThreeMonth,
                        vec![
                            BootstrapInstrument::irs(1.0, rate + 0.005),
                            BootstrapInstrument::irs(2.0, rate + 0.007),
                        ],
                    )],
                )
            })
            .collect();

        let builder = ParallelCurveSetBuilder::<f64>::with_defaults();
        let curve_sets = builder.build_batch(&inputs).unwrap();

        assert_eq!(curve_sets.len(), 10);

        // Verify each curve set is valid
        for curve_set in &curve_sets {
            assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
            let df = curve_set.discount_curve().discount_factor(1.0).unwrap();
            assert!(df > 0.0 && df < 1.0);
        }
    }
}

//! Multi-curve framework for yield curve bootstrapping.
//!
//! This module provides `MultiCurveBuilder<T>` for constructing OIS discount
//! curves and tenor-specific forward curves (3M, 6M, etc.) in a single
//! operation.
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
//! use pricer_models::market::calibration::bootstrapping::{
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

use std::collections::HashMap;
#[cfg(feature = "parallel")]
use std::sync::Arc;

use infra_master::market::RateIndex;
use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::{
    config::GenericBootstrapConfig, curve::BootstrappedCurve, engine::SequentialBootstrapper,
    engine_error::CurveEngineError, error::BootstrapError, instrument::BootstrapInstrument,
};

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
            Tenor::Overnight => from_f64(1.0 / 365.0),
            Tenor::OneMonth => from_f64(1.0 / 12.0),
            Tenor::ThreeMonth => from_f64(0.25),
            Tenor::SixMonth => from_f64(0.5),
            Tenor::TwelveMonth => from_f64(1.0),
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
/// This structure is immutable once created. Supports both tenor-based and
/// index-based lookups for flexible curve retrieval.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```ignore
/// // Tenor-based lookup
/// let forward_3m = curve_set.forward_curve(Tenor::ThreeMonth);
///
/// // Index-based lookup
/// let sofr_curve = curve_set.curve_by_index(RateIndex::Sofr);
/// ```
#[derive(Debug, Clone)]
pub struct CurveSet<T: Float> {
    /// OIS discount curve for discounting cash flows
    discount_curve: BootstrappedCurve<T>,
    /// Tenor-specific forward curves for projection
    forward_curves: HashMap<Tenor, BootstrappedCurve<T>>,
    /// Index-keyed curves for direct lookup
    index_curves: HashMap<RateIndex, BootstrappedCurve<T>>,
    /// The rate index of the discount curve
    discount_index: Option<RateIndex>,
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
            index_curves: HashMap::new(),
            discount_index: None,
        }
    }

    /// Create a new curve set with index mapping.
    ///
    /// # Arguments
    ///
    /// * `discount_curve` - The OIS discount curve
    /// * `discount_index` - The rate index of the discount curve
    /// * `forward_curves` - Tenor-specific forward curves
    /// * `index_curves` - Index-keyed curves for direct lookup
    pub fn with_indices(
        discount_curve: BootstrappedCurve<T>,
        discount_index: RateIndex,
        forward_curves: HashMap<Tenor, BootstrappedCurve<T>>,
        index_curves: HashMap<RateIndex, BootstrappedCurve<T>>,
    ) -> Self {
        Self {
            discount_curve,
            forward_curves,
            index_curves,
            discount_index: Some(discount_index),
        }
    }

    /// Create a single-curve set (self-discounting).
    ///
    /// The discount curve is also used for forward projection.
    pub fn single_curve(curve: BootstrappedCurve<T>) -> Self {
        Self {
            discount_curve: curve,
            forward_curves: HashMap::new(),
            index_curves: HashMap::new(),
            discount_index: None,
        }
    }

    /// Create a single-curve set with index association.
    pub fn single_curve_with_index(curve: BootstrappedCurve<T>, index: RateIndex) -> Self {
        let mut index_curves = HashMap::new();
        index_curves.insert(index, curve.clone());
        Self {
            discount_curve: curve,
            forward_curves: HashMap::new(),
            index_curves,
            discount_index: Some(index),
        }
    }

    /// Get the discount curve.
    pub fn discount_curve(&self) -> &BootstrappedCurve<T> { &self.discount_curve }

    /// Get the discount curve's rate index if set.
    pub fn discount_index(&self) -> Option<RateIndex> { self.discount_index }

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

    /// Get a curve by rate index.
    ///
    /// Searches in both the index_curves map and checks the discount curve.
    /// Returns `None` if no curve is associated with the given index.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let sofr_curve = curve_set.curve_by_index(RateIndex::Sofr);
    /// if let Some(curve) = sofr_curve {
    ///     let df = curve.discount_factor(1.0)?;
    /// }
    /// ```
    pub fn curve_by_index(&self, index: RateIndex) -> Option<&BootstrappedCurve<T>> {
        // First check the index_curves map
        if let Some(curve) = self.index_curves.get(&index) {
            return Some(curve);
        }

        // Check if the discount curve matches
        if self.discount_index == Some(index) {
            return Some(&self.discount_curve);
        }

        None
    }

    /// Check if a curve exists for a given index.
    pub fn has_curve_for_index(&self, index: RateIndex) -> bool {
        self.index_curves.contains_key(&index) || self.discount_index == Some(index)
    }

    /// Get all rate indices that have curves.
    pub fn indices(&self) -> Vec<RateIndex> {
        let mut indices: Vec<_> = self.index_curves.keys().copied().collect();
        if let Some(discount_idx) = self.discount_index {
            if !indices.contains(&discount_idx) {
                indices.push(discount_idx);
            }
        }
        indices
    }

    /// Get all available tenors.
    pub fn tenors(&self) -> Vec<Tenor> { self.forward_curves.keys().copied().collect() }

    /// Get the number of forward curves.
    pub fn forward_curve_count(&self) -> usize { self.forward_curves.len() }

    /// Get the total number of curves (discount + forward + index).
    pub fn total_curve_count(&self) -> usize {
        1 + self.forward_curves.len() + self.index_curves.len()
    }

    /// Check if this is a single-curve setup.
    pub fn is_single_curve(&self) -> bool {
        self.forward_curves.is_empty() && self.index_curves.is_empty()
    }

    /// Add an index-curve association.
    pub fn add_index_curve(&mut self, index: RateIndex, curve: BootstrappedCurve<T>) {
        self.index_curves.insert(index, curve);
    }
}

/// Curve dependency specification for ordered construction.
///
/// Defines what curve(s) another curve depends on for discounting.
#[derive(Debug, Clone)]
pub struct CurveDependency {
    /// The index of the curve being built.
    pub index: RateIndex,
    /// The index of the discount curve to use (if different from self).
    pub discount_index: Option<RateIndex>,
    /// Instruments for this curve.
    pub tenor: Option<Tenor>,
}

impl CurveDependency {
    /// Create a new curve dependency.
    pub fn new(index: RateIndex) -> Self {
        Self {
            index,
            discount_index: None,
            tenor: None,
        }
    }

    /// Set the discount curve dependency.
    pub fn with_discount(mut self, discount_index: RateIndex) -> Self {
        self.discount_index = Some(discount_index);
        self
    }

    /// Set the tenor for forward curves.
    pub fn with_tenor(mut self, tenor: Tenor) -> Self {
        self.tenor = Some(tenor);
        self
    }

    /// Check if this is a self-discounting curve.
    pub fn is_self_discounting(&self) -> bool {
        self.discount_index.is_none() || self.discount_index == Some(self.index)
    }
}

/// Builder for multi-curve construction.
///
/// Orchestrates the construction of OIS discount curves and tenor-specific
/// forward curves using sequential bootstrapping. Supports:
///
/// - Dependency tracking between curves
/// - Automatic build order determination
/// - Circular dependency detection
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Example
///
/// ```rust,ignore
/// use pricer_models::market::calibration::bootstrapping::{MultiCurveBuilder, GenericBootstrapConfig};
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
    pub fn with_defaults() -> Self { Self::new(GenericBootstrapConfig::default()) }

    /// Get the configuration.
    pub fn config(&self) -> &GenericBootstrapConfig<T> { &self.config }

    /// Build a multi-curve set.
    ///
    /// # Arguments
    ///
    /// * `discount_instruments` - Instruments for OIS discount curve (typically
    ///   OIS swaps)
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
    ///    - Bootstrap the forward curve using the OIS discount curve for
    ///      discounting
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

    /// Build curves with explicit dependency specification.
    ///
    /// Determines the build order automatically based on dependencies and
    /// detects circular dependencies.
    ///
    /// # Arguments
    ///
    /// * `curve_specs` - Vector of (dependency, instruments) pairs
    ///
    /// # Returns
    ///
    /// * `Ok(curve_set)` - Successfully built curve set with index mappings
    /// * `Err(CurveEngineError::CircularDependency)` - If circular dependency detected
    /// * `Err(CurveEngineError::Bootstrap)` - If bootstrapping fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let specs = vec![
    ///     (CurveDependency::new(RateIndex::Sofr), ois_instruments),
    ///     (CurveDependency::new(RateIndex::Libor3m).with_discount(RateIndex::Sofr), irs_instruments),
    /// ];
    /// let curve_set = builder.build_with_dependencies(&specs)?;
    /// ```
    pub fn build_with_dependencies(
        &self,
        curve_specs: &[(CurveDependency, Vec<BootstrapInstrument<T>>)],
    ) -> Result<CurveSet<T>, CurveEngineError> {
        // Check for circular dependencies
        Self::check_circular_dependencies(curve_specs)?;

        // Determine build order (topological sort)
        let build_order = Self::determine_build_order(curve_specs)?;

        // Build curves in dependency order
        let mut built_curves: HashMap<RateIndex, BootstrappedCurve<T>> = HashMap::new();
        let mut forward_curves: HashMap<Tenor, BootstrappedCurve<T>> = HashMap::new();
        let mut discount_curve: Option<BootstrappedCurve<T>> = None;
        let mut discount_index: Option<RateIndex> = None;

        for idx in build_order {
            let (dep, instruments) = &curve_specs[idx];

            if instruments.is_empty() {
                continue;
            }

            // Build the curve
            let result = self
                .bootstrapper
                .bootstrap(instruments)
                .map_err(CurveEngineError::Bootstrap)?;

            let curve = result.curve;

            // Track the first self-discounting curve as the discount curve
            if dep.is_self_discounting() && discount_curve.is_none() {
                discount_curve = Some(curve.clone());
                discount_index = Some(dep.index);
            }

            // Store in appropriate collections
            built_curves.insert(dep.index, curve.clone());

            if let Some(tenor) = dep.tenor {
                forward_curves.insert(tenor, curve);
            }
        }

        // Use the first built curve as discount if none was designated
        let final_discount = discount_curve.unwrap_or_else(|| {
            built_curves
                .values()
                .next()
                .cloned()
                .expect("At least one curve must be built")
        });

        Ok(CurveSet::with_indices(
            final_discount,
            discount_index.unwrap_or(RateIndex::Sofr),
            forward_curves,
            built_curves,
        ))
    }

    /// Check for circular dependencies in curve specifications.
    ///
    /// Uses depth-first search to detect cycles in the dependency graph.
    fn check_circular_dependencies(
        curve_specs: &[(CurveDependency, Vec<BootstrapInstrument<T>>)],
    ) -> Result<(), CurveEngineError> {
        // Build adjacency list
        let mut index_to_idx: HashMap<RateIndex, usize> = HashMap::new();
        for (i, (dep, _)) in curve_specs.iter().enumerate() {
            index_to_idx.insert(dep.index, i);
        }

        // For each node, check for cycles using DFS
        let n = curve_specs.len();
        let mut visited = vec![false; n];
        let mut rec_stack = vec![false; n];

        for i in 0..n {
            if !visited[i]
                && Self::has_cycle_dfs(i, curve_specs, &index_to_idx, &mut visited, &mut rec_stack)
            {
                return Err(CurveEngineError::CircularDependency);
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection.
    fn has_cycle_dfs(
        node: usize,
        curve_specs: &[(CurveDependency, Vec<BootstrapInstrument<T>>)],
        index_to_idx: &HashMap<RateIndex, usize>,
        visited: &mut [bool],
        rec_stack: &mut [bool],
    ) -> bool {
        visited[node] = true;
        rec_stack[node] = true;

        let (dep, _) = &curve_specs[node];

        // Check dependency
        if let Some(disc_idx) = dep.discount_index {
            // Skip if self-discounting
            if disc_idx != dep.index {
                if let Some(&neighbor) = index_to_idx.get(&disc_idx) {
                    if !visited[neighbor] {
                        if Self::has_cycle_dfs(
                            neighbor,
                            curve_specs,
                            index_to_idx,
                            visited,
                            rec_stack,
                        ) {
                            return true;
                        }
                    } else if rec_stack[neighbor] {
                        return true;
                    }
                }
            }
        }

        rec_stack[node] = false;
        false
    }

    /// Determine the build order using topological sort.
    ///
    /// Self-discounting curves are built first, then curves that depend on them.
    fn determine_build_order(
        curve_specs: &[(CurveDependency, Vec<BootstrapInstrument<T>>)],
    ) -> Result<Vec<usize>, CurveEngineError> {
        let n = curve_specs.len();

        // Build index mapping
        let mut index_to_idx: HashMap<RateIndex, usize> = HashMap::new();
        for (i, (dep, _)) in curve_specs.iter().enumerate() {
            index_to_idx.insert(dep.index, i);
        }

        // Calculate in-degrees
        let mut in_degree = vec![0usize; n];
        for (i, (dep, _)) in curve_specs.iter().enumerate() {
            if let Some(disc_idx) = dep.discount_index {
                if disc_idx != dep.index {
                    // Only count if dependency exists in our specs
                    if index_to_idx.contains_key(&disc_idx) {
                        in_degree[i] += 1;
                    }
                }
            }
        }

        // Kahn's algorithm for topological sort
        let mut queue: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter_map(|(i, &deg)| if deg == 0 { Some(i) } else { None })
            .collect();

        let mut result = Vec::with_capacity(n);

        while let Some(node) = queue.pop() {
            result.push(node);

            let (dep, _) = &curve_specs[node];

            // Find nodes that depend on this one
            for (i, (other_dep, _)) in curve_specs.iter().enumerate() {
                if let Some(disc_idx) = other_dep.discount_index {
                    if disc_idx == dep.index && disc_idx != other_dep.index {
                        in_degree[i] -= 1;
                        if in_degree[i] == 0 {
                            queue.push(i);
                        }
                    }
                }
            }
        }

        if result.len() != n {
            // This shouldn't happen if cycle check passed
            return Err(CurveEngineError::CircularDependency);
        }

        Ok(result)
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
/// use pricer_models::market::calibration::bootstrapping::{ParallelCurveSetBuilder, GenericBootstrapConfig};
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
    pub fn new(config: GenericBootstrapConfig<T>) -> Self { Self { config } }

    /// Create with default configuration.
    pub fn with_defaults() -> Self { Self::new(GenericBootstrapConfig::default()) }

    /// Get the configuration.
    pub fn config(&self) -> &GenericBootstrapConfig<T> { &self.config }

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
    use crate::market::curves::YieldCurve;

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

    // ========================================
    // CurveSet Index-Based Lookup Tests
    // ========================================

    #[test]
    fn test_curve_set_single_curve_with_index() {
        let curve = create_test_curve();
        let curve_set = CurveSet::single_curve_with_index(curve, RateIndex::Sofr);

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(!curve_set.has_curve_for_index(RateIndex::Sonia));
        assert_eq!(curve_set.discount_index(), Some(RateIndex::Sofr));
    }

    #[test]
    fn test_curve_set_curve_by_index() {
        let curve = create_test_curve();
        let curve_set = CurveSet::single_curve_with_index(curve, RateIndex::Sofr);

        let found = curve_set.curve_by_index(RateIndex::Sofr);
        assert!(found.is_some());

        let df = found.unwrap().discount_factor(1.0).unwrap();
        assert!((df - 0.97).abs() < 1e-10);

        // Should return None for non-existent index
        assert!(curve_set.curve_by_index(RateIndex::Sonia).is_none());
    }

    #[test]
    fn test_curve_set_with_indices() {
        let discount_curve = create_test_curve();
        let mut index_curves = HashMap::new();
        index_curves.insert(RateIndex::Sonia, create_test_curve());

        let curve_set = CurveSet::with_indices(
            discount_curve,
            RateIndex::Sofr,
            HashMap::new(),
            index_curves,
        );

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));
        assert!(!curve_set.has_curve_for_index(RateIndex::Tonar));

        let indices = curve_set.indices();
        assert!(indices.contains(&RateIndex::Sofr));
        assert!(indices.contains(&RateIndex::Sonia));
    }

    #[test]
    fn test_curve_set_total_curve_count() {
        let discount_curve = create_test_curve();
        let mut forward_curves = HashMap::new();
        forward_curves.insert(Tenor::ThreeMonth, create_test_curve());
        forward_curves.insert(Tenor::SixMonth, create_test_curve());

        let mut index_curves = HashMap::new();
        index_curves.insert(RateIndex::Sonia, create_test_curve());

        let curve_set = CurveSet::with_indices(
            discount_curve,
            RateIndex::Sofr,
            forward_curves,
            index_curves,
        );

        // 1 discount + 2 forward + 1 index = 4 total
        assert_eq!(curve_set.total_curve_count(), 4);
    }

    #[test]
    fn test_curve_set_add_index_curve() {
        let curve = create_test_curve();
        let mut curve_set = CurveSet::single_curve(curve);

        assert!(!curve_set.has_curve_for_index(RateIndex::Sofr));

        curve_set.add_index_curve(RateIndex::Sofr, create_test_curve());

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
    }

    // ========================================
    // CurveDependency Tests
    // ========================================

    #[test]
    fn test_curve_dependency_new() {
        let dep = CurveDependency::new(RateIndex::Sofr);
        assert_eq!(dep.index, RateIndex::Sofr);
        assert!(dep.discount_index.is_none());
        assert!(dep.tenor.is_none());
        assert!(dep.is_self_discounting());
    }

    #[test]
    fn test_curve_dependency_with_discount() {
        let dep = CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr);
        assert_eq!(dep.index, RateIndex::Sonia);
        assert_eq!(dep.discount_index, Some(RateIndex::Sofr));
        assert!(!dep.is_self_discounting());
    }

    #[test]
    fn test_curve_dependency_with_tenor() {
        let dep = CurveDependency::new(RateIndex::Sofr).with_tenor(Tenor::ThreeMonth);
        assert_eq!(dep.tenor, Some(Tenor::ThreeMonth));
    }

    #[test]
    fn test_curve_dependency_self_discount_explicit() {
        // Self-discount when discount_index == index
        let dep = CurveDependency::new(RateIndex::Sofr).with_discount(RateIndex::Sofr);
        assert!(dep.is_self_discounting());
    }

    // ========================================
    // Build with Dependencies Tests
    // ========================================

    #[test]
    fn test_build_with_dependencies_single_curve() {
        let specs = vec![(
            CurveDependency::new(RateIndex::Sofr),
            vec![
                BootstrapInstrument::ois(1.0, 0.03),
                BootstrapInstrument::ois(2.0, 0.032),
            ],
        )];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert_eq!(curve_set.discount_index(), Some(RateIndex::Sofr));
    }

    #[test]
    fn test_build_with_dependencies_two_independent_curves() {
        let specs = vec![
            (
                CurveDependency::new(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Sonia),
                vec![
                    BootstrapInstrument::ois(1.0, 0.025),
                    BootstrapInstrument::ois(2.0, 0.028),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));
    }

    #[test]
    fn test_build_with_dependencies_dependent_curve() {
        let specs = vec![
            (
                CurveDependency::new(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        // Both curves should be built
        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));

        // SOFR should be the discount curve
        assert_eq!(curve_set.discount_index(), Some(RateIndex::Sofr));
    }

    #[test]
    fn test_build_with_dependencies_circular_detection() {
        let specs = vec![
            (
                CurveDependency::new(RateIndex::Sofr).with_discount(RateIndex::Sonia),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.025),
                    BootstrapInstrument::ois(2.0, 0.028),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let result = builder.build_with_dependencies(&specs);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CurveEngineError::CircularDependency { .. }),
            "Expected CircularDependency error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_build_with_dependencies_with_tenor() {
        let specs = vec![
            (
                CurveDependency::new(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Sonia)
                    .with_discount(RateIndex::Sofr)
                    .with_tenor(Tenor::ThreeMonth),
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        // Should also be available by tenor
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
    }

    #[test]
    fn test_build_with_dependencies_chain() {
        // A -> B -> C (C depends on B, B depends on A)
        let specs = vec![
            (
                CurveDependency::new(RateIndex::Sofr), // A - no dependency
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr), // B depends on A
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.037),
                ],
            ),
            (
                CurveDependency::new(RateIndex::Tonar).with_discount(RateIndex::Sonia), // C depends on B
                vec![
                    BootstrapInstrument::irs(1.0, 0.032),
                    BootstrapInstrument::irs(2.0, 0.034),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));
        assert!(curve_set.has_curve_for_index(RateIndex::Tonar));
    }
}

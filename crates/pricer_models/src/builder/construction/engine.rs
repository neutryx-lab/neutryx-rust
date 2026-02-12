//! Curve construction engine.
//!
//! This module provides the [`CurveConstructionEngine`] which orchestrates
//! the entire curve building process from definitions and market rates.

use infra_domain::market::{
    CurveDefinition, DefinitionRegistry, InstrumentDefinition, MarketQuoteSet, QuoteType,
};

use super::{converter::definition_to_instrument, error::ConstructionError};
use crate::{
    builder::{BootstrapConfig, BootstrapError, CurveBootstrapper},
    market::curves::{BootstrapInterpolation, BootstrappedCurve},
};

/// Configuration for the curve construction engine.
#[derive(Debug, Clone)]
pub struct ConstructionConfig {
    /// Convergence tolerance for pricing error.
    pub tolerance: f64,
    /// Maximum iterations per pillar.
    pub max_iterations: usize,
    /// Default quote type to use when fetching rates.
    pub quote_type: QuoteType,
    /// Whether to require all instruments (strict mode).
    /// If false, missing rates are skipped with a warning.
    pub strict_mode: bool,
    /// Finite difference epsilon for numerical derivative.
    pub fd_epsilon: f64,
    /// Reference date for Event instruments (year, month, day).
    /// Required when calibrating curves with Event instruments.
    pub reference_date: Option<(i32, u32, u32)>,
}

impl Default for ConstructionConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            quote_type: QuoteType::Mid,
            strict_mode: true,
            fd_epsilon: 1e-6,
            reference_date: None,
        }
    }
}

impl ConstructionConfig {
    /// Creates a new configuration with specified tolerance.
    #[must_use]
    pub fn new(tolerance: f64) -> Self {
        Self {
            tolerance,
            ..Default::default()
        }
    }

    /// Sets the maximum iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Sets the quote type.
    #[must_use]
    pub fn with_quote_type(mut self, quote_type: QuoteType) -> Self {
        self.quote_type = quote_type;
        self
    }

    /// Sets strict mode.
    #[must_use]
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Sets the finite difference epsilon.
    #[must_use]
    pub fn with_fd_epsilon(mut self, epsilon: f64) -> Self {
        self.fd_epsilon = epsilon;
        self
    }

    /// Sets the reference date for Event instruments.
    #[must_use]
    pub fn with_reference_date(mut self, year: i32, month: u32, day: u32) -> Self {
        self.reference_date = Some((year, month, day));
        self
    }
}

/// Result of a successful curve construction.
///
/// Note: Currently uses `f64` as the bootstrapper only supports `f64`.
/// Future versions may support generic float types.
#[derive(Debug, Clone)]
pub struct ConstructionResult {
    /// The bootstrapped yield curve.
    pub curve: BootstrappedCurve<f64>,
    /// Number of instruments used in calibration.
    pub instruments_used: usize,
    /// IDs of instruments with missing rates (only populated in non-strict
    /// mode).
    pub missing_rates: Vec<String>,
    /// Final calibration residual.
    pub residual: f64,
}

/// Curve construction engine.
///
/// Orchestrates curve building from:
/// - `DefinitionRegistry` - Contains instrument, rate index, and curve
///   definitions
/// - `MarketQuoteSet` - Contains market rate quotes
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::construction::{CurveConstructionEngine, ConstructionConfig};
/// use infra_domain::market::{DefinitionRegistry, MarketQuoteSet};
///
/// let registry = DefinitionRegistry::new();
/// // ... load definitions ...
///
/// let market_rates = MarketQuoteSet::new();
/// // ... load market rates ...
///
/// let engine = CurveConstructionEngine::new(ConstructionConfig::default());
/// let result = engine.build(&registry, &market_rates, "USD-SOFR-Discount")?;
///
/// println!("Built curve with {} instruments", result.instruments_used);
/// ```
pub struct CurveConstructionEngine {
    config: ConstructionConfig,
}

impl CurveConstructionEngine {
    /// Creates a new construction engine with the given configuration.
    #[must_use]
    pub fn new(config: ConstructionConfig) -> Self { Self { config } }

    /// Creates a new construction engine with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self { Self::new(ConstructionConfig::default()) }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &ConstructionConfig { &self.config }

    /// Builds a curve from the registry using the specified curve name.
    ///
    /// # Arguments
    ///
    /// * `registry` - The definition registry containing curve and instrument
    ///   definitions
    /// * `market_rates` - The market rates for calibration
    /// * `curve_name` - Name of the curve to build
    ///
    /// # Returns
    ///
    /// A `ConstructionResult` containing the built curve and diagnostics.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The curve is not found in the registry
    /// - Required instruments are missing (in strict mode)
    /// - Calibration fails to converge
    pub fn build(
        &self,
        registry: &DefinitionRegistry,
        market_rates: &MarketQuoteSet,
        curve_name: &str,
    ) -> Result<ConstructionResult, ConstructionError> {
        // Get curve definition
        let curve_def = registry
            .get_curve(curve_name)
            .ok_or_else(|| ConstructionError::curve_not_found(curve_name))?;

        // Resolve instrument definitions
        let instrument_defs: Vec<_> = curve_def
            .instruments
            .iter()
            .filter_map(|id| registry.get_instrument(id))
            .collect();

        self.build_from_definitions(curve_def, &instrument_defs, market_rates)
    }

    /// Builds a curve directly from definitions without a registry lookup.
    ///
    /// # Arguments
    ///
    /// * `curve_def` - The curve definition
    /// * `instrument_defs` - The instrument definitions to use
    /// * `market_rates` - The market rates for calibration
    ///
    /// # Returns
    ///
    /// A `ConstructionResult` containing the built curve and diagnostics.
    pub fn build_from_definitions(
        &self,
        curve_def: &CurveDefinition,
        instrument_defs: &[&InstrumentDefinition],
        market_rates: &MarketQuoteSet,
    ) -> Result<ConstructionResult, ConstructionError> {
        use crate::market::curves::MarketInstrument;

        let mut instruments: Vec<MarketInstrument<f64>> = Vec::new();
        let mut missing_rates = Vec::new();

        for def in instrument_defs {
            // Convert instrument definition to QuoteId for lookup
            let rate_id = def
                .to_quote_id()
                .map_err(ConstructionError::InstrumentDef)?;

            // Look up the rate in market data
            let rate_value = if self.config.quote_type == QuoteType::Mid {
                market_rates.get_mid_quote(&rate_id)
            } else {
                market_rates
                    .get_quote(&rate_id, self.config.quote_type)
                    .map(|q| q.value)
            };

            match rate_value {
                Some(value) => {
                    // Convert to MarketInstrument<f64>
                    let instrument =
                        definition_to_instrument(def, value, self.config.reference_date)?;
                    instruments.push(instrument);
                }
                None => {
                    if self.config.strict_mode {
                        return Err(ConstructionError::missing_rate(&def.id));
                    }
                    missing_rates.push(def.id.clone());
                }
            }
        }

        if instruments.is_empty() {
            return Err(ConstructionError::no_instruments(&curve_def.name));
        }

        // Convert CurveDefinition interpolation to builder interpolation
        let interpolation = match curve_def.interpolation {
            infra_domain::market::InterpolationMethod::Linear => BootstrapInterpolation::Linear,
            infra_domain::market::InterpolationMethod::LogLinear => {
                BootstrapInterpolation::LogLinear
            }
            infra_domain::market::InterpolationMethod::FlatForward => {
                BootstrapInterpolation::FlatForward
            }
        };

        // Configure bootstrapper
        let bootstrap_config = BootstrapConfig {
            interpolation,
            fd_epsilon: self.config.fd_epsilon,
            ..BootstrapConfig::new(self.config.tolerance, self.config.max_iterations)
        };

        let bootstrapper = CurveBootstrapper::with_config(bootstrap_config);

        // Run calibration
        let curve = bootstrapper
            .bootstrap_to_curve(&instruments)
            .map_err(map_bootstrap_error)?;

        // Get residual from a separate bootstrap call for diagnostics
        let result = bootstrapper
            .bootstrap_instruments(&instruments)
            .map_err(map_bootstrap_error)?;

        Ok(ConstructionResult {
            curve,
            instruments_used: instruments.len(),
            missing_rates,
            residual: result.residual,
        })
    }

    /// Builds multiple curves in order, returning all results.
    ///
    /// This is useful for multi-curve bootstrapping where curves may depend
    /// on each other.
    ///
    /// # Arguments
    ///
    /// * `registry` - The definition registry
    /// * `market_rates` - The market rates
    /// * `curve_names` - Names of curves to build in order
    ///
    /// # Returns
    ///
    /// A vector of construction results, one per curve.
    pub fn build_multi(
        &self,
        registry: &DefinitionRegistry,
        market_rates: &MarketQuoteSet,
        curve_names: &[&str],
    ) -> Vec<Result<ConstructionResult, ConstructionError>> {
        curve_names
            .iter()
            .map(|name| self.build(registry, market_rates, name))
            .collect()
    }
}

impl Default for CurveConstructionEngine {
    fn default() -> Self { Self::with_defaults() }
}

/// Maps BootstrapError to ConstructionError.
fn map_bootstrap_error(err: BootstrapError) -> ConstructionError {
    match err {
        BootstrapError::ConvergenceFailure {
            residual,
            iterations,
            ..
        } => ConstructionError::ConvergenceFailed {
            residual,
            iterations,
        },
        BootstrapError::InsufficientData { required, provided } => {
            ConstructionError::InvalidConfig {
                message: format!(
                    "Insufficient data: required {}, provided {}",
                    required, provided
                ),
            }
        }
        other => ConstructionError::calibration_failed(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use infra_domain::{
        market::{
            Currency, DataSource, MarketQuote, QuoteCategory, QuoteId, RateIndex,
            RateIndexDefinition,
        },
        time::Tenor,
    };

    use super::*;

    fn create_test_registry() -> DefinitionRegistry {
        let mut registry = DefinitionRegistry::new();

        // Register instruments
        let instruments = vec![
            InstrumentDefinition::new("USD-Depo-ON", Currency::USD, QuoteCategory::Deposit, "ON"),
            InstrumentDefinition::new("USD-Depo-1M", Currency::USD, QuoteCategory::Deposit, "1M"),
            InstrumentDefinition::new("USD-Depo-3M", Currency::USD, QuoteCategory::Deposit, "3M"),
            InstrumentDefinition::new("USD-OIS-1Y", Currency::USD, QuoteCategory::Ois, "1Y"),
            InstrumentDefinition::new("USD-OIS-2Y", Currency::USD, QuoteCategory::Ois, "2Y"),
            InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, QuoteCategory::Ois, "5Y"),
        ];

        for inst in instruments {
            let _ = registry.register_instrument(inst);
        }

        // Register rate index
        let sofr = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);
        registry.register_rate_index(sofr).unwrap();

        // Register curve definition
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec![
                "USD-Depo-ON".to_string(),
                "USD-Depo-1M".to_string(),
                "USD-Depo-3M".to_string(),
                "USD-OIS-1Y".to_string(),
                "USD-OIS-2Y".to_string(),
                "USD-OIS-5Y".to_string(),
            ],
        );
        registry.register_curve(curve).unwrap();

        registry
    }

    fn create_test_market_rates() -> MarketQuoteSet {
        let mut rates = MarketQuoteSet::new();
        let ts = 1700000000000_i64;

        let data = vec![
            (Tenor::Overnight, QuoteCategory::Deposit, 0.0530),
            (Tenor::OneMonth, QuoteCategory::Deposit, 0.0535),
            (Tenor::ThreeMonths, QuoteCategory::Deposit, 0.0540),
            (Tenor::OneYear, QuoteCategory::Ois, 0.0450),
            (Tenor::TwoYears, QuoteCategory::Ois, 0.0420),
            (Tenor::FiveYears, QuoteCategory::Ois, 0.0400),
        ];

        for (tenor, quote_category, value) in data {
            let rate_id = QuoteId::new(Currency::USD, tenor, quote_category);
            let rate = MarketQuote::new(rate_id, QuoteType::Mid, value, ts, DataSource::Bloomberg)
                .unwrap();
            rates.insert(rate);
        }

        rates
    }

    #[test]
    fn test_construction_engine_default() {
        let engine = CurveConstructionEngine::default();
        assert!((engine.config().tolerance - 1e-10).abs() < f64::EPSILON);
        assert_eq!(engine.config().max_iterations, 100);
        assert!(engine.config().strict_mode);
    }

    #[test]
    fn test_construction_config_builder() {
        let config = ConstructionConfig::new(1e-12)
            .with_max_iterations(200)
            .with_quote_type(QuoteType::Bid)
            .with_strict_mode(false)
            .with_fd_epsilon(1e-8);

        assert!((config.tolerance - 1e-12).abs() < f64::EPSILON);
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.quote_type, QuoteType::Bid);
        assert!(!config.strict_mode);
        assert!((config.fd_epsilon - 1e-8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_curve_success() {
        let registry = create_test_registry();
        let market_rates = create_test_market_rates();

        let engine = CurveConstructionEngine::default();
        let result = engine.build(&registry, &market_rates, "USD-SOFR-Discount");

        assert!(result.is_ok(), "Build failed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.instruments_used, 6);
        assert!(result.missing_rates.is_empty());
        assert!(
            result.residual < 1e-6,
            "Residual too large: {}",
            result.residual
        );
    }

    #[test]
    fn test_build_curve_not_found() {
        let registry = create_test_registry();
        let market_rates = create_test_market_rates();

        let engine = CurveConstructionEngine::default();
        let result = engine.build(&registry, &market_rates, "NonExistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            ConstructionError::CurveNotFound { curve_name } => {
                assert_eq!(curve_name, "NonExistent");
            }
            other => panic!("Expected CurveNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_build_missing_rate_strict_mode() {
        let mut registry = DefinitionRegistry::new();

        // Register instrument with no corresponding market rate
        let _ = registry.register_instrument(InstrumentDefinition::new(
            "USD-OIS-30Y",
            Currency::USD,
            QuoteCategory::Ois,
            "30Y",
        ));

        let sofr = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);
        registry.register_rate_index(sofr).unwrap();

        let curve =
            CurveDefinition::new("USD-SOFR-30Y", "USD-SOFR", vec!["USD-OIS-30Y".to_string()]);
        registry.register_curve(curve).unwrap();

        let market_rates = MarketQuoteSet::new(); // Empty

        let engine =
            CurveConstructionEngine::new(ConstructionConfig::default().with_strict_mode(true));
        let result = engine.build(&registry, &market_rates, "USD-SOFR-30Y");

        assert!(result.is_err());
        match result.unwrap_err() {
            ConstructionError::MissingRate { instrument_id } => {
                assert_eq!(instrument_id, "USD-OIS-30Y");
            }
            other => panic!("Expected MissingRate, got {:?}", other),
        }
    }

    #[test]
    fn test_build_missing_rate_non_strict_mode() {
        let registry = create_test_registry();

        // Create rates without the 5Y point
        let mut market_rates = MarketQuoteSet::new();
        let ts = 1700000000000_i64;

        let data = vec![
            (Tenor::Overnight, QuoteCategory::Deposit, 0.0530),
            (Tenor::OneMonth, QuoteCategory::Deposit, 0.0535),
            (Tenor::ThreeMonths, QuoteCategory::Deposit, 0.0540),
            (Tenor::OneYear, QuoteCategory::Ois, 0.0450),
            (Tenor::TwoYears, QuoteCategory::Ois, 0.0420),
            // USD-OIS-5Y is missing
        ];

        for (tenor, quote_category, value) in data {
            let rate_id = QuoteId::new(Currency::USD, tenor, quote_category);
            let rate = MarketQuote::new(rate_id, QuoteType::Mid, value, ts, DataSource::Bloomberg)
                .unwrap();
            market_rates.insert(rate);
        }

        let engine =
            CurveConstructionEngine::new(ConstructionConfig::default().with_strict_mode(false));
        let result = engine.build(&registry, &market_rates, "USD-SOFR-Discount");

        assert!(result.is_ok(), "Build failed: {:?}", result.err());

        let result = result.unwrap();
        assert_eq!(result.instruments_used, 5); // One less
        assert_eq!(result.missing_rates.len(), 1);
        assert_eq!(result.missing_rates[0], "USD-OIS-5Y");
    }

    #[test]
    fn test_build_multi() {
        let mut registry = create_test_registry();

        // Add a second curve
        let eur_instruments = vec![
            InstrumentDefinition::new("EUR-Depo-ON", Currency::EUR, QuoteCategory::Deposit, "ON"),
            InstrumentDefinition::new("EUR-OIS-1Y", Currency::EUR, QuoteCategory::Ois, "1Y"),
        ];
        for inst in eur_instruments {
            let _ = registry.register_instrument(inst);
        }

        let estr = RateIndexDefinition::new("EUR-ESTR", Currency::EUR, RateIndex::Estr);
        registry.register_rate_index(estr).unwrap();

        let eur_curve = CurveDefinition::new(
            "EUR-ESTR-Discount",
            "EUR-ESTR",
            vec!["EUR-Depo-ON".to_string(), "EUR-OIS-1Y".to_string()],
        );
        registry.register_curve(eur_curve).unwrap();

        // Add EUR rates
        let mut market_rates = create_test_market_rates();
        let ts = 1700000000000_i64;

        market_rates.insert(
            MarketQuote::new(
                QuoteId::new(Currency::EUR, Tenor::Overnight, QuoteCategory::Deposit),
                QuoteType::Mid,
                0.0390,
                ts,
                DataSource::Bloomberg,
            )
            .unwrap(),
        );
        market_rates.insert(
            MarketQuote::new(
                QuoteId::new(Currency::EUR, Tenor::OneYear, QuoteCategory::Ois),
                QuoteType::Mid,
                0.0350,
                ts,
                DataSource::Bloomberg,
            )
            .unwrap(),
        );

        let engine = CurveConstructionEngine::default();
        let results = engine.build_multi(
            &registry,
            &market_rates,
            &["USD-SOFR-Discount", "EUR-ESTR-Discount"],
        );

        assert_eq!(results.len(), 2);
        assert!(
            results[0].is_ok(),
            "USD build failed: {:?}",
            results[0].as_ref().err()
        );
        assert!(
            results[1].is_ok(),
            "EUR build failed: {:?}",
            results[1].as_ref().err()
        );
    }

    #[test]
    fn test_construction_result_curve_access() {
        use crate::market::curves::YieldCurve;

        let registry = create_test_registry();
        let market_rates = create_test_market_rates();

        let engine = CurveConstructionEngine::default();
        let result = engine
            .build(&registry, &market_rates, "USD-SOFR-Discount")
            .unwrap();

        // Verify curve produces valid discount factors
        let df_1y = result.curve.discount_factor(1.0).unwrap();
        let df_5y = result.curve.discount_factor(5.0).unwrap();

        assert!(df_1y > 0.0 && df_1y < 1.0);
        assert!(df_5y > 0.0 && df_5y < 1.0);
        assert!(df_1y > df_5y, "DF should decrease with maturity");
    }
}

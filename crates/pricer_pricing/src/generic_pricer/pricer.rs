//! Generic Pricer implementation.
//!
//! Provides a unified pricing API for financial instruments.
//!
//! # Dual-mode Support
//!
//! This module supports two pricing modes:
//!
//! 1. **Standalone mode** (always available): Uses `SimpleLeg`,
//!    `SimpleCashflow`, `SimpleDate`, `SimpleDirection`, and `DefaultCurrency`.
//!    No external market data provider required. Use `new_standalone()` and
//!    `get_pv_simple()`.
//!
//! 2. **Integrated mode** (with `l1l2-integration` feature): Uses
//!    `infra_master` types and `MarketProvider`. Use `new()` and `get_pv()`.

#[cfg(feature = "l1l2-integration")]
use std::sync::Arc;

#[cfg(feature = "l1l2-integration")]
use chrono::Datelike;
#[cfg(feature = "l1l2-integration")]
use infra_config::{PricingConfig, PricingMethod};
#[cfg(feature = "l1l2-integration")]
use infra_master::{
    market::Currency,
    time::Date,
    trade::{Leg, Trade},
};
#[cfg(feature = "l1l2-integration")]
use pricer_models::market::{MarketProvider, YieldCurve};

// Standalone types - always available
use super::config::DefaultCurrency;
#[cfg(feature = "l1l2-integration")]
use super::payoff_evaluator::PayoffEvaluator;
#[cfg(feature = "l1l2-integration")]
use super::result::{CashflowPricingResult, LegPricingResult, PricingResult};
use super::{
    config::{ModelConfig, PricerConfig},
    error::PricingError,
    result::{SimpleDate, SimpleDirection},
};

/// Generic pricer for unified pricing API.
///
/// This is a concrete struct (not a trait) as a single implementation
/// is sufficient for all use cases. The pricer handles market data
/// resolution and delegates actual computation to the pricing kernel.
///
/// # Example
///
/// ```rust,ignore
/// use pricer_pricing::generic_pricer::{GenericPricer, ModelConfig, PricerConfig};
///
/// let pricer = GenericPricer::new(market, model_config, pricer_config);
/// let result = pricer.get_pv(&trade, valuation_date, Currency::USD)?;
/// ```
#[derive(Debug, Clone)]
pub struct GenericPricer {
    /// Market data provider (Arc-shared for thread safety).
    #[cfg(feature = "l1l2-integration")]
    market: Arc<MarketProvider>,

    /// Model configuration (simulation parameters).
    model_config: ModelConfig,

    /// Pricer configuration (Greeks, default currency, etc.).
    pricer_config: PricerConfig,
}

impl GenericPricer {
    /// Creates a new GenericPricer.
    ///
    /// # Arguments
    ///
    /// * `market` - Arc-shared market data provider
    /// * `model_config` - Model and simulation configuration
    /// * `pricer_config` - Pricer output configuration
    #[cfg(feature = "l1l2-integration")]
    pub fn new(
        market: Arc<MarketProvider>,
        model_config: ModelConfig,
        pricer_config: PricerConfig,
    ) -> Self {
        Self {
            market,
            model_config,
            pricer_config,
        }
    }

    /// Creates a new GenericPricer (standalone mode without market provider).
    #[cfg(not(feature = "l1l2-integration"))]
    pub fn new(model_config: ModelConfig, pricer_config: PricerConfig) -> Self {
        Self {
            model_config,
            pricer_config,
        }
    }

    /// Creates a new GenericPricer from a PricingConfig.
    ///
    /// This factory method converts `infra_config::PricingConfig` to the
    /// internal `ModelConfig` and `PricerConfig` structures, selecting the
    /// appropriate pricing method based on configuration.
    ///
    /// # Arguments
    ///
    /// * `market` - Arc-shared market data provider
    /// * `config` - Configuration from infra_config
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use infra_config::PricingConfig;
    /// use pricer_pricing::generic_pricer::GenericPricer;
    ///
    /// let pricing_config = PricingConfig::from_toml_str(toml_str)?;
    /// let pricer = GenericPricer::from_config(market, &pricing_config)?;
    /// ```
    #[cfg(feature = "l1l2-integration")]
    pub fn from_config(
        market: Arc<MarketProvider>,
        config: &PricingConfig,
    ) -> Result<Self, PricingError> {
        use super::config::{ModelConfigBuilder, PricerConfigBuilder};

        // Convert Monte Carlo parameters if specified
        let mut model_builder = ModelConfigBuilder::default();
        if let Some(ref mc_params) = config.monte_carlo {
            model_builder = model_builder
                .num_paths(mc_params.num_paths)
                .num_steps(mc_params.num_steps);
            if let Some(seed) = mc_params.seed {
                model_builder = model_builder.seed(seed);
            }
        }

        let model_config = model_builder
            .build()
            .map_err(|e| PricingError::InvalidInput {
                reason: format!("Invalid model configuration: {}", e),
            })?;

        // Convert currency string to Currency enum
        let currency: Currency =
            config
                .reporting_currency
                .parse()
                .map_err(|_| PricingError::InvalidInput {
                    reason: format!("Invalid currency code: {}", config.reporting_currency),
                })?;

        let pricer_config = PricerConfigBuilder::default()
            .default_currency(currency)
            .use_thread_local_buffers(config.parallel_enabled)
            .build()
            .map_err(|e| PricingError::InvalidInput {
                reason: format!("Invalid pricer configuration: {}", e),
            })?;

        Ok(Self {
            market,
            model_config,
            pricer_config,
        })
    }

    /// Returns the pricing method from config (Analytical or MonteCarlo).
    #[cfg(feature = "l1l2-integration")]
    pub fn pricing_method_from_config(config: &PricingConfig) -> PricingMethod {
        config.pricing_method
    }

    /// Prices a trade using configuration settings.
    ///
    /// This is a convenience method that extracts valuation date and
    /// reporting currency from the PricingConfig and delegates to `get_pv()`.
    ///
    /// # Arguments
    ///
    /// * `trade` - The trade to price
    /// * `config` - Pricing configuration containing valuation date and
    ///   currency
    ///
    /// # Returns
    ///
    /// `PricingResult` containing total PV, leg-level breakdown, and metadata.
    ///
    /// # Errors
    ///
    /// Returns `PricingError` if:
    /// - Required market data is missing
    /// - Currency code is invalid
    /// - The instrument type is not supported
    #[cfg(feature = "l1l2-integration")]
    pub fn price_with_config(
        &self,
        trade: &Trade,
        config: &PricingConfig,
    ) -> Result<PricingResult, PricingError> {
        // Convert valuation date from NaiveDate to Date
        let valuation_date = Date::from_ymd(
            config.valuation_date.year(),
            config.valuation_date.month(),
            config.valuation_date.day(),
        )
        .map_err(|e| PricingError::InvalidInput {
            reason: format!("Invalid valuation date: {}", e),
        })?;

        // Parse reporting currency
        let reporting_currency: Currency =
            config
                .reporting_currency
                .parse()
                .map_err(|_| PricingError::InvalidInput {
                    reason: format!("Invalid currency code: {}", config.reporting_currency),
                })?;

        self.get_pv(trade, valuation_date, reporting_currency)
    }

    /// Returns a reference to the model configuration.
    pub fn model_config(&self) -> &ModelConfig { &self.model_config }

    /// Returns a reference to the pricer configuration.
    pub fn pricer_config(&self) -> &PricerConfig { &self.pricer_config }

    /// Computes the present value of a trade.
    ///
    /// # Arguments
    ///
    /// * `trade` - The trade to price
    /// * `valuation_date` - The date at which to evaluate
    /// * `reporting_currency` - The currency for the output PV (required for
    ///   risk calculations)
    ///
    /// # Returns
    ///
    /// `PricingResult` containing total PV, leg-level breakdown, and optional
    /// path distribution.
    ///
    /// # Errors
    ///
    /// Returns `PricingError` if:
    /// - Required market data is missing
    /// - FX rate is not available
    /// - The instrument type is not supported
    #[cfg(feature = "l1l2-integration")]
    pub fn get_pv(
        &self,
        trade: &Trade,
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> Result<PricingResult, PricingError> {
        // Stage 2: Market data resolution
        let mut legs_results = Vec::with_capacity(trade.num_legs());

        for leg in trade.legs() {
            let leg_result = self.price_leg(leg, valuation_date, reporting_currency)?;
            legs_results.push(leg_result);
        }

        // Calculate total PV
        let total_pv: f64 = legs_results.iter().map(|leg| leg.pv).sum();

        Ok(PricingResult::new(
            total_pv,
            legs_results,
            reporting_currency,
        ))
    }

    /// Prices a single leg.
    #[cfg(feature = "l1l2-integration")]
    fn price_leg(
        &self,
        leg: &Leg,
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> Result<LegPricingResult, PricingError> {
        let leg_currency = leg.currency;

        // Get discount curve for the leg currency
        let curve = self.market.get_curve(leg_currency)
            .ok_or_else(|| PricingError::market_data_resolution(
                format!("No curve found for currency {:?}", leg_currency)
            ))?;

        // Get FX rate (1.0 if same currency)
        let fx_rate = if leg_currency == reporting_currency {
            1.0
        } else {
            self.get_fx_rate(leg_currency, reporting_currency)?
        };

        // Get curve set for payoff evaluation
        let curve_set = self.market.curve_set();

        // Price each cashflow
        let mut cashflows_results = Vec::with_capacity(leg.len());
        let mut pv_original = 0.0;

        for cf in leg.cashflows() {
            // Skip past cashflows
            if cf.payment_date <= valuation_date {
                continue;
            }

            // Calculate discount factor
            let payment_days = cf.payment_date.into_inner().num_days_from_ce();
            let valuation_days = valuation_date.into_inner().num_days_from_ce();
            let time_to_payment = (payment_days - valuation_days) as f64 / 365.0;
            let df = curve
                .discount_factor(time_to_payment)
                .map_err(|e| PricingError::market_data_resolution(format!("{:?}", e)))?;

            // Calculate cashflow amount using PayoffEvaluator
            let cf_amount = self.evaluate_cashflow_amount(cf, valuation_date, &curve_set)?;
            let cf_pv_original = cf_amount * df;
            let cf_pv = cf_pv_original * fx_rate;

            pv_original += cf_pv_original;

            cashflows_results.push(CashflowPricingResult::new(
                cf_pv,
                cf_pv_original,
                cf.payment_date,
                df,
                leg_currency,
            ));
        }

        // Apply direction
        let direction = leg.direction;
        let pv = pv_original * fx_rate * direction.sign();
        let pv_original_signed = pv_original * direction.sign();

        Ok(LegPricingResult::new(
            pv,
            pv_original_signed,
            leg_currency,
            fx_rate,
            direction,
            cashflows_results,
        ))
    }

    /// Evaluates the cashflow amount based on its payoff type.
    #[cfg(feature = "l1l2-integration")]
    fn evaluate_cashflow_amount(
        &self,
        cf: &infra_master::trade::Cashflow,
        valuation_date: Date,
        curve_set: &pricer_models::market::curves::CurveSet<f64>,
    ) -> Result<f64, PricingError> {
        let notional = cf.notional;
        let year_fraction = cf.year_fraction;

        let evaluator = PayoffEvaluator::new(curve_set);

        // Calculate time parameters for forward rate calculation
        let valuation_days = valuation_date.into_inner().num_days_from_ce();
        let start_days = cf.accrual_start.into_inner().num_days_from_ce();
        let end_days = cf.accrual_end.into_inner().num_days_from_ce();

        let start_time = (start_days - valuation_days) as f64 / 365.0;
        let end_time = (end_days - valuation_days) as f64 / 365.0;

        evaluator.evaluate(&cf.payoff, notional, year_fraction, start_time, end_time)
    }

    /// Gets the notional for a cashflow.
    ///
    /// Note: This method is preserved for backward compatibility. The new
    /// `evaluate_cashflow_amount` method extracts notional from `cf.notional`.
    #[cfg(feature = "l1l2-integration")]
    #[allow(dead_code)]
    fn get_notional_for_cashflow(&self, _cf: &infra_master::trade::Cashflow, _leg: &Leg) -> f64 {
        // TODO: Extract notional from cashflow/leg based on cashflow type
        // For now, return a default notional
        1_000_000.0
    }

    /// Gets the FX rate between two currencies.
    #[cfg(feature = "l1l2-integration")]
    fn get_fx_rate(&self, from: Currency, to: Currency) -> Result<f64, PricingError> {
        // TODO: Implement MarketProvider::get_fx_rate when available
        // For now, return an error indicating the feature is not yet implemented
        Err(PricingError::fx_rate_not_found(from, to))
    }
}

// =============================================================================
// Standalone mode implementation (always available)
// =============================================================================

/// Simple leg representation for standalone pricing.
///
/// Always available regardless of l1l2-integration feature.
/// Use this for standalone pricing without full market data integration.
#[derive(Debug, Clone)]
pub struct SimpleLeg {
    /// Currency of the leg.
    pub currency: DefaultCurrency,
    /// Direction of the leg.
    pub direction: SimpleDirection,
    /// Cashflows in the leg.
    pub cashflows: Vec<SimpleCashflow>,
}

/// Simple cashflow representation for standalone pricing.
///
/// Always available regardless of l1l2-integration feature.
/// Use this for standalone pricing without full market data integration.
#[derive(Debug, Clone)]
pub struct SimpleCashflow {
    /// Payment date.
    pub payment_date: SimpleDate,
    /// Cashflow amount.
    pub amount: f64,
}

impl GenericPricer {
    /// Creates a new GenericPricer in standalone mode (no market provider).
    ///
    /// Always available regardless of l1l2-integration feature.
    /// Use this for standalone pricing without full market data integration.
    pub fn new_standalone(model_config: ModelConfig, pricer_config: PricerConfig) -> Self {
        Self {
            #[cfg(feature = "l1l2-integration")]
            market: std::sync::Arc::new(pricer_models::market::MarketProvider::new()),
            model_config,
            pricer_config,
        }
    }

    /// Computes the present value using simplified inputs (standalone mode).
    ///
    /// Always available regardless of l1l2-integration feature.
    /// This mode doesn't require full market data integration.
    /// Useful for testing, demonstration, and web API purposes.
    pub fn get_pv_simple(
        &self,
        legs: Vec<SimpleLeg>,
        valuation_date: SimpleDate,
        reporting_currency: DefaultCurrency,
    ) -> Result<StandalonePricingResult, PricingError> {
        let mut legs_results = Vec::with_capacity(legs.len());

        for leg in &legs {
            let leg_result = self.price_simple_leg(leg, valuation_date, reporting_currency)?;
            legs_results.push(leg_result);
        }

        let total_pv: f64 = legs_results.iter().map(|leg| leg.pv).sum();

        Ok(StandalonePricingResult {
            total_pv,
            legs: legs_results,
            reporting_currency,
        })
    }

    /// Prices a simple leg (standalone mode).
    fn price_simple_leg(
        &self,
        leg: &SimpleLeg,
        valuation_date: SimpleDate,
        reporting_currency: DefaultCurrency,
    ) -> Result<StandaloneLegResult, PricingError> {
        // Simple flat rate discounting
        let discount_rate = 0.05; // 5% flat rate for demo

        // Get FX rate (1.0 if same currency)
        let fx_rate = if leg.currency == reporting_currency {
            1.0
        } else {
            // For standalone mode, use placeholder FX rates
            get_placeholder_fx_rate(leg.currency, reporting_currency)?
        };

        let mut cashflows_results = Vec::with_capacity(leg.cashflows.len());
        let mut pv_original = 0.0;

        for cf in &leg.cashflows {
            // Skip past cashflows
            if cf.payment_date.0 <= valuation_date.0 {
                continue;
            }

            // Calculate time to payment (simple day count)
            let time_to_payment = (cf.payment_date.0 - valuation_date.0) as f64 / 365.0;

            // Calculate discount factor
            let df = (-discount_rate * time_to_payment).exp();

            // Calculate cashflow PV
            let cf_pv_original = cf.amount * df;
            let cf_pv = cf_pv_original * fx_rate;

            pv_original += cf_pv_original;

            cashflows_results.push(StandaloneCashflowResult {
                pv: cf_pv,
                pv_original: cf_pv_original,
                payment_date: cf.payment_date,
                discount_factor: df,
            });
        }

        // Apply direction
        let pv = pv_original * fx_rate * leg.direction.sign();
        let pv_original_signed = pv_original * leg.direction.sign();

        Ok(StandaloneLegResult {
            pv,
            pv_original: pv_original_signed,
            original_currency: leg.currency,
            fx_rate,
            direction: leg.direction,
            cashflows: cashflows_results,
        })
    }
}

/// Standalone pricing result (always available).
#[derive(Debug, Clone)]
pub struct StandalonePricingResult {
    /// Total PV in reporting currency.
    pub total_pv: f64,
    /// Leg-level results.
    pub legs: Vec<StandaloneLegResult>,
    /// Reporting currency.
    pub reporting_currency: DefaultCurrency,
}

/// Standalone leg result (always available).
#[derive(Debug, Clone)]
pub struct StandaloneLegResult {
    /// PV in reporting currency.
    pub pv: f64,
    /// PV in original currency.
    pub pv_original: f64,
    /// Original leg currency.
    pub original_currency: DefaultCurrency,
    /// FX rate used.
    pub fx_rate: f64,
    /// Direction.
    pub direction: SimpleDirection,
    /// Cashflow results.
    pub cashflows: Vec<StandaloneCashflowResult>,
}

/// Standalone cashflow result (always available).
#[derive(Debug, Clone)]
pub struct StandaloneCashflowResult {
    /// PV in reporting currency.
    pub pv: f64,
    /// PV in original currency.
    pub pv_original: f64,
    /// Payment date.
    pub payment_date: SimpleDate,
    /// Discount factor.
    pub discount_factor: f64,
}

impl StandalonePricingResult {
    /// Returns the number of legs.
    pub fn leg_count(&self) -> usize { self.legs.len() }

    /// Returns the total number of cashflows across all legs.
    pub fn cashflow_count(&self) -> usize { self.legs.iter().map(|l| l.cashflows.len()).sum() }

    /// Groups PV by currency.
    ///
    /// Returns a map from currency code to total PV in that currency.
    pub fn group_by_currency(&self) -> std::collections::HashMap<&str, f64> {
        let mut result = std::collections::HashMap::new();
        for leg in &self.legs {
            *result.entry(leg.original_currency.code()).or_insert(0.0) += leg.pv_original;
        }
        result
    }
}

/// Gets a placeholder FX rate for standalone mode.
fn get_placeholder_fx_rate(
    from: DefaultCurrency,
    to: DefaultCurrency,
) -> Result<f64, PricingError> {
    // Simple placeholder rates for testing
    // In production, this would use the market provider
    match (from.code(), to.code()) {
        ("USD", "EUR") => Ok(0.92),
        ("EUR", "USD") => Ok(1.087),
        ("USD", "JPY") => Ok(149.5),
        ("JPY", "USD") => Ok(1.0 / 149.5),
        ("EUR", "JPY") => Ok(162.5),
        ("JPY", "EUR") => Ok(1.0 / 162.5),
        ("USD", "GBP") => Ok(0.79),
        ("GBP", "USD") => Ok(1.266),
        (a, b) if a == b => Ok(1.0),
        _ => Err(PricingError::standalone_fx_rate_not_found(
            from.code(),
            to.code(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Type aliases for standalone mode tests
    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::config::DefaultCurrency as Currency;
    use crate::generic_pricer::config::{ModelConfigBuilder, PricerConfigBuilder};
    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::result::{Date, Direction};

    #[test]
    fn test_generic_pricer_creation() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();

        #[cfg(not(feature = "l1l2-integration"))]
        let pricer = GenericPricer::new(model_config.clone(), pricer_config.clone());

        #[cfg(not(feature = "l1l2-integration"))]
        {
            assert_eq!(pricer.model_config().num_paths, model_config.num_paths);
            assert_eq!(
                pricer.pricer_config().use_thread_local_buffers,
                pricer_config.use_thread_local_buffers
            );
        }
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_simple_pricing() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365); // 1 year from now

        let leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0, // 100k
            }],
        };

        let result = pricer
            .get_pv_simple(vec![leg], valuation_date, Currency::USD)
            .unwrap();

        // PV should be discounted amount
        // With 5% flat rate, 1 year: 100k * exp(-0.05) ≈ 95,123
        assert!(result.total_pv > 95_000.0 && result.total_pv < 96_000.0);
        assert_eq!(result.leg_count(), 1);
        assert_eq!(result.cashflow_count(), 1);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_simple_pricing_with_fx() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        let leg = SimpleLeg {
            currency: Currency::EUR,
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        let result = pricer
            .get_pv_simple(vec![leg], valuation_date, Currency::USD)
            .unwrap();

        // EUR 100k discounted, then converted to USD (EUR/USD ≈ 1.087)
        // Expected: 95_123 * 1.087 ≈ 103,398
        assert!(result.total_pv > 103_000.0 && result.total_pv < 104_000.0);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_simple_pricing_payer_receiver() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        // Receiver leg
        let receiver_leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        // Payer leg
        let payer_leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Payer,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        let result = pricer
            .get_pv_simple(vec![receiver_leg, payer_leg], valuation_date, Currency::USD)
            .unwrap();

        // Receiver + Payer with same amounts should net to 0
        assert!(result.total_pv.abs() < 1e-10);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_simple_pricing_skip_past_cashflows() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(100);

        let leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![
                SimpleCashflow {
                    payment_date: Date::from_days(50), // Past
                    amount: 1_000_000.0,
                },
                SimpleCashflow {
                    payment_date: Date::from_days(200), // Future
                    amount: 100_000.0,
                },
            ],
        };

        let result = pricer
            .get_pv_simple(vec![leg], valuation_date, Currency::USD)
            .unwrap();

        // Only future cashflow should be priced
        assert_eq!(result.cashflow_count(), 1);
        // PV should be much less than 1M (past cashflow ignored)
        assert!(result.total_pv < 200_000.0);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_fx_rate_not_found() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        let leg = SimpleLeg {
            currency: Currency::CHF, // Swiss Franc - not in placeholder rates
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        let result = pricer.get_pv_simple(vec![leg], valuation_date, Currency::USD);

        // Standalone mode uses StandaloneFxRateNotFound (not FxRateNotFound)
        assert!(matches!(
            result,
            Err(PricingError::StandaloneFxRateNotFound { .. })
        ));
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_pricer_config_accessors() {
        let model_config = ModelConfigBuilder::default()
            .num_paths(5000)
            .num_steps(50)
            .build()
            .unwrap();
        let pricer_config = PricerConfigBuilder::default()
            .use_thread_local_buffers(false)
            .build()
            .unwrap();

        let pricer = GenericPricer::new(model_config, pricer_config);

        assert_eq!(pricer.model_config().num_paths, 5000);
        assert_eq!(pricer.model_config().num_steps, 50);
        assert!(!pricer.pricer_config().use_thread_local_buffers);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_multiple_legs_multiple_currencies() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        // USD leg (receiver)
        let usd_leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        // EUR leg (payer)
        let eur_leg = SimpleLeg {
            currency: Currency::EUR,
            direction: Direction::Payer,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 87_500.0, // Roughly equivalent to 95k USD at 1.087
            }],
        };

        let result = pricer
            .get_pv_simple(vec![usd_leg, eur_leg], valuation_date, Currency::USD)
            .unwrap();

        assert_eq!(result.leg_count(), 2);

        // Check group_by_currency
        let by_ccy = result.group_by_currency();
        assert_eq!(by_ccy.len(), 2);
    }
}

//! Generic Pricer implementation.
//!
//! Provides a unified pricing API for financial instruments.

#[cfg(feature = "l1l2-integration")]
use std::sync::Arc;

#[cfg(feature = "l1l2-integration")]
use chrono::Datelike;
#[cfg(feature = "l1l2-integration")]
use infra_master::{
    market::Currency,
    time::Date,
    trade::{Leg, Trade},
};
#[cfg(feature = "l1l2-integration")]
use pricer_models::market::{MarketProvider, YieldCurve};

#[cfg(not(feature = "l1l2-integration"))]
use super::config::DefaultCurrency as Currency;
#[cfg(not(feature = "l1l2-integration"))]
use super::result::Date;
#[cfg(not(feature = "l1l2-integration"))]
use super::result::Direction;
use super::{
    config::{ModelConfig, PricerConfig},
    error::PricingError,
    result::{CashflowPricingResult, LegPricingResult, PricingResult},
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
        let curve = self.market.get_curve(leg_currency);

        // Get FX rate (1.0 if same currency)
        let fx_rate = if leg_currency == reporting_currency {
            1.0
        } else {
            self.get_fx_rate(leg_currency, reporting_currency)?
        };

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

            // Calculate cashflow PV
            let cf_amount = cf.year_fraction * self.get_notional_for_cashflow(cf, leg);
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

    /// Gets the notional for a cashflow.
    #[cfg(feature = "l1l2-integration")]
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

// Standalone mode implementation (no l1l2-integration)
#[cfg(not(feature = "l1l2-integration"))]
impl GenericPricer {
    /// Computes the present value using simplified inputs.
    ///
    /// This is a standalone mode that doesn't require full market data
    /// integration. Useful for testing and demonstration purposes.
    pub fn get_pv_simple(
        &self,
        legs: Vec<SimpleLeg>,
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> Result<PricingResult, PricingError> {
        let mut legs_results = Vec::with_capacity(legs.len());

        for leg in &legs {
            let leg_result = self.price_simple_leg(leg, valuation_date, reporting_currency)?;
            legs_results.push(leg_result);
        }

        let total_pv: f64 = legs_results.iter().map(|leg| leg.pv).sum();

        Ok(PricingResult::new(
            total_pv,
            legs_results,
            reporting_currency,
        ))
    }

    /// Prices a simple leg.
    fn price_simple_leg(
        &self,
        leg: &SimpleLeg,
        valuation_date: Date,
        reporting_currency: Currency,
    ) -> Result<LegPricingResult, PricingError> {
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

            cashflows_results.push(CashflowPricingResult::new(
                cf_pv,
                cf_pv_original,
                cf.payment_date,
                df,
                leg.currency,
            ));
        }

        // Apply direction
        let pv = pv_original * fx_rate * leg.direction.sign();
        let pv_original_signed = pv_original * leg.direction.sign();

        Ok(LegPricingResult::new(
            pv,
            pv_original_signed,
            leg.currency,
            fx_rate,
            leg.direction,
            cashflows_results,
        ))
    }
}

/// Simple leg representation for standalone mode.
#[cfg(not(feature = "l1l2-integration"))]
#[derive(Debug, Clone)]
pub struct SimpleLeg {
    /// Currency of the leg.
    pub currency: Currency,
    /// Direction of the leg.
    pub direction: Direction,
    /// Cashflows in the leg.
    pub cashflows: Vec<SimpleCashflow>,
}

/// Simple cashflow representation for standalone mode.
#[cfg(not(feature = "l1l2-integration"))]
#[derive(Debug, Clone)]
pub struct SimpleCashflow {
    /// Payment date.
    pub payment_date: Date,
    /// Cashflow amount.
    pub amount: f64,
}

/// Gets a placeholder FX rate for standalone mode.
#[cfg(not(feature = "l1l2-integration"))]
fn get_placeholder_fx_rate(from: Currency, to: Currency) -> Result<f64, PricingError> {
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
        _ => Err(PricingError::FxRateNotFound {
            base: from.code().to_string(),
            quote: to.code().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pricer::config::{ModelConfigBuilder, PricerConfigBuilder};

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

        assert!(matches!(result, Err(PricingError::FxRateNotFound { .. })));
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

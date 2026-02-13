//! Unified Pricer — sole entry point for all PV computation.
//!
//! The [`Pricer`] dispatches to the correct pricing path based on [`TradeType`]
//! and [`CalcSetting`]:
//!
//! - **Linear products** (Swap, FxForward, Bond, Deposit, …) → cashflow
//!   discounting via the existing [`GenericPricer`] infrastructure.
//! - **Vanilla options** (FxOption, EquityOption, CommodityOption) →
//!   closed-form analytical formulae (Garman-Kohlhagen, Black-Scholes).
//! - **Barrier / exotic options** (FxBarrierOption, …) → Script engine
//!   (placeholder).
//! - **Monte Carlo / Tree** → delegated to existing MC and tree engines
//!   (placeholder).
//!
//! The struct is stateless; all data arrives through function arguments.

use std::time::Instant;

use chrono::Datelike;
use infra_config::PricingMethod;
use infra_domain::{
    market::Currency,
    time::Date,
    trade::{Leg, OptionType, Trade, TradeType},
};
use pricer_core::math::formulas::{
    black_scholes::BlackScholes,
    garman_kohlhagen::{GarmanKohlhagen, GarmanKohlhagenParams},
};
use pricer_models::market::{curves::YieldCurve, CurveSet, MarketProvider};

use crate::{
    calc_setting::{CalcSetting, PricingMethodHint},
    generic_pricer::{
        CashflowPricingResult, LegPricingResult, PayoffEvaluator, PricingError, PricingResult,
    },
    result::{PricingMetadata, UnifiedGreeks, UnifiedPricingResult},
};

// ---------------------------------------------------------------------------
// MarketEnvironment — will be provided by Agent 1 in
// `pricer_models::market_env`.  Until then we define a thin adapter over the
// existing `MarketProvider` and `CurveSet`.
// ---------------------------------------------------------------------------

/// Unified market data snapshot for a single valuation date.
///
/// Placeholder — the real implementation will live in
/// `pricer_models::market_env::MarketEnvironment`.
#[derive(Debug, Clone)]
pub struct MarketEnvironment {
    /// Valuation date.
    valuation_date: Date,
    /// Underlying market data provider.
    provider: MarketProvider,
    /// FX spot rates keyed by (base, quote).
    fx_spots: Vec<(Currency, Currency, f64)>,
    /// Flat volatility overrides keyed by a simple string key.
    flat_vols: Vec<(String, f64)>,
}

impl MarketEnvironment {
    /// Creates a new market environment.
    pub fn new(valuation_date: Date, provider: MarketProvider) -> Self {
        Self {
            valuation_date,
            provider,
            fx_spots: Vec::new(),
            flat_vols: Vec::new(),
        }
    }

    /// Builder-style: adds an FX spot rate.
    pub fn with_fx_spot(mut self, base: Currency, quote: Currency, rate: f64) -> Self {
        self.fx_spots.push((base, quote, rate));
        self
    }

    /// Builder-style: adds a flat volatility override.
    pub fn with_flat_vol(mut self, key: impl Into<String>, vol: f64) -> Self {
        self.flat_vols.push((key.into(), vol));
        self
    }

    /// Returns the valuation date.
    pub fn valuation_date(&self) -> Date { self.valuation_date }

    /// Returns the discount curve for a given currency.
    pub fn discount_curve(
        &self,
        currency: Currency,
    ) -> Result<&pricer_models::market::CurveEnum<f64>, PricingError> {
        self.provider.get_curve(currency).ok_or_else(|| {
            PricingError::missing_market_data(format!("No discount curve for {:?}", currency))
        })
    }

    /// Returns the curve set for forward rate computations.
    pub fn curve_set(&self) -> &CurveSet<f64> { self.provider.curve_set() }

    /// Returns the FX spot rate between two currencies.
    pub fn fx_rate(&self, from: Currency, to: Currency) -> Result<f64, PricingError> {
        if from == to {
            return Ok(1.0);
        }
        // Direct lookup
        for &(base, quote, rate) in &self.fx_spots {
            if base == from && quote == to {
                return Ok(rate);
            }
            if base == to && quote == from {
                return Ok(1.0 / rate);
            }
        }
        Err(PricingError::fx_rate_not_found(from, to))
    }

    /// Returns the FX spot rate for a given currency pair.
    pub fn fx_spot(&self, base: Currency, quote: Currency) -> Result<f64, PricingError> {
        self.fx_rate(base, quote)
    }

    /// Returns a flat volatility by key.
    pub fn vol_surface(&self, key: &str) -> Result<f64, PricingError> {
        for (k, v) in &self.flat_vols {
            if k == key {
                return Ok(*v);
            }
        }
        Err(PricingError::missing_market_data(format!(
            "No volatility surface for key '{}'",
            key
        )))
    }
}

// ---------------------------------------------------------------------------
// Resolved method — internal enum after Auto-resolution.
// ---------------------------------------------------------------------------

/// Resolved pricing method after auto-resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedMethod {
    /// Cashflow discounting or closed-form analytical formula.
    Analytical,
    /// Monte Carlo simulation.
    MonteCarlo,
    /// Tree-based pricing.
    Tree,
    /// Script engine (exotic payoffs).
    Script,
}

// ---------------------------------------------------------------------------
// Pricer — the unified entry point.
// ---------------------------------------------------------------------------

/// Stateless unified pricer.
///
/// All state is passed through function arguments; no mutable fields.
pub struct Pricer;

impl Pricer {
    /// Prices a trade and returns a full leg/cashflow breakdown.
    ///
    /// This is the **primary entry point** for all PV computation.  The method
    /// dispatches to the appropriate path based on the trade type and
    /// calculation settings.
    pub fn price(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<PricingResult, PricingError> {
        let resolved = Self::resolve_method(trade, calc);

        match resolved {
            ResolvedMethod::Analytical => {
                if trade.trade_type.is_option() {
                    // Option products: closed-form pricing wrapped into
                    // PricingResult for API uniformity.
                    let unified = Self::price_option_analytical(trade, market, calc)?;
                    Ok(PricingResult::new(
                        unified.pv,
                        Vec::new(),
                        calc.reporting_currency,
                    ))
                } else {
                    // Linear products: cashflow discounting.
                    Self::price_by_cashflow_discounting(trade, market, calc)
                }
            }
            ResolvedMethod::MonteCarlo => Err(PricingError::unsupported_method(
                "MonteCarlo",
                "Unified Monte Carlo path not yet wired — use MonteCarloPricer directly",
            )),
            ResolvedMethod::Tree => Err(PricingError::unsupported_method(
                "Tree",
                "Unified Tree path not yet wired — use BinomialTree directly",
            )),
            ResolvedMethod::Script => Err(PricingError::unsupported_method(
                "Script",
                "Script engine for exotic payoffs is not yet integrated",
            )),
        }
    }

    /// Prices a trade and returns a [`UnifiedPricingResult`] (PV + optional
    /// Greeks + metadata).
    ///
    /// Prefer this when the caller only needs PV and Greeks without the full
    /// cashflow breakdown.
    pub fn price_unified(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let start = Instant::now();
        let resolved = Self::resolve_method(trade, calc);

        match resolved {
            ResolvedMethod::Analytical => {
                if trade.trade_type.is_option() {
                    Self::price_option_analytical(trade, market, calc)
                } else {
                    let result = Self::price_by_cashflow_discounting(trade, market, calc)?;
                    let elapsed = start.elapsed().as_nanos() as u64;
                    Ok(UnifiedPricingResult::new(
                        result.total_pv,
                        PricingMethod::Analytical,
                        elapsed,
                    )
                    .with_metadata(PricingMetadata::Discount {
                        model: "CashflowDiscounting".to_string(),
                    }))
                }
            }
            ResolvedMethod::MonteCarlo => Err(PricingError::unsupported_method(
                "MonteCarlo",
                "Unified Monte Carlo path not yet wired",
            )),
            ResolvedMethod::Tree => Err(PricingError::unsupported_method(
                "Tree",
                "Unified Tree path not yet wired",
            )),
            ResolvedMethod::Script => Err(PricingError::unsupported_method(
                "Script",
                "Script engine for exotic payoffs is not yet integrated",
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Method resolution
    // -----------------------------------------------------------------------

    /// Resolves the effective pricing method for a given trade and calculation
    /// settings.
    fn resolve_method(trade: &Trade, calc: &CalcSetting) -> ResolvedMethod {
        match calc.method {
            PricingMethodHint::Analytical => ResolvedMethod::Analytical,
            PricingMethodHint::MonteCarlo => ResolvedMethod::MonteCarlo,
            PricingMethodHint::Tree => ResolvedMethod::Tree,
            PricingMethodHint::Auto => {
                if trade.trade_type.is_option() {
                    match &trade.trade_type {
                        // Barrier options and other exotics → Script engine.
                        TradeType::FxBarrierOption { .. } => ResolvedMethod::Script,
                        // Vanilla options → Analytical (closed-form).
                        _ => ResolvedMethod::Analytical,
                    }
                } else {
                    // Linear products: cashflow discounting.
                    ResolvedMethod::Analytical
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Cashflow discounting (linear products)
    // -----------------------------------------------------------------------

    /// Prices a trade by discounting its future cashflows.
    ///
    /// Iterates over every [`Leg`], projects each future cashflow using
    /// [`PayoffEvaluator`], discounts to valuation date, and converts to the
    /// reporting currency.
    fn price_by_cashflow_discounting(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<PricingResult, PricingError> {
        let valuation_date = market.valuation_date();
        let curve_set = market.curve_set();
        let mut leg_results = Vec::with_capacity(trade.num_legs());

        for leg in trade.legs() {
            let leg_result = Self::price_leg(
                leg,
                valuation_date,
                calc.reporting_currency,
                market,
                curve_set,
            )?;
            leg_results.push(leg_result);
        }

        let total_pv: f64 = leg_results.iter().map(|l| l.pv).sum();

        Ok(PricingResult::new(
            total_pv,
            leg_results,
            calc.reporting_currency,
        ))
    }

    /// Prices a single leg by iterating over its future cashflows.
    fn price_leg(
        leg: &Leg,
        valuation_date: Date,
        reporting_currency: Currency,
        market: &MarketEnvironment,
        curve_set: &CurveSet<f64>,
    ) -> Result<LegPricingResult, PricingError> {
        let leg_currency = leg.currency;

        let curve = market.discount_curve(leg_currency)?;

        let fx_rate = if leg_currency == reporting_currency {
            1.0
        } else {
            market.fx_rate(leg_currency, reporting_currency)?
        };

        let evaluator = PayoffEvaluator::new(curve_set);

        let mut cashflow_results = Vec::with_capacity(leg.len());
        let mut pv_original = 0.0;

        for cf in leg.future_cashflows(valuation_date) {
            // Time to payment in years (ACT/365 simple).
            let payment_days = cf.payment_date.into_inner().num_days_from_ce();
            let valuation_days = valuation_date.into_inner().num_days_from_ce();
            let t = (payment_days - valuation_days) as f64 / 365.0;

            let df = curve
                .discount_factor(t)
                .map_err(|e| PricingError::market_data_resolution(format!("{:?}", e)))?;

            // Accrual times for the payoff evaluator.
            let start_days = cf.accrual_start.into_inner().num_days_from_ce();
            let end_days = cf.accrual_end.into_inner().num_days_from_ce();
            let start_time = (start_days - valuation_days) as f64 / 365.0;
            let end_time = (end_days - valuation_days) as f64 / 365.0;

            let amount = evaluator.evaluate(
                &cf.payoff,
                cf.notional,
                cf.year_fraction,
                start_time,
                end_time,
            )?;

            let cf_pv_original = amount * df;
            let cf_pv = cf_pv_original * fx_rate;

            pv_original += cf_pv_original;

            cashflow_results.push(CashflowPricingResult::new(
                cf_pv,
                cf_pv_original,
                cf.payment_date,
                df,
                leg_currency,
            ));
        }

        let direction = leg.direction;
        let pv = pv_original * fx_rate * direction.sign();
        let pv_original_signed = pv_original * direction.sign();

        Ok(LegPricingResult::new(
            pv,
            pv_original_signed,
            leg_currency,
            fx_rate,
            direction,
            cashflow_results,
        ))
    }

    // -----------------------------------------------------------------------
    // Analytical option pricing
    // -----------------------------------------------------------------------

    /// Prices a vanilla option using a closed-form analytical formula.
    ///
    /// Dispatches to:
    /// - Garman-Kohlhagen for FX options
    /// - Black-Scholes for equity and commodity options
    fn price_option_analytical(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let start = Instant::now();

        match &trade.trade_type {
            // ----- FX vanilla option -----
            TradeType::FxOption {
                option_type,
                strike,
                expiry_date,
                ..
            } => Self::price_fx_option(
                market,
                calc,
                trade,
                option_type,
                *strike,
                *expiry_date,
                start,
            ),

            // ----- Equity vanilla option -----
            TradeType::EquityOption {
                underlyer,
                option_type,
                strike,
                expiry_date,
                contract_multiplier,
                ..
            } => Self::price_equity_option(
                market,
                calc,
                underlyer,
                option_type,
                *strike,
                *expiry_date,
                *contract_multiplier,
                start,
            ),

            // ----- Commodity vanilla option -----
            TradeType::CommodityOption {
                commodity,
                option_type,
                strike,
                expiry_date,
                quantity,
                ..
            } => Self::price_commodity_option(
                market,
                calc,
                commodity,
                option_type,
                *strike,
                *expiry_date,
                *quantity,
                start,
            ),

            other => Err(PricingError::unsupported_instrument(format!(
                "Analytical option pricing not supported for {:?}",
                other
            ))),
        }
    }

    // -----------------------------------------------------------------------
    // FX option (Garman-Kohlhagen)
    // -----------------------------------------------------------------------

    /// Prices an FX option using the Garman-Kohlhagen model.
    fn price_fx_option(
        market: &MarketEnvironment,
        calc: &CalcSetting,
        trade: &Trade,
        option_type: &OptionType,
        strike: f64,
        expiry_date: Date,
        start: Instant,
    ) -> Result<UnifiedPricingResult, PricingError> {
        // Determine the currency pair from the trade legs.
        let (base_ccy, quote_ccy) = Self::extract_fx_currencies(trade)?;

        // Market data extraction.
        let spot = market.fx_spot(base_ccy, quote_ccy)?;

        let domestic_curve = market.discount_curve(quote_ccy)?;
        let foreign_curve = market.discount_curve(base_ccy)?;

        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;

        let rd = Self::zero_rate_from_curve(domestic_curve, t)?;
        let rf = Self::zero_rate_from_curve(foreign_curve, t)?;

        let vol_key = format!("FX:{}/{}", base_ccy.code(), quote_ccy.code());
        let vol = market.vol_surface(&vol_key)?;

        let params = GarmanKohlhagenParams::new(spot, strike, rd, rf, vol, t).map_err(|e| {
            PricingError::InvalidInput {
                reason: format!("Garman-Kohlhagen parameter error: {:?}", e),
            }
        })?;

        let model = GarmanKohlhagen::new(params);
        let is_call = option_type.is_call();
        let pv = model.price(is_call);

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::Analytical, elapsed)
            .with_metadata(PricingMetadata::Discount {
                model: "Garman-Kohlhagen".to_string(),
            });

        if calc.compute_greeks {
            let greeks = UnifiedGreeks::new(
                Some(model.delta(is_call)),
                Some(model.gamma()),
                Some(model.vega()),
                Some(model.theta(is_call)),
                Some(model.rho_domestic(is_call)),
            );
            result = result.with_greeks(greeks);
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Equity option (Black-Scholes)
    // -----------------------------------------------------------------------

    /// Prices an equity option using the Black-Scholes model.
    fn price_equity_option(
        market: &MarketEnvironment,
        calc: &CalcSetting,
        underlyer: &str,
        option_type: &OptionType,
        strike: f64,
        expiry_date: Date,
        contract_multiplier: f64,
        start: Instant,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let vol_key = format!("EQ:{}", underlyer);
        let vol = market.vol_surface(&vol_key)?;

        let spot_key = format!("EQ_SPOT:{}", underlyer);
        // Attempt to get the equity spot from the flat_vols store (as a
        // lightweight lookup).
        let spot = market.vol_surface(&spot_key).map_err(|_| {
            PricingError::missing_market_data(format!("No equity spot price for '{}'", underlyer))
        })?;

        let domestic_curve = market.discount_curve(calc.reporting_currency)?;
        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;
        let r = Self::zero_rate_from_curve(domestic_curve, t)?;

        let bs = BlackScholes::new(spot, r, vol).map_err(|e| PricingError::InvalidInput {
            reason: format!("Black-Scholes parameter error: {:?}", e),
        })?;

        let is_call = option_type.is_call();
        let unit_pv = bs.price(strike, t, is_call);
        let pv = unit_pv * contract_multiplier;

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::Analytical, elapsed)
            .with_metadata(PricingMetadata::Discount {
                model: "Black-Scholes".to_string(),
            });

        if calc.compute_greeks {
            let greeks = UnifiedGreeks::new(
                Some(bs.delta(strike, t, is_call) * contract_multiplier),
                Some(bs.gamma(strike, t) * contract_multiplier),
                Some(bs.vega(strike, t) * contract_multiplier),
                Some(bs.theta(strike, t, is_call) * contract_multiplier),
                Some(bs.rho(strike, t, is_call) * contract_multiplier),
            );
            result = result.with_greeks(greeks);
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Commodity option (Black-Scholes)
    // -----------------------------------------------------------------------

    /// Prices a commodity option using the Black-Scholes model.
    fn price_commodity_option(
        market: &MarketEnvironment,
        calc: &CalcSetting,
        commodity: &str,
        option_type: &OptionType,
        strike: f64,
        expiry_date: Date,
        quantity: f64,
        start: Instant,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let vol_key = format!("CMDTY:{}", commodity);
        let vol = market.vol_surface(&vol_key)?;

        let spot_key = format!("CMDTY_SPOT:{}", commodity);
        let spot = market.vol_surface(&spot_key).map_err(|_| {
            PricingError::missing_market_data(format!(
                "No commodity spot price for '{}'",
                commodity
            ))
        })?;

        let domestic_curve = market.discount_curve(calc.reporting_currency)?;
        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;
        let r = Self::zero_rate_from_curve(domestic_curve, t)?;

        let bs = BlackScholes::new(spot, r, vol).map_err(|e| PricingError::InvalidInput {
            reason: format!("Black-Scholes parameter error: {:?}", e),
        })?;

        let is_call = option_type.is_call();
        let unit_pv = bs.price(strike, t, is_call);
        let pv = unit_pv * quantity;

        let elapsed = start.elapsed().as_nanos() as u64;

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::Analytical, elapsed)
            .with_metadata(PricingMetadata::Discount {
                model: "Black-Scholes-Commodity".to_string(),
            });

        if calc.compute_greeks {
            let greeks = UnifiedGreeks::new(
                Some(bs.delta(strike, t, is_call) * quantity),
                Some(bs.gamma(strike, t) * quantity),
                Some(bs.vega(strike, t) * quantity),
                Some(bs.theta(strike, t, is_call) * quantity),
                Some(bs.rho(strike, t, is_call) * quantity),
            );
            result = result.with_greeks(greeks);
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Computes year fraction (ACT/365 simple) between two dates.
    fn year_fraction(start: Date, end: Date) -> Result<f64, PricingError> {
        let start_days = start.into_inner().num_days_from_ce();
        let end_days = end.into_inner().num_days_from_ce();
        let days = end_days - start_days;
        if days <= 0 {
            return Err(PricingError::InvalidInput {
                reason: format!(
                    "Expiry date ({:?}) must be after valuation date ({:?})",
                    end, start
                ),
            });
        }
        Ok(days as f64 / 365.0)
    }

    /// Extracts the zero rate from a yield curve at time `t`.
    fn zero_rate_from_curve(
        curve: &pricer_models::market::CurveEnum<f64>,
        t: f64,
    ) -> Result<f64, PricingError> {
        curve
            .zero_rate(t)
            .map_err(|e| PricingError::market_data_resolution(format!("{:?}", e)))
    }

    /// Extracts the base and quote currencies from an FX trade's legs.
    ///
    /// Convention: the first leg's currency is the *base* (foreign) currency
    /// and the second leg's currency is the *quote* (domestic) currency.  If
    /// the trade has fewer than two legs we return an error.
    fn extract_fx_currencies(trade: &Trade) -> Result<(Currency, Currency), PricingError> {
        let mut legs = trade.legs();
        let first = legs.next().ok_or_else(|| {
            PricingError::invalid_trade("FX option trade must have at least one leg")
        })?;
        let second = legs.next();

        match second {
            Some(second_leg) => Ok((first.currency, second_leg.currency)),
            None => Err(PricingError::invalid_trade(
                "FX option trade must have at least two legs (base + quote currencies)",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use infra_domain::{
        market::{instrument::ExerciseStyle, Currency},
        time::Date,
        trade::{
            CashflowType, Direction, Leg, LegType, OptionType, Payoff, SettlementType, Trade,
            TradeType,
        },
    };
    use pricer_models::market::MarketProvider;

    use super::*;
    use crate::calc_setting::{CalcSetting, PricingMethodHint};

    // -- Helpers --

    fn make_valuation_date() -> Date { Date::from_ymd(2025, 1, 1).unwrap() }

    fn make_fx_market_env() -> MarketEnvironment {
        let provider = MarketProvider::new();
        let val_date = make_valuation_date();

        MarketEnvironment::new(val_date, provider)
            .with_fx_spot(Currency::EUR, Currency::USD, 1.10)
            .with_flat_vol("FX:EUR/USD", 0.15)
    }

    fn make_fx_option_trade() -> Trade {
        let expiry = Date::from_ymd(2026, 1, 1).unwrap();
        let leg_eur = Leg::new(
            vec![infra_domain::trade::Cashflow::new(
                CashflowType::Settlement,
                expiry,
                make_valuation_date(),
                expiry,
                1.0,
                1_000_000.0,
                Payoff::fixed(1.0),
                Currency::EUR,
            )],
            Direction::Receiver,
            LegType::Generic,
            Currency::EUR,
        );
        let leg_usd = Leg::new(
            vec![infra_domain::trade::Cashflow::new(
                CashflowType::Settlement,
                expiry,
                make_valuation_date(),
                expiry,
                1.0,
                1_100_000.0,
                Payoff::fixed(1.0),
                Currency::USD,
            )],
            Direction::Payer,
            LegType::Generic,
            Currency::USD,
        );

        Trade::new(
            "FX-OPT-001",
            vec![leg_eur, leg_usd],
            TradeType::FxOption {
                option_type: OptionType::Call,
                strike: 1.12,
                exercise_type: ExerciseStyle::European,
                settlement_type: SettlementType::Cash,
                expiry_date: expiry,
            },
        )
    }

    // -- Method resolution tests --

    #[test]
    fn test_resolve_method_auto_linear() {
        let trade = Trade::new("T1", vec![], TradeType::Swap);
        let calc = CalcSetting::default();

        let resolved = Pricer::resolve_method(&trade, &calc);
        assert_eq!(resolved, ResolvedMethod::Analytical);
    }

    #[test]
    fn test_resolve_method_auto_fx_option() {
        let expiry = Date::from_ymd(2026, 1, 1).unwrap();
        let trade = Trade::new(
            "T2",
            vec![],
            TradeType::FxOption {
                option_type: OptionType::Call,
                strike: 1.12,
                exercise_type: ExerciseStyle::European,
                settlement_type: SettlementType::Cash,
                expiry_date: expiry,
            },
        );
        let calc = CalcSetting::default();

        let resolved = Pricer::resolve_method(&trade, &calc);
        assert_eq!(resolved, ResolvedMethod::Analytical);
    }

    #[test]
    fn test_resolve_method_auto_barrier() {
        let expiry = Date::from_ymd(2026, 1, 1).unwrap();
        let trade = Trade::new(
            "T3",
            vec![],
            TradeType::FxBarrierOption {
                option_type: OptionType::Call,
                strike: 1.12,
                barrier: 1.20,
                barrier_type: infra_domain::market::instrument::BarrierType::KnockOut,
                barrier_direction: infra_domain::market::instrument::BarrierDirection::Up,
                exercise_type: ExerciseStyle::European,
                expiry_date: expiry,
            },
        );
        let calc = CalcSetting::default();

        let resolved = Pricer::resolve_method(&trade, &calc);
        assert_eq!(resolved, ResolvedMethod::Script);
    }

    #[test]
    fn test_resolve_method_explicit_mc() {
        let trade = Trade::new("T4", vec![], TradeType::Swap);
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .build();

        let resolved = Pricer::resolve_method(&trade, &calc);
        assert_eq!(resolved, ResolvedMethod::MonteCarlo);
    }

    #[test]
    fn test_resolve_method_explicit_tree() {
        let trade = Trade::new("T5", vec![], TradeType::Swap);
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .build();

        let resolved = Pricer::resolve_method(&trade, &calc);
        assert_eq!(resolved, ResolvedMethod::Tree);
    }

    // -- MarketEnvironment tests --

    #[test]
    fn test_market_env_fx_rate_direct() {
        let env = make_fx_market_env();
        let rate = env.fx_rate(Currency::EUR, Currency::USD).unwrap();
        assert!((rate - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_market_env_fx_rate_inverse() {
        let env = make_fx_market_env();
        let rate = env.fx_rate(Currency::USD, Currency::EUR).unwrap();
        assert!((rate - 1.0 / 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_market_env_fx_rate_same_currency() {
        let env = make_fx_market_env();
        let rate = env.fx_rate(Currency::USD, Currency::USD).unwrap();
        assert!((rate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_market_env_fx_rate_missing() {
        let env = make_fx_market_env();
        let result = env.fx_rate(Currency::GBP, Currency::JPY);
        assert!(result.is_err());
    }

    #[test]
    fn test_market_env_vol_surface() {
        let env = make_fx_market_env();
        let vol = env.vol_surface("FX:EUR/USD").unwrap();
        assert!((vol - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_market_env_vol_surface_missing() {
        let env = make_fx_market_env();
        let result = env.vol_surface("FX:GBP/USD");
        assert!(result.is_err());
    }

    // -- Year fraction helper --

    #[test]
    fn test_year_fraction_one_year() {
        let start = Date::from_ymd(2025, 1, 1).unwrap();
        let end = Date::from_ymd(2026, 1, 1).unwrap();
        let yf = Pricer::year_fraction(start, end).unwrap();
        assert!((yf - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_year_fraction_negative() {
        let start = Date::from_ymd(2026, 1, 1).unwrap();
        let end = Date::from_ymd(2025, 1, 1).unwrap();
        let result = Pricer::year_fraction(start, end);
        assert!(result.is_err());
    }

    // -- MC / Tree / Script error paths --

    #[test]
    fn test_mc_returns_unsupported() {
        let trade = Trade::new("T6", vec![], TradeType::Swap);
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .build();

        let result = Pricer::price(&trade, &env, &calc);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_method_error());
    }

    #[test]
    fn test_tree_returns_unsupported() {
        let trade = Trade::new("T7", vec![], TradeType::Swap);
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .build();

        let result = Pricer::price(&trade, &env, &calc);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_method_error());
    }

    #[test]
    fn test_script_returns_unsupported() {
        let expiry = Date::from_ymd(2026, 1, 1).unwrap();
        let trade = Trade::new(
            "T8",
            vec![],
            TradeType::FxBarrierOption {
                option_type: OptionType::Call,
                strike: 1.12,
                barrier: 1.20,
                barrier_type: infra_domain::market::instrument::BarrierType::KnockOut,
                barrier_direction: infra_domain::market::instrument::BarrierDirection::Up,
                exercise_type: ExerciseStyle::European,
                expiry_date: expiry,
            },
        );
        let env = make_fx_market_env();
        let calc = CalcSetting::default();

        let result = Pricer::price(&trade, &env, &calc);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_method_error());
    }

    // -- Extract FX currencies --

    #[test]
    fn test_extract_fx_currencies() {
        let trade = make_fx_option_trade();
        let (base, quote) = Pricer::extract_fx_currencies(&trade).unwrap();
        assert_eq!(base, Currency::EUR);
        assert_eq!(quote, Currency::USD);
    }

    #[test]
    fn test_extract_fx_currencies_no_legs() {
        let trade = Trade::new("T9", vec![], TradeType::FxForward);
        let result = Pricer::extract_fx_currencies(&trade);
        assert!(result.is_err());
    }
}

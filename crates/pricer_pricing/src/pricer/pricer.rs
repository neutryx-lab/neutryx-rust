//! Unified Pricer — sole entry point for all PV computation.
//!
//! The [`Pricer`] dispatches to the correct pricing path based on [`TradeType`]
//! and [`CalcSetting`]:
//!
//! - **Linear products** (Swap, FxForward, Bond, Deposit, …) → cashflow
//!   discounting.
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
    market::{instrument::ExerciseStyle, Currency},
    time::Date,
    trade::{Leg, OptionType, Trade, TradeType},
};
use pricer_core::math::formulas::{
    black_scholes::BlackScholes,
    garman_kohlhagen::{GarmanKohlhagen, GarmanKohlhagenParams},
};
use pricer_models::{
    compiler::{ExoticCompiler, IndexMapper, ScriptProduct},
    market::{curves::YieldCurve, CurveEnum, CurveSet, MarketDataError, MarketEnvironment},
};

use super::{
    CalcSetting, CashflowPricingResult, LegPricingResult, PayoffEvaluator, PricingError,
    PricingMethodHint, PricingResult,
};
use crate::{
    kernel::{FlatSpotProvider, ScriptEngine},
    methods::{
        mc::{GbmParams, Greek, MonteCarloConfig, MonteCarloPricer, PayoffParams},
        tree::{TreeConfig, TreeMethod, TreeType},
    },
    result::{PricingMetadata, TreeTypeMetadata, UnifiedGreeks, UnifiedPricingResult},
};

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
// Option parameters — extracted from Trade + MarketEnvironment.
// ---------------------------------------------------------------------------

/// Extracted option parameters for MC/Tree pricing.
struct OptionParams {
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    t: f64,
    is_call: bool,
    is_american: bool,
    multiplier: f64,
    /// Foreign (base-currency) rate for FX options (Garman-Kohlhagen drift).
    foreign_rate: Option<f64>,
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
            ResolvedMethod::MonteCarlo => {
                Self::price_mc(trade, market, calc)
            }
            ResolvedMethod::Tree => {
                Self::price_tree(trade, market, calc)
            }
            ResolvedMethod::Script => Err(PricingError::unsupported_method(
                "Script",
                "Trade→Script conversion not available; use Pricer::price_script_flat() for ScriptProduct pricing",
            )),
        }
    }

    /// Convenience wrapper returning only the total PV scalar.
    ///
    /// Equivalent to `Pricer::price(trade, market, calc).map(|r| r.total_pv)`
    /// but reads better at call sites that discard the full breakdown (e.g.
    /// bump-and-revalue Greeks loops).
    pub fn price_pv(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<f64, PricingError> {
        Self::price(trade, market, calc).map(|r| r.total_pv)
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
            ResolvedMethod::MonteCarlo => {
                Self::price_mc_unified(trade, market, calc, start)
            }
            ResolvedMethod::Tree => {
                Self::price_tree_unified(trade, market, calc, start)
            }
            ResolvedMethod::Script => Err(PricingError::unsupported_method(
                "Script",
                "Trade→Script conversion not available; use Pricer::price_script_flat() for ScriptProduct pricing",
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Script-based exotic pricing
    // -----------------------------------------------------------------------

    /// Prices a pre-built [`ScriptProduct`] using flat market assumptions.
    ///
    /// This is a convenience entry point for exotic products (TARF,
    /// Autocallable, etc.) that provide explicit spot, discount, and foreign
    /// rate parameters rather than a full [`MarketEnvironment`].
    ///
    /// Internally compiles the product into a [`ScriptKernel`] via
    /// [`ExoticCompiler`] and evaluates it with [`ScriptEngine`].
    pub fn price_script_flat(
        product: &ScriptProduct,
        spot: f64,
        discount_rate: f64,
        foreign_rate: f64,
    ) -> Result<f64, PricingError> {
        let mut compiler = ExoticCompiler::new(IndexMapper::new());
        let kernel = compiler
            .compile_script_product(product)
            .map_err(|e| PricingError::Internal(format!("Script compilation failed: {e}")))?;

        let provider = FlatSpotProvider::new(discount_rate, foreign_rate, spot);
        Ok(ScriptEngine::price(&kernel, &provider))
    }

    // -----------------------------------------------------------------------
    // Monte Carlo pricing
    // -----------------------------------------------------------------------

    /// Builds a [`MonteCarloPricer`] and [`GbmParams`] from trade/market data.
    fn build_mc_context(
        params: &OptionParams,
        calc: &CalcSetting,
    ) -> Result<(MonteCarloPricer, GbmParams, PayoffParams, f64), PricingError> {
        let mc_setting = calc.mc_config.clone().unwrap_or_default();

        let mut builder = MonteCarloConfig::builder()
            .n_paths(mc_setting.num_paths)
            .n_steps(mc_setting.num_steps);
        if let Some(seed) = mc_setting.seed {
            builder = builder.seed(seed);
        }
        let mc_config = builder.build().map_err(|e| PricingError::InvalidInput {
            reason: e.to_string(),
        })?;

        let mc = MonteCarloPricer::new(mc_config).map_err(|e| PricingError::InvalidInput {
            reason: e.to_string(),
        })?;

        let drift_rate = params
            .foreign_rate
            .map(|rf| params.rate - rf)
            .unwrap_or(params.rate);
        let gbm = GbmParams::new(params.spot, drift_rate, params.vol, params.t);
        let payoff = if params.is_call {
            PayoffParams::call(params.strike)
        } else {
            PayoffParams::put(params.strike)
        };
        let df = (-params.rate * params.t).exp();

        Ok((mc, gbm, payoff, df))
    }

    /// Prices an option via Monte Carlo, returning a [`PricingResult`].
    fn price_mc(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<PricingResult, PricingError> {
        let params = Self::extract_option_params(trade, market, calc)?;
        let (mut mc, gbm, payoff, df) = Self::build_mc_context(&params, calc)?;
        let mc_result = mc.price_european(gbm, payoff, df);

        let pv = mc_result.price * params.multiplier;
        let std_error = mc_result.std_error * params.multiplier;
        let num_paths = calc
            .mc_config
            .as_ref()
            .map(|c| c.num_paths)
            .unwrap_or(10_000);

        let dist = super::PathDistribution::new(pv, std_error, vec![], num_paths);

        Ok(PricingResult::with_path_distribution(
            pv,
            Vec::new(),
            calc.reporting_currency,
            dist,
        ))
    }

    /// Prices an option via Monte Carlo, returning a [`UnifiedPricingResult`].
    fn price_mc_unified(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
        start: Instant,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let params = Self::extract_option_params(trade, market, calc)?;
        let (mut mc, gbm, payoff, df) = Self::build_mc_context(&params, calc)?;
        let num_paths = mc.config().n_paths();

        let mc_result = if calc.compute_greeks {
            mc.price_with_greeks(
                gbm,
                payoff,
                df,
                &[
                    Greek::Delta,
                    Greek::Gamma,
                    Greek::Vega,
                    Greek::Theta,
                    Greek::Rho,
                ],
            )
        } else {
            mc.price_european(gbm, payoff, df)
        };

        let pv = mc_result.price * params.multiplier;
        let std_error = mc_result.std_error * params.multiplier;
        let elapsed = start.elapsed().as_nanos() as u64;

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::MonteCarlo, elapsed)
            .with_metadata(PricingMetadata::MonteCarlo {
                num_paths,
                standard_error: std_error,
            });

        if calc.compute_greeks {
            let greeks = UnifiedGreeks::new(
                mc_result.delta.map(|d| d * params.multiplier),
                mc_result.gamma.map(|g| g * params.multiplier),
                mc_result.vega.map(|v| v * params.multiplier),
                mc_result.theta.map(|t| t * params.multiplier),
                mc_result.rho.map(|r| r * params.multiplier),
            );
            result = result.with_greeks(greeks);
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Tree pricing
    // -----------------------------------------------------------------------

    /// Builds a [`TreeMethod`] from calculation settings.
    fn build_tree_method(calc: &CalcSetting) -> Result<TreeMethod, PricingError> {
        let tree_setting = calc.tree_config.clone().unwrap_or_default();
        let tree_config = TreeConfig::builder()
            .num_steps(tree_setting.num_steps)
            .tree_type(tree_setting.tree_type)
            .compute_greeks(calc.compute_greeks)
            .build()
            .map_err(|e| PricingError::InvalidInput {
                reason: e.to_string(),
            })?;
        Ok(TreeMethod::new(tree_config))
    }

    /// Prices an option via a tree method, returning a [`PricingResult`].
    fn price_tree(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<PricingResult, PricingError> {
        let params = Self::extract_option_params(trade, market, calc)?;
        let method = Self::build_tree_method(calc)?;

        let drift_rate = params
            .foreign_rate
            .map(|rf| params.rate - rf)
            .unwrap_or(params.rate);

        let tree_result = method.price(
            params.spot,
            params.strike,
            params.t,
            drift_rate,
            params.vol,
            params.is_call,
            params.is_american,
        )?;

        let pv = tree_result.pv * params.multiplier;
        Ok(PricingResult::new(pv, Vec::new(), calc.reporting_currency))
    }

    /// Prices an option via a tree method, returning a
    /// [`UnifiedPricingResult`].
    fn price_tree_unified(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
        start: Instant,
    ) -> Result<UnifiedPricingResult, PricingError> {
        let params = Self::extract_option_params(trade, market, calc)?;
        let method = Self::build_tree_method(calc)?;

        let drift_rate = params
            .foreign_rate
            .map(|rf| params.rate - rf)
            .unwrap_or(params.rate);

        let tree_result = method.price(
            params.spot,
            params.strike,
            params.t,
            drift_rate,
            params.vol,
            params.is_call,
            params.is_american,
        )?;

        let pv = tree_result.pv * params.multiplier;
        let elapsed = start.elapsed().as_nanos() as u64;
        let tree_type_meta = match method.config().tree_type {
            TreeType::Binomial => TreeTypeMetadata::Binomial,
            TreeType::Trinomial => TreeTypeMetadata::Trinomial,
        };

        let mut result = UnifiedPricingResult::new(pv, PricingMethod::Tree, elapsed).with_metadata(
            PricingMetadata::Tree {
                num_steps: method.config().num_steps,
                tree_type: tree_type_meta,
            },
        );

        if calc.compute_greeks {
            if let Some(tg) = tree_result.greeks {
                let greeks = UnifiedGreeks::from_delta_gamma(
                    tg.delta.unwrap_or(0.0) * params.multiplier,
                    tg.gamma.unwrap_or(0.0) * params.multiplier,
                );
                result = result.with_greeks(greeks);
            }
        }

        Ok(result)
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

    /// Builds a [`CurveSet`] from the forward curves in the market environment.
    ///
    /// This bridges the gap between [`MarketEnvironment`] and the existing
    /// [`PayoffEvaluator`] which expects a [`CurveSet`] reference.
    fn build_curve_set(market: &MarketEnvironment) -> CurveSet<f64> {
        let mut cs = CurveSet::new();
        for (name, curve) in market.forward_curves() {
            cs.insert(*name, curve.clone());
        }
        cs
    }

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
        let curve_set = Self::build_curve_set(market);
        let mut leg_results = Vec::with_capacity(trade.num_legs());

        for leg in trade.legs() {
            let leg_result = Self::price_leg(
                leg,
                valuation_date,
                calc.reporting_currency,
                market,
                &curve_set,
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

        let curve = market.discount_curve(&leg_currency).ok_or_else(|| {
            PricingError::missing_market_data(format!("No discount curve for {:?}", leg_currency))
        })?;

        let fx_rate = if leg_currency == reporting_currency {
            1.0
        } else {
            market
                .fx_rate(leg_currency, reporting_currency)
                .ok_or_else(|| PricingError::fx_rate_not_found(leg_currency, reporting_currency))?
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
        let spot = market
            .fx_rate(base_ccy, quote_ccy)
            .ok_or_else(|| PricingError::fx_rate_not_found(base_ccy, quote_ccy))?;

        let domestic_curve = market.discount_curve(&quote_ccy).ok_or_else(|| {
            PricingError::missing_market_data(format!(
                "No discount curve for domestic currency {:?}",
                quote_ccy
            ))
        })?;
        let foreign_curve = market.discount_curve(&base_ccy).ok_or_else(|| {
            PricingError::missing_market_data(format!(
                "No discount curve for foreign currency {:?}",
                base_ccy
            ))
        })?;

        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;

        let rd = Self::zero_rate_from_curve(domestic_curve, t)?;
        let rf = Self::zero_rate_from_curve(foreign_curve, t)?;

        // Compute forward for vol surface lookup, then retrieve implied vol.
        let forward = spot * ((rd - rf) * t).exp();
        let vol_key = format!("FX:{}/{}", base_ccy.code(), quote_ccy.code());
        let vol = market
            .implied_vol(&vol_key, strike, t, forward)
            .map_err(Self::map_market_error)?;

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
        let spot = market.spot_price(underlyer).ok_or_else(|| {
            PricingError::missing_market_data(format!("No equity spot price for '{}'", underlyer))
        })?;

        let domestic_curve = market
            .discount_curve(&calc.reporting_currency)
            .ok_or_else(|| {
                PricingError::missing_market_data(format!(
                    "No discount curve for {:?}",
                    calc.reporting_currency
                ))
            })?;

        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;
        let r = Self::zero_rate_from_curve(domestic_curve, t)?;

        // Compute forward for vol surface lookup.
        let forward = spot * (r * t).exp();
        let vol_key = format!("EQ:{}", underlyer);
        let vol = market
            .implied_vol(&vol_key, strike, t, forward)
            .map_err(Self::map_market_error)?;

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
        let spot = market.spot_price(commodity).ok_or_else(|| {
            PricingError::missing_market_data(format!(
                "No commodity spot price for '{}'",
                commodity
            ))
        })?;

        let domestic_curve = market
            .discount_curve(&calc.reporting_currency)
            .ok_or_else(|| {
                PricingError::missing_market_data(format!(
                    "No discount curve for {:?}",
                    calc.reporting_currency
                ))
            })?;

        let valuation_date = market.valuation_date();
        let t = Self::year_fraction(valuation_date, expiry_date)?;
        let r = Self::zero_rate_from_curve(domestic_curve, t)?;

        // Compute forward for vol surface lookup.
        let forward = spot * (r * t).exp();
        let vol_key = format!("CMDTY:{}", commodity);
        let vol = market
            .implied_vol(&vol_key, strike, t, forward)
            .map_err(Self::map_market_error)?;

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

    /// Converts a [`MarketDataError`] into a [`PricingError`].
    fn map_market_error(e: MarketDataError) -> PricingError {
        PricingError::market_data_resolution(e.to_string())
    }

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
    fn zero_rate_from_curve(curve: &CurveEnum<f64>, t: f64) -> Result<f64, PricingError> {
        curve
            .zero_rate(t)
            .map_err(|e| PricingError::market_data_resolution(format!("{:?}", e)))
    }

    /// Extracts vanilla option parameters from a trade and market environment.
    ///
    /// Supports FX, Equity, and Commodity vanilla options.  Returns an error
    /// for non-option trade types or unsupported option types.
    fn extract_option_params(
        trade: &Trade,
        market: &MarketEnvironment,
        calc: &CalcSetting,
    ) -> Result<OptionParams, PricingError> {
        let valuation_date = market.valuation_date();

        match &trade.trade_type {
            TradeType::FxOption {
                option_type,
                strike,
                exercise_type,
                expiry_date,
                ..
            } => {
                let (base_ccy, quote_ccy) = Self::extract_fx_currencies(trade)?;
                let spot = market
                    .fx_rate(base_ccy, quote_ccy)
                    .ok_or_else(|| PricingError::fx_rate_not_found(base_ccy, quote_ccy))?;

                let domestic_curve = market.discount_curve(&quote_ccy).ok_or_else(|| {
                    PricingError::missing_market_data(format!(
                        "No discount curve for domestic currency {:?}",
                        quote_ccy
                    ))
                })?;
                let foreign_curve = market.discount_curve(&base_ccy).ok_or_else(|| {
                    PricingError::missing_market_data(format!(
                        "No discount curve for foreign currency {:?}",
                        base_ccy
                    ))
                })?;

                let t = Self::year_fraction(valuation_date, *expiry_date)?;
                let rd = Self::zero_rate_from_curve(domestic_curve, t)?;
                let rf = Self::zero_rate_from_curve(foreign_curve, t)?;

                let forward = spot * ((rd - rf) * t).exp();
                let vol_key = format!("FX:{}/{}", base_ccy.code(), quote_ccy.code());
                let vol = market
                    .implied_vol(&vol_key, *strike, t, forward)
                    .map_err(Self::map_market_error)?;

                Ok(OptionParams {
                    spot,
                    strike: *strike,
                    rate: rd,
                    vol,
                    t,
                    is_call: option_type.is_call(),
                    is_american: *exercise_type == ExerciseStyle::American,
                    multiplier: 1.0,
                    foreign_rate: Some(rf),
                })
            }

            TradeType::EquityOption {
                underlyer,
                option_type,
                strike,
                exercise_type,
                expiry_date,
                contract_multiplier,
                ..
            } => {
                let spot = market.spot_price(underlyer).ok_or_else(|| {
                    PricingError::missing_market_data(format!(
                        "No equity spot price for '{}'",
                        underlyer
                    ))
                })?;

                let domestic_curve =
                    market
                        .discount_curve(&calc.reporting_currency)
                        .ok_or_else(|| {
                            PricingError::missing_market_data(format!(
                                "No discount curve for {:?}",
                                calc.reporting_currency
                            ))
                        })?;

                let t = Self::year_fraction(valuation_date, *expiry_date)?;
                let r = Self::zero_rate_from_curve(domestic_curve, t)?;

                let forward = spot * (r * t).exp();
                let vol_key = format!("EQ:{}", underlyer);
                let vol = market
                    .implied_vol(&vol_key, *strike, t, forward)
                    .map_err(Self::map_market_error)?;

                Ok(OptionParams {
                    spot,
                    strike: *strike,
                    rate: r,
                    vol,
                    t,
                    is_call: option_type.is_call(),
                    is_american: *exercise_type == ExerciseStyle::American,
                    multiplier: *contract_multiplier,
                    foreign_rate: None,
                })
            }

            TradeType::CommodityOption {
                commodity,
                option_type,
                strike,
                exercise_type,
                expiry_date,
                quantity,
                ..
            } => {
                let spot = market.spot_price(commodity).ok_or_else(|| {
                    PricingError::missing_market_data(format!(
                        "No commodity spot price for '{}'",
                        commodity
                    ))
                })?;

                let domestic_curve =
                    market
                        .discount_curve(&calc.reporting_currency)
                        .ok_or_else(|| {
                            PricingError::missing_market_data(format!(
                                "No discount curve for {:?}",
                                calc.reporting_currency
                            ))
                        })?;

                let t = Self::year_fraction(valuation_date, *expiry_date)?;
                let r = Self::zero_rate_from_curve(domestic_curve, t)?;

                let forward = spot * (r * t).exp();
                let vol_key = format!("CMDTY:{}", commodity);
                let vol = market
                    .implied_vol(&vol_key, *strike, t, forward)
                    .map_err(Self::map_market_error)?;

                Ok(OptionParams {
                    spot,
                    strike: *strike,
                    rate: r,
                    vol,
                    t,
                    is_call: option_type.is_call(),
                    is_american: *exercise_type == ExerciseStyle::American,
                    multiplier: *quantity,
                    foreign_rate: None,
                })
            }

            other => Err(PricingError::unsupported_instrument(format!(
                "MC/Tree pricing not supported for {:?}",
                other
            ))),
        }
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
        market::{instrument::ExerciseStyle, Currency, CurrencyPair},
        time::Date,
        trade::{
            CashflowType, Direction, Leg, LegType, OptionType, Payoff, SettlementType, Trade,
            TradeType,
        },
    };
    use pricer_models::market::{CurveEnum, MarketEnvironmentBuilder, VolSurfaceEnum};

    use super::{
        super::{MonteCarloSetting, TreeSetting},
        *,
    };

    // -- Helpers --

    fn make_valuation_date() -> Date { Date::from_ymd(2025, 1, 1).unwrap() }

    fn make_fx_market_env() -> MarketEnvironment {
        let val_date = make_valuation_date();
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let vol_surface = VolSurfaceEnum::<f64>::flat(0.15).unwrap();

        MarketEnvironmentBuilder::new(val_date)
            .with_discount_curve(Currency::USD, CurveEnum::flat(0.05))
            .with_discount_curve(Currency::EUR, CurveEnum::flat(0.03))
            .with_fx_spot(pair, 1.10)
            .with_vol_surface("FX:EUR/USD", vol_surface)
            .build()
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

    // -- MarketEnvironment integration tests --

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
        assert!(env.fx_rate(Currency::GBP, Currency::JPY).is_none());
    }

    #[test]
    fn test_market_env_vol_surface() {
        let env = make_fx_market_env();
        let vol = env.implied_vol("FX:EUR/USD", 1.12, 1.0, 1.12).unwrap();
        assert!((vol - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_market_env_vol_surface_missing() {
        let env = make_fx_market_env();
        let result = env.implied_vol("FX:GBP/USD", 1.30, 1.0, 1.30);
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

    // -- MC / Tree / Script error & success paths --

    #[test]
    fn test_mc_non_option_returns_unsupported() {
        let trade = Trade::new("T6", vec![], TradeType::Swap);
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .build();

        let result = Pricer::price(&trade, &env, &calc);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_instrument_error());
    }

    #[test]
    fn test_tree_non_option_returns_unsupported() {
        let trade = Trade::new("T7", vec![], TradeType::Swap);
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .build();

        let result = Pricer::price(&trade, &env, &calc);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_instrument_error());
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

    // -- Monte Carlo dispatch --

    #[test]
    fn test_mc_fx_option_price() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .mc_config(MonteCarloSetting {
                num_paths: 50_000,
                num_steps: 100,
                seed: Some(42),
            })
            .build();

        let result = Pricer::price(&trade, &env, &calc).unwrap();
        assert!(result.total_pv > 0.0, "MC FX call PV should be positive");
        assert!(
            result.path_distribution.is_some(),
            "MC should include path distribution"
        );
        let dist = result.path_distribution.unwrap();
        assert_eq!(dist.path_count, 50_000);
    }

    #[test]
    fn test_mc_fx_option_reproducible() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .mc_config(MonteCarloSetting {
                num_paths: 10_000,
                num_steps: 50,
                seed: Some(123),
            })
            .build();

        let pv1 = Pricer::price_pv(&trade, &env, &calc).unwrap();
        let pv2 = Pricer::price_pv(&trade, &env, &calc).unwrap();
        assert!(
            (pv1 - pv2).abs() < 1e-10,
            "Same seed should give identical PVs"
        );
    }

    #[test]
    fn test_mc_unified_with_greeks() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::MonteCarlo)
            .compute_greeks(true)
            .mc_config(MonteCarloSetting {
                num_paths: 20_000,
                num_steps: 50,
                seed: Some(42),
            })
            .build();

        let result = Pricer::price_unified(&trade, &env, &calc).unwrap();
        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::MonteCarlo);
        assert!(result.has_greeks());
        assert!(result.standard_error().is_some());
        assert!(result.num_paths() == Some(20_000));
    }

    // -- Tree dispatch --

    #[test]
    fn test_tree_fx_option_price() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .tree_config(TreeSetting {
                num_steps: 200,
                tree_type: TreeType::Binomial,
            })
            .build();

        let result = Pricer::price(&trade, &env, &calc).unwrap();
        assert!(result.total_pv > 0.0, "Tree FX call PV should be positive");
    }

    #[test]
    fn test_tree_trinomial_fx_option() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .tree_config(TreeSetting {
                num_steps: 200,
                tree_type: TreeType::Trinomial,
            })
            .build();

        let result = Pricer::price(&trade, &env, &calc).unwrap();
        assert!(result.total_pv > 0.0);
    }

    #[test]
    fn test_tree_unified_with_greeks() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .compute_greeks(true)
            .tree_config(TreeSetting {
                num_steps: 200,
                tree_type: TreeType::Binomial,
            })
            .build();

        let result = Pricer::price_unified(&trade, &env, &calc).unwrap();
        assert!(result.pv > 0.0);
        assert_eq!(result.method, PricingMethod::Tree);
        assert!(result.has_greeks());
        assert!(result.num_steps() == Some(200));
    }

    #[test]
    fn test_tree_close_to_analytical() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();

        // Analytical price.
        let analytical_calc = CalcSetting::builder()
            .method(PricingMethodHint::Analytical)
            .build();
        let analytical = Pricer::price_unified(&trade, &env, &analytical_calc)
            .unwrap()
            .pv;

        // Tree price (high step count for convergence).
        let tree_calc = CalcSetting::builder()
            .method(PricingMethodHint::Tree)
            .tree_config(TreeSetting {
                num_steps: 500,
                tree_type: TreeType::Binomial,
            })
            .build();
        let tree = Pricer::price(&trade, &env, &tree_calc).unwrap().total_pv;

        let rel_err = (tree - analytical).abs() / analytical.abs();
        assert!(
            rel_err < 0.05,
            "Tree should converge to analytical: tree={tree}, analytical={analytical}, rel_err={rel_err}"
        );
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

    // -- End-to-end FX option pricing --

    #[test]
    fn test_fx_option_pricing_e2e() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::builder().compute_greeks(true).build();

        let result = Pricer::price_unified(&trade, &env, &calc).unwrap();
        assert!(result.pv > 0.0, "FX call option PV should be positive");
        assert!(result.greeks.is_some(), "Greeks should be computed");

        let greeks = result.greeks.unwrap();
        assert!(greeks.delta.is_some());
        assert!(greeks.gamma.is_some());
        assert!(greeks.vega.is_some());
    }

    #[test]
    fn test_fx_option_pricing_no_greeks() {
        let trade = make_fx_option_trade();
        let env = make_fx_market_env();
        let calc = CalcSetting::default();

        let result = Pricer::price_unified(&trade, &env, &calc).unwrap();
        assert!(result.pv > 0.0);
        assert!(result.greeks.is_none());
    }
}

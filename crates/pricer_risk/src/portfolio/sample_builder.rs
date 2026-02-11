//! Sample Portfolio Builder for demonstration and testing.

use infra_domain::{
    market::Currency,
    trade::{
        ExerciseStyle, Forward, ForwardDirection, InstrumentParams, PayoffType, PricingInstrument,
        VanillaOption,
    },
};

use super::{
    error::PortfolioError, Counterparty, CounterpartyId, CreditParams, NettingSet, NettingSetId,
    Portfolio, PortfolioBuilder, Trade, TradeId,
};

/// Configuration for asset class distribution in a sample portfolio.
#[derive(Debug, Clone, Copy)]
pub struct AssetMix {
    /// Proportion of equity trades (0.0 - 1.0)
    pub equity: f64,
    /// Proportion of rates trades (0.0 - 1.0)
    pub rates: f64,
    /// Proportion of FX trades (0.0 - 1.0)
    pub fx: f64,
}

impl AssetMix {
    /// Creates a new asset mix with validation, returning error if proportions
    /// are invalid.
    pub fn new(equity: f64, rates: f64, fx: f64) -> Result<Self, PortfolioError> {
        if equity < 0.0 || rates < 0.0 || fx < 0.0 {
            return Err(PortfolioError::BuilderError(
                "Asset mix proportions must be non-negative".to_string(),
            ));
        }

        let sum = equity + rates + fx;
        if (sum - 1.0).abs() > 1e-6 {
            return Err(PortfolioError::BuilderError(format!(
                "Asset mix proportions must sum to 1.0, got {:.4}",
                sum
            )));
        }

        Ok(Self { equity, rates, fx })
    }

    /// Returns the default balanced asset mix (1/3 each).
    pub fn balanced() -> Self {
        Self {
            equity: 1.0 / 3.0,
            rates: 1.0 / 3.0,
            fx: 1.0 / 3.0,
        }
    }

    /// Returns an equity-heavy asset mix (60% equity, 25% rates, 15% FX).
    pub fn equity_heavy() -> Self {
        Self {
            equity: 0.6,
            rates: 0.25,
            fx: 0.15,
        }
    }

    /// Returns a rates-heavy asset mix (20% equity, 60% rates, 20% FX).
    pub fn rates_heavy() -> Self {
        Self {
            equity: 0.2,
            rates: 0.6,
            fx: 0.2,
        }
    }
}

impl Default for AssetMix {
    fn default() -> Self { Self::balanced() }
}

/// Builder for creating sample portfolios with configurable parameters
/// demonstrating shared market data and graph optimisation.
#[derive(Debug)]
pub struct SamplePortfolioBuilder {
    trade_count: usize,
    asset_mix: AssetMix,
    base_notional: f64,
    epsilon: f64,
}

impl SamplePortfolioBuilder {
    /// Creates a new SamplePortfolioBuilder with default settings (10 trades,
    /// balanced mix, 1M notional, 1e-6 epsilon).
    pub fn new() -> Self {
        Self {
            trade_count: 10,
            asset_mix: AssetMix::balanced(),
            base_notional: 1_000_000.0,
            epsilon: 1e-6,
        }
    }

    /// Sets the target trade count.
    pub fn with_trade_count(mut self, count: usize) -> Self {
        self.trade_count = count;
        self
    }

    /// Sets the asset mix using proportions (must sum to 1.0, validated on
    /// build).
    pub fn with_asset_mix(mut self, equity: f64, rates: f64, fx: f64) -> Self {
        self.asset_mix = AssetMix { equity, rates, fx };
        self
    }

    /// Sets the asset mix using a preset configuration.
    pub fn with_asset_mix_preset(mut self, mix: AssetMix) -> Self {
        self.asset_mix = mix;
        self
    }

    /// Sets the base notional amount.
    pub fn with_base_notional(mut self, notional: f64) -> Self {
        self.base_notional = notional;
        self
    }

    /// Sets the smoothing epsilon for AD compatibility.
    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon;
        self
    }

    /// Builds the sample portfolio, validating trade count > 0 and asset mix
    /// proportions.
    pub fn build(self) -> Result<Portfolio, PortfolioError> {
        if self.trade_count == 0 {
            return Err(PortfolioError::BuilderError(
                "Trade count must be greater than 0".to_string(),
            ));
        }

        let asset_mix = AssetMix::new(
            self.asset_mix.equity,
            self.asset_mix.rates,
            self.asset_mix.fx,
        )?;

        let equity_count = (self.trade_count as f64 * asset_mix.equity).round() as usize;
        let rates_count = (self.trade_count as f64 * asset_mix.rates).round() as usize;
        let fx_count = self.trade_count.saturating_sub(equity_count + rates_count);

        let (equity_count, rates_count, fx_count) = if self.trade_count >= 3 {
            ensure_minimum_diversity(equity_count, rates_count, fx_count, self.trade_count)
        } else {
            (equity_count, rates_count, fx_count)
        };

        let credit = CreditParams::new(0.02, 0.4).map_err(|_| {
            PortfolioError::InvalidCreditParams("Failed to create credit params".to_string())
        })?;
        let counterparty = Counterparty::new(CounterpartyId::new("CP_SAMPLE"), credit)
            .with_name("Sample Counterparty");

        let mut netting_set = NettingSet::new(
            NettingSetId::new("NS_SAMPLE"),
            CounterpartyId::new("CP_SAMPLE"),
        );

        let mut trades = Vec::with_capacity(self.trade_count);

        let _equity_underlyings = ["AAPL", "MSFT", "GOOGL", "AMZN"];
        let equity_strikes = [95.0, 100.0, 105.0, 110.0];

        for i in 0..equity_count {
            let trade_id = TradeId::new(&format!("EQ_{:04}", i + 1));
            let strike_idx = i % equity_strikes.len();
            let payoff_type = if i % 2 == 0 {
                PayoffType::Call
            } else {
                PayoffType::Put
            };

            let expiry = match i % 4 {
                0 => 0.25,
                1 => 0.5,
                2 => 1.0,
                _ => 2.0,
            };

            let params = InstrumentParams::new(
                equity_strikes[strike_idx],
                expiry,
                self.base_notional * (1.0 + (i as f64 * 0.1) % 1.0),
            )
            .map_err(|e| PortfolioError::BuilderError(format!("Invalid params: {}", e)))?;

            let option =
                VanillaOption::new(params, payoff_type, ExerciseStyle::European, self.epsilon);
            let instrument = PricingInstrument::Vanilla(option);

            let trade = Trade::new(
                trade_id.clone(),
                instrument,
                Currency::USD,
                CounterpartyId::new("CP_SAMPLE"),
                NettingSetId::new("NS_SAMPLE"),
                self.base_notional * (1.0 + (i as f64 * 0.1) % 1.0),
            );

            netting_set.add_trade(trade_id);
            trades.push(trade);
        }

        let rates_maturities = [1.0, 2.0, 5.0, 10.0];
        let rates_currencies = [Currency::USD, Currency::EUR, Currency::JPY];

        for i in 0..rates_count {
            let trade_id = TradeId::new(&format!("IR_{:04}", i + 1));
            let maturity_idx = i % rates_maturities.len();
            let currency_idx = i % rates_currencies.len();
            let direction = if i % 2 == 0 {
                ForwardDirection::Long
            } else {
                ForwardDirection::Short
            };

            let forward = Forward::new(
                100.0,
                rates_maturities[maturity_idx],
                self.base_notional * (1.0 + (i as f64 * 0.05) % 1.0),
                direction,
            )
            .map_err(|e| PortfolioError::BuilderError(format!("Invalid forward: {}", e)))?;

            let instrument = PricingInstrument::Forward(forward);

            let trade = Trade::new(
                trade_id.clone(),
                instrument,
                rates_currencies[currency_idx],
                CounterpartyId::new("CP_SAMPLE"),
                NettingSetId::new("NS_SAMPLE"),
                self.base_notional * (1.0 + (i as f64 * 0.05) % 1.0),
            );

            netting_set.add_trade(trade_id);
            trades.push(trade);
        }

        let fx_pairs = [
            (Currency::EUR, Currency::USD),
            (Currency::USD, Currency::JPY),
            (Currency::GBP, Currency::USD),
        ];
        let fx_strikes = [1.0, 1.05, 1.10, 0.95];

        for i in 0..fx_count {
            let trade_id = TradeId::new(&format!("FX_{:04}", i + 1));
            let pair_idx = i % fx_pairs.len();
            let strike_idx = i % fx_strikes.len();
            let payoff_type = if i % 2 == 0 {
                PayoffType::Call
            } else {
                PayoffType::Put
            };

            let expiry = match i % 3 {
                0 => 0.25,
                1 => 0.5,
                _ => 1.0,
            };

            let strike = 100.0 * fx_strikes[strike_idx];

            let params = InstrumentParams::new(
                strike,
                expiry,
                self.base_notional * (1.0 + (i as f64 * 0.15) % 1.0),
            )
            .map_err(|e| PortfolioError::BuilderError(format!("Invalid FX params: {}", e)))?;

            let option =
                VanillaOption::new(params, payoff_type, ExerciseStyle::European, self.epsilon);
            let instrument = PricingInstrument::Vanilla(option);

            let (_base, quote) = fx_pairs[pair_idx];

            let trade = Trade::new(
                trade_id.clone(),
                instrument,
                quote,
                CounterpartyId::new("CP_SAMPLE"),
                NettingSetId::new("NS_SAMPLE"),
                self.base_notional * (1.0 + (i as f64 * 0.15) % 1.0),
            );

            netting_set.add_trade(trade_id);
            trades.push(trade);
        }

        PortfolioBuilder::new()
            .add_counterparty(counterparty)
            .add_netting_set(netting_set)
            .add_trades(trades)
            .build()
    }

    /// Returns the configured trade count.
    pub fn trade_count(&self) -> usize { self.trade_count }

    /// Returns the configured asset mix.
    pub fn asset_mix(&self) -> &AssetMix { &self.asset_mix }
}

impl Default for SamplePortfolioBuilder {
    fn default() -> Self { Self::new() }
}

fn ensure_minimum_diversity(
    mut equity: usize,
    mut rates: usize,
    mut fx: usize,
    total: usize,
) -> (usize, usize, usize) {
    if equity == 0 && total >= 3 {
        equity = 1;
        if rates > 1 {
            rates -= 1;
        } else if fx > 1 {
            fx -= 1;
        }
    }
    if rates == 0 && total >= 3 {
        rates = 1;
        if equity > 1 {
            equity -= 1;
        } else if fx > 1 {
            fx -= 1;
        }
    }
    if fx == 0 && total >= 3 {
        fx = 1;
        if equity > 1 {
            equity -= 1;
        } else if rates > 1 {
            rates -= 1;
        }
    }

    let current_total = equity + rates + fx;
    if current_total != total {
        let diff = total as i64 - current_total as i64;
        if diff > 0 {
            if equity >= rates && equity >= fx {
                equity += diff as usize;
            } else if rates >= fx {
                rates += diff as usize;
            } else {
                fx += diff as usize;
            }
        } else if equity > 1 {
            equity = equity.saturating_sub((-diff) as usize);
        } else if rates > 1 {
            rates = rates.saturating_sub((-diff) as usize);
        } else {
            fx = fx.saturating_sub((-diff) as usize);
        }
    }

    (equity, rates, fx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default_values() {
        let builder = SamplePortfolioBuilder::new();

        assert_eq!(builder.trade_count(), 10);
        assert!((builder.asset_mix().equity - 1.0 / 3.0).abs() < 1e-6);
        assert!((builder.asset_mix().rates - 1.0 / 3.0).abs() < 1e-6);
        assert!((builder.asset_mix().fx - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_builder_with_trade_count() {
        let builder = SamplePortfolioBuilder::new().with_trade_count(50);
        assert_eq!(builder.trade_count(), 50);
    }

    #[test]
    fn test_builder_with_asset_mix() {
        let builder = SamplePortfolioBuilder::new().with_asset_mix(0.5, 0.3, 0.2);

        assert!((builder.asset_mix().equity - 0.5).abs() < 1e-6);
        assert!((builder.asset_mix().rates - 0.3).abs() < 1e-6);
        assert!((builder.asset_mix().fx - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_builder_chained_configuration() {
        let builder = SamplePortfolioBuilder::new()
            .with_trade_count(100)
            .with_asset_mix(0.4, 0.4, 0.2)
            .with_base_notional(5_000_000.0)
            .with_epsilon(1e-8);

        assert_eq!(builder.trade_count(), 100);
        assert!((builder.asset_mix().equity - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_build_generates_correct_trade_count() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(20)
            .build()
            .unwrap();

        assert_eq!(portfolio.trade_count(), 20);
    }

    #[test]
    fn test_build_contains_multiple_instrument_types() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(30)
            .with_asset_mix(0.34, 0.33, 0.33)
            .build()
            .unwrap();

        let mut vanilla_count = 0;
        let mut forward_count = 0;

        for trade in portfolio.trades() {
            if trade.is_vanilla() {
                vanilla_count += 1;
            } else if trade.is_forward() {
                forward_count += 1;
            }
        }

        assert!(vanilla_count > 0, "Should have vanilla options");
        assert!(forward_count > 0, "Should have forwards");
    }

    #[test]
    fn test_build_guarantees_minimum_3_instrument_types() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(30)
            .build()
            .unwrap();

        let mut equity_count = 0;
        let mut rates_count = 0;
        let mut fx_count = 0;

        for trade in portfolio.trades() {
            let id = trade.id().as_str();
            if id.starts_with("EQ_") {
                equity_count += 1;
            } else if id.starts_with("IR_") {
                rates_count += 1;
            } else if id.starts_with("FX_") {
                fx_count += 1;
            }
        }

        assert!(equity_count >= 1, "Should have at least 1 equity trade");
        assert!(rates_count >= 1, "Should have at least 1 rates trade");
        assert!(fx_count >= 1, "Should have at least 1 FX trade");
    }

    #[test]
    fn test_shared_currencies_in_portfolio() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(50)
            .build()
            .unwrap();

        let mut usd_count = 0;

        for trade in portfolio.trades() {
            if trade.currency() == Currency::USD {
                usd_count += 1;
            }
        }

        assert!(
            usd_count > 1,
            "Multiple trades should share USD currency for graph optimisation"
        );
    }

    #[test]
    fn test_shared_expiries_in_equity_options() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(20)
            .with_asset_mix(1.0, 0.0, 0.0)
            .build()
            .unwrap();

        let expiries: Vec<f64> = portfolio.trades().map(|t| t.expiry()).collect();

        let unique_expiries: std::collections::HashSet<_> =
            expiries.iter().map(|&e| (e * 100.0) as i64).collect();

        assert!(
            unique_expiries.len() < expiries.len(),
            "Should have repeated expiries for shared maturity dates"
        );
    }

    #[test]
    fn test_build_error_zero_trade_count() {
        let result = SamplePortfolioBuilder::new().with_trade_count(0).build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PortfolioError::BuilderError(_)),
            "Should return BuilderError for zero trade count"
        );
    }

    #[test]
    fn test_build_error_invalid_asset_mix_negative() {
        let result = SamplePortfolioBuilder::new()
            .with_asset_mix(-0.1, 0.6, 0.5)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_build_error_invalid_asset_mix_sum() {
        let result = SamplePortfolioBuilder::new()
            .with_asset_mix(0.5, 0.5, 0.5)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_asset_mix_new_validation() {
        let valid = AssetMix::new(0.4, 0.3, 0.3);
        assert!(valid.is_ok());

        let invalid_neg = AssetMix::new(-0.1, 0.6, 0.5);
        assert!(invalid_neg.is_err());

        let invalid_sum = AssetMix::new(0.4, 0.4, 0.4);
        assert!(invalid_sum.is_err());
    }

    #[test]
    fn test_asset_mix_presets() {
        let balanced = AssetMix::balanced();
        assert!((balanced.equity + balanced.rates + balanced.fx - 1.0).abs() < 1e-6);

        let equity_heavy = AssetMix::equity_heavy();
        assert!(equity_heavy.equity > equity_heavy.rates);
        assert!(equity_heavy.equity > equity_heavy.fx);

        let rates_heavy = AssetMix::rates_heavy();
        assert!(rates_heavy.rates > rates_heavy.equity);
    }

    #[test]
    fn test_large_portfolio_generation() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(100)
            .build()
            .unwrap();

        assert_eq!(portfolio.trade_count(), 100);
        assert_eq!(portfolio.counterparty_count(), 1);
        assert_eq!(portfolio.netting_set_count(), 1);
    }

    #[test]
    fn test_portfolio_notionals_vary() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(10)
            .build()
            .unwrap();

        let notionals: Vec<f64> = portfolio.trades().map(|t| t.notional()).collect();
        let unique_notionals: std::collections::HashSet<_> =
            notionals.iter().map(|&n| (n * 100.0) as i64).collect();

        assert!(unique_notionals.len() > 1, "Notionals should vary");
    }

    #[test]
    fn test_minimum_diversity_with_small_count() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(3)
            .build()
            .unwrap();

        let mut has_eq = false;
        let mut has_ir = false;
        let mut has_fx = false;

        for trade in portfolio.trades() {
            let id = trade.id().as_str();
            if id.starts_with("EQ_") {
                has_eq = true;
            } else if id.starts_with("IR_") {
                has_ir = true;
            } else if id.starts_with("FX_") {
                has_fx = true;
            }
        }

        assert!(has_eq && has_ir && has_fx, "Should have all 3 asset types");
    }

    #[test]
    fn test_asset_mix_skewed_equity() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(100)
            .with_asset_mix(0.8, 0.1, 0.1)
            .build()
            .unwrap();

        let mut equity_count = 0;
        for trade in portfolio.trades() {
            if trade.id().as_str().starts_with("EQ_") {
                equity_count += 1;
            }
        }

        assert!(
            equity_count >= 75,
            "Should have ~80% equity, got {}",
            equity_count
        );
    }

    #[test]
    fn test_trades_belong_to_netting_set() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(10)
            .build()
            .unwrap();

        let ns_id = NettingSetId::new("NS_SAMPLE");
        let trades_in_ns = portfolio.trades_in_netting_set(&ns_id);

        assert_eq!(
            trades_in_ns.len(),
            10,
            "All trades should be in the netting set"
        );
    }
}

//! Bloomberg-style snapshot market data simulation.
//!
//! Simulates Bloomberg terminal style data with comprehensive
//! market data snapshots including yield curves and volatility surfaces.

use adapter_feeds::MarketQuote;
use chrono::Utc;
use pricer_core::types::Currency;
use rand::Rng;
use std::collections::HashMap;

/// Bloomberg-style market data provider
pub struct BloombergSim {
    /// Equity quotes
    equity_quotes: HashMap<String, QuoteData>,
    /// FX rates
    fx_rates: HashMap<String, f64>,
    /// Yield curve tenors and rates
    yield_curves: HashMap<Currency, Vec<(f64, f64)>>,
    /// Volatility surfaces (strike, expiry, vol)
    vol_surfaces: HashMap<String, Vec<VolPoint>>,
}

/// Internal quote data
#[derive(Clone)]
struct QuoteData {
    bid: f64,
    ask: f64,
    last: f64,
    currency: Currency,
}

/// Volatility surface point
#[derive(Clone)]
pub struct VolPoint {
    /// Strike as percentage of spot (e.g., 100 = ATM)
    pub strike_pct: f64,
    /// Time to expiry in years
    pub expiry: f64,
    /// Implied volatility
    pub vol: f64,
}

impl BloombergSim {
    /// Create a new Bloomberg simulator with default data
    pub fn new() -> Self {
        let mut sim = Self {
            equity_quotes: HashMap::new(),
            fx_rates: HashMap::new(),
            yield_curves: HashMap::new(),
            vol_surfaces: HashMap::new(),
        };
        sim.initialize_default_data();
        sim
    }

    /// Initialize default market data
    fn initialize_default_data(&mut self) {
        // Equity quotes
        self.equity_quotes.insert(
            "AAPL".to_string(),
            QuoteData {
                bid: 184.95,
                ask: 185.05,
                last: 185.00,
                currency: Currency::USD,
            },
        );
        self.equity_quotes.insert(
            "GOOGL".to_string(),
            QuoteData {
                bid: 139.90,
                ask: 140.10,
                last: 140.00,
                currency: Currency::USD,
            },
        );
        self.equity_quotes.insert(
            "MSFT".to_string(),
            QuoteData {
                bid: 379.80,
                ask: 380.20,
                last: 380.00,
                currency: Currency::USD,
            },
        );

        // FX rates
        self.fx_rates.insert("USDJPY".to_string(), 150.25);
        self.fx_rates.insert("EURUSD".to_string(), 1.0850);
        self.fx_rates.insert("GBPUSD".to_string(), 1.2650);
        self.fx_rates.insert("USDCHF".to_string(), 0.8820);

        // USD yield curve (tenor in years, rate in %)
        self.yield_curves.insert(
            Currency::USD,
            vec![
                (0.25, 5.35),
                (0.5, 5.30),
                (1.0, 5.10),
                (2.0, 4.65),
                (3.0, 4.35),
                (5.0, 4.15),
                (7.0, 4.10),
                (10.0, 4.05),
                (30.0, 4.25),
            ],
        );

        // EUR yield curve
        self.yield_curves.insert(
            Currency::EUR,
            vec![
                (0.25, 3.90),
                (0.5, 3.75),
                (1.0, 3.50),
                (2.0, 3.10),
                (3.0, 2.85),
                (5.0, 2.65),
                (7.0, 2.60),
                (10.0, 2.55),
                (30.0, 2.75),
            ],
        );

        // JPY yield curve
        self.yield_curves.insert(
            Currency::JPY,
            vec![
                (0.25, -0.05),
                (0.5, 0.00),
                (1.0, 0.05),
                (2.0, 0.15),
                (3.0, 0.25),
                (5.0, 0.45),
                (7.0, 0.65),
                (10.0, 0.85),
                (30.0, 1.65),
            ],
        );

        // AAPL volatility surface
        self.vol_surfaces.insert(
            "AAPL".to_string(),
            vec![
                // 1M expiry
                VolPoint { strike_pct: 80.0, expiry: 1.0 / 12.0, vol: 0.35 },
                VolPoint { strike_pct: 90.0, expiry: 1.0 / 12.0, vol: 0.28 },
                VolPoint { strike_pct: 100.0, expiry: 1.0 / 12.0, vol: 0.25 },
                VolPoint { strike_pct: 110.0, expiry: 1.0 / 12.0, vol: 0.27 },
                VolPoint { strike_pct: 120.0, expiry: 1.0 / 12.0, vol: 0.32 },
                // 3M expiry
                VolPoint { strike_pct: 80.0, expiry: 0.25, vol: 0.32 },
                VolPoint { strike_pct: 90.0, expiry: 0.25, vol: 0.26 },
                VolPoint { strike_pct: 100.0, expiry: 0.25, vol: 0.24 },
                VolPoint { strike_pct: 110.0, expiry: 0.25, vol: 0.25 },
                VolPoint { strike_pct: 120.0, expiry: 0.25, vol: 0.29 },
                // 1Y expiry
                VolPoint { strike_pct: 80.0, expiry: 1.0, vol: 0.30 },
                VolPoint { strike_pct: 90.0, expiry: 1.0, vol: 0.25 },
                VolPoint { strike_pct: 100.0, expiry: 1.0, vol: 0.23 },
                VolPoint { strike_pct: 110.0, expiry: 1.0, vol: 0.24 },
                VolPoint { strike_pct: 120.0, expiry: 1.0, vol: 0.27 },
            ],
        );
    }

    /// Get equity quotes
    pub fn get_equity_quotes(&self) -> Vec<MarketQuote> {
        let timestamp = Utc::now().timestamp_millis();
        self.equity_quotes
            .iter()
            .map(|(ticker, data)| {
                MarketQuote::new(ticker, data.bid, data.ask)
                    .with_currency(data.currency)
                    .with_timestamp(timestamp)
            })
            .collect()
    }

    /// Get FX rates
    pub fn get_fx_rates(&self) -> HashMap<String, f64> {
        self.fx_rates.clone()
    }

    /// Get yield curve for a currency
    pub fn get_yield_curve(&self, currency: Currency) -> Option<Vec<(f64, f64)>> {
        self.yield_curves.get(&currency).cloned()
    }

    /// Get volatility surface for an instrument
    pub fn get_vol_surface(&self, ticker: &str) -> Option<Vec<VolPoint>> {
        self.vol_surfaces.get(ticker).cloned()
    }

    /// Get all available yield curves
    pub fn get_all_yield_curves(&self) -> HashMap<Currency, Vec<(f64, f64)>> {
        self.yield_curves.clone()
    }

    /// Refresh data with random perturbations
    pub fn refresh(&mut self) {
        let mut rng = rand::thread_rng();

        // Perturb equity quotes
        for data in self.equity_quotes.values_mut() {
            let noise: f64 = rng.gen_range(-0.002..0.002);
            let mid = (data.bid + data.ask) / 2.0 * (1.0 + noise);
            let spread = (data.ask - data.bid) / 2.0;
            data.bid = mid - spread;
            data.ask = mid + spread;
            data.last = mid;
        }

        // Perturb FX rates
        for rate in self.fx_rates.values_mut() {
            let noise: f64 = rng.gen_range(-0.0005..0.0005);
            *rate *= 1.0 + noise;
        }

        // Perturb yield curves
        for curve in self.yield_curves.values_mut() {
            for (_, rate) in curve.iter_mut() {
                let noise: f64 = rng.gen_range(-0.02..0.02); // 2bp noise
                *rate += noise;
            }
        }
    }
}

impl Default for BloombergSim {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloomberg_sim_equity() {
        let sim = BloombergSim::new();
        let quotes = sim.get_equity_quotes();
        assert!(!quotes.is_empty());
    }

    #[test]
    fn test_bloomberg_sim_yield_curve() {
        let sim = BloombergSim::new();
        let curve = sim.get_yield_curve(Currency::USD);
        assert!(curve.is_some());
        assert_eq!(curve.unwrap().len(), 9);
    }
}

//! Synthetic market data generator.
//!
//! Generates synthetic market data with configurable parameters
//! for testing and demonstration purposes.

use adapter_feeds::MarketQuote;
use chrono::Utc;
use pricer_core::types::Currency;
use rand::Rng;
use rand_distr::{Distribution, Normal};

/// Synthetic data generator
pub struct SyntheticGenerator {
    /// Number of instruments to generate
    num_instruments: usize,
    /// Base price range
    price_range: (f64, f64),
    /// Volatility range
    vol_range: (f64, f64),
}

impl SyntheticGenerator {
    /// Create a new synthetic generator
    pub fn new(num_instruments: usize) -> Self {
        Self {
            num_instruments,
            price_range: (10.0, 1000.0),
            vol_range: (0.15, 0.50),
        }
    }

    /// Set price range
    pub fn with_price_range(mut self, min: f64, max: f64) -> Self {
        self.price_range = (min, max);
        self
    }

    /// Set volatility range
    pub fn with_vol_range(mut self, min: f64, max: f64) -> Self {
        self.vol_range = (min, max);
        self
    }

    /// Generate synthetic equity quotes
    pub fn generate_equities(&self) -> Vec<MarketQuote> {
        let mut rng = rand::thread_rng();
        let timestamp = Utc::now().timestamp_millis();

        (0..self.num_instruments)
            .map(|i| {
                let price = rng.gen_range(self.price_range.0..self.price_range.1);
                let spread_bps: f64 = rng.gen_range(2.0..20.0);
                let half_spread = price * spread_bps / 10000.0 / 2.0;

                MarketQuote::new(format!("SYN{:04}", i), price - half_spread, price + half_spread)
                    .with_currency(Currency::USD)
                    .with_timestamp(timestamp)
            })
            .collect()
    }

    /// Generate synthetic yield curve
    pub fn generate_yield_curve(&self, currency: Currency) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();
        let base_rate: f64 = rng.gen_range(0.01..0.06);

        let tenors = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 20.0, 30.0];
        let normal = Normal::new(0.0, 0.002).unwrap();

        tenors
            .into_iter()
            .map(|tenor| {
                // Simple curve shape: slightly upward sloping with noise
                let slope = 0.003 * tenor.ln().max(0.0);
                let noise: f64 = normal.sample(&mut rng);
                let rate = (base_rate + slope + noise).max(0.0);
                (tenor, rate * 100.0) // Return as percentage
            })
            .collect()
    }

    /// Generate synthetic volatility surface
    pub fn generate_vol_surface(&self, ticker: &str) -> Vec<(f64, f64, f64)> {
        let mut rng = rand::thread_rng();
        let atm_vol: f64 = rng.gen_range(self.vol_range.0..self.vol_range.1);

        let strikes = vec![80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0];
        let expiries = vec![0.083, 0.25, 0.5, 1.0, 2.0];

        let mut surface = Vec::new();

        for expiry in &expiries {
            for strike in &strikes {
                // Simple skew model
                let moneyness = (strike / 100.0).ln();
                let skew = -0.1 * moneyness; // Negative skew
                let term_adj = -0.02 * expiry.sqrt(); // Vol decreases with time

                let vol = (atm_vol + skew + term_adj).max(0.05);
                surface.push((*strike, *expiry, vol));
            }
        }

        surface
    }

    /// Generate correlated price paths for multiple assets
    pub fn generate_correlated_paths(
        &self,
        num_paths: usize,
        num_steps: usize,
        correlation: f64,
    ) -> Vec<Vec<Vec<f64>>> {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();

        let mut paths = vec![vec![vec![0.0; num_steps]; self.num_instruments]; num_paths];

        for path in 0..num_paths {
            // Generate initial prices
            let initial_prices: Vec<f64> = (0..self.num_instruments)
                .map(|_| rng.gen_range(self.price_range.0..self.price_range.1))
                .collect();

            let vols: Vec<f64> = (0..self.num_instruments)
                .map(|_| rng.gen_range(self.vol_range.0..self.vol_range.1))
                .collect();

            for asset in 0..self.num_instruments {
                paths[path][asset][0] = initial_prices[asset];
            }

            // Generate correlated random numbers
            for step in 1..num_steps {
                let common: f64 = normal.sample(&mut rng);

                for asset in 0..self.num_instruments {
                    let idio: f64 = normal.sample(&mut rng);
                    let z = correlation.sqrt() * common + (1.0 - correlation).sqrt() * idio;

                    let dt = 1.0 / 252.0; // Daily steps
                    let prev = paths[path][asset][step - 1];
                    let drift = 0.0;
                    let diffusion = vols[asset] * dt.sqrt() * z;

                    paths[path][asset][step] = prev * (1.0 + drift * dt + diffusion).max(0.01);
                }
            }
        }

        paths
    }
}

impl Default for SyntheticGenerator {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_equities() {
        let gen = SyntheticGenerator::new(5);
        let quotes = gen.generate_equities();
        assert_eq!(quotes.len(), 5);
    }

    #[test]
    fn test_synthetic_yield_curve() {
        let gen = SyntheticGenerator::new(1);
        let curve = gen.generate_yield_curve(Currency::USD);
        assert_eq!(curve.len(), 10);
    }
}

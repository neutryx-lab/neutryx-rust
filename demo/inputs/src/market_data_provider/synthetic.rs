//! Synthetic market data generator.
//!
//! Generates synthetic market data with configurable parameters
//! for testing and demonstration purposes.
//!
//! ## Price Evolution Models
//!
//! This module supports two primary price evolution models:
//!
//! - **Random Walk (GBM)**: Geometric Brownian Motion for equity-like price dynamics
//! - **Mean Reversion (OU)**: Ornstein-Uhlenbeck process for interest rate-like dynamics

use adapter_feeds::MarketQuote;
use chrono::Utc;
use pricer_core::types::Currency;
use rand::Rng;
use rand_distr::{Distribution, Normal};

/// Price evolution model trait for synthetic data generation.
pub trait PriceEvolutionModel: Send + Sync {
    /// Evolve the price from the current state to the next state.
    ///
    /// # Arguments
    /// * `current_price` - Current price level
    /// * `dt` - Time step in years
    /// * `random_draw` - Standard normal random number
    fn evolve(&self, current_price: f64, dt: f64, random_draw: f64) -> f64;

    /// Get the model name for identification.
    fn name(&self) -> &'static str;
}

/// Random Walk (Geometric Brownian Motion) model.
///
/// dS = μ*S*dt + σ*S*dW
///
/// Used for equity-like price dynamics where prices follow log-normal distribution.
#[derive(Debug, Clone)]
pub struct RandomWalkModel {
    /// Annual drift rate (μ)
    pub drift: f64,
    /// Annual volatility (σ)
    pub volatility: f64,
}

impl RandomWalkModel {
    /// Create a new Random Walk model.
    pub fn new(drift: f64, volatility: f64) -> Self {
        Self { drift, volatility }
    }

    /// Create with zero drift (martingale).
    pub fn zero_drift(volatility: f64) -> Self {
        Self::new(0.0, volatility)
    }
}

impl Default for RandomWalkModel {
    fn default() -> Self {
        Self::new(0.0, 0.20) // 20% annual vol, zero drift
    }
}

impl PriceEvolutionModel for RandomWalkModel {
    fn evolve(&self, current_price: f64, dt: f64, random_draw: f64) -> f64 {
        // GBM discrete approximation: S(t+dt) = S(t) * exp((μ - σ²/2)*dt + σ*sqrt(dt)*Z)
        let drift_term = (self.drift - 0.5 * self.volatility * self.volatility) * dt;
        let diffusion_term = self.volatility * dt.sqrt() * random_draw;
        (current_price * (drift_term + diffusion_term).exp()).max(0.0001)
    }

    fn name(&self) -> &'static str {
        "RandomWalk"
    }
}

/// Mean Reversion (Ornstein-Uhlenbeck) model.
///
/// dX = κ*(θ - X)*dt + σ*dW
///
/// Used for interest rate-like dynamics where prices revert to a long-term mean.
#[derive(Debug, Clone)]
pub struct MeanReversionModel {
    /// Mean reversion speed (κ)
    pub speed: f64,
    /// Long-term mean level (θ)
    pub mean_level: f64,
    /// Volatility (σ)
    pub volatility: f64,
}

impl MeanReversionModel {
    /// Create a new Mean Reversion model.
    pub fn new(speed: f64, mean_level: f64, volatility: f64) -> Self {
        Self {
            speed,
            mean_level,
            volatility,
        }
    }

    /// Create with typical parameters for interest rates.
    pub fn for_rates(mean_level: f64) -> Self {
        Self::new(0.5, mean_level, 0.01) // 50% mean reversion speed, 1% vol
    }

    /// Create with typical parameters for spreads.
    pub fn for_spreads(mean_level: f64) -> Self {
        Self::new(1.0, mean_level, 0.005) // Fast reversion, low vol
    }
}

impl Default for MeanReversionModel {
    fn default() -> Self {
        Self::new(0.5, 100.0, 5.0) // Mean revert to 100 with 5 vol
    }
}

impl PriceEvolutionModel for MeanReversionModel {
    fn evolve(&self, current_price: f64, dt: f64, random_draw: f64) -> f64 {
        // Exact OU discretisation
        let e_kt = (-self.speed * dt).exp();
        let mean = self.mean_level + (current_price - self.mean_level) * e_kt;
        let variance = self.volatility * self.volatility * (1.0 - e_kt * e_kt) / (2.0 * self.speed);
        let std_dev = variance.sqrt();
        (mean + std_dev * random_draw).max(0.0)
    }

    fn name(&self) -> &'static str {
        "MeanReversion"
    }
}

/// Streaming price generator that produces a series of quotes.
pub struct StreamingPriceGenerator {
    /// Identifier for the instrument
    identifier: String,
    /// Current price level
    current_price: f64,
    /// Currency
    currency: Currency,
    /// Time step in years
    dt: f64,
    /// Price evolution model
    model: Box<dyn PriceEvolutionModel>,
    /// Normal distribution for random draws
    normal: Normal<f64>,
    /// Spread in basis points
    spread_bps: f64,
}

impl StreamingPriceGenerator {
    /// Create a new streaming price generator.
    pub fn new(
        identifier: impl Into<String>,
        initial_price: f64,
        model: Box<dyn PriceEvolutionModel>,
    ) -> Self {
        Self {
            identifier: identifier.into(),
            current_price: initial_price,
            currency: Currency::USD,
            dt: 1.0 / 252.0 / 24.0 / 60.0, // 1 minute
            model,
            normal: Normal::new(0.0, 1.0).unwrap(),
            spread_bps: 10.0,
        }
    }

    /// Set the currency.
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Set the time step.
    pub fn with_dt(mut self, dt: f64) -> Self {
        self.dt = dt;
        self
    }

    /// Set the spread in basis points.
    pub fn with_spread_bps(mut self, spread_bps: f64) -> Self {
        self.spread_bps = spread_bps;
        self
    }

    /// Generate the next quote.
    pub fn next_quote(&mut self) -> MarketQuote {
        let mut rng = rand::thread_rng();
        let z: f64 = self.normal.sample(&mut rng);

        self.current_price = self.model.evolve(self.current_price, self.dt, z);

        let half_spread = self.current_price * self.spread_bps / 10000.0 / 2.0;
        let timestamp = Utc::now().timestamp_millis();

        MarketQuote::new(
            &self.identifier,
            self.current_price - half_spread,
            self.current_price + half_spread,
        )
        .with_currency(self.currency)
        .with_timestamp(timestamp)
    }

    /// Generate a batch of quotes.
    pub fn generate_batch(&mut self, count: usize) -> Vec<MarketQuote> {
        (0..count).map(|_| self.next_quote()).collect()
    }

    /// Get the current price.
    pub fn current_price(&self) -> f64 {
        self.current_price
    }
}

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
            .map(|tenor: f64| {
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

        let strikes: Vec<f64> = vec![80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0];
        let expiries: Vec<f64> = vec![0.083, 0.25, 0.5, 1.0, 2.0];

        let mut surface = Vec::new();

        for expiry in &expiries {
            for strike in &strikes {
                // Simple skew model
                let moneyness = (*strike / 100.0_f64).ln();
                let skew = -0.1 * moneyness; // Negative skew
                let term_adj = -0.02 * (*expiry).sqrt(); // Vol decreases with time

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

                    let dt: f64 = 1.0 / 252.0; // Daily steps
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

    #[test]
    fn test_random_walk_model_positive_prices() {
        let model = RandomWalkModel::default();
        let initial_price = 100.0;
        let dt = 1.0 / 252.0;

        // Test multiple random draws
        for z in [-3.0, -1.0, 0.0, 1.0, 3.0] {
            let next_price = model.evolve(initial_price, dt, z);
            assert!(next_price > 0.0, "Price should always be positive");
        }
    }

    #[test]
    fn test_random_walk_model_volatility_effect() {
        let low_vol = RandomWalkModel::new(0.0, 0.10);
        let high_vol = RandomWalkModel::new(0.0, 0.50);
        let initial_price = 100.0;
        let dt = 1.0 / 252.0;
        let z = 2.0; // Positive shock

        let low_vol_price = low_vol.evolve(initial_price, dt, z);
        let high_vol_price = high_vol.evolve(initial_price, dt, z);

        // Higher volatility should result in larger price change
        assert!(
            (high_vol_price - initial_price).abs() > (low_vol_price - initial_price).abs(),
            "Higher volatility should cause larger price changes"
        );
    }

    #[test]
    fn test_mean_reversion_model_reverts_to_mean() {
        let mean_level = 100.0;
        let model = MeanReversionModel::new(2.0, mean_level, 0.0); // High reversion, no vol
        let dt = 1.0;

        // Price above mean should move down
        let price_above = 150.0;
        let next_price_above = model.evolve(price_above, dt, 0.0);
        assert!(
            next_price_above < price_above,
            "Price above mean should decrease"
        );
        assert!(
            next_price_above > mean_level,
            "Price should not overshoot mean"
        );

        // Price below mean should move up
        let price_below = 50.0;
        let next_price_below = model.evolve(price_below, dt, 0.0);
        assert!(
            next_price_below > price_below,
            "Price below mean should increase"
        );
        assert!(
            next_price_below < mean_level,
            "Price should not overshoot mean"
        );
    }

    #[test]
    fn test_mean_reversion_model_for_rates() {
        let model = MeanReversionModel::for_rates(0.03); // 3% mean
        assert_eq!(model.mean_level, 0.03);
        assert!(model.volatility < 0.1, "Rate vol should be small");
    }

    #[test]
    fn test_streaming_price_generator_random_walk() {
        let model = Box::new(RandomWalkModel::default());
        let mut generator = StreamingPriceGenerator::new("AAPL", 150.0, model);

        let quotes = generator.generate_batch(10);
        assert_eq!(quotes.len(), 10);

        for quote in &quotes {
            assert_eq!(quote.identifier, "AAPL");
            assert!(quote.bid.is_some());
            assert!(quote.ask.is_some());
            assert!(quote.bid.unwrap() > 0.0);
            assert!(quote.ask.unwrap() > quote.bid.unwrap());
        }
    }

    #[test]
    fn test_streaming_price_generator_mean_reversion() {
        let model = Box::new(MeanReversionModel::default());
        let mut generator = StreamingPriceGenerator::new("RATE", 100.0, model)
            .with_currency(Currency::EUR)
            .with_spread_bps(5.0);

        let quote = generator.next_quote();
        assert_eq!(quote.identifier, "RATE");
        assert_eq!(quote.currency, Currency::EUR);
    }

    #[test]
    fn test_streaming_generator_produces_adapter_feeds_compatible_quotes() {
        let model = Box::new(RandomWalkModel::default());
        let mut generator = StreamingPriceGenerator::new("TEST", 100.0, model);

        let quote = generator.next_quote();

        // Verify adapter_feeds MarketQuote compatibility
        assert!(!quote.identifier.is_empty());
        assert!(quote.mid().is_some());
        assert!(quote.spread().is_some());
        assert!(quote.timestamp_ms > 0);
    }
}

//! Reuters-style real-time ticker simulation.
//!
//! Simulates high-frequency market data streaming with realistic
//! price movements and spread dynamics.

use adapter_feeds::MarketQuote;
use async_channel::{Receiver, Sender};
use chrono::Utc;
use pricer_core::types::Currency;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::info;

/// Reuters-style ticker simulation
pub struct ReutersSim {
    /// Instruments to simulate
    instruments: Vec<InstrumentSpec>,
    /// Update interval in milliseconds
    update_interval_ms: u64,
    /// Running flag
    running: Arc<AtomicBool>,
}

/// Specification for a simulated instrument
#[derive(Clone)]
pub struct InstrumentSpec {
    /// Ticker symbol
    pub ticker: String,
    /// Initial price
    pub initial_price: f64,
    /// Volatility (annualized)
    pub volatility: f64,
    /// Typical spread in bps
    pub spread_bps: f64,
    /// Currency
    pub currency: Currency,
}

impl Default for InstrumentSpec {
    fn default() -> Self {
        Self {
            ticker: "UNKNOWN".to_string(),
            initial_price: 100.0,
            volatility: 0.20,
            spread_bps: 5.0,
            currency: Currency::USD,
        }
    }
}

impl ReutersSim {
    /// Create a new Reuters simulator with default instruments
    pub fn new() -> Self {
        Self {
            instruments: Self::default_instruments(),
            update_interval_ms: 100,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with custom instruments
    pub fn with_instruments(instruments: Vec<InstrumentSpec>) -> Self {
        Self {
            instruments,
            update_interval_ms: 100,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set update interval
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.update_interval_ms = interval_ms;
        self
    }

    /// Default equity instruments
    fn default_instruments() -> Vec<InstrumentSpec> {
        vec![
            InstrumentSpec {
                ticker: "AAPL".to_string(),
                initial_price: 185.0,
                volatility: 0.25,
                spread_bps: 2.0,
                currency: Currency::USD,
            },
            InstrumentSpec {
                ticker: "GOOGL".to_string(),
                initial_price: 140.0,
                volatility: 0.28,
                spread_bps: 3.0,
                currency: Currency::USD,
            },
            InstrumentSpec {
                ticker: "MSFT".to_string(),
                initial_price: 380.0,
                volatility: 0.22,
                spread_bps: 2.0,
                currency: Currency::USD,
            },
            InstrumentSpec {
                ticker: "7203.T".to_string(), // Toyota
                initial_price: 2800.0,
                volatility: 0.20,
                spread_bps: 5.0,
                currency: Currency::JPY,
            },
            InstrumentSpec {
                ticker: "DBK.DE".to_string(), // Deutsche Bank
                initial_price: 15.50,
                volatility: 0.35,
                spread_bps: 8.0,
                currency: Currency::EUR,
            },
        ]
    }

    /// Start streaming quotes
    pub async fn start(&self) -> Receiver<MarketQuote> {
        let (tx, rx) = async_channel::bounded(1000);
        self.running.store(true, Ordering::SeqCst);

        let instruments = self.instruments.clone();
        let interval_ms = self.update_interval_ms;
        let running = self.running.clone();

        tokio::spawn(async move {
            Self::run_simulation(instruments, interval_ms, tx, running).await;
        });

        info!("ReutersSim started with {} instruments", self.instruments.len());
        rx
    }

    /// Stop streaming
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        info!("ReutersSim stopped");
    }

    /// Run the simulation loop
    async fn run_simulation(
        instruments: Vec<InstrumentSpec>,
        interval_ms: u64,
        tx: Sender<MarketQuote>,
        running: Arc<AtomicBool>,
    ) {
        let mut rng = StdRng::from_entropy();
        let mut prices: Vec<f64> = instruments.iter().map(|i| i.initial_price).collect();
        let mut tick_interval = interval(Duration::from_millis(interval_ms));

        // Time step for price evolution (convert interval to years)
        let dt = interval_ms as f64 / (1000.0 * 365.0 * 24.0 * 3600.0);

        while running.load(Ordering::SeqCst) {
            tick_interval.tick().await;

            for (idx, spec) in instruments.iter().enumerate() {
                // Geometric Brownian Motion step
                let normal = Normal::new(0.0, 1.0).unwrap();
                let z: f64 = normal.sample(&mut rng);
                let drift = 0.0; // No drift for simulation
                let diffusion = spec.volatility * dt.sqrt() * z;
                prices[idx] *= (1.0 + drift * dt + diffusion).max(0.01);

                // Calculate bid/ask with spread
                let mid = prices[idx];
                let half_spread = mid * spec.spread_bps / 10000.0 / 2.0;
                let bid = mid - half_spread;
                let ask = mid + half_spread;

                let quote = MarketQuote::new(&spec.ticker, bid, ask)
                    .with_currency(spec.currency)
                    .with_timestamp(Utc::now().timestamp_millis());

                if tx.send(quote).await.is_err() {
                    return; // Channel closed
                }
            }
        }
    }

    /// Get a snapshot of current prices
    pub fn snapshot(&self) -> Vec<MarketQuote> {
        let mut rng = rand::thread_rng();
        self.instruments
            .iter()
            .map(|spec| {
                // Add small random noise for snapshot
                let noise: f64 = rng.gen_range(-0.001..0.001);
                let mid = spec.initial_price * (1.0 + noise);
                let half_spread = mid * spec.spread_bps / 10000.0 / 2.0;

                MarketQuote::new(&spec.ticker, mid - half_spread, mid + half_spread)
                    .with_currency(spec.currency)
                    .with_timestamp(Utc::now().timestamp_millis())
            })
            .collect()
    }
}

impl Default for ReutersSim {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reuters_sim_snapshot() {
        let sim = ReutersSim::new();
        let quotes = sim.snapshot();
        assert_eq!(quotes.len(), 5);
        assert!(quotes[0].bid.unwrap() > 0.0);
    }
}

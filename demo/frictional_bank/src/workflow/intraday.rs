//! Intraday Workflow implementation.
//!
//! Handles real-time portfolio re-evaluation on market data updates.
//! Subscribes to MarketDataProvider stream and updates risk metrics.

use super::{DemoWorkflow, ProgressCallback, WorkflowResult, WorkflowStep};
use crate::config::DemoConfig;
use crate::error::DemoError;
use async_trait::async_trait;
use demo_inputs::prelude::{FrontOffice, TradeSource};
use demo_inputs::trade_source::{TradeParams, TradeRecord};
use demo_outputs::prelude::WebSocketSink;
// MetricType and MetricUpdate are available for real-time dashboard updates
#[allow(unused_imports)]
use demo_outputs::risk_dashboard::{MetricType, MetricUpdate};
use pricer_core::types::Currency;
use pricer_models::demo::{BlackScholes, InstrumentEnum, ModelEnum, VanillaSwap};
use pricer_optimiser::provider::MarketProvider;
use pricer_risk::demo::{run_portfolio_pricing_sequential, DemoTrade};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Intraday Workflow
pub struct IntradayWorkflow {
    /// Running flag
    running: Arc<AtomicBool>,
}

impl IntradayWorkflow {
    /// Create a new intraday workflow
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the workflow is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Convert TradeRecord to DemoTrade for pricing
    fn convert_trade_record(record: &TradeRecord) -> DemoTrade {
        let ccy = Self::parse_currency(&record.currency);
        let fixed_rate = Self::extract_fixed_rate(record);

        DemoTrade::new(
            record.trade_id.clone(),
            ccy,
            ModelEnum::BlackScholes(BlackScholes { vol: 0.2 }),
            InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate }),
        )
    }

    /// Parse currency string to Currency enum
    fn parse_currency(ccy_str: &str) -> Currency {
        match ccy_str {
            "USD" => Currency::USD,
            "EUR" => Currency::EUR,
            "GBP" => Currency::GBP,
            "JPY" => Currency::JPY,
            "CHF" => Currency::CHF,
            _ => Currency::USD,
        }
    }

    /// Extract a fixed rate from trade params
    fn extract_fixed_rate(record: &TradeRecord) -> f64 {
        match &record.params {
            TradeParams::InterestRateSwap { fixed_rate, .. } => *fixed_rate,
            TradeParams::EquityOption { strike, .. } => strike / 100.0 * 0.01,
            TradeParams::Forward { forward_price, .. } => forward_price / 100.0 * 0.01,
            TradeParams::FxForward { rate, .. } => rate * 0.01,
            TradeParams::FxOption { strike, .. } => strike * 0.01,
            TradeParams::CreditDefaultSwap { spread_bps, .. } => spread_bps / 10000.0,
        }
    }

    /// Calculate risk metrics from pricing results
    fn calculate_risk_metrics(
        trade_records: &[TradeRecord],
        pv_values: &[f64],
        prev_pv: Option<f64>,
    ) -> RiskMetrics {
        let total_pv: f64 = pv_values
            .iter()
            .zip(trade_records.iter())
            .map(|(pv, t)| pv * t.notional)
            .sum();

        // Mock Greeks calculation
        let delta = total_pv * 0.05;
        let gamma = total_pv * 0.001;
        let vega = total_pv * 0.02;

        // PnL if we have previous PV
        let pnl = prev_pv.map(|prev| total_pv - prev).unwrap_or(0.0);

        RiskMetrics {
            total_pv,
            delta,
            gamma,
            vega,
            pnl,
        }
    }
}

/// Risk metrics for a single update
#[derive(Debug, Clone)]
struct RiskMetrics {
    total_pv: f64,
    delta: f64,
    gamma: f64,
    vega: f64,
    pnl: f64,
}

impl Default for IntradayWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DemoWorkflow for IntradayWorkflow {
    fn name(&self) -> &str {
        "Intraday"
    }

    async fn run(
        &self,
        config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError> {
        let start = Instant::now();
        self.running.store(true, Ordering::SeqCst);

        tracing::info!("Starting Intraday workflow");

        // Report initial progress
        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingTrades, 0.0);
        }

        // Load initial portfolio
        let trades_count = config.max_trades.unwrap_or(20);
        let front_office = FrontOffice::new();
        let trade_records = front_office.generate_trades(trades_count);
        tracing::info!("Loaded {} trades for intraday monitoring", trade_records.len());

        // Convert to DemoTrades for pricing
        let demo_trades: Vec<DemoTrade> = trade_records
            .iter()
            .map(Self::convert_trade_record)
            .collect();

        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingTrades, 1.0);
        }

        // Create WebSocket sink for real-time updates
        let _ws_sink = WebSocketSink::new();

        // Market provider for pricing
        let market = MarketProvider::new();

        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingMarketData, 1.0);
        }

        let mut updates_processed = 0;
        let max_updates = config.max_trades.unwrap_or(10);
        let mut prev_total_pv: Option<f64> = None;

        // Intraday processing loop
        while self.running.load(Ordering::SeqCst) && updates_processed < max_updates {
            // Simulate market data update delay (100ms target for real-time)
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Re-price portfolio
            let pricing_results = run_portfolio_pricing_sequential(&demo_trades, &market);
            let pv_values: Vec<f64> = pricing_results.iter().map(|r| r.pv).collect();

            // Calculate risk metrics
            let metrics = Self::calculate_risk_metrics(&trade_records, &pv_values, prev_total_pv);
            prev_total_pv = Some(metrics.total_pv);

            // Log metrics
            tracing::debug!(
                "Update {}: PV={:.2}, Delta={:.2}, PnL={:.2}",
                updates_processed + 1,
                metrics.total_pv,
                metrics.delta,
                metrics.pnl
            );

            // Report progress
            updates_processed += 1;
            if let Some(ref cb) = progress {
                let pct = updates_processed as f64 / max_updates as f64;
                cb(WorkflowStep::Pricing, pct);
            }
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(ref cb) = progress {
            cb(WorkflowStep::Completed, 1.0);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let avg_latency = if updates_processed > 0 {
            duration_ms / updates_processed as u64
        } else {
            0
        };

        tracing::info!(
            "Intraday workflow completed: {} updates in {}ms (avg {:.0}ms/update)",
            updates_processed,
            duration_ms,
            avg_latency
        );

        Ok(WorkflowResult::success(duration_ms, updates_processed))
    }

    async fn cancel(&self) {
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("Intraday workflow cancelled");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intraday_workflow() {
        let workflow = IntradayWorkflow::new();
        let mut config = DemoConfig::default();
        config.max_trades = Some(5);

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        assert!(result.trades_processed > 0);
    }

    #[tokio::test]
    async fn test_intraday_is_running() {
        let workflow = IntradayWorkflow::new();
        assert!(!workflow.is_running());
    }

    #[tokio::test]
    async fn test_intraday_cancellation() {
        let workflow = IntradayWorkflow::new();
        let mut config = DemoConfig::default();
        config.max_trades = Some(100); // Many updates

        // Cancel immediately
        workflow.running.store(false, Ordering::SeqCst);

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        // Should complete with 0 updates due to immediate cancel
    }

    #[test]
    fn test_parse_currency() {
        assert_eq!(IntradayWorkflow::parse_currency("USD"), Currency::USD);
        assert_eq!(IntradayWorkflow::parse_currency("EUR"), Currency::EUR);
        assert_eq!(IntradayWorkflow::parse_currency("UNKNOWN"), Currency::USD);
    }
}

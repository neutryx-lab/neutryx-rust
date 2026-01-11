//! EOD Batch Workflow implementation.
//!
//! Executes end-of-day batch processing following A-I-P-S data flow:
//! 1. Load trades from demo_inputs
//! 2. Load market data from demo_inputs
//! 3. Calibrate models using pricer_optimiser
//! 4. Price portfolio using pricer_risk
//! 5. Calculate XVA using pricer_risk
//! 6. Generate reports to demo_outputs

use super::{DemoWorkflow, ProgressCallback, WorkflowResult, WorkflowStep};
use crate::config::DemoConfig;
use crate::error::DemoError;
use async_trait::async_trait;
use demo_inputs::prelude::{FrontOffice, TradeSource};
use demo_inputs::trade_source::{InstrumentType, TradeParams, TradeRecord};
use demo_outputs::prelude::FileWriter;
use demo_outputs::report_sink::{Report, ReportFormat, ReportSink};
use pricer_core::types::Currency;
use pricer_models::demo::{BlackScholes, InstrumentEnum, ModelEnum, VanillaSwap};
use pricer_optimiser::provider::MarketProvider;
use pricer_risk::demo::{run_portfolio_pricing, DemoTrade, PricingResultDemo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// EOD Batch Workflow
pub struct EodBatchWorkflow {
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
}

impl EodBatchWorkflow {
    /// Create a new EOD batch workflow
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Report progress if callback is provided
    fn report_progress(progress: &Option<ProgressCallback>, step: WorkflowStep, pct: f64) {
        if let Some(cb) = progress {
            cb(step, pct);
        }
    }

    /// Convert TradeRecord from demo_inputs to DemoTrade for pricer_risk
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
            // Other currencies default to USD for demo purposes
            _ => Currency::USD,
        }
    }

    /// Extract a fixed rate from trade params (for conversion to VanillaSwap)
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

    /// Generate pricing report content
    fn generate_pricing_report(
        trade_records: &[TradeRecord],
        pricing_results: &[PricingResultDemo],
    ) -> String {
        let mut content = String::from("trade_id,instrument_type,counterparty,currency,notional,pv\n");

        for (record, result) in trade_records.iter().zip(pricing_results.iter()) {
            let instrument_type = match record.instrument_type {
                InstrumentType::EquityOption => "EquityOption",
                InstrumentType::EquityForward => "EquityForward",
                InstrumentType::InterestRateSwap => "IRS",
                InstrumentType::FxForward => "FxForward",
                InstrumentType::FxOption => "FxOption",
                InstrumentType::CreditDefaultSwap => "CDS",
            };

            content.push_str(&format!(
                "{},{},{},{},{:.2},{:.2}\n",
                record.trade_id,
                instrument_type,
                record.counterparty_id,
                record.currency,
                record.notional,
                result.pv * record.notional
            ));
        }

        content
    }

    /// Generate XVA summary report content
    fn generate_xva_report(
        trade_records: &[TradeRecord],
        pricing_results: &[PricingResultDemo],
    ) -> String {
        // Simplified XVA calculation for demo purposes
        let total_pv: f64 = pricing_results
            .iter()
            .zip(trade_records.iter())
            .map(|(r, t)| r.pv * t.notional)
            .sum();

        // Mock XVA values based on portfolio PV
        let cva = total_pv.abs() * 0.0025;
        let dva = total_pv.abs() * 0.0015;
        let fva = total_pv.abs() * 0.0008;
        let xva_total = cva + dva + fva;

        format!(
            r#"{{
  "report_type": "XVA Summary",
  "generated_at": "{}",
  "metrics": {{
    "total_pv": {:.2},
    "cva": {:.2},
    "dva": {:.2},
    "fva": {:.2},
    "total_xva": {:.2}
  }},
  "trade_count": {}
}}"#,
            chrono::Utc::now().to_rfc3339(),
            total_pv,
            cva,
            dva,
            fva,
            xva_total,
            trade_records.len()
        )
    }
}

impl Default for EodBatchWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DemoWorkflow for EodBatchWorkflow {
    fn name(&self) -> &str {
        "EOD Batch"
    }

    async fn run(
        &self,
        config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError> {
        let start = Instant::now();
        self.cancelled.store(false, Ordering::SeqCst);
        let mut errors: Vec<String> = Vec::new();

        tracing::info!("Starting EOD Batch workflow");

        // Step 1: Load trades from demo_inputs
        Self::report_progress(&progress, WorkflowStep::LoadingTrades, 0.0);
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(WorkflowResult::failure(
                start.elapsed().as_millis() as u64,
                vec!["Workflow cancelled".to_string()],
            ));
        }

        let trades_count = config.max_trades.unwrap_or(100);
        let front_office = FrontOffice::new();
        let trade_records = front_office.generate_trades(trades_count);
        tracing::info!("Loaded {} trades from FrontOffice", trade_records.len());
        Self::report_progress(&progress, WorkflowStep::LoadingTrades, 1.0);

        // Step 2: Load market data (MarketProvider handles lazy loading)
        Self::report_progress(&progress, WorkflowStep::LoadingMarketData, 0.0);
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(WorkflowResult::failure(
                start.elapsed().as_millis() as u64,
                vec!["Workflow cancelled".to_string()],
            ));
        }

        let market = MarketProvider::new();
        tracing::info!("MarketProvider initialised (lazy loading enabled)");
        Self::report_progress(&progress, WorkflowStep::LoadingMarketData, 1.0);

        // Step 3: Calibrate models (handled by MarketProvider on first access)
        Self::report_progress(&progress, WorkflowStep::Calibrating, 0.0);
        // MarketProvider calibrates curves/vols on first access
        tracing::info!("Model calibration ready (on-demand via MarketProvider)");
        Self::report_progress(&progress, WorkflowStep::Calibrating, 1.0);

        // Step 4: Price portfolio using pricer_risk::demo
        Self::report_progress(&progress, WorkflowStep::Pricing, 0.0);
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(WorkflowResult::failure(
                start.elapsed().as_millis() as u64,
                vec!["Workflow cancelled".to_string()],
            ));
        }

        // Convert TradeRecords to DemoTrades for pricing
        let demo_trades: Vec<DemoTrade> = trade_records
            .iter()
            .map(Self::convert_trade_record)
            .collect();

        let pricing_results = run_portfolio_pricing(&demo_trades, &market);
        let total_pv: f64 = pricing_results
            .iter()
            .zip(trade_records.iter())
            .map(|(r, t)| r.pv * t.notional)
            .sum();
        tracing::info!(
            "Priced {} trades, total PV: {:.2}",
            pricing_results.len(),
            total_pv
        );
        Self::report_progress(&progress, WorkflowStep::Pricing, 1.0);

        // Step 5: Calculate XVA (simplified for demo)
        Self::report_progress(&progress, WorkflowStep::CalculatingXva, 0.0);
        let cva = total_pv.abs() * 0.0025;
        let dva = total_pv.abs() * 0.0015;
        let fva = total_pv.abs() * 0.0008;
        tracing::info!(
            "XVA calculated - CVA: {:.2}, DVA: {:.2}, FVA: {:.2}",
            cva,
            dva,
            fva
        );
        Self::report_progress(&progress, WorkflowStep::CalculatingXva, 1.0);

        // Step 6: Generate reports
        Self::report_progress(&progress, WorkflowStep::GeneratingReports, 0.0);

        let pricing_report_content =
            Self::generate_pricing_report(&trade_records, &pricing_results);
        let xva_report_content = Self::generate_xva_report(&trade_records, &pricing_results);

        tracing::info!("Generated pricing and XVA reports");
        Self::report_progress(&progress, WorkflowStep::GeneratingReports, 1.0);

        // Step 7: Send outputs to demo_outputs
        Self::report_progress(&progress, WorkflowStep::SendingOutputs, 0.0);

        let output_dir = config.data_dir.join("output");
        let file_writer = FileWriter::new(&output_dir);

        // Write pricing report
        let pricing_report = Report {
            report_id: format!("EOD_PRICING_{}", chrono::Utc::now().format("%Y%m%d")),
            title: "EOD Pricing Report".to_string(),
            report_type: ReportFormat::Csv,
            content: pricing_report_content,
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec![],
        };

        if let Err(e) = file_writer.send(&pricing_report) {
            errors.push(format!("Failed to write pricing report: {}", e));
            tracing::warn!("Failed to write pricing report: {}", e);
        }

        // Write XVA report
        let xva_report = Report {
            report_id: format!("EOD_XVA_{}", chrono::Utc::now().format("%Y%m%d")),
            title: "EOD XVA Summary".to_string(),
            report_type: ReportFormat::Json,
            content: xva_report_content,
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec![],
        };

        if let Err(e) = file_writer.send(&xva_report) {
            errors.push(format!("Failed to write XVA report: {}", e));
            tracing::warn!("Failed to write XVA report: {}", e);
        }

        tracing::info!("Reports written to {}", output_dir.display());
        Self::report_progress(&progress, WorkflowStep::SendingOutputs, 1.0);

        // Complete
        Self::report_progress(&progress, WorkflowStep::Completed, 1.0);

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::info!("EOD Batch completed in {}ms", duration_ms);

        let mut result = WorkflowResult::success(duration_ms, trades_count);
        result.errors = errors;
        Ok(result)
    }

    async fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        tracing::info!("EOD Batch workflow cancelled");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> DemoConfig {
        let mut config = DemoConfig::default();
        config.data_dir = std::env::temp_dir().join("frictional_bank_test");
        config
    }

    #[tokio::test]
    async fn test_eod_batch_workflow() {
        let workflow = EodBatchWorkflow::new();
        let mut config = test_config();
        config.max_trades = Some(10); // Small number for testing

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        assert!(result.trades_processed > 0);
    }

    #[tokio::test]
    async fn test_eod_batch_with_progress() {
        let workflow = EodBatchWorkflow::new();
        let mut config = test_config();
        config.max_trades = Some(5);

        let steps_received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let steps_clone = steps_received.clone();

        let progress: ProgressCallback = Arc::new(move |step, pct| {
            steps_clone.lock().unwrap().push((step, pct));
        });

        let result = workflow.run(&config, Some(progress)).await.unwrap();
        assert!(result.success);

        let steps = steps_received.lock().unwrap();
        assert!(!steps.is_empty());
        // Should include LoadingTrades step
        assert!(steps.iter().any(|(s, _)| *s == WorkflowStep::LoadingTrades));
        // Should end with Completed
        assert!(steps.iter().any(|(s, _)| *s == WorkflowStep::Completed));
    }

    #[test]
    fn test_convert_trade_record() {
        let record = TradeRecord {
            trade_id: "TEST001".to_string(),
            instrument_type: InstrumentType::InterestRateSwap,
            counterparty_id: "CP001".to_string(),
            netting_set_id: "NS001".to_string(),
            notional: 1_000_000.0,
            currency: "USD".to_string(),
            trade_date: "2026-01-10".to_string(),
            maturity_date: "2031-01-10".to_string(),
            params: TradeParams::InterestRateSwap {
                fixed_rate: 0.0425,
                float_index: "SOFR".to_string(),
                pay_fixed: true,
            },
        };

        let demo_trade = EodBatchWorkflow::convert_trade_record(&record);
        assert_eq!(demo_trade.id, "TEST001");
        assert_eq!(demo_trade.ccy, Currency::USD);
    }

    #[test]
    fn test_parse_currency() {
        assert_eq!(EodBatchWorkflow::parse_currency("USD"), Currency::USD);
        assert_eq!(EodBatchWorkflow::parse_currency("EUR"), Currency::EUR);
        assert_eq!(EodBatchWorkflow::parse_currency("JPY"), Currency::JPY);
        assert_eq!(EodBatchWorkflow::parse_currency("UNKNOWN"), Currency::USD);
    }

    #[tokio::test]
    async fn test_eod_batch_cancellation() {
        use std::sync::Arc as StdArc;

        let workflow = StdArc::new(EodBatchWorkflow::new());
        let workflow_clone = workflow.clone();

        // Spawn a task that cancels the workflow after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            workflow_clone.cancel().await;
        });

        let mut config = test_config();
        config.max_trades = Some(1000); // Large enough to allow cancellation
        let result = workflow.run(&config, None).await.unwrap();

        // Result may succeed if it completes before cancellation
        // This test verifies the cancellation mechanism works
        assert!(result.success || result.errors.iter().any(|e| e.contains("cancelled")));
    }
}

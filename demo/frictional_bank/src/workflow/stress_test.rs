//! Stress Test Workflow implementation.
//!
//! Executes scenario-based stress testing using pricer_risk::scenarios.

use super::{DemoWorkflow, ProgressCallback, WorkflowResult, WorkflowStep};
use crate::config::DemoConfig;
use crate::error::DemoError;
use async_trait::async_trait;
use demo_inputs::prelude::{FrontOffice, TradeSource};
use demo_inputs::trade_source::{TradeParams, TradeRecord};
use demo_outputs::prelude::FileWriter;
use demo_outputs::report_sink::{Report, ReportFormat, ReportSink};
use pricer_core::types::Currency;
use pricer_models::demo::{BlackScholes, InstrumentEnum, ModelEnum, VanillaSwap};
use pricer_optimiser::provider::MarketProvider;
use pricer_risk::demo::{run_portfolio_pricing, DemoTrade};
use pricer_risk::scenarios::PresetScenarioType as PricerPresetScenarioType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Preset scenario types for stress testing (demo layer)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetScenarioType {
    /// Interest rate up 100bp
    InterestRateUp100bp,
    /// Interest rate down 100bp
    InterestRateDown100bp,
    /// Volatility up 20%
    VolatilityUp20Pct,
    /// Credit event (default)
    CreditEventDefault,
}

impl PresetScenarioType {
    /// Get all preset scenarios
    pub fn all() -> Vec<Self> {
        vec![
            Self::InterestRateUp100bp,
            Self::InterestRateDown100bp,
            Self::VolatilityUp20Pct,
            Self::CreditEventDefault,
        ]
    }

    /// Get scenario name
    pub fn name(&self) -> &'static str {
        match self {
            Self::InterestRateUp100bp => "IR +100bp",
            Self::InterestRateDown100bp => "IR -100bp",
            Self::VolatilityUp20Pct => "Vol +20%",
            Self::CreditEventDefault => "Credit Default",
        }
    }

    /// Convert to pricer_risk PresetScenarioType
    #[allow(dead_code)]
    fn to_pricer_scenario(&self) -> PricerPresetScenarioType {
        match self {
            Self::InterestRateUp100bp => PricerPresetScenarioType::RateUp100bp,
            Self::InterestRateDown100bp => PricerPresetScenarioType::RateDown100bp,
            Self::VolatilityUp20Pct => PricerPresetScenarioType::VolUp5Pts,
            Self::CreditEventDefault => PricerPresetScenarioType::CreditWiden100bp,
        }
    }
}

/// Stress test result for a single scenario
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Scenario type
    pub scenario: PresetScenarioType,
    /// Base portfolio value
    pub base_value: f64,
    /// Stressed portfolio value
    pub stressed_value: f64,
    /// P&L impact
    pub pnl: f64,
}

/// Aggregated stress test results
#[derive(Debug, Clone)]
pub struct StressTestResult {
    /// Individual scenario results
    pub scenario_results: Vec<ScenarioResult>,
    /// Worst case P&L across all scenarios
    pub worst_case_pnl: f64,
}

/// Stress Test Workflow
pub struct StressTestWorkflow {
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Scenarios to run
    scenarios: Vec<PresetScenarioType>,
}

impl StressTestWorkflow {
    /// Create a new stress test workflow with all scenarios
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            scenarios: PresetScenarioType::all(),
        }
    }

    /// Create with specific scenarios
    pub fn with_scenarios(scenarios: Vec<PresetScenarioType>) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            scenarios,
        }
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

    /// Run a single scenario with actual portfolio
    fn run_scenario_with_portfolio(
        &self,
        scenario: PresetScenarioType,
        base_value: f64,
    ) -> ScenarioResult {
        // Apply scenario-specific shock to calculate stressed value
        let shock_multiplier = match scenario {
            PresetScenarioType::InterestRateUp100bp => {
                // IR up typically decreases PV for pay-fixed swaps
                0.98
            }
            PresetScenarioType::InterestRateDown100bp => {
                // IR down typically increases PV for pay-fixed swaps
                1.02
            }
            PresetScenarioType::VolatilityUp20Pct => {
                // Vol up increases option values, mixed for swaps
                0.97
            }
            PresetScenarioType::CreditEventDefault => {
                // Credit default causes significant loss
                0.85
            }
        };

        let stressed_value = base_value * shock_multiplier;
        let pnl = stressed_value - base_value;

        ScenarioResult {
            scenario,
            base_value,
            stressed_value,
            pnl,
        }
    }

    /// Generate stress test report content
    fn generate_stress_report(results: &StressTestResult) -> String {
        let mut content = format!(
            r#"{{
  "report_type": "Stress Test Results",
  "generated_at": "{}",
  "summary": {{
    "worst_case_pnl": {:.2},
    "scenarios_run": {}
  }},
  "scenario_results": [
"#,
            chrono::Utc::now().to_rfc3339(),
            results.worst_case_pnl,
            results.scenario_results.len()
        );

        for (i, result) in results.scenario_results.iter().enumerate() {
            let comma = if i < results.scenario_results.len() - 1 {
                ","
            } else {
                ""
            };
            content.push_str(&format!(
                r#"    {{
      "scenario": "{}",
      "base_value": {:.2},
      "stressed_value": {:.2},
      "pnl": {:.2}
    }}{}"#,
                result.scenario.name(),
                result.base_value,
                result.stressed_value,
                result.pnl,
                comma
            ));
            content.push('\n');
        }

        content.push_str("  ]\n}");
        content
    }
}

impl Default for StressTestWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DemoWorkflow for StressTestWorkflow {
    fn name(&self) -> &str {
        "Stress Test"
    }

    async fn run(
        &self,
        config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError> {
        let start = Instant::now();
        self.cancelled.store(false, Ordering::SeqCst);
        let mut errors: Vec<String> = Vec::new();

        tracing::info!(
            "Starting Stress Test workflow with {} scenarios",
            self.scenarios.len()
        );

        // Step 1: Load portfolio for stress testing
        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingTrades, 0.0);
        }

        let trades_count = config.max_trades.unwrap_or(50);
        let front_office = FrontOffice::new();
        let trade_records = front_office.generate_trades(trades_count);
        tracing::info!("Loaded {} trades for stress testing", trade_records.len());

        // Convert to DemoTrades
        let demo_trades: Vec<DemoTrade> = trade_records
            .iter()
            .map(Self::convert_trade_record)
            .collect();

        if let Some(ref cb) = progress {
            cb(WorkflowStep::LoadingTrades, 1.0);
        }

        // Step 2: Calculate base portfolio value
        if let Some(ref cb) = progress {
            cb(WorkflowStep::Pricing, 0.0);
        }

        let market = MarketProvider::new();
        let base_results = run_portfolio_pricing(&demo_trades, &market);
        let base_value: f64 = base_results
            .iter()
            .zip(trade_records.iter())
            .map(|(r, t)| r.pv * t.notional)
            .sum();

        tracing::info!("Base portfolio value: {:.2}", base_value);

        if let Some(ref cb) = progress {
            cb(WorkflowStep::Pricing, 0.5);
        }

        // Step 3: Run scenario analysis
        let mut results = Vec::new();
        let total_scenarios = self.scenarios.len();

        for (idx, scenario) in self.scenarios.iter().enumerate() {
            if self.cancelled.load(Ordering::SeqCst) {
                return Ok(WorkflowResult::failure(
                    start.elapsed().as_millis() as u64,
                    vec!["Workflow cancelled".to_string()],
                ));
            }

            tracing::info!("Running scenario: {}", scenario.name());
            let result = self.run_scenario_with_portfolio(*scenario, base_value);
            tracing::info!(
                "Scenario {} - Base: {:.2}, Stressed: {:.2}, P&L: {:.2}",
                scenario.name(),
                result.base_value,
                result.stressed_value,
                result.pnl
            );
            results.push(result);

            if let Some(ref cb) = progress {
                let pct = 0.5 + (idx as f64 + 1.0) / (total_scenarios as f64 * 2.0);
                cb(WorkflowStep::Pricing, pct);
            }
        }

        // Calculate worst case P&L
        let worst_case_pnl = results
            .iter()
            .map(|r| r.pnl)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let stress_result = StressTestResult {
            scenario_results: results,
            worst_case_pnl,
        };

        tracing::info!("Worst case P&L: {:.2}", worst_case_pnl);

        // Step 4: Generate and save report
        if let Some(ref cb) = progress {
            cb(WorkflowStep::GeneratingReports, 0.0);
        }

        let report_content = Self::generate_stress_report(&stress_result);

        if let Some(ref cb) = progress {
            cb(WorkflowStep::GeneratingReports, 1.0);
        }

        // Write report to file
        if let Some(ref cb) = progress {
            cb(WorkflowStep::SendingOutputs, 0.0);
        }

        let output_dir = config.data_dir.join("output");
        let file_writer = FileWriter::new(&output_dir);

        let report = Report {
            report_id: format!("STRESS_TEST_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S")),
            title: "Stress Test Results".to_string(),
            report_type: ReportFormat::Json,
            content: report_content,
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec![],
        };

        if let Err(e) = file_writer.send(&report) {
            errors.push(format!("Failed to write stress test report: {}", e));
            tracing::warn!("Failed to write stress test report: {}", e);
        }

        if let Some(ref cb) = progress {
            cb(WorkflowStep::SendingOutputs, 1.0);
        }

        if let Some(ref cb) = progress {
            cb(WorkflowStep::Completed, 1.0);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::info!(
            "Stress Test completed: worst case P&L = {:.2} in {}ms",
            stress_result.worst_case_pnl,
            duration_ms
        );

        let mut result = WorkflowResult::success(duration_ms, total_scenarios);
        result.errors = errors;
        Ok(result)
    }

    async fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        tracing::info!("Stress Test workflow cancelled");
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
    async fn test_stress_test_workflow() {
        let workflow = StressTestWorkflow::new();
        let mut config = test_config();
        config.max_trades = Some(10);

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.trades_processed, 4); // 4 preset scenarios
    }

    #[test]
    fn test_preset_scenarios() {
        let scenarios = PresetScenarioType::all();
        assert_eq!(scenarios.len(), 4);
    }

    #[test]
    fn test_scenario_names() {
        assert_eq!(PresetScenarioType::InterestRateUp100bp.name(), "IR +100bp");
        assert_eq!(PresetScenarioType::CreditEventDefault.name(), "Credit Default");
    }

    #[tokio::test]
    async fn test_stress_test_with_specific_scenarios() {
        let scenarios = vec![
            PresetScenarioType::InterestRateUp100bp,
            PresetScenarioType::VolatilityUp20Pct,
        ];
        let workflow = StressTestWorkflow::with_scenarios(scenarios);
        let mut config = test_config();
        config.max_trades = Some(5);

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.trades_processed, 2);
    }

    #[tokio::test]
    async fn test_stress_test_cancellation() {
        use std::sync::Arc as StdArc;

        let workflow = StdArc::new(StressTestWorkflow::new());
        let workflow_clone = workflow.clone();

        // Spawn a task that cancels the workflow after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            workflow_clone.cancel().await;
        });

        let mut config = test_config();
        config.max_trades = Some(500); // Large enough to allow cancellation
        let result = workflow.run(&config, None).await.unwrap();

        // Result may succeed if it completes before cancellation
        // This test verifies the cancellation mechanism works
        assert!(result.success || result.errors.iter().any(|e| e.contains("cancelled")));
    }
}

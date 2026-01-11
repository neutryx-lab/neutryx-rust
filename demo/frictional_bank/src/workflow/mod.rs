//! Demo workflow definitions and implementations.
//!
//! Provides the trait interface and implementations for demo workflows:
//! - EOD Batch Processing
//! - Intraday Real-time Processing
//! - Stress Testing

mod eod_batch;
mod intraday;
mod stress_test;

pub use eod_batch::EodBatchWorkflow;
pub use intraday::IntradayWorkflow;
pub use stress_test::{PresetScenarioType, ScenarioResult, StressTestResult, StressTestWorkflow};

use crate::config::DemoConfig;
use crate::error::DemoError;
use async_trait::async_trait;
use std::sync::Arc;

/// Workflow processing step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStep {
    /// Loading trade data
    LoadingTrades,
    /// Loading market data
    LoadingMarketData,
    /// Calibrating models
    Calibrating,
    /// Pricing portfolio
    Pricing,
    /// Calculating XVA
    CalculatingXva,
    /// Generating reports
    GeneratingReports,
    /// Sending outputs
    SendingOutputs,
    /// Workflow completed
    Completed,
}

impl WorkflowStep {
    /// Get the step name for display
    pub fn name(&self) -> &'static str {
        match self {
            Self::LoadingTrades => "Loading Trades",
            Self::LoadingMarketData => "Loading Market Data",
            Self::Calibrating => "Calibrating",
            Self::Pricing => "Pricing",
            Self::CalculatingXva => "Calculating XVA",
            Self::GeneratingReports => "Generating Reports",
            Self::SendingOutputs => "Sending Outputs",
            Self::Completed => "Completed",
        }
    }
}

/// Progress callback type for reporting workflow progress
pub type ProgressCallback = Arc<dyn Fn(WorkflowStep, f64) + Send + Sync>;

/// Workflow execution result
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// Whether the workflow completed successfully
    pub success: bool,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Number of trades processed
    pub trades_processed: usize,
    /// List of non-fatal errors encountered
    pub errors: Vec<String>,
}

impl WorkflowResult {
    /// Create a successful result
    pub fn success(duration_ms: u64, trades_processed: usize) -> Self {
        Self {
            success: true,
            duration_ms,
            trades_processed,
            errors: Vec::new(),
        }
    }

    /// Create a failure result
    pub fn failure(duration_ms: u64, errors: Vec<String>) -> Self {
        Self {
            success: false,
            duration_ms,
            trades_processed: 0,
            errors,
        }
    }

    /// Add an error to the result
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }
}

/// Demo workflow trait
///
/// All demo workflows implement this trait for unified execution.
#[async_trait]
pub trait DemoWorkflow: Send + Sync {
    /// Get the workflow name
    fn name(&self) -> &str;

    /// Execute the workflow
    async fn run(
        &self,
        config: &DemoConfig,
        progress: Option<ProgressCallback>,
    ) -> Result<WorkflowResult, DemoError>;

    /// Cancel the workflow
    async fn cancel(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_step_names() {
        assert_eq!(WorkflowStep::LoadingTrades.name(), "Loading Trades");
        assert_eq!(WorkflowStep::Completed.name(), "Completed");
    }

    #[test]
    fn test_workflow_result() {
        let result = WorkflowResult::success(1000, 50);
        assert!(result.success);
        assert_eq!(result.trades_processed, 50);

        let result = WorkflowResult::failure(500, vec!["error".to_string()]);
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
    }
}

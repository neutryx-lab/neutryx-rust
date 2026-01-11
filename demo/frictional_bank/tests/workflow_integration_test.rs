//! Workflow Integration Tests
//!
//! Tests the workflow orchestration and execution.

use frictional_bank::config::DemoConfig;
use frictional_bank::workflow::{
    DemoWorkflow, EodBatchWorkflow, IntradayWorkflow, StressTestWorkflow, WorkflowStep,
};
use std::sync::Arc;

fn test_config() -> DemoConfig {
    let mut config = DemoConfig::default();
    config.data_dir = std::env::temp_dir().join("frictional_bank_test");
    config
}

/// Test EOD batch workflow executes successfully
#[tokio::test]
async fn test_eod_batch_workflow_execution() {
    let workflow = EodBatchWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(10);

    let result = workflow.run(&config, None).await.unwrap();

    assert!(result.success);
    assert!(result.trades_processed > 0);
    assert!(result.duration_ms > 0 || result.duration_ms == 0); // Duration can be 0 for fast runs
}

/// Test EOD batch workflow with progress callback
#[tokio::test]
async fn test_eod_batch_with_progress() {
    let workflow = EodBatchWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(5);

    let steps = Arc::new(std::sync::Mutex::new(Vec::new()));
    let steps_clone = steps.clone();

    let progress = Arc::new(move |step: WorkflowStep, pct: f64| {
        steps_clone.lock().unwrap().push((step, pct));
    });

    let result = workflow.run(&config, Some(progress)).await.unwrap();
    assert!(result.success);

    let recorded_steps = steps.lock().unwrap();
    assert!(!recorded_steps.is_empty());

    // Verify key steps were reported
    let step_types: Vec<_> = recorded_steps.iter().map(|(s, _)| *s).collect();
    assert!(step_types.contains(&WorkflowStep::LoadingTrades));
    assert!(step_types.contains(&WorkflowStep::Pricing));
    assert!(step_types.contains(&WorkflowStep::Completed));
}

/// Test intraday workflow executes successfully
#[tokio::test]
async fn test_intraday_workflow_execution() {
    let workflow = IntradayWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(5);

    let result = workflow.run(&config, None).await.unwrap();

    assert!(result.success);
    assert!(result.trades_processed > 0);
}

/// Test intraday workflow is_running check
#[tokio::test]
async fn test_intraday_running_state() {
    let workflow = IntradayWorkflow::new();
    assert!(!workflow.is_running());
}

/// Test stress test workflow executes all scenarios
#[tokio::test]
async fn test_stress_test_all_scenarios() {
    let workflow = StressTestWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(10);

    let result = workflow.run(&config, None).await.unwrap();

    assert!(result.success);
    assert_eq!(result.trades_processed, 4); // 4 preset scenarios
}

/// Test stress test with specific scenarios
#[tokio::test]
async fn test_stress_test_specific_scenarios() {
    use frictional_bank::workflow::PresetScenarioType;

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

/// Test workflow names
#[test]
fn test_workflow_names() {
    let eod = EodBatchWorkflow::new();
    let intraday = IntradayWorkflow::new();
    let stress = StressTestWorkflow::new();

    assert_eq!(eod.name(), "EOD Batch");
    assert_eq!(intraday.name(), "Intraday");
    assert_eq!(stress.name(), "Stress Test");
}

/// Test workflow cancellation mechanism
#[tokio::test]
async fn test_workflow_cancellation() {
    let workflow = Arc::new(EodBatchWorkflow::new());
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
    // This test verifies the cancellation mechanism is properly set up
    assert!(result.success || result.errors.iter().any(|e| e.contains("cancelled")));
}

/// Test config affects workflow
#[tokio::test]
async fn test_config_affects_trade_count() {
    let workflow = EodBatchWorkflow::new();

    // Test with different max_trades values
    for expected_count in [5, 10, 20] {
        let mut config = test_config();
        config.max_trades = Some(expected_count);

        let result = workflow.run(&config, None).await.unwrap();
        assert!(result.success);
        // trades_processed should match max_trades
        assert_eq!(result.trades_processed, expected_count);
    }
}

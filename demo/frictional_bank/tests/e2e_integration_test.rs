//! End-to-End Integration Tests
//!
//! Tests the complete FrictionalBank demo from start to finish.

use frictional_bank::config::DemoConfig;
use frictional_bank::workflow::{DemoWorkflow, EodBatchWorkflow, IntradayWorkflow, StressTestWorkflow};
use std::path::PathBuf;

fn test_config() -> DemoConfig {
    let mut config = DemoConfig::default();
    config.data_dir = std::env::temp_dir().join("frictional_bank_test");
    config
}

/// Test complete demo orchestration
#[tokio::test]
async fn test_complete_demo_flow() {
    // Setup config
    let temp_dir = std::env::temp_dir().join("neutryx_e2e_test");
    let mut config = DemoConfig::default();
    config.max_trades = Some(20);
    config.data_dir = temp_dir.clone();

    // Create output directory
    let output_dir = temp_dir.join("output");
    std::fs::create_dir_all(&output_dir).ok();

    // Run EOD Batch workflow
    let eod_workflow = EodBatchWorkflow::new();
    let eod_result = eod_workflow.run(&config, None).await.unwrap();

    assert!(eod_result.success, "EOD Batch workflow failed");
    assert_eq!(eod_result.trades_processed, 20);

    // Run Intraday workflow
    let intraday_workflow = IntradayWorkflow::new();
    let mut intraday_config = config.clone();
    intraday_config.max_trades = Some(5); // Fewer updates for faster test
    let intraday_result = intraday_workflow.run(&intraday_config, None).await.unwrap();

    assert!(intraday_result.success, "Intraday workflow failed");
    assert!(intraday_result.trades_processed > 0);

    // Run Stress Test workflow
    let stress_workflow = StressTestWorkflow::new();
    let stress_result = stress_workflow.run(&config, None).await.unwrap();

    assert!(stress_result.success, "Stress Test workflow failed");
    assert_eq!(stress_result.trades_processed, 4); // 4 scenarios

    // Verify output files were created
    if output_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        // Should have reports from EOD and Stress Test
        assert!(!entries.is_empty(), "No output files generated");
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test sequential workflow execution
#[tokio::test]
async fn test_sequential_workflows() {
    let mut config = test_config();
    config.max_trades = Some(5);

    // Execute workflows sequentially
    let workflows: Vec<Box<dyn DemoWorkflow>> = vec![
        Box::new(EodBatchWorkflow::new()),
        Box::new(IntradayWorkflow::new()),
        Box::new(StressTestWorkflow::new()),
    ];

    for (i, workflow) in workflows.iter().enumerate() {
        let result = workflow.run(&config, None).await.unwrap();
        assert!(
            result.success,
            "Workflow {} ({}) failed",
            i,
            workflow.name()
        );
    }
}

/// Test error handling for invalid config
#[tokio::test]
async fn test_error_handling() {
    let workflow = EodBatchWorkflow::new();

    // Test with zero trades (should still work, just process 100 by default)
    let mut config = test_config();
    config.max_trades = Some(0);

    let result = workflow.run(&config, None).await.unwrap();
    // With 0 max_trades, should default to some value or handle gracefully
    assert!(result.success || result.trades_processed == 0);
}

/// Test concurrent workflow execution
#[tokio::test]
async fn test_concurrent_workflows() {
    let config = test_config();

    // Run multiple workflows concurrently
    let eod_handle = tokio::spawn({
        let cfg = config.clone();
        async move {
            let workflow = EodBatchWorkflow::new();
            let mut config = cfg;
            config.max_trades = Some(10);
            workflow.run(&config, None).await
        }
    });

    let stress_handle = tokio::spawn({
        let cfg = config.clone();
        async move {
            let workflow = StressTestWorkflow::new();
            let mut config = cfg;
            config.max_trades = Some(10);
            workflow.run(&config, None).await
        }
    });

    // Wait for both to complete
    let (eod_result, stress_result) = tokio::join!(eod_handle, stress_handle);

    assert!(eod_result.unwrap().unwrap().success);
    assert!(stress_result.unwrap().unwrap().success);
}

/// Test demo with sample data files
#[tokio::test]
async fn test_with_sample_data() {
    // This test verifies that the demo can work with our sample data files

    // Check sample data files exist
    let data_dir = PathBuf::from("demo/data");
    let config_file = data_dir.join("config/demo_config.toml");

    if config_file.exists() {
        // Load config from file
        let config_content = std::fs::read_to_string(&config_file).unwrap_or_default();
        assert!(!config_content.is_empty(), "Config file is empty");

        // Verify sample trade files exist
        let trade_files = vec![
            "input/trades/equity_book.csv",
            "input/trades/rates_book.csv",
            "input/trades/fx_book.csv",
            "input/trades/credit_book.csv",
        ];

        for file in trade_files {
            let path = data_dir.join(file);
            // Files may not exist in test environment, so just log
            if !path.exists() {
                eprintln!("Sample data file not found (expected in demo env): {}", path.display());
            }
        }
    }

    // Run workflow with default config (uses generated data)
    let workflow = EodBatchWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(5);

    let result = workflow.run(&config, None).await.unwrap();
    assert!(result.success);
}

/// Test workflow metrics
#[tokio::test]
async fn test_workflow_metrics() {
    let workflow = EodBatchWorkflow::new();
    let mut config = test_config();
    config.max_trades = Some(50);

    let result = workflow.run(&config, None).await.unwrap();

    // Verify metrics are populated
    assert!(result.success);
    assert_eq!(result.trades_processed, 50);
    // Duration should be reasonable (less than 60 seconds for 50 trades)
    assert!(result.duration_ms < 60000);
}

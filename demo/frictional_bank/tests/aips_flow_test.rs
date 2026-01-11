//! A-I-P-S Data Flow Integration Tests
//!
//! Tests the complete data flow through the architecture:
//! A (Adapter) -> I (Infra) -> P (Pricer) -> S (Service)

use demo_inputs::prelude::{FrontOffice, TradeSource};
use demo_outputs::prelude::FileWriter;
use demo_outputs::report_sink::{Report, ReportFormat, ReportSink};
use pricer_core::types::Currency;
use pricer_models::demo::{BlackScholes, InstrumentEnum, ModelEnum, VanillaSwap};
use pricer_optimiser::provider::MarketProvider;
use pricer_risk::demo::{run_portfolio_pricing, DemoTrade};

/// Test the complete A-I-P-S data flow
#[test]
fn test_aips_flow_complete() {
    // =========================================================================
    // STAGE A: Adapter - Load trades from demo_inputs
    // =========================================================================
    let front_office = FrontOffice::new();
    let trade_records = front_office.generate_trades(10);

    assert!(!trade_records.is_empty());
    assert!(trade_records.len() >= 10);

    // Verify trade structure
    for record in &trade_records {
        assert!(!record.trade_id.is_empty());
        assert!(!record.counterparty_id.is_empty());
        assert!(record.notional > 0.0);
    }

    // =========================================================================
    // STAGE I: Infra - Market data provider
    // =========================================================================
    let market = MarketProvider::new();

    // Verify market provider works for major currencies
    let _usd_curve = market.get_curve(Currency::USD);
    let _eur_curve = market.get_curve(Currency::EUR);
    let _jpy_curve = market.get_curve(Currency::JPY);

    // =========================================================================
    // STAGE P: Pricer - Price the portfolio
    // =========================================================================
    let demo_trades: Vec<DemoTrade> = trade_records
        .iter()
        .map(|record| {
            let ccy = match record.currency.as_str() {
                "USD" => Currency::USD,
                "EUR" => Currency::EUR,
                "JPY" => Currency::JPY,
                _ => Currency::USD,
            };
            DemoTrade::new(
                record.trade_id.clone(),
                ccy,
                ModelEnum::BlackScholes(BlackScholes { vol: 0.2 }),
                InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 }),
            )
        })
        .collect();

    let pricing_results = run_portfolio_pricing(&demo_trades, &market);

    // Verify pricing results
    assert_eq!(pricing_results.len(), demo_trades.len());
    for result in &pricing_results {
        assert!(!result.trade_id.is_empty());
        // PV should be a valid number (not NaN)
        assert!(result.pv.is_finite());
    }

    // =========================================================================
    // STAGE S: Service - Output to demo_outputs
    // =========================================================================
    let temp_dir = std::env::temp_dir().join("neutryx_aips_test");
    let file_writer = FileWriter::new(&temp_dir);

    // Generate a simple report
    let total_pv: f64 = pricing_results.iter().map(|r| r.pv).sum();
    let report_content = format!(
        r#"{{"trades":{}, "total_pv":{:.2}}}"#,
        pricing_results.len(),
        total_pv
    );

    let report = Report {
        report_id: "AIPS_TEST".to_string(),
        title: "A-I-P-S Test Report".to_string(),
        report_type: ReportFormat::Json,
        content: report_content,
        generated_at: chrono::Utc::now().to_rfc3339(),
        recipients: vec![],
    };

    let result = file_writer.send(&report);
    assert!(result.is_ok());

    // Verify file was written
    let written_files = file_writer.get_written_files();
    assert!(!written_files.is_empty());

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test trade source generates valid data
#[test]
fn test_trade_source_validity() {
    let front_office = FrontOffice::new();

    // Generate various trade counts
    for count in [1, 10, 50, 100] {
        let trades = front_office.generate_trades(count);
        assert!(
            trades.len() >= count,
            "Expected at least {} trades, got {}",
            count,
            trades.len()
        );
    }
}

/// Test market provider caching
#[test]
fn test_market_provider_caching() {
    use std::sync::Arc;

    let market = MarketProvider::new();

    // Get the same curve twice
    let curve1 = market.get_curve(Currency::USD);
    let curve2 = market.get_curve(Currency::USD);

    // Should be the same Arc (cached)
    assert!(Arc::ptr_eq(&curve1, &curve2));

    // Different currencies should be different
    let eur_curve = market.get_curve(Currency::EUR);
    assert!(!Arc::ptr_eq(&curve1, &eur_curve));
}

/// Test portfolio pricing produces consistent results
#[test]
fn test_pricing_consistency() {
    let market = MarketProvider::new();

    let trades = vec![
        DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02),
        DemoTrade::new_vanilla_swap("T002", Currency::USD, 0.03),
    ];

    // Price twice
    let results1 = run_portfolio_pricing(&trades, &market);
    let results2 = run_portfolio_pricing(&trades, &market);

    // Results should be identical
    assert_eq!(results1.len(), results2.len());
    for i in 0..results1.len() {
        assert!(
            (results1[i].pv - results2[i].pv).abs() < 1e-10,
            "PV mismatch for trade {}: {} vs {}",
            i,
            results1[i].pv,
            results2[i].pv
        );
    }
}

/// Test empty portfolio handling
#[test]
fn test_empty_portfolio() {
    let market = MarketProvider::new();
    let empty_trades: Vec<DemoTrade> = vec![];

    let results = run_portfolio_pricing(&empty_trades, &market);
    assert!(results.is_empty());
}

/// Test report sink file writing
#[test]
fn test_report_sink_multiple_formats() {
    let temp_dir = std::env::temp_dir().join("neutryx_report_test");
    let file_writer = FileWriter::new(&temp_dir);

    let formats = vec![
        (ReportFormat::Csv, "csv"),
        (ReportFormat::Json, "json"),
        (ReportFormat::Html, "html"),
    ];

    for (format, ext) in formats {
        let report = Report {
            report_id: format!("TEST_{}", ext.to_uppercase()),
            title: format!("Test {} Report", ext),
            report_type: format,
            content: "test content".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            recipients: vec![],
        };

        let result = file_writer.send(&report);
        assert!(result.is_ok(), "Failed to write {} report", ext);
    }

    // Verify files were written
    let written_files = file_writer.get_written_files();
    assert_eq!(written_files.len(), 3);

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

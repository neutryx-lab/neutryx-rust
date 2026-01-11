//! Integration tests to validate demo sample data files.

use std::path::Path;

/// Get the demo data directory path
fn demo_data_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .join("data")
}

#[test]
fn test_demo_data_directory_exists() {
    let data_dir = demo_data_dir();
    assert!(data_dir.exists(), "Demo data directory should exist at {:?}", data_dir);
}

#[test]
fn test_counterparties_csv_exists_and_valid() {
    let path = demo_data_dir().join("input/counterparties/counterparties.csv");
    assert!(path.exists(), "counterparties.csv should exist");

    let content = std::fs::read_to_string(&path).expect("Should read counterparties.csv");
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers = reader.headers().expect("Should have headers");
    assert!(headers.iter().any(|h| h == "counterparty_id"), "Should have counterparty_id column");
    assert!(headers.iter().any(|h| h == "name"), "Should have name column");
    assert!(headers.iter().any(|h| h == "rating"), "Should have rating column");

    let records: Vec<_> = reader.records().collect();
    assert!(records.len() >= 5, "Should have at least 5 counterparties");

    for result in records {
        let record = result.expect("Each record should be valid");
        assert!(!record.get(0).unwrap().is_empty(), "counterparty_id should not be empty");
    }
}

#[test]
fn test_netting_sets_csv_exists_and_valid() {
    let path = demo_data_dir().join("input/counterparties/netting_sets.csv");
    assert!(path.exists(), "netting_sets.csv should exist");

    let content = std::fs::read_to_string(&path).expect("Should read netting_sets.csv");
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers = reader.headers().expect("Should have headers");
    assert!(headers.iter().any(|h| h == "netting_set_id"), "Should have netting_set_id column");
    assert!(headers.iter().any(|h| h == "counterparty_id"), "Should have counterparty_id column");
    assert!(headers.iter().any(|h| h == "threshold"), "Should have threshold column");

    let records: Vec<_> = reader.records().collect();
    assert!(records.len() >= 5, "Should have at least 5 netting sets");
}

#[test]
fn test_neutryx_toml_exists_and_valid() {
    let path = demo_data_dir().join("config/neutryx.toml");
    assert!(path.exists(), "neutryx.toml should exist");

    let content = std::fs::read_to_string(&path).expect("Should read neutryx.toml");
    let config: toml::Value = toml::from_str(&content).expect("Should parse as valid TOML");

    assert!(config.get("engine").is_some(), "Should have [engine] section");
    assert!(config.get("pricing").is_some(), "Should have [pricing] section");
    assert!(config.get("risk").is_some(), "Should have [risk] section");
}

#[test]
fn test_demo_config_toml_exists_and_valid() {
    let path = demo_data_dir().join("config/demo_config.toml");
    assert!(path.exists(), "demo_config.toml should exist");

    let content = std::fs::read_to_string(&path).expect("Should read demo_config.toml");
    let config: toml::Value = toml::from_str(&content).expect("Should parse as valid TOML");

    assert!(config.get("demo").is_some(), "Should have [demo] section");
    assert!(config.get("gateway").is_some(), "Should have [gateway] section");
    assert!(config.get("workflows").is_some(), "Should have [workflows] section");

    // Verify demo mode is valid
    let demo = config.get("demo").unwrap();
    let mode = demo.get("mode").expect("Should have mode").as_str().expect("mode should be string");
    assert!(["full", "quick", "custom"].contains(&mode), "mode should be full, quick, or custom");
}

#[test]
fn test_holidays_csv_exists_and_valid() {
    let path = demo_data_dir().join("config/calendars/holidays_2026.csv");
    assert!(path.exists(), "holidays_2026.csv should exist");

    let content = std::fs::read_to_string(&path).expect("Should read holidays_2026.csv");
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers = reader.headers().expect("Should have headers");
    assert!(headers.iter().any(|h| h == "date"), "Should have date column");
    assert!(headers.iter().any(|h| h == "calendar"), "Should have calendar column");
    assert!(headers.iter().any(|h| h == "name"), "Should have name column");

    let records: Vec<_> = reader.records().collect();
    assert!(records.len() >= 10, "Should have at least 10 holidays");

    // Check we have entries for all three calendar types
    let calendars: std::collections::HashSet<_> = records.iter()
        .filter_map(|r| r.as_ref().ok())
        .filter_map(|r| r.get(1).map(|s| s.to_string()))
        .collect();

    assert!(calendars.contains("TARGET"), "Should have TARGET holidays");
    assert!(calendars.contains("NY") || calendars.contains("US"), "Should have NY/US holidays");
    assert!(calendars.contains("JP"), "Should have JP holidays");
}

#[test]
fn test_output_directory_exists() {
    let path = demo_data_dir().join("output");
    assert!(path.exists(), "output directory should exist");
    assert!(path.is_dir(), "output should be a directory");
}

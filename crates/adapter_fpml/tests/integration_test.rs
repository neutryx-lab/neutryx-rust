//! Integration tests for FpML parser using actual demo files.

use adapter_fpml::FpmlParser;
use std::fs;

/// Path to the demo FpML files.
const FPML_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../demo/data/trades/fpml");

#[test]
fn test_parse_irs_usd() {
    let xml = fs::read_to_string(format!("{}/rates/irs_usd_001.xml", FPML_BASE))
        .expect("Failed to read IRS file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse IRS");

    assert_eq!(trade.id.as_str(), "IRS-USD-001");
    assert!(trade.trade_type.is_swap());
    assert_eq!(trade.num_legs(), 2);
}

#[test]
fn test_parse_swaption() {
    let xml = fs::read_to_string(format!("{}/rates/swaption_usd_001.xml", FPML_BASE))
        .expect("Failed to read swaption file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse swaption");

    assert_eq!(trade.id.as_str(), "SWPTN-USD-001");
    assert!(trade.trade_type.is_swaption());
}

#[test]
fn test_parse_cap() {
    let xml = fs::read_to_string(format!("{}/rates/cap_usd_001.xml", FPML_BASE))
        .expect("Failed to read cap file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse cap");

    assert_eq!(trade.id.as_str(), "CAP-USD-001");
    assert!(matches!(
        trade.trade_type,
        infra_domain::trade::TradeType::CapFloor
    ));
}

#[test]
fn test_parse_fx_forward() {
    let xml = fs::read_to_string(format!("{}/fx/fxforward_eurusd_001.xml", FPML_BASE))
        .expect("Failed to read FX forward file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse FX forward");

    assert_eq!(trade.id.as_str(), "FXFWD-EURUSD-001");
    assert!(trade.trade_type.is_fx());
}

#[test]
fn test_parse_fx_swap() {
    let xml = fs::read_to_string(format!("{}/fx/fxswap_eurusd_001.xml", FPML_BASE))
        .expect("Failed to read FX swap file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse FX swap");

    assert_eq!(trade.id.as_str(), "FXSWAP-EURUSD-001");
    assert!(trade.trade_type.is_fx());
}

#[test]
fn test_parse_fx_option() {
    let xml = fs::read_to_string(format!("{}/fx/fxoption_eurusd_001.xml", FPML_BASE))
        .expect("Failed to read FX option file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse FX option");

    assert_eq!(trade.id.as_str(), "FXOPT-EURUSD-001");
    assert!(trade.trade_type.is_fx());
    assert!(trade.trade_type.is_option());
}

#[test]
fn test_parse_equity_option() {
    let xml = fs::read_to_string(format!("{}/equity/eqoption_aapl_001.xml", FPML_BASE))
        .expect("Failed to read equity option file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse equity option");

    assert_eq!(trade.id.as_str(), "EQOPT-AAPL-001");
    assert!(trade.trade_type.is_equity());
    assert!(trade.trade_type.is_option());
}

#[test]
fn test_parse_cds() {
    let xml = fs::read_to_string(format!("{}/credit/cds_ibm_001.xml", FPML_BASE))
        .expect("Failed to read CDS file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse CDS");

    assert_eq!(trade.id.as_str(), "CDS-IBM-001");
    assert!(trade.trade_type.is_credit());
}

#[test]
fn test_parse_cdx() {
    let xml = fs::read_to_string(format!("{}/credit/cdx_ig_001.xml", FPML_BASE))
        .expect("Failed to read CDX file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse CDX");

    assert_eq!(trade.id.as_str(), "CDXIG-001");
    assert!(trade.trade_type.is_credit());
}

#[test]
fn test_parse_commodity_swap() {
    let xml = fs::read_to_string(format!("{}/commodity/comswap_brent_001.xml", FPML_BASE))
        .expect("Failed to read commodity swap file");

    let trade = FpmlParser::parse(&xml).expect("Failed to parse commodity swap");

    assert_eq!(trade.id.as_str(), "COMSWAP-BRENT-001");
    assert!(trade.trade_type.is_commodity());
}

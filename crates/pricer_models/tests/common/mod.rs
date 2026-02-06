//! Common test fixtures and utilities for pricer_models tests.
//!
//! This module provides shared test utilities to reduce code duplication
//! across integration tests.

use chrono::NaiveDate;
use infra_domain::market::RateIndex;
use pricer_models::market::calibration::bootstrapping::{CurveDefinition, InstrumentSpec, InstrumentTenor};

/// Create a minimal curve definition for testing.
///
/// Uses SOFR index with standard OIS tenors (1Y, 2Y, 5Y).
#[allow(dead_code)]
pub fn minimal_curve_definition() -> CurveDefinition {
    CurveDefinition::default_usd_sofr()
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
}

/// Standard test date (2026-01-25).
#[allow(dead_code)]
pub fn test_as_of_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()
}

/// Create expiry dates relative to as-of date.
#[allow(dead_code)]
pub fn test_expiry_dates(as_of: NaiveDate) -> (NaiveDate, NaiveDate, NaiveDate) {
    let expiry_1y = as_of + chrono::Duration::days(365);
    let expiry_2y = as_of + chrono::Duration::days(730);
    let expiry_5y = as_of + chrono::Duration::days(1825);
    (expiry_1y, expiry_2y, expiry_5y)
}

/// Standard forward rate for testing (3.5%).
#[allow(dead_code)]
pub const STANDARD_FORWARD_RATE: f64 = 0.035;

/// Standard volatility for testing (20%).
#[allow(dead_code)]
pub const STANDARD_VOLATILITY: f64 = 0.20;

/// Create flat market rates for testing.
///
/// Returns a vector of (tenor_years, rate) pairs.
#[allow(dead_code)]
pub fn flat_market_rates(rate: f64) -> Vec<(f64, f64)> {
    vec![
        (0.25, rate),
        (0.5, rate),
        (1.0, rate),
        (2.0, rate),
        (5.0, rate),
        (10.0, rate),
    ]
}

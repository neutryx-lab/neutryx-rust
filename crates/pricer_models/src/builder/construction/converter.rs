//! Definition to MarketInstrument converter.
//!
//! This module converts `InstrumentDefinition` and `EventInstrument` from
//! infra_domain to `MarketInstrument<f64>` used by the pricer_models
//! calibration engine.
//!
//! Note: Currently uses `f64` as the bootstrapper only supports `f64`.

use infra_domain::market::{InstrumentDefinition, RateType};

use super::error::ConstructionError;
use crate::market::curves::{Frequency, MarketInstrument};

/// Reference date for converting event dates to maturities.
/// Represented as (year, month, day).
pub type ReferenceDate = (i32, u32, u32);

/// Converts an `InstrumentDefinition` and rate value to a
/// `MarketInstrument<f64>`.
///
/// # Arguments
///
/// * `def` - The instrument definition from infra_domain
/// * `rate` - The market rate value
/// * `reference_date` - Optional reference date for event instruments (year,
///   month, day)
///
/// # Returns
///
/// A `MarketInstrument<f64>` suitable for calibration.
///
/// # Errors
///
/// Returns error if:
/// - The tenor cannot be parsed
/// - The rate type is not supported for calibration
/// - Event instrument is missing event_date or reference_date
pub fn definition_to_instrument(
    def: &InstrumentDefinition,
    rate: f64,
    reference_date: Option<ReferenceDate>,
) -> Result<MarketInstrument<f64>, ConstructionError> {
    match def.rate_type() {
        RateType::Deposit => {
            let maturity = def
                .tenor_years()
                .map_err(ConstructionError::InstrumentDef)?;
            Ok(MarketInstrument::ois(maturity, rate))
        }

        RateType::Ois => {
            let maturity = def
                .tenor_years()
                .map_err(ConstructionError::InstrumentDef)?;

            // Get payment frequency from conventions if specified
            let frequency = def
                .conventions
                .as_ref()
                .and_then(|c| c.payment_frequency)
                .map(infra_freq_to_pricer_freq)
                .unwrap_or(Frequency::Annual);

            Ok(MarketInstrument::Ois {
                maturity,
                rate,
                payment_frequency: frequency,
            })
        }

        RateType::Swap => {
            let maturity = def
                .tenor_years()
                .map_err(ConstructionError::InstrumentDef)?;

            // Get fixed frequency from conventions if specified
            let frequency = def
                .conventions
                .as_ref()
                .and_then(|c| c.payment_frequency)
                .map(infra_freq_to_pricer_freq)
                .unwrap_or(Frequency::SemiAnnual);

            Ok(MarketInstrument::Irs {
                maturity,
                rate,
                fixed_frequency: frequency,
            })
        }

        RateType::Fra => {
            let (start_years, end_years) =
                def.fra_tenors()
                    .ok_or_else(|| ConstructionError::TenorParseError {
                        tenor: def.tenor.clone(),
                        message: "FRA requires start x end tenor format (e.g., '3x6')".to_string(),
                    })?;

            Ok(MarketInstrument::Fra {
                start: start_years,
                end: end_years,
                rate,
            })
        }

        RateType::Futures => {
            let maturity = def
                .tenor_years()
                .map_err(ConstructionError::InstrumentDef)?;

            // Convexity adjustment is typically applied externally
            Ok(MarketInstrument::Future {
                maturity,
                rate,
                convexity_adjustment: 0.0,
            })
        }

        RateType::Event => {
            let event_date_str =
                def.event_date
                    .as_ref()
                    .ok_or_else(|| ConstructionError::InvalidConfig {
                        message: format!("Event instrument '{}' requires eventDate field", def.id),
                    })?;

            let ref_date = reference_date.ok_or_else(|| ConstructionError::InvalidConfig {
                message: format!(
                    "Event instrument '{}' requires reference_date for maturity calculation",
                    def.id
                ),
            })?;

            let maturity = parse_date_to_years(event_date_str, ref_date).map_err(|e| {
                ConstructionError::InvalidConfig {
                    message: format!("Failed to parse event date '{}': {}", event_date_str, e),
                }
            })?;

            // Rate is expected in basis points (e.g., 25 for 25bp)
            // Convert to absolute rate change
            let expected_jump = rate * 0.0001;

            Ok(MarketInstrument::Event {
                maturity,
                expected_jump,
            })
        }

        // These rate types are not directly mappable to calibration instruments
        other => Err(ConstructionError::UnsupportedRateType { rate_type: other }),
    }
}

/// Parses a date string (YYYY-MM-DD) and converts it to years from reference
/// date.
///
/// Uses a simple 365-day year approximation for maturity calculation.
fn parse_date_to_years(date_str: &str, reference_date: ReferenceDate) -> Result<f64, String> {
    // Parse date string: "YYYY-MM-DD"
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err("Expected format YYYY-MM-DD".to_string());
    }

    let year: i32 = parts[0].parse().map_err(|_| "Invalid year")?;
    let month: u32 = parts[1].parse().map_err(|_| "Invalid month")?;
    let day: u32 = parts[2].parse().map_err(|_| "Invalid day")?;

    // Convert both dates to day count from a reference point
    let (ref_year, ref_month, ref_day) = reference_date;

    let event_days = date_to_days(year, month, day);
    let ref_days = date_to_days(ref_year, ref_month, ref_day);

    // Calculate difference in years (using 365.25 days per year for accuracy)
    let days_diff = event_days - ref_days;
    Ok(days_diff as f64 / 365.25)
}

/// Converts a date to days since year 0 (simplified calculation).
fn date_to_days(year: i32, month: u32, day: u32) -> i64 {
    // Simplified Julian day calculation
    let a = (14 - month as i64) / 12;
    let y = year as i64 + 4800 - a;
    let m = month as i64 + 12 * a - 3;

    day as i64 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

/// Converts infra_domain's Frequency to pricer_models's Frequency.
fn infra_freq_to_pricer_freq(freq: infra_domain::time::Frequency) -> Frequency {
    use infra_domain::time::Frequency as InfraFreq;

    match freq {
        InfraFreq::Daily => Frequency::Daily,
        InfraFreq::Weekly => Frequency::Weekly,
        InfraFreq::Monthly => Frequency::Monthly,
        InfraFreq::Quarterly => Frequency::Quarterly,
        InfraFreq::SemiAnnual => Frequency::SemiAnnual,
        InfraFreq::Annual => Frequency::Annual,
    }
}

#[cfg(test)]
mod tests {
    use infra_domain::market::Currency;

    use super::*;

    #[test]
    fn test_convert_deposit() {
        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M");

        let inst = definition_to_instrument(&def, 0.05, None).unwrap();

        match inst {
            MarketInstrument::Ois { maturity, rate, .. } => {
                assert!((maturity - 0.25).abs() < 1e-6);
                assert!((rate - 0.05).abs() < 1e-10);
            }
            _ => panic!("Expected OIS for deposit"),
        }
    }

    #[test]
    fn test_convert_ois() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");

        let inst = definition_to_instrument(&def, 0.04, None).unwrap();

        match inst {
            MarketInstrument::Ois { maturity, rate, .. } => {
                assert!((maturity - 5.0).abs() < 1e-6);
                assert!((rate - 0.04).abs() < 1e-10);
            }
            _ => panic!("Expected OIS"),
        }
    }

    #[test]
    fn test_convert_swap() {
        let def = InstrumentDefinition::new("USD-Swap-10Y", Currency::USD, RateType::Swap, "10Y");

        let inst = definition_to_instrument(&def, 0.035, None).unwrap();

        match inst {
            MarketInstrument::Irs {
                maturity,
                rate,
                fixed_frequency,
            } => {
                assert!((maturity - 10.0).abs() < 1e-6);
                assert!((rate - 0.035).abs() < 1e-10);
                assert_eq!(fixed_frequency, Frequency::SemiAnnual);
            }
            _ => panic!("Expected IRS"),
        }
    }

    #[test]
    fn test_convert_fra() {
        let def = InstrumentDefinition::new("USD-FRA-3x6", Currency::USD, RateType::Fra, "3x6");

        let inst = definition_to_instrument(&def, 0.045, None).unwrap();

        match inst {
            MarketInstrument::Fra { start, end, rate } => {
                assert!((start - 0.25).abs() < 1e-6);
                assert!((end - 0.5).abs() < 1e-6);
                assert!((rate - 0.045).abs() < 1e-10);
            }
            _ => panic!("Expected FRA"),
        }
    }

    #[test]
    fn test_convert_futures() {
        let def = InstrumentDefinition::new("USD-Fut-3M", Currency::USD, RateType::Futures, "3M");

        let inst = definition_to_instrument(&def, 0.042, None).unwrap();

        match inst {
            MarketInstrument::Future {
                maturity,
                rate,
                convexity_adjustment,
            } => {
                assert!((maturity - 0.25).abs() < 1e-6);
                assert!((rate - 0.042).abs() < 1e-10);
                assert!((convexity_adjustment - 0.0).abs() < 1e-10);
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_convert_unsupported_rate_type() {
        let def = InstrumentDefinition::new("USD-Vol-1Y", Currency::USD, RateType::Vol, "1Y");

        let result = definition_to_instrument(&def, 0.2, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            ConstructionError::UnsupportedRateType { rate_type } => {
                assert_eq!(rate_type, RateType::Vol);
            }
            _ => panic!("Expected UnsupportedRateType error"),
        }
    }

    #[test]
    fn test_convert_fra_without_fra_tenor() {
        // FRA with standard tenor (not 3x6 format) - should fail
        let def = InstrumentDefinition::new("USD-FRA-6M", Currency::USD, RateType::Fra, "6M");

        let result = definition_to_instrument(&def, 0.045, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            ConstructionError::TenorParseError { tenor, .. } => {
                assert_eq!(tenor, "6M");
            }
            _ => panic!("Expected TenorParseError"),
        }
    }

    #[test]
    fn test_convert_overnight_deposit() {
        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "ON");

        let inst = definition_to_instrument(&def, 0.055, None).unwrap();

        match inst {
            MarketInstrument::Ois { maturity, rate, .. } => {
                // ON is approximately 1/365 years
                assert!((maturity - 1.0 / 365.0).abs() < 1e-6);
                assert!((rate - 0.055).abs() < 1e-10);
            }
            _ => panic!("Expected OIS"),
        }
    }

    #[test]
    fn test_convert_event() {
        // Create an Event instrument using from_event constructor
        let def = InstrumentDefinition::from_event(
            "USD-FOMC-2024-03",
            Currency::USD,
            "2024-03-20",
            "USD-SOFR",
        );

        // Reference date is 2024-01-01
        let reference_date = Some((2024, 1, 1));

        // Rate is 25bp expected hike
        let inst = definition_to_instrument(&def, 25.0, reference_date).unwrap();

        match inst {
            MarketInstrument::Event {
                maturity,
                expected_jump,
            } => {
                // March 20, 2024 is about 79 days from Jan 1, 2024
                // 79 / 365.25 ≈ 0.216
                assert!((maturity - 0.216).abs() < 0.01, "maturity was {}", maturity);
                // 25bp = 0.0025
                assert!((expected_jump - 0.0025).abs() < 1e-10);
            }
            _ => panic!("Expected Event"),
        }
    }

    #[test]
    fn test_convert_event_missing_reference_date() {
        let def = InstrumentDefinition::from_event(
            "USD-FOMC-2024-03",
            Currency::USD,
            "2024-03-20",
            "USD-SOFR",
        );

        // No reference date
        let result = definition_to_instrument(&def, 25.0, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            ConstructionError::InvalidConfig { message } => {
                assert!(message.contains("reference_date"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_date_to_years_calculation() {
        // Test the date parsing function directly
        let ref_date = (2024, 1, 1);

        // Same day should be 0
        let maturity = parse_date_to_years("2024-01-01", ref_date).unwrap();
        assert!((maturity - 0.0).abs() < 1e-6);

        // One year later
        let maturity = parse_date_to_years("2025-01-01", ref_date).unwrap();
        assert!((maturity - 1.0).abs() < 0.01);

        // 6 months later (approximately)
        let maturity = parse_date_to_years("2024-07-01", ref_date).unwrap();
        assert!((maturity - 0.5).abs() < 0.02);
    }
}

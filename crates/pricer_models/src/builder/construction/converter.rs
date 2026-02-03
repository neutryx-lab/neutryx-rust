//! Definition to MarketInstrument converter.
//!
//! This module converts `InstrumentDefinition` from infra_master to
//! `MarketInstrument<f64>` used by the pricer_models calibration engine.
//!
//! Note: Currently uses `f64` as the bootstrapper only supports `f64`.

use infra_master::market::{InstrumentDefinition, RateType};

use crate::market::curves::{Frequency, MarketInstrument};

use super::error::ConstructionError;

/// Converts an `InstrumentDefinition` and rate value to a `MarketInstrument<f64>`.
///
/// # Arguments
///
/// * `def` - The instrument definition from infra_master
/// * `rate` - The market rate value
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
pub fn definition_to_instrument(
    def: &InstrumentDefinition,
    rate: f64,
) -> Result<MarketInstrument<f64>, ConstructionError> {
    match def.rate_type {
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
            let (start_years, end_years) = def.fra_tenors().ok_or_else(|| {
                ConstructionError::TenorParseError {
                    tenor: def.tenor.clone(),
                    message: "FRA requires start x end tenor format (e.g., '3x6')".to_string(),
                }
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

        // These rate types are not directly mappable to calibration instruments
        other => Err(ConstructionError::UnsupportedRateType { rate_type: other }),
    }
}

/// Converts infra_master's Frequency to pricer_models's Frequency.
fn infra_freq_to_pricer_freq(freq: infra_master::time::Frequency) -> Frequency {
    use infra_master::time::Frequency as InfraFreq;

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
    use super::*;
    use infra_master::market::Currency;

    #[test]
    fn test_convert_deposit() {
        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M");

        let inst = definition_to_instrument(&def, 0.05).unwrap();

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

        let inst = definition_to_instrument(&def, 0.04).unwrap();

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

        let inst = definition_to_instrument(&def, 0.035).unwrap();

        match inst {
            MarketInstrument::Irs {
                maturity, rate, fixed_frequency,
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

        let inst = definition_to_instrument(&def, 0.045).unwrap();

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

        let inst = definition_to_instrument(&def, 0.042).unwrap();

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

        let result = definition_to_instrument(&def, 0.2);

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

        let result = definition_to_instrument(&def, 0.045);

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
        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N");

        let inst = definition_to_instrument(&def, 0.055).unwrap();

        match inst {
            MarketInstrument::Ois { maturity, rate, .. } => {
                // O/N is approximately 1/365 years
                assert!((maturity - 1.0 / 365.0).abs() < 1e-6);
                assert!((rate - 0.055).abs() < 1e-10);
            }
            _ => panic!("Expected OIS"),
        }
    }
}

//! Instrument parsing utilities for curve building.

use pricer_models::market::curves::MarketInstrument;

use crate::vol_surface_loader::{parse_fra_tenor, parse_tenor_string};

/// Raw instrument specification for curve building.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentSpec {
    /// Instrument type (e.g., "deposit", "ois", "fra", "swap", "future",.
    pub instrument_type: String,
    /// Tenor string (e.g., "1M", "1Y", "3x6" for FRA).
    pub tenor: String,
    /// Market-quoted rate (as decimal, e.g., 0.0430 for 4.30%).
    pub rate: f64,
    /// Event date in ISO format (YYYY-MM-DD).
    pub event_date: Option<String>,
    /// Expected rate spike for Event instruments (as decimal, e.g., -0.0025
    /// for.
    pub expected_rate_spike: Option<f64>,
    /// Coupon rate for Bond instruments (as decimal, e.g., 0.04 for 4%).
    pub coupon_rate: Option<f64>,
}

impl InstrumentSpec {
    /// Create a new instrument specification.
    pub fn new(instrument_type: impl Into<String>, tenor: impl Into<String>, rate: f64) -> Self {
        Self {
            instrument_type: instrument_type.into(),
            tenor: tenor.into(),
            rate,
            event_date: None,
            expected_rate_spike: None,
            coupon_rate: None,
        }
    }

    /// Create an Event instrument specification.
    pub fn event(event_date: impl Into<String>, expected_rate_spike: f64) -> Self {
        Self {
            instrument_type: "event".to_string(),
            tenor: String::new(),
            rate: expected_rate_spike,
            event_date: Some(event_date.into()),
            expected_rate_spike: Some(expected_rate_spike),
            coupon_rate: None,
        }
    }

    /// Parse the tenor string into year fraction.
    pub fn tenor_years(&self) -> Result<f64, InstrumentParseError> {
        if self.is_event() {
            return Ok(0.0);
        }

        if self.is_fra() {
            if let Some((_, end)) = parse_fra_tenor(&self.tenor) {
                return Ok(end);
            }
        }

        parse_tenor_string(&self.tenor).map_err(|e| InstrumentParseError::InvalidTenor {
            tenor: self.tenor.clone(),
            reason: e,
        })
    }

    /// Check if this instrument is a FRA.
    fn is_fra(&self) -> bool {
        let t = self.instrument_type.to_lowercase();
        t == "fra"
    }

    /// Check if this instrument is an Event.
    fn is_event(&self) -> bool {
        let t = self.instrument_type.to_lowercase();
        t == "event"
    }

    /// Convert to a `MarketInstrument` for curve calibration.
    pub fn to_market_instrument(&self) -> Result<MarketInstrument<f64>, InstrumentParseError> {
        let instrument_type = self.instrument_type.to_lowercase();

        match instrument_type.as_str() {
            "deposit" | "depo" => {
                let tenor_years = self.tenor_years()?;
                Ok(MarketInstrument::ois(tenor_years, self.rate))
            }
            "ois" => {
                let tenor_years = self.tenor_years()?;
                Ok(MarketInstrument::ois(tenor_years, self.rate))
            }
            "swap" | "irs" => {
                let tenor_years = self.tenor_years()?;
                Ok(MarketInstrument::irs(tenor_years, self.rate))
            }
            "fra" => {
                if let Some((start, end)) = parse_fra_tenor(&self.tenor) {
                    Ok(MarketInstrument::fra(start, end, self.rate))
                } else {
                    let tenor_years = parse_tenor_string(&self.tenor).map_err(|e| {
                        InstrumentParseError::InvalidTenor {
                            tenor: self.tenor.clone(),
                            reason: e,
                        }
                    })?;
                    Ok(MarketInstrument::fra(0.0, tenor_years, self.rate))
                }
            }
            "future" | "futures" => {
                let tenor_years = self.tenor_years()?;
                Ok(MarketInstrument::future(tenor_years, self.rate))
            }
            "event" => {
                let spike = self.expected_rate_spike.unwrap_or(self.rate);
                Ok(MarketInstrument::event_with_rate(0.0, spike))
            }
            "bond" => {
                let tenor_years = self.tenor_years()?;
                let coupon_rate =
                    self.coupon_rate
                        .ok_or_else(|| InstrumentParseError::InvalidRate {
                            rate: self.rate,
                            reason: "Bond requires coupon_rate field".to_string(),
                        })?;
                Ok(MarketInstrument::bond(tenor_years, self.rate, coupon_rate))
            }
            _ => Err(InstrumentParseError::UnknownType {
                instrument_type: self.instrument_type.clone(),
            }),
        }
    }
}

/// Error during instrument parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentParseError {
    /// Invalid tenor format.
    InvalidTenor {
        /// The tenor string that failed to parse.
        tenor: String,
        /// Reason for the failure.
        reason: String,
    },
    /// Unknown instrument type.
    UnknownType {
        /// The unrecognised instrument type.
        instrument_type: String,
    },
    /// Invalid rate value.
    InvalidRate {
        /// The rate value.
        rate: f64,
        /// Reason for the failure.
        reason: String,
    },
    /// No instruments provided.
    EmptyInstruments,
}

impl std::fmt::Display for InstrumentParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTenor { tenor, reason } => {
                write!(f, "Invalid tenor '{}': {}", tenor, reason)
            }
            Self::UnknownType { instrument_type } => {
                write!(f, "Unknown instrument type: {}", instrument_type)
            }
            Self::InvalidRate { rate, reason } => {
                write!(f, "Invalid rate {}: {}", rate, reason)
            }
            Self::EmptyInstruments => {
                write!(f, "At least one instrument is required")
            }
        }
    }
}

impl std::error::Error for InstrumentParseError {}

/// Parse a collection of instrument specifications into `MarketInstrument`.
pub fn parse_instruments(
    specs: &[InstrumentSpec],
) -> Result<Vec<MarketInstrument<f64>>, InstrumentParseError> {
    if specs.is_empty() {
        return Err(InstrumentParseError::EmptyInstruments);
    }

    let mut instruments_with_tenors: Vec<(MarketInstrument<f64>, f64)> =
        Vec::with_capacity(specs.len());

    for spec in specs {
        let instrument = spec.to_market_instrument()?;
        let tenor = spec.tenor_years()?;
        instruments_with_tenors.push((instrument, tenor));
    }

    instruments_with_tenors
        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(instruments_with_tenors
        .into_iter()
        .map(|(i, _)| i)
        .collect())
}

/// Validate instrument rate is within acceptable range.
pub fn validate_rate(rate: f64, min_rate: f64, max_rate: f64) -> Result<(), InstrumentParseError> {
    if rate < min_rate || rate > max_rate {
        return Err(InstrumentParseError::InvalidRate {
            rate,
            reason: format!("Rate must be between {} and {}", min_rate, max_rate),
        });
    }
    Ok(())
}

/// Validate all instrument rates in a collection.
pub fn validate_rates(
    specs: &[InstrumentSpec],
    min_rate: f64,
    max_rate: f64,
) -> Result<(), InstrumentParseError> {
    for spec in specs {
        validate_rate(spec.rate, min_rate, max_rate)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_spec_deposit() {
        let spec = InstrumentSpec::new("deposit", "1M", 0.0430);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Ois { maturity, rate, .. } => {
                assert!((maturity - 1.0 / 12.0).abs() < 1e-10);
                assert!((rate - 0.0430).abs() < 1e-10);
            }
            _ => panic!("Expected OIS instrument for deposit"),
        }
    }

    #[test]
    fn test_instrument_spec_fra_nxm() {
        let spec = InstrumentSpec::new("fra", "3x6", 0.0405);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Fra { start, end, rate } => {
                assert!((start - 0.25).abs() < 1e-10);
                assert!((end - 0.5).abs() < 1e-10);
                assert!((rate - 0.0405).abs() < 1e-10);
            }
            _ => panic!("Expected FRA instrument"),
        }
    }

    #[test]
    fn test_instrument_spec_fra_fallback() {
        let spec = InstrumentSpec::new("fra", "6M", 0.0400);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Fra { start, end, rate } => {
                assert!((start - 0.0).abs() < 1e-10);
                assert!((end - 0.5).abs() < 1e-10);
                assert!((rate - 0.0400).abs() < 1e-10);
            }
            _ => panic!("Expected FRA instrument"),
        }
    }

    #[test]
    fn test_instrument_spec_ois() {
        let spec = InstrumentSpec::new("ois", "1Y", 0.0358);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Ois { maturity, rate, .. } => {
                assert!((maturity - 1.0).abs() < 1e-10);
                assert!((rate - 0.0358).abs() < 1e-10);
            }
            _ => panic!("Expected OIS instrument"),
        }
    }

    #[test]
    fn test_instrument_spec_swap() {
        let spec = InstrumentSpec::new("swap", "5Y", 0.0342);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Irs { maturity, rate, .. } => {
                assert!((maturity - 5.0).abs() < 1e-10);
                assert!((rate - 0.0342).abs() < 1e-10);
            }
            _ => panic!("Expected IRS instrument"),
        }
    }

    #[test]
    fn test_instrument_spec_future() {
        let spec = InstrumentSpec::new("future", "3M", 0.0415);
        let instrument = spec.to_market_instrument().unwrap();

        match instrument {
            MarketInstrument::Future { maturity, rate, .. } => {
                assert!((maturity - 0.25).abs() < 1e-10);
                assert!((rate - 0.0415).abs() < 1e-10);
            }
            _ => panic!("Expected Future instrument"),
        }
    }

    #[test]
    fn test_instrument_spec_unknown_type() {
        let spec = InstrumentSpec::new("unknown", "1Y", 0.05);
        let result = spec.to_market_instrument();
        assert!(result.is_err());
        match result.unwrap_err() {
            InstrumentParseError::UnknownType { instrument_type } => {
                assert_eq!(instrument_type, "unknown");
            }
            _ => panic!("Expected UnknownType error"),
        }
    }

    #[test]
    fn test_parse_instruments_sorted() {
        let specs = vec![
            InstrumentSpec::new("ois", "1Y", 0.0358),
            InstrumentSpec::new("deposit", "1M", 0.0430),
            InstrumentSpec::new("fra", "3x6", 0.0405),
        ];

        let instruments = parse_instruments(&specs).unwrap();
        assert_eq!(instruments.len(), 3);
    }

    #[test]
    fn test_parse_instruments_empty() {
        let specs: Vec<InstrumentSpec> = vec![];
        let result = parse_instruments(&specs);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InstrumentParseError::EmptyInstruments
        ));
    }

    #[test]
    fn test_validate_rate_valid() {
        assert!(validate_rate(0.05, -0.10, 0.50).is_ok());
        assert!(validate_rate(-0.05, -0.10, 0.50).is_ok());
        assert!(validate_rate(0.0, -0.10, 0.50).is_ok());
    }

    #[test]
    fn test_validate_rate_invalid() {
        assert!(validate_rate(0.60, -0.10, 0.50).is_err());
        assert!(validate_rate(-0.15, -0.10, 0.50).is_err());
    }
}

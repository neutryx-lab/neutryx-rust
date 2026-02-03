//! Construction error types for curve building.
//!
//! This module provides error types for the curve construction engine.

use thiserror::Error;

use infra_master::market::{CurveDefError, InstrumentDefError, RateIndexDefError, RegistryError, RateType};

/// Errors that can occur during curve construction.
#[derive(Debug, Clone, Error)]
pub enum ConstructionError {
    /// Registry error during validation or lookup.
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    /// Curve definition error.
    #[error("Curve definition error: {0}")]
    CurveDef(#[from] CurveDefError),

    /// Instrument definition error.
    #[error("Instrument definition error: {0}")]
    InstrumentDef(#[from] InstrumentDefError),

    /// Rate index definition error.
    #[error("Rate index definition error: {0}")]
    RateIndexDef(#[from] RateIndexDefError),

    /// Curve not found in registry.
    #[error("Curve not found: {curve_name}")]
    CurveNotFound {
        /// Name of the missing curve.
        curve_name: String,
    },

    /// Missing market rate for an instrument.
    #[error("Missing rate for instrument: {instrument_id}")]
    MissingRate {
        /// Instrument ID that has no corresponding rate.
        instrument_id: String,
    },

    /// Multiple missing rates.
    #[error("Missing rates for {count} instruments: {}", instrument_ids.join(", "))]
    MissingRates {
        /// Number of missing rates.
        count: usize,
        /// List of instrument IDs with missing rates.
        instrument_ids: Vec<String>,
    },

    /// No instruments available for calibration.
    #[error("No instruments available for curve: {curve_name}")]
    NoInstruments {
        /// Curve name with no instruments.
        curve_name: String,
    },

    /// Unsupported rate type for conversion.
    #[error("Unsupported rate type: {rate_type:?}")]
    UnsupportedRateType {
        /// The unsupported rate type.
        rate_type: RateType,
    },

    /// Calibration (bootstrap) failed.
    #[error("Calibration failed: {message}")]
    CalibrationFailed {
        /// Error message from the bootstrapper.
        message: String,
    },

    /// Convergence failure during calibration.
    #[error("Convergence failed: residual {residual} after {iterations} iterations")]
    ConvergenceFailed {
        /// Final residual.
        residual: f64,
        /// Number of iterations performed.
        iterations: usize,
    },

    /// Invalid configuration.
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the invalid configuration.
        message: String,
    },

    /// Tenor parsing error.
    #[error("Failed to parse tenor '{tenor}': {message}")]
    TenorParseError {
        /// The tenor string that failed to parse.
        tenor: String,
        /// Error message.
        message: String,
    },
}

impl ConstructionError {
    /// Creates a curve not found error.
    #[must_use]
    pub fn curve_not_found(curve_name: impl Into<String>) -> Self {
        Self::CurveNotFound {
            curve_name: curve_name.into(),
        }
    }

    /// Creates a missing rate error.
    #[must_use]
    pub fn missing_rate(instrument_id: impl Into<String>) -> Self {
        Self::MissingRate {
            instrument_id: instrument_id.into(),
        }
    }

    /// Creates a missing rates error from a list of IDs.
    #[must_use]
    pub fn missing_rates(instrument_ids: Vec<String>) -> Self {
        Self::MissingRates {
            count: instrument_ids.len(),
            instrument_ids,
        }
    }

    /// Creates a no instruments error.
    #[must_use]
    pub fn no_instruments(curve_name: impl Into<String>) -> Self {
        Self::NoInstruments {
            curve_name: curve_name.into(),
        }
    }

    /// Creates a calibration failed error.
    #[must_use]
    pub fn calibration_failed(message: impl Into<String>) -> Self {
        Self::CalibrationFailed {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_not_found_error() {
        let err = ConstructionError::curve_not_found("USD-SOFR");
        assert!(err.to_string().contains("USD-SOFR"));
    }

    #[test]
    fn test_missing_rate_error() {
        let err = ConstructionError::missing_rate("USD-OIS-5Y");
        assert!(err.to_string().contains("USD-OIS-5Y"));
    }

    #[test]
    fn test_missing_rates_error() {
        let err = ConstructionError::missing_rates(vec![
            "USD-OIS-5Y".to_string(),
            "USD-OIS-10Y".to_string(),
        ]);
        assert!(err.to_string().contains("2 instruments"));
        assert!(err.to_string().contains("USD-OIS-5Y"));
        assert!(err.to_string().contains("USD-OIS-10Y"));
    }

    #[test]
    fn test_no_instruments_error() {
        let err = ConstructionError::no_instruments("EUR-ESTR");
        assert!(err.to_string().contains("EUR-ESTR"));
    }

    #[test]
    fn test_calibration_failed_error() {
        let err = ConstructionError::calibration_failed("Newton-Raphson diverged");
        assert!(err.to_string().contains("Newton-Raphson diverged"));
    }
}

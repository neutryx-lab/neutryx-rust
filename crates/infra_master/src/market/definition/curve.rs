//! Curve definition for yield curve construction.
//!
//! This module provides [`CurveDefinition`] which defines the recipe for
//! building a yield curve. It references [`InstrumentDefinition`]s by ID
//! to specify which instruments to use for calibration.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::definition::CurveDefinition;
//!
//! let curve = CurveDefinition::new(
//!     "USD-SOFR-Discount",
//!     "USD-SOFR",
//!     vec![
//!         "USD-Depo-ON".to_string(),
//!         "USD-Depo-1M".to_string(),
//!         "USD-OIS-1Y".to_string(),
//!         "USD-OIS-5Y".to_string(),
//!     ],
//! );
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Curve definition - the recipe for building a yield curve.
///
/// References [`InstrumentDefinition`]s and [`RateIndexDefinition`] by their IDs
/// to specify which instruments and index to use for curve construction.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CurveDefinition {
    /// Curve name (e.g., "USD-SOFR-Discount")
    pub name: String,

    /// Reference to RateIndexDefinition.id (e.g., "USD-SOFR")
    pub rate_index: String,

    /// List of InstrumentDefinition.id references
    pub instruments: Vec<String>,

    /// Calibration method
    #[cfg_attr(feature = "serde", serde(default))]
    pub calibration_method: CalibrationMethod,

    /// Interpolation method for the resulting curve
    #[cfg_attr(feature = "serde", serde(default))]
    pub interpolation: InterpolationMethod,

    /// Whether to allow extrapolation beyond the last pillar
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub allow_extrapolation: bool,
}

fn default_true() -> bool {
    true
}

/// Calibration method for curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum CalibrationMethod {
    /// Sequential bootstrapping (pillar-by-pillar)
    #[default]
    Sequential,
    /// Global calibration (all pillars simultaneously)
    Global,
}

/// Interpolation method for the resulting yield curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum InterpolationMethod {
    /// Linear interpolation on discount factors
    Linear,
    /// Log-linear interpolation (default, preserves no-arbitrage)
    #[default]
    LogLinear,
    /// Cubic spline interpolation
    CubicSpline,
}

/// Error type for curve definition validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveDefError {
    /// Missing required field
    MissingField(&'static str),
    /// No instruments specified
    NoInstruments,
    /// Unknown rate index reference
    UnknownRateIndex(String),
    /// Unknown instrument reference
    UnknownInstrument(String),
    /// Duplicate instrument in the list
    DuplicateInstrument(String),
}

impl std::fmt::Display for CurveDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::NoInstruments => write!(f, "No instruments specified"),
            Self::UnknownRateIndex(id) => write!(f, "Unknown rate index: {}", id),
            Self::UnknownInstrument(id) => write!(f, "Unknown instrument: {}", id),
            Self::DuplicateInstrument(id) => write!(f, "Duplicate instrument: {}", id),
        }
    }
}

impl std::error::Error for CurveDefError {}

impl CurveDefinition {
    /// Creates a new curve definition.
    ///
    /// # Arguments
    ///
    /// * `name` - Curve name
    /// * `rate_index` - Reference to RateIndexDefinition ID
    /// * `instruments` - List of InstrumentDefinition IDs
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        rate_index: impl Into<String>,
        instruments: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            rate_index: rate_index.into(),
            instruments,
            calibration_method: CalibrationMethod::default(),
            interpolation: InterpolationMethod::default(),
            allow_extrapolation: true,
        }
    }

    /// Sets the calibration method.
    #[must_use]
    pub fn with_calibration_method(mut self, method: CalibrationMethod) -> Self {
        self.calibration_method = method;
        self
    }

    /// Sets the interpolation method.
    #[must_use]
    pub fn with_interpolation(mut self, interpolation: InterpolationMethod) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets whether extrapolation is allowed.
    #[must_use]
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Adds an instrument to the definition.
    #[must_use]
    pub fn with_instrument(mut self, instrument_id: impl Into<String>) -> Self {
        self.instruments.push(instrument_id.into());
        self
    }

    /// Validates the curve definition (basic validation only).
    ///
    /// For full validation including reference checking, use
    /// [`DefinitionRegistry::register_curve`].
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), CurveDefError> {
        if self.name.is_empty() {
            return Err(CurveDefError::MissingField("name"));
        }

        if self.rate_index.is_empty() {
            return Err(CurveDefError::MissingField("rate_index"));
        }

        if self.instruments.is_empty() {
            return Err(CurveDefError::NoInstruments);
        }

        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        for inst_id in &self.instruments {
            if !seen.insert(inst_id) {
                return Err(CurveDefError::DuplicateInstrument(inst_id.clone()));
            }
        }

        Ok(())
    }

    /// Returns the number of instruments in the definition.
    #[must_use]
    pub fn instrument_count(&self) -> usize {
        self.instruments.len()
    }
}

impl CalibrationMethod {
    /// Returns the display name of the calibration method.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential Bootstrapping",
            Self::Global => "Global Calibration",
        }
    }
}

impl InterpolationMethod {
    /// Returns the display name of the interpolation method.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::LogLinear => "Log-Linear",
            Self::CubicSpline => "Cubic Spline",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_definition_new() {
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-OIS-5Y".to_string()],
        );

        assert_eq!(curve.name, "USD-SOFR-Discount");
        assert_eq!(curve.rate_index, "USD-SOFR");
        assert_eq!(curve.instruments.len(), 2);
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
    }

    #[test]
    fn test_curve_definition_builder() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec![])
            .with_instrument("USD-Depo-ON")
            .with_instrument("USD-OIS-5Y")
            .with_calibration_method(CalibrationMethod::Global)
            .with_interpolation(InterpolationMethod::CubicSpline)
            .with_extrapolation(false);

        assert_eq!(curve.instruments.len(), 2);
        assert_eq!(curve.calibration_method, CalibrationMethod::Global);
        assert_eq!(curve.interpolation, InterpolationMethod::CubicSpline);
        assert!(!curve.allow_extrapolation);
    }

    #[test]
    fn test_validate_success() {
        let curve = CurveDefinition::new(
            "USD-SOFR",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string()],
        );
        assert!(curve.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let curve = CurveDefinition::new("", "USD-SOFR", vec!["USD-Depo-ON".to_string()]);
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::MissingField("name"))
        ));
    }

    #[test]
    fn test_validate_empty_rate_index() {
        let curve = CurveDefinition::new("USD-SOFR", "", vec!["USD-Depo-ON".to_string()]);
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::MissingField("rate_index"))
        ));
    }

    #[test]
    fn test_validate_no_instruments() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec![]);
        assert!(matches!(curve.validate(), Err(CurveDefError::NoInstruments)));
    }

    #[test]
    fn test_validate_duplicate_instrument() {
        let curve = CurveDefinition::new(
            "USD-SOFR",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-Depo-ON".to_string()],
        );
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::DuplicateInstrument(_))
        ));
    }

    #[test]
    fn test_instrument_count() {
        let curve = CurveDefinition::new(
            "USD-SOFR",
            "USD-SOFR",
            vec![
                "USD-Depo-ON".to_string(),
                "USD-OIS-1Y".to_string(),
                "USD-OIS-5Y".to_string(),
            ],
        );
        assert_eq!(curve.instrument_count(), 3);
    }

    #[test]
    fn test_calibration_method_defaults() {
        assert_eq!(CalibrationMethod::default(), CalibrationMethod::Sequential);
    }

    #[test]
    fn test_interpolation_method_defaults() {
        assert_eq!(InterpolationMethod::default(), InterpolationMethod::LogLinear);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-OIS-5Y".to_string()],
        )
        .with_calibration_method(CalibrationMethod::Global);

        let json = serde_json::to_string(&curve).unwrap();
        let parsed: CurveDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(curve, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json() {
        let json = r#"{
            "name": "USD-SOFR-Discount",
            "rateIndex": "USD-SOFR",
            "instruments": ["USD-Depo-ON", "USD-OIS-1Y", "USD-OIS-5Y"],
            "calibrationMethod": "sequential",
            "interpolation": "loglinear",
            "allowExtrapolation": true
        }"#;

        let curve: CurveDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(curve.name, "USD-SOFR-Discount");
        assert_eq!(curve.rate_index, "USD-SOFR");
        assert_eq!(curve.instruments.len(), 3);
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_defaults_applied() {
        let json = r#"{
            "name": "EUR-ESTR",
            "rateIndex": "EUR-ESTR",
            "instruments": ["EUR-Depo-ON"]
        }"#;

        let curve: CurveDefinition = serde_json::from_str(json).unwrap();
        // Defaults should be applied
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
    }
}

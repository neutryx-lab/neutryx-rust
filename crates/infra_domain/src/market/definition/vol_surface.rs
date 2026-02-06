//! Volatility surface/cube definition for vol surface construction.
//!
//! This module provides [`VolSurfaceDefinition`] which defines the recipe for
//! building a volatility surface or cube. It references calibration instruments
//! by ID to specify which instruments to use for calibration.
//!
//! # Examples
//!
//! ## Vol Surface Definition
//!
//! ```
//! use infra_domain::market::definition::{VolSurfaceDefinition, CalibrationModel, StrikeAxisType};
//!
//! let vol_surface = VolSurfaceDefinition::new(
//!     "USD-SOFR-Swaption-Vol",
//!     vec![
//!         "USD-SOFR-1Y1Y-ATM".to_string(),
//!         "USD-SOFR-5Y5Y-ATM".to_string(),
//!     ],
//! )
//! .with_model(CalibrationModel::Sabr)
//! .with_strike_axis(StrikeAxisType::Delta);
//!
//! assert_eq!(vol_surface.name, "USD-SOFR-Swaption-Vol");
//! ```
//!
//! ## Calibration Model
//!
//! ```
//! use infra_domain::market::definition::CalibrationModel;
//!
//! let model = CalibrationModel::Sabr;
//! assert!(model.is_enabled());
//! assert_eq!(model.parameter_count(), 4);
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Calibration model for volatility surfaces.
///
/// Defines the parametric model used to fit volatility smiles/surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CalibrationModel {
    /// SABR (Stochastic Alpha Beta Rho) model.
    ///
    /// Standard model for interest rate volatility surfaces.
    /// Parameters: alpha, beta, rho, nu.
    #[default]
    Sabr,
    /// SVI (Stochastic Volatility Inspired) model.
    ///
    /// Popular parametrisation for equity volatility surfaces.
    /// Parameters: a, b, rho, m, sigma.
    Svi,
    /// Dupire's local volatility model.
    ///
    /// Non-parametric, arbitrage-free model derived from option prices.
    LocalVolatility,
}

impl CalibrationModel {
    /// Get the display name for this model.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sabr => "SABR",
            Self::Svi => "SVI",
            Self::LocalVolatility => "Local Volatility",
        }
    }

    /// Get a description of this model.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sabr => "Stochastic Alpha Beta Rho - standard for rates",
            Self::Svi => "Stochastic Volatility Inspired - popular for equity",
            Self::LocalVolatility => "Dupire's local volatility - arbitrage-free",
        }
    }

    /// Check if this model is currently enabled/implemented.
    pub fn is_enabled(&self) -> bool { matches!(self, Self::Sabr) }

    /// Get the number of parameters for this model.
    pub fn parameter_count(&self) -> usize {
        match self {
            Self::Sabr => 4,            // alpha, beta, rho, nu
            Self::Svi => 5,             // a, b, rho, m, sigma
            Self::LocalVolatility => 0, // Non-parametric
        }
    }

    /// Returns all calibration model variants.
    pub fn all() -> &'static [CalibrationModel] {
        &[
            CalibrationModel::Sabr,
            CalibrationModel::Svi,
            CalibrationModel::LocalVolatility,
        ]
    }

    /// Returns only enabled/implemented models.
    pub fn enabled() -> Vec<CalibrationModel> {
        Self::all()
            .iter()
            .copied()
            .filter(|m| m.is_enabled())
            .collect()
    }
}

impl std::fmt::Display for CalibrationModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// VolSurfaceDefinition
// =============================================================================

/// Volatility surface/cube definition - the recipe for building a vol surface.
///
/// References calibration instrument IDs to specify which instruments to use
/// for surface construction. Similar to [`CurveDefinition`] for yield curves.
///
/// # Asset Class Support
///
/// Volatility surfaces can be built for multiple asset classes:
/// - **Rates**: Swaption vols, cap/floor vols
/// - **FX**: FX option vols
/// - **Equity**: Equity option vols
/// - **Commodity**: Commodity option vols
///
/// # Examples
///
/// ```
/// use infra_domain::market::definition::{VolSurfaceDefinition, CalibrationModel, StrikeAxisType};
///
/// let vol_surface = VolSurfaceDefinition::new(
///     "USD-SOFR-Swaption-Vol",
///     vec!["USD-SOFR-1Y1Y-ATM".to_string()],
/// )
/// .with_model(CalibrationModel::Sabr)
/// .with_strike_axis(StrikeAxisType::Delta);
///
/// assert_eq!(vol_surface.model, CalibrationModel::Sabr);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct VolSurfaceDefinition {
    /// Surface name (e.g., "USD-SOFR-Swaption-Vol", "EURUSD-FX-Vol")
    pub name: String,

    /// List of calibration instrument IDs
    pub instruments: Vec<String>,

    /// Calibration model (SABR, SVI, LocalVol)
    #[cfg_attr(feature = "serde", serde(default))]
    pub model: CalibrationModel,

    /// Strike axis representation
    #[cfg_attr(feature = "serde", serde(default))]
    pub strike_axis: StrikeAxisType,

    /// Time interpolation method for the surface
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_interpolation: TimeInterpolation,

    /// Strike interpolation method for the surface
    #[cfg_attr(feature = "serde", serde(default))]
    pub strike_interpolation: StrikeInterpolation,

    /// Whether to allow extrapolation beyond calibrated region
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub allow_extrapolation: bool,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

/// Time axis interpolation method for volatility surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TimeInterpolation {
    /// Linear interpolation in total variance (σ²T)
    #[default]
    LinearVariance,
    /// Flat forward variance interpolation
    FlatForward,
    /// Linear interpolation in volatility
    LinearVol,
}

impl TimeInterpolation {
    /// Returns the display name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LinearVariance => "Linear Variance",
            Self::FlatForward => "Flat Forward",
            Self::LinearVol => "Linear Volatility",
        }
    }
}

/// Strike axis interpolation method for volatility surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StrikeInterpolation {
    /// Linear interpolation
    Linear,
    /// Cubic spline interpolation
    #[default]
    CubicSpline,
    /// SABR model interpolation (smile dynamics)
    Sabr,
}

impl StrikeInterpolation {
    /// Returns the display name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::CubicSpline => "Cubic Spline",
            Self::Sabr => "SABR",
        }
    }
}

/// Error type for vol surface definition validation.
#[derive(Debug, Clone, PartialEq)]
pub enum VolSurfaceDefError {
    /// Missing required field
    MissingField(&'static str),
    /// No instruments specified
    NoInstruments,
    /// Duplicate instrument in the list
    DuplicateInstrument(String),
    /// Unknown instrument reference
    UnknownInstrument(String),
    /// Model not supported for this surface type
    UnsupportedModel(CalibrationModel),
}

impl std::fmt::Display for VolSurfaceDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::NoInstruments => write!(f, "No instruments specified"),
            Self::DuplicateInstrument(id) => write!(f, "Duplicate instrument: {}", id),
            Self::UnknownInstrument(id) => write!(f, "Unknown instrument: {}", id),
            Self::UnsupportedModel(model) => write!(f, "Unsupported model: {}", model),
        }
    }
}

impl std::error::Error for VolSurfaceDefError {}

impl VolSurfaceDefinition {
    /// Creates a new vol surface definition.
    ///
    /// # Arguments
    ///
    /// * `name` - Surface name
    /// * `instruments` - List of calibration instrument IDs
    #[must_use]
    pub fn new(name: impl Into<String>, instruments: Vec<String>) -> Self {
        Self {
            name: name.into(),
            instruments,
            model: CalibrationModel::default(),
            strike_axis: StrikeAxisType::default(),
            time_interpolation: TimeInterpolation::default(),
            strike_interpolation: StrikeInterpolation::default(),
            allow_extrapolation: true,
        }
    }

    /// Sets the calibration model.
    #[must_use]
    pub fn with_model(mut self, model: CalibrationModel) -> Self {
        self.model = model;
        self
    }

    /// Sets the strike axis type.
    #[must_use]
    pub fn with_strike_axis(mut self, axis: StrikeAxisType) -> Self {
        self.strike_axis = axis;
        self
    }

    /// Sets the time interpolation method.
    #[must_use]
    pub fn with_time_interpolation(mut self, interp: TimeInterpolation) -> Self {
        self.time_interpolation = interp;
        self
    }

    /// Sets the strike interpolation method.
    #[must_use]
    pub fn with_strike_interpolation(mut self, interp: StrikeInterpolation) -> Self {
        self.strike_interpolation = interp;
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

    /// Returns the number of instruments in the definition.
    #[must_use]
    pub fn instrument_count(&self) -> usize {
        self.instruments.len()
    }

    /// Validates the vol surface definition (basic validation only).
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), VolSurfaceDefError> {
        if self.name.is_empty() {
            return Err(VolSurfaceDefError::MissingField("name"));
        }

        if self.instruments.is_empty() {
            return Err(VolSurfaceDefError::NoInstruments);
        }

        // Check for duplicate instruments
        let mut seen = std::collections::HashSet::new();
        for inst_id in &self.instruments {
            if !seen.insert(inst_id) {
                return Err(VolSurfaceDefError::DuplicateInstrument(inst_id.clone()));
            }
        }

        // Check model is enabled
        if !self.model.is_enabled() {
            return Err(VolSurfaceDefError::UnsupportedModel(self.model));
        }

        Ok(())
    }
}

/// Strike axis type for volatility surface display.
///
/// Defines how the strike axis is represented in visualisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StrikeAxisType {
    /// Absolute strike values.
    #[default]
    Absolute,
    /// Moneyness (K/F).
    Moneyness,
    /// Log-moneyness (ln(K/F)).
    LogMoneyness,
    /// Delta (option delta).
    Delta,
}

impl StrikeAxisType {
    /// Get the display name for this axis type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Absolute => "Absolute Strike",
            Self::Moneyness => "Moneyness (K/F)",
            Self::LogMoneyness => "Log-Moneyness",
            Self::Delta => "Delta",
        }
    }

    /// Returns all strike axis type variants.
    pub fn all() -> &'static [StrikeAxisType] {
        &[
            StrikeAxisType::Absolute,
            StrikeAxisType::Moneyness,
            StrikeAxisType::LogMoneyness,
            StrikeAxisType::Delta,
        ]
    }
}

impl std::fmt::Display for StrikeAxisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_model_default() {
        assert_eq!(CalibrationModel::default(), CalibrationModel::Sabr);
    }

    #[test]
    fn test_display_name() {
        assert_eq!(CalibrationModel::Sabr.display_name(), "SABR");
        assert_eq!(CalibrationModel::Svi.display_name(), "SVI");
    }

    #[test]
    fn test_is_enabled() {
        assert!(CalibrationModel::Sabr.is_enabled());
        assert!(!CalibrationModel::Svi.is_enabled());
        assert!(!CalibrationModel::LocalVolatility.is_enabled());
    }

    #[test]
    fn test_parameter_count() {
        assert_eq!(CalibrationModel::Sabr.parameter_count(), 4);
        assert_eq!(CalibrationModel::Svi.parameter_count(), 5);
        assert_eq!(CalibrationModel::LocalVolatility.parameter_count(), 0);
    }

    #[test]
    fn test_enabled() {
        let enabled = CalibrationModel::enabled();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0], CalibrationModel::Sabr);
    }

    #[test]
    fn test_strike_axis_default() {
        assert_eq!(StrikeAxisType::default(), StrikeAxisType::Absolute);
    }

    #[test]
    fn test_strike_axis_display() {
        assert_eq!(StrikeAxisType::Delta.display_name(), "Delta");
    }

    // =========================================================================
    // VolSurfaceDefinition Tests
    // =========================================================================

    #[test]
    fn test_vol_surface_definition_new() {
        let surface = VolSurfaceDefinition::new(
            "USD-SOFR-Swaption-Vol",
            vec!["USD-SOFR-1Y1Y-ATM".to_string()],
        );

        assert_eq!(surface.name, "USD-SOFR-Swaption-Vol");
        assert_eq!(surface.instruments.len(), 1);
        assert_eq!(surface.model, CalibrationModel::Sabr);
        assert_eq!(surface.strike_axis, StrikeAxisType::Absolute);
        assert!(surface.allow_extrapolation);
    }

    #[test]
    fn test_vol_surface_definition_builder() {
        let surface = VolSurfaceDefinition::new("FX-EURUSD-Vol", vec![])
            .with_instrument("EURUSD-1M-25D-RR")
            .with_instrument("EURUSD-1M-ATM")
            .with_model(CalibrationModel::Sabr)
            .with_strike_axis(StrikeAxisType::Delta)
            .with_time_interpolation(TimeInterpolation::FlatForward)
            .with_strike_interpolation(StrikeInterpolation::Sabr)
            .with_extrapolation(false);

        assert_eq!(surface.instruments.len(), 2);
        assert_eq!(surface.model, CalibrationModel::Sabr);
        assert_eq!(surface.strike_axis, StrikeAxisType::Delta);
        assert_eq!(surface.time_interpolation, TimeInterpolation::FlatForward);
        assert_eq!(surface.strike_interpolation, StrikeInterpolation::Sabr);
        assert!(!surface.allow_extrapolation);
    }

    #[test]
    fn test_vol_surface_validate_success() {
        let surface = VolSurfaceDefinition::new(
            "USD-SOFR-Vol",
            vec!["USD-SOFR-1Y1Y-ATM".to_string()],
        );
        assert!(surface.validate().is_ok());
    }

    #[test]
    fn test_vol_surface_validate_empty_name() {
        let surface = VolSurfaceDefinition::new("", vec!["inst1".to_string()]);
        assert!(matches!(
            surface.validate(),
            Err(VolSurfaceDefError::MissingField("name"))
        ));
    }

    #[test]
    fn test_vol_surface_validate_no_instruments() {
        let surface = VolSurfaceDefinition::new("USD-Vol", vec![]);
        assert!(matches!(
            surface.validate(),
            Err(VolSurfaceDefError::NoInstruments)
        ));
    }

    #[test]
    fn test_vol_surface_validate_duplicate_instrument() {
        let surface = VolSurfaceDefinition::new(
            "USD-Vol",
            vec!["inst1".to_string(), "inst1".to_string()],
        );
        assert!(matches!(
            surface.validate(),
            Err(VolSurfaceDefError::DuplicateInstrument(_))
        ));
    }

    #[test]
    fn test_vol_surface_validate_unsupported_model() {
        let surface = VolSurfaceDefinition::new("USD-Vol", vec!["inst1".to_string()])
            .with_model(CalibrationModel::Svi);
        assert!(matches!(
            surface.validate(),
            Err(VolSurfaceDefError::UnsupportedModel(_))
        ));
    }

    #[test]
    fn test_time_interpolation_name() {
        assert_eq!(TimeInterpolation::LinearVariance.name(), "Linear Variance");
        assert_eq!(TimeInterpolation::FlatForward.name(), "Flat Forward");
        assert_eq!(TimeInterpolation::LinearVol.name(), "Linear Volatility");
    }

    #[test]
    fn test_strike_interpolation_name() {
        assert_eq!(StrikeInterpolation::Linear.name(), "Linear");
        assert_eq!(StrikeInterpolation::CubicSpline.name(), "Cubic Spline");
        assert_eq!(StrikeInterpolation::Sabr.name(), "SABR");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_vol_surface_serde_roundtrip() {
        let surface = VolSurfaceDefinition::new(
            "USD-SOFR-Vol",
            vec!["USD-SOFR-1Y1Y-ATM".to_string()],
        )
        .with_model(CalibrationModel::Sabr)
        .with_strike_axis(StrikeAxisType::Delta);

        let json = serde_json::to_string(&surface).unwrap();
        let parsed: VolSurfaceDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(surface, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_vol_surface_serde_from_json() {
        let json = r#"{
            "name": "EURUSD-FX-Vol",
            "instruments": ["EURUSD-1M-ATM", "EURUSD-3M-ATM"],
            "model": "sabr",
            "strikeAxis": "delta",
            "timeInterpolation": "flat_forward",
            "strikeInterpolation": "sabr",
            "allowExtrapolation": false
        }"#;

        let surface: VolSurfaceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(surface.name, "EURUSD-FX-Vol");
        assert_eq!(surface.instruments.len(), 2);
        assert_eq!(surface.model, CalibrationModel::Sabr);
        assert_eq!(surface.strike_axis, StrikeAxisType::Delta);
        assert_eq!(surface.time_interpolation, TimeInterpolation::FlatForward);
        assert_eq!(surface.strike_interpolation, StrikeInterpolation::Sabr);
        assert!(!surface.allow_extrapolation);
    }
}

//! Calibration model definitions.
//!
//! This module provides the classification of volatility calibration models
//! used for fitting implied volatility surfaces.

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
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Sabr)
    }

    /// Get the number of parameters for this model.
    pub fn parameter_count(&self) -> usize {
        match self {
            Self::Sabr => 4,          // alpha, beta, rho, nu
            Self::Svi => 5,           // a, b, rho, m, sigma
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
        Self::all().iter().copied().filter(|m| m.is_enabled()).collect()
    }
}

impl std::fmt::Display for CalibrationModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
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
}

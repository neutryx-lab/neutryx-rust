//! Risk configuration structures.
//!
//! Provides [`RiskConfig`] for configuration-driven risk/Greeks calculations.

use serde::{Deserialize, Serialize};

use crate::ConfigError;

/// Greeks calculation method selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GreeksMethod {
    /// Automatic Differentiation (Enzyme-based AAD).
    /// Requires `enzyme-ad` feature to be enabled.
    Aad,
    /// Bump-and-Revalue (finite difference approximation).
    #[default]
    Bump,
}

/// Greek sensitivity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GreekType {
    /// ∂V/∂S - Sensitivity to spot price.
    Delta,
    /// ∂²V/∂S² - Convexity with respect to spot.
    Gamma,
    /// ∂V/∂σ - Sensitivity to volatility.
    Vega,
    /// ∂V/∂τ - Sensitivity to time (time decay).
    Theta,
    /// ∂V/∂r - Sensitivity to interest rate.
    Rho,
    /// ∂²V/∂S∂σ - Cross sensitivity (delta-vol).
    Vanna,
    /// ∂²V/∂σ² - Volatility convexity (vomma).
    Volga,
}

impl GreekType {
    /// Returns true if this is a second-order Greek.
    pub fn is_second_order(&self) -> bool {
        matches!(self, Self::Gamma | Self::Vanna | Self::Volga)
    }

    /// Returns all first-order Greeks.
    pub fn first_order() -> Vec<Self> {
        vec![Self::Delta, Self::Vega, Self::Theta, Self::Rho]
    }

    /// Returns all second-order Greeks.
    pub fn second_order() -> Vec<Self> {
        vec![Self::Gamma, Self::Vanna, Self::Volga]
    }

    /// Returns all Greek types.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Delta,
            Self::Gamma,
            Self::Vega,
            Self::Theta,
            Self::Rho,
            Self::Vanna,
            Self::Volga,
        ]
    }
}

/// Bump sizes for finite difference calculations.
///
/// Default values follow market conventions:
/// - Rate: 1bp (0.0001)
/// - Vol: 1% (0.01)
/// - Spot: 1% (0.01)
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct BumpSizes {
    /// Rate bump size (default: 1bp = 0.0001).
    #[serde(default = "default_rate_bump")]
    pub rate: f64,
    /// Volatility bump size (default: 1% = 0.01).
    #[serde(default = "default_vol_bump")]
    pub vol: f64,
    /// Spot price bump size (default: 1% = 0.01).
    #[serde(default = "default_spot_bump")]
    pub spot: f64,
}

fn default_rate_bump() -> f64 {
    0.0001 // 1bp
}

fn default_vol_bump() -> f64 {
    0.01 // 1%
}

fn default_spot_bump() -> f64 {
    0.01 // 1%
}

impl Default for BumpSizes {
    fn default() -> Self {
        Self {
            rate: default_rate_bump(),
            vol: default_vol_bump(),
            spot: default_spot_bump(),
        }
    }
}

impl BumpSizes {
    /// Validates bump sizes are positive and within reasonable bounds.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.rate <= 0.0 || self.rate > 0.01 {
            return Err(ConfigError::InvalidValue {
                key: "bump_sizes.rate".to_string(),
                message: format!(
                    "rate bump must be in (0, 0.01], got {}",
                    self.rate
                ),
            });
        }
        if self.vol <= 0.0 || self.vol > 0.5 {
            return Err(ConfigError::InvalidValue {
                key: "bump_sizes.vol".to_string(),
                message: format!(
                    "vol bump must be in (0, 0.5], got {}",
                    self.vol
                ),
            });
        }
        if self.spot <= 0.0 || self.spot > 0.5 {
            return Err(ConfigError::InvalidValue {
                key: "bump_sizes.spot".to_string(),
                message: format!(
                    "spot bump must be in (0, 0.5], got {}",
                    self.spot
                ),
            });
        }
        Ok(())
    }
}

/// Second-order Greeks calculation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecondOrderMode {
    /// Calculate second-order Greeks in parallel (faster).
    #[default]
    Parallel,
    /// Calculate second-order Greeks sequentially (lower memory).
    Serial,
}

/// Scenario configuration for scenario-based Greeks.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ScenarioConfig {
    /// Predefined scenario type.
    #[serde(default)]
    pub preset: Option<String>,
    /// Custom market shifts.
    #[serde(default)]
    pub custom_shifts: Vec<MarketShift>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            preset: None,
            custom_shifts: Vec::new(),
        }
    }
}

/// A single market shift for scenario analysis.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MarketShift {
    /// Risk factor identifier.
    pub factor: String,
    /// Shift type (absolute or relative).
    #[serde(default)]
    pub shift_type: ShiftType,
    /// Shift amount.
    pub amount: f64,
}

/// Type of market shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShiftType {
    /// Absolute shift (additive).
    #[default]
    Absolute,
    /// Relative shift (multiplicative).
    Relative,
}

/// Configuration for risk/Greeks calculations.
///
/// This structure defines all parameters needed for risk calculations,
/// supporting both TOML and JSON configuration formats.
///
/// # Example
///
/// ```rust
/// use infra_config::{RiskConfig, GreeksMethod, GreekType};
///
/// let config = RiskConfig {
///     greeks_method: GreeksMethod::Bump,
///     target_greeks: vec![GreekType::Delta, GreekType::Gamma, GreekType::Vega],
///     ..Default::default()
/// };
///
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RiskConfig {
    /// Greeks calculation method.
    #[serde(default)]
    pub greeks_method: GreeksMethod,
    /// Bump sizes for finite difference calculations.
    #[serde(default)]
    pub bump_sizes: BumpSizes,
    /// Target Greeks to calculate.
    #[serde(default = "default_target_greeks")]
    pub target_greeks: Vec<GreekType>,
    /// Second-order Greeks calculation mode.
    #[serde(default)]
    pub second_order_mode: SecondOrderMode,
    /// Scenario configuration for scenario-based Greeks.
    #[serde(default)]
    pub scenarios: Option<ScenarioConfig>,
}

fn default_target_greeks() -> Vec<GreekType> {
    vec![GreekType::Delta, GreekType::Gamma, GreekType::Vega]
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            greeks_method: GreeksMethod::default(),
            bump_sizes: BumpSizes::default(),
            target_greeks: default_target_greeks(),
            second_order_mode: SecondOrderMode::default(),
            scenarios: None,
        }
    }
}

impl RiskConfig {
    /// Validates the configuration.
    ///
    /// # Validation Rules
    ///
    /// - `target_greeks` must not be empty
    /// - `bump_sizes` must be within valid ranges
    /// - If AAD method is selected, enzyme-ad feature should be available
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` with specific validation failure details.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate target_greeks is not empty
        if self.target_greeks.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "target_greeks".to_string(),
                message: "at least one Greek must be specified".to_string(),
            });
        }

        // Validate bump sizes
        self.bump_sizes.validate()?;

        // Note: AAD availability check is done at runtime in RiskEngine,
        // not at config validation time, to allow config to be valid
        // even when enzyme-ad feature is not available.

        Ok(())
    }

    /// Loads configuration from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(json).map_err(|e| ConfigError::InvalidValue {
            key: "json".to_string(),
            message: e.to_string(),
        })
    }

    /// Loads configuration from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(|e| ConfigError::InvalidValue {
            key: "toml".to_string(),
            message: e.to_string(),
        })
    }

    /// Returns true if any second-order Greeks are requested.
    pub fn has_second_order_greeks(&self) -> bool {
        self.target_greeks.iter().any(|g| g.is_second_order())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // GreeksMethod Tests
    // =========================================================================

    #[test]
    fn test_greeks_method_default_is_bump() {
        assert_eq!(GreeksMethod::default(), GreeksMethod::Bump);
    }

    #[test]
    fn test_greeks_method_serde() {
        assert_eq!(
            serde_json::to_string(&GreeksMethod::Aad).unwrap(),
            "\"aad\""
        );
        assert_eq!(
            serde_json::to_string(&GreeksMethod::Bump).unwrap(),
            "\"bump\""
        );

        let aad: GreeksMethod = serde_json::from_str("\"aad\"").unwrap();
        assert_eq!(aad, GreeksMethod::Aad);
    }

    // =========================================================================
    // GreekType Tests
    // =========================================================================

    #[test]
    fn test_greek_type_is_second_order() {
        assert!(!GreekType::Delta.is_second_order());
        assert!(GreekType::Gamma.is_second_order());
        assert!(!GreekType::Vega.is_second_order());
        assert!(!GreekType::Theta.is_second_order());
        assert!(!GreekType::Rho.is_second_order());
        assert!(GreekType::Vanna.is_second_order());
        assert!(GreekType::Volga.is_second_order());
    }

    #[test]
    fn test_greek_type_collections() {
        assert_eq!(GreekType::first_order().len(), 4);
        assert_eq!(GreekType::second_order().len(), 3);
        assert_eq!(GreekType::all().len(), 7);
    }

    #[test]
    fn test_greek_type_serde() {
        assert_eq!(
            serde_json::to_string(&GreekType::Delta).unwrap(),
            "\"delta\""
        );
        assert_eq!(
            serde_json::to_string(&GreekType::Gamma).unwrap(),
            "\"gamma\""
        );

        let delta: GreekType = serde_json::from_str("\"delta\"").unwrap();
        assert_eq!(delta, GreekType::Delta);
    }

    // =========================================================================
    // BumpSizes Tests
    // =========================================================================

    #[test]
    fn test_bump_sizes_default() {
        let bump = BumpSizes::default();
        assert!((bump.rate - 0.0001).abs() < f64::EPSILON);
        assert!((bump.vol - 0.01).abs() < f64::EPSILON);
        assert!((bump.spot - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bump_sizes_validation_valid() {
        let bump = BumpSizes::default();
        assert!(bump.validate().is_ok());
    }

    #[test]
    fn test_bump_sizes_validation_invalid_rate_zero() {
        let bump = BumpSizes {
            rate: 0.0,
            ..Default::default()
        };
        let result = bump.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate"));
    }

    #[test]
    fn test_bump_sizes_validation_invalid_rate_too_large() {
        let bump = BumpSizes {
            rate: 0.02,
            ..Default::default()
        };
        let result = bump.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_bump_sizes_validation_invalid_vol() {
        let bump = BumpSizes {
            vol: -0.01,
            ..Default::default()
        };
        let result = bump.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("vol"));
    }

    #[test]
    fn test_bump_sizes_validation_invalid_spot() {
        let bump = BumpSizes {
            spot: 0.6,
            ..Default::default()
        };
        let result = bump.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("spot"));
    }

    // =========================================================================
    // SecondOrderMode Tests
    // =========================================================================

    #[test]
    fn test_second_order_mode_default_is_parallel() {
        assert_eq!(SecondOrderMode::default(), SecondOrderMode::Parallel);
    }

    #[test]
    fn test_second_order_mode_serde() {
        assert_eq!(
            serde_json::to_string(&SecondOrderMode::Parallel).unwrap(),
            "\"parallel\""
        );
        assert_eq!(
            serde_json::to_string(&SecondOrderMode::Serial).unwrap(),
            "\"serial\""
        );
    }

    // =========================================================================
    // RiskConfig Tests
    // =========================================================================

    #[test]
    fn test_risk_config_default_creates_valid_config() {
        let config = RiskConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.greeks_method, GreeksMethod::Bump);
        assert_eq!(config.target_greeks.len(), 3);
        assert!(config.target_greeks.contains(&GreekType::Delta));
    }

    #[test]
    fn test_risk_config_validates_empty_target_greeks() {
        let config = RiskConfig {
            target_greeks: vec![],
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("target_greeks"));
    }

    #[test]
    fn test_risk_config_validates_bump_sizes() {
        let config = RiskConfig {
            bump_sizes: BumpSizes {
                rate: 0.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_risk_config_has_second_order_greeks() {
        let config_with_gamma = RiskConfig {
            target_greeks: vec![GreekType::Delta, GreekType::Gamma],
            ..Default::default()
        };
        assert!(config_with_gamma.has_second_order_greeks());

        let config_first_order_only = RiskConfig {
            target_greeks: vec![GreekType::Delta, GreekType::Vega],
            ..Default::default()
        };
        assert!(!config_first_order_only.has_second_order_greeks());
    }

    #[test]
    fn test_risk_config_from_json() {
        let json = r#"{
            "greeks_method": "aad",
            "target_greeks": ["delta", "gamma", "vega", "theta"],
            "second_order_mode": "serial"
        }"#;

        let config = RiskConfig::from_json(json).unwrap();
        assert_eq!(config.greeks_method, GreeksMethod::Aad);
        assert_eq!(config.target_greeks.len(), 4);
        assert_eq!(config.second_order_mode, SecondOrderMode::Serial);
    }

    #[test]
    fn test_risk_config_from_json_with_bump_sizes() {
        let json = r#"{
            "greeks_method": "bump",
            "bump_sizes": {
                "rate": 0.0002,
                "vol": 0.02,
                "spot": 0.05
            },
            "target_greeks": ["delta", "vega"]
        }"#;

        let config = RiskConfig::from_json(json).unwrap();
        assert!((config.bump_sizes.rate - 0.0002).abs() < f64::EPSILON);
        assert!((config.bump_sizes.vol - 0.02).abs() < f64::EPSILON);
        assert!((config.bump_sizes.spot - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_risk_config_from_toml() {
        let toml_str = r#"
            greeks_method = "bump"
            target_greeks = ["delta", "gamma", "vega", "rho"]
            second_order_mode = "parallel"

            [bump_sizes]
            rate = 0.0001
            vol = 0.01
            spot = 0.01
        "#;

        let config = RiskConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.greeks_method, GreeksMethod::Bump);
        assert_eq!(config.target_greeks.len(), 4);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_risk_config_from_toml_with_scenarios() {
        let toml_str = r#"
            greeks_method = "bump"
            target_greeks = ["delta", "vega"]

            [scenarios]
            preset = "stress_test_2008"

            [[scenarios.custom_shifts]]
            factor = "USD_SOFR_1Y"
            shift_type = "absolute"
            amount = 0.005

            [[scenarios.custom_shifts]]
            factor = "SPX_VOL"
            shift_type = "relative"
            amount = 0.25
        "#;

        let config = RiskConfig::from_toml(toml_str).unwrap();
        let scenarios = config.scenarios.unwrap();
        assert_eq!(scenarios.preset, Some("stress_test_2008".to_string()));
        assert_eq!(scenarios.custom_shifts.len(), 2);
        assert_eq!(scenarios.custom_shifts[0].factor, "USD_SOFR_1Y");
        assert_eq!(scenarios.custom_shifts[0].shift_type, ShiftType::Absolute);
        assert!((scenarios.custom_shifts[0].amount - 0.005).abs() < f64::EPSILON);
    }

    #[test]
    fn test_risk_config_serializes_to_json() {
        let config = RiskConfig {
            greeks_method: GreeksMethod::Aad,
            bump_sizes: BumpSizes::default(),
            target_greeks: vec![GreekType::Delta, GreekType::Vega],
            second_order_mode: SecondOrderMode::Parallel,
            scenarios: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"greeks_method\":\"aad\""));
        assert!(json.contains("\"delta\""));
        assert!(json.contains("\"vega\""));
    }

    #[test]
    fn test_market_shift_serde() {
        let shift = MarketShift {
            factor: "EUR_ESTR_5Y".to_string(),
            shift_type: ShiftType::Absolute,
            amount: 0.001,
        };

        let json = serde_json::to_string(&shift).unwrap();
        assert!(json.contains("\"factor\":\"EUR_ESTR_5Y\""));

        let parsed: MarketShift = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.factor, "EUR_ESTR_5Y");
    }
}

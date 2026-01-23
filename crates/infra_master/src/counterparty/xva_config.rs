//! XVA calculation configuration structures.
//!
//! This module provides configuration types for XVA (X-Value Adjustment)
//! calculations including CVA, DVA, FVA, KVA, and MVA.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use crate::market::Currency;
use super::NettingSetId;

// ============================================================================
// XvaCalculationLevel
// ============================================================================

/// XVA calculation aggregation level.
///
/// Defines at which level XVA calculations should be aggregated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum XvaCalculationLevel {
    /// Trade-level calculation (no aggregation).
    Trade,
    /// Netting set level (standard for CVA).
    #[default]
    NettingSet,
    /// Counterparty level (aggregates netting sets).
    Counterparty,
    /// Book level (aggregates by trading book).
    Book,
    /// Portfolio level (full aggregation).
    Portfolio,
}

// ============================================================================
// XvaScope
// ============================================================================

/// XVA calculation scope definition.
///
/// Defines the scope of XVA calculations including which netting sets
/// to include and simulation parameters.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XvaScope {
    /// Netting set IDs to include in calculation.
    netting_set_ids: Vec<NettingSetId>,
    /// Time horizon for simulation in years.
    time_horizon_years: f64,
    /// Number of simulation paths.
    num_paths: u32,
    /// Time step size in years.
    time_step_years: f64,
    /// Calculation level.
    calculation_level: XvaCalculationLevel,
}

impl XvaScope {
    /// Creates a new XvaScope with default values.
    pub fn new() -> Self { Self::default() }

    /// Sets the netting set IDs to include.
    pub fn with_netting_sets(
        mut self,
        ids: impl IntoIterator<Item = impl Into<NettingSetId>>,
    ) -> Self {
        self.netting_set_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a netting set ID.
    pub fn add_netting_set(mut self, id: impl Into<NettingSetId>) -> Self {
        let nsi = id.into();
        if !self.netting_set_ids.contains(&nsi) {
            self.netting_set_ids.push(nsi);
        }
        self
    }

    /// Sets the time horizon in years.
    pub fn with_time_horizon(mut self, years: f64) -> Self {
        self.time_horizon_years = years;
        self
    }

    /// Sets the number of simulation paths.
    pub fn with_num_paths(mut self, paths: u32) -> Self {
        self.num_paths = paths;
        self
    }

    /// Sets the time step size in years.
    pub fn with_time_step(mut self, step: f64) -> Self {
        self.time_step_years = step;
        self
    }

    /// Sets the calculation level.
    pub fn with_calculation_level(mut self, level: XvaCalculationLevel) -> Self {
        self.calculation_level = level;
        self
    }

    /// Returns the netting set IDs.
    pub fn netting_set_ids(&self) -> &[NettingSetId] { &self.netting_set_ids }

    /// Returns the time horizon in years.
    pub fn time_horizon_years(&self) -> f64 { self.time_horizon_years }

    /// Returns the number of simulation paths.
    pub fn num_paths(&self) -> u32 { self.num_paths }

    /// Returns the time step size in years.
    pub fn time_step_years(&self) -> f64 { self.time_step_years }

    /// Returns the calculation level.
    pub fn calculation_level(&self) -> XvaCalculationLevel { self.calculation_level }
}

impl Default for XvaScope {
    fn default() -> Self {
        Self {
            netting_set_ids: Vec::new(),
            time_horizon_years: 10.0,
            num_paths: 10_000,
            time_step_years: 0.25,
            calculation_level: XvaCalculationLevel::default(),
        }
    }
}

// ============================================================================
// XvaConfig
// ============================================================================

/// XVA calculation configuration flags.
///
/// Enables/disables individual XVA components.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XvaConfig {
    /// Calculate CVA (Credit Valuation Adjustment).
    calculate_cva: bool,
    /// Calculate DVA (Debit Valuation Adjustment).
    calculate_dva: bool,
    /// Calculate FVA (Funding Valuation Adjustment).
    calculate_fva: bool,
    /// Calculate KVA (Capital Valuation Adjustment).
    calculate_kva: bool,
    /// Calculate MVA (Margin Valuation Adjustment).
    calculate_mva: bool,
    /// Funding configuration (required if FVA enabled).
    funding_config: Option<FundingConfig>,
    /// Capital configuration (required if KVA enabled).
    capital_config: Option<CapitalConfig>,
    /// Wrong-way risk configuration.
    wwr_config: Option<WrongWayRiskConfig>,
}

impl XvaConfig {
    /// Creates a new XvaConfig with all XVA types disabled.
    pub fn new() -> Self { Self::default() }

    /// Creates a configuration with CVA enabled.
    pub fn cva_only() -> Self {
        Self {
            calculate_cva: true,
            ..Self::default()
        }
    }

    /// Creates a configuration with CVA and DVA enabled.
    pub fn bilateral() -> Self {
        Self {
            calculate_cva: true,
            calculate_dva: true,
            ..Self::default()
        }
    }

    /// Creates a full XVA configuration.
    pub fn full() -> Self {
        Self {
            calculate_cva: true,
            calculate_dva: true,
            calculate_fva: true,
            calculate_kva: true,
            calculate_mva: true,
            funding_config: Some(FundingConfig::default()),
            capital_config: Some(CapitalConfig::default()),
            wwr_config: None,
        }
    }

    /// Enables CVA calculation.
    pub fn with_cva(mut self, enabled: bool) -> Self {
        self.calculate_cva = enabled;
        self
    }

    /// Enables DVA calculation.
    pub fn with_dva(mut self, enabled: bool) -> Self {
        self.calculate_dva = enabled;
        self
    }

    /// Enables FVA calculation.
    pub fn with_fva(mut self, enabled: bool) -> Self {
        self.calculate_fva = enabled;
        self
    }

    /// Enables KVA calculation.
    pub fn with_kva(mut self, enabled: bool) -> Self {
        self.calculate_kva = enabled;
        self
    }

    /// Enables MVA calculation.
    pub fn with_mva(mut self, enabled: bool) -> Self {
        self.calculate_mva = enabled;
        self
    }

    /// Sets the funding configuration.
    pub fn with_funding_config(mut self, config: FundingConfig) -> Self {
        self.funding_config = Some(config);
        self
    }

    /// Sets the capital configuration.
    pub fn with_capital_config(mut self, config: CapitalConfig) -> Self {
        self.capital_config = Some(config);
        self
    }

    /// Sets the wrong-way risk configuration.
    pub fn with_wwr_config(mut self, config: WrongWayRiskConfig) -> Self {
        self.wwr_config = Some(config);
        self
    }

    /// Returns whether CVA is enabled.
    pub fn calculate_cva(&self) -> bool { self.calculate_cva }

    /// Returns whether DVA is enabled.
    pub fn calculate_dva(&self) -> bool { self.calculate_dva }

    /// Returns whether FVA is enabled.
    pub fn calculate_fva(&self) -> bool { self.calculate_fva }

    /// Returns whether KVA is enabled.
    pub fn calculate_kva(&self) -> bool { self.calculate_kva }

    /// Returns whether MVA is enabled.
    pub fn calculate_mva(&self) -> bool { self.calculate_mva }

    /// Returns the funding configuration.
    pub fn funding_config(&self) -> Option<&FundingConfig> { self.funding_config.as_ref() }

    /// Returns the capital configuration.
    pub fn capital_config(&self) -> Option<&CapitalConfig> { self.capital_config.as_ref() }

    /// Returns the wrong-way risk configuration.
    pub fn wwr_config(&self) -> Option<&WrongWayRiskConfig> { self.wwr_config.as_ref() }
}

impl Default for XvaConfig {
    fn default() -> Self {
        Self {
            calculate_cva: false,
            calculate_dva: false,
            calculate_fva: false,
            calculate_kva: false,
            calculate_mva: false,
            funding_config: None,
            capital_config: None,
            wwr_config: None,
        }
    }
}

// ============================================================================
// FundingConfig
// ============================================================================

/// Funding configuration for FVA calculation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FundingConfig {
    /// Funding spread curve ID.
    pub funding_spread_curve_id: String,
    /// Collateral rate curve ID.
    pub collateral_rate_curve_id: String,
    /// Funding currency.
    pub funding_currency: Currency,
    /// Asymmetric funding flag (different borrow/lend rates).
    pub asymmetric_funding: bool,
}

impl Default for FundingConfig {
    fn default() -> Self {
        Self {
            funding_spread_curve_id: String::new(),
            collateral_rate_curve_id: String::new(),
            funding_currency: Currency::USD,
            asymmetric_funding: false,
        }
    }
}

// ============================================================================
// RegulatoryCapitalMethod
// ============================================================================

/// Regulatory capital calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RegulatoryCapitalMethod {
    /// Standardised Approach for Counterparty Credit Risk.
    #[default]
    SaCcr,
    /// Internal Model Method.
    Imm,
}

// ============================================================================
// CapitalConfig
// ============================================================================

/// Capital configuration for KVA calculation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapitalConfig {
    /// Regulatory capital calculation method.
    pub regulatory_method: RegulatoryCapitalMethod,
    /// Capital rate (cost of capital, e.g., 0.10 for 10%).
    pub capital_rate: f64,
    /// Risk weight multiplier.
    pub risk_weight_multiplier: f64,
}

impl Default for CapitalConfig {
    fn default() -> Self {
        Self {
            regulatory_method: RegulatoryCapitalMethod::default(),
            capital_rate: 0.10,
            risk_weight_multiplier: 1.0,
        }
    }
}

// ============================================================================
// WwrModelType
// ============================================================================

/// Wrong-Way Risk model type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WwrModelType {
    /// No WWR adjustment.
    #[default]
    None,
    /// Constant correlation model.
    ConstantCorrelation,
    /// Hull-White correlation model.
    HullWhite,
    /// Jump-to-default model.
    JumpToDefault,
}

// ============================================================================
// WrongWayRiskConfig
// ============================================================================

/// Wrong-Way Risk (WWR) configuration.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WrongWayRiskConfig {
    /// Correlation estimate between exposure and credit.
    pub correlation_estimate: f64,
    /// Stress correlation for adverse scenarios.
    pub stress_correlation: f64,
    /// WWR model type.
    pub model_type: WwrModelType,
}

impl Default for WrongWayRiskConfig {
    fn default() -> Self {
        Self {
            correlation_estimate: 0.0,
            stress_correlation: 0.0,
            model_type: WwrModelType::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // XvaCalculationLevel tests
    // ========================================================================

    #[test]
    fn test_xva_calculation_level_default() {
        assert_eq!(
            XvaCalculationLevel::default(),
            XvaCalculationLevel::NettingSet
        );
    }

    // ========================================================================
    // XvaScope tests
    // ========================================================================

    #[test]
    fn test_xva_scope_default() {
        let scope = XvaScope::default();
        assert!(scope.netting_set_ids().is_empty());
        assert!((scope.time_horizon_years() - 10.0).abs() < f64::EPSILON);
        assert_eq!(scope.num_paths(), 10_000);
        assert!((scope.time_step_years() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_xva_scope_builder() {
        let scope = XvaScope::new()
            .add_netting_set("NS001")
            .add_netting_set("NS002")
            .with_time_horizon(5.0)
            .with_num_paths(50_000)
            .with_time_step(0.1)
            .with_calculation_level(XvaCalculationLevel::Counterparty);

        assert_eq!(scope.netting_set_ids().len(), 2);
        assert!((scope.time_horizon_years() - 5.0).abs() < f64::EPSILON);
        assert_eq!(scope.num_paths(), 50_000);
        assert_eq!(
            scope.calculation_level(),
            XvaCalculationLevel::Counterparty
        );
    }

    #[test]
    fn test_xva_scope_dedup_netting_sets() {
        let scope = XvaScope::new()
            .add_netting_set("NS001")
            .add_netting_set("NS001") // Duplicate
            .add_netting_set("NS002");

        assert_eq!(scope.netting_set_ids().len(), 2);
    }

    // ========================================================================
    // XvaConfig tests
    // ========================================================================

    #[test]
    fn test_xva_config_default() {
        let config = XvaConfig::default();
        assert!(!config.calculate_cva());
        assert!(!config.calculate_dva());
        assert!(!config.calculate_fva());
        assert!(!config.calculate_kva());
        assert!(!config.calculate_mva());
    }

    #[test]
    fn test_xva_config_cva_only() {
        let config = XvaConfig::cva_only();
        assert!(config.calculate_cva());
        assert!(!config.calculate_dva());
    }

    #[test]
    fn test_xva_config_bilateral() {
        let config = XvaConfig::bilateral();
        assert!(config.calculate_cva());
        assert!(config.calculate_dva());
        assert!(!config.calculate_fva());
    }

    #[test]
    fn test_xva_config_full() {
        let config = XvaConfig::full();
        assert!(config.calculate_cva());
        assert!(config.calculate_dva());
        assert!(config.calculate_fva());
        assert!(config.calculate_kva());
        assert!(config.calculate_mva());
        assert!(config.funding_config().is_some());
        assert!(config.capital_config().is_some());
    }

    #[test]
    fn test_xva_config_builder() {
        let config = XvaConfig::new()
            .with_cva(true)
            .with_fva(true)
            .with_funding_config(FundingConfig {
                funding_spread_curve_id: "FUND_SPREAD".to_string(),
                collateral_rate_curve_id: "OIS".to_string(),
                funding_currency: Currency::EUR,
                asymmetric_funding: true,
            });

        assert!(config.calculate_cva());
        assert!(config.calculate_fva());
        assert!(config.funding_config().is_some());
        assert_eq!(
            config.funding_config().unwrap().funding_currency,
            Currency::EUR
        );
    }

    // ========================================================================
    // FundingConfig tests
    // ========================================================================

    #[test]
    fn test_funding_config_default() {
        let config = FundingConfig::default();
        assert!(config.funding_spread_curve_id.is_empty());
        assert_eq!(config.funding_currency, Currency::USD);
        assert!(!config.asymmetric_funding);
    }

    // ========================================================================
    // CapitalConfig tests
    // ========================================================================

    #[test]
    fn test_capital_config_default() {
        let config = CapitalConfig::default();
        assert_eq!(config.regulatory_method, RegulatoryCapitalMethod::SaCcr);
        assert!((config.capital_rate - 0.10).abs() < f64::EPSILON);
        assert!((config.risk_weight_multiplier - 1.0).abs() < f64::EPSILON);
    }

    // ========================================================================
    // WrongWayRiskConfig tests
    // ========================================================================

    #[test]
    fn test_wwr_config_default() {
        let config = WrongWayRiskConfig::default();
        assert!(config.correlation_estimate.abs() < f64::EPSILON);
        assert!(config.stress_correlation.abs() < f64::EPSILON);
        assert_eq!(config.model_type, WwrModelType::None);
    }

    #[test]
    fn test_wwr_model_type_default() {
        assert_eq!(WwrModelType::default(), WwrModelType::None);
    }
}

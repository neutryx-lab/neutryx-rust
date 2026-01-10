//! Risk factor shifts and bump scenarios.
//!
//! Provides infrastructure for defining market data shocks:
//! - `RiskFactorShift`: Individual factor shift specification
//! - `BumpScenario`: Collection of coordinated shifts
//! - `Scenario`: Named scenario with description

use pricer_core::traits::risk::{RiskFactorType, ShiftType};
use pricer_core::traits::Float;

/// Specification for shifting a single risk factor.
///
/// Combines a factor type, identifier pattern, and shift specification.
///
/// # Requirements
///
/// - Requirements: 10.2
#[derive(Clone, Debug)]
pub struct RiskFactorShift<T: Float> {
    /// Type of risk factor being shifted
    factor_type: RiskFactorType,
    /// Identifier pattern (e.g., "USD.OIS.*" for all USD OIS tenors)
    identifier_pattern: String,
    /// The shift to apply
    shift: ShiftType<T>,
}

impl<T: Float> RiskFactorShift<T> {
    /// Create a new risk factor shift.
    pub fn new(
        factor_type: RiskFactorType,
        identifier_pattern: impl Into<String>,
        shift: ShiftType<T>,
    ) -> Self {
        Self {
            factor_type,
            identifier_pattern: identifier_pattern.into(),
            shift,
        }
    }

    /// Create an interest rate parallel shift.
    pub fn rate_parallel(pattern: impl Into<String>, amount: T) -> Self {
        Self::new(
            RiskFactorType::InterestRate,
            pattern,
            ShiftType::Parallel(amount),
        )
    }

    /// Create a credit spread shift.
    pub fn credit_spread(pattern: impl Into<String>, amount: T) -> Self {
        Self::new(RiskFactorType::Credit, pattern, ShiftType::Absolute(amount))
    }

    /// Create an FX rate relative shift.
    pub fn fx_relative(pattern: impl Into<String>, percentage: T) -> Self {
        Self::new(RiskFactorType::Fx, pattern, ShiftType::Relative(percentage))
    }

    /// Create an equity price relative shift.
    pub fn equity_relative(pattern: impl Into<String>, percentage: T) -> Self {
        Self::new(
            RiskFactorType::Equity,
            pattern,
            ShiftType::Relative(percentage),
        )
    }

    /// Create a volatility absolute shift.
    pub fn volatility_shift(pattern: impl Into<String>, amount: T) -> Self {
        Self::new(
            RiskFactorType::Volatility,
            pattern,
            ShiftType::Absolute(amount),
        )
    }

    /// Get the factor type.
    pub fn factor_type(&self) -> RiskFactorType {
        self.factor_type
    }

    /// Get the identifier pattern.
    pub fn identifier_pattern(&self) -> &str {
        &self.identifier_pattern
    }

    /// Get the shift.
    pub fn shift(&self) -> &ShiftType<T> {
        &self.shift
    }

    /// Check if an identifier matches the pattern.
    ///
    /// Supports simple glob patterns with `*` wildcard.
    pub fn matches(&self, identifier: &str) -> bool {
        if self.identifier_pattern == "*" {
            return true;
        }
        if self.identifier_pattern.ends_with('*') {
            let prefix = &self.identifier_pattern[..self.identifier_pattern.len() - 1];
            return identifier.starts_with(prefix);
        }
        self.identifier_pattern == identifier
    }
}

/// A collection of risk factor shifts forming a bump scenario.
///
/// Used to define coordinated shifts across multiple factors.
///
/// # Requirements
///
/// - Requirements: 10.2
#[derive(Clone, Debug)]
pub struct BumpScenario<T: Float> {
    /// List of factor shifts
    shifts: Vec<RiskFactorShift<T>>,
}

impl<T: Float> BumpScenario<T> {
    /// Create a new empty bump scenario.
    pub fn new() -> Self {
        Self { shifts: Vec::new() }
    }

    /// Add a shift to the scenario.
    pub fn with_shift(mut self, shift: RiskFactorShift<T>) -> Self {
        self.shifts.push(shift);
        self
    }

    /// Add multiple shifts.
    pub fn with_shifts(mut self, shifts: impl IntoIterator<Item = RiskFactorShift<T>>) -> Self {
        self.shifts.extend(shifts);
        self
    }

    /// Get all shifts.
    pub fn shifts(&self) -> &[RiskFactorShift<T>] {
        &self.shifts
    }

    /// Get shifts for a specific factor type.
    pub fn shifts_for_type(&self, factor_type: RiskFactorType) -> Vec<&RiskFactorShift<T>> {
        self.shifts
            .iter()
            .filter(|s| s.factor_type == factor_type)
            .collect()
    }

    /// Check if scenario has any shifts for a given factor type.
    pub fn has_shifts_for(&self, factor_type: RiskFactorType) -> bool {
        self.shifts.iter().any(|s| s.factor_type == factor_type)
    }

    /// Get the number of shifts.
    pub fn len(&self) -> usize {
        self.shifts.len()
    }

    /// Check if scenario is empty.
    pub fn is_empty(&self) -> bool {
        self.shifts.is_empty()
    }
}

impl<T: Float> Default for BumpScenario<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A named scenario with description and bump specification.
///
/// # Requirements
///
/// - Requirements: 10.4
#[derive(Clone, Debug)]
pub struct Scenario<T: Float> {
    /// Scenario name
    name: String,
    /// Scenario description
    description: String,
    /// The bump scenario specification
    bumps: BumpScenario<T>,
}

impl<T: Float> Scenario<T> {
    /// Create a new named scenario.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        bumps: BumpScenario<T>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            bumps,
        }
    }

    /// Create a scenario with just a name and bumps.
    pub fn named(name: impl Into<String>, bumps: BumpScenario<T>) -> Self {
        let name_str = name.into();
        Self {
            description: name_str.clone(),
            name: name_str,
            bumps,
        }
    }

    /// Get scenario name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get scenario description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the bump scenario.
    pub fn bumps(&self) -> &BumpScenario<T> {
        &self.bumps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 11.2: RiskFactorShift and BumpScenario tests (TDD)
    // ================================================================

    #[test]
    fn test_risk_factor_shift_new() {
        let shift = RiskFactorShift::new(
            RiskFactorType::InterestRate,
            "USD.OIS.5Y",
            ShiftType::parallel(0.0001_f64),
        );
        assert_eq!(shift.factor_type(), RiskFactorType::InterestRate);
        assert_eq!(shift.identifier_pattern(), "USD.OIS.5Y");
    }

    #[test]
    fn test_risk_factor_shift_rate_parallel() {
        let shift = RiskFactorShift::rate_parallel("USD.OIS.*", 0.0001_f64);
        assert_eq!(shift.factor_type(), RiskFactorType::InterestRate);
        assert!(matches!(shift.shift(), ShiftType::Parallel(_)));
    }

    #[test]
    fn test_risk_factor_shift_credit_spread() {
        let shift = RiskFactorShift::credit_spread("XYZ.CDS.5Y", 0.005_f64);
        assert_eq!(shift.factor_type(), RiskFactorType::Credit);
    }

    #[test]
    fn test_risk_factor_shift_fx_relative() {
        let shift = RiskFactorShift::fx_relative("USDJPY", 0.10_f64);
        assert_eq!(shift.factor_type(), RiskFactorType::Fx);
        assert!(matches!(shift.shift(), ShiftType::Relative(_)));
    }

    #[test]
    fn test_risk_factor_shift_matches_exact() {
        let shift = RiskFactorShift::rate_parallel("USD.OIS.5Y", 0.0001_f64);
        assert!(shift.matches("USD.OIS.5Y"));
        assert!(!shift.matches("USD.OIS.10Y"));
    }

    #[test]
    fn test_risk_factor_shift_matches_wildcard() {
        let shift = RiskFactorShift::rate_parallel("USD.OIS.*", 0.0001_f64);
        assert!(shift.matches("USD.OIS.5Y"));
        assert!(shift.matches("USD.OIS.10Y"));
        assert!(!shift.matches("EUR.OIS.5Y"));
    }

    #[test]
    fn test_risk_factor_shift_matches_all() {
        let shift = RiskFactorShift::rate_parallel("*", 0.0001_f64);
        assert!(shift.matches("USD.OIS.5Y"));
        assert!(shift.matches("EUR.OIS.10Y"));
        assert!(shift.matches("anything"));
    }

    #[test]
    fn test_bump_scenario_new() {
        let scenario = BumpScenario::<f64>::new();
        assert!(scenario.is_empty());
        assert_eq!(scenario.len(), 0);
    }

    #[test]
    fn test_bump_scenario_with_shift() {
        let scenario = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("USD.*", 0.0001_f64))
            .with_shift(RiskFactorShift::credit_spread("*", 0.005_f64));

        assert_eq!(scenario.len(), 2);
        assert!(!scenario.is_empty());
    }

    #[test]
    fn test_bump_scenario_shifts_for_type() {
        let scenario = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("USD.*", 0.0001_f64))
            .with_shift(RiskFactorShift::rate_parallel("EUR.*", 0.0002_f64))
            .with_shift(RiskFactorShift::credit_spread("*", 0.005_f64));

        let rate_shifts = scenario.shifts_for_type(RiskFactorType::InterestRate);
        assert_eq!(rate_shifts.len(), 2);

        let credit_shifts = scenario.shifts_for_type(RiskFactorType::Credit);
        assert_eq!(credit_shifts.len(), 1);

        let fx_shifts = scenario.shifts_for_type(RiskFactorType::Fx);
        assert_eq!(fx_shifts.len(), 0);
    }

    #[test]
    fn test_bump_scenario_has_shifts_for() {
        let scenario = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("USD.*", 0.0001_f64));

        assert!(scenario.has_shifts_for(RiskFactorType::InterestRate));
        assert!(!scenario.has_shifts_for(RiskFactorType::Credit));
    }

    #[test]
    fn test_scenario_new() {
        let bumps = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("*", 0.0001_f64));

        let scenario = Scenario::new("IR +1bp", "Interest rate parallel up 1bp", bumps);

        assert_eq!(scenario.name(), "IR +1bp");
        assert_eq!(scenario.description(), "Interest rate parallel up 1bp");
        assert_eq!(scenario.bumps().len(), 1);
    }

    #[test]
    fn test_scenario_named() {
        let bumps = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("*", 0.01_f64));

        let scenario = Scenario::named("IR +100bp", bumps);

        assert_eq!(scenario.name(), "IR +100bp");
        assert_eq!(scenario.description(), "IR +100bp"); // Same as name
    }

    #[test]
    fn test_bump_scenario_clone() {
        let scenario = BumpScenario::new()
            .with_shift(RiskFactorShift::rate_parallel("*", 0.0001_f64));

        let cloned = scenario.clone();
        assert_eq!(cloned.len(), scenario.len());
    }

    // Test: f32 compatibility
    #[test]
    fn test_risk_factor_shift_f32() {
        let shift = RiskFactorShift::rate_parallel("USD.*", 0.0001_f32);
        assert_eq!(shift.factor_type(), RiskFactorType::InterestRate);
    }
}

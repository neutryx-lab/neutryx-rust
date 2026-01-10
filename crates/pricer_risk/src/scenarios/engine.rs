//! Scenario execution engine.
//!
//! Provides infrastructure for running scenarios and collecting results.

use super::shifts::Scenario;
use pricer_core::traits::Float;
use std::collections::HashMap;

/// P&L result from a single scenario.
///
/// # Requirements
///
/// - Requirements: 10.4
#[derive(Clone, Debug)]
pub struct ScenarioPnL<T: Float> {
    /// Scenario name
    pub scenario_name: String,
    /// Base value before scenario
    pub base_value: T,
    /// Stressed value after scenario
    pub stressed_value: T,
    /// P&L (stressed - base)
    pub pnl: T,
    /// P&L as percentage of base
    pub pnl_pct: T,
}

impl<T: Float> ScenarioPnL<T> {
    /// Create a new scenario P&L result.
    pub fn new(scenario_name: impl Into<String>, base_value: T, stressed_value: T) -> Self {
        let pnl = stressed_value - base_value;
        let pnl_pct = if base_value != T::zero() {
            pnl / base_value.abs()
        } else {
            T::zero()
        };
        Self {
            scenario_name: scenario_name.into(),
            base_value,
            stressed_value,
            pnl,
            pnl_pct,
        }
    }

    /// Check if P&L is a loss (negative).
    pub fn is_loss(&self) -> bool {
        self.pnl < T::zero()
    }

    /// Check if P&L is a gain (positive).
    pub fn is_gain(&self) -> bool {
        self.pnl > T::zero()
    }
}

/// Complete result from scenario execution.
#[derive(Clone, Debug)]
pub struct ScenarioResult<T: Float> {
    /// Scenario that was executed
    pub scenario_name: String,
    /// P&L by trade
    pub trade_pnls: HashMap<String, ScenarioPnL<T>>,
    /// Portfolio-level P&L
    pub portfolio_pnl: ScenarioPnL<T>,
}

impl<T: Float> ScenarioResult<T> {
    /// Create a new scenario result.
    pub fn new(scenario_name: impl Into<String>, portfolio_pnl: ScenarioPnL<T>) -> Self {
        Self {
            scenario_name: scenario_name.into(),
            trade_pnls: HashMap::new(),
            portfolio_pnl,
        }
    }

    /// Add a trade P&L.
    pub fn add_trade_pnl(&mut self, trade_id: impl Into<String>, pnl: ScenarioPnL<T>) {
        self.trade_pnls.insert(trade_id.into(), pnl);
    }

    /// Get trade P&L count.
    pub fn trade_count(&self) -> usize {
        self.trade_pnls.len()
    }

    /// Get worst trade P&L.
    pub fn worst_trade_pnl(&self) -> Option<&ScenarioPnL<T>> {
        self.trade_pnls.values().min_by(|a, b| {
            a.pnl
                .partial_cmp(&b.pnl)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Engine for executing scenarios against portfolios.
///
/// # Requirements
///
/// - Requirements: 10.4
#[derive(Clone, Debug)]
pub struct ScenarioEngine<T: Float> {
    /// Registered scenarios
    scenarios: Vec<Scenario<T>>,
    /// Results from executed scenarios
    results: Vec<ScenarioResult<T>>,
}

impl<T: Float> Default for ScenarioEngine<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> ScenarioEngine<T> {
    /// Create a new scenario engine.
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Add a scenario.
    pub fn add_scenario(&mut self, scenario: Scenario<T>) {
        self.scenarios.push(scenario);
    }

    /// Add multiple scenarios.
    pub fn add_scenarios(&mut self, scenarios: impl IntoIterator<Item = Scenario<T>>) {
        self.scenarios.extend(scenarios);
    }

    /// Get registered scenarios.
    pub fn scenarios(&self) -> &[Scenario<T>] {
        &self.scenarios
    }

    /// Get scenario count.
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Execute a single scenario.
    ///
    /// This is a simplified implementation that computes P&L based on
    /// a pricing function provided by the caller.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to execute
    /// * `base_value` - The base portfolio value
    /// * `pricer` - Function to compute stressed value given scenario name
    ///
    /// # Returns
    ///
    /// The scenario result with P&L.
    pub fn execute_scenario<F>(&mut self, scenario: &Scenario<T>, base_value: T, pricer: F) -> ScenarioResult<T>
    where
        F: Fn(&str) -> T,
    {
        let stressed_value = pricer(scenario.name());
        let pnl = ScenarioPnL::new(scenario.name(), base_value, stressed_value);
        let result = ScenarioResult::new(scenario.name(), pnl);
        self.results.push(result.clone());
        result
    }

    /// Execute all registered scenarios.
    ///
    /// # Arguments
    ///
    /// * `base_value` - The base portfolio value
    /// * `pricer` - Function to compute stressed value given scenario name
    ///
    /// # Returns
    ///
    /// Vector of scenario results.
    pub fn execute_all<F>(&mut self, base_value: T, pricer: F) -> Vec<ScenarioResult<T>>
    where
        F: Fn(&str) -> T,
    {
        let mut results = Vec::new();
        for scenario in &self.scenarios.clone() {
            let result = self.execute_scenario(scenario, base_value, &pricer);
            results.push(result);
        }
        results
    }

    /// Get all results.
    pub fn results(&self) -> &[ScenarioResult<T>] {
        &self.results
    }

    /// Get the worst-case scenario (largest loss).
    pub fn worst_case(&self) -> Option<&ScenarioResult<T>> {
        self.results.iter().min_by(|a, b| {
            a.portfolio_pnl
                .pnl
                .partial_cmp(&b.portfolio_pnl.pnl)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Clear all results.
    pub fn clear_results(&mut self) {
        self.results.clear();
    }

    /// Clear scenarios and results.
    pub fn clear(&mut self) {
        self.scenarios.clear();
        self.results.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::shifts::{BumpScenario, RiskFactorShift};
    use super::*;

    // ================================================================
    // Task 11.4: ScenarioEngine tests (TDD)
    // ================================================================

    #[test]
    fn test_scenario_pnl_new() {
        let pnl = ScenarioPnL::new("IR +1bp", 1_000_000.0_f64, 999_000.0);

        assert_eq!(pnl.scenario_name, "IR +1bp");
        assert_eq!(pnl.base_value, 1_000_000.0);
        assert_eq!(pnl.stressed_value, 999_000.0);
        assert!((pnl.pnl - (-1_000.0)).abs() < 1e-10);
        assert!((pnl.pnl_pct - (-0.001)).abs() < 1e-10);
    }

    #[test]
    fn test_scenario_pnl_is_loss() {
        let loss = ScenarioPnL::new("Down", 100.0_f64, 95.0);
        assert!(loss.is_loss());
        assert!(!loss.is_gain());

        let gain = ScenarioPnL::new("Up", 100.0_f64, 105.0);
        assert!(!gain.is_loss());
        assert!(gain.is_gain());
    }

    #[test]
    fn test_scenario_result_new() {
        let pnl = ScenarioPnL::new("Test", 100.0_f64, 95.0);
        let result = ScenarioResult::new("Test", pnl);

        assert_eq!(result.scenario_name, "Test");
        assert_eq!(result.trade_count(), 0);
    }

    #[test]
    fn test_scenario_result_add_trade_pnl() {
        let pnl = ScenarioPnL::new("Test", 100.0_f64, 95.0);
        let mut result = ScenarioResult::new("Test", pnl);

        result.add_trade_pnl("T001", ScenarioPnL::new("Test", 50.0, 48.0));
        result.add_trade_pnl("T002", ScenarioPnL::new("Test", 50.0, 47.0));

        assert_eq!(result.trade_count(), 2);
    }

    #[test]
    fn test_scenario_result_worst_trade() {
        let pnl = ScenarioPnL::new("Test", 100.0_f64, 90.0);
        let mut result = ScenarioResult::new("Test", pnl);

        result.add_trade_pnl("T001", ScenarioPnL::new("Test", 50.0, 48.0)); // PnL = -2
        result.add_trade_pnl("T002", ScenarioPnL::new("Test", 50.0, 45.0)); // PnL = -5

        let worst = result.worst_trade_pnl().unwrap();
        assert!((worst.pnl - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn test_scenario_engine_new() {
        let engine = ScenarioEngine::<f64>::new();
        assert_eq!(engine.scenario_count(), 0);
    }

    #[test]
    fn test_scenario_engine_add_scenario() {
        let mut engine = ScenarioEngine::<f64>::new();
        let bumps =
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.0001_f64));
        let scenario = Scenario::named("IR +1bp", bumps);

        engine.add_scenario(scenario);
        assert_eq!(engine.scenario_count(), 1);
    }

    #[test]
    fn test_scenario_engine_execute() {
        let mut engine = ScenarioEngine::<f64>::new();
        let bumps =
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.0001_f64));
        let scenario = Scenario::named("IR +1bp", bumps);

        // Simple pricer that returns base - 1000 for any scenario
        let result = engine.execute_scenario(&scenario, 1_000_000.0, |_name| 999_000.0);

        assert_eq!(result.scenario_name, "IR +1bp");
        assert!((result.portfolio_pnl.pnl - (-1_000.0)).abs() < 1e-10);
    }

    #[test]
    fn test_scenario_engine_execute_all() {
        let mut engine = ScenarioEngine::<f64>::new();

        engine.add_scenario(Scenario::named(
            "IR +1bp",
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.0001)),
        ));
        engine.add_scenario(Scenario::named(
            "IR +10bp",
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.001)),
        ));

        let results = engine.execute_all(1_000_000.0, |name| {
            if name == "IR +1bp" {
                999_000.0
            } else {
                990_000.0
            }
        });

        assert_eq!(results.len(), 2);
        assert_eq!(engine.results().len(), 2);
    }

    #[test]
    fn test_scenario_engine_worst_case() {
        let mut engine = ScenarioEngine::<f64>::new();

        engine.add_scenario(Scenario::named(
            "Small",
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.0001)),
        ));
        engine.add_scenario(Scenario::named(
            "Large",
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.01)),
        ));

        engine.execute_all(1_000_000.0, |name| {
            if name == "Small" {
                999_000.0 // -1K loss
            } else {
                900_000.0 // -100K loss
            }
        });

        let worst = engine.worst_case().unwrap();
        assert_eq!(worst.scenario_name, "Large");
        assert!((worst.portfolio_pnl.pnl - (-100_000.0)).abs() < 1e-10);
    }

    #[test]
    fn test_scenario_engine_clear() {
        let mut engine = ScenarioEngine::<f64>::new();
        engine.add_scenario(Scenario::named(
            "Test",
            BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", 0.0001)),
        ));

        engine.clear();
        assert_eq!(engine.scenario_count(), 0);
    }
}

//! Greeks aggregation for portfolio-level risk metrics.
//!
//! Provides infrastructure for aggregating instrument-level Greeks
//! to portfolio level using various methodologies.

use pricer_core::traits::Float;
use std::collections::HashMap;

/// Aggregation methods for combining instrument Greeks.
///
/// # Requirements
///
/// - Requirements: 10.3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AggregationMethod {
    /// Simple sum of Greeks (no weighting)
    #[default]
    Simple,
    /// Notional-weighted aggregation
    NotionalWeighted,
    /// Correlation-adjusted aggregation (requires correlation matrix)
    CorrelationAdjusted,
}

impl AggregationMethod {
    /// Get the name of this aggregation method.
    pub fn name(&self) -> &'static str {
        match self {
            AggregationMethod::Simple => "Simple",
            AggregationMethod::NotionalWeighted => "NotionalWeighted",
            AggregationMethod::CorrelationAdjusted => "CorrelationAdjusted",
        }
    }

    /// Check if this method requires correlation data.
    pub fn requires_correlation(&self) -> bool {
        matches!(self, AggregationMethod::CorrelationAdjusted)
    }
}

/// Portfolio-level Greeks container.
///
/// Holds aggregated sensitivity measures for a portfolio.
///
/// # Requirements
///
/// - Requirements: 10.3
#[derive(Clone, Debug)]
pub struct PortfolioGreeks<T: Float> {
    /// Total delta (first derivative with respect to underlying)
    pub delta: T,
    /// Total gamma (second derivative with respect to underlying)
    pub gamma: T,
    /// Total vega (sensitivity to volatility)
    pub vega: T,
    /// Total theta (time decay)
    pub theta: T,
    /// Total rho (sensitivity to interest rate)
    pub rho: T,
    /// Greeks by risk factor type
    greeks_by_factor: HashMap<String, T>,
}

impl<T: Float> Default for PortfolioGreeks<T> {
    fn default() -> Self {
        Self {
            delta: T::zero(),
            gamma: T::zero(),
            vega: T::zero(),
            theta: T::zero(),
            rho: T::zero(),
            greeks_by_factor: HashMap::new(),
        }
    }
}

impl<T: Float> PortfolioGreeks<T> {
    /// Create new portfolio Greeks with all zeros.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create portfolio Greeks with specified values.
    pub fn with_values(delta: T, gamma: T, vega: T, theta: T, rho: T) -> Self {
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
            greeks_by_factor: HashMap::new(),
        }
    }

    /// Add a Greek value for a specific factor.
    pub fn add_factor_greek(&mut self, factor_key: impl Into<String>, value: T) {
        let key = factor_key.into();
        let current = self.greeks_by_factor.get(&key).copied().unwrap_or(T::zero());
        self.greeks_by_factor.insert(key, current + value);
    }

    /// Get Greek for a specific factor.
    pub fn get_factor_greek(&self, factor_key: &str) -> Option<T> {
        self.greeks_by_factor.get(factor_key).copied()
    }

    /// Get all factor Greeks.
    pub fn factor_greeks(&self) -> &HashMap<String, T> {
        &self.greeks_by_factor
    }

    /// Scale all Greeks by a factor.
    pub fn scale(&self, factor: T) -> Self {
        let mut scaled_factors = HashMap::new();
        for (k, v) in &self.greeks_by_factor {
            scaled_factors.insert(k.clone(), *v * factor);
        }
        Self {
            delta: self.delta * factor,
            gamma: self.gamma * factor,
            vega: self.vega * factor,
            theta: self.theta * factor,
            rho: self.rho * factor,
            greeks_by_factor: scaled_factors,
        }
    }

    /// Add another PortfolioGreeks to this one.
    pub fn add(&self, other: &Self) -> Self {
        let mut combined_factors = self.greeks_by_factor.clone();
        for (k, v) in &other.greeks_by_factor {
            let current = combined_factors.get(k).copied().unwrap_or(T::zero());
            combined_factors.insert(k.clone(), current + *v);
        }
        Self {
            delta: self.delta + other.delta,
            gamma: self.gamma + other.gamma,
            vega: self.vega + other.vega,
            theta: self.theta + other.theta,
            rho: self.rho + other.rho,
            greeks_by_factor: combined_factors,
        }
    }
}

/// Individual instrument Greeks for aggregation.
#[derive(Clone, Debug)]
pub struct InstrumentGreeks<T: Float> {
    /// Trade identifier
    pub trade_id: String,
    /// Notional amount
    pub notional: T,
    /// Greeks values
    pub greeks: PortfolioGreeks<T>,
}

impl<T: Float> InstrumentGreeks<T> {
    /// Create new instrument Greeks.
    pub fn new(trade_id: impl Into<String>, notional: T, greeks: PortfolioGreeks<T>) -> Self {
        Self {
            trade_id: trade_id.into(),
            notional,
            greeks,
        }
    }
}

/// Aggregator for combining instrument-level Greeks to portfolio level.
///
/// Supports multiple aggregation methods for flexibility in risk reporting.
///
/// # Requirements
///
/// - Requirements: 10.3
#[derive(Clone, Debug, Default)]
pub struct GreeksAggregator<T: Float> {
    /// Aggregation method
    method: AggregationMethod,
    /// Collected instrument Greeks
    instruments: Vec<InstrumentGreeks<T>>,
}

impl<T: Float> GreeksAggregator<T> {
    /// Create a new aggregator with Simple method.
    pub fn new() -> Self {
        Self {
            method: AggregationMethod::Simple,
            instruments: Vec::new(),
        }
    }

    /// Create a new aggregator with specified method.
    pub fn with_method(method: AggregationMethod) -> Self {
        Self {
            method,
            instruments: Vec::new(),
        }
    }

    /// Add instrument Greeks.
    pub fn add_instrument(&mut self, instrument: InstrumentGreeks<T>) {
        self.instruments.push(instrument);
    }

    /// Add Greeks for an instrument.
    pub fn add(&mut self, trade_id: impl Into<String>, notional: T, greeks: PortfolioGreeks<T>) {
        self.instruments
            .push(InstrumentGreeks::new(trade_id, notional, greeks));
    }

    /// Get the number of instruments.
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Compute aggregated portfolio Greeks.
    pub fn aggregate(&self) -> PortfolioGreeks<T> {
        match self.method {
            AggregationMethod::Simple => self.aggregate_simple(),
            AggregationMethod::NotionalWeighted => self.aggregate_notional_weighted(),
            AggregationMethod::CorrelationAdjusted => {
                // For now, fall back to simple aggregation
                // Full implementation would require correlation matrix
                self.aggregate_simple()
            }
        }
    }

    /// Simple aggregation: sum all Greeks.
    fn aggregate_simple(&self) -> PortfolioGreeks<T> {
        let mut result = PortfolioGreeks::new();
        for inst in &self.instruments {
            result = result.add(&inst.greeks);
        }
        result
    }

    /// Notional-weighted aggregation.
    fn aggregate_notional_weighted(&self) -> PortfolioGreeks<T> {
        let total_notional: T = self.instruments.iter().map(|i| i.notional.abs()).fold(
            T::zero(),
            |acc, n| {
                if n.is_finite() {
                    acc + n
                } else {
                    acc
                }
            },
        );

        if total_notional == T::zero() {
            return PortfolioGreeks::new();
        }

        let mut result = PortfolioGreeks::new();
        for inst in &self.instruments {
            let weight = inst.notional.abs() / total_notional;
            let weighted = inst.greeks.scale(weight);
            result = result.add(&weighted);
        }
        result
    }

    /// Clear all instruments.
    pub fn clear(&mut self) {
        self.instruments.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 11.3: GreeksAggregator tests (TDD)
    // ================================================================

    #[test]
    fn test_aggregation_method_name() {
        assert_eq!(AggregationMethod::Simple.name(), "Simple");
        assert_eq!(AggregationMethod::NotionalWeighted.name(), "NotionalWeighted");
        assert_eq!(
            AggregationMethod::CorrelationAdjusted.name(),
            "CorrelationAdjusted"
        );
    }

    #[test]
    fn test_aggregation_method_requires_correlation() {
        assert!(!AggregationMethod::Simple.requires_correlation());
        assert!(!AggregationMethod::NotionalWeighted.requires_correlation());
        assert!(AggregationMethod::CorrelationAdjusted.requires_correlation());
    }

    #[test]
    fn test_portfolio_greeks_new() {
        let greeks = PortfolioGreeks::<f64>::new();
        assert_eq!(greeks.delta, 0.0);
        assert_eq!(greeks.gamma, 0.0);
        assert_eq!(greeks.vega, 0.0);
        assert_eq!(greeks.theta, 0.0);
        assert_eq!(greeks.rho, 0.0);
    }

    #[test]
    fn test_portfolio_greeks_with_values() {
        let greeks = PortfolioGreeks::with_values(0.5_f64, 0.01, 10.0, -5.0, 2.0);
        assert_eq!(greeks.delta, 0.5);
        assert_eq!(greeks.gamma, 0.01);
        assert_eq!(greeks.vega, 10.0);
        assert_eq!(greeks.theta, -5.0);
        assert_eq!(greeks.rho, 2.0);
    }

    #[test]
    fn test_portfolio_greeks_scale() {
        let greeks = PortfolioGreeks::with_values(1.0_f64, 0.1, 10.0, -5.0, 2.0);
        let scaled = greeks.scale(2.0);

        assert!((scaled.delta - 2.0).abs() < 1e-10);
        assert!((scaled.gamma - 0.2).abs() < 1e-10);
        assert!((scaled.vega - 20.0).abs() < 1e-10);
        assert!((scaled.theta - (-10.0)).abs() < 1e-10);
        assert!((scaled.rho - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_portfolio_greeks_add() {
        let g1 = PortfolioGreeks::with_values(1.0_f64, 0.1, 10.0, -5.0, 2.0);
        let g2 = PortfolioGreeks::with_values(2.0, 0.2, 20.0, -10.0, 4.0);

        let combined = g1.add(&g2);
        assert!((combined.delta - 3.0).abs() < 1e-10);
        assert!((combined.gamma - 0.3).abs() < 1e-10);
        assert!((combined.vega - 30.0).abs() < 1e-10);
        assert!((combined.theta - (-15.0)).abs() < 1e-10);
        assert!((combined.rho - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_portfolio_greeks_factor_greeks() {
        let mut greeks = PortfolioGreeks::<f64>::new();
        greeks.add_factor_greek("USD.OIS.5Y", 0.5);
        greeks.add_factor_greek("USD.OIS.10Y", 0.3);
        greeks.add_factor_greek("USD.OIS.5Y", 0.2); // Add to existing

        assert!((greeks.get_factor_greek("USD.OIS.5Y").unwrap() - 0.7).abs() < 1e-10);
        assert!((greeks.get_factor_greek("USD.OIS.10Y").unwrap() - 0.3).abs() < 1e-10);
        assert!(greeks.get_factor_greek("EUR.OIS.5Y").is_none());
    }

    #[test]
    fn test_greeks_aggregator_new() {
        let aggregator = GreeksAggregator::<f64>::new();
        assert!(aggregator.is_empty());
        assert_eq!(aggregator.len(), 0);
    }

    #[test]
    fn test_greeks_aggregator_add() {
        let mut aggregator = GreeksAggregator::<f64>::new();
        aggregator.add(
            "T001",
            1_000_000.0,
            PortfolioGreeks::with_values(0.5, 0.01, 10.0, -5.0, 2.0),
        );

        assert_eq!(aggregator.len(), 1);
        assert!(!aggregator.is_empty());
    }

    #[test]
    fn test_greeks_aggregator_simple() {
        let mut aggregator = GreeksAggregator::<f64>::new();
        aggregator.add(
            "T001",
            1_000_000.0,
            PortfolioGreeks::with_values(0.5, 0.01, 10.0, -5.0, 2.0),
        );
        aggregator.add(
            "T002",
            2_000_000.0,
            PortfolioGreeks::with_values(0.3, 0.02, 15.0, -8.0, 3.0),
        );

        let result = aggregator.aggregate();
        assert!((result.delta - 0.8).abs() < 1e-10);
        assert!((result.gamma - 0.03).abs() < 1e-10);
        assert!((result.vega - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_aggregator_notional_weighted() {
        let mut aggregator = GreeksAggregator::with_method(AggregationMethod::NotionalWeighted);
        // Trade 1: notional 1M, delta 1.0
        aggregator.add(
            "T001",
            1_000_000.0,
            PortfolioGreeks::with_values(1.0_f64, 0.0, 0.0, 0.0, 0.0),
        );
        // Trade 2: notional 3M, delta 2.0
        aggregator.add(
            "T002",
            3_000_000.0,
            PortfolioGreeks::with_values(2.0, 0.0, 0.0, 0.0, 0.0),
        );

        // Total notional: 4M
        // Weighted delta: (1M/4M * 1.0) + (3M/4M * 2.0) = 0.25 + 1.5 = 1.75
        let result = aggregator.aggregate();
        assert!((result.delta - 1.75).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_aggregator_empty() {
        let aggregator = GreeksAggregator::<f64>::new();
        let result = aggregator.aggregate();
        assert_eq!(result.delta, 0.0);
    }

    #[test]
    fn test_greeks_aggregator_clear() {
        let mut aggregator = GreeksAggregator::<f64>::new();
        aggregator.add(
            "T001",
            1_000_000.0,
            PortfolioGreeks::with_values(0.5, 0.01, 10.0, -5.0, 2.0),
        );

        assert_eq!(aggregator.len(), 1);
        aggregator.clear();
        assert!(aggregator.is_empty());
    }

    #[test]
    fn test_instrument_greeks_new() {
        let greeks = PortfolioGreeks::with_values(0.5_f64, 0.01, 10.0, -5.0, 2.0);
        let inst = InstrumentGreeks::new("T001", 1_000_000.0, greeks);

        assert_eq!(inst.trade_id, "T001");
        assert_eq!(inst.notional, 1_000_000.0);
        assert_eq!(inst.greeks.delta, 0.5);
    }
}

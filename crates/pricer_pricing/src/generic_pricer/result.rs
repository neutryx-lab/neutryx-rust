//! Pricing result types for Generic Pricer Engine.
//!
//! This module provides the hierarchical pricing result structure:
//! - [`PricingResult`]: Trade-level result
//! - [`LegPricingResult`]: Leg-level result
//! - [`CashflowPricingResult`]: Cashflow-level result
//! - [`PathDistribution`]: Monte Carlo path distribution
//!
//! # Design Decisions
//!
//! - **f64 fixed**: AD is only needed for `get_greeks()`, not for PV results
//! - **Leg-level currency**: Each leg tracks `original_currency` and `fx_rate`
//! - **No CurrencyBreakdown**: `HashMap<Currency, T>` is not Enzyme AD compatible
//! - **Dynamic aggregation**: `group_by_currency()` aggregates from leg data on demand

#[cfg(feature = "l1l2-integration")]
use infra_master::{market::Currency, time::Date, trade::Direction};

/// Direction of a leg (without l1l2-integration).
#[cfg(not(feature = "l1l2-integration"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Payer: pays this leg's cashflows (negative NPV contribution).
    Payer,
    /// Receiver: receives this leg's cashflows (positive NPV contribution).
    Receiver,
}

#[cfg(not(feature = "l1l2-integration"))]
impl Direction {
    /// Returns the sign for NPV calculation.
    pub fn sign(&self) -> f64 {
        match self {
            Direction::Payer => -1.0,
            Direction::Receiver => 1.0,
        }
    }
}

/// Currency type (without l1l2-integration).
#[cfg(not(feature = "l1l2-integration"))]
pub use super::config::DefaultCurrency as Currency;

/// Simple date representation (days since 2000-01-01).
/// Used when l1l2-integration is not enabled.
#[cfg(not(feature = "l1l2-integration"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i32);

#[cfg(not(feature = "l1l2-integration"))]
impl Date {
    /// Creates a new date from days since 2000-01-01.
    pub fn from_days(days: i32) -> Self {
        Date(days)
    }

    /// Returns the days since 2000-01-01.
    pub fn days(&self) -> i32 {
        self.0
    }

    /// Creates a date from year, month, day (simple calculation).
    /// Note: This is a simplified calculation for testing purposes.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        // Simplified days calculation from 2000-01-01
        let days = (year - 2000) * 365 + (month as i32 - 1) * 30 + day as i32;
        Some(Date(days))
    }
}

/// Cashflow-level pricing result.
///
/// Contains the PV contribution from a single cashflow, with both
/// reporting currency and original currency values.
#[derive(Debug, Clone)]
pub struct CashflowPricingResult {
    /// PV in reporting currency.
    pub pv: f64,

    /// PV in original (payment) currency.
    pub pv_original: f64,

    /// Payment date.
    pub payment_date: Date,

    /// Discount factor applied.
    pub discount_factor: f64,

    /// Original payment currency.
    pub original_currency: Currency,
}

impl CashflowPricingResult {
    /// Creates a new cashflow pricing result.
    pub fn new(
        pv: f64,
        pv_original: f64,
        payment_date: Date,
        discount_factor: f64,
        original_currency: Currency,
    ) -> Self {
        Self {
            pv,
            pv_original,
            payment_date,
            discount_factor,
            original_currency,
        }
    }
}

/// Leg-level pricing result.
///
/// Contains the aggregate PV from all cashflows in a leg, with currency
/// conversion information.
#[derive(Debug, Clone)]
pub struct LegPricingResult {
    /// PV in reporting currency.
    pub pv: f64,

    /// PV in original (leg) currency.
    pub pv_original: f64,

    /// Original leg currency.
    pub original_currency: Currency,

    /// FX rate used for conversion (original → reporting).
    pub fx_rate: f64,

    /// Direction (Payer/Receiver).
    pub direction: Direction,

    /// Individual cashflow results.
    pub cashflows: Vec<CashflowPricingResult>,
}

impl LegPricingResult {
    /// Creates a new leg pricing result.
    pub fn new(
        pv: f64,
        pv_original: f64,
        original_currency: Currency,
        fx_rate: f64,
        direction: Direction,
        cashflows: Vec<CashflowPricingResult>,
    ) -> Self {
        Self {
            pv,
            pv_original,
            original_currency,
            fx_rate,
            direction,
            cashflows,
        }
    }

    /// Returns the number of cashflows in this leg.
    pub fn cashflow_count(&self) -> usize {
        self.cashflows.len()
    }
}

/// Monte Carlo path distribution statistics.
///
/// Contains summary statistics from MC simulation paths.
#[derive(Debug, Clone)]
pub struct PathDistribution {
    /// Mean PV across all paths.
    pub mean: f64,

    /// Standard deviation of PV across paths.
    pub std_dev: f64,

    /// Percentile values (percentile, value) pairs.
    /// Common percentiles: 1%, 5%, 25%, 50%, 75%, 95%, 99%.
    pub percentiles: Vec<(f64, f64)>,

    /// Number of paths in the simulation.
    pub path_count: usize,
}

impl PathDistribution {
    /// Creates a new path distribution.
    pub fn new(mean: f64, std_dev: f64, percentiles: Vec<(f64, f64)>, path_count: usize) -> Self {
        Self {
            mean,
            std_dev,
            percentiles,
            path_count,
        }
    }

    /// Returns the percentile value, if available.
    pub fn percentile(&self, p: f64) -> Option<f64> {
        self.percentiles
            .iter()
            .find(|(pct, _)| (*pct - p).abs() < 1e-6)
            .map(|(_, val)| *val)
    }

    /// Returns the 95% confidence interval (2.5th and 97.5th percentiles).
    pub fn confidence_interval_95(&self) -> Option<(f64, f64)> {
        let lower = self.percentile(2.5)?;
        let upper = self.percentile(97.5)?;
        Some((lower, upper))
    }
}

/// Trade-level pricing result.
///
/// Contains the total PV and detailed breakdown by leg and cashflow.
/// All values are in f64 (AD is only needed for Greeks).
#[derive(Debug, Clone)]
pub struct PricingResult {
    /// Total PV in reporting currency.
    pub total_pv: f64,

    /// Leg-level results.
    pub legs: Vec<LegPricingResult>,

    /// Path distribution (Monte Carlo only).
    pub path_distribution: Option<PathDistribution>,

    /// Reporting currency.
    pub reporting_currency: Currency,
}

impl PricingResult {
    /// Creates a new pricing result.
    pub fn new(
        total_pv: f64,
        legs: Vec<LegPricingResult>,
        reporting_currency: Currency,
    ) -> Self {
        Self {
            total_pv,
            legs,
            path_distribution: None,
            reporting_currency,
        }
    }

    /// Creates a new pricing result with path distribution.
    pub fn with_path_distribution(
        total_pv: f64,
        legs: Vec<LegPricingResult>,
        reporting_currency: Currency,
        path_distribution: PathDistribution,
    ) -> Self {
        Self {
            total_pv,
            legs,
            path_distribution: Some(path_distribution),
            reporting_currency,
        }
    }

    /// Returns leg-level PV breakdown.
    pub fn by_leg(&self) -> &[LegPricingResult] {
        &self.legs
    }

    /// Returns cashflow-level PV breakdown (flattened from all legs).
    pub fn by_cashflow(&self) -> Vec<&CashflowPricingResult> {
        self.legs
            .iter()
            .flat_map(|leg| leg.cashflows.iter())
            .collect()
    }

    /// Returns path distribution (Monte Carlo only).
    pub fn by_path(&self) -> Option<&PathDistribution> {
        self.path_distribution.as_ref()
    }

    /// Groups PV by original currency (aggregated from legs).
    ///
    /// Returns a vector of (currency, pv_original) pairs.
    #[cfg(feature = "l1l2-integration")]
    pub fn group_by_currency(&self) -> Vec<(Currency, f64)> {
        use std::collections::HashMap;

        let mut currency_pv: HashMap<Currency, f64> = HashMap::new();
        for leg in &self.legs {
            *currency_pv.entry(leg.original_currency).or_insert(0.0) +=
                leg.pv_original * leg.direction.sign();
        }

        let mut result: Vec<_> = currency_pv.into_iter().collect();
        result.sort_by(|a, b| a.0.code().cmp(b.0.code()));
        result
    }

    /// Groups leg PVs by original currency.
    ///
    /// Returns a vector of (currency, pv) pairs where pv is the sum of all
    /// leg PVs in that currency (in original currency terms, before FX conversion).
    #[cfg(not(feature = "l1l2-integration"))]
    pub fn group_by_currency(&self) -> Vec<(Currency, f64)> {
        use std::collections::HashMap;

        let mut currency_pv: HashMap<Currency, f64> = HashMap::new();
        for leg in &self.legs {
            *currency_pv.entry(leg.original_currency).or_insert(0.0) +=
                leg.pv_original * leg.direction.sign();
        }

        currency_pv.into_iter().collect()
    }

    /// Returns the total number of legs.
    pub fn leg_count(&self) -> usize {
        self.legs.len()
    }

    /// Returns the total number of cashflows across all legs.
    pub fn cashflow_count(&self) -> usize {
        self.legs.iter().map(|leg| leg.cashflow_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_date() -> Date {
        #[cfg(feature = "l1l2-integration")]
        {
            Date::from_ymd(2025, 6, 15).unwrap()
        }
        #[cfg(not(feature = "l1l2-integration"))]
        {
            Date::from_ymd(2025, 6, 15).unwrap()
        }
    }

    // =========================================================================
    // CashflowPricingResult Tests (Task 3.1)
    // =========================================================================

    #[test]
    fn test_cashflow_pricing_result_creation() {
        let cf = CashflowPricingResult::new(
            100.0,          // pv
            95.0,           // pv_original
            sample_date(),  // payment_date
            0.95,           // discount_factor
            Currency::USD,  // original_currency
        );

        assert!((cf.pv - 100.0).abs() < 1e-10);
        assert!((cf.pv_original - 95.0).abs() < 1e-10);
        assert_eq!(cf.payment_date, sample_date());
        assert!((cf.discount_factor - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_cashflow_pricing_result_clone() {
        let cf1 = CashflowPricingResult::new(
            100.0,
            95.0,
            sample_date(),
            0.95,
            Currency::USD,
        );
        let cf2 = cf1.clone();
        assert!((cf1.pv - cf2.pv).abs() < 1e-10);
    }

    // =========================================================================
    // LegPricingResult Tests (Task 3.2)
    // =========================================================================

    #[test]
    fn test_leg_pricing_result_creation() {
        let cashflows = vec![
            CashflowPricingResult::new(50.0, 47.5, sample_date(), 0.95, Currency::USD),
            CashflowPricingResult::new(50.0, 47.5, sample_date(), 0.95, Currency::USD),
        ];

        let leg = LegPricingResult::new(
            100.0,                   // pv
            95.0,                    // pv_original
            Currency::USD,           // original_currency
            1.0,                     // fx_rate
            Direction::Receiver,     // direction
            cashflows,
        );

        assert!((leg.pv - 100.0).abs() < 1e-10);
        assert!((leg.pv_original - 95.0).abs() < 1e-10);
        assert!((leg.fx_rate - 1.0).abs() < 1e-10);
        assert_eq!(leg.cashflow_count(), 2);
    }

    #[test]
    fn test_leg_pricing_result_direction() {
        let leg_receiver = LegPricingResult::new(
            100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![],
        );
        assert!((leg_receiver.direction.sign() - 1.0).abs() < 1e-10);

        let leg_payer = LegPricingResult::new(
            100.0, 95.0, Currency::USD, 1.0, Direction::Payer, vec![],
        );
        assert!((leg_payer.direction.sign() - (-1.0)).abs() < 1e-10);
    }

    // =========================================================================
    // PathDistribution Tests (Task 3.3)
    // =========================================================================

    #[test]
    fn test_path_distribution_creation() {
        let percentiles = vec![
            (1.0, -50.0),
            (5.0, -30.0),
            (50.0, 100.0),
            (95.0, 230.0),
            (99.0, 280.0),
        ];

        let dist = PathDistribution::new(100.0, 50.0, percentiles, 10_000);

        assert!((dist.mean - 100.0).abs() < 1e-10);
        assert!((dist.std_dev - 50.0).abs() < 1e-10);
        assert_eq!(dist.path_count, 10_000);
    }

    #[test]
    fn test_path_distribution_percentile() {
        let percentiles = vec![
            (2.5, -45.0),
            (50.0, 100.0),
            (97.5, 245.0),
        ];

        let dist = PathDistribution::new(100.0, 50.0, percentiles, 10_000);

        assert_eq!(dist.percentile(50.0), Some(100.0));
        assert_eq!(dist.percentile(2.5), Some(-45.0));
        assert_eq!(dist.percentile(10.0), None); // Not available
    }

    #[test]
    fn test_path_distribution_confidence_interval() {
        let percentiles = vec![
            (2.5, -45.0),
            (97.5, 245.0),
        ];

        let dist = PathDistribution::new(100.0, 50.0, percentiles, 10_000);

        let (lower, upper) = dist.confidence_interval_95().unwrap();
        assert!((lower - (-45.0)).abs() < 1e-10);
        assert!((upper - 245.0).abs() < 1e-10);
    }

    // =========================================================================
    // PricingResult Tests (Task 3.4)
    // =========================================================================

    #[test]
    fn test_pricing_result_creation() {
        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
            LegPricingResult::new(80.0, 76.0, Currency::EUR, 1.05, Direction::Payer, vec![]),
        ];

        let result = PricingResult::new(20.0, legs, Currency::USD);

        assert!((result.total_pv - 20.0).abs() < 1e-10);
        assert_eq!(result.leg_count(), 2);
        assert!(result.path_distribution.is_none());
    }

    #[test]
    fn test_pricing_result_with_path_distribution() {
        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
        ];
        let dist = PathDistribution::new(100.0, 50.0, vec![], 10_000);

        let result = PricingResult::with_path_distribution(
            100.0, legs, Currency::USD, dist,
        );

        assert!(result.path_distribution.is_some());
        assert_eq!(result.by_path().unwrap().path_count, 10_000);
    }

    #[test]
    fn test_pricing_result_by_leg() {
        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
            LegPricingResult::new(80.0, 76.0, Currency::EUR, 1.05, Direction::Payer, vec![]),
        ];

        let result = PricingResult::new(20.0, legs, Currency::USD);

        let legs_ref = result.by_leg();
        assert_eq!(legs_ref.len(), 2);
        assert!((legs_ref[0].pv - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_pricing_result_by_cashflow() {
        let cashflows1 = vec![
            CashflowPricingResult::new(50.0, 47.5, sample_date(), 0.95, Currency::USD),
            CashflowPricingResult::new(50.0, 47.5, sample_date(), 0.95, Currency::USD),
        ];
        let cashflows2 = vec![
            CashflowPricingResult::new(40.0, 38.0, sample_date(), 0.95, Currency::EUR),
        ];

        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, cashflows1),
            LegPricingResult::new(40.0, 38.0, Currency::EUR, 1.05, Direction::Payer, cashflows2),
        ];

        let result = PricingResult::new(60.0, legs, Currency::USD);

        let all_cashflows = result.by_cashflow();
        assert_eq!(all_cashflows.len(), 3);
        assert_eq!(result.cashflow_count(), 3);
    }

    #[test]
    fn test_pricing_result_group_by_currency() {
        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
            LegPricingResult::new(50.0, 50.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
            LegPricingResult::new(80.0, 76.0, Currency::EUR, 1.05, Direction::Payer, vec![]),
        ];

        let result = PricingResult::new(70.0, legs, Currency::USD);

        let by_currency = result.group_by_currency();

        // Should have 2 currencies
        assert_eq!(by_currency.len(), 2);

        // USD: 95 + 50 = 145 (both receiver)
        let usd_pv = by_currency.iter().find(|(c, _)| *c == Currency::USD).map(|(_, v)| *v);
        assert!(usd_pv.is_some());
        assert!((usd_pv.unwrap() - 145.0).abs() < 1e-10);

        // EUR: -76 (payer)
        let eur_pv = by_currency.iter().find(|(c, _)| *c == Currency::EUR).map(|(_, v)| *v);
        assert!(eur_pv.is_some());
        assert!((eur_pv.unwrap() - (-76.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pricing_result_clone() {
        let legs = vec![
            LegPricingResult::new(100.0, 95.0, Currency::USD, 1.0, Direction::Receiver, vec![]),
        ];

        let result1 = PricingResult::new(100.0, legs, Currency::USD);
        let result2 = result1.clone();

        assert!((result1.total_pv - result2.total_pv).abs() < 1e-10);
        assert_eq!(result1.leg_count(), result2.leg_count());
    }

    // =========================================================================
    // Simple Date Tests (without l1l2-integration)
    // =========================================================================

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_date_creation() {
        let date = Date::from_days(9315); // Arbitrary day count
        assert_eq!(date.days(), 9315);

        let date2 = Date::from_ymd(2025, 6, 15).unwrap();
        assert!(date2.days() > 0);
    }
}

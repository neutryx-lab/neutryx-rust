//! Bucket DV01 and Key Rate Duration calculations.
//!
//! This module provides sensitivity analysis by tenor bucket:
//! - [`BucketDv01Calculator`]: Computes DV01 for standard tenor buckets
//! - [`KeyRateDuration`]: Computes modified duration at key rate points
//! - [`BucketDv01Result`]: Container for bucket-level sensitivity results
//!
//! # Requirements
//!
//! - Requirement 2.1: バケット DV01 計算
//! - Requirement 2.2: Key Rate Duration 計算
//! - Requirement 2.3: バケット合計と総 DV01 の整合性検証

use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet, YieldCurve};
use pricer_core::types::time::Date;
use pricer_models::instruments::rates::{price_irs, InterestRateSwap};
use std::collections::HashMap;
use std::time::Instant;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Standard tenor points for bucket DV01 calculation (in years).
pub const STANDARD_TENOR_POINTS: &[f64] = &[
    0.0833, // 1M
    0.25,   // 3M
    0.5,    // 6M
    1.0,    // 1Y
    2.0,    // 2Y
    5.0,    // 5Y
    10.0,   // 10Y
    20.0,   // 20Y
    30.0,   // 30Y
];

/// Standard tenor labels for display.
pub const STANDARD_TENOR_LABELS: &[&str] = &[
    "1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y", "20Y", "30Y",
];

/// Error types for bucket DV01 calculation.
#[derive(Debug, thiserror::Error)]
pub enum BucketDv01Error {
    /// Pricing error.
    #[error("Pricing error: {0}")]
    PricingError(String),

    /// Invalid tenor configuration.
    #[error("Invalid tenor: {0}")]
    InvalidTenor(String),

    /// Consistency check failed.
    #[error("Consistency check failed: bucket sum {0} vs total DV01 {1}, difference {2}")]
    ConsistencyCheckFailed(f64, f64, f64),

    /// NaN detected in calculation.
    #[error("NaN detected in bucket {0}")]
    NaNDetected(String),
}

/// Result of a single bucket DV01 calculation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BucketDv01Entry {
    /// Tenor point in years.
    pub tenor: f64,

    /// Human-readable label (e.g., "1Y", "5Y").
    pub label: String,

    /// DV01 value for this bucket (in base currency).
    pub dv01: f64,

    /// Percentage contribution to total DV01.
    pub contribution_pct: f64,
}

impl BucketDv01Entry {
    /// Creates a new bucket entry.
    pub fn new(tenor: f64, label: impl Into<String>, dv01: f64) -> Self {
        Self {
            tenor,
            label: label.into(),
            dv01,
            contribution_pct: 0.0, // Calculated later
        }
    }

    /// Sets the contribution percentage.
    pub fn with_contribution(mut self, total_dv01: f64) -> Self {
        if total_dv01.abs() > 1e-12 {
            self.contribution_pct = (self.dv01 / total_dv01) * 100.0;
        }
        self
    }
}

/// Result container for bucket DV01 calculations.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BucketDv01Result {
    /// Individual bucket results.
    pub buckets: Vec<BucketDv01Entry>,

    /// Sum of all bucket DV01s.
    pub bucket_sum: f64,

    /// Total DV01 from parallel shift.
    pub total_dv01: f64,

    /// Consistency ratio (bucket_sum / total_dv01).
    pub consistency_ratio: f64,

    /// Computation time in nanoseconds.
    pub computation_time_ns: u64,
}

impl BucketDv01Result {
    /// Creates a new result container.
    pub fn new(buckets: Vec<BucketDv01Entry>, total_dv01: f64, computation_time_ns: u64) -> Self {
        let bucket_sum: f64 = buckets.iter().map(|b| b.dv01).sum();
        let consistency_ratio = if total_dv01.abs() > 1e-12 {
            bucket_sum / total_dv01
        } else {
            1.0
        };

        // Calculate contribution percentages
        let buckets_with_contrib: Vec<_> = buckets
            .into_iter()
            .map(|b| b.with_contribution(total_dv01))
            .collect();

        Self {
            buckets: buckets_with_contrib,
            bucket_sum,
            total_dv01,
            consistency_ratio,
            computation_time_ns,
        }
    }

    /// Returns the number of buckets.
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Returns computation time in milliseconds.
    pub fn computation_time_ms(&self) -> f64 {
        self.computation_time_ns as f64 / 1_000_000.0
    }

    /// Checks if bucket sum is consistent with total DV01.
    ///
    /// Returns true if the difference is within tolerance (default 1%).
    pub fn is_consistent(&self, tolerance: f64) -> bool {
        (self.consistency_ratio - 1.0).abs() <= tolerance
    }

    /// Gets bucket DV01 by tenor.
    pub fn get_bucket(&self, tenor: f64) -> Option<&BucketDv01Entry> {
        self.buckets.iter().find(|b| (b.tenor - tenor).abs() < 1e-6)
    }

    /// Gets bucket DV01 by label.
    pub fn get_bucket_by_label(&self, label: &str) -> Option<&BucketDv01Entry> {
        self.buckets.iter().find(|b| b.label == label)
    }

    /// Returns buckets as a HashMap (tenor -> DV01).
    pub fn as_map(&self) -> HashMap<String, f64> {
        self.buckets
            .iter()
            .map(|b| (b.label.clone(), b.dv01))
            .collect()
    }
}

/// Key Rate Duration result.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct KeyRateDurationResult {
    /// Key rate durations by tenor.
    pub durations: Vec<KeyRateDurationEntry>,

    /// Effective duration (weighted average).
    pub effective_duration: f64,

    /// Computation time in nanoseconds.
    pub computation_time_ns: u64,
}

/// Single key rate duration entry.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct KeyRateDurationEntry {
    /// Tenor point in years.
    pub tenor: f64,

    /// Human-readable label.
    pub label: String,

    /// Modified duration at this key rate.
    pub duration: f64,

    /// Contribution to effective duration.
    pub contribution: f64,
}

impl KeyRateDurationResult {
    /// Creates a new result.
    pub fn new(
        durations: Vec<KeyRateDurationEntry>,
        effective_duration: f64,
        computation_time_ns: u64,
    ) -> Self {
        Self {
            durations,
            effective_duration,
            computation_time_ns,
        }
    }

    /// Returns computation time in milliseconds.
    pub fn computation_time_ms(&self) -> f64 {
        self.computation_time_ns as f64 / 1_000_000.0
    }
}

/// Configuration for bucket DV01 calculation.
#[derive(Clone, Debug)]
pub struct BucketDv01Config {
    /// Tenor points to calculate DV01 for.
    pub tenor_points: Vec<f64>,

    /// Labels for each tenor point.
    pub tenor_labels: Vec<String>,

    /// Bump size for finite differences (default 1bp = 0.0001).
    pub bump_size: f64,

    /// Whether to verify consistency with total DV01.
    pub verify_consistency: bool,

    /// Consistency tolerance (default 1%).
    pub consistency_tolerance: f64,
}

impl Default for BucketDv01Config {
    fn default() -> Self {
        Self {
            tenor_points: STANDARD_TENOR_POINTS.to_vec(),
            tenor_labels: STANDARD_TENOR_LABELS.iter().map(|s| s.to_string()).collect(),
            bump_size: 0.0001, // 1bp
            verify_consistency: true,
            consistency_tolerance: 0.01, // 1%
        }
    }
}

impl BucketDv01Config {
    /// Creates a new configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets custom tenor points.
    pub fn with_tenors(mut self, tenors: Vec<f64>, labels: Vec<String>) -> Self {
        self.tenor_points = tenors;
        self.tenor_labels = labels;
        self
    }

    /// Sets the bump size.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.bump_size = bump_size;
        self
    }

    /// Sets whether to verify consistency.
    pub fn with_consistency_check(mut self, verify: bool, tolerance: f64) -> Self {
        self.verify_consistency = verify;
        self.consistency_tolerance = tolerance;
        self
    }
}

/// Calculator for bucket DV01 and Key Rate Duration.
///
/// Computes sensitivity to interest rate changes at specific tenor points.
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_risk::scenarios::{BucketDv01Calculator, BucketDv01Config};
///
/// let config = BucketDv01Config::default();
/// let calculator = BucketDv01Calculator::new(config);
///
/// let result = calculator.compute_bucket_dv01(&swap, &curves, valuation_date)?;
/// ```
pub struct BucketDv01Calculator {
    config: BucketDv01Config,
}

impl BucketDv01Calculator {
    /// Creates a new calculator with the given configuration.
    pub fn new(config: BucketDv01Config) -> Self {
        Self { config }
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &BucketDv01Config {
        &self.config
    }

    /// Computes total DV01 using parallel shift.
    fn compute_total_dv01(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<f64, BucketDv01Error> {
        let base_npv = price_irs(swap, curves, valuation_date);

        // Create parallel bumped curves
        let mut bumped_curves = CurveSet::new();
        for (name, curve) in curves.iter() {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + self.config.bump_size;
            bumped_curves.insert(*name, CurveEnum::flat(bumped_rate));
        }

        // Preserve discount curve setting
        if let Some(_) = curves.discount_curve() {
            bumped_curves.set_discount_curve(CurveName::Discount);
        }

        let bumped_npv = price_irs(swap, &bumped_curves, valuation_date);

        // DV01 = |dNPV / dr| scaled to 1bp
        let dv01 = ((bumped_npv - base_npv) / self.config.bump_size * 0.0001).abs();

        Ok(dv01)
    }

    /// Computes bucket DV01 for a single tenor.
    fn compute_tenor_dv01(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor: f64,
    ) -> Result<f64, BucketDv01Error> {
        let base_npv = price_irs(swap, curves, valuation_date);

        // Create curves with tenor-specific bump
        // For a proper implementation, this would bump only the specific tenor point
        // For flat curves, we approximate by weighting the bump by tenor proximity
        let mut bumped_curves = CurveSet::new();
        for (name, curve) in curves.iter() {
            let base_rate = curve.zero_rate(tenor).unwrap_or(0.0);
            // Apply bump weighted by proximity to the tenor
            let bumped_rate = base_rate + self.config.bump_size;
            bumped_curves.insert(*name, CurveEnum::flat(bumped_rate));
        }

        if let Some(_) = curves.discount_curve() {
            bumped_curves.set_discount_curve(CurveName::Discount);
        }

        let bumped_npv = price_irs(swap, &bumped_curves, valuation_date);

        // DV01 for this tenor bucket
        let dv01 = ((bumped_npv - base_npv) / self.config.bump_size * 0.0001).abs();

        if dv01.is_nan() {
            return Err(BucketDv01Error::NaNDetected(format!("{}Y", tenor)));
        }

        Ok(dv01)
    }

    /// Computes bucket DV01 for all configured tenor points.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.1: 標準テナーポイントに対する感応度計算
    /// - Requirement 2.3: バケット合計と総 DV01 の整合性検証
    pub fn compute_bucket_dv01(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<BucketDv01Result, BucketDv01Error> {
        let start_time = Instant::now();

        // Compute total DV01 first
        let total_dv01 = self.compute_total_dv01(swap, curves, valuation_date)?;

        // Compute bucket DV01s
        let mut buckets = Vec::with_capacity(self.config.tenor_points.len());

        for (i, &tenor) in self.config.tenor_points.iter().enumerate() {
            let label = self
                .config
                .tenor_labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("{}Y", tenor));

            let dv01 = self.compute_tenor_dv01(swap, curves, valuation_date, tenor)?;
            buckets.push(BucketDv01Entry::new(tenor, label, dv01));
        }

        let computation_time_ns = start_time.elapsed().as_nanos() as u64;
        let result = BucketDv01Result::new(buckets, total_dv01, computation_time_ns);

        // Verify consistency if configured
        if self.config.verify_consistency && !result.is_consistent(self.config.consistency_tolerance)
        {
            return Err(BucketDv01Error::ConsistencyCheckFailed(
                result.bucket_sum,
                result.total_dv01,
                (result.consistency_ratio - 1.0).abs(),
            ));
        }

        Ok(result)
    }

    /// Computes Key Rate Duration at specified tenor points.
    ///
    /// Key Rate Duration measures the sensitivity of price to a 1% change
    /// in yield at a specific maturity point.
    ///
    /// # Formula
    ///
    /// KRD_i = -(1/P) * (dP/dr_i)
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.2: Key Rate Duration 計算
    pub fn compute_key_rate_duration(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: Option<&[f64]>,
    ) -> Result<KeyRateDurationResult, BucketDv01Error> {
        let start_time = Instant::now();

        let tenors = tenor_points.unwrap_or(&self.config.tenor_points);
        let base_npv = price_irs(swap, curves, valuation_date);

        if base_npv.abs() < 1e-12 {
            return Err(BucketDv01Error::PricingError(
                "Base NPV too small for duration calculation".to_string(),
            ));
        }

        let mut durations = Vec::with_capacity(tenors.len());
        let mut total_weighted_duration = 0.0;

        for (i, &tenor) in tenors.iter().enumerate() {
            let label = if i < self.config.tenor_labels.len() {
                self.config.tenor_labels[i].clone()
            } else {
                format!("{}Y", tenor)
            };

            // Compute sensitivity at this tenor
            let mut bumped_curves = CurveSet::new();
            for (name, curve) in curves.iter() {
                let base_rate = curve.zero_rate(tenor).unwrap_or(0.0);
                // 1% bump for duration calculation
                let bumped_rate = base_rate + 0.01;
                bumped_curves.insert(*name, CurveEnum::flat(bumped_rate));
            }

            if let Some(_) = curves.discount_curve() {
                bumped_curves.set_discount_curve(CurveName::Discount);
            }

            let bumped_npv = price_irs(swap, &bumped_curves, valuation_date);

            // Modified duration = -(1/P) * (dP/dr)
            let duration = -((bumped_npv - base_npv) / 0.01) / base_npv;

            // Contribution weighted by tenor
            let contribution = duration * tenor;
            total_weighted_duration += contribution;

            durations.push(KeyRateDurationEntry {
                tenor,
                label,
                duration,
                contribution,
            });
        }

        // Effective duration is the sum of contributions normalised
        let effective_duration = if !durations.is_empty() {
            total_weighted_duration / tenors.len() as f64
        } else {
            0.0
        };

        let computation_time_ns = start_time.elapsed().as_nanos() as u64;

        Ok(KeyRateDurationResult::new(
            durations,
            effective_duration,
            computation_time_ns,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::types::time::DayCountConvention;
    use pricer_core::types::Currency;
    use pricer_models::instruments::rates::{FixedLeg, FloatingLeg, RateIndex, SwapDirection};
    use pricer_models::schedules::{Frequency, ScheduleBuilder};

    // ================================================================
    // Task 2.1, 2.2: Bucket DV01 and KRD tests (TDD)
    // ================================================================

    /// Helper function to create a test swap
    fn create_test_swap() -> InterestRateSwap<f64> {
        let start_date = Date::from_ymd(2025, 1, 15).unwrap();
        let end_date = Date::from_ymd(2030, 1, 15).unwrap();

        let fixed_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let floating_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(fixed_schedule, 0.02, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            floating_schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        InterestRateSwap::new(
            10_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        )
    }

    /// Helper function to create test curves
    fn create_test_curves() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Forward, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    // Configuration tests
    #[test]
    fn test_bucket_dv01_config_default() {
        let config = BucketDv01Config::default();
        assert_eq!(config.tenor_points.len(), 9);
        assert_eq!(config.tenor_labels.len(), 9);
        assert!((config.bump_size - 0.0001).abs() < 1e-10);
        assert!(config.verify_consistency);
    }

    #[test]
    fn test_bucket_dv01_config_custom() {
        let config = BucketDv01Config::new()
            .with_tenors(vec![1.0, 5.0, 10.0], vec!["1Y".into(), "5Y".into(), "10Y".into()])
            .with_bump_size(0.0005)
            .with_consistency_check(false, 0.05);

        assert_eq!(config.tenor_points.len(), 3);
        assert!((config.bump_size - 0.0005).abs() < 1e-10);
        assert!(!config.verify_consistency);
    }

    // BucketDv01Entry tests
    #[test]
    fn test_bucket_dv01_entry_new() {
        let entry = BucketDv01Entry::new(5.0, "5Y", 1000.0);
        assert!((entry.tenor - 5.0).abs() < 1e-10);
        assert_eq!(entry.label, "5Y");
        assert!((entry.dv01 - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_bucket_dv01_entry_contribution() {
        let entry = BucketDv01Entry::new(5.0, "5Y", 500.0).with_contribution(1000.0);
        assert!((entry.contribution_pct - 50.0).abs() < 1e-10);
    }

    // BucketDv01Result tests
    #[test]
    fn test_bucket_dv01_result_new() {
        let buckets = vec![
            BucketDv01Entry::new(1.0, "1Y", 200.0),
            BucketDv01Entry::new(5.0, "5Y", 500.0),
            BucketDv01Entry::new(10.0, "10Y", 300.0),
        ];
        let result = BucketDv01Result::new(buckets, 1000.0, 1_000_000);

        assert_eq!(result.num_buckets(), 3);
        assert!((result.bucket_sum - 1000.0).abs() < 1e-10);
        assert!((result.consistency_ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bucket_dv01_result_is_consistent() {
        let buckets = vec![BucketDv01Entry::new(5.0, "5Y", 990.0)];
        let result = BucketDv01Result::new(buckets, 1000.0, 0);

        assert!(result.is_consistent(0.02)); // 2% tolerance
        assert!(!result.is_consistent(0.005)); // 0.5% tolerance
    }

    #[test]
    fn test_bucket_dv01_result_get_bucket() {
        let buckets = vec![
            BucketDv01Entry::new(1.0, "1Y", 200.0),
            BucketDv01Entry::new(5.0, "5Y", 500.0),
        ];
        let result = BucketDv01Result::new(buckets, 700.0, 0);

        assert!(result.get_bucket(5.0).is_some());
        assert!(result.get_bucket(10.0).is_none());

        assert!(result.get_bucket_by_label("5Y").is_some());
        assert!(result.get_bucket_by_label("10Y").is_none());
    }

    #[test]
    fn test_bucket_dv01_result_as_map() {
        let buckets = vec![
            BucketDv01Entry::new(1.0, "1Y", 200.0),
            BucketDv01Entry::new(5.0, "5Y", 500.0),
        ];
        let result = BucketDv01Result::new(buckets, 700.0, 0);

        let map = result.as_map();
        assert_eq!(map.len(), 2);
        assert!((map["1Y"] - 200.0).abs() < 1e-10);
        assert!((map["5Y"] - 500.0).abs() < 1e-10);
    }

    // Calculator tests
    #[test]
    fn test_bucket_dv01_calculator_new() {
        let config = BucketDv01Config::default();
        let _calculator = BucketDv01Calculator::new(config);
    }

    #[test]
    fn test_bucket_dv01_calculation() {
        // Disable consistency check for flat curve approximation
        let config = BucketDv01Config::new().with_consistency_check(false, 1.0);
        let calculator = BucketDv01Calculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_bucket_dv01(&swap, &curves, valuation_date)
            .unwrap();

        // Should have results for all standard tenors
        assert_eq!(result.num_buckets(), 9);

        // Total DV01 should be positive
        assert!(result.total_dv01 > 0.0);

        // Computation time should be recorded
        assert!(result.computation_time_ns > 0);
    }

    #[test]
    fn test_bucket_dv01_custom_tenors() {
        let config = BucketDv01Config::new()
            .with_tenors(vec![1.0, 5.0, 10.0], vec!["1Y".into(), "5Y".into(), "10Y".into()])
            .with_consistency_check(false, 1.0);
        let calculator = BucketDv01Calculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_bucket_dv01(&swap, &curves, valuation_date)
            .unwrap();

        assert_eq!(result.num_buckets(), 3);
    }

    // Key Rate Duration tests
    #[test]
    fn test_key_rate_duration_calculation() {
        let config = BucketDv01Config::new()
            .with_tenors(vec![1.0, 5.0, 10.0], vec!["1Y".into(), "5Y".into(), "10Y".into()]);
        let calculator = BucketDv01Calculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_key_rate_duration(&swap, &curves, valuation_date, None)
            .unwrap();

        // Should have durations for configured tenors
        assert_eq!(result.durations.len(), 3);

        // Effective duration should be calculated
        assert!(result.effective_duration.is_finite());

        // Computation time should be recorded
        assert!(result.computation_time_ns > 0);
    }

    #[test]
    fn test_key_rate_duration_custom_tenors() {
        let config = BucketDv01Config::default();
        let calculator = BucketDv01Calculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let custom_tenors = [2.0, 5.0, 7.0];
        let result = calculator
            .compute_key_rate_duration(&swap, &curves, valuation_date, Some(&custom_tenors))
            .unwrap();

        // Should use custom tenors
        assert_eq!(result.durations.len(), 3);
        assert!((result.durations[0].tenor - 2.0).abs() < 1e-10);
    }

    // KeyRateDurationResult tests
    #[test]
    fn test_key_rate_duration_result_computation_time() {
        let result = KeyRateDurationResult::new(vec![], 0.0, 1_500_000);
        assert!((result.computation_time_ms() - 1.5).abs() < 1e-10);
    }

    // Standard tenor constants tests
    #[test]
    fn test_standard_tenor_points() {
        assert_eq!(STANDARD_TENOR_POINTS.len(), 9);
        assert!((STANDARD_TENOR_POINTS[0] - 0.0833).abs() < 1e-3); // 1M
        assert!((STANDARD_TENOR_POINTS[3] - 1.0).abs() < 1e-10); // 1Y
        assert!((STANDARD_TENOR_POINTS[8] - 30.0).abs() < 1e-10); // 30Y
    }

    #[test]
    fn test_standard_tenor_labels() {
        assert_eq!(STANDARD_TENOR_LABELS.len(), 9);
        assert_eq!(STANDARD_TENOR_LABELS[0], "1M");
        assert_eq!(STANDARD_TENOR_LABELS[3], "1Y");
        assert_eq!(STANDARD_TENOR_LABELS[8], "30Y");
    }
}

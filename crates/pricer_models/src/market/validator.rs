//! Market validation utilities.
//!
//! This module provides [`MarketValidator`] for validating that an
//! [`IndexedMarket`] contains all required indices for a pricing operation.
//!
//! # Examples
//!
//! ```ignore
//! use pricer_models::market::{MarketValidator, IndexedMarket, TradeIndexRequirements};
//!
//! let validator = MarketValidator::new();
//! let result = validator.validate(&trade, &market);
//!
//! if let Err(report) = result {
//!     println!("Missing indices: {:?}", report.missing_indices());
//! }
//! ```

use std::collections::HashSet;

use infra_master::trade::IndexRequirement;
use num_traits::Float;

use super::{IndexedMarket, MarketDataError, TradeIndexRequirements};

// ============================================================================
// ValidationReport
// ============================================================================

/// Report of market validation results.
///
/// Contains details about validation success or failure, including
/// all missing indices if validation failed.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Whether validation passed (all indices available).
    is_valid: bool,
    /// Missing indices (empty if valid).
    missing: Vec<IndexRequirement>,
    /// Total required indices.
    total_required: usize,
    /// Available indices count.
    available_count: usize,
}

impl ValidationReport {
    /// Returns `true` if validation passed.
    #[must_use]
    pub fn is_valid(&self) -> bool { self.is_valid }

    /// Returns the list of missing indices.
    #[must_use]
    pub fn missing_indices(&self) -> &[IndexRequirement] { &self.missing }

    /// Returns the count of missing indices.
    #[must_use]
    pub fn missing_count(&self) -> usize { self.missing.len() }

    /// Returns the total number of required indices.
    #[must_use]
    pub fn total_required(&self) -> usize { self.total_required }

    /// Returns the count of available indices.
    #[must_use]
    pub fn available_count(&self) -> usize { self.available_count }

    /// Converts this report to a Result.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If validation passed
    /// * `Err(MarketDataError)` - If validation failed (first missing index)
    pub fn to_result(&self) -> Result<(), MarketDataError> {
        if self.is_valid {
            Ok(())
        } else if let Some(first_missing) = self.missing.first() {
            Err(MarketDataError::IndexNotFound {
                index: format!("{}", first_missing),
            })
        } else {
            Err(MarketDataError::MissingData {
                description: "Unknown validation failure".to_string(),
            })
        }
    }
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid {
            write!(f, "Valid: all {} indices available", self.total_required)
        } else {
            write!(
                f,
                "Invalid: {} of {} indices missing: {:?}",
                self.missing.len(),
                self.total_required,
                self.missing
            )
        }
    }
}

// ============================================================================
// MarketValidator
// ============================================================================

/// Validator for checking market data completeness.
///
/// `MarketValidator` verifies that an [`IndexedMarket`] contains all
/// indices required by a pricing target (implementing
/// [`TradeIndexRequirements`]).
///
/// # Design
///
/// The validator:
/// 1. Collects all required indices from the pricing target
/// 2. Deduplicates them (same index may be required multiple times)
/// 3. Checks availability of each index in the market
/// 4. Returns a comprehensive report
///
/// # Examples
///
/// ## Basic Validation
///
/// ```ignore
/// let validator = MarketValidator::new();
/// let report = validator.validate_report(&trade, &market);
///
/// if report.is_valid() {
///     // Proceed with pricing
/// } else {
///     eprintln!("Missing: {:?}", report.missing_indices());
/// }
/// ```
///
/// ## Quick Check
///
/// ```ignore
/// let validator = MarketValidator::new();
/// if validator.is_valid(&trade, &market) {
///     // Proceed with pricing
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct MarketValidator {
    /// Whether to deduplicate requirements before validation.
    deduplicate: bool,
}

impl MarketValidator {
    /// Creates a new validator.
    #[must_use]
    pub fn new() -> Self { Self { deduplicate: true } }

    /// Creates a validator without deduplication.
    ///
    /// This is useful when you want to count exact occurrences
    /// of each requirement.
    #[must_use]
    pub fn without_dedup() -> Self { Self { deduplicate: false } }

    /// Validates a pricing target against a market.
    ///
    /// # Arguments
    ///
    /// * `target` - The pricing target (Trade, Leg, etc.)
    /// * `market` - The market to validate against
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All required indices are available
    /// * `Err(MarketDataError)` - First missing index
    pub fn validate<R, T>(
        &self,
        target: &R,
        market: &IndexedMarket<T>,
    ) -> Result<(), MarketDataError>
    where
        R: TradeIndexRequirements,
        T: Float,
    {
        self.validate_report(target, market).to_result()
    }

    /// Returns whether validation passes.
    ///
    /// # Arguments
    ///
    /// * `target` - The pricing target
    /// * `market` - The market to validate against
    ///
    /// # Returns
    ///
    /// `true` if all required indices are available.
    pub fn is_valid<R, T>(&self, target: &R, market: &IndexedMarket<T>) -> bool
    where
        R: TradeIndexRequirements,
        T: Float,
    {
        self.validate_report(target, market).is_valid()
    }

    /// Generates a detailed validation report.
    ///
    /// # Arguments
    ///
    /// * `target` - The pricing target
    /// * `market` - The market to validate against
    ///
    /// # Returns
    ///
    /// A [`ValidationReport`] with details about the validation result.
    pub fn validate_report<R, T>(&self, target: &R, market: &IndexedMarket<T>) -> ValidationReport
    where
        R: TradeIndexRequirements,
        T: Float,
    {
        let requirements = target.required_indices();

        // Optionally deduplicate
        let requirements: Vec<IndexRequirement> = if self.deduplicate {
            let set: HashSet<_> = requirements.into_iter().collect();
            set.into_iter().collect()
        } else {
            requirements
        };

        let total_required = requirements.len();
        let mut missing = Vec::new();
        let mut available_count = 0;

        for req in requirements {
            let available = match &req {
                IndexRequirement::RateCurve(index) => market.has_curve(*index),
                IndexRequirement::SwaptionVol(index) => market.has_volcube(*index),
                IndexRequirement::FxCurve(pair) => market.has_fx_curve(*pair),
                IndexRequirement::FxVol(pair) => market.has_fx_vol_surface(*pair),
            };

            if available {
                available_count += 1;
            } else {
                missing.push(req);
            }
        }

        ValidationReport {
            is_valid: missing.is_empty(),
            missing,
            total_required,
            available_count,
        }
    }

    /// Validates multiple targets at once.
    ///
    /// # Arguments
    ///
    /// * `targets` - Iterator of pricing targets
    /// * `market` - The market to validate against
    ///
    /// # Returns
    ///
    /// A single `ValidationReport` covering all targets.
    pub fn validate_all<'a, R, T, I>(
        &self,
        targets: I,
        market: &IndexedMarket<T>,
    ) -> ValidationReport
    where
        R: TradeIndexRequirements + 'a,
        T: Float,
        I: Iterator<Item = &'a R>,
    {
        // Collect all requirements from all targets
        let all_requirements: Vec<IndexRequirement> =
            targets.flat_map(|t| t.required_indices()).collect();

        // Create a pseudo-target that has all requirements
        struct AllRequirements(Vec<IndexRequirement>);
        impl TradeIndexRequirements for AllRequirements {
            fn required_indices(&self) -> Vec<IndexRequirement> { self.0.clone() }
        }

        self.validate_report(&AllRequirements(all_requirements), market)
    }
}

#[cfg(test)]
mod tests {
    use infra_master::{trade::instrument_def::CurrencyPair, Currency, Date, RateIndex};

    use super::*;
    use crate::market::{curves::FlatCurve, IndexedMarketBuilder};

    // Mock implementations for testing
    struct MockTrade {
        reqs: Vec<IndexRequirement>,
    }

    impl TradeIndexRequirements for MockTrade {
        fn required_indices(&self) -> Vec<IndexRequirement> { self.reqs.clone() }
    }

    // ========================================
    // Basic Validation Tests
    // ========================================

    #[test]
    fn test_validate_success() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        assert!(validator.validate(&trade, &market).is_ok());
    }

    #[test]
    fn test_validate_failure() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        assert!(validator.validate(&trade, &market).is_err());
    }

    #[test]
    fn test_is_valid() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        assert!(validator.is_valid(&trade, &market));
    }

    // ========================================
    // ValidationReport Tests
    // ========================================

    #[test]
    fn test_report_valid() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![
                IndexRequirement::RateCurve(RateIndex::Sofr),
                IndexRequirement::RateCurve(RateIndex::Euribor3M),
            ],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);

        assert!(report.is_valid());
        assert!(report.missing_indices().is_empty());
        assert_eq!(report.missing_count(), 0);
        assert_eq!(report.available_count(), 2);
    }

    #[test]
    fn test_report_partial() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![
                IndexRequirement::RateCurve(RateIndex::Sofr),
                IndexRequirement::RateCurve(RateIndex::Euribor3M),
            ],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);

        assert!(!report.is_valid());
        assert_eq!(report.missing_count(), 1);
        assert_eq!(report.available_count(), 1);
        assert_eq!(report.total_required(), 2);
    }

    #[test]
    fn test_report_display() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);
        let display = format!("{}", report);
        assert!(display.contains("Valid"));
    }

    #[test]
    fn test_report_to_result() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);
        let result = report.to_result();

        assert!(result.is_err());
    }

    // ========================================
    // Deduplication Tests
    // ========================================

    #[test]
    fn test_deduplication() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        // Same requirement twice
        let trade = MockTrade {
            reqs: vec![
                IndexRequirement::RateCurve(RateIndex::Sofr),
                IndexRequirement::RateCurve(RateIndex::Sofr),
            ],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);

        // With dedup, should count as 1 requirement
        assert!(report.is_valid());
        assert_eq!(report.total_required(), 1);
    }

    #[test]
    fn test_without_deduplication() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        // Same requirement twice
        let trade = MockTrade {
            reqs: vec![
                IndexRequirement::RateCurve(RateIndex::Sofr),
                IndexRequirement::RateCurve(RateIndex::Sofr),
            ],
        };

        let validator = MarketValidator::without_dedup();
        let report = validator.validate_report(&trade, &market);

        // Without dedup, should count as 2 requirements
        assert!(report.is_valid());
        assert_eq!(report.total_required(), 2);
    }

    // ========================================
    // Multi-Target Tests
    // ========================================

    #[test]
    fn test_validate_all() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            .build()
            .unwrap();

        let trades = vec![
            MockTrade {
                reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
            },
            MockTrade {
                reqs: vec![IndexRequirement::RateCurve(RateIndex::Euribor3M)],
            },
        ];

        let validator = MarketValidator::new();
        let report = validator.validate_all(trades.iter(), &market);

        assert!(report.is_valid());
    }

    #[test]
    fn test_validate_all_partial() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let trades = vec![
            MockTrade {
                reqs: vec![IndexRequirement::RateCurve(RateIndex::Sofr)],
            },
            MockTrade {
                reqs: vec![IndexRequirement::RateCurve(RateIndex::Sonia)],
            },
        ];

        let validator = MarketValidator::new();
        let report = validator.validate_all(trades.iter(), &market);

        assert!(!report.is_valid());
        assert_eq!(report.missing_count(), 1);
    }

    // ========================================
    // FX and Vol Tests
    // ========================================

    #[test]
    fn test_fx_curve_validation() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::FxCurve(pair)],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);

        assert!(!report.is_valid());
        assert_eq!(report.missing_count(), 1);
    }

    #[test]
    fn test_volcube_validation() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let trade = MockTrade {
            reqs: vec![IndexRequirement::SwaptionVol(RateIndex::Sofr)],
        };

        let validator = MarketValidator::new();
        let report = validator.validate_report(&trade, &market);

        assert!(!report.is_valid());
    }
}

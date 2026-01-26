//! Trade index requirements trait.
//!
//! This module provides [`TradeIndexRequirements`], a trait for determining
//! what market indices are needed to price a Trade or Cashflow.
//!
//! # Examples
//!
//! ```ignore
//! use pricer_models::market::TradeIndexRequirements;
//! use infra_master::trade::IndexRequirement;
//!
//! impl TradeIndexRequirements for MyTrade {
//!     fn required_indices(&self) -> Vec<IndexRequirement> {
//!         vec![
//!             IndexRequirement::RateCurve(RateIndex::Sofr),
//!         ]
//!     }
//! }
//! ```

use infra_master::trade::IndexRequirement;
use num_traits::Float;

use super::{IndexedMarket, MarketDataError};

/// Trait for determining required market indices.
///
/// Implemented by Trade, Leg, Cashflow, or any pricing target to specify
/// what market data is needed for pricing.
///
/// # Design
///
/// This trait follows the "Requirements Pattern" where:
/// 1. The pricing target declares what it needs (`required_indices`)
/// 2. The market validator verifies availability
/// 3. The pricer retrieves data from IndexedMarket using the indices
///
/// # Implementation Guidelines
///
/// - Return all indices needed for complete pricing
/// - Include both discount curves and projection curves
/// - Include volatility data for options
/// - Deduplicate indices where possible (use `HashSet` internally if needed)
///
/// # Examples
///
/// ## Fixed Leg (single discount curve)
///
/// ```ignore
/// impl TradeIndexRequirements for FixedLeg {
///     fn required_indices(&self) -> Vec<IndexRequirement> {
///         vec![
///             IndexRequirement::RateCurve(self.discount_index),
///         ]
///     }
/// }
/// ```
///
/// ## Floating Leg (discount + projection curve)
///
/// ```ignore
/// impl TradeIndexRequirements for FloatingLeg {
///     fn required_indices(&self) -> Vec<IndexRequirement> {
///         vec![
///             IndexRequirement::RateCurve(self.discount_index),
///             IndexRequirement::RateCurve(self.rate_index),
///         ]
///     }
/// }
/// ```
///
/// ## Swaption (curves + vol cube)
///
/// ```ignore
/// impl TradeIndexRequirements for Swaption {
///     fn required_indices(&self) -> Vec<IndexRequirement> {
///         vec![
///             IndexRequirement::RateCurve(self.rate_index),
///             IndexRequirement::SwaptionVol(self.rate_index),
///         ]
///     }
/// }
/// ```
pub trait TradeIndexRequirements {
    /// Returns the list of market indices required for pricing.
    ///
    /// # Returns
    ///
    /// A vector of `IndexRequirement` representing all market data needed.
    /// The vector may contain duplicates; callers should deduplicate if needed.
    fn required_indices(&self) -> Vec<IndexRequirement>;

    /// Validates that all required indices are available in the market.
    ///
    /// # Default Implementation
    ///
    /// Checks each required index against the market and returns
    /// the first missing index as an error.
    ///
    /// # Arguments
    ///
    /// * `market` - The IndexedMarket to validate against
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All required indices are available
    /// * `Err(MarketDataError::IndexNotFound)` - First missing index
    fn validate_market<T: Float>(&self, market: &IndexedMarket<T>) -> Result<(), MarketDataError> {
        for req in self.required_indices() {
            let available = match &req {
                IndexRequirement::RateCurve(index) => market.has_curve(*index),
                IndexRequirement::SwaptionVol(index) => market.has_volcube(*index),
                IndexRequirement::FxCurve(pair) => market.has_fx_curve(*pair),
                IndexRequirement::FxVol(pair) => market.has_fx_vol_surface(*pair),
            };

            if !available {
                return Err(MarketDataError::IndexNotFound {
                    index: format!("{}", req),
                });
            }
        }
        Ok(())
    }

    /// Returns missing indices that are not available in the market.
    ///
    /// # Arguments
    ///
    /// * `market` - The IndexedMarket to check against
    ///
    /// # Returns
    ///
    /// A vector of `IndexRequirement` that are required but not available.
    fn missing_indices<T: Float>(&self, market: &IndexedMarket<T>) -> Vec<IndexRequirement> {
        self.required_indices()
            .into_iter()
            .filter(|req| {
                let available = match req {
                    IndexRequirement::RateCurve(index) => market.has_curve(*index),
                    IndexRequirement::SwaptionVol(index) => market.has_volcube(*index),
                    IndexRequirement::FxCurve(pair) => market.has_fx_curve(*pair),
                    IndexRequirement::FxVol(pair) => market.has_fx_vol_surface(*pair),
                };
                !available
            })
            .collect()
    }
}

/// Blanket implementation for Vec<T> where T: TradeIndexRequirements.
///
/// Collects all requirements from all elements.
impl<T: TradeIndexRequirements> TradeIndexRequirements for Vec<T> {
    fn required_indices(&self) -> Vec<IndexRequirement> {
        self.iter().flat_map(|t| t.required_indices()).collect()
    }
}

/// Blanket implementation for Option<T> where T: TradeIndexRequirements.
///
/// Returns empty vec for None, delegating to inner for Some.
impl<T: TradeIndexRequirements> TradeIndexRequirements for Option<T> {
    fn required_indices(&self) -> Vec<IndexRequirement> {
        self.as_ref()
            .map_or_else(Vec::new, |t| t.required_indices())
    }
}

#[cfg(test)]
mod tests {
    use infra_master::{trade::instrument_def::CurrencyPair, Currency, Date, RateIndex};

    use super::*;
    use crate::market::{curves::FlatCurve, IndexedMarketBuilder};

    // Test implementation for a simple fixed leg
    struct MockFixedLeg {
        discount_index: RateIndex,
    }

    impl TradeIndexRequirements for MockFixedLeg {
        fn required_indices(&self) -> Vec<IndexRequirement> {
            vec![IndexRequirement::RateCurve(self.discount_index)]
        }
    }

    // Test implementation for a floating leg
    struct MockFloatingLeg {
        discount_index: RateIndex,
        rate_index: RateIndex,
    }

    impl TradeIndexRequirements for MockFloatingLeg {
        fn required_indices(&self) -> Vec<IndexRequirement> {
            vec![
                IndexRequirement::RateCurve(self.discount_index),
                IndexRequirement::RateCurve(self.rate_index),
            ]
        }
    }

    // Test implementation for an FX forward
    struct MockFxForward {
        pair: CurrencyPair,
    }

    impl TradeIndexRequirements for MockFxForward {
        fn required_indices(&self) -> Vec<IndexRequirement> {
            vec![IndexRequirement::FxCurve(self.pair)]
        }
    }

    // ========================================
    // Basic Trait Tests
    // ========================================

    #[test]
    fn test_fixed_leg_requirements() {
        let leg = MockFixedLeg {
            discount_index: RateIndex::Sofr,
        };

        let reqs = leg.required_indices();
        assert_eq!(reqs.len(), 1);
        assert!(reqs[0].is_rate_curve());
    }

    #[test]
    fn test_floating_leg_requirements() {
        let leg = MockFloatingLeg {
            discount_index: RateIndex::Sofr,
            rate_index: RateIndex::Euribor3M,
        };

        let reqs = leg.required_indices();
        assert_eq!(reqs.len(), 2);
        assert!(reqs.iter().all(|r| r.is_rate_curve()));
    }

    #[test]
    fn test_fx_forward_requirements() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fwd = MockFxForward { pair };

        let reqs = fwd.required_indices();
        assert_eq!(reqs.len(), 1);
        assert!(reqs[0].is_fx_curve());
    }

    // ========================================
    // Validate Market Tests
    // ========================================

    #[test]
    fn test_validate_market_success() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let leg = MockFixedLeg {
            discount_index: RateIndex::Sofr,
        };

        assert!(leg.validate_market(&market).is_ok());
    }

    #[test]
    fn test_validate_market_missing_curve() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let leg = MockFixedLeg {
            discount_index: RateIndex::Sofr,
        };

        let result = leg.validate_market(&market);
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            MarketDataError::IndexNotFound { index } => {
                assert!(index.contains("RateCurve"));
            }
            _ => panic!("Expected IndexNotFound"),
        }
    }

    #[test]
    fn test_validate_market_partial() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        // This leg needs both SOFR and EURIBOR
        let leg = MockFloatingLeg {
            discount_index: RateIndex::Sofr,
            rate_index: RateIndex::Euribor3M,
        };

        // Should fail because EURIBOR is missing
        let result = leg.validate_market(&market);
        assert!(result.is_err());
    }

    // ========================================
    // Missing Indices Tests
    // ========================================

    #[test]
    fn test_missing_indices_none() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let leg = MockFixedLeg {
            discount_index: RateIndex::Sofr,
        };

        let missing = leg.missing_indices(&market);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_missing_indices_some() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let leg = MockFloatingLeg {
            discount_index: RateIndex::Sofr,
            rate_index: RateIndex::Euribor3M,
        };

        let missing = leg.missing_indices(&market);
        assert_eq!(missing.len(), 1);
        assert_eq!(
            missing[0],
            IndexRequirement::RateCurve(RateIndex::Euribor3M)
        );
    }

    // ========================================
    // Blanket Implementation Tests
    // ========================================

    #[test]
    fn test_vec_requirements() {
        let legs = vec![
            MockFixedLeg {
                discount_index: RateIndex::Sofr,
            },
            MockFixedLeg {
                discount_index: RateIndex::Euribor3M,
            },
        ];

        let reqs = legs.required_indices();
        assert_eq!(reqs.len(), 2);
    }

    #[test]
    fn test_option_some_requirements() {
        let leg = Some(MockFixedLeg {
            discount_index: RateIndex::Sofr,
        });

        let reqs = leg.required_indices();
        assert_eq!(reqs.len(), 1);
    }

    #[test]
    fn test_option_none_requirements() {
        let leg: Option<MockFixedLeg> = None;
        let reqs = leg.required_indices();
        assert!(reqs.is_empty());
    }
}

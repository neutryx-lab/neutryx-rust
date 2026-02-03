//! Rate validation traits and implementations.
//!
//! This module provides the [`RateValidator`] trait and
//! [`StandardRateValidator`] implementation for validating market rate values.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{
//!     RateValidator, StandardRateValidator, MarketRate,
//!     RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::Tenor;
//!
//! let validator = StandardRateValidator::default();
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     rate_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! assert!(validator.validate(&rate).is_ok());
//! ```

use super::{error::MarketRateError, rate::MarketRate, rate_type::RateType};

/// Trait for validating market rates.
///
/// Implementations of this trait define custom validation logic
/// for market rate values beyond basic NaN/Infinite checks.
///
/// # Examples
///
/// ```
/// use infra_master::market::{RateValidator, StandardRateValidator, MarketRateError};
///
/// struct StrictValidator;
///
/// impl RateValidator for StrictValidator {
///     fn validate(&self, rate: &infra_master::market::MarketRate) -> Result<(), MarketRateError> {
///         // Custom validation logic
///         if rate.value < 0.0 {
///             return Err(MarketRateError::ValidationFailed(
///                 "Negative rates not allowed".to_string()
///             ));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait RateValidator {
    /// Validates a market rate.
    ///
    /// # Arguments
    ///
    /// * `rate` - The market rate to validate
    ///
    /// # Errors
    ///
    /// Returns [`MarketRateError`] if validation fails.
    fn validate(&self, rate: &MarketRate) -> Result<(), MarketRateError>;
}

/// Standard rate validator with reasonable default bounds.
///
/// Validates rates based on their type:
/// - Interest rates: -10% to 100% (-0.10 to 1.00)
/// - FX rates: 0.0001 to 100,000
/// - Volatility: 0% to 500% (0.0 to 5.0)
///
/// # Examples
///
/// ```
/// use infra_master::market::{
///     RateValidator, StandardRateValidator, MarketRate,
///     RateId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_master::time::Tenor;
///
/// let validator = StandardRateValidator::default();
///
/// // Valid interest rate
/// let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Swap);
/// let rate = MarketRate::new(
///     rate_id,
///     QuoteType::Mid,
///     0.05,  // 5% - within bounds
///     1700000000000,
///     DataSource::Bloomberg,
/// ).unwrap();
///
/// assert!(validator.validate(&rate).is_ok());
/// ```
#[derive(Debug, Clone, Default)]
pub struct StandardRateValidator;

impl StandardRateValidator {
    /// Minimum allowed interest rate (-10%).
    pub const MIN_INTEREST_RATE: f64 = -0.10;

    /// Maximum allowed interest rate (100%).
    pub const MAX_INTEREST_RATE: f64 = 1.00;

    /// Minimum allowed FX rate.
    pub const MIN_FX_RATE: f64 = 0.0001;

    /// Maximum allowed FX rate.
    pub const MAX_FX_RATE: f64 = 100_000.0;

    /// Minimum allowed volatility (0%).
    pub const MIN_VOLATILITY: f64 = 0.0;

    /// Maximum allowed volatility (500%).
    pub const MAX_VOLATILITY: f64 = 5.0;

    /// Creates a new `StandardRateValidator`.
    #[must_use]
    pub fn new() -> Self { Self }

    /// Validates the basic properties of a rate value.
    ///
    /// Checks for NaN and Infinite values.
    fn validate_basic(&self, value: f64) -> Result<(), MarketRateError> {
        if value.is_nan() {
            return Err(MarketRateError::nan());
        }
        if value.is_infinite() {
            return Err(MarketRateError::infinite(value));
        }
        Ok(())
    }

    /// Validates an interest rate value.
    fn validate_interest_rate(&self, value: f64) -> Result<(), MarketRateError> {
        self.validate_basic(value)?;

        if value < Self::MIN_INTEREST_RATE || value > Self::MAX_INTEREST_RATE {
            return Err(MarketRateError::out_of_bounds(
                value,
                Self::MIN_INTEREST_RATE,
                Self::MAX_INTEREST_RATE,
            ));
        }
        Ok(())
    }

    /// Validates an FX rate value.
    fn validate_fx_rate(&self, value: f64) -> Result<(), MarketRateError> {
        self.validate_basic(value)?;

        if value < Self::MIN_FX_RATE || value > Self::MAX_FX_RATE {
            return Err(MarketRateError::out_of_bounds(
                value,
                Self::MIN_FX_RATE,
                Self::MAX_FX_RATE,
            ));
        }
        Ok(())
    }

    /// Validates a volatility value.
    fn validate_volatility(&self, value: f64) -> Result<(), MarketRateError> {
        self.validate_basic(value)?;

        if value < Self::MIN_VOLATILITY || value > Self::MAX_VOLATILITY {
            return Err(MarketRateError::out_of_bounds(
                value,
                Self::MIN_VOLATILITY,
                Self::MAX_VOLATILITY,
            ));
        }
        Ok(())
    }
}

impl RateValidator for StandardRateValidator {
    fn validate(&self, rate: &MarketRate) -> Result<(), MarketRateError> {
        let value = rate.value;
        let rate_type = rate.id.rate_type;

        match rate_type {
            // Interest rate types
            RateType::Deposit
            | RateType::Fra
            | RateType::Futures
            | RateType::Swap
            | RateType::Ois
            | RateType::BasisSwap
            | RateType::Event => self.validate_interest_rate(value),

            // FX types
            RateType::FxSpot | RateType::FxForward => self.validate_fx_rate(value),

            // Volatility
            RateType::Vol => self.validate_volatility(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        market::{Currency, DataSource, QuoteType, RateId},
        time::Tenor,
    };

    fn create_rate(rate_type: RateType, value: f64) -> MarketRate {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, rate_type);
        MarketRate::new(
            rate_id,
            QuoteType::Mid,
            value,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }

    // Helper to create rate without validation (for testing invalid values)
    fn create_rate_unchecked(rate_type: RateType, value: f64) -> MarketRate {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, rate_type);
        MarketRate {
            id: rate_id,
            quote_type: QuoteType::Mid,
            value,
            timestamp: 1700000000000,
            source: DataSource::Bloomberg,
        }
    }

    #[test]
    fn test_standard_validator_default() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, 0.05);
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_standard_validator_new() {
        let validator = StandardRateValidator::new();
        let rate = create_rate(RateType::Swap, 0.05);
        assert!(validator.validate(&rate).is_ok());
    }

    // Interest rate validation tests

    #[test]
    fn test_validate_interest_rate_valid() {
        let validator = StandardRateValidator::default();

        // Valid interest rates
        for rate_type in [
            RateType::Deposit,
            RateType::Fra,
            RateType::Futures,
            RateType::Swap,
            RateType::Ois,
            RateType::BasisSwap,
        ] {
            let rate = create_rate(rate_type, 0.05);
            assert!(
                validator.validate(&rate).is_ok(),
                "Failed for {:?}",
                rate_type
            );
        }
    }

    #[test]
    fn test_validate_interest_rate_zero() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, 0.0);
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_interest_rate_negative_valid() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, -0.005); // -0.5%
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_interest_rate_at_lower_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, -0.10); // -10%
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_interest_rate_at_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, 1.0); // 100%
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_interest_rate_below_lower_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, -0.15); // -15%

        let result = validator.validate(&rate);
        assert!(result.is_err());
        match result {
            Err(MarketRateError::InvalidRate { value, .. }) => {
                assert!((value - (-0.15)).abs() < f64::EPSILON);
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_validate_interest_rate_above_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Swap, 1.5); // 150%

        let result = validator.validate(&rate);
        assert!(result.is_err());
        match result {
            Err(MarketRateError::InvalidRate { value, .. }) => {
                assert!((value - 1.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    // FX rate validation tests

    #[test]
    fn test_validate_fx_rate_valid() {
        let validator = StandardRateValidator::default();

        for rate_type in [RateType::FxSpot, RateType::FxForward] {
            let rate = create_rate(rate_type, 1.2345);
            assert!(
                validator.validate(&rate).is_ok(),
                "Failed for {:?}",
                rate_type
            );
        }
    }

    #[test]
    fn test_validate_fx_rate_at_lower_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::FxSpot, 0.0001);
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_fx_rate_at_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::FxSpot, 100_000.0);
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_fx_rate_below_lower_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::FxSpot, 0.00001);

        let result = validator.validate(&rate);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_fx_rate_above_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::FxSpot, 200_000.0);

        let result = validator.validate(&rate);
        assert!(result.is_err());
    }

    // Volatility validation tests

    #[test]
    fn test_validate_volatility_valid() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Vol, 0.20); // 20%
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_volatility_at_lower_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Vol, 0.0);
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_volatility_at_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Vol, 5.0); // 500%
        assert!(validator.validate(&rate).is_ok());
    }

    #[test]
    fn test_validate_volatility_negative() {
        let validator = StandardRateValidator::default();
        let rate = create_rate_unchecked(RateType::Vol, -0.01);

        let result = validator.validate(&rate);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_volatility_above_upper_bound() {
        let validator = StandardRateValidator::default();
        let rate = create_rate(RateType::Vol, 6.0); // 600%

        let result = validator.validate(&rate);
        assert!(result.is_err());
    }

    // NaN and Infinite tests

    #[test]
    fn test_validate_nan() {
        let validator = StandardRateValidator::default();
        let rate = create_rate_unchecked(RateType::Swap, f64::NAN);

        let result = validator.validate(&rate);
        assert!(result.is_err());
        match result {
            Err(MarketRateError::InvalidRate { reason, .. }) => {
                assert!(reason.contains("NaN"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_validate_positive_infinity() {
        let validator = StandardRateValidator::default();
        let rate = create_rate_unchecked(RateType::Swap, f64::INFINITY);

        let result = validator.validate(&rate);
        assert!(result.is_err());
        match result {
            Err(MarketRateError::InvalidRate { value, reason }) => {
                assert!(value.is_infinite());
                assert!(reason.contains("infinite"));
            }
            _ => panic!("Expected InvalidRate error"),
        }
    }

    #[test]
    fn test_validate_negative_infinity() {
        let validator = StandardRateValidator::default();
        let rate = create_rate_unchecked(RateType::Swap, f64::NEG_INFINITY);

        let result = validator.validate(&rate);
        assert!(result.is_err());
    }

    #[test]
    fn test_validator_clone() {
        let validator = StandardRateValidator::default();
        let cloned = validator.clone();

        let rate = create_rate(RateType::Swap, 0.05);
        assert_eq!(
            validator.validate(&rate).is_ok(),
            cloned.validate(&rate).is_ok()
        );
    }

    #[test]
    fn test_validator_debug() {
        let validator = StandardRateValidator::default();
        let debug_str = format!("{:?}", validator);
        assert!(debug_str.contains("StandardRateValidator"));
    }
}

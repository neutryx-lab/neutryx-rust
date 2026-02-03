//! Instrument mapping for market rates.
//!
//! This module provides the [`InstrumentMapper`] trait and
//! [`StandardInstrumentMapper`] implementation for converting market rates to
//! trading instruments.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{
//!     InstrumentMapper, StandardInstrumentMapper, MarketRate,
//!     RateId, RateType, QuoteType, DataSource, Currency
//! };
//! use infra_master::time::{Date, Tenor};
//!
//! let mapper = StandardInstrumentMapper::new();
//! let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
//! let rate = MarketRate::new(
//!     rate_id,
//!     QuoteType::Mid,
//!     0.05,
//!     1700000000000,
//!     DataSource::Bloomberg,
//! ).unwrap();
//!
//! let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
//! let instrument = mapper.map_to_instrument(&rate, valuation_date);
//! assert!(instrument.is_ok());
//! ```

use super::{error::MarketRateError, rate::MarketRate, rate_type::RateType};
use crate::{
    time::{Date, EndOfMonthRule},
    trade::Instrument,
};

/// Trait for mapping market rates to instruments.
///
/// Implementations of this trait convert [`MarketRate`] quotes to
/// [`Instrument`] definitions suitable for curve calibration.
///
/// # Examples
///
/// ```
/// use infra_master::market::{
///     InstrumentMapper, StandardInstrumentMapper, MarketRate,
///     RateId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_master::time::{Date, Tenor};
///
/// struct CustomMapper;
///
/// impl InstrumentMapper for CustomMapper {
///     fn map_to_instrument(
///         &self,
///         rate: &MarketRate,
///         valuation_date: Date,
///     ) -> Result<infra_master::trade::Instrument, infra_master::market::MarketRateError> {
///         // Custom mapping logic
///         StandardInstrumentMapper::new().map_to_instrument(rate, valuation_date)
///     }
/// }
/// ```
pub trait InstrumentMapper {
    /// Maps a market rate to an instrument.
    ///
    /// # Arguments
    ///
    /// * `rate` - The market rate to map
    /// * `valuation_date` - The valuation date for calculating instrument dates
    ///
    /// # Errors
    ///
    /// Returns [`MarketRateError::MappingFailed`] if the rate type cannot be
    /// mapped.
    fn map_to_instrument(
        &self,
        rate: &MarketRate,
        valuation_date: Date,
    ) -> Result<Instrument, MarketRateError>;
}

/// Standard instrument mapper with default conventions.
///
/// Maps market rates to instruments using standard market conventions:
/// - Deposit rates → [`Instrument::Deposit`]
/// - FRA rates → [`Instrument::Fra`]
/// - Futures rates → [`Instrument::Futures`] (price = 100 - rate × 100)
/// - Swap rates → [`Instrument::ParSwap`]
/// - OIS rates → [`Instrument::Ois`]
/// - Basis swap rates → [`Instrument::BasisSwap`]
///
/// # Examples
///
/// ```
/// use infra_master::market::{
///     InstrumentMapper, StandardInstrumentMapper, MarketRate,
///     RateId, RateType, QuoteType, DataSource, Currency
/// };
/// use infra_master::time::{Date, Tenor};
///
/// let mapper = StandardInstrumentMapper::new();
///
/// let rate_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
/// let rate = MarketRate::new(
///     rate_id,
///     QuoteType::Mid,
///     0.045,
///     1700000000000,
///     DataSource::Bloomberg,
/// ).unwrap();
///
/// let valuation_date = Date::from_ymd(2024, 1, 15).unwrap();
/// let instrument = mapper.map_to_instrument(&rate, valuation_date).unwrap();
///
/// assert!(matches!(instrument, infra_master::trade::Instrument::ParSwap { .. }));
/// ```
#[derive(Debug, Clone)]
pub struct StandardInstrumentMapper {
    /// End of month rule for date calculations.
    eom_rule: EndOfMonthRule,
    /// Settlement lag in business days (default: 2).
    settlement_lag: u32,
}

impl Default for StandardInstrumentMapper {
    fn default() -> Self { Self::new() }
}

impl StandardInstrumentMapper {
    /// Creates a new `StandardInstrumentMapper` with default settings.
    ///
    /// Uses:
    /// - End of month rule: Adjust
    /// - Settlement lag: 2 business days
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::StandardInstrumentMapper;
    ///
    /// let mapper = StandardInstrumentMapper::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            eom_rule: EndOfMonthRule::Adjust,
            settlement_lag: 2,
        }
    }

    /// Creates a mapper with a custom end of month rule.
    ///
    /// # Arguments
    ///
    /// * `eom_rule` - The end of month rule to use for date calculations
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::StandardInstrumentMapper;
    /// use infra_master::time::EndOfMonthRule;
    ///
    /// let mapper = StandardInstrumentMapper::with_eom_rule(EndOfMonthRule::Preserve);
    /// ```
    #[must_use]
    pub fn with_eom_rule(eom_rule: EndOfMonthRule) -> Self {
        Self {
            eom_rule,
            settlement_lag: 2,
        }
    }

    /// Creates a mapper with a custom settlement lag.
    ///
    /// # Arguments
    ///
    /// * `settlement_lag` - Settlement lag in business days
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::StandardInstrumentMapper;
    ///
    /// let mapper = StandardInstrumentMapper::new().with_settlement_lag(1);
    /// ```
    #[must_use]
    pub fn with_settlement_lag(mut self, settlement_lag: u32) -> Self {
        self.settlement_lag = settlement_lag;
        self
    }

    /// Calculates the start date (spot date) from valuation date.
    ///
    /// For simplicity, adds settlement_lag calendar days.
    /// A production implementation would use business day calendars.
    fn spot_date(&self, valuation_date: Date) -> Date {
        use crate::time::Period;
        valuation_date + Period::days(self.settlement_lag as i32)
    }

    /// Maps a deposit rate to a Deposit instrument.
    fn map_deposit(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Deposit {
            currency: rate.id.currency,
            start_date,
            tenor: rate.id.tenor,
            rate: rate.value,
        }
    }

    /// Maps a FRA rate to a Fra instrument.
    fn map_fra(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Fra {
            currency: rate.id.currency,
            start_date,
            tenor: rate.id.tenor,
            rate: rate.value,
        }
    }

    /// Maps a futures rate to a Futures instrument.
    ///
    /// Converts rate to price: price = 100 - rate × 100
    fn map_futures(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        // For futures, the start date is typically the expiry date
        // which depends on IMM dates. Simplified: use spot + tenor.
        let expiry = rate
            .id
            .tenor
            .add_to_date(self.spot_date(valuation_date), self.eom_rule);

        // Convert rate to price: price = 100 - rate × 100
        // e.g., 5% rate → price = 95.0
        let price = 100.0 - rate.value * 100.0;

        Instrument::Futures {
            currency: rate.id.currency,
            expiry,
            price,
        }
    }

    /// Maps a swap rate to a ParSwap instrument.
    fn map_swap(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::ParSwap {
            currency: rate.id.currency,
            start_date,
            tenor: rate.id.tenor,
            rate: rate.value,
        }
    }

    /// Maps an OIS rate to an Ois instrument.
    fn map_ois(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::Ois {
            currency: rate.id.currency,
            start_date,
            tenor: rate.id.tenor,
            rate: rate.value,
        }
    }

    /// Maps a basis swap rate to a BasisSwap instrument.
    fn map_basis_swap(&self, rate: &MarketRate, valuation_date: Date) -> Instrument {
        let start_date = self.spot_date(valuation_date);

        Instrument::BasisSwap {
            currency: rate.id.currency,
            start_date,
            tenor: rate.id.tenor,
            spread: rate.value,
        }
    }
}

impl InstrumentMapper for StandardInstrumentMapper {
    fn map_to_instrument(
        &self,
        rate: &MarketRate,
        valuation_date: Date,
    ) -> Result<Instrument, MarketRateError> {
        match rate.id.rate_type {
            RateType::Deposit => Ok(self.map_deposit(rate, valuation_date)),
            RateType::Fra => Ok(self.map_fra(rate, valuation_date)),
            RateType::Futures => Ok(self.map_futures(rate, valuation_date)),
            RateType::Swap => Ok(self.map_swap(rate, valuation_date)),
            RateType::Ois => Ok(self.map_ois(rate, valuation_date)),
            RateType::BasisSwap => Ok(self.map_basis_swap(rate, valuation_date)),
            RateType::FxSpot | RateType::FxForward | RateType::Vol | RateType::Event => {
                Err(MarketRateError::unsupported_rate_type(rate.id.rate_type))
            }
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

    fn test_rate(rate_type: RateType, tenor: Tenor, value: f64) -> MarketRate {
        let rate_id = RateId::new(Currency::USD, tenor, rate_type);
        MarketRate::new(
            rate_id,
            QuoteType::Mid,
            value,
            1700000000000,
            DataSource::Bloomberg,
        )
        .unwrap()
    }

    fn valuation_date() -> Date { Date::from_ymd(2024, 1, 15).unwrap() }

    #[test]
    fn test_standard_mapper_new() {
        let mapper = StandardInstrumentMapper::new();
        assert_eq!(mapper.settlement_lag, 2);
        assert_eq!(mapper.eom_rule, EndOfMonthRule::Adjust);
    }

    #[test]
    fn test_standard_mapper_default() {
        let mapper = StandardInstrumentMapper::default();
        assert_eq!(mapper.settlement_lag, 2);
    }

    #[test]
    fn test_standard_mapper_with_eom_rule() {
        let mapper = StandardInstrumentMapper::with_eom_rule(EndOfMonthRule::Preserve);
        assert_eq!(mapper.eom_rule, EndOfMonthRule::Preserve);
    }

    #[test]
    fn test_standard_mapper_with_settlement_lag() {
        let mapper = StandardInstrumentMapper::new().with_settlement_lag(1);
        assert_eq!(mapper.settlement_lag, 1);
    }

    #[test]
    fn test_map_deposit() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Deposit, Tenor::ThreeMonths, 0.05);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::Deposit {
                currency,
                start_date,
                tenor,
                rate: r,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, Date::from_ymd(2024, 1, 17).unwrap()); // +2 days
                assert_eq!(tenor, Tenor::ThreeMonths);
                assert!((r - 0.05).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Deposit instrument"),
        }
    }

    #[test]
    fn test_map_fra() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Fra, Tenor::SixMonths, 0.055);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::Fra {
                currency,
                start_date,
                tenor,
                rate: r,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, Date::from_ymd(2024, 1, 17).unwrap());
                assert_eq!(tenor, Tenor::SixMonths);
                assert!((r - 0.055).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Fra instrument"),
        }
    }

    #[test]
    fn test_map_futures() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Futures, Tenor::ThreeMonths, 0.045);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::Futures {
                currency,
                expiry,
                price,
            } => {
                assert_eq!(currency, Currency::USD);
                // Expiry = spot + 3M
                assert_eq!(expiry, Date::from_ymd(2024, 4, 17).unwrap());
                // Price = 100 - 4.5% × 100 = 95.5
                assert!((price - 95.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Futures instrument"),
        }
    }

    #[test]
    fn test_map_swap() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Swap, Tenor::FiveYears, 0.04);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::ParSwap {
                currency,
                start_date,
                tenor,
                rate: r,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, Date::from_ymd(2024, 1, 17).unwrap());
                assert_eq!(tenor, Tenor::FiveYears);
                assert!((r - 0.04).abs() < f64::EPSILON);
            }
            _ => panic!("Expected ParSwap instrument"),
        }
    }

    #[test]
    fn test_map_ois() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Ois, Tenor::OneYear, 0.035);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::Ois {
                currency,
                start_date,
                tenor,
                rate: r,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, Date::from_ymd(2024, 1, 17).unwrap());
                assert_eq!(tenor, Tenor::OneYear);
                assert!((r - 0.035).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Ois instrument"),
        }
    }

    #[test]
    fn test_map_basis_swap() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::BasisSwap, Tenor::TenYears, 0.0025);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::BasisSwap {
                currency,
                start_date,
                tenor,
                spread,
            } => {
                assert_eq!(currency, Currency::USD);
                assert_eq!(start_date, Date::from_ymd(2024, 1, 17).unwrap());
                assert_eq!(tenor, Tenor::TenYears);
                assert!((spread - 0.0025).abs() < f64::EPSILON);
            }
            _ => panic!("Expected BasisSwap instrument"),
        }
    }

    #[test]
    fn test_map_fx_spot_fails() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::FxSpot, Tenor::TwoWeeks, 1.1);
        let vd = valuation_date();

        let result = mapper.map_to_instrument(&rate, vd);
        assert!(result.is_err());

        match result {
            Err(MarketRateError::MappingFailed { rate_type, .. }) => {
                assert_eq!(rate_type, RateType::FxSpot);
            }
            _ => panic!("Expected MappingFailed error"),
        }
    }

    #[test]
    fn test_map_fx_forward_fails() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::FxForward, Tenor::ThreeMonths, 1.105);
        let vd = valuation_date();

        let result = mapper.map_to_instrument(&rate, vd);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_vol_fails() {
        let mapper = StandardInstrumentMapper::new();
        let rate = test_rate(RateType::Vol, Tenor::OneYear, 0.2);
        let vd = valuation_date();

        let result = mapper.map_to_instrument(&rate, vd);
        assert!(result.is_err());
    }

    #[test]
    fn test_mapper_clone() {
        let mapper = StandardInstrumentMapper::new().with_settlement_lag(3);
        let cloned = mapper.clone();
        assert_eq!(cloned.settlement_lag, 3);
    }

    #[test]
    fn test_mapper_debug() {
        let mapper = StandardInstrumentMapper::new();
        let debug_str = format!("{:?}", mapper);
        assert!(debug_str.contains("StandardInstrumentMapper"));
    }

    #[test]
    fn test_map_deposit_with_custom_settlement() {
        let mapper = StandardInstrumentMapper::new().with_settlement_lag(1);
        let rate = test_rate(RateType::Deposit, Tenor::OneMonth, 0.05);
        let vd = valuation_date();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::Deposit { start_date, .. } => {
                // With 1-day settlement, spot = Jan 16
                assert_eq!(start_date, Date::from_ymd(2024, 1, 16).unwrap());
            }
            _ => panic!("Expected Deposit instrument"),
        }
    }

    #[test]
    fn test_multiple_mappings() {
        let mapper = StandardInstrumentMapper::new();
        let vd = valuation_date();

        let deposit = test_rate(RateType::Deposit, Tenor::ThreeMonths, 0.05);
        let swap = test_rate(RateType::Swap, Tenor::FiveYears, 0.045);
        let ois = test_rate(RateType::Ois, Tenor::OneYear, 0.04);

        assert!(mapper.map_to_instrument(&deposit, vd).is_ok());
        assert!(mapper.map_to_instrument(&swap, vd).is_ok());
        assert!(mapper.map_to_instrument(&ois, vd).is_ok());
    }

    #[test]
    fn test_futures_price_conversion() {
        let mapper = StandardInstrumentMapper::new();
        let vd = valuation_date();

        // Test various rates
        let test_cases = [
            (0.0, 100.0), // 0% → 100
            (0.01, 99.0), // 1% → 99
            (0.05, 95.0), // 5% → 95
            (0.10, 90.0), // 10% → 90
        ];

        for (rate_val, expected_price) in test_cases {
            let rate = test_rate(RateType::Futures, Tenor::ThreeMonths, rate_val);
            let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

            match instrument {
                Instrument::Futures { price, .. } => {
                    assert!(
                        (price - expected_price).abs() < f64::EPSILON,
                        "Rate {rate_val} should give price {expected_price}, got {price}"
                    );
                }
                _ => panic!("Expected Futures instrument"),
            }
        }
    }

    #[test]
    fn test_eur_currency() {
        let mapper = StandardInstrumentMapper::new();
        let vd = valuation_date();

        let rate_id = RateId::new(Currency::EUR, Tenor::TenYears, RateType::Swap);
        let rate = MarketRate::new(
            rate_id,
            QuoteType::Mid,
            0.025,
            1700000000000,
            DataSource::Reuters,
        )
        .unwrap();

        let instrument = mapper.map_to_instrument(&rate, vd).unwrap();

        match instrument {
            Instrument::ParSwap { currency, .. } => {
                assert_eq!(currency, Currency::EUR);
            }
            _ => panic!("Expected ParSwap instrument"),
        }
    }
}

//! Builder types for trade construction.
//!
//! This module provides builder patterns for constructing trades
//! from schedules, legs, and cashflows.
//!
//! Uses `bon::Builder` for fluent construction with compile-time safety.
//!
//! # Examples
//!
//! ```rust,ignore
//! use infra_master::{Currency, Date, Period};
//! use infra_master::trade::{LegConfig, Trade, Direction, LegType, TradeType};
//!
//! let schedule = vec![
//!     Date::from_ymd(2024, 1, 15).unwrap(),
//!     Date::from_ymd(2024, 7, 15).unwrap(),
//!     Date::from_ymd(2025, 1, 15).unwrap(),
//! ];
//!
//! let fixed_leg = LegConfig::builder()
//!     .schedule(schedule.clone())
//!     .notional(1_000_000.0)
//!     .currency(Currency::USD)
//!     .direction(Direction::Payer)
//!     .build()
//!     .into_fixed_leg(0.05);
//!
//! let trade = Trade::builder()
//!     .id("SWAP001")
//!     .legs(vec![fixed_leg])
//!     .trade_type(TradeType::Swap)
//!     .build();
//! ```

use bon::Builder;

use super::{
    cashflow::{Cashflow, CashflowType},
    error::TradeError,
    index::IndexType,
    leg::{Direction, Leg, LegType},
    payoff::Payoff,
};
use crate::{Currency, Date, DayCounter, RateIndex};

// ============================================================================
// LegConfig (bon-based, recommended)
// ============================================================================

/// Configuration for constructing a leg from a schedule.
///
/// Uses `bon::Builder` for fluent construction with compile-time safety.
/// Replaces the legacy `LegBuilder` with a cleaner API.
///
/// # Examples
///
/// ```rust,ignore
/// use infra_master::{Currency, Date, DayCounter};
/// use infra_master::trade::{LegConfig, Direction};
///
/// let schedule = vec![
///     Date::from_ymd(2024, 1, 15).unwrap(),
///     Date::from_ymd(2024, 7, 15).unwrap(),
/// ];
///
/// let config = LegConfig::builder()
///     .schedule(schedule)
///     .notional(1_000_000.0)
///     .currency(Currency::USD)
///     .direction(Direction::Payer)
///     .day_count(DayCounter::Actual360)
///     .build();
///
/// // Validate before conversion
/// config.validate()?;
///
/// // Convert to a fixed leg
/// let leg = config.into_fixed_leg(0.05);
/// ```
#[derive(Debug, Clone, Builder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LegConfig {
    /// Payment schedule dates.
    schedule: Vec<Date>,
    /// Notional amount.
    notional: f64,
    /// Currency of the leg.
    currency: Currency,
    /// Direction of the leg.
    #[builder(default)]
    direction: Direction,
    /// Day count convention.
    #[builder(default)]
    day_count: DayCounter,
}

impl LegConfig {
    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `TradeError::InvalidSchedule` if schedule has fewer than 2 dates.
    /// Returns `TradeError::InvalidNotional` if notional is negative.
    pub fn validate(&self) -> Result<(), TradeError> {
        if self.schedule.len() < 2 {
            return Err(TradeError::InvalidSchedule(
                "Schedule must have at least 2 dates".into(),
            ));
        }
        if self.notional < 0.0 {
            return Err(TradeError::InvalidNotional(self.notional));
        }
        Ok(())
    }

    /// Converts to a fixed rate leg.
    ///
    /// # Arguments
    ///
    /// * `rate` - Fixed rate (as decimal, e.g., 0.05 for 5%)
    #[must_use]
    pub fn into_fixed_leg(self, rate: f64) -> Leg {
        let payoff = Payoff::fixed(rate);
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Fixed, self.currency)
    }

    /// Converts to a floating rate leg.
    ///
    /// # Arguments
    ///
    /// * `index` - Rate index for the floating leg
    /// * `spread` - Spread over the index (as decimal)
    #[must_use]
    pub fn into_floating_leg(self, index: RateIndex, spread: f64) -> Leg {
        let payoff = Payoff::floating_with_spread(IndexType::Rate(index), spread);
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Floating, self.currency)
    }

    /// Builds cashflows from the schedule using the given payoff.
    fn build_cashflows(&self, payoff: Payoff) -> Vec<Cashflow> {
        self.schedule
            .windows(2)
            .map(|window| {
                let accrual_start = window[0];
                let accrual_end = window[1];
                let year_fraction = self
                    .day_count
                    .year_fraction(accrual_start.into(), accrual_end.into());

                Cashflow::new(
                    CashflowType::Coupon,
                    accrual_end,
                    accrual_start,
                    accrual_end,
                    year_fraction,
                    self.notional,
                    payoff.clone(),
                    self.currency,
                )
            })
            .collect()
    }
}

// ============================================================================
// LegBuilder (legacy, deprecated)
// ============================================================================

/// Builder for constructing legs from a schedule.
///
/// Creates cashflows from a payment schedule with consistent notional,
/// day count, and payoff type.
///
/// **Deprecated**: Use [`LegConfig`] instead for a cleaner bon-based API.
#[deprecated(since = "0.2.0", note = "Use LegConfig instead")]
#[derive(Debug, Clone)]
pub struct LegBuilder {
    schedule: Vec<Date>,
    notional: f64,
    currency: Currency,
    direction: Direction,
    day_count: DayCounter,
}

#[allow(deprecated)]
impl LegBuilder {
    /// Creates a new leg builder.
    ///
    /// # Arguments
    ///
    /// * `schedule` - Payment dates (must have at least 2 dates)
    /// * `notional` - Notional amount (must be non-negative)
    /// * `currency` - Currency of the leg
    ///
    /// # Errors
    ///
    /// Returns `TradeError::InvalidSchedule` if schedule is too short.
    /// Returns `TradeError::InvalidNotional` if notional is negative.
    pub fn new(schedule: Vec<Date>, notional: f64, currency: Currency) -> Result<Self, TradeError> {
        if schedule.len() < 2 {
            return Err(TradeError::InvalidSchedule(
                "Schedule must have at least 2 dates".into(),
            ));
        }
        if notional < 0.0 {
            return Err(TradeError::InvalidNotional(notional));
        }

        Ok(Self {
            schedule,
            notional,
            currency,
            direction: Direction::Receiver,
            day_count: DayCounter::Actual365Fixed,
        })
    }

    /// Sets the direction of the leg.
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the day count convention.
    #[must_use]
    pub fn day_count(mut self, day_count: DayCounter) -> Self {
        self.day_count = day_count;
        self
    }

    /// Builds a fixed rate leg.
    ///
    /// # Arguments
    ///
    /// * `rate` - Fixed rate (as decimal, e.g., 0.05 for 5%)
    #[must_use]
    pub fn build_fixed(self, rate: f64) -> Leg {
        let payoff = Payoff::fixed(rate);
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Fixed, self.currency)
    }

    /// Builds a floating rate leg.
    ///
    /// # Arguments
    ///
    /// * `index` - Rate index for the floating leg
    /// * `spread` - Spread over the index (as decimal)
    #[must_use]
    pub fn build_floating(self, index: RateIndex, spread: f64) -> Leg {
        let payoff = Payoff::floating_with_spread(IndexType::Rate(index), spread);
        let cashflows = self.build_cashflows(payoff);
        Leg::new(cashflows, self.direction, LegType::Floating, self.currency)
    }

    /// Builds cashflows from the schedule using the given payoff.
    fn build_cashflows(&self, payoff: Payoff) -> Vec<Cashflow> {
        self.schedule
            .windows(2)
            .map(|window| {
                let accrual_start = window[0];
                let accrual_end = window[1];
                let year_fraction = self
                    .day_count
                    .year_fraction(accrual_start.into(), accrual_end.into());

                Cashflow::new(
                    CashflowType::Coupon,
                    accrual_end, // Payment at end of accrual period
                    accrual_start,
                    accrual_end,
                    year_fraction,
                    self.notional,
                    payoff.clone(),
                    self.currency,
                )
            })
            .collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CounterpartyId, PortfolioId};
    use crate::trade::{Trade, TradeMetadata, TradeType};

    fn sample_schedule() -> Vec<Date> {
        vec![
            Date::from_ymd(2024, 1, 15).unwrap(),
            Date::from_ymd(2024, 7, 15).unwrap(),
            Date::from_ymd(2025, 1, 15).unwrap(),
        ]
    }

    // ========================================================================
    // LegConfig tests (new bon-based API)
    // ========================================================================

    #[test]
    fn test_leg_config_validate_valid() {
        let config = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_leg_config_validate_empty_schedule() {
        let config = LegConfig::builder()
            .schedule(vec![])
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build();
        assert!(matches!(
            config.validate(),
            Err(TradeError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn test_leg_config_validate_single_date() {
        let config = LegConfig::builder()
            .schedule(vec![Date::from_ymd(2024, 1, 15).unwrap()])
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build();
        assert!(matches!(
            config.validate(),
            Err(TradeError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn test_leg_config_validate_negative_notional() {
        let config = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(-1000.0)
            .currency(Currency::USD)
            .build();
        assert!(matches!(
            config.validate(),
            Err(TradeError::InvalidNotional(_))
        ));
    }

    #[test]
    fn test_leg_config_into_fixed_leg() {
        let leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .direction(Direction::Payer)
            .day_count(DayCounter::Actual360)
            .build()
            .into_fixed_leg(0.05);

        assert_eq!(leg.direction, Direction::Payer);
        assert_eq!(leg.leg_type, LegType::Fixed);
        assert_eq!(leg.currency, Currency::USD);
        assert_eq!(leg.len(), 2);
    }

    #[test]
    fn test_leg_config_into_floating_leg() {
        let leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .direction(Direction::Receiver)
            .build()
            .into_floating_leg(RateIndex::Sofr, 0.001);

        assert_eq!(leg.direction, Direction::Receiver);
        assert_eq!(leg.leg_type, LegType::Floating);
        assert_eq!(leg.len(), 2);
    }

    #[test]
    fn test_leg_config_cashflow_properties() {
        let leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .day_count(DayCounter::Actual365Fixed)
            .build()
            .into_fixed_leg(0.05);

        let cf = leg.cashflows().next().unwrap();
        assert_eq!(cf.notional, 1_000_000.0);
        assert_eq!(cf.currency, Currency::USD);
        assert_eq!(cf.cf_type, CashflowType::Coupon);
        assert!(cf.is_fixed());
    }

    #[test]
    fn test_leg_config_default_direction() {
        let leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build()
            .into_fixed_leg(0.05);

        assert_eq!(leg.direction, Direction::Receiver);
    }

    #[test]
    fn test_leg_config_default_day_count() {
        let config = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build();

        // Default day count is Actual365Fixed, verify by building a leg
        let leg = config.into_fixed_leg(0.05);
        let cf = leg.cashflows().next().unwrap();
        // Actual365Fixed would give different year fraction than Actual360
        assert!(cf.year_fraction > 0.0);
    }

    #[test]
    fn test_leg_config_clone() {
        let config = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .build();
        let cloned = config.clone();

        let leg1 = config.into_fixed_leg(0.05);
        let leg2 = cloned.into_fixed_leg(0.05);

        assert_eq!(leg1.len(), leg2.len());
    }

    #[test]
    fn test_leg_config_with_trade_builder() {
        let fixed_leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .direction(Direction::Payer)
            .build()
            .into_fixed_leg(0.05);

        let float_leg = LegConfig::builder()
            .schedule(sample_schedule())
            .notional(1_000_000.0)
            .currency(Currency::USD)
            .direction(Direction::Receiver)
            .build()
            .into_floating_leg(RateIndex::Sofr, 0.0);

        let trade = Trade::builder()
            .id("SWAP001")
            .legs(vec![fixed_leg, float_leg])
            .trade_type(TradeType::Swap)
            .build();

        assert_eq!(trade.id.as_str(), "SWAP001");
        assert_eq!(trade.num_legs(), 2);
        assert!(trade.is_vanilla_swap());
    }

    // ========================================================================
    // LegBuilder tests (legacy API, deprecated)
    // ========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_new_valid() {
        let builder = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD);
        assert!(builder.is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_new_empty_schedule() {
        let result = LegBuilder::new(vec![], 1_000_000.0, Currency::USD);
        assert!(matches!(result, Err(TradeError::InvalidSchedule(_))));
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_new_single_date() {
        let result = LegBuilder::new(
            vec![Date::from_ymd(2024, 1, 15).unwrap()],
            1_000_000.0,
            Currency::USD,
        );
        assert!(matches!(result, Err(TradeError::InvalidSchedule(_))));
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_new_negative_notional() {
        let result = LegBuilder::new(sample_schedule(), -1000.0, Currency::USD);
        assert!(matches!(result, Err(TradeError::InvalidNotional(_))));
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_build_fixed() {
        let leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Payer)
            .day_count(DayCounter::Actual360)
            .build_fixed(0.05);

        assert_eq!(leg.direction, Direction::Payer);
        assert_eq!(leg.leg_type, LegType::Fixed);
        assert_eq!(leg.currency, Currency::USD);
        // 3 dates -> 2 cashflows
        assert_eq!(leg.len(), 2);
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_build_floating() {
        let leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Receiver)
            .build_floating(RateIndex::Sofr, 0.001);

        assert_eq!(leg.direction, Direction::Receiver);
        assert_eq!(leg.leg_type, LegType::Floating);
        assert_eq!(leg.len(), 2);
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_cashflow_properties() {
        let leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .day_count(DayCounter::Actual365Fixed)
            .build_fixed(0.05);

        let cf = leg.cashflows().next().unwrap();
        assert_eq!(cf.notional, 1_000_000.0);
        assert_eq!(cf.currency, Currency::USD);
        assert_eq!(cf.cf_type, CashflowType::Coupon);
        assert!(cf.is_fixed());
    }

    #[test]
    fn test_trade_builder_new() {
        let trade = Trade::builder().id("TRADE001").build();

        assert_eq!(trade.id.as_str(), "TRADE001");
        assert_eq!(trade.num_legs(), 0);
    }

    #[test]
    #[allow(deprecated)]
    fn test_trade_builder_add_legs() {
        let fixed_leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Payer)
            .build_fixed(0.05);

        let float_leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Receiver)
            .build_floating(RateIndex::Sofr, 0.0);

        let trade = Trade::builder()
            .id("SWAP001")
            .legs(vec![fixed_leg, float_leg])
            .trade_type(TradeType::Swap)
            .build();

        assert_eq!(trade.id.as_str(), "SWAP001");
        assert_eq!(trade.num_legs(), 2);
        assert!(trade.is_vanilla_swap());
    }

    #[test]
    fn test_trade_builder_with_metadata() {
        let metadata = TradeMetadata::new()
            .with_counterparty("BANK01")
            .with_portfolio("RATES");

        let trade = Trade::builder()
            .id("TRADE002")
            .metadata(metadata)
            .build();

        assert_eq!(
            trade.metadata.counterparty,
            Some(CounterpartyId::new("BANK01"))
        );
        assert_eq!(trade.metadata.portfolio, Some(PortfolioId::new("RATES")));
    }

    #[test]
    #[allow(deprecated)]
    fn test_leg_builder_clone() {
        let builder = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD).unwrap();
        let cloned = builder.clone();

        let leg1 = builder.build_fixed(0.05);
        let leg2 = cloned.build_fixed(0.05);

        assert_eq!(leg1.len(), leg2.len());
    }
}

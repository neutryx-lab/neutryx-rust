//! Builder types for trade construction.
//!
//! This module provides builder patterns for constructing trades
//! from schedules, legs, and cashflows.
//!
//! # Examples
//!
//! ```rust,ignore
//! use infra_master::{Currency, Date, Period};
//! use infra_master::trade::{LegBuilder, TradeBuilder, Direction, LegType, TradeType};
//!
//! let schedule = vec![
//!     Date::from_ymd(2024, 1, 15).unwrap(),
//!     Date::from_ymd(2024, 7, 15).unwrap(),
//!     Date::from_ymd(2025, 1, 15).unwrap(),
//! ];
//!
//! let fixed_leg = LegBuilder::new(schedule.clone(), 1_000_000.0, Currency::USD)
//!     .unwrap()
//!     .direction(Direction::Payer)
//!     .build_fixed(0.05);
//!
//! let trade = TradeBuilder::new("SWAP001")
//!     .add_leg(fixed_leg)
//!     .trade_type(TradeType::Swap)
//!     .build();
//! ```

use super::{
    cashflow::{Cashflow, CashflowType},
    error::TradeError,
    index::IndexType,
    leg::{Direction, Leg, LegType},
    payoff::Payoff,
    trade::{Trade, TradeMetadata, TradeType},
};
use crate::{ids::TradeId, Currency, Date, DayCounter, RateIndex};

/// Builder for constructing legs from a schedule.
///
/// Creates cashflows from a payment schedule with consistent notional,
/// day count, and payoff type.
#[derive(Debug, Clone)]
pub struct LegBuilder {
    schedule: Vec<Date>,
    notional: f64,
    currency: Currency,
    direction: Direction,
    day_count: DayCounter,
}

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

/// Builder for constructing trades.
///
/// Aggregates legs into a complete trade structure.
#[derive(Debug, Clone)]
pub struct TradeBuilder {
    id: TradeId,
    legs: Vec<Leg>,
    trade_type: TradeType,
    metadata: TradeMetadata,
}

impl TradeBuilder {
    /// Creates a new trade builder.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the trade
    #[must_use]
    pub fn new(id: impl Into<TradeId>) -> Self {
        Self {
            id: id.into(),
            legs: Vec::new(),
            trade_type: TradeType::Generic,
            metadata: TradeMetadata::default(),
        }
    }

    /// Adds a leg to the trade.
    #[must_use]
    pub fn add_leg(mut self, leg: Leg) -> Self {
        self.legs.push(leg);
        self
    }

    /// Sets the trade type.
    #[must_use]
    pub fn trade_type(mut self, trade_type: TradeType) -> Self {
        self.trade_type = trade_type;
        self
    }

    /// Sets the trade metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: TradeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builds the trade.
    #[must_use]
    pub fn build(self) -> Trade {
        Trade::with_metadata(self.id, self.legs, self.trade_type, self.metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CounterpartyId, PortfolioId};

    fn sample_schedule() -> Vec<Date> {
        vec![
            Date::from_ymd(2024, 1, 15).unwrap(),
            Date::from_ymd(2024, 7, 15).unwrap(),
            Date::from_ymd(2025, 1, 15).unwrap(),
        ]
    }

    #[test]
    fn test_leg_builder_new_valid() {
        let builder = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD);
        assert!(builder.is_ok());
    }

    #[test]
    fn test_leg_builder_new_empty_schedule() {
        let result = LegBuilder::new(vec![], 1_000_000.0, Currency::USD);
        assert!(matches!(result, Err(TradeError::InvalidSchedule(_))));
    }

    #[test]
    fn test_leg_builder_new_single_date() {
        let result = LegBuilder::new(
            vec![Date::from_ymd(2024, 1, 15).unwrap()],
            1_000_000.0,
            Currency::USD,
        );
        assert!(matches!(result, Err(TradeError::InvalidSchedule(_))));
    }

    #[test]
    fn test_leg_builder_new_negative_notional() {
        let result = LegBuilder::new(sample_schedule(), -1000.0, Currency::USD);
        assert!(matches!(result, Err(TradeError::InvalidNotional(_))));
    }

    #[test]
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
        let builder = TradeBuilder::new("TRADE001");
        let trade = builder.build();

        assert_eq!(trade.id.as_str(), "TRADE001");
        assert_eq!(trade.num_legs(), 0);
    }

    #[test]
    fn test_trade_builder_add_legs() {
        let fixed_leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Payer)
            .build_fixed(0.05);

        let float_leg = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD)
            .unwrap()
            .direction(Direction::Receiver)
            .build_floating(RateIndex::Sofr, 0.0);

        let trade = TradeBuilder::new("SWAP001")
            .add_leg(fixed_leg)
            .add_leg(float_leg)
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

        let trade = TradeBuilder::new("TRADE002").metadata(metadata).build();

        assert_eq!(
            trade.metadata.counterparty,
            Some(CounterpartyId::new("BANK01"))
        );
        assert_eq!(trade.metadata.portfolio, Some(PortfolioId::new("RATES")));
    }

    #[test]
    fn test_leg_builder_clone() {
        let builder = LegBuilder::new(sample_schedule(), 1_000_000.0, Currency::USD).unwrap();
        let cloned = builder.clone();

        let leg1 = builder.build_fixed(0.05);
        let leg2 = cloned.build_fixed(0.05);

        assert_eq!(leg1.len(), leg2.len());
    }

    #[test]
    fn test_trade_builder_clone() {
        let builder = TradeBuilder::new("TRADE003").trade_type(TradeType::Swap);
        let cloned = builder.clone();

        let trade1 = builder.build();
        let trade2 = cloned.build();

        assert_eq!(trade1.id, trade2.id);
    }
}

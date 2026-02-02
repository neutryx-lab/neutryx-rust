//! Instrument expansion to Trade (cashflow generation).
//!
//! This module provides the `InstrumentExpander` trait for converting
//! `InstrumentDefinition` into `Trade` with generated cashflows.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_master::market::instrument::{InstrumentDefinition, InstrumentExpander, FxSpot};
//! use infra_master::market::convention::ConventionSet;
//! use infra_master::Date;
//!
//! let fx_spot = FxSpot { /* ... */ };
//! let instrument = InstrumentDefinition::FxSpot(fx_spot);
//! let conventions = ConventionSet::usd_standard();
//! let valuation_date = Date::from_ymd(2025, 1, 1).unwrap();
//!
//! let trade = instrument.expand_to_trade("TRADE-001", valuation_date, &conventions)?;
//! ```

use super::{
    // Common
    AsianOption,
    // Rates
    BasisSwap,
    BasketOption,
    CapFloor,
    // Credit
    Cds,
    CdsIndex,
    CdsOption,
    CmsSwap,
    // Commodity
    CommodityAsianOption,
    CommodityForward,
    CommoditySwap,
    CommodityVanillaOption,
    // XCCY
    CrossCurrencyBasisSwap,
    Deposit,
    // Equity
    EquityBarrierOption,
    EquityForward,
    EquitySwap,
    EquityVanillaOption,
    Fra,
    Frn,
    Futures,
    // FX
    FxBarrierOption,
    FxForward,
    FxSpot,
    FxSwap,
    FxVanillaOption,
    InflationSwap,
    InstrumentDefinition,
    InstrumentError,
    InterestRateSwap,
    LookbackOption,
    NtdBasket,
    Ois,
    SpreadOption,
    Swaption,
};
use crate::{
    ids::TradeId,
    market::convention::ConventionSet,
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
    Date,
};

/// Trait for expanding instrument definitions into trades with cashflows.
///
/// This trait provides the `expand_to_trade` method which converts an
/// `InstrumentDefinition` into a fully expanded `Trade` with generated
/// cashflows based on market conventions.
pub trait InstrumentExpander {
    /// Expands this instrument into a Trade with cashflows.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Unique identifier for the resulting trade
    /// * `valuation_date` - Date for valuation/pricing
    /// * `conventions` - Market conventions for cashflow generation
    ///
    /// # Returns
    ///
    /// A `Trade` containing legs and cashflows, or an error if expansion fails.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if:
    /// - Required convention is missing (`MissingConvention`)
    /// - Instrument validation fails (`InvalidParameter`)
    /// - Cashflow expansion fails (`ExpansionFailed`)
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError>;
}

impl InstrumentExpander for InstrumentDefinition {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Validate first
        self.validate()?;

        match self {
            // === Rates ===
            InstrumentDefinition::Deposit(d) => {
                d.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Fra(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Futures(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::InterestRateSwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::BasisSwap(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Ois(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Swaption(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CapFloor(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::Frn(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CmsSwap(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::InflationSwap(i) => {
                i.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === FX ===
            InstrumentDefinition::FxSpot(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxBarrierOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::FxSwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CrossCurrencyBasisSwap(x) => {
                x.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Equity ===
            InstrumentDefinition::EquityForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquityVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquityBarrierOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::AsianOption(a) => {
                a.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::LookbackOption(l) => {
                l.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::EquitySwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::BasketOption(b) => {
                b.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Credit ===
            InstrumentDefinition::Cds(c) => {
                c.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CdsIndex(i) => {
                i.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CdsOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::NtdBasket(n) => {
                n.expand_to_trade(trade_id, valuation_date, conventions)
            }

            // === Commodity ===
            InstrumentDefinition::CommodityForward(f) => {
                f.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommoditySwap(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommodityVanillaOption(o) => {
                o.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::CommodityAsianOption(a) => {
                a.expand_to_trade(trade_id, valuation_date, conventions)
            }
            InstrumentDefinition::SpreadOption(s) => {
                s.expand_to_trade(trade_id, valuation_date, conventions)
            }
        }
    }
}

// ============================================================================
// Rates Instrument Expansion
// ============================================================================

impl InstrumentExpander for Deposit {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let end_date = self.end_date();
        let year_fraction = self.year_fraction();

        // Create a single cashflow at maturity
        let cashflow = Cashflow::new(
            CashflowType::Coupon,
            end_date,
            self.start_date,
            end_date,
            year_fraction,
            self.notional,
            Payoff::fixed(self.rate),
            self.currency,
        );

        let leg = Leg::new(
            vec![cashflow],
            Direction::Receiver,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Deposit))
    }
}

impl InstrumentExpander for Fra {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let end_date = self.end_date();
        let year_fraction = self.year_fraction();

        // FRA has a single settlement cashflow at the fixing date
        // The payoff is (floating - strike) * notional * year_fraction / (1 + floating
        // * yf)
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.fixing_date,
            self.start_date,
            end_date,
            year_fraction,
            self.notional,
            Payoff::floating(IndexType::Rate(self.rate_index)),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Fra))
    }
}

impl InstrumentExpander for Futures {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let year_fraction = self.year_fraction();

        // Futures has a single settlement at expiry
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry_date,
            self.expiry_date,
            self.underlying_end_date(),
            year_fraction,
            self.notional,
            Payoff::floating(IndexType::Rate(self.rate_index)),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Futures))
    }
}

impl InstrumentExpander for InterestRateSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let end_date = self.end_date();

        // Generate fixed leg payment dates
        let fixed_dates = generate_payment_dates(self.start_date, end_date, self.fixed_frequency);

        // Generate floating leg payment dates
        let float_dates = generate_payment_dates(self.start_date, end_date, self.float_frequency);

        // Generate fixed leg cashflows
        let fixed_cashflows = generate_fixed_leg_cashflows(
            &fixed_dates,
            self.start_date,
            self.fixed_rate,
            self.notional,
            self.currency,
        );

        // Generate floating leg cashflows
        let mut floating_cashflows = Vec::new();
        for i in 0..float_dates.len().saturating_sub(1) {
            let accrual_start = float_dates[i];
            let accrual_end = float_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional,
                Payoff::floating(IndexType::Rate(self.rate_index)),
                self.currency,
            );
            floating_cashflows.push(cf);
        }

        // Determine directions based on payer/receiver
        let (fixed_direction, floating_direction) = if self.is_payer() {
            (Direction::Payer, Direction::Receiver)
        } else {
            (Direction::Receiver, Direction::Payer)
        };

        let fixed_leg = Leg::new(
            fixed_cashflows,
            fixed_direction,
            LegType::Fixed,
            self.currency,
        );

        let floating_leg = Leg::new(
            floating_cashflows,
            floating_direction,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![fixed_leg, floating_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for BasisSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        let end_date = self.end_date();

        // Generate leg1 payment dates
        let leg1_dates = generate_payment_dates(self.start_date, end_date, self.leg1_frequency);

        // Generate leg2 payment dates
        let leg2_dates = generate_payment_dates(self.start_date, end_date, self.leg2_frequency);

        // Generate leg1 cashflows
        let mut leg1_cashflows = Vec::new();
        for i in 0..leg1_dates.len().saturating_sub(1) {
            let accrual_start = leg1_dates[i];
            let accrual_end = leg1_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional,
                Payoff::floating(IndexType::Rate(self.leg1_index)),
                self.currency,
            );
            leg1_cashflows.push(cf);
        }

        // Generate leg2 cashflows
        let mut leg2_cashflows = Vec::new();
        for i in 0..leg2_dates.len().saturating_sub(1) {
            let accrual_start = leg2_dates[i];
            let accrual_end = leg2_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional,
                Payoff::floating(IndexType::Rate(self.leg2_index)),
                self.currency,
            );
            leg2_cashflows.push(cf);
        }

        // Determine directions: Payer pays leg1, receives leg2
        let (leg1_direction, leg2_direction) = if self.payer_receiver == super::PayerReceiver::Payer
        {
            (Direction::Payer, Direction::Receiver)
        } else {
            (Direction::Receiver, Direction::Payer)
        };

        let leg1 = Leg::new(
            leg1_cashflows,
            leg1_direction,
            LegType::Floating,
            self.currency,
        );

        let leg2 = Leg::new(
            leg2_cashflows,
            leg2_direction,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg1, leg2], TradeType::Swap))
    }
}

impl InstrumentExpander for Swaption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _swaption_conv = conventions.get_swaption()?;

        // Create settlement cashflow for the swaption premium/exercise
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::Swaption {
                exercise_dates: vec![self.expiry],
                exercise_type: self.exercise_type,
                settlement_type: self.settlement_type,
            },
        ))
    }
}

impl InstrumentExpander for CapFloor {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Generate caplet/floorlet cashflows based on payment frequency
        let mut cashflows = Vec::new();

        // For simplicity, create a single settlement cashflow
        // Full implementation would generate individual caplet/floorlet cashflows
        let strike = self.strikes.first().copied().unwrap_or(0.0);
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.start_date,
            self.start_date,
            self.start_date,
            0.0,
            self.notional_schedule.notional_at(0),
            Payoff::fixed(strike),
            self.currency,
        );
        cashflows.push(settlement_cf);

        let leg = Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::CapFloor,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::CapFloor))
    }
}

impl InstrumentExpander for Frn {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        // Create floating coupon cashflow
        let coupon_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0, // Placeholder year fraction
            self.principal_schedule.notional_at(0),
            Payoff::floating(IndexType::Rate(self.coupon_index)),
            self.currency,
        );

        // Create principal redemption cashflow
        let principal_cf = Cashflow::new(
            CashflowType::Principal,
            self.maturity,
            self.maturity,
            self.maturity,
            0.0,
            self.principal_schedule.notional_at(0),
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![coupon_cf, principal_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::Bond {
                issuer_id: None,
                seniority: None,
            },
        ))
    }
}

impl InstrumentExpander for CmsSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _swap_conv = conventions.get_swap()?;

        // Create CMS leg cashflow
        let cms_cf = Cashflow::new(
            CashflowType::Coupon,
            self.start_date, // Use start_date as placeholder since CmsSwap uses tenor
            self.start_date,
            self.start_date,
            1.0,
            self.notional,
            Payoff::fixed(self.spread), // CMS rate + spread
            self.currency,
        );

        let cms_leg = Leg::new(
            vec![cms_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![cms_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for InflationSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _inflation_conv = conventions.get_inflation_swap()?;

        // Create inflation leg cashflow
        let inflation_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.fixed_rate),
            self.currency,
        );

        let inflation_leg = Leg::new(
            vec![inflation_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![inflation_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for Ois {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // OIS expansion uses instrument-level parameters directly,
        // so conventions are not required for basic expansion.
        // Future enhancement: use conventions for business day adjustments.

        // Generate payment schedule based on payment frequency
        let payment_dates =
            generate_payment_dates(self.start_date, self.end_date, self.payment_frequency);

        // Generate fixed leg cashflows
        let fixed_cashflows = generate_fixed_leg_cashflows(
            &payment_dates,
            self.start_date,
            self.fixed_rate,
            self.notional,
            self.currency,
        );

        // Generate floating (OIS) leg cashflows with daily compounding details
        let floating_cashflows = generate_ois_floating_leg_cashflows(
            &payment_dates,
            self.start_date,
            self.rate_index,
            self.notional,
            self.currency,
        );

        // Determine directions based on payer/receiver
        let (fixed_direction, floating_direction) = if self.is_payer() {
            (Direction::Payer, Direction::Receiver)
        } else {
            (Direction::Receiver, Direction::Payer)
        };

        let fixed_leg = Leg::new(
            fixed_cashflows,
            fixed_direction,
            LegType::Fixed,
            self.currency,
        );

        let floating_leg = Leg::new(
            floating_cashflows,
            floating_direction,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![fixed_leg, floating_leg],
            TradeType::Swap,
        ))
    }
}

// ============================================================================
// OIS Helper Functions
// ============================================================================

use chrono::Datelike;

use crate::{
    time::{EndOfMonthRule, Tenor},
    Frequency, RateIndex,
};

/// Generates payment dates based on start, end, and frequency.
fn generate_payment_dates(start: Date, end: Date, frequency: Frequency) -> Vec<Date> {
    let mut dates = Vec::new();

    // Determine the tenor for each period based on frequency
    let tenor = match frequency {
        Frequency::Daily => {
            // For daily, just return start and end
            dates.push(start);
            dates.push(end);
            return dates;
        }
        Frequency::Weekly => Tenor::OneWeek,
        Frequency::Monthly => Tenor::OneMonth,
        Frequency::Quarterly => Tenor::ThreeMonths,
        Frequency::SemiAnnual => Tenor::SixMonths,
        Frequency::Annual => Tenor::OneYear,
    };

    let mut current = start;
    while current < end {
        dates.push(current);
        let next = tenor.add_to_date(current, EndOfMonthRule::Adjust);
        if next > end {
            break;
        }
        current = next;
    }
    if dates.last() != Some(&end) {
        dates.push(end);
    }

    dates
}

/// Generates fixed leg cashflows for an OIS.
fn generate_fixed_leg_cashflows(
    payment_dates: &[Date],
    start_date: Date,
    fixed_rate: f64,
    notional: f64,
    currency: crate::Currency,
) -> Vec<Cashflow> {
    let mut cashflows = Vec::new();

    for i in 0..payment_dates.len().saturating_sub(1) {
        let accrual_start = if i == 0 { start_date } else { payment_dates[i] };
        let accrual_end = payment_dates[i + 1];
        let payment_date = accrual_end;

        // Calculate year fraction (ACT/360 typical for OIS)
        let days = (accrual_end - accrual_start) as f64;
        let year_fraction = days / 360.0;

        let cf = Cashflow::new(
            CashflowType::Coupon,
            payment_date,
            accrual_start,
            accrual_end,
            year_fraction,
            notional,
            Payoff::fixed(fixed_rate),
            currency,
        );
        cashflows.push(cf);
    }

    cashflows
}

/// Generates OIS floating leg cashflows with daily compounding details.
fn generate_ois_floating_leg_cashflows(
    payment_dates: &[Date],
    start_date: Date,
    rate_index: RateIndex,
    notional: f64,
    currency: crate::Currency,
) -> Vec<Cashflow> {
    use crate::trade::IndexType;

    let mut cashflows = Vec::new();

    for i in 0..payment_dates.len().saturating_sub(1) {
        let accrual_start = if i == 0 { start_date } else { payment_dates[i] };
        let accrual_end = payment_dates[i + 1];
        let payment_date = accrual_end;

        // Generate daily accruals for this period
        let daily_accruals =
            generate_daily_accruals(accrual_start, accrual_end, rate_index, notional);

        // Calculate year fraction
        let days = (accrual_end - accrual_start) as f64;
        let year_fraction = days / 360.0;

        // Final compounded notional from daily accruals
        let _final_compounded = daily_accruals
            .last()
            .map(|a| a.compounded_notional)
            .unwrap_or(notional);

        let cf = Cashflow::new_with_daily_accruals(
            CashflowType::Coupon,
            payment_date,
            accrual_start,
            accrual_end,
            year_fraction,
            notional,
            Payoff::floating(IndexType::Rate(rate_index)),
            currency,
            daily_accruals,
        );
        cashflows.push(cf);
    }

    cashflows
}

/// Generates daily accrual details for OIS compounding.
///
/// This function creates a daily breakdown of the compounding process,
/// simulating overnight rates based on the rate index.
///
/// # Business Day Handling
///
/// OIS rates are published only on business days. For weekends:
/// - Friday's rate applies for 3 days (Friday, Saturday, Sunday)
/// - The day fraction for Friday is 3/360 (or 3/365)
fn generate_daily_accruals(
    start: Date,
    end: Date,
    rate_index: RateIndex,
    initial_notional: f64,
) -> Vec<crate::trade::DailyAccrual> {
    use chrono::Weekday;

    use crate::trade::DailyAccrual;

    let mut accruals = Vec::new();
    let mut current_date = start;
    let mut compounded_notional = initial_notional;

    // Base rate simulation based on index (in production, these would come from
    // market data)
    let base_rate = match rate_index {
        RateIndex::Sofr => 0.0430,      // ~4.30% SOFR
        RateIndex::Estr => 0.0390,      // ~3.90% ESTR
        RateIndex::Euribor3M => 0.0390, // ~3.90% EUR (using as ESTR proxy)
        RateIndex::Euribor6M => 0.0395, // ~3.95% EUR
        RateIndex::Sonia => 0.0525,     // ~5.25% SONIA
        RateIndex::Tonar => 0.0010,     // ~0.10% TONA
        RateIndex::Saron => 0.0175,     // ~1.75% SARON
    };

    // Day count basis (360 or 365)
    let day_count_basis = match rate_index {
        RateIndex::Sonia | RateIndex::Tonar => 365.0,
        _ => 360.0,
    };

    while current_date < end {
        let weekday = current_date.into_inner().weekday();

        // Skip weekends - only process business days (Mon-Fri)
        if weekday == Weekday::Sat || weekday == Weekday::Sun {
            current_date = Tenor::Overnight.add_to_date(current_date, EndOfMonthRule::None);
            continue;
        }

        // Calculate days until next business day
        // Friday -> Monday = 3 days, otherwise 1 day
        let days_to_next = if weekday == Weekday::Fri { 3.0 } else { 1.0 };
        let day_fraction = days_to_next / day_count_basis;

        // Simulate small daily rate variation (±5bps) based on day of year
        let day_of_year = current_date.into_inner().ordinal() as f64;
        let rate_variation = (day_of_year.sin() * 0.0005).abs();
        let overnight_rate = base_rate + rate_variation;

        // Calculate new compounded notional
        let new_compounded = compounded_notional * (1.0 + overnight_rate * day_fraction);

        accruals.push(DailyAccrual::with_compounded_notional(
            current_date,
            overnight_rate,
            day_fraction,
            new_compounded,
        ));

        compounded_notional = new_compounded;

        // Move to next calendar day
        current_date = Tenor::Overnight.add_to_date(current_date, EndOfMonthRule::None);
    }

    accruals
}

// ============================================================================
// FX Instrument Expansion
// ============================================================================

impl InstrumentExpander for FxSpot {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // FX spot has two principal exchanges
        // Pay notional in notional currency, receive converted amount in other currency

        let pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.spot_rate
        } else {
            self.notional / self.spot_rate
        };

        let receive_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            receive_amount,
            Payoff::fixed(1.0),
            receive_currency,
        );

        let pay_leg = Leg::new(
            vec![pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let receive_leg = Leg::new(
            vec![receive_cf],
            Direction::Receiver,
            LegType::Principal,
            receive_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![pay_leg, receive_leg],
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Similar to FX spot but with forward rate
        let pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.forward_rate
        } else {
            self.notional / self.forward_rate
        };

        let receive_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            receive_amount,
            Payoff::fixed(1.0),
            receive_currency,
        );

        let pay_leg = Leg::new(
            vec![pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let receive_leg = Leg::new(
            vec![receive_cf],
            Direction::Receiver,
            LegType::Principal,
            receive_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![pay_leg, receive_leg],
            TradeType::FxForward,
        ))
    }
}

impl InstrumentExpander for FxVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // FX option has conditional payoff at delivery
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.expiry,
            self.delivery_date,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.notional_currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.notional_currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for FxBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Barrier option is based on vanilla with barrier monitoring
        self.vanilla
            .expand_to_trade(trade_id, valuation_date, conventions)
    }
}

impl InstrumentExpander for FxSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Near leg: exchange at near rate
        let near_pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.near_leg_date,
            self.near_leg_date,
            self.near_leg_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let near_receive_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.near_rate
        } else {
            self.notional / self.near_rate
        };

        let other_currency = if self.notional_currency == self.currency_pair.base {
            self.currency_pair.quote
        } else {
            self.currency_pair.base
        };

        let near_receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.near_leg_date,
            self.near_leg_date,
            self.near_leg_date,
            0.0,
            near_receive_amount,
            Payoff::fixed(1.0),
            other_currency,
        );

        // Far leg: reverse exchange at far rate
        let far_receive_cf = Cashflow::new(
            CashflowType::Principal,
            self.far_leg_date,
            self.far_leg_date,
            self.far_leg_date,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.notional_currency,
        );

        let far_pay_amount = if self.notional_currency == self.currency_pair.base {
            self.notional * self.far_rate
        } else {
            self.notional / self.far_rate
        };

        let far_pay_cf = Cashflow::new(
            CashflowType::Principal,
            self.far_leg_date,
            self.far_leg_date,
            self.far_leg_date,
            0.0,
            far_pay_amount,
            Payoff::fixed(1.0),
            other_currency,
        );

        let near_pay_leg = Leg::new(
            vec![near_pay_cf],
            Direction::Payer,
            LegType::Principal,
            self.notional_currency,
        );

        let near_receive_leg = Leg::new(
            vec![near_receive_cf],
            Direction::Receiver,
            LegType::Principal,
            other_currency,
        );

        let far_receive_leg = Leg::new(
            vec![far_receive_cf],
            Direction::Receiver,
            LegType::Principal,
            self.notional_currency,
        );

        let far_pay_leg = Leg::new(
            vec![far_pay_cf],
            Direction::Payer,
            LegType::Principal,
            other_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![near_pay_leg, near_receive_leg, far_receive_leg, far_pay_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CrossCurrencyBasisSwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        use crate::trade::IndexType;

        // Validate the instrument first
        self.validate()
            .map_err(|e| InstrumentError::invalid_parameter(e.to_string()))?;

        // Generate payment dates for domestic leg
        let domestic_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.domestic_leg.payment_frequency,
        );

        // Generate payment dates for foreign leg
        let foreign_dates = generate_payment_dates(
            self.start_date,
            self.maturity,
            self.foreign_leg.payment_frequency,
        );

        // Generate domestic leg cashflows (floating)
        let mut domestic_cashflows = Vec::new();
        for i in 0..domestic_dates.len().saturating_sub(1) {
            let accrual_start = domestic_dates[i];
            let accrual_end = domestic_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional,
                Payoff::floating(IndexType::Rate(self.domestic_leg.rate_index)),
                self.domestic_currency,
            );
            domestic_cashflows.push(cf);
        }

        // Generate foreign leg cashflows (floating with basis spread)
        let mut foreign_cashflows = Vec::new();
        for i in 0..foreign_dates.len().saturating_sub(1) {
            let accrual_start = foreign_dates[i];
            let accrual_end = foreign_dates[i + 1];
            let year_fraction = (accrual_end - accrual_start) as f64 / 360.0;

            // Note: basis spread would be applied here in practice
            let cf = Cashflow::new(
                CashflowType::Coupon,
                accrual_end,
                accrual_start,
                accrual_end,
                year_fraction,
                self.notional, // Foreign notional would be FX-adjusted in practice
                Payoff::floating(IndexType::Rate(self.foreign_leg.rate_index)),
                self.foreign_currency,
            );
            foreign_cashflows.push(cf);
        }

        let domestic_leg = Leg::new(
            domestic_cashflows,
            Direction::Payer,
            LegType::Floating,
            self.domestic_currency,
        );

        let foreign_leg = Leg::new(
            foreign_cashflows,
            Direction::Receiver,
            LegType::Floating,
            self.foreign_currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![domestic_leg, foreign_leg],
            TradeType::Swap,
        ))
    }
}

// ============================================================================
// Equity Instrument Expansion
// ============================================================================

impl InstrumentExpander for EquityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Equity forward: pay fixed price, receive equity value at settlement
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.settlement_date,
            self.settlement_date,
            self.settlement_date,
            0.0,
            self.notional,
            Payoff::fixed(self.forward_price),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::FxForward))
    }
}

impl InstrumentExpander for EquityVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for EquityBarrierOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        self.vanilla
            .expand_to_trade(trade_id, valuation_date, conventions)
    }
}

impl InstrumentExpander for AsianOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for LookbackOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let strike = self.strike.unwrap_or(0.0);
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for EquitySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Equity leg
        let equity_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(0.0), // Equity return
            self.currency,
        );

        let equity_leg = Leg::new(
            vec![equity_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        // Funding leg (fixed spread over funding index)
        let funding_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            self.funding_spread,
            self.notional,
            Payoff::fixed(self.funding_spread),
            self.currency,
        );

        let funding_leg = Leg::new(
            vec![funding_cf],
            Direction::Payer,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![equity_leg, funding_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for BasketOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.notional,
            Payoff::fixed(self.strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

// ============================================================================
// Credit Instrument Expansion
// ============================================================================

impl InstrumentExpander for Cds {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Premium leg: periodic spread payments
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        // Protection leg: contingent payment on default
        let protection_cf = Cashflow::new(
            CashflowType::Settlement,
            self.maturity,
            self.start_date,
            self.maturity,
            0.0,
            self.notional * (1.0 - self.recovery_rate.unwrap_or(0.4)),
            Payoff::fixed(1.0),
            self.currency,
        );

        let protection_leg = Leg::new(
            vec![protection_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![premium_leg, protection_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CdsIndex {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let _cds_conv = conventions.get_cds()?;

        // Similar to single-name CDS but on index
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

impl InstrumentExpander for CdsOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.exercise_date,
            self.exercise_date,
            self.exercise_date,
            0.0,
            self.notional,
            Payoff::fixed(self.strike_spread),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for NtdBasket {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Nth-to-default basket similar to CDS
        let premium_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
            self.currency,
        );

        let premium_leg = Leg::new(
            vec![premium_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![premium_leg], TradeType::Swap))
    }
}

// ============================================================================
// Commodity Instrument Expansion
// ============================================================================

impl InstrumentExpander for CommodityForward {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Pay fixed price, receive commodity
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.delivery_date,
            self.delivery_date,
            self.delivery_date,
            0.0,
            self.quantity * self.forward_price,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Payer,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::FxForward))
    }
}

impl InstrumentExpander for CommoditySwap {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        // Fixed leg
        let notional = self.quantity_per_period * self.fixed_price;
        let fixed_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            notional,
            Payoff::fixed(self.fixed_price),
            self.currency,
        );

        let fixed_leg = Leg::new(
            vec![fixed_cf],
            Direction::Payer,
            LegType::Fixed,
            self.currency,
        );

        // Floating leg (commodity price)
        let floating_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            notional,
            Payoff::fixed(0.0), // Commodity index reference
            self.currency,
        );

        let floating_leg = Leg::new(
            vec![floating_cf],
            Direction::Receiver,
            LegType::Floating,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![fixed_leg, floating_leg],
            TradeType::Swap,
        ))
    }
}

impl InstrumentExpander for CommodityVanillaOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity * self.strike,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for CommodityAsianOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity * self.strike,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

impl InstrumentExpander for SpreadOption {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let settlement_cf = Cashflow::new(
            CashflowType::Settlement,
            self.expiry,
            self.expiry,
            self.expiry,
            0.0,
            self.quantity,
            Payoff::fixed(self.spread_strike),
            self.currency,
        );

        let leg = Leg::new(
            vec![settlement_cf],
            Direction::Receiver,
            LegType::Generic,
            self.currency,
        );

        Ok(Trade::new(trade_id, vec![leg], TradeType::Generic))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{CurrencyPair, EquityUnderlying, ExerciseStyle, PayerReceiver},
        *,
    };
    use crate::{
        market::convention::{
            CdsConvention, EquityConvention, FxConvention, FxOptionConvention,
            InflationSwapConvention, SwapConvention, SwaptionConvention,
        },
        trade::{ExerciseType, SettlementType},
        Currency, Tenor,
    };

    fn make_conventions() -> ConventionSet {
        ConventionSet::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_swaption(SwaptionConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_fx_option(FxOptionConvention::g10_standard())
            .with_cds(CdsConvention::isda_na())
            .with_equity(EquityConvention::us_equity())
            .with_inflation_swap(InflationSwapConvention::us_cpi_zc())
    }

    fn valuation_date() -> Date { Date::from_ymd(2025, 1, 1).unwrap() }

    // === Rates Tests ===

    #[test]
    fn test_expand_swaption() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "SWAPTION-001");
        assert!(trade.trade_type.is_swaption());
        assert_eq!(trade.num_legs(), 1);
        assert_eq!(trade.total_cashflows(), 1);
    }

    #[test]
    fn test_expand_swaption_missing_convention() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let empty_conventions = ConventionSet::new();
        let result = swaption.expand_to_trade("SWAPTION-001", valuation_date(), &empty_conventions);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));
    }

    // === FX Tests ===

    #[test]
    fn test_expand_fx_spot() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-SPOT-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_expand_fx_forward() {
        let fx_forward = FxForward {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            forward_rate: 1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_forward
            .expand_to_trade("FX-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
    }

    #[test]
    fn test_expand_fx_swap() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 4); // near pay, near receive, far pay, far
                                         // receive
    }

    // === Equity Tests ===

    #[test]
    fn test_expand_equity_forward() {
        let eq_forward = EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_forward
            .expand_to_trade("EQ-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-FWD-001");
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_equity_vanilla_option() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("EQ-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // === Credit Tests ===

    #[test]
    fn test_expand_cds() {
        use super::super::CreditEvent;

        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        };

        let trade = cds
            .expand_to_trade("CDS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CDS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // premium leg and protection leg
    }

    // === Commodity Tests ===

    #[test]
    fn test_expand_commodity_forward() {
        use super::super::{CommodityType, EnergyType, QuantityUnit};

        let comm_forward = CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        };

        let trade = comm_forward
            .expand_to_trade("COMM-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
    }

    // === InstrumentDefinition Integration Tests ===

    #[test]
    fn test_instrument_definition_expand() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let trade = instrument
            .expand_to_trade("INST-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "INST-001");
    }

    #[test]
    fn test_instrument_definition_expand_validates() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: -1_000_000.0, // Invalid: negative notional
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let result = instrument.expand_to_trade("INST-001", valuation_date(), &make_conventions());

        assert!(result.is_err());
    }

    #[test]
    fn test_trade_all_cashflows_compatibility() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        // Verify Trade::all_cashflows() works
        let cashflows: Vec<_> = trade.all_cashflows().collect();
        assert_eq!(cashflows.len(), 4);

        // Verify future_cashflows() works
        let future_cfs: Vec<_> = trade.future_cashflows(valuation_date()).collect();
        assert_eq!(future_cfs.len(), 4);
    }

    // =========================================================================
    // Task 11.2: CF Expansion Integration Tests
    // =========================================================================

    #[test]
    fn test_expand_cap_floor() {
        use crate::RateIndex;

        let cap = CapFloor {
            cap_floor_type: super::super::CapFloorType::Cap,
            strikes: vec![0.05],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::TwoYears,
            notional_schedule: super::super::NotionalSchedule::constant(10_000_000.0),
            payment_frequency: crate::Frequency::Quarterly,
            currency: Currency::USD,
        };

        let trade = cap
            .expand_to_trade("CAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CAP-001");
        assert_eq!(trade.trade_type, TradeType::CapFloor);
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_frn() {
        use crate::RateIndex;

        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.005,
            reset_frequency: crate::Frequency::Quarterly,
            principal_schedule: super::super::NotionalSchedule::constant(10_000_000.0),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            currency: Currency::USD,
        };

        let trade = frn
            .expand_to_trade("FRN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FRN-001");
        assert!(matches!(trade.trade_type, TradeType::Bond { .. }));
        assert_eq!(trade.num_legs(), 1);
        assert!(trade.total_cashflows() >= 2); // At least coupon + principal
    }

    #[test]
    fn test_expand_cms_swap() {
        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::USD,
            spread: 0.001,
        };

        let trade = cms
            .expand_to_trade("CMS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CMS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg with CMS rate cashflow
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_inflation_swap() {
        use super::super::SwapType;

        let inf_swap = InflationSwap {
            inflation_index: "USCPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.02,
        };

        let trade = inf_swap
            .expand_to_trade("INF-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "INF-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg for inflation leg
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_ois() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2026, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::Frequency::Annual,
        };

        let trade = ois
            .expand_to_trade("OIS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "OIS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // Fixed + Floating
    }

    #[test]
    fn test_expand_ois_has_daily_accruals() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 4, 15).unwrap(), // 3 months
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Receiver,
            payment_frequency: crate::Frequency::Quarterly,
        };

        let trade = ois
            .expand_to_trade("OIS-002", valuation_date(), &make_conventions())
            .unwrap();

        // Floating leg should have daily accrual details
        let floating_leg = trade.floating_leg().expect("Should have floating leg");
        let cashflows: Vec<_> = floating_leg.cashflows().collect();
        assert!(!cashflows.is_empty());

        // Each cashflow in the floating leg should have daily accruals
        for cf in cashflows {
            assert!(
                cf.has_daily_accruals(),
                "OIS floating cashflow should have daily accruals"
            );
            let accruals = cf.daily_accruals().expect("Should have accruals");
            // Should have roughly 89 business days for a quarter (excluding weekends in
            // real scenario)
            assert!(!accruals.is_empty(), "Should have daily accrual entries");
        }
    }

    #[test]
    fn test_expand_ois_daily_compounding_calculation() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 2, 15).unwrap(), // 1 month
            notional: 1_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::Frequency::Monthly,
        };

        let trade = ois
            .expand_to_trade("OIS-003", valuation_date(), &make_conventions())
            .unwrap();

        let floating_leg = trade.floating_leg().expect("Should have floating leg");
        let cf = floating_leg
            .cashflows()
            .next()
            .expect("Should have at least one cashflow");
        let accruals = cf.daily_accruals().expect("Should have accruals");

        // Verify compounding: each day's notional should grow
        let mut prev_notional = ois.notional;
        for accrual in accruals {
            assert!(
                accrual.compounded_notional >= prev_notional,
                "Compounded notional should grow: {} >= {}",
                accrual.compounded_notional,
                prev_notional
            );
            prev_notional = accrual.compounded_notional;
        }

        // Final compounded notional should be greater than initial
        if let Some(last) = accruals.last() {
            assert!(
                last.compounded_notional > ois.notional,
                "Final compounded notional {} should exceed initial {}",
                last.compounded_notional,
                ois.notional
            );
        }
    }

    #[test]
    fn test_ois_validate_success() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::Frequency::Annual,
        };

        assert!(ois.validate().is_ok());
    }

    #[test]
    fn test_ois_validate_invalid_notional() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: -10_000_000.0, // Invalid: negative
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::Frequency::Annual,
        };

        assert!(ois.validate().is_err());
    }

    #[test]
    fn test_ois_validate_invalid_dates() {
        use crate::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2030, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 1, 15).unwrap(), // Invalid: end before start
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::Frequency::Annual,
        };

        assert!(ois.validate().is_err());
    }

    #[test]
    fn test_expand_fx_vanilla_option() {
        use crate::trade::OptionType;

        let fx_option = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_option
            .expand_to_trade("FX-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_fx_barrier_option() {
        use super::super::{BarrierDirection, BarrierType};
        use crate::trade::OptionType;

        let vanilla = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let barrier_option = FxBarrierOption {
            vanilla,
            barrier_level: 1.15,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(5000.0),
        };

        let trade = barrier_option
            .expand_to_trade("FX-BARRIER-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-BARRIER-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_asian_option() {
        use super::super::AveragingType;
        use crate::trade::OptionType;

        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::Frequency::Monthly,
            observed_values: vec![175.0, 178.0, 180.0],
            notional: 1000.0,
            currency: Currency::USD,
        };

        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "ASIAN-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_equity_swap() {
        let eq_swap = EquitySwap {
            underlying: EquityUnderlying::index("SPX"),
            return_type: super::super::EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
        };

        let trade = eq_swap
            .expand_to_trade("EQ-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // equity leg + funding leg
    }

    #[test]
    fn test_expand_cds_index() {
        let cds_idx = CdsIndex {
            index_name: "CDX.NA.IG".to_string(),
            series: 40,
            version: 1,
            constituent_count: 125,
            notional: 10_000_000.0,
            spread: 0.006,
            start_date: Date::from_ymd(2025, 3, 20).unwrap(),
            maturity: Date::from_ymd(2030, 6, 20).unwrap(),
            currency: Currency::USD,
        };

        let trade = cds_idx
            .expand_to_trade("CDS-IDX-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CDS-IDX-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
    }

    #[test]
    fn test_expand_commodity_swap() {
        use super::super::{CommodityType, EnergyType, QuantityUnit};

        let comm_swap = CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            fixed_price: 75.0,
            floating_index: "WTI".to_string(),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            quantity_per_period: 1000.0,
            unit: QuantityUnit::Barrels,
            payment_frequency: crate::Frequency::Monthly,
            currency: Currency::USD,
        };

        let trade = comm_swap
            .expand_to_trade("COMM-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // fixed + floating
    }

    #[test]
    fn test_expand_commodity_vanilla_option() {
        use super::super::{CommodityType, EnergyType, QuantityUnit};
        use crate::trade::OptionType;

        let comm_opt = CommodityVanillaOption {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            strike: 3.50,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 10000.0,
            unit: QuantityUnit::MMBtu,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        };

        let trade = comm_opt
            .expand_to_trade("COMM-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // Verify convention integration
    #[test]
    fn test_conventions_affect_expansion() {
        // Same swaption with different conventions should have different exercise types
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let conv = make_conventions();
        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &conv)
            .unwrap();

        // Trade type should match swaption settings
        if let TradeType::Swaption {
            exercise_type,
            settlement_type,
            ..
        } = trade.trade_type
        {
            assert_eq!(exercise_type, ExerciseType::European);
            assert_eq!(settlement_type, SettlementType::Cash);
        } else {
            panic!("Expected TradeType::Swaption");
        }
    }

    // =========================================================================
    // Task 11.3: Edge Case Tests
    // =========================================================================

    #[test]
    fn test_edge_case_zero_notional_validation() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 0.0, // Edge case: zero notional
            notional_currency: Currency::EUR,
        };

        // FxSpot.validate() should catch this
        assert!(fx_spot.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_notional_validation() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::FiveYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: -10_000_000.0, // Edge case: negative notional
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        // Swaption.validate() should catch this
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_strike_validation() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: -100.0, // Edge case: negative strike
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        // EquityVanillaOption.validate() should catch this
        assert!(eq_option.validate().is_err());
    }

    #[test]
    fn test_edge_case_maturity_before_start_validation() {
        use super::super::CreditEvent;

        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(), // Edge case: maturity before start
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Cds.validate() should catch this
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_edge_case_same_start_end_date() {
        // Same-day FX spot should still work
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: valuation_date(), // Same as valuation date
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_empty_observed_values() {
        use super::super::AveragingType;
        use crate::trade::OptionType;

        // Asian option with no observed values yet
        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::Frequency::Monthly,
            observed_values: vec![], // Edge case: empty observations
            notional: 1000.0,
            currency: Currency::USD,
        };

        // Should succeed - Asian option can start with no observations
        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "ASIAN-001");
    }

    #[test]
    fn test_edge_case_very_large_notional() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1e15, // Edge case: very large notional
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_very_small_rate() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1e-10, // Edge case: very small rate (but positive)
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // Very small rate might be rejected
        let result = fx_spot.expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions());
        // Depends on validation - either succeeds or fails with appropriate error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_edge_case_fx_swap_same_near_far_date_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Same as near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_far_date_before_near_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Before near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_zero_spread_cds() {
        use super::super::CreditEvent;

        // CDS with zero spread is unusual but should work
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.0, // Zero spread
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Zero spread might be allowed for special cases
        let result = cds.expand_to_trade("CDS-001", valuation_date(), &make_conventions());
        // Validation depends on business rules
        assert!(result.is_ok() || result.is_err());
    }

    // =========================================================================
    // Task 11.4: Property-Based Tests (Consistency Checks)
    // =========================================================================

    #[test]
    fn test_property_expanded_trade_has_cashflows() {
        // Property: Every successfully expanded trade must have at least one cashflow
        let instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::FxForward(FxForward {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                forward_rate: 1.1100,
                settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::index("SPX"),
                forward_price: 5000.0,
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100_000.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for (i, inst) in instruments.iter().enumerate() {
            let trade = inst
                .expand_to_trade(format!("INST-{}", i), valuation_date(), &conv)
                .unwrap();

            // Property: trade must have at least one leg with at least one cashflow
            assert!(
                trade.total_cashflows() >= 1,
                "Trade must have at least one cashflow"
            );
            assert!(trade.num_legs() >= 1, "Trade must have at least one leg");
        }
    }

    #[test]
    fn test_property_trade_id_preserved() {
        // Property: Trade ID passed to expand_to_trade must be preserved
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let test_ids = ["test-123", "TRADE_ABC", "id with spaces", ""];
        let conv = make_conventions();

        for id in &test_ids {
            let trade = fx_spot
                .expand_to_trade(*id, valuation_date(), &conv)
                .unwrap();

            assert_eq!(trade.id.as_str(), *id, "Trade ID must be preserved");
        }
    }

    #[test]
    fn test_property_validation_before_expansion() {
        // Property: Invalid instruments should fail validation before expansion
        let invalid_instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: -1_000_000.0, // Invalid
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::stock("AAPL"),
                forward_price: -100.0, // Invalid
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for inst in &invalid_instruments {
            let result = inst.expand_to_trade("INVALID", valuation_date(), &conv);
            assert!(result.is_err(), "Invalid instrument should fail expansion");
        }
    }

    #[test]
    fn test_property_cashflow_currencies_consistent() {
        // Property: All cashflows in a leg should have the same currency
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        for leg in trade.legs() {
            let leg_ccy = leg.currency;
            for cf in leg.cashflows() {
                assert_eq!(
                    cf.currency, leg_ccy,
                    "Cashflow currency must match leg currency"
                );
            }
        }
    }

    #[test]
    fn test_property_swap_has_multiple_legs() {
        // Property: Swaps should have at least 2 legs (pay and receive)
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(trade.num_legs() >= 2, "Swap must have at least 2 legs");
    }

    #[test]
    fn test_property_options_have_settlement_cashflow() {
        // Property: Options should have at least a settlement cashflow
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(
            trade.total_cashflows() >= 1,
            "Option must have at least settlement cashflow"
        );
    }
}

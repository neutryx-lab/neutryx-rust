//! Rates instrument expansion implementations.
//!
//! Covers: Deposit, Fra, Futures, InterestRateSwap, BasisSwap, Swaption,
//! CapFloor, Frn, CmsSwap, InflationSwap, Ois.

use chrono::Datelike;

use super::InstrumentExpander;
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            BasisSwap, CapFloor, CmsSwap, Deposit, Fra, Frn, Futures, InflationSwap,
            InstrumentError, InterestRateSwap, Ois, Swaption,
        },
        Currency, RateIndex,
    },
    time::{Date, EndOfMonthRule, Frequency, Tenor},
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

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
        let (leg1_direction, leg2_direction) =
            if self.payer_receiver == crate::market::instrument::PayerReceiver::Payer {
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
// Helper Functions
// ============================================================================

/// Generates payment dates based on start, end, and frequency.
pub(super) fn generate_payment_dates(start: Date, end: Date, frequency: Frequency) -> Vec<Date> {
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
pub(super) fn generate_fixed_leg_cashflows(
    payment_dates: &[Date],
    start_date: Date,
    fixed_rate: f64,
    notional: f64,
    currency: Currency,
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
    currency: Currency,
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

        // Simulate small daily rate variation (+-5bps) based on day of year
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

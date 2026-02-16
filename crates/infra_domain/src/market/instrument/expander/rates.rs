//! Rates instrument expansion implementations.

use chrono::Datelike;

use super::InstrumentExpander;
use crate::{
    ids::TradeId,
    market::{
        convention::ConventionSet,
        instrument::{
            BasisSwap, Bond, CapFloor, CmsSwap, Deposit, Fra, Frn, Futures, InflationSwap,
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
        let end_date = self.end_date();

        let fixed_dates = generate_payment_dates(self.start_date, end_date, self.fixed_frequency);

        let float_dates = generate_payment_dates(self.start_date, end_date, self.float_frequency);

        let fixed_cashflows = generate_fixed_leg_cashflows(
            &fixed_dates,
            self.start_date,
            self.fixed_rate,
            self.notional,
            self.currency,
        );

        let floating_cashflows = super::generate_floating_leg_cashflows(
            &float_dates,
            self.rate_index,
            self.notional,
            self.currency,
        );

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
        let end_date = self.end_date();

        let leg1_dates = generate_payment_dates(self.start_date, end_date, self.leg1_frequency);

        let leg2_dates = generate_payment_dates(self.start_date, end_date, self.leg2_frequency);

        let leg1_cashflows = super::generate_floating_leg_cashflows(
            &leg1_dates,
            self.leg1_index,
            self.notional,
            self.currency,
        );

        let leg2_cashflows = super::generate_floating_leg_cashflows(
            &leg2_dates,
            self.leg2_index,
            self.notional,
            self.currency,
        );

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
        let mut cashflows = Vec::new();

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

impl InstrumentExpander for Bond {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        _valuation_date: Date,
        _conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        let coupon_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.notional,
            Payoff::fixed(self.coupon_rate),
            self.currency,
        );

        let principal_cf = Cashflow::new(
            CashflowType::Principal,
            self.maturity,
            self.maturity,
            self.maturity,
            0.0,
            self.notional,
            Payoff::fixed(1.0),
            self.currency,
        );

        let leg = Leg::new(
            vec![coupon_cf, principal_cf],
            Direction::Receiver,
            LegType::Fixed,
            self.currency,
        );

        Ok(Trade::new(
            trade_id,
            vec![leg],
            TradeType::Bond {
                issuer_id: Some(self.issuer.clone().into()),
                seniority: None,
            },
        ))
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

        let coupon_cf = Cashflow::new(
            CashflowType::Coupon,
            self.maturity,
            self.start_date,
            self.maturity,
            1.0,
            self.principal_schedule.notional_at(0),
            Payoff::floating(IndexType::Rate(self.coupon_index)),
            self.currency,
        );

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

        let cms_cf = Cashflow::new(
            CashflowType::Coupon,
            self.start_date,
            self.start_date,
            self.start_date,
            1.0,
            self.notional,
            Payoff::fixed(self.spread),
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
        let payment_dates =
            generate_payment_dates(self.start_date, self.end_date, self.payment_frequency);

        let fixed_cashflows = generate_fixed_leg_cashflows(
            &payment_dates,
            self.start_date,
            self.fixed_rate,
            self.notional,
            self.currency,
        );

        let floating_cashflows = generate_ois_floating_leg_cashflows(
            &payment_dates,
            self.start_date,
            self.rate_index,
            self.notional,
            self.currency,
        );

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

/// Generates payment dates based on start, end, and frequency.
pub(super) fn generate_payment_dates(start: Date, end: Date, frequency: Frequency) -> Vec<Date> {
    let mut dates = Vec::new();

    let tenor = match frequency {
        Frequency::Daily => {
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

        let daily_accruals =
            generate_daily_accruals(accrual_start, accrual_end, rate_index, notional);

        let days = (accrual_end - accrual_start) as f64;
        let year_fraction = days / 360.0;

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

    let base_rate = match rate_index {
        RateIndex::Sofr => 0.0430,
        RateIndex::Estr => 0.0390,
        RateIndex::Euribor3M => 0.0390,
        RateIndex::Euribor6M => 0.0395,
        RateIndex::Sonia => 0.0525,
        RateIndex::Tonar => 0.0010,
        RateIndex::Saron => 0.0175,
    };

    let day_count_basis = match rate_index {
        RateIndex::Sonia | RateIndex::Tonar => 365.0,
        _ => 360.0,
    };

    while current_date < end {
        let weekday = current_date.into_inner().weekday();

        if weekday == Weekday::Sat || weekday == Weekday::Sun {
            current_date = Tenor::Overnight.add_to_date(current_date, EndOfMonthRule::None);
            continue;
        }

        let days_to_next = if weekday == Weekday::Fri { 3.0 } else { 1.0 };
        let day_fraction = days_to_next / day_count_basis;

        let day_of_year = current_date.into_inner().ordinal() as f64;
        let rate_variation = (day_of_year.sin() * 0.0005).abs();
        let overnight_rate = base_rate + rate_variation;

        let new_compounded = compounded_notional * (1.0 + overnight_rate * day_fraction);

        accruals.push(DailyAccrual::with_compounded_notional(
            current_date,
            overnight_rate,
            day_fraction,
            new_compounded,
        ));

        compounded_notional = new_compounded;

        current_date = Tenor::Overnight.add_to_date(current_date, EndOfMonthRule::None);
    }

    accruals
}

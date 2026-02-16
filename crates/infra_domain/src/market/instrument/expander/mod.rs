//! Instrument expansion to Trade (cashflow generation).

mod commodity;
mod credit;
mod equity;
mod fx;
pub(crate) mod rates;
#[cfg(test)]
mod tests;

use super::{InstrumentDefinition, InstrumentError};
use crate::{
    ids::TradeId,
    market::{convention::ConventionSet, Currency, CurrencyPair},
    time::Date,
    trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, Trade, TradeType},
};

/// Trait for expanding instrument definitions into trades with cashflows.
pub trait InstrumentExpander {
    /// Expands this instrument into a Trade with cashflows.
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError>;
}

/// Creates a single-leg settlement trade (options, forwards, etc.).
pub(super) fn settlement_trade(
    trade_id: impl Into<TradeId>,
    date: Date,
    notional: f64,
    payoff_value: f64,
    currency: Currency,
    direction: Direction,
    trade_type: TradeType,
) -> Trade {
    let cf = Cashflow::new(
        CashflowType::Settlement,
        date,
        date,
        date,
        0.0,
        notional,
        Payoff::fixed(payoff_value),
        currency,
    );
    let leg = Leg::new(vec![cf], direction, LegType::Generic, currency);
    Trade::new(trade_id, vec![leg], trade_type)
}

/// Creates a two-leg FX exchange trade (FX spot, FX forward).
pub(super) fn fx_exchange_trade(
    trade_id: impl Into<TradeId>,
    settlement_date: Date,
    notional: f64,
    rate: f64,
    notional_currency: Currency,
    currency_pair: &CurrencyPair,
    trade_type: TradeType,
) -> Trade {
    let (receive_amount, receive_currency) = if notional_currency == currency_pair.base {
        (notional * rate, currency_pair.quote)
    } else {
        (notional / rate, currency_pair.base)
    };

    let pay_cf = Cashflow::new(
        CashflowType::Principal,
        settlement_date,
        settlement_date,
        settlement_date,
        0.0,
        notional,
        Payoff::fixed(1.0),
        notional_currency,
    );
    let receive_cf = Cashflow::new(
        CashflowType::Principal,
        settlement_date,
        settlement_date,
        settlement_date,
        0.0,
        receive_amount,
        Payoff::fixed(1.0),
        receive_currency,
    );

    let pay_leg = Leg::new(
        vec![pay_cf],
        Direction::Payer,
        LegType::Principal,
        notional_currency,
    );
    let receive_leg = Leg::new(
        vec![receive_cf],
        Direction::Receiver,
        LegType::Principal,
        receive_currency,
    );
    Trade::new(trade_id, vec![pay_leg, receive_leg], trade_type)
}

/// Creates a two-leg coupon swap trade (CDS, commodity swap, equity swap).
pub(super) fn coupon_swap_trade(
    trade_id: impl Into<TradeId>,
    start_date: Date,
    maturity: Date,
    notional: f64,
    fixed_payoff: f64,
    floating_payoff: f64,
    currency: Currency,
    fixed_leg_type: LegType,
    floating_leg_type: LegType,
) -> Trade {
    let fixed_cf = Cashflow::new(
        CashflowType::Coupon,
        maturity,
        start_date,
        maturity,
        1.0,
        notional,
        Payoff::fixed(fixed_payoff),
        currency,
    );
    let floating_cf = Cashflow::new(
        CashflowType::Coupon,
        maturity,
        start_date,
        maturity,
        1.0,
        notional,
        Payoff::fixed(floating_payoff),
        currency,
    );

    let fixed_leg = Leg::new(vec![fixed_cf], Direction::Payer, fixed_leg_type, currency);
    let floating_leg = Leg::new(
        vec![floating_cf],
        Direction::Receiver,
        floating_leg_type,
        currency,
    );
    Trade::new(trade_id, vec![fixed_leg, floating_leg], TradeType::Swap)
}

/// Generates floating-rate leg cashflows from payment dates.
///
/// Shared by `InterestRateSwap`, `BasisSwap`, `CrossCurrencyBasisSwap`, etc.
pub(super) fn generate_floating_leg_cashflows(
    payment_dates: &[Date],
    rate_index: crate::market::RateIndex,
    notional: f64,
    currency: Currency,
) -> Vec<Cashflow> {
    use crate::trade::IndexType;

    (0..payment_dates.len().saturating_sub(1))
        .map(|i| {
            let (start, end) = (payment_dates[i], payment_dates[i + 1]);
            let year_fraction = (end - start) as f64 / 360.0;
            Cashflow::new(
                CashflowType::Coupon,
                end,
                start,
                end,
                year_fraction,
                notional,
                Payoff::floating(IndexType::Rate(rate_index)),
                currency,
            )
        })
        .collect()
}

/// Creates a CDS-style premium leg (single coupon paying a spread).
///
/// Shared by `Cds`, `CdsIndex`, `NtdBasket`.
pub(super) fn credit_premium_leg(
    start_date: Date,
    maturity: Date,
    notional: f64,
    spread: f64,
    currency: Currency,
) -> Leg {
    let premium_cf = Cashflow::new(
        CashflowType::Coupon,
        maturity,
        start_date,
        maturity,
        1.0,
        notional,
        Payoff::fixed(spread),
        currency,
    );
    Leg::new(vec![premium_cf], Direction::Payer, LegType::Fixed, currency)
}

macro_rules! dispatch_expand {
    ($self:expr, $tid:expr, $vd:expr, $conv:expr; $($Variant:ident),+ $(,)?) => {
        match $self {
            $(InstrumentDefinition::$Variant(inner) => inner.expand_to_trade($tid, $vd, $conv),)+
            other => Err(InstrumentError::invalid_parameter(
                format!("Instrument expansion not yet supported for {other}"),
            )),
        }
    };
}

impl InstrumentExpander for InstrumentDefinition {
    fn expand_to_trade(
        &self,
        trade_id: impl Into<TradeId>,
        valuation_date: Date,
        conventions: &ConventionSet,
    ) -> Result<Trade, InstrumentError> {
        self.validate()?;
        dispatch_expand!(self, trade_id, valuation_date, conventions;
            Deposit, Fra, Futures, InterestRateSwap, BasisSwap, Ois,
            Swaption, CapFloor, Bond, Frn, CmsSwap, InflationSwap,
            FxSpot, FxForward, FxVanillaOption, FxBarrierOption, FxSwap, CrossCurrencyBasisSwap,
            EquityForward, EquityVanillaOption, EquityBarrierOption, AsianOption, LookbackOption, EquitySwap, BasketOption,
            Cds, CdsIndex, CdsOption, NtdBasket,
            CommodityForward, CommoditySwap, CommodityVanillaOption, CommodityAsianOption, SpreadOption,
        )
    }
}

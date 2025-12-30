//! Trade structure with instrument reference.
//!
//! This module provides the Trade structure that wraps instruments
//! with metadata for portfolio management.

use pricer_core::types::Currency;
use pricer_models::instruments::{Instrument, PayoffType};

use super::ids::{CounterpartyId, NettingSetId, TradeId};

/// Trade with instrument and metadata.
///
/// A trade represents a single financial instrument with associated
/// metadata for portfolio management and XVA calculations.
///
/// # Examples
///
/// ```
/// use pricer_xva::portfolio::{Trade, TradeId, CounterpartyId, NettingSetId};
/// use pricer_core::types::Currency;
/// use pricer_models::instruments::{
///     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
/// };
///
/// let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let instrument = Instrument::Vanilla(call);
///
/// let trade = Trade::new(
///     TradeId::new("T001"),
///     instrument,
///     Currency::USD,
///     CounterpartyId::new("CP001"),
///     NettingSetId::new("NS001"),
///     1_000_000.0,
/// );
///
/// assert_eq!(trade.id().as_str(), "T001");
/// assert_eq!(trade.notional(), 1_000_000.0);
/// ```
#[derive(Clone, Debug)]
pub struct Trade {
    id: TradeId,
    instrument: Instrument<f64>,
    currency: Currency,
    counterparty_id: CounterpartyId,
    netting_set_id: NettingSetId,
    notional: f64,
}

impl Trade {
    /// Creates a new trade.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique trade identifier
    /// * `instrument` - Underlying financial instrument
    /// * `currency` - Trade currency
    /// * `counterparty_id` - Counterparty identifier
    /// * `netting_set_id` - Netting set identifier
    /// * `notional` - Notional amount
    #[inline]
    pub fn new(
        id: TradeId,
        instrument: Instrument<f64>,
        currency: Currency,
        counterparty_id: CounterpartyId,
        netting_set_id: NettingSetId,
        notional: f64,
    ) -> Self {
        Self {
            id,
            instrument,
            currency,
            counterparty_id,
            netting_set_id,
            notional,
        }
    }

    /// Returns the trade ID.
    #[inline]
    pub fn id(&self) -> &TradeId {
        &self.id
    }

    /// Returns a reference to the underlying instrument.
    #[inline]
    pub fn instrument(&self) -> &Instrument<f64> {
        &self.instrument
    }

    /// Returns the trade currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the counterparty ID.
    #[inline]
    pub fn counterparty_id(&self) -> &CounterpartyId {
        &self.counterparty_id
    }

    /// Returns the netting set ID.
    #[inline]
    pub fn netting_set_id(&self) -> &NettingSetId {
        &self.netting_set_id
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> f64 {
        self.notional
    }

    /// Computes the payoff at given spot price.
    ///
    /// The payoff is scaled by the notional amount.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price
    ///
    /// # Returns
    ///
    /// Notional-scaled payoff value.
    #[inline]
    pub fn payoff(&self, spot: f64) -> f64 {
        self.instrument.payoff(spot) * self.notional
    }

    /// Returns the instrument expiry in years.
    #[inline]
    pub fn expiry(&self) -> f64 {
        self.instrument.expiry()
    }

    /// Returns whether this is a vanilla option trade.
    #[inline]
    pub fn is_vanilla(&self) -> bool {
        self.instrument.is_vanilla()
    }

    /// Returns whether this is a forward trade.
    #[inline]
    pub fn is_forward(&self) -> bool {
        self.instrument.is_forward()
    }

    /// Returns whether this is a swap trade.
    #[inline]
    pub fn is_swap(&self) -> bool {
        self.instrument.is_swap()
    }

    /// Returns the strike price if this is a vanilla option.
    pub fn strike(&self) -> Option<f64> {
        self.instrument.as_vanilla().map(|v| v.params().strike())
    }

    /// Returns the payoff type if this is a vanilla option.
    pub fn payoff_type(&self) -> Option<PayoffType> {
        self.instrument.as_vanilla().map(|v| v.payoff_type())
    }
}

/// Builder for creating trades with optional fields.
#[derive(Debug)]
pub struct TradeBuilder {
    id: Option<TradeId>,
    instrument: Option<Instrument<f64>>,
    currency: Option<Currency>,
    counterparty_id: Option<CounterpartyId>,
    netting_set_id: Option<NettingSetId>,
    notional: Option<f64>,
}

impl Default for TradeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeBuilder {
    /// Creates a new trade builder.
    pub fn new() -> Self {
        Self {
            id: None,
            instrument: None,
            currency: None,
            counterparty_id: None,
            netting_set_id: None,
            notional: None,
        }
    }

    /// Sets the trade ID.
    pub fn id(mut self, id: impl Into<TradeId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the instrument.
    pub fn instrument(mut self, instrument: Instrument<f64>) -> Self {
        self.instrument = Some(instrument);
        self
    }

    /// Sets the currency.
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }

    /// Sets the counterparty ID.
    pub fn counterparty_id(mut self, id: impl Into<CounterpartyId>) -> Self {
        self.counterparty_id = Some(id.into());
        self
    }

    /// Sets the netting set ID.
    pub fn netting_set_id(mut self, id: impl Into<NettingSetId>) -> Self {
        self.netting_set_id = Some(id.into());
        self
    }

    /// Sets the notional amount.
    pub fn notional(mut self, notional: f64) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Builds the trade.
    ///
    /// # Panics
    ///
    /// Panics if any required field is not set.
    pub fn build(self) -> Trade {
        Trade::new(
            self.id.expect("Trade ID is required"),
            self.instrument.expect("Instrument is required"),
            self.currency.expect("Currency is required"),
            self.counterparty_id.expect("Counterparty ID is required"),
            self.netting_set_id.expect("Netting set ID is required"),
            self.notional.expect("Notional is required"),
        )
    }

    /// Tries to build the trade, returning None if any required field is missing.
    pub fn try_build(self) -> Option<Trade> {
        Some(Trade::new(
            self.id?,
            self.instrument?,
            self.currency?,
            self.counterparty_id?,
            self.netting_set_id?,
            self.notional?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use pricer_models::instruments::{
        Direction, ExerciseStyle, Forward, InstrumentParams, VanillaOption,
    };

    fn create_test_call() -> Instrument<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
        Instrument::Vanilla(call)
    }

    fn create_test_forward() -> Instrument<f64> {
        let forward = Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap();
        Instrument::Forward(forward)
    }

    #[test]
    fn test_trade_creation() {
        let instrument = create_test_call();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1_000_000.0,
        );

        assert_eq!(trade.id().as_str(), "T001");
        assert_eq!(trade.currency(), Currency::USD);
        assert_eq!(trade.counterparty_id().as_str(), "CP001");
        assert_eq!(trade.netting_set_id().as_str(), "NS001");
        assert_eq!(trade.notional(), 1_000_000.0);
    }

    #[test]
    fn test_trade_payoff() {
        let instrument = create_test_call();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1_000_000.0,
        );

        // ITM call: spot = 110, strike = 100, payoff ≈ 10 * 1_000_000
        let payoff = trade.payoff(110.0);
        assert_relative_eq!(payoff, 10_000_000.0, max_relative = 0.01);

        // OTM call: payoff ≈ 0
        let payoff_otm = trade.payoff(90.0);
        assert!(payoff_otm < 1.0); // Nearly zero due to smoothing
    }

    #[test]
    fn test_trade_expiry() {
        let instrument = create_test_call();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        assert_eq!(trade.expiry(), 1.0);
    }

    #[test]
    fn test_trade_is_vanilla() {
        let vanilla = create_test_call();
        let forward = create_test_forward();

        let trade_vanilla = Trade::new(
            TradeId::new("T001"),
            vanilla,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        let trade_forward = Trade::new(
            TradeId::new("T002"),
            forward,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        assert!(trade_vanilla.is_vanilla());
        assert!(!trade_vanilla.is_forward());

        assert!(!trade_forward.is_vanilla());
        assert!(trade_forward.is_forward());
    }

    #[test]
    fn test_trade_strike() {
        let instrument = create_test_call();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        assert_eq!(trade.strike(), Some(100.0));
    }

    #[test]
    fn test_trade_strike_forward() {
        let instrument = create_test_forward();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        assert_eq!(trade.strike(), None);
    }

    #[test]
    fn test_trade_payoff_type() {
        let instrument = create_test_call();
        let trade = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );

        assert_eq!(trade.payoff_type(), Some(PayoffType::Call));
    }

    #[test]
    fn test_trade_builder() {
        let instrument = create_test_call();
        let trade = TradeBuilder::new()
            .id("T001")
            .instrument(instrument)
            .currency(Currency::USD)
            .counterparty_id("CP001")
            .netting_set_id("NS001")
            .notional(1_000_000.0)
            .build();

        assert_eq!(trade.id().as_str(), "T001");
        assert_eq!(trade.notional(), 1_000_000.0);
    }

    #[test]
    fn test_trade_builder_try_build() {
        let instrument = create_test_call();

        // Complete builder
        let trade = TradeBuilder::new()
            .id("T001")
            .instrument(instrument.clone())
            .currency(Currency::USD)
            .counterparty_id("CP001")
            .netting_set_id("NS001")
            .notional(1.0)
            .try_build();
        assert!(trade.is_some());

        // Incomplete builder
        let trade = TradeBuilder::new()
            .id("T001")
            .instrument(instrument)
            .try_build();
        assert!(trade.is_none());
    }

    #[test]
    fn test_trade_clone() {
        let instrument = create_test_call();
        let trade1 = Trade::new(
            TradeId::new("T001"),
            instrument,
            Currency::USD,
            CounterpartyId::new("CP001"),
            NettingSetId::new("NS001"),
            1.0,
        );
        let trade2 = trade1.clone();

        assert_eq!(trade1.id(), trade2.id());
        assert_eq!(trade1.notional(), trade2.notional());
    }
}

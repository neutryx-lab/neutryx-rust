//! Cross-currency swap compiler for multi-currency trades.
//!
//! This module provides `XCcyCompiler` which compiles Trade structures with
//! multiple currencies into `PricingKernel` IR with proper FX index
//! assignments.
//!
//! # Cross-Currency Support
//!
//! The compiler handles:
//! - Multi-currency trades (e.g., EUR/USD cross-currency swap)
//! - FX conversion for cashflows in non-base currencies
//! - Collateral currency specification for discounting
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::compiler::{XCcyCompiler, IndexMapper};
//! use infra_domain::Currency;
//!
//! let mapper = IndexMapper::with_common_indices();
//! let compiler = XCcyCompiler::new(mapper)
//!     .with_base_currency(Currency::USD);
//!
//! let kernel = compiler.compile(&xccy_swap)?;
//! ```

use std::sync::Arc;

use infra_domain::{
    time::{BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar},
    trade::{IndexType, Payoff, Trade},
    Currency, Date,
};
use pricer_core::{
    kernel::{CompileError, PricingKernel, PricingKernelBuilder},
    types::FxPair,
};

use super::{IndexMapper, TradeCompiler};

/// Compiler for cross-currency products (X-Ccy swaps).
///
/// Transforms multi-currency `Trade` structures into `PricingKernel` IR
/// with proper FX index assignments for currency conversion.
///
/// # Base Currency
///
/// The compiler requires a base currency (reporting currency) to be set.
/// All cashflows in other currencies will be assigned an FX index for
/// conversion to the base currency.
///
/// # Supported Products
///
/// - Cross-currency basis swaps
/// - Cross-currency fixed-float swaps
/// - Multi-currency structured products
///
/// # Example
///
/// ```ignore
/// use pricer_models::compiler::{XCcyCompiler, IndexMapper};
/// use infra_domain::Currency;
///
/// let mapper = IndexMapper::new();
/// let compiler = XCcyCompiler::new(mapper)
///     .with_base_currency(Currency::USD);
///
/// let kernel = compiler.compile(&xccy_swap)?;
/// ```
#[derive(Clone)]
pub struct XCcyCompiler {
    /// Index mapper for ID resolution.
    mapper: IndexMapper,
    /// Reference date for converting dates to days from epoch.
    epoch: Date,
    /// Base (reporting) currency for FX conversion.
    base_currency: Currency,
    /// Optional calendar for business day adjustment.
    calendar: Option<Arc<dyn Calendar>>,
    /// Business day convention for payment dates.
    payment_bdc: BusinessDayConvention,
    /// Business day convention for fixing dates.
    fixing_bdc: BusinessDayConvention,
    /// Collateral currency (for CSA discounting).
    collateral_currency: Option<Currency>,
}

impl std::fmt::Debug for XCcyCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XCcyCompiler")
            .field("mapper", &self.mapper)
            .field("epoch", &self.epoch)
            .field("base_currency", &self.base_currency)
            .field("has_calendar", &self.calendar.is_some())
            .field("payment_bdc", &self.payment_bdc)
            .field("fixing_bdc", &self.fixing_bdc)
            .field("collateral_currency", &self.collateral_currency)
            .finish()
    }
}

impl XCcyCompiler {
    /// Creates a new cross-currency compiler with the given index mapper.
    ///
    /// Uses Unix epoch (1970-01-01) as the reference date.
    /// Default base currency is USD.
    #[must_use]
    pub fn new(mapper: IndexMapper) -> Self {
        let epoch = Date::from_ymd(1970, 1, 1).expect("Unix epoch is valid");
        Self {
            mapper,
            epoch,
            base_currency: Currency::USD,
            calendar: None,
            payment_bdc: BusinessDayConvention::Unadjusted,
            fixing_bdc: BusinessDayConvention::Unadjusted,
            collateral_currency: None,
        }
    }

    /// Creates a compiler with a custom reference date for day counting.
    #[must_use]
    pub fn with_epoch(mapper: IndexMapper, epoch: Date) -> Self {
        Self {
            mapper,
            epoch,
            base_currency: Currency::USD,
            calendar: None,
            payment_bdc: BusinessDayConvention::Unadjusted,
            fixing_bdc: BusinessDayConvention::Unadjusted,
            collateral_currency: None,
        }
    }

    /// Sets the base (reporting) currency for FX conversion.
    ///
    /// Cashflows in currencies other than the base will be converted
    /// using the corresponding FX rate.
    ///
    /// # Arguments
    ///
    /// * `currency` - The base currency (e.g., USD)
    #[must_use]
    pub fn with_base_currency(mut self, currency: Currency) -> Self {
        self.base_currency = currency;
        self
    }

    /// Sets the collateral currency for CSA discounting.
    ///
    /// When set, all cashflows will be discounted using the collateral
    /// currency curve instead of the cashflow currency curve.
    ///
    /// # Arguments
    ///
    /// * `currency` - The collateral currency (e.g., EUR for
    ///   EUR-collateralised)
    #[must_use]
    pub fn with_collateral_currency(mut self, currency: Currency) -> Self {
        self.collateral_currency = Some(currency);
        self
    }

    /// Configures the compiler with a calendar for business day adjustment.
    #[must_use]
    pub fn with_calendar(
        mut self,
        calendar_id: CalendarId,
        convention: BusinessDayConvention,
    ) -> Self {
        self.calendar = Some(Arc::new(ConcreteCalendar::new(calendar_id)));
        self.payment_bdc = convention;
        self.fixing_bdc = convention;
        self
    }

    /// Configures the compiler with a custom calendar implementation.
    #[must_use]
    pub fn with_custom_calendar(
        mut self,
        calendar: Arc<dyn Calendar>,
        convention: BusinessDayConvention,
    ) -> Self {
        self.calendar = Some(calendar);
        self.payment_bdc = convention;
        self.fixing_bdc = convention;
        self
    }

    /// Configures separate conventions for payment and fixing dates.
    #[must_use]
    pub fn with_conventions(
        mut self,
        payment_convention: BusinessDayConvention,
        fixing_convention: BusinessDayConvention,
    ) -> Self {
        self.payment_bdc = payment_convention;
        self.fixing_bdc = fixing_convention;
        self
    }

    /// Returns a reference to the index mapper.
    #[must_use]
    pub fn mapper(&self) -> &IndexMapper { &self.mapper }

    /// Returns a mutable reference to the index mapper.
    pub fn mapper_mut(&mut self) -> &mut IndexMapper { &mut self.mapper }

    /// Returns true if a calendar is configured.
    #[must_use]
    pub fn has_calendar(&self) -> bool { self.calendar.is_some() }

    /// Returns the base (reporting) currency.
    #[must_use]
    pub fn base_currency(&self) -> Currency { self.base_currency }

    /// Returns the collateral currency if set.
    #[must_use]
    pub fn collateral_currency(&self) -> Option<Currency> { self.collateral_currency }

    /// Adjusts a date according to the calendar and convention.
    fn adjust_date(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match &self.calendar {
            Some(cal) => cal.adjust(date, convention),
            None => date,
        }
    }

    /// Converts a Date to days from epoch.
    fn date_to_days(&self, date: Date) -> i32 { (date - self.epoch) as i32 }

    /// Converts a Date to days from epoch with payment date adjustment.
    fn payment_date_to_days(&self, date: Date) -> i32 {
        let adjusted = self.adjust_date(date, self.payment_bdc);
        self.date_to_days(adjusted)
    }

    /// Converts a Date to days from epoch with fixing date adjustment.
    fn fixing_date_to_days(&self, date: Date) -> i32 {
        let adjusted = self.adjust_date(date, self.fixing_bdc);
        self.date_to_days(adjusted)
    }

    /// Gets or registers the FX index for converting from cashflow currency to
    /// base.
    ///
    /// Returns 0 (dummy) if the cashflow currency is the same as base currency.
    fn get_fx_index_id(&mut self, cashflow_currency: Currency) -> u16 {
        if cashflow_currency == self.base_currency {
            return 0; // No FX conversion needed
        }

        // FX pair: cashflow_currency/base_currency
        // This means: 1 unit of cashflow_currency = X units of base_currency
        let fx_pair = FxPair::new(cashflow_currency, self.base_currency);
        self.mapper.get_or_register_fx_pair(fx_pair)
    }

    /// Determines the discount curve to use for a cashflow.
    ///
    /// If collateral currency is set, uses the collateral currency curve.
    /// Otherwise, uses the cashflow currency curve.
    fn get_discount_curve_name(&self, cashflow_currency: Currency) -> String {
        match self.collateral_currency {
            Some(coll) => coll.code().to_string(),
            None => cashflow_currency.code().to_string(),
        }
    }

    /// Extracts gearing and spread from a Payoff.
    fn extract_payoff_params(&mut self, payoff: &Payoff) -> Result<(f64, f64, u16), CompileError> {
        match payoff {
            Payoff::Fixed { rate } => Ok((0.0, *rate, 0)),
            Payoff::Linear {
                index,
                spread,
                multiplier,
            } => {
                let fwd_index_id = match index {
                    IndexType::Rate(rate_index) => {
                        self.mapper.get_or_register_forward_index(*rate_index)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedPayoff(format!(
                            "Non-rate index not supported for X-Ccy: {:?}",
                            index
                        )));
                    }
                };
                Ok((*multiplier, *spread, fwd_index_id))
            }
            Payoff::VanillaOption { .. } | Payoff::Digital { .. } => {
                Err(CompileError::UnsupportedPayoff(
                    "Option payoffs not supported in X-Ccy compiler".to_string(),
                ))
            }
        }
    }
}

impl TradeCompiler<Trade> for XCcyCompiler {
    fn compile(&self, trade: &Trade) -> Result<PricingKernel, CompileError> {
        let mut compiler = self.clone();
        compiler.compile_with_registration(trade)
    }
}

impl XCcyCompiler {
    /// Internal compilation with mutable mapper access.
    fn compile_with_registration(&mut self, trade: &Trade) -> Result<PricingKernel, CompileError> {
        let total_cashflows: usize = trade.legs().map(|leg| leg.len()).sum();

        if total_cashflows == 0 {
            return Err(CompileError::EmptyTrade(trade.id.to_string()));
        }

        let mut builder = PricingKernelBuilder::with_capacity(total_cashflows);

        // Process each leg
        for leg in trade.legs() {
            let cashflow_currency = leg.currency;
            let currency_id = self.mapper.get_or_register_currency(cashflow_currency);

            // Get FX index for currency conversion (0 if same as base)
            let fx_index_id = self.get_fx_index_id(cashflow_currency);

            // Determine discount curve (collateral or cashflow currency)
            let discount_curve_name = self.get_discount_curve_name(cashflow_currency);
            let discount_curve_id = self
                .mapper
                .get_or_register_discount_curve(&discount_curve_name);

            // Get direction sign for notional
            let direction_sign = leg.direction.sign();

            // Process each cashflow in the leg
            for cf in leg.cashflows() {
                // Skip Fee cashflows
                if matches!(cf.cf_type, infra_domain::trade::CashflowType::Fee) {
                    continue;
                }

                let payment_date = self.payment_date_to_days(cf.payment_date);
                let fixing_date = self.fixing_date_to_days(cf.accrual_start);

                let (gearing, spread, fwd_index_id) = self.extract_payoff_params(&cf.payoff)?;

                let notional = cf.notional * direction_sign;

                builder.add_cashflow(
                    payment_date,
                    fixing_date,
                    cf.year_fraction,
                    notional,
                    spread,
                    gearing,
                    currency_id,
                    discount_curve_id,
                    fwd_index_id,
                    fx_index_id,
                );
            }
        }

        builder.increment_trade_count();
        builder.sort_by_payment_date();
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use infra_domain::{
        trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff, TradeType},
        Currency, RateIndex,
    };

    use super::*;

    /// Creates a USD fixed leg.
    fn create_usd_fixed_leg() -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::fixed(0.05),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::fixed(0.05),
                Currency::USD,
            ),
        ];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        )
    }

    /// Creates a EUR floating leg.
    fn create_eur_floating_leg() -> Leg {
        use infra_domain::trade::IndexType;

        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Estr), 0.001),
                Currency::EUR,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Estr), 0.001),
                Currency::EUR,
            ),
        ];

        Leg::new(
            cashflows,
            Direction::Payer,
            LegType::Floating,
            Currency::EUR,
        )
    }

    /// Creates a cross-currency swap (receive USD fixed, pay EUR floating).
    fn create_xccy_swap() -> Trade {
        Trade::new(
            "XCCY001",
            vec![create_usd_fixed_leg(), create_eur_floating_leg()],
            TradeType::Swap,
        )
    }

    #[test]
    fn test_xccy_compiler_new() {
        let mapper = IndexMapper::new();
        let compiler = XCcyCompiler::new(mapper);

        assert_eq!(compiler.base_currency(), Currency::USD);
        assert!(compiler.collateral_currency().is_none());
        assert!(!compiler.has_calendar());
    }

    #[test]
    fn test_xccy_compiler_with_base_currency() {
        let mapper = IndexMapper::new();
        let compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::EUR);

        assert_eq!(compiler.base_currency(), Currency::EUR);
    }

    #[test]
    fn test_xccy_compiler_with_collateral_currency() {
        let mapper = IndexMapper::new();
        let compiler = XCcyCompiler::new(mapper).with_collateral_currency(Currency::EUR);

        assert_eq!(compiler.collateral_currency(), Some(Currency::EUR));
    }

    #[test]
    fn test_compile_xccy_swap() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let xccy_swap = create_xccy_swap();
        let kernel = compiler.compile_with_registration(&xccy_swap).unwrap();

        // Should have 4 cashflows (2 USD + 2 EUR)
        assert_eq!(kernel.len(), 4);
        assert_eq!(kernel.trade_count(), 1);
    }

    #[test]
    fn test_xccy_fx_index_assignment() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let xccy_swap = create_xccy_swap();
        let kernel = compiler.compile_with_registration(&xccy_swap).unwrap();

        // USD cashflows should have fx_index_id = 0 (no conversion)
        // EUR cashflows should have fx_index_id > 0 (EUR/USD conversion)
        let mut has_zero_fx = false;
        let mut has_nonzero_fx = false;

        for i in 0..kernel.len() {
            if kernel.fx_index_ids[i] == 0 {
                has_zero_fx = true;
            } else {
                has_nonzero_fx = true;
            }
        }

        assert!(has_zero_fx, "USD cashflows should have fx_index_id = 0");
        assert!(has_nonzero_fx, "EUR cashflows should have fx_index_id > 0");
    }

    #[test]
    fn test_xccy_currency_registration() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let xccy_swap = create_xccy_swap();
        compiler.compile_with_registration(&xccy_swap).unwrap();

        // Both USD and EUR should be registered
        assert!(compiler.mapper().get_currency_id(Currency::USD).is_some());
        assert!(compiler.mapper().get_currency_id(Currency::EUR).is_some());
        assert_eq!(compiler.mapper().currency_count(), 2);
    }

    #[test]
    fn test_xccy_fx_pair_registration() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let xccy_swap = create_xccy_swap();
        compiler.compile_with_registration(&xccy_swap).unwrap();

        // EUR/USD pair should be registered
        let eur_usd = FxPair::new(Currency::EUR, Currency::USD);
        assert!(compiler.mapper().get_fx_pair_id(eur_usd).is_some());
        assert_eq!(compiler.mapper().fx_pair_count(), 1);
    }

    #[test]
    fn test_xccy_single_currency_no_fx() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        // Single currency trade (USD only)
        let usd_only = Trade::new("USD001", vec![create_usd_fixed_leg()], TradeType::Swap);

        let kernel = compiler.compile_with_registration(&usd_only).unwrap();

        // All cashflows should have fx_index_id = 0
        for i in 0..kernel.len() {
            assert_eq!(
                kernel.fx_index_ids[i], 0,
                "Single currency trade should have no FX conversion"
            );
        }
    }

    #[test]
    fn test_xccy_with_collateral_currency_discounting() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper)
            .with_base_currency(Currency::USD)
            .with_collateral_currency(Currency::EUR);

        let xccy_swap = create_xccy_swap();
        compiler.compile_with_registration(&xccy_swap).unwrap();

        // All discount curves should use EUR
        let eur_curve_id = compiler.mapper().get_discount_curve_id("EUR");
        assert!(
            eur_curve_id.is_some(),
            "EUR discount curve should be registered"
        );

        // USD curve should NOT be registered (using EUR collateral)
        let usd_curve_id = compiler.mapper().get_discount_curve_id("USD");
        assert!(
            usd_curve_id.is_none(),
            "USD discount curve should NOT be registered when using EUR collateral"
        );
    }

    #[test]
    fn test_xccy_without_collateral_currency() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let xccy_swap = create_xccy_swap();
        compiler.compile_with_registration(&xccy_swap).unwrap();

        // Both USD and EUR discount curves should be registered
        assert!(compiler.mapper().get_discount_curve_id("USD").is_some());
        assert!(compiler.mapper().get_discount_curve_id("EUR").is_some());
    }

    #[test]
    fn test_xccy_empty_trade_error() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper);

        let empty_trade = Trade::new("EMPTY001", vec![], TradeType::Swap);

        let result = compiler.compile_with_registration(&empty_trade);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::EmptyTrade(_)));
    }

    #[test]
    fn test_xccy_payment_dates_sorted() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper);

        let xccy_swap = create_xccy_swap();
        let kernel = compiler.compile_with_registration(&xccy_swap).unwrap();

        for i in 1..kernel.len() {
            assert!(
                kernel.payment_dates[i] >= kernel.payment_dates[i - 1],
                "Payment dates should be sorted ascending"
            );
        }
    }

    #[test]
    fn test_xccy_direction_preserved() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper);

        let xccy_swap = create_xccy_swap();
        let kernel = compiler.compile_with_registration(&xccy_swap).unwrap();

        let mut has_positive = false;
        let mut has_negative = false;

        for i in 0..kernel.len() {
            if kernel.notionals[i] > 0.0 {
                has_positive = true;
            } else if kernel.notionals[i] < 0.0 {
                has_negative = true;
            }
        }

        assert!(has_positive, "Should have receiver (positive) cashflows");
        assert!(has_negative, "Should have payer (negative) cashflows");
    }

    #[test]
    fn test_xccy_kernel_is_aligned() {
        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper);

        let xccy_swap = create_xccy_swap();
        let kernel = compiler.compile_with_registration(&xccy_swap).unwrap();

        assert!(
            kernel.is_aligned(),
            "Kernel buffers should be 64-byte aligned"
        );
    }

    #[test]
    fn test_xccy_three_currency_swap() {
        // Create a 3-currency trade: USD + EUR + GBP
        let usd_leg = create_usd_fixed_leg();
        let eur_leg = create_eur_floating_leg();

        // Create a GBP leg
        let gbp_cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 6, 30).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 6, 30).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.04),
            Currency::GBP,
        )];
        let gbp_leg = Leg::new(
            gbp_cashflows,
            Direction::Payer,
            LegType::Fixed,
            Currency::GBP,
        );

        let three_ccy = Trade::new(
            "3CCY001",
            vec![usd_leg, eur_leg, gbp_leg],
            TradeType::Generic,
        );

        let mapper = IndexMapper::new();
        let mut compiler = XCcyCompiler::new(mapper).with_base_currency(Currency::USD);

        let kernel = compiler.compile_with_registration(&three_ccy).unwrap();

        // Should have 5 cashflows (2 USD + 2 EUR + 1 GBP)
        assert_eq!(kernel.len(), 5);

        // Should have 3 currencies registered
        assert_eq!(compiler.mapper().currency_count(), 3);

        // Should have 2 FX pairs registered (EUR/USD and GBP/USD)
        assert_eq!(compiler.mapper().fx_pair_count(), 2);
    }

    #[test]
    fn test_xccy_compiler_debug() {
        let mapper = IndexMapper::new();
        let compiler = XCcyCompiler::new(mapper)
            .with_base_currency(Currency::EUR)
            .with_collateral_currency(Currency::USD);

        let debug_str = format!("{:?}", compiler);
        assert!(debug_str.contains("XCcyCompiler"));
        assert!(debug_str.contains("EUR"));
        assert!(debug_str.contains("USD"));
    }
}

//! Linear products compiler for IRS, Bonds, and other linear instruments.
//!
//! This module provides `LinearProductsCompiler` which compiles Trade
//! structures into PricingKernel IR for linear products.

use std::sync::Arc;

use infra_domain::{
    market::Currency,
    time::{BusinessDayConvention, Calendar, CalendarId, ConcreteCalendar, Date, Tenor},
    trade::{IndexType, Payoff, Trade},
};
use pricer_core::kernel::{CompileError, PricingKernel, PricingKernelBuilder};

use super::{
    index_mapper::{CmsIndex, IndexMapper},
    TradeCompiler,
};

/// Compiler for linear products (IRS, Bonds, FRAs).
///
/// Transforms `Trade` structures with fixed and floating legs into
/// `PricingKernel` IR format optimised for SIMD pricing.
///
/// # Supported Products
///
/// - Interest Rate Swaps (IRS) - fixed vs floating
/// - Fixed Rate Bonds
/// - Forward Rate Agreements (FRAs)
/// - Cross-currency swaps (basic support)
///
/// # Calendar Support
///
/// The compiler supports optional business day adjustment for payment and
/// fixing dates via [`Calendar`] and [`BusinessDayConvention`].
///
/// # Example
///
/// ```ignore
/// use pricer_models::compiler::{LinearProductsCompiler, IndexMapper};
/// use infra_domain::trade::Trade;
/// use infra_domain::time::{CalendarId, BusinessDayConvention};
///
/// let mapper = IndexMapper::with_common_indices();
/// let compiler = LinearProductsCompiler::new(mapper)
///     .with_calendar(CalendarId::NewYork, BusinessDayConvention::ModifiedFollowing);
///
/// let kernel = compiler.compile(&trade)?;
/// ```
#[derive(Clone)]
pub struct LinearProductsCompiler {
    /// Index mapper for ID resolution.
    mapper: IndexMapper,
    /// Reference date for converting dates to days from epoch.
    epoch: Date,
    /// Optional calendar for business day adjustment.
    calendar: Option<Arc<dyn Calendar>>,
    /// Business day convention for payment dates.
    payment_bdc: BusinessDayConvention,
    /// Business day convention for fixing dates.
    fixing_bdc: BusinessDayConvention,
}

impl std::fmt::Debug for LinearProductsCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinearProductsCompiler")
            .field("mapper", &self.mapper)
            .field("epoch", &self.epoch)
            .field("has_calendar", &self.calendar.is_some())
            .field("payment_bdc", &self.payment_bdc)
            .field("fixing_bdc", &self.fixing_bdc)
            .finish()
    }
}

impl LinearProductsCompiler {
    /// Creates a new compiler with the given index mapper.
    ///
    /// Uses Unix epoch (1970-01-01) as the reference date.
    /// No calendar adjustment is applied by default.
    #[must_use]
    pub fn new(mapper: IndexMapper) -> Self {
        // Unix epoch: 1970-01-01
        let epoch = Date::from_ymd(1970, 1, 1).expect("Unix epoch is valid");
        Self {
            mapper,
            epoch,
            calendar: None,
            payment_bdc: BusinessDayConvention::Unadjusted,
            fixing_bdc: BusinessDayConvention::Unadjusted,
        }
    }

    /// Creates a compiler with a custom reference date for day counting.
    ///
    /// # Arguments
    ///
    /// * `mapper` - Index mapper for ID resolution
    /// * `epoch` - Reference date for converting dates to integers
    #[must_use]
    pub fn with_epoch(mapper: IndexMapper, epoch: Date) -> Self {
        Self {
            mapper,
            epoch,
            calendar: None,
            payment_bdc: BusinessDayConvention::Unadjusted,
            fixing_bdc: BusinessDayConvention::Unadjusted,
        }
    }

    /// Configures the compiler with a calendar for business day adjustment.
    ///
    /// # Arguments
    ///
    /// * `calendar_id` - The calendar identifier (e.g., `CalendarId::NewYork`)
    /// * `convention` - Business day convention for payment dates
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pricer_models::compiler::{LinearProductsCompiler, IndexMapper};
    /// use infra_domain::time::{CalendarId, BusinessDayConvention};
    ///
    /// let compiler = LinearProductsCompiler::new(IndexMapper::new())
    ///     .with_calendar(CalendarId::Target, BusinessDayConvention::ModifiedFollowing);
    /// ```
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
    ///
    /// # Arguments
    ///
    /// * `calendar` - The calendar implementation
    /// * `convention` - Business day convention for payment dates
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
    ///
    /// # Arguments
    ///
    /// * `payment_convention` - Convention for payment date adjustment
    /// * `fixing_convention` - Convention for fixing date adjustment
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

    /// Returns the payment date business day convention.
    #[must_use]
    pub fn payment_convention(&self) -> BusinessDayConvention { self.payment_bdc }

    /// Returns the fixing date business day convention.
    #[must_use]
    pub fn fixing_convention(&self) -> BusinessDayConvention { self.fixing_bdc }

    /// Adjusts a date according to the calendar and convention.
    ///
    /// If no calendar is configured, returns the original date.
    fn adjust_date(&self, date: Date, convention: BusinessDayConvention) -> Date {
        match &self.calendar {
            Some(cal) => cal.adjust(date, convention),
            None => date,
        }
    }

    /// Converts a Date to days from epoch, optionally adjusting for business
    /// days.
    fn date_to_days(&self, date: Date) -> i32 {
        // Date subtraction returns i64 (number of days between dates)
        (date - self.epoch) as i32
    }

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

    /// Extracts gearing and spread from a Payoff.
    ///
    /// Returns `(gearing, spread, fwd_index_id)`:
    /// - Fixed: (0.0, rate, 0)
    /// - Linear: (multiplier, spread, index_id)
    fn extract_payoff_params(&mut self, payoff: &Payoff) -> Result<(f64, f64, u16), CompileError> {
        match payoff {
            Payoff::Fixed { rate } => {
                // Fixed: gearing = 0, spread = rate, fwd_index_id = 0 (dummy)
                Ok((0.0, *rate, 0))
            }
            Payoff::Linear {
                index,
                spread,
                multiplier,
            } => {
                // Extract rate index from IndexType
                let fwd_index_id = match index {
                    IndexType::Rate(rate_index) => {
                        self.mapper.get_or_register_forward_index(*rate_index)
                    }
                    IndexType::SwapRate { currency, tenor } => {
                        // CMS (Constant Maturity Swap) index
                        // Parse currency and tenor strings to create CmsIndex
                        let ccy = currency
                            .parse::<Currency>()
                            .map_err(|_| CompileError::UnknownCurrency(currency.clone()))?;
                        let swap_tenor = tenor
                            .parse::<Tenor>()
                            .map_err(CompileError::InvalidSchedule)?;

                        let cms_index = CmsIndex::new(ccy, swap_tenor);
                        self.mapper.get_or_register_cms_index(cms_index)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedPayoff(format!(
                            "Non-rate index not supported for linear products: {:?}",
                            index
                        )));
                    }
                };
                Ok((*multiplier, *spread, fwd_index_id))
            }
            Payoff::VanillaOption { .. } | Payoff::Digital { .. } => {
                Err(CompileError::UnsupportedPayoff(
                    "Option payoffs not supported in linear compiler".to_string(),
                ))
            }
        }
    }
}

impl TradeCompiler<Trade> for LinearProductsCompiler {
    fn compile(&self, trade: &Trade) -> Result<PricingKernel, CompileError> {
        // We need a mutable mapper for registration, so clone it
        let mut compiler = self.clone();
        compiler.compile_with_registration(trade)
    }
}

impl LinearProductsCompiler {
    /// Internal compilation with mutable mapper access.
    fn compile_with_registration(&mut self, trade: &Trade) -> Result<PricingKernel, CompileError> {
        // Count total cashflows for capacity hint
        let total_cashflows: usize = trade.legs().map(|leg| leg.len()).sum();

        if total_cashflows == 0 {
            return Err(CompileError::EmptyTrade(trade.id.to_string()));
        }

        let mut builder = PricingKernelBuilder::with_capacity(total_cashflows);

        // Process each leg
        for leg in trade.legs() {
            let currency_id = self.mapper.get_or_register_currency(leg.currency);

            // Determine discount curve based on currency
            // Default convention: currency name as discount curve
            let discount_curve_id = self
                .mapper
                .get_or_register_discount_curve(leg.currency.code());

            // Get direction sign for notional
            let direction_sign = leg.direction.sign();

            // Process each cashflow in the leg
            for cf in leg.cashflows() {
                // Skip Fee cashflows (not relevant for pricing)
                if matches!(cf.cf_type, infra_domain::trade::CashflowType::Fee) {
                    continue;
                }

                // Convert dates to days from epoch with business day adjustment
                let payment_date = self.payment_date_to_days(cf.payment_date);
                let fixing_date = self.fixing_date_to_days(cf.accrual_start); // Use accrual start as fixing date

                // Extract payoff parameters
                let (gearing, spread, fwd_index_id) = self.extract_payoff_params(&cf.payoff)?;

                // Apply direction to notional
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
                    0, // fx_index_id = 0 (no FX conversion for single currency)
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
        market::{Currency, RateIndex},
        trade::{Cashflow, CashflowType, Direction, Leg, LegType, Payoff},
    };

    use super::*;

    fn create_fixed_leg() -> Leg {
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

    fn create_floating_leg() -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), 0.001),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                1_000_000.0,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), 0.001),
                Currency::USD,
            ),
        ];

        Leg::new(
            cashflows,
            Direction::Payer,
            LegType::Floating,
            Currency::USD,
        )
    }

    fn create_test_swap() -> Trade {
        use infra_domain::trade::TradeType;

        Trade::new(
            "SWAP001",
            vec![create_fixed_leg(), create_floating_leg()],
            TradeType::Swap,
        )
    }

    #[test]
    fn test_linear_compiler_new() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        assert_eq!(compiler.mapper().forward_index_count(), 0);
    }

    #[test]
    fn test_compile_swap() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        let kernel = compiler.compile_with_registration(&swap).unwrap();

        // Should have 4 cashflows (2 fixed + 2 floating)
        assert_eq!(kernel.len(), 4);
        assert_eq!(kernel.trade_count(), 1);
    }

    #[test]
    fn test_compile_fixed_leg_gearing() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "FIXED001",
            vec![create_fixed_leg()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Fixed leg should have gearing = 0.0
        for i in 0..kernel.len() {
            assert!(
                (kernel.gearings[i] - 0.0).abs() < 1e-10,
                "Fixed cashflow gearing should be 0"
            );
            assert_eq!(
                kernel.fwd_index_ids[i], 0,
                "Fixed cashflow should use dummy index"
            );
        }
    }

    #[test]
    fn test_compile_floating_leg_gearing() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "FLOAT001",
            vec![create_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Floating leg should have gearing = 1.0 (default multiplier)
        for i in 0..kernel.len() {
            assert!(
                (kernel.gearings[i] - 1.0).abs() < 1e-10,
                "Floating cashflow gearing should be 1.0"
            );
            assert_ne!(
                kernel.fwd_index_ids[i], 0,
                "Floating cashflow should use real index"
            );
        }
    }

    #[test]
    fn test_compile_direction_sign() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        let kernel = compiler.compile_with_registration(&swap).unwrap();

        // Check that direction is applied (receiver = positive, payer = negative)
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
    fn test_compile_payment_dates_sorted() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        let kernel = compiler.compile_with_registration(&swap).unwrap();

        // Payment dates should be sorted
        for i in 1..kernel.len() {
            assert!(
                kernel.payment_dates[i] >= kernel.payment_dates[i - 1],
                "Payment dates should be sorted ascending"
            );
        }
    }

    #[test]
    fn test_compile_empty_trade_error() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let empty_trade = Trade::new("EMPTY001", vec![], infra_domain::trade::TradeType::Generic);

        let result = compiler.compile_with_registration(&empty_trade);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::EmptyTrade(_)));
    }

    #[test]
    fn test_compile_currency_registration() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        compiler.compile_with_registration(&swap).unwrap();

        // USD should be registered
        assert!(compiler.mapper().get_currency_id(Currency::USD).is_some());
        assert_eq!(compiler.mapper().currency_count(), 1);
    }

    #[test]
    fn test_compile_index_registration() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        compiler.compile_with_registration(&swap).unwrap();

        // SOFR should be registered
        assert!(compiler
            .mapper()
            .get_forward_index_id(RateIndex::Sofr)
            .is_some());
    }

    #[test]
    fn test_date_to_days() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        // Test known date: 2020-01-01 is 18262 days from Unix epoch
        let date = Date::from_ymd(2020, 1, 1).unwrap();
        let days = compiler.date_to_days(date);
        assert_eq!(days, 18262);
    }

    #[test]
    fn test_kernel_is_aligned() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let swap = create_test_swap();
        let kernel = compiler.compile_with_registration(&swap).unwrap();

        assert!(
            kernel.is_aligned(),
            "Kernel buffers should be 64-byte aligned"
        );
    }

    #[test]
    fn test_compile_with_spread() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        // The floating leg has 0.001 (10bp) spread
        let trade = Trade::new(
            "FLOAT001",
            vec![create_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Check that spread is captured
        for i in 0..kernel.len() {
            assert!(
                (kernel.spreads[i] - 0.001).abs() < 1e-10,
                "Spread should be 0.001 (10bp)"
            );
        }
    }

    // === Task 3.2: IRS Amortizing Support Tests ===

    /// Creates an amortizing fixed leg where notional decreases each period.
    fn create_amortizing_fixed_leg() -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                1_000_000.0, // Full notional
                Payoff::fixed(0.05),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                750_000.0, // 75% notional
                Payoff::fixed(0.05),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2026, 6, 30).unwrap(),
                Date::from_ymd(2026, 1, 1).unwrap(),
                Date::from_ymd(2026, 6, 30).unwrap(),
                0.5,
                500_000.0, // 50% notional
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

    #[test]
    fn test_compile_amortizing_irs() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "AMORT001",
            vec![create_amortizing_fixed_leg()],
            infra_domain::trade::TradeType::Swap,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Should have 3 cashflows with decreasing notionals
        assert_eq!(kernel.len(), 3);

        // Verify notionals are preserved (with receiver direction = positive)
        assert!((kernel.notionals[0] - 1_000_000.0).abs() < 1e-6);
        assert!((kernel.notionals[1] - 750_000.0).abs() < 1e-6);
        assert!((kernel.notionals[2] - 500_000.0).abs() < 1e-6);
    }

    // === Task 3.3: Bond Principal and FRA Support Tests ===

    /// Creates a bond with coupons and principal redemption.
    fn create_bond_with_principal() -> Leg {
        let cashflows = vec![
            // Coupon 1
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                100_000_000.0,
                Payoff::fixed(0.04), // 4% coupon
                Currency::USD,
            ),
            // Coupon 2
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                100_000_000.0,
                Payoff::fixed(0.04),
                Currency::USD,
            ),
            // Principal redemption at maturity
            Cashflow::new(
                CashflowType::Principal,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                1.0, // year_fraction = 1.0 for principal
                100_000_000.0,
                Payoff::fixed(1.0), // rate = 1.0 for principal (100% return)
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

    #[test]
    fn test_compile_bond_with_principal() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "BOND001",
            vec![create_bond_with_principal()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Should have 3 cashflows (2 coupons + 1 principal)
        assert_eq!(kernel.len(), 3);

        // First two should be coupons (rate = 0.04)
        assert!((kernel.spreads[0] - 0.04).abs() < 1e-10);
        assert!((kernel.spreads[1] - 0.04).abs() < 1e-10);

        // Third should be principal (rate = 1.0)
        assert!((kernel.spreads[2] - 1.0).abs() < 1e-10);
    }

    /// Creates a single FRA settlement cashflow.
    fn create_fra_settlement() -> Leg {
        // FRA: Single settlement cashflow
        let cashflows = vec![Cashflow::new(
            CashflowType::Settlement,
            Date::from_ymd(2025, 3, 15).unwrap(), // Settlement date
            Date::from_ymd(2025, 3, 15).unwrap(),
            Date::from_ymd(2025, 6, 15).unwrap(), // 3M tenor
            0.25,                                 // 3-month year fraction
            10_000_000.0,
            Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), -0.025), /* FRA rate as negative spread */
            Currency::USD,
        )];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Floating,
            Currency::USD,
        )
    }

    #[test]
    fn test_compile_fra() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "FRA001",
            vec![create_fra_settlement()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Should have 1 cashflow
        assert_eq!(kernel.len(), 1);

        // Floating with gearing = 1.0
        assert!((kernel.gearings[0] - 1.0).abs() < 1e-10);

        // Spread = -0.025 (FRA rate)
        assert!((kernel.spreads[0] - (-0.025)).abs() < 1e-10);
    }

    // === Task 3.4: Calendar and Business Day Adjustment Tests ===

    use infra_domain::time::{BusinessDayConvention, CalendarId, ConcreteCalendar};

    #[test]
    fn test_compiler_with_calendar() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper).with_calendar(
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
        );

        assert!(compiler.has_calendar());
        assert_eq!(
            compiler.payment_convention(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            compiler.fixing_convention(),
            BusinessDayConvention::ModifiedFollowing
        );
    }

    #[test]
    fn test_compiler_without_calendar() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        assert!(!compiler.has_calendar());
        assert_eq!(
            compiler.payment_convention(),
            BusinessDayConvention::Unadjusted
        );
    }

    #[test]
    fn test_compiler_with_custom_calendar() {
        use std::sync::Arc;

        let mapper = IndexMapper::new();
        let calendar = Arc::new(ConcreteCalendar::new(CalendarId::Target));
        let compiler = LinearProductsCompiler::new(mapper)
            .with_custom_calendar(calendar, BusinessDayConvention::Following);

        assert!(compiler.has_calendar());
        assert_eq!(
            compiler.payment_convention(),
            BusinessDayConvention::Following
        );
    }

    #[test]
    fn test_compiler_with_separate_conventions() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper)
            .with_calendar(
                CalendarId::NewYork,
                BusinessDayConvention::ModifiedFollowing,
            )
            .with_conventions(
                BusinessDayConvention::ModifiedFollowing,
                BusinessDayConvention::Preceding,
            );

        assert_eq!(
            compiler.payment_convention(),
            BusinessDayConvention::ModifiedFollowing
        );
        assert_eq!(
            compiler.fixing_convention(),
            BusinessDayConvention::Preceding
        );
    }

    /// Creates a leg with a payment date on a weekend.
    fn create_leg_with_weekend_payment() -> Leg {
        // 2026-01-10 is Saturday
        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2026, 1, 10).unwrap(), // Saturday
            Date::from_ymd(2025, 7, 10).unwrap(),
            Date::from_ymd(2026, 1, 10).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.05),
            Currency::USD,
        )];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        )
    }

    #[test]
    fn test_compile_with_calendar_adjustment() {
        let mapper = IndexMapper::new();
        let mut compiler_with_cal = LinearProductsCompiler::new(mapper.clone())
            .with_calendar(CalendarId::WeekendOnly, BusinessDayConvention::Following);
        let mut compiler_no_cal = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "TEST001",
            vec![create_leg_with_weekend_payment()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel_with_cal = compiler_with_cal.compile_with_registration(&trade).unwrap();
        let kernel_no_cal = compiler_no_cal.compile_with_registration(&trade).unwrap();

        // Without calendar: 2026-01-10 (Saturday) stays as Saturday
        // With calendar (Following): 2026-01-10 -> 2026-01-12 (Monday)
        // The difference should be 2 days (Saturday -> Monday)
        let diff = kernel_with_cal.payment_dates[0] - kernel_no_cal.payment_dates[0];
        assert_eq!(
            diff, 2,
            "Following convention should adjust Saturday to Monday (+2 days)"
        );
    }

    #[test]
    fn test_compile_with_modified_following_month_boundary() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper).with_calendar(
            CalendarId::WeekendOnly,
            BusinessDayConvention::ModifiedFollowing,
        );

        // 2026-01-31 is Saturday, next business day (Feb 2) crosses month
        // ModifiedFollowing should go to Jan 30 (Friday)
        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2026, 1, 31).unwrap(), // Saturday (month end)
            Date::from_ymd(2025, 7, 31).unwrap(),
            Date::from_ymd(2026, 1, 31).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.05),
            Currency::USD,
        )];

        let leg = Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        );

        let trade = Trade::new(
            "MONTHEND001",
            vec![leg],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Jan 30, 2026 in days from Unix epoch
        let jan_30_2026 = Date::from_ymd(2026, 1, 30).unwrap();
        let epoch = Date::from_ymd(1970, 1, 1).unwrap();
        let expected_days = (jan_30_2026 - epoch) as i32;

        assert_eq!(
            kernel.payment_dates[0], expected_days,
            "ModifiedFollowing should adjust to Jan 30 (Friday), not Feb 2 (Monday)"
        );
    }

    #[test]
    fn test_compile_with_target_calendar_holiday() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper)
            .with_calendar(CalendarId::Target, BusinessDayConvention::Following);

        // 2026-12-25 is Christmas Day (Friday), a TARGET holiday
        // Following should go to Dec 28 (Monday, skip Sat/Sun and Boxing Day)
        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2026, 12, 25).unwrap(), // Christmas
            Date::from_ymd(2026, 6, 25).unwrap(),
            Date::from_ymd(2026, 12, 25).unwrap(),
            0.5,
            1_000_000.0,
            Payoff::fixed(0.05),
            Currency::USD,
        )];

        let leg = Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Fixed,
            Currency::USD,
        );

        let trade = Trade::new(
            "XMAS001",
            vec![leg],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Dec 25, 2026 is Friday (Christmas)
        // Dec 26, 2026 is Saturday (Boxing Day + weekend)
        // Dec 27, 2026 is Sunday
        // Dec 28, 2026 is Monday - first business day
        let dec_28_2026 = Date::from_ymd(2026, 12, 28).unwrap();
        let epoch = Date::from_ymd(1970, 1, 1).unwrap();
        let expected_days = (dec_28_2026 - epoch) as i32;

        assert_eq!(
            kernel.payment_dates[0], expected_days,
            "TARGET calendar should adjust Christmas to Dec 28 (Monday)"
        );
    }

    #[test]
    fn test_compiler_debug_format() {
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper).with_calendar(
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
        );

        let debug_str = format!("{:?}", compiler);
        assert!(debug_str.contains("LinearProductsCompiler"));
        assert!(debug_str.contains("has_calendar: true"));
        assert!(debug_str.contains("ModifiedFollowing"));
    }

    // === Task 8.1: CMS Index Integration Tests ===

    /// Creates a CMS floating leg (10Y USD CMS rate).
    fn create_cms_floating_leg() -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                10_000_000.0,
                Payoff::Linear {
                    index: IndexType::SwapRate {
                        currency: "USD".to_string(),
                        tenor: "10Y".to_string(),
                    },
                    spread: 0.002, // 20bp spread
                    multiplier: 1.0,
                },
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                10_000_000.0,
                Payoff::Linear {
                    index: IndexType::SwapRate {
                        currency: "USD".to_string(),
                        tenor: "10Y".to_string(),
                    },
                    spread: 0.002,
                    multiplier: 1.0,
                },
                Currency::USD,
            ),
        ];

        Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Floating,
            Currency::USD,
        )
    }

    #[test]
    fn test_compile_cms_leg() {
        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "CMS001",
            vec![create_cms_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Should have 2 cashflows
        assert_eq!(kernel.len(), 2);

        // CMS leg should have gearing = 1.0 (floating)
        for i in 0..kernel.len() {
            assert!(
                (kernel.gearings[i] - 1.0).abs() < 1e-10,
                "CMS cashflow gearing should be 1.0"
            );
            assert_ne!(
                kernel.fwd_index_ids[i], 0,
                "CMS cashflow should use real index (not dummy)"
            );
            assert!(
                (kernel.spreads[i] - 0.002).abs() < 1e-10,
                "CMS spread should be 0.002 (20bp)"
            );
        }

        // CMS index should be registered
        assert!(
            compiler.mapper().cms_index_count() >= 1,
            "CMS index should be registered"
        );
    }

    #[test]
    fn test_compile_cms_index_registered() {
        use infra_domain::time::Tenor;

        use crate::compiler::CmsIndex;

        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        let trade = Trade::new(
            "CMS002",
            vec![create_cms_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );

        compiler.compile_with_registration(&trade).unwrap();

        // Should be able to find the 10Y USD CMS index
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        assert!(
            compiler.mapper().get_cms_index_id(cms10y).is_some(),
            "10Y USD CMS index should be registered"
        );
    }

    #[test]
    fn test_compile_cms_5y_eur() {
        use infra_domain::time::Tenor;

        use crate::compiler::CmsIndex;

        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        // Create a 5Y EUR CMS leg
        let cashflows = vec![Cashflow::new(
            CashflowType::Coupon,
            Date::from_ymd(2025, 6, 30).unwrap(),
            Date::from_ymd(2025, 1, 1).unwrap(),
            Date::from_ymd(2025, 6, 30).unwrap(),
            0.5,
            10_000_000.0,
            Payoff::Linear {
                index: IndexType::SwapRate {
                    currency: "EUR".to_string(),
                    tenor: "5Y".to_string(),
                },
                spread: 0.0,
                multiplier: 0.8, // 80% gearing
            },
            Currency::EUR,
        )];

        let leg = Leg::new(
            cashflows,
            Direction::Receiver,
            LegType::Floating,
            Currency::EUR,
        );

        let trade = Trade::new(
            "CMS_EUR001",
            vec![leg],
            infra_domain::trade::TradeType::Generic,
        );

        let kernel = compiler.compile_with_registration(&trade).unwrap();

        // Verify gearing is preserved
        assert!(
            (kernel.gearings[0] - 0.8).abs() < 1e-10,
            "CMS gearing should be 0.8"
        );

        // 5Y EUR CMS index should be registered
        let cms5y_eur = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
        assert!(
            compiler.mapper().get_cms_index_id(cms5y_eur).is_some(),
            "5Y EUR CMS index should be registered"
        );
    }

    #[test]
    fn test_cms_and_ibor_share_id_space() {
        use infra_domain::time::Tenor;

        use crate::compiler::CmsIndex;

        let mapper = IndexMapper::new();
        let mut compiler = LinearProductsCompiler::new(mapper);

        // First compile a regular IBOR leg
        let ibor_trade = Trade::new(
            "IBOR001",
            vec![create_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );
        compiler.compile_with_registration(&ibor_trade).unwrap();

        let sofr_id = compiler
            .mapper()
            .get_forward_index_id(RateIndex::Sofr)
            .expect("SOFR should be registered");

        // Then compile a CMS leg
        let cms_trade = Trade::new(
            "CMS003",
            vec![create_cms_floating_leg()],
            infra_domain::trade::TradeType::Generic,
        );
        compiler.compile_with_registration(&cms_trade).unwrap();

        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms_id = compiler
            .mapper()
            .get_cms_index_id(cms10y)
            .expect("CMS should be registered");

        // CMS and IBOR should use different IDs
        assert_ne!(sofr_id, cms_id, "CMS and IBOR should have different IDs");
        // CMS ID should be greater (registered after IBOR)
        assert!(cms_id > sofr_id, "CMS ID should be > IBOR ID");
    }
}

//! Callable/Bermudan product compiler.
//!
//! This module provides `CallableCompiler` for transforming Bermudan/callable
//! products into `CallableKernel` IR.
//!
//! # Design
//!
//! The compiler:
//! 1. Divides the product timeline by exercise dates into blocks
//! 2. Compiles underlying cashflows within each block
//! 3. Attaches exercise definitions at block boundaries
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::compiler::{CallableCompiler, IndexMapper};
//! use pricer_core::ir::CallableKernel;
//!
//! let mut mapper = IndexMapper::new();
//! let compiler = CallableCompiler::new(&mut mapper);
//!
//! let kernel = compiler.compile_bermudan_swaption(
//!     &exercise_dates,
//!     &swaption,
//!     valuation_date,
//! )?;
//! ```

use pricer_core::ir::{
    CallableBlock, CallableKernel, CallableKernelBuilder, CompileError, ExerciseDef, ExerciseStyle,
    PricingKernel, PricingKernelBuilder,
};

use super::IndexMapper;

/// Compiler for callable/Bermudan products.
///
/// Transforms products with early exercise features (Bermudan swaptions,
/// callable bonds) into `CallableKernel` IR.
///
/// # Type Parameters
///
/// Uses a mutable reference to `IndexMapper` to register indices during
/// compilation.
pub struct CallableCompiler<'a> {
    mapper: &'a mut IndexMapper,
}

impl<'a> CallableCompiler<'a> {
    /// Creates a new `CallableCompiler`.
    ///
    /// # Arguments
    ///
    /// * `mapper` - Mutable reference to index mapper for ID allocation
    #[must_use]
    pub fn new(mapper: &'a mut IndexMapper) -> Self { Self { mapper } }

    /// Returns a reference to the underlying index mapper.
    #[must_use]
    pub fn mapper(&self) -> &IndexMapper { self.mapper }

    /// Returns a mutable reference to the underlying index mapper.
    pub fn mapper_mut(&mut self) -> &mut IndexMapper { self.mapper }

    /// Compiles a generic callable product from exercise schedule and
    /// cashflows.
    ///
    /// This is the core compilation method that other product-specific
    /// methods delegate to.
    ///
    /// # Arguments
    ///
    /// * `exercise_dates` - Sorted list of exercise dates (days from epoch)
    /// * `cashflows` - All underlying cashflows (will be partitioned by
    ///   exercise dates)
    /// * `base_currency_id` - Base currency ID for valuation
    /// * `exercise_style` - Bermudan or American
    /// * `exercise_costs` - Optional exercise costs for each exercise date
    ///
    /// # Returns
    ///
    /// `CallableKernel` with cashflows partitioned into exercise blocks.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if:
    /// - Exercise dates are not sorted
    /// - Exercise dates list is empty
    pub fn compile_from_schedule(
        &mut self,
        start_date: i32,
        maturity: i32,
        exercise_dates: &[i32],
        cashflows: &CashflowSchedule,
        base_currency_id: u8,
        exercise_style: ExerciseStyle,
        exercise_costs: Option<&[f64]>,
    ) -> Result<CallableKernel, CompileError> {
        // Validate inputs
        if exercise_dates.is_empty() {
            return Err(CompileError::invalid_schedule(
                "At least one exercise date required for callable product",
            ));
        }

        // Check exercise dates are sorted
        for window in exercise_dates.windows(2) {
            if window[0] >= window[1] {
                return Err(CompileError::invalid_schedule(
                    "Exercise dates must be strictly increasing",
                ));
            }
        }

        // Validate exercise costs length if provided
        if let Some(costs) = exercise_costs {
            if costs.len() != exercise_dates.len() {
                return Err(CompileError::length_mismatch(
                    exercise_dates.len(),
                    costs.len(),
                ));
            }
        }

        let mut builder = CallableKernelBuilder::new().base_currency(base_currency_id);

        // Build blocks based on exercise dates
        let mut current_start = start_date;

        for (i, &exercise_date) in exercise_dates.iter().enumerate() {
            // Get cashflows within this block [current_start, exercise_date]
            let block_flows =
                self.extract_cashflows_for_period(cashflows, current_start, exercise_date)?;

            // Get exercise cost if provided
            let exercise_cost = exercise_costs.map_or(0.0, |costs| costs[i]);

            // Create exercise definition
            let exercise_def = ExerciseDef::new(exercise_date, exercise_cost, exercise_style);

            // Add block
            builder = builder.add_block(CallableBlock::new(
                current_start,
                exercise_date,
                block_flows,
                Some(exercise_def),
            ));

            current_start = exercise_date;
        }

        // Add final block from last exercise to maturity (no exercise)
        if current_start < maturity {
            let final_flows =
                self.extract_cashflows_for_period(cashflows, current_start, maturity)?;

            builder = builder.add_block(CallableBlock::new(
                current_start,
                maturity,
                final_flows,
                None,
            ));
        }

        Ok(builder.build())
    }

    /// Extracts cashflows within a given period.
    fn extract_cashflows_for_period(
        &self,
        cashflows: &CashflowSchedule,
        period_start: i32,
        period_end: i32,
    ) -> Result<PricingKernel, CompileError> {
        let mut builder = PricingKernelBuilder::with_capacity(cashflows.len());

        for cf in cashflows.iter() {
            // Include cashflows with payment date in (period_start, period_end]
            if cf.payment_date > period_start && cf.payment_date <= period_end {
                builder.add_cashflow(
                    cf.payment_date,
                    cf.fixing_date,
                    cf.year_fraction,
                    cf.notional,
                    cf.spread,
                    cf.gearing,
                    cf.currency_id,
                    cf.discount_curve_id,
                    cf.fwd_index_id,
                    cf.fx_index_id,
                );
            }
        }

        builder.build()
    }

    /// Compiles a Bermudan swaption from exercise dates and underlying
    /// swap details.
    ///
    /// # Arguments
    ///
    /// * `exercise_dates` - Bermudan exercise dates (days from epoch)
    /// * `swap_start` - Start date of the underlying swap
    /// * `swap_maturity` - Maturity date of the underlying swap
    /// * `fixed_rate` - Fixed rate of the underlying swap
    /// * `notional` - Notional amount
    /// * `currency_id` - Currency ID
    /// * `discount_curve_id` - Discount curve ID
    /// * `fwd_index_id` - Forward index ID for floating leg
    /// * `is_payer` - True if payer swaption (pay fixed)
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if compilation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_bermudan_swaption(
        &mut self,
        exercise_dates: &[i32],
        swap_start: i32,
        swap_maturity: i32,
        fixed_rate: f64,
        notional: f64,
        currency_id: u8,
        discount_curve_id: u8,
        fwd_index_id: u16,
        is_payer: bool,
    ) -> Result<CallableKernel, CompileError> {
        // Generate underlying swap cashflows
        // For simplicity, assume annual payment frequency
        let cashflows = self.generate_swap_cashflows(
            swap_start,
            swap_maturity,
            fixed_rate,
            notional,
            currency_id,
            discount_curve_id,
            fwd_index_id,
            is_payer,
        );

        self.compile_from_schedule(
            swap_start,
            swap_maturity,
            exercise_dates,
            &cashflows,
            currency_id,
            ExerciseStyle::Bermudan,
            None,
        )
    }

    /// Generates swap cashflows for the underlying swap.
    ///
    /// Creates both fixed and floating leg cashflows for a standard IRS.
    #[allow(clippy::too_many_arguments)]
    fn generate_swap_cashflows(
        &self,
        start_date: i32,
        maturity: i32,
        fixed_rate: f64,
        notional: f64,
        currency_id: u8,
        discount_curve_id: u8,
        fwd_index_id: u16,
        is_payer: bool,
    ) -> CashflowSchedule {
        let mut cashflows = Vec::new();

        // Assume annual frequency (365 days per period)
        let period_days = 365;

        // Direction multiplier: payer pays fixed, receives floating
        let fixed_direction = if is_payer { -1.0 } else { 1.0 };
        let float_direction = if is_payer { 1.0 } else { -1.0 };

        let mut period_start = start_date;
        while period_start < maturity {
            let period_end = (period_start + period_days).min(maturity);
            let actual_year_fraction = (period_end - period_start) as f64 / 365.0;

            // Fixing date is typically 2 days before period start
            let fixing_date = period_start - 2;

            // Fixed leg cashflow
            cashflows.push(Cashflow {
                payment_date: period_end,
                fixing_date,
                year_fraction: actual_year_fraction,
                notional: notional * fixed_direction,
                spread: fixed_rate,
                gearing: 0.0, // Fixed leg
                currency_id,
                discount_curve_id,
                fwd_index_id: 0, // Dummy for fixed
                fx_index_id: 0,
            });

            // Floating leg cashflow
            cashflows.push(Cashflow {
                payment_date: period_end,
                fixing_date,
                year_fraction: actual_year_fraction,
                notional: notional * float_direction,
                spread: 0.0,
                gearing: 1.0, // Floating leg
                currency_id,
                discount_curve_id,
                fwd_index_id,
                fx_index_id: 0,
            });

            period_start = period_end;
        }

        CashflowSchedule::new(cashflows)
    }
}

/// Individual cashflow data for compilation.
#[derive(Debug, Clone)]
pub struct Cashflow {
    /// Payment date (days from epoch).
    pub payment_date: i32,
    /// Fixing date for floating coupons.
    pub fixing_date: i32,
    /// Year fraction.
    pub year_fraction: f64,
    /// Notional amount (signed for direction).
    pub notional: f64,
    /// Spread or fixed rate.
    pub spread: f64,
    /// Gearing (0 for fixed, 1 for floating).
    pub gearing: f64,
    /// Currency ID.
    pub currency_id: u8,
    /// Discount curve ID.
    pub discount_curve_id: u8,
    /// Forward index ID (0 for fixed).
    pub fwd_index_id: u16,
    /// FX index ID (0 for no FX).
    pub fx_index_id: u16,
}

/// Schedule of cashflows for compilation.
#[derive(Debug, Clone)]
pub struct CashflowSchedule {
    cashflows: Vec<Cashflow>,
}

impl CashflowSchedule {
    /// Creates a new cashflow schedule.
    #[must_use]
    pub fn new(cashflows: Vec<Cashflow>) -> Self { Self { cashflows } }

    /// Creates an empty schedule.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            cashflows: Vec::new(),
        }
    }

    /// Returns the number of cashflows.
    #[must_use]
    pub fn len(&self) -> usize { self.cashflows.len() }

    /// Returns true if the schedule is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.cashflows.is_empty() }

    /// Returns an iterator over cashflows.
    pub fn iter(&self) -> impl Iterator<Item = &Cashflow> { self.cashflows.iter() }

    /// Adds a cashflow to the schedule.
    pub fn push(&mut self, cashflow: Cashflow) { self.cashflows.push(cashflow); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callable_compiler_new() {
        let mut mapper = IndexMapper::new();
        let compiler = CallableCompiler::new(&mut mapper);
        assert_eq!(compiler.mapper().forward_index_count(), 0);
    }

    #[test]
    fn test_cashflow_schedule_new() {
        let schedule = CashflowSchedule::empty();
        assert!(schedule.is_empty());
        assert_eq!(schedule.len(), 0);
    }

    #[test]
    fn test_cashflow_schedule_push() {
        let mut schedule = CashflowSchedule::empty();
        schedule.push(Cashflow {
            payment_date: 19365,
            fixing_date: 19363,
            year_fraction: 1.0,
            notional: 1_000_000.0,
            spread: 0.03,
            gearing: 0.0,
            currency_id: 0,
            discount_curve_id: 0,
            fwd_index_id: 0,
            fx_index_id: 0,
        });

        assert_eq!(schedule.len(), 1);
    }

    #[test]
    fn test_compile_from_schedule_empty_exercise_dates() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let result = compiler.compile_from_schedule(
            19000,
            20000,
            &[],
            &CashflowSchedule::empty(),
            0,
            ExerciseStyle::Bermudan,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_compile_from_schedule_unsorted_dates() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let result = compiler.compile_from_schedule(
            19000,
            20000,
            &[19500, 19400, 19600], // Not sorted
            &CashflowSchedule::empty(),
            0,
            ExerciseStyle::Bermudan,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_compile_from_schedule_success() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let exercise_dates = vec![19365, 19730];
        let cashflows = CashflowSchedule::new(vec![
            Cashflow {
                payment_date: 19365,
                fixing_date: 19363,
                year_fraction: 1.0,
                notional: 1_000_000.0,
                spread: 0.03,
                gearing: 0.0,
                currency_id: 0,
                discount_curve_id: 0,
                fwd_index_id: 0,
                fx_index_id: 0,
            },
            Cashflow {
                payment_date: 19730,
                fixing_date: 19728,
                year_fraction: 1.0,
                notional: 1_000_000.0,
                spread: 0.03,
                gearing: 0.0,
                currency_id: 0,
                discount_curve_id: 0,
                fwd_index_id: 0,
                fx_index_id: 0,
            },
        ]);

        let kernel = compiler
            .compile_from_schedule(
                19000,
                20000,
                &exercise_dates,
                &cashflows,
                0,
                ExerciseStyle::Bermudan,
                None,
            )
            .unwrap();

        // Should have 3 blocks: [start,ex1], [ex1,ex2], [ex2,maturity]
        assert_eq!(kernel.block_count(), 3);
        assert_eq!(kernel.exercise_count(), 2);
    }

    #[test]
    fn test_compile_bermudan_swaption() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let exercise_dates = vec![19365, 19730]; // 1Y, 2Y
        let swap_start = 19000;
        let swap_maturity = 22645; // ~10Y from start
        let fixed_rate = 0.03;
        let notional = 10_000_000.0;

        let kernel = compiler
            .compile_bermudan_swaption(
                &exercise_dates,
                swap_start,
                swap_maturity,
                fixed_rate,
                notional,
                0, // currency
                0, // discount curve
                1, // fwd index
                true,
            )
            .unwrap();

        assert_eq!(kernel.exercise_count(), 2);
        assert!(kernel.block_count() >= 2);
    }

    #[test]
    fn test_compile_with_exercise_costs() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let exercise_dates = vec![19365, 19730];
        let exercise_costs = vec![100.0, 50.0];
        let cashflows = CashflowSchedule::empty();

        let kernel = compiler
            .compile_from_schedule(
                19000,
                20000,
                &exercise_dates,
                &cashflows,
                0,
                ExerciseStyle::Bermudan,
                Some(&exercise_costs),
            )
            .unwrap();

        // Check exercise costs were set
        let first_block = &kernel.blocks[0];
        assert!(first_block.exercise.is_some());
        let exercise = first_block.exercise.as_ref().unwrap();
        assert!((exercise.exercise_cost - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_compile_with_mismatched_costs_length() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let exercise_dates = vec![19365, 19730];
        let exercise_costs = vec![100.0]; // Wrong length

        let result = compiler.compile_from_schedule(
            19000,
            20000,
            &exercise_dates,
            &CashflowSchedule::empty(),
            0,
            ExerciseStyle::Bermudan,
            Some(&exercise_costs),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_cashflow_partitioning() {
        let mut mapper = IndexMapper::new();
        let mut compiler = CallableCompiler::new(&mut mapper);

        let exercise_dates = vec![19365];
        let cashflows = CashflowSchedule::new(vec![
            // Cashflow in first block
            Cashflow {
                payment_date: 19200,
                fixing_date: 19198,
                year_fraction: 0.5,
                notional: 1_000_000.0,
                spread: 0.03,
                gearing: 0.0,
                currency_id: 0,
                discount_curve_id: 0,
                fwd_index_id: 0,
                fx_index_id: 0,
            },
            // Cashflow at exercise date (included in first block)
            Cashflow {
                payment_date: 19365,
                fixing_date: 19363,
                year_fraction: 0.5,
                notional: 1_000_000.0,
                spread: 0.03,
                gearing: 0.0,
                currency_id: 0,
                discount_curve_id: 0,
                fwd_index_id: 0,
                fx_index_id: 0,
            },
            // Cashflow in final block
            Cashflow {
                payment_date: 19500,
                fixing_date: 19498,
                year_fraction: 0.5,
                notional: 1_000_000.0,
                spread: 0.03,
                gearing: 0.0,
                currency_id: 0,
                discount_curve_id: 0,
                fwd_index_id: 0,
                fx_index_id: 0,
            },
        ]);

        let kernel = compiler
            .compile_from_schedule(
                19000,
                20000,
                &exercise_dates,
                &cashflows,
                0,
                ExerciseStyle::Bermudan,
                None,
            )
            .unwrap();

        // 2 blocks: [start, ex1], [ex1, maturity]
        assert_eq!(kernel.block_count(), 2);

        // First block should have 2 cashflows
        assert_eq!(kernel.blocks[0].cashflow_count(), 2);

        // Second block should have 1 cashflow
        assert_eq!(kernel.blocks[1].cashflow_count(), 1);
    }
}

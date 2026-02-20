//! Exotic option compiler for path-dependent products.
//!
//! This module provides `ExoticCompiler` which compiles exotic options
//! (barriers, Asians) into `ScriptKernel` IR.
//!
//! # Supported Products
//!
//! - **Barrier Options**: Up/Down, In/Out, continuous/discrete monitoring
//! - **Asian Options**: Arithmetic averaging on underlying
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::compiler::{ExoticCompiler, IndexMapper};
//! use infra_domain::market::Currency;
//!
//! let mapper = IndexMapper::new();
//! let compiler = ExoticCompiler::new(mapper);
//!
//! let script = compiler.compile_barrier_option(&trade)?;
//! ```

use infra_domain::{market::Currency, time::Date, trade::Payoff};
use pricer_core::kernel::{BarrierType, CompileError, ScriptKernel, ScriptKernelBuilder, ScriptOp};

use super::{
    script_product::{ObservationAction, ScriptProduct, ScriptProductType},
    IndexMapper,
};

/// Compiler for exotic options (barriers, Asians).
///
/// Transforms exotic option definitions into `ScriptKernel` IR
/// suitable for event-driven simulation.
///
/// # Supported Products
///
/// - Barrier options (single barrier)
/// - Asian options (arithmetic average)
///
/// # Example
///
/// ```ignore
/// use pricer_models::compiler::{ExoticCompiler, IndexMapper};
///
/// let mapper = IndexMapper::new();
/// let compiler = ExoticCompiler::new(mapper);
///
/// let kernel = compiler.compile_barrier_option(&trade)?;
/// ```
#[derive(Clone, Debug)]
pub struct ExoticCompiler {
    /// Index mapper for ID resolution.
    mapper: IndexMapper,
    /// Reference date for converting dates to time.
    epoch: Date,
    /// Valuation date (days from epoch).
    valuation_date_days: i32,
}

impl ExoticCompiler {
    /// Creates a new exotic compiler with the given index mapper.
    #[must_use]
    pub fn new(mapper: IndexMapper) -> Self {
        let epoch = Date::from_ymd(1970, 1, 1).expect("Unix epoch is valid");
        Self {
            mapper,
            epoch,
            valuation_date_days: 0,
        }
    }

    /// Sets the valuation date for time calculations.
    #[must_use]
    pub fn with_valuation_date(mut self, valuation_date: Date) -> Self {
        self.valuation_date_days = (valuation_date - self.epoch) as i32;
        self
    }

    /// Sets the valuation date in days from epoch.
    #[must_use]
    pub fn with_valuation_date_days(mut self, days: i32) -> Self {
        self.valuation_date_days = days;
        self
    }

    /// Returns a reference to the index mapper.
    #[must_use]
    pub fn mapper(&self) -> &IndexMapper { &self.mapper }

    /// Returns a mutable reference to the index mapper.
    pub fn mapper_mut(&mut self) -> &mut IndexMapper { &mut self.mapper }

    /// Converts date to time in years from valuation date.
    fn date_to_years(&self, date: Date) -> f64 {
        let days_from_epoch = (date - self.epoch) as i32;
        let days_to_date = days_from_epoch - self.valuation_date_days;
        days_to_date as f64 / 365.0
    }

    /// Compiles a barrier option into ScriptKernel.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `expiry` - Option expiry date
    /// * `barrier_level` - Barrier level (as fraction of spot, e.g., 1.1 =
    ///   110%)
    /// * `strike` - Strike price (as fraction of spot)
    /// * `notional` - Notional amount
    /// * `barrier_type` - Type of barrier (Up/Down, In/Out)
    /// * `is_call` - True for call, false for put
    /// * `underlying_index` - Index ID for the underlying
    /// * `currency` - Payment currency
    /// * `monitoring_dates` - Dates to check barrier (empty = continuous)
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_barrier_option(
        &mut self,
        trade_id: &str,
        expiry: Date,
        barrier_level: f64,
        strike: f64,
        notional: f64,
        barrier_type: BarrierType,
        is_call: bool,
        underlying_index: u16,
        currency: Currency,
        monitoring_dates: &[Date],
    ) -> Result<ScriptKernel, CompileError> {
        let currency_id = self.mapper.get_or_register_currency(currency);
        let dc_id = self.mapper.get_or_register_discount_curve(currency.code());

        let mut builder = ScriptKernelBuilder::new().trade_id(trade_id);

        // Add observation times (monitoring dates or just expiry)
        let observation_times: Vec<f64> = if monitoring_dates.is_empty() {
            // Discrete: just check at expiry
            vec![self.date_to_years(expiry)]
        } else {
            monitoring_dates
                .iter()
                .map(|&d| self.date_to_years(d))
                .collect()
        };

        for &t in &observation_times {
            builder = builder.add_observation_time(t);
        }

        // Add constants
        let barrier_idx = builder.add_constant(barrier_level);
        let strike_idx = builder.add_constant(strike);
        let notional_idx = builder.add_constant(notional);
        let gearing_idx = builder.add_constant(1.0);
        let spread_idx = builder.add_constant(0.0);

        // Add barrier check operations for each monitoring date
        for _ in &observation_times {
            builder = builder.push_op(ScriptOp::CheckBarrier {
                barrier_idx,
                barrier_type,
            });
        }

        // Get underlying value at expiry
        builder = builder.push_op(ScriptOp::CalcFloat {
            index_id: underlying_index,
            gearing_idx,
            spread_idx,
        });

        // Apply payoff
        builder = builder.push_op(ScriptOp::ApplyPayoff {
            strike_idx,
            is_call,
        });

        // Apply notional
        builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });

        // Pay
        builder = builder.push_op(ScriptOp::Pay {
            ccy_id: currency_id,
            dc_id,
        });

        builder.build()
    }

    /// Compiles an Asian option into ScriptKernel.
    ///
    /// Creates an arithmetic averaging option based on discrete observations.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `observation_dates` - Dates for averaging observations
    /// * `payment_date` - Payment date
    /// * `strike` - Strike price (as fraction of average)
    /// * `notional` - Notional amount
    /// * `is_call` - True for call, false for put
    /// * `underlying_index` - Index ID for the underlying
    /// * `currency` - Payment currency
    ///
    /// # Errors
    ///
    /// Returns error if observation_dates is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_asian_option(
        &mut self,
        trade_id: &str,
        observation_dates: &[Date],
        payment_date: Date,
        strike: f64,
        notional: f64,
        is_call: bool,
        underlying_index: u16,
        currency: Currency,
    ) -> Result<ScriptKernel, CompileError> {
        if observation_dates.is_empty() {
            return Err(CompileError::EmptyTrade(
                "Asian option requires at least one observation date".to_string(),
            ));
        }

        let currency_id = self.mapper.get_or_register_currency(currency);
        let dc_id = self.mapper.get_or_register_discount_curve(currency.code());

        let mut builder = ScriptKernelBuilder::new().trade_id(trade_id);

        // Add observation times
        for &date in observation_dates {
            builder = builder.add_observation_time(self.date_to_years(date));
        }

        // Add payment date as final observation
        builder = builder.add_observation_time(self.date_to_years(payment_date));

        // Add constants
        let strike_idx = builder.add_constant(strike);
        let notional_idx = builder.add_constant(notional);
        let gearing_idx = builder.add_constant(1.0);
        let spread_idx = builder.add_constant(0.0);

        // Accumulate observations
        for _ in observation_dates {
            builder = builder.push_op(ScriptOp::CalcFloat {
                index_id: underlying_index,
                gearing_idx,
                spread_idx,
            });
            builder = builder.push_op(ScriptOp::Accumulate);
        }

        // Calculate average
        builder = builder.push_op(ScriptOp::CalcAverage);

        // Apply payoff
        builder = builder.push_op(ScriptOp::ApplyPayoff {
            strike_idx,
            is_call,
        });

        // Apply notional
        builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });

        // Pay
        builder = builder.push_op(ScriptOp::Pay {
            ccy_id: currency_id,
            dc_id,
        });

        builder.build()
    }

    /// Compiles a vanilla European option into ScriptKernel.
    ///
    /// This is a simplified case that can be handled by ScriptEngine
    /// for consistency (though PricingKernel is more efficient).
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Trade identifier
    /// * `expiry` - Option expiry date
    /// * `strike` - Strike price
    /// * `notional` - Notional amount
    /// * `is_call` - True for call, false for put
    /// * `underlying_index` - Index ID for the underlying
    /// * `currency` - Payment currency
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_vanilla_option(
        &mut self,
        trade_id: &str,
        expiry: Date,
        strike: f64,
        notional: f64,
        is_call: bool,
        underlying_index: u16,
        currency: Currency,
    ) -> Result<ScriptKernel, CompileError> {
        let currency_id = self.mapper.get_or_register_currency(currency);
        let dc_id = self.mapper.get_or_register_discount_curve(currency.code());

        let mut builder = ScriptKernelBuilder::new().trade_id(trade_id);

        // Single observation at expiry
        builder = builder.add_observation_time(self.date_to_years(expiry));

        // Add constants
        let strike_idx = builder.add_constant(strike);
        let notional_idx = builder.add_constant(notional);
        let gearing_idx = builder.add_constant(1.0);
        let spread_idx = builder.add_constant(0.0);

        // Get underlying value
        builder = builder.push_op(ScriptOp::CalcFloat {
            index_id: underlying_index,
            gearing_idx,
            spread_idx,
        });

        // Apply payoff
        builder = builder.push_op(ScriptOp::ApplyPayoff {
            strike_idx,
            is_call,
        });

        // Apply notional
        builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });

        // Pay
        builder = builder.push_op(ScriptOp::Pay {
            ccy_id: currency_id,
            dc_id,
        });

        builder.build()
    }

    /// Compiles a generic `ScriptProduct` into a `ScriptKernel`.
    ///
    /// Dispatches to the appropriate compilation method based on
    /// the product type.
    ///
    /// # Arguments
    ///
    /// * `product` - The script product configuration to compile
    ///
    /// # Errors
    ///
    /// Returns error if the product configuration is invalid.
    pub fn compile_script_product(
        &mut self,
        product: &ScriptProduct,
    ) -> Result<ScriptKernel, CompileError> {
        match product.product_type {
            ScriptProductType::Tarf => self.compile_tarf(product),
            ScriptProductType::Autocallable => self.compile_autocallable(product),
            ScriptProductType::AccumulatorForward => self.compile_accumulator(product),
            ScriptProductType::SnowballNote => self.compile_snowball(product),
        }
    }

    /// Compiles a TARF product into a `ScriptKernel`.
    ///
    /// Each observation generates the following op sequence:
    /// 1. `Accumulate` - observe spot
    /// 2. `CalcAverage` - set current_value to observed spot
    /// 3. `ApplyPayoff` - apply call payoff (spot - strike)+
    /// 4. `ApplyNotional` - scale by notional per fixing
    /// 5. `Accumulate` - track P&L in accumulated_sum
    /// 6. `CheckTarget` - terminate if target reached
    /// 7. `PayIntermediate` - discount at current observation time
    fn compile_tarf(&mut self, product: &ScriptProduct) -> Result<ScriptKernel, CompileError> {
        if product.observations.is_empty() {
            return Err(CompileError::EmptyTrade(
                "TARF requires at least one observation".to_string(),
            ));
        }

        let target_config = product.target.as_ref().ok_or_else(|| {
            CompileError::EmptyTrade("TARF requires target configuration".to_string())
        })?;

        let ccy_id = product.currency_id;
        let dc_id = product.discount_curve_id;

        let mut builder = ScriptKernelBuilder::new().trade_id(&product.trade_id);

        // Add all observation times
        for obs in &product.observations {
            builder = builder.add_observation_time(obs.time);
        }

        // Add constants: collect unique values needed
        let target_idx = builder.add_constant(target_config.target_level);

        // Generate ops per observation
        for obs in &product.observations {
            if let ObservationAction::TarfAccrual {
                strike,
                notional_per_fixing,
                ..
            } = &obs.action
            {
                let strike_idx = builder.add_constant(*strike);
                let notional_idx = builder.add_constant(*notional_per_fixing);

                // Observe spot price
                builder = builder.push_op(ScriptOp::Accumulate);
                // Set current_value = average (i.e., latest spot since we reset)
                builder = builder.push_op(ScriptOp::CalcAverage);
                // Apply call payoff: max(spot - strike, 0)
                builder = builder.push_op(ScriptOp::ApplyPayoff {
                    strike_idx,
                    is_call: true,
                });
                // Scale by notional per fixing
                builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });
                // Track P&L in accumulated_sum
                builder = builder.push_op(ScriptOp::Accumulate);
                // Check if target reached
                builder = builder.push_op(ScriptOp::CheckTarget {
                    target_idx,
                    terminate_above: true,
                });
                // Pay at current observation time
                builder = builder.push_op(ScriptOp::PayIntermediate { ccy_id, dc_id });
            }
        }

        builder.build()
    }

    /// Compiles an Autocallable product into a `ScriptKernel`.
    ///
    /// Per observation (except final):
    /// 1. `CheckBarrier(UpIn)` - check autocall barrier
    /// 2. `CalcFixed` - coupon + principal amount
    /// 3. `PayIntermediate` - pay at current observation time
    /// 4. `EarlyTerminate` - terminate the product
    /// 5. `EndIf` - end of conditional block
    ///
    /// Final observation adds downside protection if configured.
    fn compile_autocallable(
        &mut self,
        product: &ScriptProduct,
    ) -> Result<ScriptKernel, CompileError> {
        if product.observations.is_empty() {
            return Err(CompileError::EmptyTrade(
                "Autocallable requires at least one observation".to_string(),
            ));
        }

        let ccy_id = product.currency_id;
        let dc_id = product.discount_curve_id;

        let mut builder = ScriptKernelBuilder::new().trade_id(&product.trade_id);

        // Add all observation times
        for obs in &product.observations {
            builder = builder.add_observation_time(obs.time);
        }

        // Generate ops per observation
        for obs in &product.observations {
            if let ObservationAction::AutocallCheck {
                barrier_level,
                coupon_amount,
                principal_return,
            } = &obs.action
            {
                let barrier_idx = builder.add_constant(*barrier_level);
                let payment_amount = *coupon_amount + *principal_return;
                let payment_idx = builder.add_constant(payment_amount);

                // Check autocall barrier (UpIn: spot >= barrier triggers)
                builder = builder.push_op(ScriptOp::CheckBarrier {
                    barrier_idx,
                    barrier_type: BarrierType::UpIn,
                });
                // Set current_value to coupon + principal
                builder = builder.push_op(ScriptOp::CalcFixed {
                    amount_idx: payment_idx,
                });
                // Pay at current observation time
                builder = builder.push_op(ScriptOp::PayIntermediate { ccy_id, dc_id });
                // Terminate the product
                builder = builder.push_op(ScriptOp::EarlyTerminate);
                // End conditional block
                builder = builder.push_op(ScriptOp::EndIf);
            }
        }

        // Add downside protection at final observation if configured
        if let Some(downside) = &product.downside {
            let ki_barrier_idx = builder.add_constant(downside.barrier_level);
            let put_strike_idx = builder.add_constant(downside.put_strike);
            let notional_idx = builder.add_constant(product.notional);

            // Check knock-in barrier
            builder = builder.push_op(ScriptOp::CheckBarrier {
                barrier_idx: ki_barrier_idx,
                barrier_type: downside.barrier_type,
            });
            // Apply put payoff: max(strike - spot, 0)
            builder = builder.push_op(ScriptOp::ApplyPayoff {
                strike_idx: put_strike_idx,
                is_call: false,
            });
            // Scale by notional
            builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });
            // Pay at maturity
            builder = builder.push_op(ScriptOp::Pay { ccy_id, dc_id });
        }

        builder.build()
    }

    /// Compiles an Accumulator Forward product into a `ScriptKernel`.
    ///
    /// Per observation:
    /// 1. `AccumulateQuantity` - track accumulated quantity
    /// 2. `CalcFloat` - observe spot price
    /// 3. `ApplyPayoff` - apply call payoff (spot - strike)+
    /// 4. `ApplyNotional` - scale by quantity per fixing
    /// 5. `Accumulate` - track P&L in accumulated_sum
    /// 6. `CheckTarget` - terminate if target quantity reached
    /// 7. `PayIntermediate` - discount at current observation time
    fn compile_accumulator(
        &mut self,
        product: &ScriptProduct,
    ) -> Result<ScriptKernel, CompileError> {
        if product.observations.is_empty() {
            return Err(CompileError::EmptyTrade(
                "Accumulator requires at least one observation".to_string(),
            ));
        }

        let ccy_id = product.currency_id;
        let dc_id = product.discount_curve_id;

        let mut builder = ScriptKernelBuilder::new().trade_id(&product.trade_id);

        // Add all observation times
        for obs in &product.observations {
            builder = builder.add_observation_time(obs.time);
        }

        // Add target constant
        let target_level = product
            .target
            .as_ref()
            .map(|t| t.target_level)
            .unwrap_or(f64::MAX);
        let target_idx = builder.add_constant(target_level);

        // Generate ops per observation
        for obs in &product.observations {
            if let ObservationAction::AccumulatorFixing {
                strike,
                quantity_per_fixing,
            } = &obs.action
            {
                let strike_idx = builder.add_constant(*strike);
                let qty_idx = builder.add_constant(*quantity_per_fixing);
                let gearing_idx = builder.add_constant(1.0);
                let spread_idx = builder.add_constant(0.0);

                // Track accumulated quantity
                builder = builder.push_op(ScriptOp::AccumulateQuantity {
                    quantity_idx: qty_idx,
                });
                // Observe spot price
                builder = builder.push_op(ScriptOp::CalcFloat {
                    index_id: product.underlying_index,
                    gearing_idx,
                    spread_idx,
                });
                // Apply call payoff: max(spot - strike, 0)
                builder = builder.push_op(ScriptOp::ApplyPayoff {
                    strike_idx,
                    is_call: true,
                });
                // Scale by quantity per fixing
                builder = builder.push_op(ScriptOp::ApplyNotional {
                    notional_idx: qty_idx,
                });
                // Track P&L in accumulated_sum
                builder = builder.push_op(ScriptOp::Accumulate);
                // Check if target quantity reached
                builder = builder.push_op(ScriptOp::CheckTarget {
                    target_idx,
                    terminate_above: true,
                });
                // Pay at current observation time
                builder = builder.push_op(ScriptOp::PayIntermediate { ccy_id, dc_id });
            }
        }

        builder.build()
    }

    /// Compiles a Snowball Note product into a `ScriptKernel`.
    ///
    /// Per observation (except final):
    /// 1. `CouponMemory` - accumulate memory coupon
    /// 2. `CheckBarrier(UpIn)` - check autocall barrier
    /// 3. `PayIntermediate` - pay accumulated memory coupons
    /// 4. `EarlyTerminate` - terminate the product
    /// 5. `EndIf` - end of conditional block
    ///
    /// Final observation adds put payoff if configured.
    fn compile_snowball(&mut self, product: &ScriptProduct) -> Result<ScriptKernel, CompileError> {
        if product.observations.is_empty() {
            return Err(CompileError::EmptyTrade(
                "Snowball requires at least one observation".to_string(),
            ));
        }

        let ccy_id = product.currency_id;
        let dc_id = product.discount_curve_id;

        let mut builder = ScriptKernelBuilder::new().trade_id(&product.trade_id);

        // Add all observation times
        for obs in &product.observations {
            builder = builder.add_observation_time(obs.time);
        }

        // Generate ops per observation
        for obs in &product.observations {
            match &obs.action {
                ObservationAction::SnowballCoupon {
                    coupon_amount,
                    barrier_level,
                } => {
                    let coupon_idx = builder.add_constant(*coupon_amount);
                    let barrier_idx = builder.add_constant(*barrier_level);

                    // Accumulate memory coupon
                    builder = builder.push_op(ScriptOp::CouponMemory { coupon_idx });
                    // Check autocall barrier (UpIn: spot >= barrier triggers)
                    builder = builder.push_op(ScriptOp::CheckBarrier {
                        barrier_idx,
                        barrier_type: BarrierType::UpIn,
                    });
                    // Pay accumulated memory coupons at current observation time
                    builder = builder.push_op(ScriptOp::PayIntermediate { ccy_id, dc_id });
                    // Terminate the product
                    builder = builder.push_op(ScriptOp::EarlyTerminate);
                    // End conditional block
                    builder = builder.push_op(ScriptOp::EndIf);
                }
                ObservationAction::FinalPayoff {
                    strike,
                    is_call,
                    notional,
                } => {
                    let strike_idx = builder.add_constant(*strike);
                    let notional_idx = builder.add_constant(*notional);
                    let gearing_idx = builder.add_constant(1.0);
                    let spread_idx = builder.add_constant(0.0);

                    // Get underlying value at maturity
                    builder = builder.push_op(ScriptOp::CalcFloat {
                        index_id: product.underlying_index,
                        gearing_idx,
                        spread_idx,
                    });
                    // Apply payoff
                    builder = builder.push_op(ScriptOp::ApplyPayoff {
                        strike_idx,
                        is_call: *is_call,
                    });
                    // Scale by notional
                    builder = builder.push_op(ScriptOp::ApplyNotional { notional_idx });
                    // Pay at maturity
                    builder = builder.push_op(ScriptOp::Pay { ccy_id, dc_id });
                }
                _ => {}
            }
        }

        builder.build()
    }

    /// Determines if a payoff requires ScriptKernel compilation.
    ///
    /// Returns true for path-dependent payoffs (barriers, Asians).
    /// Returns false for linear payoffs that should use PricingKernel.
    #[must_use]
    pub fn requires_script_kernel(payoff: &Payoff) -> bool {
        !matches!(payoff, Payoff::Fixed { .. } | Payoff::Linear { .. })
    }
}

#[cfg(test)]
mod tests {
    use infra_domain::market::Currency;
    use pricer_core::kernel::BarrierType;

    use super::*;

    fn test_date(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd(year, month, day).unwrap()
    }

    #[test]
    fn test_exotic_compiler_new() {
        let mapper = IndexMapper::new();
        let compiler = ExoticCompiler::new(mapper);

        assert_eq!(compiler.valuation_date_days, 0);
    }

    #[test]
    fn test_exotic_compiler_with_valuation_date() {
        let mapper = IndexMapper::new();
        let val_date = test_date(2025, 1, 1);
        let compiler = ExoticCompiler::new(mapper).with_valuation_date(val_date);

        // 2025-01-01 is 20089 days from Unix epoch (approximately)
        assert!(compiler.valuation_date_days > 0);
    }

    #[test]
    fn test_compile_barrier_option_up_out() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        let kernel = compiler
            .compile_barrier_option(
                "BARRIER001",
                test_date(2025, 12, 31),
                1.10, // 110% barrier
                1.0,  // ATM strike
                1_000_000.0,
                BarrierType::UpOut,
                true, // call
                1,    // underlying index
                Currency::USD,
                &[], // no monitoring dates (discrete at expiry)
            )
            .expect("Valid barrier option");

        assert!(kernel.has_barriers());
        assert!(!kernel.has_accumulation());
        assert_eq!(kernel.trade_id, "BARRIER001");
        assert_eq!(kernel.observation_count(), 1);
    }

    #[test]
    fn test_compile_barrier_option_with_monitoring() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        // Quarterly monitoring
        let monitoring_dates = vec![
            test_date(2025, 3, 31),
            test_date(2025, 6, 30),
            test_date(2025, 9, 30),
            test_date(2025, 12, 31),
        ];

        let kernel = compiler
            .compile_barrier_option(
                "BARRIER002",
                test_date(2025, 12, 31),
                0.85, // 85% barrier (down)
                1.0,  // ATM strike
                1_000_000.0,
                BarrierType::DownIn,
                false, // put
                1,
                Currency::EUR,
                &monitoring_dates,
            )
            .expect("Valid barrier option");

        assert!(kernel.has_barriers());
        assert_eq!(kernel.observation_count(), 4);
    }

    #[test]
    fn test_compile_asian_option() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        let observation_dates = vec![
            test_date(2025, 3, 31),
            test_date(2025, 6, 30),
            test_date(2025, 9, 30),
            test_date(2025, 12, 31),
        ];

        let kernel = compiler
            .compile_asian_option(
                "ASIAN001",
                &observation_dates,
                test_date(2026, 1, 15), // payment date
                1.0,                    // ATM strike
                1_000_000.0,
                true, // call
                1,
                Currency::USD,
            )
            .expect("Valid Asian option");

        assert!(kernel.has_accumulation());
        assert!(!kernel.has_barriers());
        assert_eq!(kernel.trade_id, "ASIAN001");
        // 4 observations + 1 payment date
        assert_eq!(kernel.observation_count(), 5);
    }

    #[test]
    fn test_compile_asian_option_empty_dates() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let result = compiler.compile_asian_option(
            "ASIAN002",
            &[], // empty
            test_date(2025, 12, 31),
            1.0,
            1_000_000.0,
            true,
            1,
            Currency::USD,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_compile_vanilla_option() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        let kernel = compiler
            .compile_vanilla_option(
                "VANILLA001",
                test_date(2025, 12, 31),
                100.0,       // strike
                1_000_000.0, // notional
                true,        // call
                1,           // underlying
                Currency::USD,
            )
            .expect("Valid vanilla option");

        assert!(!kernel.has_barriers());
        assert!(!kernel.has_accumulation());
        assert_eq!(kernel.observation_count(), 1);
    }

    #[test]
    fn test_currency_registration() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let _ = compiler
            .compile_vanilla_option(
                "TEST001",
                test_date(2025, 12, 31),
                100.0,
                1_000_000.0,
                true,
                1,
                Currency::EUR,
            )
            .expect("Valid option");

        // EUR should be registered
        assert!(compiler.mapper().get_currency_id(Currency::EUR).is_some());
        // EUR discount curve should be registered
        assert!(compiler.mapper().get_discount_curve_id("EUR").is_some());
    }

    #[test]
    fn test_requires_script_kernel() {
        use infra_domain::trade::{IndexType, OptionType};

        assert!(!ExoticCompiler::requires_script_kernel(&Payoff::fixed(
            0.05
        )));
        assert!(!ExoticCompiler::requires_script_kernel(&Payoff::floating(
            IndexType::Rate(infra_domain::market::RateIndex::Sofr)
        )));

        // VanillaOption requires ScriptKernel
        let vanilla = Payoff::VanillaOption {
            index: IndexType::Rate(infra_domain::market::RateIndex::Sofr),
            strike: 0.05,
            option_type: OptionType::Call,
        };
        assert!(ExoticCompiler::requires_script_kernel(&vanilla));
    }

    #[test]
    fn test_barrier_types() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        for barrier_type in [
            BarrierType::UpIn,
            BarrierType::UpOut,
            BarrierType::DownIn,
            BarrierType::DownOut,
        ] {
            let kernel = compiler
                .compile_barrier_option(
                    &format!("BARRIER_{:?}", barrier_type),
                    test_date(2025, 12, 31),
                    1.10,
                    1.0,
                    1_000_000.0,
                    barrier_type,
                    true,
                    1,
                    Currency::USD,
                    &[],
                )
                .expect("Valid barrier option");

            assert!(kernel.has_barriers());
        }
    }

    #[test]
    fn test_exotic_compiler_debug() {
        let mapper = IndexMapper::new();
        let compiler = ExoticCompiler::new(mapper);

        let debug_str = format!("{:?}", compiler);
        assert!(debug_str.contains("ExoticCompiler"));
    }

    #[test]
    fn test_date_to_years() {
        let mapper = IndexMapper::new();
        let val_date = test_date(2025, 1, 1);
        let compiler = ExoticCompiler::new(mapper).with_valuation_date(val_date);

        // Test 1 year later
        let expiry = test_date(2026, 1, 1);
        let years = compiler.date_to_years(expiry);
        assert!((years - 1.0).abs() < 0.01, "Should be approximately 1 year");

        // Test 6 months later
        let mid_year = test_date(2025, 7, 1);
        let years = compiler.date_to_years(mid_year);
        assert!(
            (years - 0.5).abs() < 0.05,
            "Should be approximately 0.5 years"
        );
    }

    #[test]
    fn test_asian_monthly_observations() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper).with_valuation_date(test_date(2025, 1, 1));

        // Monthly observations for a year
        let observation_dates: Vec<Date> =
            (1..=12).map(|month| test_date(2025, month, 15)).collect();

        let kernel = compiler
            .compile_asian_option(
                "ASIAN_MONTHLY",
                &observation_dates,
                test_date(2025, 12, 31),
                1.0,
                1_000_000.0,
                true,
                1,
                Currency::USD,
            )
            .expect("Valid Asian option");

        // 12 observations + 1 payment date
        assert_eq!(kernel.observation_count(), 13);

        // Should have 12 accumulate operations
        let accum_count = kernel
            .ops
            .iter()
            .filter(|op| matches!(op, ScriptOp::Accumulate))
            .count();
        assert_eq!(accum_count, 12);
    }

    #[test]
    fn test_compile_script_product_tarf() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct::tarf(
            "TARF_SCRIPT",
            100.0,
            1_000_000.0,
            50_000.0,
            vec![0.25, 0.5, 0.75, 1.0],
        );

        let kernel = compiler
            .compile_script_product(&product)
            .expect("Valid TARF product");

        assert_eq!(kernel.trade_id, "TARF_SCRIPT");
        assert_eq!(kernel.observation_count(), 4);
        assert!(kernel.has_accumulation());
    }

    #[test]
    fn test_compile_script_product_autocallable() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct::autocallable(
            "AUTO_SCRIPT",
            1_000_000.0,
            105.0,
            0.10,
            vec![0.25, 0.5, 0.75, 1.0],
            70.0,
            100.0,
        );

        let kernel = compiler
            .compile_script_product(&product)
            .expect("Valid Autocallable product");

        assert_eq!(kernel.trade_id, "AUTO_SCRIPT");
        assert_eq!(kernel.observation_count(), 4);
        assert!(kernel.has_barriers());
    }

    #[test]
    fn test_compile_script_product_accumulator() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct::accumulator(
            "ACCUM_SCRIPT",
            100.0,
            1_000.0,
            50_000.0,
            vec![0.25, 0.5, 0.75, 1.0],
        );

        let kernel = compiler
            .compile_script_product(&product)
            .expect("Valid Accumulator product");

        assert_eq!(kernel.trade_id, "ACCUM_SCRIPT");
        assert_eq!(kernel.observation_count(), 4);
        assert!(kernel.has_accumulation());

        // Should have 4 AccumulateQuantity operations
        let qty_count = kernel
            .ops
            .iter()
            .filter(|op| matches!(op, ScriptOp::AccumulateQuantity { .. }))
            .count();
        assert_eq!(qty_count, 4);
    }

    #[test]
    fn test_compile_script_product_snowball() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct::snowball(
            "SNOW_SCRIPT",
            1_000_000.0,
            10_000.0,
            105.0,
            vec![0.25, 0.5, 0.75, 1.0],
            90.0,
        );

        let kernel = compiler
            .compile_script_product(&product)
            .expect("Valid Snowball product");

        assert_eq!(kernel.trade_id, "SNOW_SCRIPT");
        assert!(kernel.has_barriers());

        // Should have 4 CouponMemory operations
        let memory_count = kernel
            .ops
            .iter()
            .filter(|op| matches!(op, ScriptOp::CouponMemory { .. }))
            .count();
        assert_eq!(memory_count, 4);
    }

    #[test]
    fn test_compile_accumulator_empty_observations() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct {
            product_type: ScriptProductType::AccumulatorForward,
            trade_id: "ACCUM_EMPTY".to_string(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: 1_000.0,
            observations: vec![],
            target: None,
            downside: None,
            memory_coupon: None,
        };

        let result = compiler.compile_script_product(&product);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_snowball_empty_observations() {
        let mapper = IndexMapper::new();
        let mut compiler = ExoticCompiler::new(mapper);

        let product = ScriptProduct {
            product_type: ScriptProductType::SnowballNote,
            trade_id: "SNOW_EMPTY".to_string(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: 1_000_000.0,
            observations: vec![],
            target: None,
            downside: None,
            memory_coupon: None,
        };

        let result = compiler.compile_script_product(&product);
        assert!(result.is_err());
    }
}

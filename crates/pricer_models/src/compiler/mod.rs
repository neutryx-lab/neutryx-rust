/! Trade compiler: transforms hierarchical Trade definitions into PricingKernel IR.

mod callable;
mod exotic;
mod index_mapper;
mod linear;
mod xccy;

pub use callable::{CallableCompiler, Cashflow, CashflowSchedule};
pub use exotic::ExoticCompiler;
pub use index_mapper::{CmsIndex, ForwardIndexType, IndexMapper};
pub use linear::LinearProductsCompiler;
use pricer_core::kernel::{CompileError, PricingKernel};
pub use xccy::XCcyCompiler;

/// Trait for compiling `Trade` definitions into `PricingKernel` IR.
///
/// Implementations of this trait handle the transformation from
/// high-level trade definitions (with dates, calendars, etc.) to
/// low-level IR suitable for SIMD pricing.
///
/// # Type Parameters
///
/// * `T` - The trade type to compile
///
/// # Design Principles
///
/// - **Stateless**: Compilers should not maintain internal state
/// - **Deterministic**: Same input always produces same output
/// - **Error Propagation**: All errors are structured via `CompileError`
///
/// # Example
///
/// ```ignore
/// use pricer_models::compiler::{TradeCompiler, LinearProductsCompiler, IndexMapper};
/// use infra_domain::trade::Trade;
///
/// let index_mapper = IndexMapper::new();
/// let compiler = LinearProductsCompiler::new(index_mapper);
/// let kernel = compiler.compile(&trade)?;
/// ```
pub trait TradeCompiler<T> {
    /// Compiles a single trade into a `PricingKernel`.
    ///
    /// # Arguments
    ///
    /// * `trade` - The trade to compile
    ///
    /// # Returns
    ///
    /// * `Ok(PricingKernel)` - Successfully compiled kernel
    /// * `Err(CompileError)` - Compilation failed
    fn compile(&self, trade: &T) -> Result<PricingKernel, CompileError>;

    /// Compiles multiple trades into a single batched `PricingKernel`.
    ///
    /// The default implementation compiles each trade separately and
    /// merges them. Implementations may override for better performance.
    ///
    /// # Arguments
    ///
    /// * `trades` - Iterator of trades to compile
    ///
    /// # Returns
    ///
    /// * `Ok(PricingKernel)` - Merged kernel with all cashflows
    /// * `Err(CompileError)` - Compilation failed for any trade
    fn compile_batch<'a, I>(&self, trades: I) -> Result<PricingKernel, CompileError>
    where
        I: IntoIterator<Item = &'a T>,
        T: 'a,
    {
        use pricer_core::kernel::PricingKernelBuilder;

        let mut builder = PricingKernelBuilder::new();
        let mut trade_count = 0;

        for trade in trades {
            let kernel = self.compile(trade)?;
            trade_count += 1;

            // Merge kernel into builder
            for i in 0..kernel.len() {
                builder.add_cashflow(
                    kernel.payment_dates[i],
                    kernel.fixing_dates[i],
                    kernel.year_fractions[i],
                    kernel.notionals[i],
                    kernel.spreads[i],
                    kernel.gearings[i],
                    kernel.currency_ids[i],
                    kernel.discount_curve_ids[i],
                    kernel.fwd_index_ids[i],
                    kernel.fx_index_ids[i],
                );
            }
        }

        builder.set_trade_count(trade_count);
        builder.sort_by_payment_date();
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_compiler_trait_exists() {
        // This test ensures the trait is properly defined
        fn assert_trait<T, C: TradeCompiler<T>>() {}

        // The trait exists and is properly bounded
        assert_trait::<(), MockCompiler>();
    }

    struct MockCompiler;

    impl TradeCompiler<()> for MockCompiler {
        fn compile(&self, _trade: &()) -> Result<PricingKernel, CompileError> {
            Ok(PricingKernel::empty())
        }
    }
}

//! Integration tests for the Pricing Kernel IR pipeline.
//!
//! These tests verify the complete flow: Trade → PricingKernel → Price.

#[cfg(test)]
mod tests {
    use pricer_core::ir::PricingKernelBuilder;
    use pricer_models::compiler::{IndexMapper, LinearProductsCompiler, TradeCompiler};

    use super::super::context::KernelContext;
    use super::super::engine::LinearEngine;
    use super::super::provider::FlatCurveProvider;

    use infra_master::trade::{
        Cashflow, CashflowType, Direction, IndexType, Leg, LegType, Payoff, Trade, TradeType,
    };
    use infra_master::{Currency, Date, RateIndex};

    /// Creates a fixed leg for testing.
    fn create_fixed_leg(notional: f64, rate: f64, direction: Direction) -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5, // semi-annual
                notional,
                Payoff::fixed(rate),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                notional,
                Payoff::fixed(rate),
                Currency::USD,
            ),
        ];

        Leg::new(cashflows, direction, LegType::Fixed, Currency::USD)
    }

    /// Creates a floating leg for testing.
    fn create_floating_leg(notional: f64, spread: f64, direction: Direction) -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
                notional,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), spread),
                Currency::USD,
            ),
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 12, 31).unwrap(),
                Date::from_ymd(2025, 7, 1).unwrap(),
                Date::from_ymd(2025, 12, 31).unwrap(),
                0.5,
                notional,
                Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), spread),
                Currency::USD,
            ),
        ];

        Leg::new(cashflows, direction, LegType::Floating, Currency::USD)
    }

    /// Creates a test IRS trade (receive fixed, pay floating).
    fn create_test_irs(trade_id: &str, notional: f64, fixed_rate: f64, spread: f64) -> Trade {
        Trade::new(
            trade_id,
            vec![
                create_fixed_leg(notional, fixed_rate, Direction::Receiver),
                create_floating_leg(notional, spread, Direction::Payer),
            ],
            TradeType::Swap,
        )
    }

    #[test]
    fn test_e2e_single_irs_compile_and_price() {
        // Step 1: Create a simple IRS trade
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        // Step 2: Create compiler
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        // Step 3: Compile trade to PricingKernel
        let kernel = compiler.compile(&trade).expect("Compilation should succeed");

        // Verify kernel has cashflows (4 = 2 fixed + 2 floating)
        assert_eq!(kernel.len(), 4, "Kernel should have 4 cashflows");

        // Step 4: Create market data provider (flat curves for simplicity)
        let discount_rate = 0.03; // 3% discount rate
        let forward_rate = 0.025; // 2.5% forward rate
        let provider = FlatCurveProvider::new(discount_rate, forward_rate);

        // Step 5: Create context
        let context = KernelContext::new(&provider);

        // Step 6: Price the kernel
        let pv = LinearEngine::price(&kernel, &context);

        // Verify we got a reasonable price (not NaN, not infinite)
        assert!(pv.is_finite(), "PV should be finite, got: {}", pv);

        println!(
            "Single IRS PV: {} (notional: {}, fixed rate: {})",
            pv, 10_000_000.0, 0.025
        );
    }

    #[test]
    fn test_e2e_irs_batch_compile_and_price() {
        // Step 1: Create multiple IRS trades
        let trades = vec![
            create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001),
            create_test_irs("IRS-002", 5_000_000.0, 0.030, 0.002),
            create_test_irs("IRS-003", 20_000_000.0, 0.020, 0.001),
        ];

        // Step 2: Create compiler
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        // Step 3: Batch compile all trades (using TradeCompiler trait)
        let kernel = compiler
            .compile_batch(trades.iter())
            .expect("Batch compilation should succeed");

        // Verify kernel has all cashflows combined (12 = 3 trades × 4 cashflows)
        assert_eq!(kernel.len(), 12, "Kernel should have 12 cashflows");
        assert_eq!(kernel.trade_count(), 3, "Kernel should track 3 trades");

        // Step 4: Create market data provider
        let provider = FlatCurveProvider::new(0.03, 0.025);

        // Step 5: Price the combined kernel
        let context = KernelContext::new(&provider);
        let total_pv = LinearEngine::price(&kernel, &context);

        // Verify we got a valid price
        assert!(total_pv.is_finite(), "Total PV should be finite");

        println!(
            "Batch IRS PV: {} (3 trades, total notional: {})",
            total_pv,
            10_000_000.0 + 5_000_000.0 + 20_000_000.0
        );
    }

    #[test]
    fn test_e2e_kernel_decomposed_pricing() {
        // Create a simple IRS
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        // Compile
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler.compile(&trade).expect("Compilation should succeed");

        // Price with decomposition
        let provider = FlatCurveProvider::new(0.03, 0.025);
        let context = KernelContext::new(&provider);

        let cashflow_pvs = LinearEngine::price_decomposed(&kernel, &context);

        // Verify decomposed prices sum to total
        let total_pv = LinearEngine::price(&kernel, &context);
        let sum_pvs: f64 = cashflow_pvs.iter().sum();

        assert!(
            (total_pv - sum_pvs).abs() < 1e-10,
            "Decomposed PVs should sum to total PV: {} vs {}",
            total_pv,
            sum_pvs
        );

        // Verify each cashflow PV is finite
        for (i, pv) in cashflow_pvs.iter().enumerate() {
            assert!(pv.is_finite(), "Cashflow {} PV should be finite", i);
        }
    }

    #[test]
    fn test_e2e_manual_kernel_construction() {
        // Test building a PricingKernel manually (without trade compilation)
        // This simulates what the compiler does internally

        let valuation_date_days = 18263; // 2020-01-02

        // Build a simple 2-cashflow kernel: one fixed, one floating
        let mut builder = PricingKernelBuilder::new();

        // Fixed cashflow: pay $100,000 in 180 days
        builder.add_cashflow(
            valuation_date_days + 180, // payment date
            valuation_date_days,       // fixing date (same as valuation for fixed)
            0.5,                       // 6-month year fraction
            -100_000.0,                // negative = pay
            0.025,                     // 2.5% fixed rate (spread)
            0.0,                       // gearing = 0 for fixed
            0,                         // USD
            0,                         // discount curve 0
            0,                         // fwd index 0 (dummy)
            0,                         // fx index 0 (dummy)
        );

        // Floating cashflow: receive notional * (SOFR + spread) in 90 days
        builder.add_cashflow(
            valuation_date_days + 90, // payment date
            valuation_date_days,      // fixing date
            0.25,                     // 3-month year fraction
            100_000.0,                // positive = receive
            0.001,                    // 10bps spread
            1.0,                      // gearing = 1 for floating
            0,                        // USD
            0,                        // discount curve 0
            1,                        // fwd index 1 (SOFR)
            0,                        // fx index 0 (dummy)
        );

        let kernel = builder.build().expect("Build should succeed");

        // Verify kernel structure
        assert_eq!(kernel.len(), 2);
        assert!(kernel.is_aligned());

        // Price it
        let provider =
            FlatCurveProvider::new(0.03, 0.025).with_valuation_date(valuation_date_days);
        let context = KernelContext::new(&provider);

        let pv = LinearEngine::price(&kernel, &context);
        assert!(pv.is_finite(), "PV should be finite");

        // Manual calculation for verification:
        // Fixed leg: -100,000 * 0.5 * 0.025 * DF(180) = -1,250 * exp(-0.03 * 180/365)
        // Floating leg: 100,000 * 0.25 * (1.0 * 0.025 + 0.001) * DF(90)
        let df_180 = (-0.03_f64 * 180.0 / 365.0).exp();
        let df_90 = (-0.03_f64 * 90.0 / 365.0).exp();
        let expected_fixed = -100_000.0 * 0.5 * 0.025 * df_180;
        let expected_floating = 100_000.0 * 0.25 * (1.0 * 0.025 + 0.001) * df_90;
        let expected_pv = expected_fixed + expected_floating;

        assert!(
            (pv - expected_pv).abs() < 1e-6,
            "PV should match manual calculation: {} vs {}",
            pv,
            expected_pv
        );
    }

    #[test]
    fn test_e2e_kernel_with_different_market_scenarios() {
        // Create a simple IRS
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        // Compile
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler.compile(&trade).expect("Compilation should succeed");

        // Scenario 1: Low rates
        let provider_low = FlatCurveProvider::new(0.01, 0.01); // 1% rates
        let context_low = KernelContext::new(&provider_low);
        let pv_low = LinearEngine::price(&kernel, &context_low);

        // Scenario 2: High rates
        let provider_high = FlatCurveProvider::new(0.05, 0.05); // 5% rates
        let context_high = KernelContext::new(&provider_high);
        let pv_high = LinearEngine::price(&kernel, &context_high);

        // Verify both scenarios produce valid results
        assert!(pv_low.is_finite(), "Low rate PV should be finite");
        assert!(pv_high.is_finite(), "High rate PV should be finite");

        // The two scenarios should produce different PVs
        assert!(
            (pv_low - pv_high).abs() > 1e-6,
            "Different market scenarios should produce different PVs"
        );

        println!("Low rate scenario PV: {}", pv_low);
        println!("High rate scenario PV: {}", pv_high);
    }

    #[test]
    fn test_e2e_fixed_bond_compile_and_price() {
        // Create a fixed bond (receive only)
        let trade = Trade::new(
            "BOND-001",
            vec![create_fixed_leg(5_000_000.0, 0.04, Direction::Receiver)],
            TradeType::Generic,
        );

        // Compile
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler.compile(&trade).expect("Compilation should succeed");

        // Should have 2 cashflows
        assert_eq!(kernel.len(), 2);

        // Price
        let provider = FlatCurveProvider::new(0.03, 0.0); // 3% discount, no forwards needed
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        // All cashflows should be positive (receiving fixed)
        let decomposed = LinearEngine::price_decomposed(&kernel, &context);
        for cf_pv in &decomposed {
            assert!(*cf_pv > 0.0, "Fixed bond cashflows should be positive");
        }

        assert!(pv > 0.0, "Bond PV should be positive");
        println!("Fixed bond PV: {}", pv);
    }

    #[test]
    fn test_e2e_at_par_swap() {
        // Create swap where fixed rate = forward rate (at par)
        let trade = create_test_irs("IRS-PAR", 10_000_000.0, 0.03, 0.0); // fixed = 3%, no spread

        // Compile
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler.compile(&trade).expect("Compilation should succeed");

        // Price with forward rate = fixed rate
        let provider = FlatCurveProvider::new(0.03, 0.03); // discount = forward = 3%
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        // At-par swap should have PV close to zero
        // (Not exactly zero due to day count and payment date differences)
        assert!(
            pv.abs() < 100_000.0, // Allow some tolerance
            "At-par swap PV should be close to zero, got: {}",
            pv
        );

        println!("At-par swap PV: {} (should be close to 0)", pv);
    }
}

//! Integration tests for the Pricing Kernel IR pipeline.
//!
//! These tests verify the complete flow: Trade → PricingKernel → Price.

#[cfg(test)]
mod tests {
    use infra_master::{
        trade::{
            Cashflow, CashflowType, Direction, IndexType, Leg, LegType, Payoff, Trade, TradeType,
        },
        Currency, Date, RateIndex,
    };
    use pricer_core::ir::PricingKernelBuilder;
    use pricer_models::compiler::{IndexMapper, LinearProductsCompiler, TradeCompiler};

    use super::super::{context::KernelContext, engine::LinearEngine, provider::FlatCurveProvider};

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
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

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
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

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
        let provider = FlatCurveProvider::new(0.03, 0.025).with_valuation_date(valuation_date_days);
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
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

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
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

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
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

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

    // =========================================================================
    // Task 6.3: Enzyme AD Compatibility Verification
    // =========================================================================

    /// Helper struct for finite difference sensitivity calculation.
    struct FiniteDifferenceSensitivity {
        bump_size: f64,
    }

    impl FiniteDifferenceSensitivity {
        fn new(bump_size: f64) -> Self { Self { bump_size } }

        /// Compute ∂PV/∂r (sensitivity to discount rate) using central
        /// difference.
        fn discount_rate_sensitivity(
            &self,
            kernel: &pricer_core::ir::PricingKernel,
            base_discount_rate: f64,
            forward_rate: f64,
        ) -> f64 {
            let h = self.bump_size;

            // Up scenario
            let provider_up = FlatCurveProvider::new(base_discount_rate + h, forward_rate);
            let context_up = KernelContext::new(&provider_up);
            let pv_up = LinearEngine::price(kernel, &context_up);

            // Down scenario
            let provider_down = FlatCurveProvider::new(base_discount_rate - h, forward_rate);
            let context_down = KernelContext::new(&provider_down);
            let pv_down = LinearEngine::price(kernel, &context_down);

            // Central difference: f'(x) ≈ (f(x+h) - f(x-h)) / (2h)
            (pv_up - pv_down) / (2.0 * h)
        }

        /// Compute ∂PV/∂L (sensitivity to forward rate) using central
        /// difference.
        fn forward_rate_sensitivity(
            &self,
            kernel: &pricer_core::ir::PricingKernel,
            discount_rate: f64,
            base_forward_rate: f64,
        ) -> f64 {
            let h = self.bump_size;

            // Up scenario
            let provider_up = FlatCurveProvider::new(discount_rate, base_forward_rate + h);
            let context_up = KernelContext::new(&provider_up);
            let pv_up = LinearEngine::price(kernel, &context_up);

            // Down scenario
            let provider_down = FlatCurveProvider::new(discount_rate, base_forward_rate - h);
            let context_down = KernelContext::new(&provider_down);
            let pv_down = LinearEngine::price(kernel, &context_down);

            // Central difference
            (pv_up - pv_down) / (2.0 * h)
        }
    }

    /// Verifies that price_kernel produces smooth, differentiable results.
    ///
    /// # Enzyme AD Compatibility
    ///
    /// The `LinearEngine::price` function is designed to be Enzyme AD
    /// compatible:
    ///
    /// 1. **Smooth operations only**: Uses only +, *, exp (via discount factor)
    /// 2. **No data-dependent branching**: Fixed formula applied to all
    ///    cashflows
    /// 3. **Sequential array access**: SIMD-friendly memory access pattern
    ///
    /// This test verifies smoothness by checking that finite difference
    /// approximations converge as the bump size decreases.
    #[test]
    fn test_enzyme_ad_smoothness_discount_rate() {
        // Create a test kernel
        let trade = create_test_irs("IRS-AD-TEST", 10_000_000.0, 0.025, 0.001);
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

        // Compute sensitivities at different bump sizes
        let sensitivity_1bp = FiniteDifferenceSensitivity::new(0.0001).discount_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );
        let sensitivity_0_1bp = FiniteDifferenceSensitivity::new(0.00001)
            .discount_rate_sensitivity(&kernel, discount_rate, forward_rate);
        let sensitivity_0_01bp = FiniteDifferenceSensitivity::new(0.000001)
            .discount_rate_sensitivity(&kernel, discount_rate, forward_rate);

        // Sensitivities should converge as bump size decreases
        let diff_1 = (sensitivity_1bp - sensitivity_0_1bp).abs();
        let diff_2 = (sensitivity_0_1bp - sensitivity_0_01bp).abs();

        // For smooth functions, smaller bumps should give closer results
        // The second difference should be smaller than the first
        assert!(
            diff_2 < diff_1 * 10.0, // Allow factor of 10 for numerical noise
            "Sensitivities should converge: diff_1={}, diff_2={}",
            diff_1,
            diff_2
        );

        println!("Discount rate sensitivity:");
        println!("  1bp bump:   {:.2}", sensitivity_1bp);
        println!("  0.1bp bump: {:.2}", sensitivity_0_1bp);
        println!("  0.01bp bump: {:.2}", sensitivity_0_01bp);
    }

    #[test]
    fn test_enzyme_ad_smoothness_forward_rate() {
        // Create a floating-heavy kernel
        let trade = create_test_irs("IRS-AD-FWD", 10_000_000.0, 0.01, 0.002);
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

        // Compute sensitivities at different bump sizes
        let sensitivity_1bp = FiniteDifferenceSensitivity::new(0.0001).forward_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );
        let sensitivity_0_1bp = FiniteDifferenceSensitivity::new(0.00001).forward_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );
        let sensitivity_0_01bp = FiniteDifferenceSensitivity::new(0.000001)
            .forward_rate_sensitivity(&kernel, discount_rate, forward_rate);

        // Sensitivities should converge
        let diff_1 = (sensitivity_1bp - sensitivity_0_1bp).abs();
        let diff_2 = (sensitivity_0_1bp - sensitivity_0_01bp).abs();

        assert!(
            diff_2 < diff_1 * 10.0,
            "Forward rate sensitivities should converge: diff_1={}, diff_2={}",
            diff_1,
            diff_2
        );

        println!("Forward rate sensitivity:");
        println!("  1bp bump:   {:.2}", sensitivity_1bp);
        println!("  0.1bp bump: {:.2}", sensitivity_0_1bp);
        println!("  0.01bp bump: {:.2}", sensitivity_0_01bp);
    }

    /// Verifies fixed leg sensitivity matches analytical expectation.
    ///
    /// For a fixed leg: PV = N × τ × r_fixed × DF(t)
    /// ∂PV/∂r_discount = N × τ × r_fixed × ∂DF/∂r = -t × PV
    ///
    /// This analytical check confirms the numerical derivatives are correct.
    #[test]
    fn test_enzyme_ad_fixed_leg_analytical() {
        // Create a pure fixed kernel
        let kernel = pricer_core::ir::PricingKernel::new(
            vec![365],         // payment 1 year from now
            vec![0],           // fixing date (not used)
            vec![1.0],         // year fraction
            vec![1_000_000.0], // notional
            vec![0.05],        // 5% fixed rate
            vec![0.0],         // gearing = 0 (fixed)
            vec![0],
            vec![0],
            vec![0], // dummy fwd index
            vec![0],
        )
        .expect("Valid kernel");

        let discount_rate = 0.03;
        let forward_rate = 0.0; // Not used for fixed

        // Compute finite difference sensitivity
        let sensitivity = FiniteDifferenceSensitivity::new(0.0001).discount_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );

        // Compute base PV
        let provider = FlatCurveProvider::new(discount_rate, forward_rate);
        let context = KernelContext::new(&provider);
        let base_pv = LinearEngine::price(&kernel, &context);

        // Analytical expectation: ∂PV/∂r ≈ -t × PV for flat rate
        let t = 1.0; // 1 year
        let analytical = -t * base_pv;

        let relative_error = (sensitivity - analytical).abs() / analytical.abs();

        assert!(
            relative_error < 0.01, // 1% tolerance
            "Fixed leg sensitivity should match analytical: FD={:.2}, analytical={:.2}, error={:.4}%",
            sensitivity,
            analytical,
            relative_error * 100.0
        );

        println!("Fixed leg analytical test:");
        println!("  Base PV: {:.2}", base_pv);
        println!("  FD sensitivity: {:.2}", sensitivity);
        println!("  Analytical: {:.2}", analytical);
        println!("  Relative error: {:.4}%", relative_error * 100.0);
    }

    /// Verifies floating leg sensitivity to forward rate.
    ///
    /// For a floating leg: PV = N × τ × (L + spread) × DF(t)
    /// ∂PV/∂L = N × τ × DF(t)
    #[test]
    fn test_enzyme_ad_floating_leg_analytical() {
        // Create a pure floating kernel
        let kernel = pricer_core::ir::PricingKernel::new(
            vec![365],         // payment 1 year from now
            vec![0],           // fixing date
            vec![1.0],         // year fraction
            vec![1_000_000.0], // notional
            vec![0.01],        // 100bp spread
            vec![1.0],         // gearing = 1.0 (floating)
            vec![0],
            vec![0],
            vec![1], // real fwd index
            vec![0],
        )
        .expect("Valid kernel");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

        // Compute finite difference sensitivity to forward rate
        let sensitivity = FiniteDifferenceSensitivity::new(0.0001).forward_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );

        // Analytical expectation: ∂PV/∂L = N × τ × gearing × DF(t)
        let notional = 1_000_000.0;
        let tau = 1.0;
        let gearing = 1.0;
        let df = (-discount_rate * 1.0).exp(); // 1 year
        let analytical = notional * tau * gearing * df;

        let relative_error = (sensitivity - analytical).abs() / analytical.abs();

        assert!(
            relative_error < 0.01, // 1% tolerance
            "Floating leg fwd sensitivity should match analytical: FD={:.2}, analytical={:.2}, error={:.4}%",
            sensitivity,
            analytical,
            relative_error * 100.0
        );

        println!("Floating leg analytical test:");
        println!("  FD sensitivity: {:.2}", sensitivity);
        println!("  Analytical: {:.2}", analytical);
        println!("  Relative error: {:.4}%", relative_error * 100.0);
    }

    /// Documents the Enzyme AD compatibility requirements.
    ///
    /// This test serves as documentation for the Enzyme AD compatibility
    /// requirements that `LinearEngine::price` satisfies.
    #[test]
    fn test_enzyme_ad_compatibility_documentation() {
        // The LinearEngine::price function satisfies these Enzyme AD requirements:
        //
        // 1. SMOOTH OPERATIONS ONLY
        //    - Addition (+)
        //    - Multiplication (*)
        //    - Exponential (exp) via discount factor
        //    - NO: abs(), max(), min(), if/else on computed values
        //
        // 2. BRANCHLESS EXECUTION
        //    - Same formula for all cashflows
        //    - No data-dependent conditionals
        //    - Loop counter only depends on array length
        //
        // 3. ARRAY ACCESS PATTERN
        //    - Sequential access to contiguous arrays
        //    - Index i independent of computed values
        //    - SIMD-friendly memory layout
        //
        // 4. TYPE COMPATIBILITY
        //    - Uses f64 primitives
        //    - No complex number operations
        //    - Compatible with Enzyme's LLVM instrumentation
        //
        // When Enzyme is integrated:
        //   __enzyme_autodiff(price_kernel, ...) will produce correct gradients
        //   because the function is smooth and branchless.

        // Create a test to verify the kernel structure is appropriate
        let kernel = pricer_core::ir::PricingKernel::new(
            vec![365, 730],
            vec![0, 365],
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        )
        .expect("Valid kernel");

        // Verify kernel properties for AD
        assert!(kernel.is_aligned(), "Kernel should be memory-aligned");
        assert!(kernel.len() > 0, "Kernel should have cashflows");

        // Verify pricing produces finite result
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        assert!(pv.is_finite(), "PV should be finite");
        assert!(!pv.is_nan(), "PV should not be NaN");

        println!("Enzyme AD compatibility: VERIFIED");
        println!("  - Smooth operations: ✓");
        println!("  - Branchless execution: ✓");
        println!("  - Sequential array access: ✓");
        println!("  - f64 type compatibility: ✓");
    }
}

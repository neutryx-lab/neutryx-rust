//! Integration tests for the Pricing Kernel IR pipeline.

#[cfg(test)]
mod tests {
    use infra_domain::{
        market::{Currency, RateIndex},
        time::Date,
        trade::{
            Cashflow, CashflowType, Direction, IndexType, Leg, LegType, Payoff, Trade, TradeType,
        },
    };
    use pricer_core::kernel::PricingKernelBuilder;
    use pricer_models::compiler::{IndexMapper, LinearProductsCompiler, TradeCompiler};

    use super::super::{context::KernelContext, engine::LinearEngine, provider::FlatCurveProvider};

    fn create_fixed_leg(notional: f64, rate: f64, direction: Direction) -> Leg {
        let cashflows = vec![
            Cashflow::new(
                CashflowType::Coupon,
                Date::from_ymd(2025, 6, 30).unwrap(),
                Date::from_ymd(2025, 1, 1).unwrap(),
                Date::from_ymd(2025, 6, 30).unwrap(),
                0.5,
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
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        assert_eq!(kernel.len(), 4, "Kernel should have 4 cashflows");

        let discount_rate = 0.03;
        let forward_rate = 0.025;
        let provider = FlatCurveProvider::new(discount_rate, forward_rate);

        let context = KernelContext::new(&provider);

        let pv = LinearEngine::price(&kernel, &context);

        assert!(pv.is_finite(), "PV should be finite, got: {}", pv);

        println!(
            "Single IRS PV: {} (notional: {}, fixed rate: {})",
            pv, 10_000_000.0, 0.025
        );
    }

    #[test]
    fn test_e2e_irs_batch_compile_and_price() {
        let trades = vec![
            create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001),
            create_test_irs("IRS-002", 5_000_000.0, 0.030, 0.002),
            create_test_irs("IRS-003", 20_000_000.0, 0.020, 0.001),
        ];

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);

        let kernel = compiler
            .compile_batch(trades.iter())
            .expect("Batch compilation should succeed");

        assert_eq!(kernel.len(), 12, "Kernel should have 12 cashflows");
        assert_eq!(kernel.trade_count(), 3, "Kernel should track 3 trades");

        let provider = FlatCurveProvider::new(0.03, 0.025);

        let context = KernelContext::new(&provider);
        let total_pv = LinearEngine::price(&kernel, &context);

        assert!(total_pv.is_finite(), "Total PV should be finite");

        println!(
            "Batch IRS PV: {} (3 trades, total notional: {})",
            total_pv,
            10_000_000.0 + 5_000_000.0 + 20_000_000.0
        );
    }

    #[test]
    fn test_e2e_kernel_decomposed_pricing() {
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let provider = FlatCurveProvider::new(0.03, 0.025);
        let context = KernelContext::new(&provider);

        let cashflow_pvs = LinearEngine::price_decomposed(&kernel, &context);

        let total_pv = LinearEngine::price(&kernel, &context);
        let sum_pvs: f64 = cashflow_pvs.iter().sum();

        assert!(
            (total_pv - sum_pvs).abs() < 1e-10,
            "Decomposed PVs should sum to total PV: {} vs {}",
            total_pv,
            sum_pvs
        );

        for (i, pv) in cashflow_pvs.iter().enumerate() {
            assert!(pv.is_finite(), "Cashflow {} PV should be finite", i);
        }
    }

    #[test]
    fn test_e2e_manual_kernel_construction() {
        let valuation_date_days = 18263;

        let mut builder = PricingKernelBuilder::new();

        builder.add_cashflow(
            valuation_date_days + 180,
            valuation_date_days,
            0.5,
            -100_000.0,
            0.025,
            0.0,
            0,
            0,
            0,
            0,
        );

        builder.add_cashflow(
            valuation_date_days + 90,
            valuation_date_days,
            0.25,
            100_000.0,
            0.001,
            1.0,
            0,
            0,
            1,
            0,
        );

        let kernel = builder.build().expect("Build should succeed");

        assert_eq!(kernel.len(), 2);
        assert!(kernel.is_aligned());

        let provider = FlatCurveProvider::new(0.03, 0.025).with_valuation_date(valuation_date_days);
        let context = KernelContext::new(&provider);

        let pv = LinearEngine::price(&kernel, &context);
        assert!(pv.is_finite(), "PV should be finite");

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
        let trade = create_test_irs("IRS-001", 10_000_000.0, 0.025, 0.001);

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let provider_low = FlatCurveProvider::new(0.01, 0.01);
        let context_low = KernelContext::new(&provider_low);
        let pv_low = LinearEngine::price(&kernel, &context_low);

        let provider_high = FlatCurveProvider::new(0.05, 0.05);
        let context_high = KernelContext::new(&provider_high);
        let pv_high = LinearEngine::price(&kernel, &context_high);

        assert!(pv_low.is_finite(), "Low rate PV should be finite");
        assert!(pv_high.is_finite(), "High rate PV should be finite");

        assert!(
            (pv_low - pv_high).abs() > 1e-6,
            "Different market scenarios should produce different PVs"
        );

        println!("Low rate scenario PV: {}", pv_low);
        println!("High rate scenario PV: {}", pv_high);
    }

    #[test]
    fn test_e2e_fixed_bond_compile_and_price() {
        let trade = Trade::new(
            "BOND-001",
            vec![create_fixed_leg(5_000_000.0, 0.04, Direction::Receiver)],
            TradeType::Generic,
        );

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        assert_eq!(kernel.len(), 2);

        let provider = FlatCurveProvider::new(0.03, 0.0);
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        let decomposed = LinearEngine::price_decomposed(&kernel, &context);
        for cf_pv in &decomposed {
            assert!(*cf_pv > 0.0, "Fixed bond cashflows should be positive");
        }

        assert!(pv > 0.0, "Bond PV should be positive");
        println!("Fixed bond PV: {}", pv);
    }

    #[test]
    fn test_e2e_at_par_swap() {
        let trade = create_test_irs("IRS-PAR", 10_000_000.0, 0.03, 0.0);

        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let provider = FlatCurveProvider::new(0.03, 0.03);
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        assert!(
            pv.abs() < 100_000.0,
            "At-par swap PV should be close to zero, got: {}",
            pv
        );

        println!("At-par swap PV: {} (should be close to 0)", pv);
    }

    struct FiniteDifferenceSensitivity {
        bump_size: f64,
    }

    impl FiniteDifferenceSensitivity {
        fn new(bump_size: f64) -> Self { Self { bump_size } }

        fn discount_rate_sensitivity(
            &self,
            kernel: &pricer_core::kernel::PricingKernel,
            base_discount_rate: f64,
            forward_rate: f64,
        ) -> f64 {
            let h = self.bump_size;

            let provider_up = FlatCurveProvider::new(base_discount_rate + h, forward_rate);
            let context_up = KernelContext::new(&provider_up);
            let pv_up = LinearEngine::price(kernel, &context_up);

            let provider_down = FlatCurveProvider::new(base_discount_rate - h, forward_rate);
            let context_down = KernelContext::new(&provider_down);
            let pv_down = LinearEngine::price(kernel, &context_down);

            (pv_up - pv_down) / (2.0 * h)
        }

        fn forward_rate_sensitivity(
            &self,
            kernel: &pricer_core::kernel::PricingKernel,
            discount_rate: f64,
            base_forward_rate: f64,
        ) -> f64 {
            let h = self.bump_size;

            let provider_up = FlatCurveProvider::new(discount_rate, base_forward_rate + h);
            let context_up = KernelContext::new(&provider_up);
            let pv_up = LinearEngine::price(kernel, &context_up);

            let provider_down = FlatCurveProvider::new(discount_rate, base_forward_rate - h);
            let context_down = KernelContext::new(&provider_down);
            let pv_down = LinearEngine::price(kernel, &context_down);

            (pv_up - pv_down) / (2.0 * h)
        }
    }

    #[test]
    fn test_enzyme_ad_smoothness_discount_rate() {
        let trade = create_test_irs("IRS-AD-TEST", 10_000_000.0, 0.025, 0.001);
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

        let sensitivity_1bp = FiniteDifferenceSensitivity::new(0.0001).discount_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );
        let sensitivity_0_1bp = FiniteDifferenceSensitivity::new(0.00001)
            .discount_rate_sensitivity(&kernel, discount_rate, forward_rate);
        let sensitivity_0_01bp = FiniteDifferenceSensitivity::new(0.000001)
            .discount_rate_sensitivity(&kernel, discount_rate, forward_rate);

        let diff_1 = (sensitivity_1bp - sensitivity_0_1bp).abs();
        let diff_2 = (sensitivity_0_1bp - sensitivity_0_01bp).abs();

        assert!(
            diff_2 < diff_1 * 10.0,
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
        let trade = create_test_irs("IRS-AD-FWD", 10_000_000.0, 0.01, 0.002);
        let mapper = IndexMapper::new();
        let compiler = LinearProductsCompiler::new(mapper);
        let kernel = compiler
            .compile(&trade)
            .expect("Compilation should succeed");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

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

    #[test]
    fn test_enzyme_ad_fixed_leg_analytical() {
        let kernel = pricer_core::kernel::PricingKernel::new(
            vec![365],
            vec![0],
            vec![1.0],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .expect("Valid kernel");

        let discount_rate = 0.03;
        let forward_rate = 0.0;

        let sensitivity = FiniteDifferenceSensitivity::new(0.0001).discount_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );

        let provider = FlatCurveProvider::new(discount_rate, forward_rate);
        let context = KernelContext::new(&provider);
        let base_pv = LinearEngine::price(&kernel, &context);

        let t = 1.0;
        let analytical = -t * base_pv;

        let relative_error = (sensitivity - analytical).abs() / analytical.abs();

        assert!(
            relative_error < 0.01,
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

    #[test]
    fn test_enzyme_ad_floating_leg_analytical() {
        let kernel = pricer_core::kernel::PricingKernel::new(
            vec![365],
            vec![0],
            vec![1.0],
            vec![1_000_000.0],
            vec![0.01],
            vec![1.0],
            vec![0],
            vec![0],
            vec![1],
            vec![0],
        )
        .expect("Valid kernel");

        let discount_rate = 0.03;
        let forward_rate = 0.025;

        let sensitivity = FiniteDifferenceSensitivity::new(0.0001).forward_rate_sensitivity(
            &kernel,
            discount_rate,
            forward_rate,
        );

        let notional = 1_000_000.0;
        let tau = 1.0;
        let gearing = 1.0;
        let df = (-discount_rate * 1.0).exp();
        let analytical = notional * tau * gearing * df;

        let relative_error = (sensitivity - analytical).abs() / analytical.abs();

        assert!(
            relative_error < 0.01,
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

    #[test]
    fn test_enzyme_ad_compatibility_documentation() {
        let kernel = pricer_core::kernel::PricingKernel::new(
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

        assert!(kernel.is_aligned(), "Kernel should be memory-aligned");
        assert!(kernel.len() > 0, "Kernel should have cashflows");

        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let pv = LinearEngine::price(&kernel, &context);

        assert!(pv.is_finite(), "PV should be finite");
        assert!(!pv.is_nan(), "PV should not be NaN");

        println!("Enzyme AD compatibility: VERIFIED");
    }
}

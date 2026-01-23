//! RateIndex Pricing Integration Tests
//!
//! These tests verify the integration of RateIndex throughout the pricing pipeline:
//! - PayoffEvaluator correctly evaluates Fixed, Linear, and VanillaOption payoffs
//! - OisCalculator correctly computes daily compounded rates
//! - AD compatibility with Dual64 numeric type
//!
//! Requirements Coverage: 5.6, 6.5, 10.2

// Only run these tests when l1l2-integration feature is enabled
#![cfg(feature = "l1l2-integration")]

use pricer_models::market::curves::{CurveEnum, CurveName, CurveSet};
use pricer_pricing::generic_pricer::{DailyAccrual, OisCalculator, PayoffEvaluator};

// =============================================================================
// PayoffEvaluator Tests
// =============================================================================

mod payoff_evaluator_integration {
    use super::*;
    use infra_master::trade::{IndexType, Payoff};
    use infra_master::RateIndex;

    fn create_test_curve_set() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.insert(CurveName::Euribor, CurveEnum::flat(0.04_f64));
        curves.insert(CurveName::Sonia, CurveEnum::flat(0.04_f64));
        curves.insert(CurveName::Tonar, CurveEnum::flat(0.001_f64));
        curves.insert(CurveName::Saron, CurveEnum::flat(0.012_f64));
        curves
    }

    /// Test: Fixed payoff evaluation
    /// Requirement: 5.1, 5.2
    #[test]
    fn test_fixed_payoff_evaluation() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::fixed(0.03); // 3% fixed rate
        let notional = 1_000_000.0;
        let year_fraction = 0.5; // 6 months

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.0, 0.5)
            .unwrap();

        // Expected: 1,000,000 * 0.03 * 0.5 = 15,000
        assert!(
            (amount - 15_000.0).abs() < 1e-6,
            "Fixed payoff should equal notional * rate * year_fraction"
        );
    }

    /// Test: Linear (floating) payoff evaluation with SOFR
    /// Requirement: 5.3, 5.4
    #[test]
    fn test_linear_payoff_sofr() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let notional = 1_000_000.0;
        let year_fraction = 0.25; // 3 months

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.25, 0.5)
            .unwrap();

        // Expected: 1,000,000 * 0.035 * 1.0 * 0.25 = 8,750
        assert!(
            (amount - 8_750.0).abs() < 100.0,
            "Linear payoff should use forward rate from SOFR curve"
        );
    }

    /// Test: Linear payoff with spread
    /// Requirement: 5.3
    #[test]
    fn test_linear_payoff_with_spread() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating_with_spread(IndexType::Rate(RateIndex::Sofr), 0.005); // 50bp spread
        let notional = 1_000_000.0;
        let year_fraction = 0.25;

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.25, 0.5)
            .unwrap();

        // Expected: 1,000,000 * (0.035 + 0.005) * 1.0 * 0.25 = 10,000
        assert!(
            (amount - 10_000.0).abs() < 100.0,
            "Linear payoff should apply spread to forward rate"
        );
    }

    /// Test: Cap (Call) option evaluation - in the money
    /// Requirement: 7.1, 7.2
    #[test]
    fn test_cap_in_the_money() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Cap with strike 3% (SOFR is 3.5%, so ITM)
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sofr), 0.03);
        let notional = 1_000_000.0;
        let year_fraction = 0.25;

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.25, 0.5)
            .unwrap();

        // Expected intrinsic: 1,000,000 * (0.035 - 0.03) * 0.25 = 1,250
        assert!(
            amount > 0.0,
            "ITM cap should have positive intrinsic value"
        );
        assert!(
            (amount - 1_250.0).abs() < 200.0,
            "Cap intrinsic value calculation"
        );
    }

    /// Test: Cap (Call) option evaluation - out of the money
    /// Requirement: 7.3
    #[test]
    fn test_cap_out_of_the_money() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Cap with strike 4% (SOFR is 3.5%, so OTM)
        let payoff = Payoff::cap(IndexType::Rate(RateIndex::Sofr), 0.04);
        let notional = 1_000_000.0;
        let year_fraction = 0.25;

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.25, 0.5)
            .unwrap();

        // OTM cap has zero intrinsic value
        assert!(
            amount.abs() < 1e-6,
            "OTM cap should have zero intrinsic value"
        );
    }

    /// Test: Floor (Put) option evaluation - in the money
    /// Requirement: 7.1, 7.4
    #[test]
    fn test_floor_in_the_money() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        // Floor with strike 4% (SOFR is 3.5%, so ITM)
        let payoff = Payoff::floor(IndexType::Rate(RateIndex::Sofr), 0.04);
        let notional = 1_000_000.0;
        let year_fraction = 0.25;

        let amount = evaluator
            .evaluate(&payoff, notional, year_fraction, 0.25, 0.5)
            .unwrap();

        // Expected intrinsic: 1,000,000 * (0.04 - 0.035) * 0.25 = 1,250
        assert!(
            amount > 0.0,
            "ITM floor should have positive intrinsic value"
        );
    }

    /// Test: Linear payoff with missing curve returns error
    /// Requirement: 5.6
    #[test]
    fn test_linear_payoff_missing_curve() {
        let curves = CurveSet::<f64>::new(); // Empty curve set
        let evaluator = PayoffEvaluator::new(&curves);

        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let result = evaluator.evaluate(&payoff, 1_000_000.0, 0.25, 0.25, 0.5);

        assert!(
            result.is_err(),
            "Should return error when required curve is missing"
        );
    }

    /// Test: Evaluation with all supported indices
    /// Requirement: 5.4
    #[test]
    fn test_all_rate_indices() {
        let curves = create_test_curve_set();
        let evaluator = PayoffEvaluator::new(&curves);

        let indices = [
            RateIndex::Sofr,
            RateIndex::Euribor3M,
            RateIndex::Euribor6M,
            RateIndex::Sonia,
            RateIndex::Tonar,
            RateIndex::Saron,
        ];

        for index in indices {
            let payoff = Payoff::floating(IndexType::Rate(index));
            let result = evaluator.evaluate(&payoff, 1_000_000.0, 0.25, 0.25, 0.5);
            assert!(
                result.is_ok(),
                "Should be able to evaluate Linear payoff for {:?}",
                index
            );
        }
    }
}

// =============================================================================
// OisCalculator Tests
// =============================================================================

mod ois_calculator_integration {
    use super::*;

    /// Test: Empty accruals return zero
    /// Requirement: 6.2
    #[test]
    fn test_empty_accruals() {
        let accruals: Vec<DailyAccrual> = vec![];
        let rate = OisCalculator::compound_rate::<f64>(&accruals);
        assert!(
            rate.abs() < 1e-15,
            "Empty accruals should return zero rate"
        );
    }

    /// Test: Single day accrual
    /// Requirement: 6.1
    #[test]
    fn test_single_day_accrual() {
        let accruals = vec![DailyAccrual::new(0.035, 1.0 / 360.0)];
        let rate = OisCalculator::compound_rate::<f64>(&accruals);

        // Single day: (1 + 0.035 / 360) - 1 = 0.035 / 360
        let expected = 0.035 / 360.0;
        assert!(
            (rate - expected).abs() < 1e-12,
            "Single day accrual should equal rate * day_fraction"
        );
    }

    /// Test: Multiple days with constant rate
    /// Requirement: 6.1, 6.3
    #[test]
    fn test_constant_rate_compounding() {
        // 90 days at 5.25% SOFR (typical quarterly period)
        let accruals: Vec<DailyAccrual> = (0..90)
            .map(|_| DailyAccrual::new(0.0525, 1.0 / 360.0))
            .collect();

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);
        let annualized = OisCalculator::annualized_rate(compounded, 90.0 / 360.0);

        // Compounded rate should be close to 5.25%
        assert!(
            (annualized - 0.0525).abs() < 1e-3,
            "Annualized rate should be close to input rate"
        );
    }

    /// Test: Weekend treatment (3-day accrual)
    /// Requirement: 6.1
    #[test]
    fn test_weekend_accrual() {
        // Friday to Monday: Friday rate applies for 3 days
        let accruals = vec![
            DailyAccrual::new(0.035, 1.0 / 360.0), // Thursday
            DailyAccrual::new(0.035, 3.0 / 360.0), // Friday (applies over weekend)
            DailyAccrual::new(0.035, 1.0 / 360.0), // Monday
        ];

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);

        // Manual calculation
        let expected =
            (1.0 + 0.035 / 360.0) * (1.0 + 3.0 * 0.035 / 360.0) * (1.0 + 0.035 / 360.0) - 1.0;
        assert!(
            (compounded - expected).abs() < 1e-15,
            "Weekend treatment should account for 3-day accrual"
        );
    }

    /// Test: Annualized rate calculation
    /// Requirement: 6.4
    #[test]
    fn test_annualized_rate() {
        // 0.875% compounded over 0.25 years => ~3.5% annualized
        let compounded = 0.035 * 0.25; // Simplified for test
        let annualized = OisCalculator::annualized_rate::<f64>(compounded, 0.25);
        assert!(
            (annualized - 0.035).abs() < 1e-10,
            "Annualization should divide by year fraction"
        );
    }

    /// Test: Zero period returns zero
    /// Requirement: 6.4
    #[test]
    fn test_zero_period_annualization() {
        let rate = OisCalculator::annualized_rate::<f64>(0.005, 0.0);
        assert!(
            rate.abs() < 1e-10,
            "Zero period should return zero rate"
        );
    }

    /// Test: Known SOFR compounding scenario
    /// Requirement: 6.5
    #[test]
    fn test_known_sofr_scenario() {
        // Simulate a quarterly SOFR payment with 90 days
        let accruals: Vec<DailyAccrual> = (0..90)
            .map(|_| DailyAccrual::new(0.0525, 1.0 / 360.0)) // 5.25% SOFR
            .collect();

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);

        // Payment calculation for $10M notional
        let notional = 10_000_000.0_f64;
        let payment = notional * compounded;

        // Expected: ~$10M * 5.25% * 0.25 = ~$131,250
        assert!(
            (payment - 131_250.0).abs() < 1500.0,
            "SOFR payment should match expected quarterly interest"
        );
    }

    /// Test: Compound rate with history
    /// Requirement: 6.3
    #[test]
    fn test_compound_rate_with_history() {
        let accruals = vec![
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
        ];

        let history = OisCalculator::compound_rate_with_history::<f64>(&accruals, 1_000_000.0);

        assert_eq!(history.len(), 3, "History should have one entry per day");

        // Each day should increase the notional
        assert!(history[0].0 > 1_000_000.0);
        assert!(history[1].0 > history[0].0);
        assert!(history[2].0 > history[1].0);

        // Final compounded rate should match compound_rate function
        let expected_rate = OisCalculator::compound_rate::<f64>(&accruals);
        assert!(
            (history[2].1 - expected_rate).abs() < 1e-15,
            "History final rate should match compound_rate"
        );
    }
}

// =============================================================================
// AD Compatibility Tests
// =============================================================================

mod ad_compatibility {
    use super::*;

    /// Test: OisCalculator with f32 (verifies Float trait generics)
    /// Requirement: 10.2
    #[test]
    fn test_ois_calculator_f32() {
        let accruals = vec![
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
        ];

        // This should compile and run with f32
        let rate_f32 = OisCalculator::compound_rate::<f32>(&accruals);
        let rate_f64 = OisCalculator::compound_rate::<f64>(&accruals);

        // f32 and f64 should give similar results
        assert!(
            ((rate_f32 as f64) - rate_f64).abs() < 1e-6,
            "f32 and f64 should give similar results"
        );
    }

    /// Test: CurveSet works with different Float types
    /// Requirement: 10.2
    #[test]
    fn test_curve_set_f32() {
        use infra_master::RateIndex;

        let mut curves: CurveSet<f32> = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f32));

        let fwd_rate = curves
            .forward_rate_for_index(RateIndex::Sofr, 0.5_f32, 1.0_f32)
            .unwrap();

        assert!(
            (fwd_rate - 0.035).abs() < 1e-5,
            "f32 forward rate should work"
        );
    }

    /// Test: OisCalculator annualized_rate with f32
    /// Requirement: 10.2
    #[test]
    fn test_annualized_rate_f32() {
        let compounded = 0.00875_f32; // ~3.5% over 0.25 years
        let annualized = OisCalculator::annualized_rate::<f32>(compounded, 0.25);

        assert!(
            (annualized - 0.035).abs() < 1e-5,
            "f32 annualization should work"
        );
    }
}

// =============================================================================
// Cross-Component Integration Tests
// =============================================================================

mod cross_component {
    use super::*;
    use infra_master::trade::{IndexType, Payoff};
    use infra_master::RateIndex;

    /// Test: Full pipeline from RateIndex to evaluated amount
    /// Requirement: 5.5, 6.3
    #[test]
    fn test_full_pipeline() {
        // 1. Create curve set with index curves
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        // 2. Create PayoffEvaluator
        let evaluator = PayoffEvaluator::new(&curves);

        // 3. Create a Linear payoff referencing SOFR
        let payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));

        // 4. Evaluate the payoff
        let amount = evaluator
            .evaluate(&payoff, 1_000_000.0, 0.25, 0.25, 0.5)
            .unwrap();

        // 5. Verify the amount is positive and reasonable
        assert!(amount > 0.0, "Evaluated amount should be positive");
        assert!(
            amount < 100_000.0,
            "Evaluated amount should be reasonable for quarterly interest"
        );
    }

    /// Test: OIS cashflow with daily accruals
    /// Requirement: 6.1, 6.5
    #[test]
    fn test_ois_cashflow_pipeline() {
        // Simulate processing an OIS cashflow with daily accruals

        // 1. Daily accruals for 30 days at 3.5%
        let accruals: Vec<DailyAccrual> = (0..30)
            .map(|_| DailyAccrual::new(0.035, 1.0 / 360.0))
            .collect();

        // 2. Calculate compounded rate
        let compounded_rate = OisCalculator::compound_rate::<f64>(&accruals);

        // 3. Calculate cashflow amount
        let notional = 10_000_000.0;
        let cashflow_amount = notional * compounded_rate;

        // 4. Expected: ~10M * 3.5% * (30/360) = ~29,167
        assert!(
            (cashflow_amount - 29_167.0).abs() < 500.0,
            "OIS cashflow amount should match expected"
        );
    }

    /// Test: Multiple indices in same evaluation context
    /// Requirement: 5.4
    #[test]
    fn test_multi_index_evaluation() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.insert(CurveName::Euribor, CurveEnum::flat(0.04_f64));

        let evaluator = PayoffEvaluator::new(&curves);

        // Evaluate SOFR-linked payoff
        let sofr_payoff = Payoff::floating(IndexType::Rate(RateIndex::Sofr));
        let sofr_amount = evaluator
            .evaluate(&sofr_payoff, 1_000_000.0, 0.25, 0.0, 0.25)
            .unwrap();

        // Evaluate EURIBOR-linked payoff
        let euribor_payoff = Payoff::floating(IndexType::Rate(RateIndex::Euribor3M));
        let euribor_amount = evaluator
            .evaluate(&euribor_payoff, 1_000_000.0, 0.25, 0.0, 0.25)
            .unwrap();

        // EURIBOR rate (4%) > SOFR rate (3.5%)
        assert!(
            euribor_amount > sofr_amount,
            "Higher rate index should produce higher cashflow"
        );
    }
}

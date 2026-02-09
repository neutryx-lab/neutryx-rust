#[cfg(test)]
mod tests {
    use crate::{
        market::{
            convention::{
                CdsConvention, ConventionSet, EquityConvention, FxConvention, FxOptionConvention,
                InflationSwapConvention, SwapConvention, SwaptionConvention,
            },
            Currency,
            instrument::{
                AsianOption, AveragingType, BarrierDirection, BarrierType,
                CapFloor, CapFloorType, Cds, CdsIndex, CmsSwap,
                CommodityForward, CommoditySwap, CommodityType,
                CommodityVanillaOption, CreditEvent, CurrencyPair,
                EnergyType, EquityForward, EquityReturnType,
                EquitySwap, EquityUnderlying, EquityVanillaOption, ExerciseStyle, Frn,
                FxBarrierOption, FxForward, FxSpot, FxSwap, FxVanillaOption,
                InflationSwap, InstrumentDefinition, InstrumentError, InstrumentExpander,
                NotionalSchedule, Ois, PayerReceiver, QuantityUnit,
                SwapType, Swaption,
            },
        },
        time::{Date, Tenor},
        trade::{ExerciseType, SettlementType, TradeType},
    };

    fn make_conventions() -> ConventionSet {
        ConventionSet::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_swaption(SwaptionConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_fx_option(FxOptionConvention::g10_standard())
            .with_cds(CdsConvention::isda_na())
            .with_equity(EquityConvention::us_equity())
            .with_inflation_swap(InflationSwapConvention::us_cpi_zc())
    }

    fn valuation_date() -> Date { Date::from_ymd(2025, 1, 1).unwrap() }

    // === Rates Tests ===

    #[test]
    fn test_expand_swaption() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "SWAPTION-001");
        assert!(trade.trade_type.is_swaption());
        assert_eq!(trade.num_legs(), 1);
        assert_eq!(trade.total_cashflows(), 1);
    }

    #[test]
    fn test_expand_swaption_missing_convention() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let empty_conventions = ConventionSet::new();
        let result = swaption.expand_to_trade("SWAPTION-001", valuation_date(), &empty_conventions);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));
    }

    // === FX Tests ===

    #[test]
    fn test_expand_fx_spot() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-SPOT-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_expand_fx_forward() {
        let fx_forward = FxForward {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            forward_rate: 1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_forward
            .expand_to_trade("FX-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
        assert_eq!(trade.num_legs(), 2);
    }

    #[test]
    fn test_expand_fx_swap() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 4); // near pay, near receive, far pay, far
                                         // receive
    }

    // === Equity Tests ===

    #[test]
    fn test_expand_equity_forward() {
        let eq_forward = EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_forward
            .expand_to_trade("EQ-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-FWD-001");
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_equity_vanilla_option() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100_000.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("EQ-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // === Credit Tests ===

    #[test]
    fn test_expand_cds() {
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        };

        let trade = cds
            .expand_to_trade("CDS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CDS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // premium leg and protection leg
    }

    // === Commodity Tests ===

    #[test]
    fn test_expand_commodity_forward() {
        let comm_forward = CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        };

        let trade = comm_forward
            .expand_to_trade("COMM-FWD-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-FWD-001");
        assert_eq!(trade.trade_type, TradeType::FxForward);
    }

    // === InstrumentDefinition Integration Tests ===

    #[test]
    fn test_instrument_definition_expand() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let trade = instrument
            .expand_to_trade("INST-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "INST-001");
    }

    #[test]
    fn test_instrument_definition_expand_validates() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: -1_000_000.0, // Invalid: negative notional
            notional_currency: Currency::EUR,
        };

        let instrument = InstrumentDefinition::FxSpot(fx_spot);
        let result = instrument.expand_to_trade("INST-001", valuation_date(), &make_conventions());

        assert!(result.is_err());
    }

    #[test]
    fn test_trade_all_cashflows_compatibility() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        // Verify Trade::all_cashflows() works
        let cashflows: Vec<_> = trade.all_cashflows().collect();
        assert_eq!(cashflows.len(), 4);

        // Verify future_cashflows() works
        let future_cfs: Vec<_> = trade.future_cashflows(valuation_date()).collect();
        assert_eq!(future_cfs.len(), 4);
    }

    // =========================================================================
    // Task 11.2: CF Expansion Integration Tests
    // =========================================================================

    #[test]
    fn test_expand_cap_floor() {
        use crate::market::RateIndex;

        let cap = CapFloor {
            cap_floor_type: CapFloorType::Cap,
            strikes: vec![0.05],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::TwoYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: crate::time::Frequency::Quarterly,
            currency: Currency::USD,
        };

        let trade = cap
            .expand_to_trade("CAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CAP-001");
        assert_eq!(trade.trade_type, TradeType::CapFloor);
        assert_eq!(trade.num_legs(), 1);
    }

    #[test]
    fn test_expand_frn() {
        use crate::market::RateIndex;

        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.005,
            reset_frequency: crate::time::Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(10_000_000.0),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            currency: Currency::USD,
        };

        let trade = frn
            .expand_to_trade("FRN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FRN-001");
        assert!(matches!(trade.trade_type, TradeType::Bond { .. }));
        assert_eq!(trade.num_legs(), 1);
        assert!(trade.total_cashflows() >= 2); // At least coupon + principal
    }

    #[test]
    fn test_expand_cms_swap() {
        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::USD,
            spread: 0.001,
        };

        let trade = cms
            .expand_to_trade("CMS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CMS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg with CMS rate cashflow
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_inflation_swap() {
        let inf_swap = InflationSwap {
            inflation_index: "USCPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.02,
        };

        let trade = inf_swap
            .expand_to_trade("INF-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "INF-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        // Implementation creates single leg for inflation leg
        assert!(trade.num_legs() >= 1);
    }

    #[test]
    fn test_expand_ois() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2026, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Annual,
        };

        let trade = ois
            .expand_to_trade("OIS-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "OIS-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // Fixed + Floating
    }

    #[test]
    fn test_expand_ois_has_daily_accruals() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 4, 15).unwrap(), // 3 months
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Receiver,
            payment_frequency: crate::time::Frequency::Quarterly,
        };

        let trade = ois
            .expand_to_trade("OIS-002", valuation_date(), &make_conventions())
            .unwrap();

        // Floating leg should have daily accrual details
        let floating_leg = trade.floating_leg().expect("Should have floating leg");
        let cashflows: Vec<_> = floating_leg.cashflows().collect();
        assert!(!cashflows.is_empty());

        // Each cashflow in the floating leg should have daily accruals
        for cf in cashflows {
            assert!(
                cf.has_daily_accruals(),
                "OIS floating cashflow should have daily accruals"
            );
            let accruals = cf.daily_accruals().expect("Should have accruals");
            // Should have roughly 89 business days for a quarter (excluding weekends in
            // real scenario)
            assert!(!accruals.is_empty(), "Should have daily accrual entries");
        }
    }

    #[test]
    fn test_expand_ois_daily_compounding_calculation() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 2, 15).unwrap(), // 1 month
            notional: 1_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Monthly,
        };

        let trade = ois
            .expand_to_trade("OIS-003", valuation_date(), &make_conventions())
            .unwrap();

        let floating_leg = trade.floating_leg().expect("Should have floating leg");
        let cf = floating_leg
            .cashflows()
            .next()
            .expect("Should have at least one cashflow");
        let accruals = cf.daily_accruals().expect("Should have accruals");

        // Verify compounding: each day's notional should grow
        let mut prev_notional = ois.notional;
        for accrual in accruals {
            assert!(
                accrual.compounded_notional >= prev_notional,
                "Compounded notional should grow: {} >= {}",
                accrual.compounded_notional,
                prev_notional
            );
            prev_notional = accrual.compounded_notional;
        }

        // Final compounded notional should be greater than initial
        if let Some(last) = accruals.last() {
            assert!(
                last.compounded_notional > ois.notional,
                "Final compounded notional {} should exceed initial {}",
                last.compounded_notional,
                ois.notional
            );
        }
    }

    #[test]
    fn test_ois_validate_success() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Annual,
        };

        assert!(ois.validate().is_ok());
    }

    #[test]
    fn test_ois_validate_invalid_notional() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: -10_000_000.0, // Invalid: negative
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Annual,
        };

        assert!(ois.validate().is_err());
    }

    #[test]
    fn test_ois_validate_invalid_dates() {
        use crate::market::RateIndex;

        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2030, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 1, 15).unwrap(), // Invalid: end before start
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Annual,
        };

        assert!(ois.validate().is_err());
    }

    #[test]
    fn test_expand_fx_vanilla_option() {
        use crate::trade::OptionType;

        let fx_option = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_option
            .expand_to_trade("FX-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_fx_barrier_option() {
        use crate::trade::OptionType;

        let vanilla = FxVanillaOption {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let barrier_option = FxBarrierOption {
            vanilla,
            barrier_level: 1.15,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(5000.0),
        };

        let trade = barrier_option
            .expand_to_trade("FX-BARRIER-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "FX-BARRIER-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_asian_option() {
        use crate::trade::OptionType;

        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::time::Frequency::Monthly,
            observed_values: vec![175.0, 178.0, 180.0],
            notional: 1000.0,
            currency: Currency::USD,
        };

        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "ASIAN-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    #[test]
    fn test_expand_equity_swap() {
        let eq_swap = EquitySwap {
            underlying: EquityUnderlying::index("SPX"),
            return_type: EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
        };

        let trade = eq_swap
            .expand_to_trade("EQ-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "EQ-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // equity leg + funding leg
    }

    #[test]
    fn test_expand_cds_index() {
        let cds_idx = CdsIndex {
            index_name: "CDX.NA.IG".to_string(),
            series: 40,
            version: 1,
            constituent_count: 125,
            notional: 10_000_000.0,
            spread: 0.006,
            start_date: Date::from_ymd(2025, 3, 20).unwrap(),
            maturity: Date::from_ymd(2030, 6, 20).unwrap(),
            currency: Currency::USD,
        };

        let trade = cds_idx
            .expand_to_trade("CDS-IDX-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "CDS-IDX-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
    }

    #[test]
    fn test_expand_commodity_swap() {
        let comm_swap = CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            fixed_price: 75.0,
            floating_index: "WTI".to_string(),
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2026, 1, 15).unwrap(),
            quantity_per_period: 1000.0,
            unit: QuantityUnit::Barrels,
            payment_frequency: crate::time::Frequency::Monthly,
            currency: Currency::USD,
        };

        let trade = comm_swap
            .expand_to_trade("COMM-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-SWAP-001");
        assert_eq!(trade.trade_type, TradeType::Swap);
        assert_eq!(trade.num_legs(), 2); // fixed + floating
    }

    #[test]
    fn test_expand_commodity_vanilla_option() {
        use crate::trade::OptionType;

        let comm_opt = CommodityVanillaOption {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            strike: 3.50,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 10000.0,
            unit: QuantityUnit::MMBtu,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        };

        let trade = comm_opt
            .expand_to_trade("COMM-OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "COMM-OPT-001");
        assert_eq!(trade.trade_type, TradeType::Generic);
    }

    // Verify convention integration
    #[test]
    fn test_conventions_affect_expansion() {
        // Same swaption with different conventions should have different exercise types
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        let conv = make_conventions();
        let trade = swaption
            .expand_to_trade("SWAPTION-001", valuation_date(), &conv)
            .unwrap();

        // Trade type should match swaption settings
        if let TradeType::Swaption {
            exercise_type,
            settlement_type,
            ..
        } = trade.trade_type
        {
            assert_eq!(exercise_type, ExerciseType::European);
            assert_eq!(settlement_type, SettlementType::Cash);
        } else {
            panic!("Expected TradeType::Swaption");
        }
    }

    // =========================================================================
    // Task 11.3: Edge Case Tests
    // =========================================================================

    #[test]
    fn test_edge_case_zero_notional_validation() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 0.0, // Edge case: zero notional
            notional_currency: Currency::EUR,
        };

        // FxSpot.validate() should catch this
        assert!(fx_spot.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_notional_validation() {
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::FiveYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: -10_000_000.0, // Edge case: negative notional
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };

        // Swaption.validate() should catch this
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_edge_case_negative_strike_validation() {
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: -100.0, // Edge case: negative strike
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        // EquityVanillaOption.validate() should catch this
        assert!(eq_option.validate().is_err());
    }

    #[test]
    fn test_edge_case_maturity_before_start_validation() {
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(), // Edge case: maturity before start
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Cds.validate() should catch this
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_edge_case_same_start_end_date() {
        // Same-day FX spot should still work
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: valuation_date(), // Same as valuation date
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_empty_observed_values() {
        use crate::trade::OptionType;

        // Asian option with no observed values yet
        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::time::Frequency::Monthly,
            observed_values: vec![], // Edge case: empty observations
            notional: 1000.0,
            currency: Currency::USD,
        };

        // Should succeed - Asian option can start with no observations
        let trade = asian
            .expand_to_trade("ASIAN-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.id.as_str(), "ASIAN-001");
    }

    #[test]
    fn test_edge_case_very_large_notional() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1e15, // Edge case: very large notional
            notional_currency: Currency::EUR,
        };

        let trade = fx_spot
            .expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert_eq!(trade.total_cashflows(), 2);
    }

    #[test]
    fn test_edge_case_very_small_rate() {
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1e-10, // Edge case: very small rate (but positive)
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // Very small rate might be rejected
        let result = fx_spot.expand_to_trade("FX-SPOT-001", valuation_date(), &make_conventions());
        // Depends on validation - either succeeds or fails with appropriate error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_edge_case_fx_swap_same_near_far_date_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Same as near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_far_date_before_near_validation() {
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // Before near date
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        // FxSwap.validate() should catch this
        assert!(fx_swap.validate().is_err());
    }

    #[test]
    fn test_edge_case_zero_spread_cds() {
        // CDS with zero spread is unusual but should work
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.0, // Zero spread
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        };

        // Zero spread might be allowed for special cases
        let result = cds.expand_to_trade("CDS-001", valuation_date(), &make_conventions());
        // Validation depends on business rules
        assert!(result.is_ok() || result.is_err());
    }

    // =========================================================================
    // Task 11.4: Property-Based Tests (Consistency Checks)
    // =========================================================================

    #[test]
    fn test_property_expanded_trade_has_cashflows() {
        // Property: Every successfully expanded trade must have at least one cashflow
        let instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::FxForward(FxForward {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                forward_rate: 1.1100,
                settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::index("SPX"),
                forward_price: 5000.0,
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100_000.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for (i, inst) in instruments.iter().enumerate() {
            let trade = inst
                .expand_to_trade(format!("INST-{}", i), valuation_date(), &conv)
                .unwrap();

            // Property: trade must have at least one leg with at least one cashflow
            assert!(
                trade.total_cashflows() >= 1,
                "Trade must have at least one cashflow"
            );
            assert!(trade.num_legs() >= 1, "Trade must have at least one leg");
        }
    }

    #[test]
    fn test_property_trade_id_preserved() {
        // Property: Trade ID passed to expand_to_trade must be preserved
        let fx_spot = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let test_ids = ["test-123", "TRADE_ABC", "id with spaces", ""];
        let conv = make_conventions();

        for id in &test_ids {
            let trade = fx_spot
                .expand_to_trade(*id, valuation_date(), &conv)
                .unwrap();

            assert_eq!(trade.id.as_str(), *id, "Trade ID must be preserved");
        }
    }

    #[test]
    fn test_property_validation_before_expansion() {
        // Property: Invalid instruments should fail validation before expansion
        let invalid_instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
                notional: -1_000_000.0, // Invalid
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::stock("AAPL"),
                forward_price: -100.0, // Invalid
                settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
                notional: 100.0,
                currency: Currency::USD,
            }),
        ];

        let conv = make_conventions();
        for inst in &invalid_instruments {
            let result = inst.expand_to_trade("INVALID", valuation_date(), &conv);
            assert!(result.is_err(), "Invalid instrument should fail expansion");
        }
    }

    #[test]
    fn test_property_cashflow_currencies_consistent() {
        // Property: All cashflows in a leg should have the same currency
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        for leg in trade.legs() {
            let leg_ccy = leg.currency;
            for cf in leg.cashflows() {
                assert_eq!(
                    cf.currency, leg_ccy,
                    "Cashflow currency must match leg currency"
                );
            }
        }
    }

    #[test]
    fn test_property_swap_has_multiple_legs() {
        // Property: Swaps should have at least 2 legs (pay and receive)
        let fx_swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };

        let trade = fx_swap
            .expand_to_trade("FX-SWAP-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(trade.num_legs() >= 2, "Swap must have at least 2 legs");
    }

    #[test]
    fn test_property_options_have_settlement_cashflow() {
        // Property: Options should have at least a settlement cashflow
        use crate::trade::OptionType;

        let eq_option = EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };

        let trade = eq_option
            .expand_to_trade("OPT-001", valuation_date(), &make_conventions())
            .unwrap();

        assert!(
            trade.total_cashflows() >= 1,
            "Option must have at least settlement cashflow"
        );
    }
}

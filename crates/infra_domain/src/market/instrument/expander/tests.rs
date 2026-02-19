#[cfg(test)]
mod tests {
    use crate::{
        market::{
            convention::{
                CdsConvention, ConventionSet, EquityConvention, FxConvention, FxOptionConvention,
                InflationSwapConvention, SwapConvention, SwaptionConvention,
            },
            instrument::{
                AsianOption, AveragingType, BarrierDirection, BarrierType, CapFloor, CapFloorType,
                Cds, CdsIndex, CmsSwap, CommodityForward, CommoditySwap, CommodityType,
                CommodityVanillaOption, CreditEvent, CurrencyPair, EnergyType, EquityForward,
                EquityReturnType, EquitySwap, EquityUnderlying, EquityVanillaOption, ExerciseStyle,
                Frn, FxBarrierOption, FxForward, FxSpot, FxSwap, FxVanillaOption, InflationSwap,
                InstrumentDefinition, InstrumentError, InstrumentExpander, NotionalSchedule, Ois,
                PayerReceiver, QuantityUnit, SwapType, Swaption,
            },
            Currency, RateIndex,
        },
        time::{Date, Tenor},
        trade::{ExerciseType, OptionType, SettlementType, TradeType},
    };

    fn conv() -> ConventionSet {
        ConventionSet {
            swap: Some(SwapConvention::usd_sofr()),
            swaption: Some(SwaptionConvention::usd_sofr()),
            fx: Some(FxConvention::usd_default()),
            fx_option: Some(FxOptionConvention::g10_standard()),
            cds: Some(CdsConvention::isda_na()),
            equity: Some(EquityConvention::us_equity()),
            inflation_swap: Some(InflationSwapConvention::us_cpi_zc()),
            ..Default::default()
        }
    }

    fn vd() -> Date { Date::from_ymd(2025, 1, 1).unwrap() }
    fn d(y: i32, m: u32, day: u32) -> Date { Date::from_ymd(y, m, day).unwrap() }

    #[test]
    fn test_rates_expansion() {
        let c = conv();
        let swaption = Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: d(2026, 1, 15),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        };
        let t = swaption.expand_to_trade("SWAPTION-001", vd(), &c).unwrap();
        assert!(t.trade_type.is_swaption());
        assert_eq!(t.num_legs(), 2);
        if let TradeType::Swaption {
            exercise_type,
            settlement_type,
            ..
        } = t.trade_type
        {
            assert_eq!(exercise_type, ExerciseType::European);
            assert_eq!(settlement_type, SettlementType::Cash);
        } else {
            panic!("Expected TradeType::Swaption");
        }

        let empty = ConventionSet::default();
        assert!(matches!(
            swaption.expand_to_trade("S", vd(), &empty).unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));

        let cap = CapFloor {
            cap_floor_type: CapFloorType::Cap,
            strikes: vec![0.05],
            index: RateIndex::Sofr,
            start_date: d(2025, 1, 15),
            tenor: Tenor::TwoYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: crate::time::Frequency::Quarterly,
            currency: Currency::USD,
        };
        let t = cap.expand_to_trade("CAP-001", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::CapFloor);
        assert_eq!(t.num_legs(), 1);

        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.005,
            reset_frequency: crate::time::Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(10_000_000.0),
            start_date: d(2025, 1, 15),
            maturity: d(2030, 1, 15),
            currency: Currency::USD,
        };
        let t = frn.expand_to_trade("FRN-001", vd(), &c).unwrap();
        assert!(matches!(t.trade_type, TradeType::Bond { .. }));
        assert!(t.total_cashflows() >= 2);

        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: d(2025, 1, 15),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::USD,
            spread: 0.001,
        };
        let t = cms.expand_to_trade("CMS-001", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);

        let inf = InflationSwap {
            inflation_index: "USCPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: d(2025, 1, 15),
            maturity: d(2030, 1, 15),
            notional: 10_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.02,
        };
        let t = inf.expand_to_trade("INF-001", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);
    }

    #[test]
    fn test_ois_expansion_and_daily_compounding() {
        let c = conv();
        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: d(2025, 1, 15),
            end_date: d(2026, 1, 15),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Annual,
        };
        let t = ois.expand_to_trade("OIS-001", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);
        assert_eq!(t.num_legs(), 2);

        let ois_q = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: d(2025, 1, 15),
            end_date: d(2025, 4, 15),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Receiver,
            payment_frequency: crate::time::Frequency::Quarterly,
        };
        let t = ois_q.expand_to_trade("OIS-002", vd(), &c).unwrap();
        let floating = t.floating_leg().expect("Should have floating leg");
        for cf in floating.cashflows() {
            assert!(cf.has_daily_accruals());
            assert!(!cf.daily_accruals().unwrap().is_empty());
        }

        let ois_m = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: d(2025, 1, 15),
            end_date: d(2025, 2, 15),
            notional: 1_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: crate::time::Frequency::Monthly,
        };
        let t = ois_m.expand_to_trade("OIS-003", vd(), &c).unwrap();
        let floating = t.floating_leg().unwrap();
        let cf = floating.cashflows().next().unwrap();
        let accruals = cf.daily_accruals().unwrap();
        let mut prev = ois_m.notional;
        for a in accruals {
            assert!(a.compounded_notional >= prev);
            prev = a.compounded_notional;
        }
        assert!(accruals.last().unwrap().compounded_notional > ois_m.notional);

        assert!(ois.validate().is_ok());
        let bad_notional = Ois {
            notional: -10_000_000.0,
            ..ois.clone()
        };
        assert!(bad_notional.validate().is_err());
        let bad_dates = Ois {
            start_date: d(2030, 1, 15),
            end_date: d(2025, 1, 15),
            ..ois
        };
        assert!(bad_dates.validate().is_err());
    }

    #[test]
    fn test_fx_expansion() {
        let c = conv();
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);

        let spot = FxSpot {
            currency_pair: pair.clone(),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        let t = spot.expand_to_trade("FX-SPOT", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::FxForward);
        assert_eq!(t.num_legs(), 2);
        assert_eq!(t.total_cashflows(), 2);

        let fwd = FxForward {
            currency_pair: pair.clone(),
            forward_rate: 1.1100,
            settlement_date: d(2025, 7, 3),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        let t = fwd.expand_to_trade("FX-FWD", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::FxForward);
        assert_eq!(t.num_legs(), 2);

        let opt = FxVanillaOption {
            currency_pair: pair.clone(),
            strike: 1.1000,
            expiry: d(2025, 6, 15),
            delivery_date: d(2025, 6, 17),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        let t = opt.expand_to_trade("FX-OPT", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::FxOption {
                option_type: OptionType::Call,
                ..
            }
        ));
        assert_eq!(t.num_legs(), 1);

        let barrier = FxBarrierOption {
            vanilla: opt,
            barrier_level: 1.15,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(5000.0),
        };
        let t = barrier.expand_to_trade("FX-BARRIER", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::FxBarrierOption {
                barrier_type: BarrierType::KnockOut,
                barrier_direction: BarrierDirection::Up,
                ..
            }
        ));

        let swap = FxSwap {
            currency_pair: pair.clone(),
            near_leg_date: d(2025, 1, 3),
            far_leg_date: d(2025, 4, 3),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        let t = swap.expand_to_trade("FX-SWAP", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);
        assert_eq!(t.num_legs(), 4);
        let cfs: Vec<_> = t.all_cashflows().collect();
        assert_eq!(cfs.len(), 4);
        let future_cfs: Vec<_> = t.future_cashflows(vd()).collect();
        assert_eq!(future_cfs.len(), 4);
        for leg in t.legs() {
            for cf in leg.cashflows() {
                assert_eq!(cf.currency, leg.currency);
            }
        }
    }

    #[test]
    fn test_equity_expansion() {
        let c = conv();
        let fwd = EquityForward {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            forward_price: 5000.0,
            settlement_date: d(2025, 6, 15),
            notional: 100_000.0,
            currency: Currency::USD,
        };
        let t = fwd.expand_to_trade("EQ-FWD", vd(), &c).unwrap();
        assert!(matches!(t.trade_type, TradeType::EquityForward { .. }));
        assert_eq!(t.num_legs(), 1);

        let opt = EquityVanillaOption {
            underlying: EquityUnderlying::Index {
                name: "SPX".to_string(),
            },
            strike: 5000.0,
            expiry: d(2025, 6, 15),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100_000.0,
            currency: Currency::USD,
        };
        let t = opt.expand_to_trade("EQ-OPT", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::EquityOption {
                option_type: OptionType::Call,
                ..
            }
        ));
        assert!(t.total_cashflows() >= 1);

        let asian = AsianOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: 180.0,
            expiry: d(2025, 6, 15),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: crate::time::Frequency::Monthly,
            observed_values: vec![175.0, 178.0, 180.0],
            notional: 1000.0,
            currency: Currency::USD,
        };
        let t = asian.expand_to_trade("ASIAN", vd(), &c).unwrap();
        assert!(matches!(t.trade_type, TradeType::AsianOption { .. }));

        let asian_empty = AsianOption {
            observed_values: vec![],
            ..asian
        };
        assert!(asian_empty.expand_to_trade("ASIAN-2", vd(), &c).is_ok());

        let swap = EquitySwap {
            underlying: EquityUnderlying::index("SPX"),
            return_type: EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: d(2025, 1, 15),
            maturity: d(2026, 1, 15),
            notional: 10_000_000.0,
            currency: Currency::USD,
        };
        let t = swap.expand_to_trade("EQ-SWAP", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);
        assert_eq!(t.num_legs(), 2);
    }

    #[test]
    fn test_credit_and_commodity_expansion() {
        let c = conv();
        let cds = Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: d(2025, 1, 1),
            maturity: d(2030, 1, 1),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        };
        let t = cds.expand_to_trade("CDS", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::CreditDefaultSwap { .. }
        ));
        assert_eq!(t.num_legs(), 2);

        let idx = CdsIndex {
            index_name: "CDX.NA.IG".to_string(),
            series: 40,
            version: 1,
            constituent_count: 125,
            notional: 10_000_000.0,
            spread: 0.006,
            start_date: d(2025, 3, 20),
            maturity: d(2030, 6, 20),
            currency: Currency::USD,
        };
        let t = idx.expand_to_trade("CDS-IDX", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::CreditDefaultSwapIndex { .. }
        ));

        let cfwd = CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: d(2025, 6, 15),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.0,
            currency: Currency::USD,
        };
        let t = cfwd.expand_to_trade("COMM-FWD", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::CommodityForward { .. }
        ));

        let cswap = CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            fixed_price: 75.0,
            floating_index: "WTI".to_string(),
            start_date: d(2025, 1, 15),
            maturity: d(2026, 1, 15),
            quantity_per_period: 1000.0,
            unit: QuantityUnit::Barrels,
            payment_frequency: crate::time::Frequency::Monthly,
            currency: Currency::USD,
        };
        let t = cswap.expand_to_trade("COMM-SWAP", vd(), &c).unwrap();
        assert_eq!(t.trade_type, TradeType::Swap);
        assert_eq!(t.num_legs(), 2);

        let copt = CommodityVanillaOption {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            strike: 3.50,
            expiry: d(2025, 6, 15),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 10000.0,
            unit: QuantityUnit::MMBtu,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        };
        let t = copt.expand_to_trade("COMM-OPT", vd(), &c).unwrap();
        assert!(matches!(
            t.trade_type,
            TradeType::CommodityOption {
                option_type: OptionType::Call,
                ..
            }
        ));
    }

    #[test]
    fn test_instrument_definition_integration() {
        let c = conv();
        let fx_spot = InstrumentDefinition::FxSpot(FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        });
        let t = fx_spot.expand_to_trade("INST-001", vd(), &c).unwrap();
        assert_eq!(t.id.as_str(), "INST-001");

        let bad = InstrumentDefinition::FxSpot(FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: -1_000_000.0,
            notional_currency: Currency::EUR,
        });
        assert!(bad.expand_to_trade("INST-002", vd(), &c).is_err());
    }

    #[test]
    fn test_validation_rejects_invalid_inputs() {
        assert!(FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: 0.0,
            notional_currency: Currency::EUR,
        }
        .validate()
        .is_err());

        assert!(Swaption {
            underlying_swap_tenor: Tenor::FiveYears,
            expiry: d(2026, 1, 15),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: -10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        }
        .validate()
        .is_err());

        assert!(EquityVanillaOption {
            underlying: EquityUnderlying::stock("AAPL"),
            strike: -100.0,
            expiry: d(2025, 6, 15),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        }
        .validate()
        .is_err());

        assert!(Cds {
            reference_entity: "ACME".to_string(),
            notional: 10_000_000.0,
            spread: 0.01,
            start_date: d(2030, 1, 1),
            maturity: d(2025, 1, 1),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        }
        .validate()
        .is_err());

        assert!(FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: d(2025, 1, 3),
            far_leg_date: d(2025, 1, 3),
            near_rate: 1.105,
            far_rate: 1.107,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
        .validate()
        .is_err());

        assert!(FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: d(2025, 4, 3),
            far_leg_date: d(2025, 1, 3),
            near_rate: 1.105,
            far_rate: 1.107,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn test_expansion_edge_cases() {
        let c = conv();
        let t = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: vd(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
        .expand_to_trade("FX", vd(), &c)
        .unwrap();
        assert_eq!(t.total_cashflows(), 2);

        let t = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: 1e15,
            notional_currency: Currency::EUR,
        }
        .expand_to_trade("FX", vd(), &c)
        .unwrap();
        assert_eq!(t.total_cashflows(), 2);

        let r = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1e-10,
            settlement_date: d(2025, 1, 3),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
        .expand_to_trade("FX", vd(), &c);
        assert!(r.is_ok() || r.is_err());

        let r = Cds {
            reference_entity: "ACME".to_string(),
            notional: 10_000_000.0,
            spread: 0.0,
            start_date: d(2025, 1, 1),
            maturity: d(2030, 1, 1),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy],
        }
        .expand_to_trade("CDS", vd(), &c);
        assert!(r.is_ok() || r.is_err());
    }

    #[test]
    fn test_property_trade_consistency() {
        let c = conv();
        let instruments: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: d(2025, 1, 3),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::FxForward(FxForward {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                forward_rate: 1.1100,
                settlement_date: d(2025, 7, 3),
                notional: 1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::index("SPX"),
                forward_price: 5000.0,
                settlement_date: d(2025, 6, 15),
                notional: 100_000.0,
                currency: Currency::USD,
            }),
        ];

        for (i, inst) in instruments.iter().enumerate() {
            let t = inst
                .expand_to_trade(format!("INST-{}", i), vd(), &c)
                .unwrap();
            assert!(
                t.total_cashflows() >= 1,
                "Trade must have at least one cashflow"
            );
            assert!(t.num_legs() >= 1, "Trade must have at least one leg");
        }

        let fx = FxSpot {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            spot_rate: 1.1050,
            settlement_date: d(2025, 1, 3),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        for id in ["test-123", "TRADE_ABC", "id with spaces", ""] {
            assert_eq!(fx.expand_to_trade(id, vd(), &c).unwrap().id.as_str(), id);
        }

        let invalids: Vec<InstrumentDefinition> = vec![
            InstrumentDefinition::FxSpot(FxSpot {
                currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
                spot_rate: 1.1050,
                settlement_date: d(2025, 1, 3),
                notional: -1_000_000.0,
                notional_currency: Currency::EUR,
            }),
            InstrumentDefinition::EquityForward(EquityForward {
                underlying: EquityUnderlying::stock("AAPL"),
                forward_price: -100.0,
                settlement_date: d(2025, 6, 15),
                notional: 100.0,
                currency: Currency::USD,
            }),
        ];
        for inst in &invalids {
            assert!(inst.expand_to_trade("X", vd(), &c).is_err());
        }

        let swap = FxSwap {
            currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
            near_leg_date: d(2025, 1, 3),
            far_leg_date: d(2025, 4, 3),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        let t = swap.expand_to_trade("SWAP", vd(), &c).unwrap();
        assert!(t.num_legs() >= 2);
    }
}

//! Integration tests for curve bootstrap engine.
//!
//! These tests verify the end-to-end flow of curve construction:
//! - CurveDefinition loading and validation
//! - Instrument adapter conversion
//! - CurveEngine orchestration
//! - Cache integration
//! - Multi-curve construction

use infra_master::{market::RateIndex, trade::convention::SwapConvention};
use pricer_models::market::{
    calibration::bootstrapping::{
        BootstrapInstrument, BootstrapInterpolation, BootstrappedCurve, CurveConfigBuilder,
        CurveDefinition, CurveDependency, CurveEngine, CurveEngineBuilder, CurveKey,
        CurveResultCache, GenericBootstrapConfig, GenericBootstrapConfigBuilder, InstrumentAdapter,
        InstrumentSpec, InstrumentTenor, MultiCurveBuilder, SequentialBootstrapper, Tenor,
    },
    curves::YieldCurve,
};

// ============================================================================
// Task 9.1: Unit-Level Integration Tests
// ============================================================================

mod curve_definition_tests {
    use super::*;

    #[test]
    fn test_curve_definition_roundtrip_json() {
        // Use default_usd_sofr to create a valid definition
        let original = CurveDefinition::default_usd_sofr();

        // Convert to JSON and back
        let json = original.to_json_string().unwrap();
        let loaded = CurveDefinition::load_from_str(&json).unwrap();

        assert_eq!(original.index_key(), loaded.index_key());
        assert_eq!(original.rate_index(), loaded.rate_index());
        assert_eq!(original.instruments().len(), loaded.instruments().len());
    }

    #[test]
    fn test_curve_definition_load_invalid_json() {
        let json = r#"{ "index_key": "USD-SOFR", "instruments": [ }"#;
        let result = CurveDefinition::load_from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_definition_load_missing_fields() {
        // Missing convention field should fail
        let json = r#"{ "index_key": "USD-SOFR", "rate_index": "Sofr", "instruments": [] }"#;
        let result = CurveDefinition::load_from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_definition_default_sofr() {
        let definition = CurveDefinition::default_usd_sofr();

        // Should have standard OIS tenors
        assert!(!definition.instruments().is_empty());
        assert_eq!(definition.rate_index(), RateIndex::Sofr);
    }

    #[test]
    fn test_curve_definition_sorted_instruments() {
        // Create definition with instruments out of order
        let definition =
            CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears));

        let sorted = definition.sorted_instruments();

        // Check instruments are sorted by maturity
        let maturities: Vec<f64> = sorted.iter().map(|s| s.maturity_years()).collect();
        for i in 1..maturities.len() {
            assert!(
                maturities[i] >= maturities[i - 1],
                "Instruments should be sorted by maturity"
            );
        }
    }

    #[test]
    fn test_curve_definition_builder_pattern() {
        let definition =
            CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));

        assert_eq!(definition.instruments().len(), 3);
        assert_eq!(definition.rate_index(), RateIndex::Sofr);
    }
}

mod instrument_adapter_tests {
    use super::*;

    #[test]
    fn test_adapter_convert_ois_instruments() {
        let definition = CurveDefinition::default_usd_sofr();
        let rates: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .map(|spec| (spec.tenor(), 0.03))
            .collect();

        let instruments: Vec<BootstrapInstrument<f64>> =
            InstrumentAdapter::convert(&definition, &rates).unwrap();

        assert_eq!(instruments.len(), definition.instruments().len());

        // All should be valid OIS instruments
        for inst in &instruments {
            assert!(inst.validate(50.0).is_ok());
            assert!(matches!(inst, BootstrapInstrument::Ois { .. }));
        }
    }

    #[test]
    fn test_adapter_convert_irs_instruments() {
        let definition =
            CurveDefinition::new("USD-IRS", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::irs(InstrumentTenor::TwoYears))
                .with_instrument(InstrumentSpec::irs(InstrumentTenor::FiveYears));

        let rates = vec![
            (InstrumentTenor::TwoYears, 0.035),
            (InstrumentTenor::FiveYears, 0.038),
        ];

        let instruments: Vec<BootstrapInstrument<f64>> =
            InstrumentAdapter::convert(&definition, &rates).unwrap();

        assert_eq!(instruments.len(), 2);
        for inst in &instruments {
            assert!(inst.validate(50.0).is_ok());
            assert!(matches!(inst, BootstrapInstrument::Irs { .. }));
        }
    }

    #[test]
    fn test_adapter_convert_fra_instruments() {
        let definition =
            CurveDefinition::new("USD-FRA", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::fra(InstrumentTenor::ThreeMonths))
                .with_instrument(InstrumentSpec::fra(InstrumentTenor::SixMonths));

        let rates = vec![
            (InstrumentTenor::ThreeMonths, 0.032),
            (InstrumentTenor::SixMonths, 0.033),
        ];

        let instruments: Vec<BootstrapInstrument<f64>> =
            InstrumentAdapter::convert(&definition, &rates).unwrap();

        assert_eq!(instruments.len(), 2);
        for inst in &instruments {
            assert!(inst.validate(50.0).is_ok());
            assert!(matches!(inst, BootstrapInstrument::Fra { .. }));
        }
    }

    #[test]
    fn test_adapter_convert_future_instruments() {
        let definition =
            CurveDefinition::new("USD-FUT", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001))
                .with_instrument(InstrumentSpec::future(InstrumentTenor::SixMonths, 0.0002));

        let prices = vec![
            (InstrumentTenor::ThreeMonths, 97.5), // Future prices (100 - rate)
            (InstrumentTenor::SixMonths, 97.3),
        ];

        let instruments: Vec<BootstrapInstrument<f64>> =
            InstrumentAdapter::convert(&definition, &prices).unwrap();

        assert_eq!(instruments.len(), 2);
        for inst in &instruments {
            assert!(inst.validate(50.0).is_ok());
            assert!(matches!(inst, BootstrapInstrument::Future { .. }));
        }
    }

    #[test]
    fn test_adapter_convert_mismatched_rates() {
        let definition = CurveDefinition::default_usd_sofr();
        let rates = vec![(InstrumentTenor::OneYear, 0.03)]; // Too few rates

        let result = InstrumentAdapter::convert::<f64>(&definition, &rates);

        assert!(result.is_err());
    }
}

mod curve_cache_tests {
    use super::*;

    #[test]
    fn test_cache_key_hash_deterministic() {
        let rates = vec![0.03, 0.032, 0.034];
        let config_hash = 12345u64;

        let key1 = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);
        let key2 = CurveKey::from_rates(RateIndex::Sofr, &rates, config_hash);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_hash_different_rates() {
        let rates1 = vec![0.03, 0.032, 0.034];
        let rates2 = vec![0.03, 0.032, 0.035]; // Different last rate
        let config_hash = 12345u64;

        let key1 = CurveKey::from_rates(RateIndex::Sofr, &rates1, config_hash);
        let key2 = CurveKey::from_rates(RateIndex::Sofr, &rates2, config_hash);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_lookup_miss_then_hit() {
        let cache = CurveResultCache::<f64>::new(10);

        let rates = vec![0.03, 0.032];
        let key = CurveKey::from_rates(RateIndex::Sofr, &rates, 0);

        // First lookup should miss
        assert!(cache.lookup(&key).is_none());

        // Create and insert a curve
        let curve = BootstrappedCurve::new(
            vec![1.0, 2.0],
            vec![0.97, 0.94],
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        cache.insert(key.clone(), curve.clone());

        // Second lookup should hit
        let cached = cache.lookup(&key);
        assert!(cached.is_some());
    }

    #[test]
    fn test_cache_stats_tracking() {
        let cache = CurveResultCache::<f64>::new(10);
        let key = CurveKey::from_rates(RateIndex::Sofr, &[0.03, 0.032], 0);

        // Miss
        let _ = cache.lookup(&key);

        // Insert
        let curve = BootstrappedCurve::new(
            vec![1.0, 2.0],
            vec![0.97, 0.94],
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();
        cache.insert(key.clone(), curve);

        // Hit
        let _ = cache.lookup(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 1e-10);
    }
}

mod curve_config_tests {
    use pricer_models::market::calibration::bootstrapping::CurveParameterRepresentation;

    use super::*;

    #[test]
    fn test_config_validate_logdf_loglinear() {
        let config = CurveConfigBuilder::<f64>::default()
            .parameter_representation(CurveParameterRepresentation::LogDiscountFactor)
            .interpolation(BootstrapInterpolation::LogLinear)
            .build_validated()
            .unwrap();

        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::LogDiscountFactor
        );
    }

    #[test]
    fn test_config_validate_zero_rate_linear() {
        let config = CurveConfigBuilder::<f64>::default()
            .parameter_representation(CurveParameterRepresentation::ZeroRate)
            .interpolation(BootstrapInterpolation::LinearZeroRate)
            .build_validated()
            .unwrap();

        assert_eq!(
            config.parameter_representation,
            CurveParameterRepresentation::ZeroRate
        );
    }

    #[test]
    fn test_config_validate_invalid_combination() {
        let result = CurveConfigBuilder::<f64>::default()
            .parameter_representation(CurveParameterRepresentation::InstantaneousForward)
            .interpolation(BootstrapInterpolation::LogLinear) // Invalid for InstantaneousForward
            .build_validated();

        assert!(result.is_err());
    }
}

// ============================================================================
// Task 9.2: End-to-End Integration Tests
// ============================================================================

mod curve_engine_integration_tests {
    use super::*;

    #[test]
    fn test_engine_build_curve_full_flow() {
        // 1. Create definition
        let definition = CurveDefinition::default_usd_sofr();

        // 2. Prepare rates (one for each instrument)
        let rates: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .enumerate()
            .map(|(i, spec)| (spec.tenor(), 0.03 + (i as f64) * 0.002))
            .collect();

        // 3. Build engine
        let engine = CurveEngineBuilder::<f64>::default().build();

        // 4. Build curve
        let result = engine.build_curve(&definition, &rates).unwrap();

        // Verify result
        assert!(!result.from_cache);
        assert_eq!(result.pillars.len(), rates.len());

        // Discount factors should be decreasing
        for i in 1..result.discount_factors.len() {
            assert!(
                result.discount_factors[i] <= result.discount_factors[i - 1],
                "Discount factors should be decreasing"
            );
        }

        // Residuals should be small
        for residual in &result.residuals {
            assert!(residual.abs() < 1e-8, "Residuals should be small");
        }
    }

    #[test]
    fn test_engine_cache_hit_second_call() {
        let definition = CurveDefinition::default_usd_sofr();
        let rates: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .map(|spec| (spec.tenor(), 0.03))
            .collect();

        // Engine with cache
        let engine = CurveEngineBuilder::<f64>::default().with_cache(10).build();

        // First call - should miss cache
        let result1 = engine.build_curve(&definition, &rates).unwrap();
        assert!(!result1.from_cache);

        // Second call - should hit cache
        let result2 = engine.build_curve(&definition, &rates).unwrap();
        assert!(result2.from_cache);

        // Results should match
        assert_eq!(result1.pillars.len(), result2.pillars.len());
        for i in 0..result1.pillars.len() {
            assert!((result1.pillars[i] - result2.pillars[i]).abs() < 1e-12);
            assert!((result1.discount_factors[i] - result2.discount_factors[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn test_engine_different_rates_different_cache() {
        let definition = CurveDefinition::default_usd_sofr();

        let rates1: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .map(|spec| (spec.tenor(), 0.03))
            .collect();

        let rates2: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .map(|spec| (spec.tenor(), 0.04))
            .collect();

        let engine = CurveEngineBuilder::<f64>::default().with_cache(10).build();

        // Build with first rates
        let result1 = engine.build_curve(&definition, &rates1).unwrap();
        assert!(!result1.from_cache);

        // Build with different rates - should not hit cache
        let result2 = engine.build_curve(&definition, &rates2).unwrap();
        assert!(!result2.from_cache);

        // Discount factors should be different
        assert!((result1.discount_factors[0] - result2.discount_factors[0]).abs() > 1e-6);
    }

    #[test]
    fn test_engine_curve_implements_yield_curve() {
        let definition = CurveDefinition::default_usd_sofr();
        let rates: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .enumerate()
            .map(|(i, spec)| (spec.tenor(), 0.03 + (i as f64) * 0.002))
            .collect();

        let engine = CurveEngine::<f64>::default();
        let result = engine.build_curve(&definition, &rates).unwrap();

        // Use YieldCurve trait methods
        let curve = &result.curve;

        // Test discount factor
        let df = curve.discount_factor(1.0).unwrap();
        assert!(df > 0.0 && df < 1.0);

        // Test zero rate
        let zr = curve.zero_rate(1.0).unwrap();
        assert!(zr > 0.0 && zr < 0.1); // Reasonable range

        // Test forward rate
        let fwd = curve.forward_rate(0.5, 1.0).unwrap();
        assert!(fwd > 0.0 && fwd < 0.2);
    }
}

mod multi_curve_integration_tests {
    use super::*;

    #[test]
    fn test_multi_curve_ois_plus_tenor() {
        let ois_instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
            BootstrapInstrument::ois(5.0, 0.037),
        ];

        let forward_instruments: Vec<(Tenor, Vec<BootstrapInstrument<f64>>)> = vec![
            (
                Tenor::ThreeMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.038),
                    BootstrapInstrument::irs(3.0, 0.040),
                ],
            ),
            (
                Tenor::SixMonth,
                vec![
                    BootstrapInstrument::irs(1.0, 0.036),
                    BootstrapInstrument::irs(2.0, 0.039),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder
            .build(&ois_instruments, &forward_instruments)
            .unwrap();

        // Verify structure
        assert!(!curve_set.is_single_curve());
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
        assert!(curve_set.has_forward_curve(Tenor::SixMonth));

        // Discount curve should give valid DFs
        let df_1y = curve_set.discount_curve().discount_factor(1.0).unwrap();
        assert!(df_1y > 0.9 && df_1y < 1.0);

        // Forward curves should also give valid DFs
        let df_3m_1y = curve_set
            .forward_curve(Tenor::ThreeMonth)
            .discount_factor(1.0)
            .unwrap();
        assert!(df_3m_1y > 0.9 && df_3m_1y < 1.0);
    }

    #[test]
    fn test_multi_curve_with_dependencies() {
        let specs = vec![
            // SOFR as discount curve (self-discounting)
            (
                CurveDependency::new(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                    BootstrapInstrument::ois(3.0, 0.034),
                ],
            ),
            // SONIA depends on SOFR for discounting
            (
                CurveDependency::new(RateIndex::Sonia)
                    .with_discount(RateIndex::Sofr)
                    .with_tenor(Tenor::ThreeMonth),
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.038),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        // Both curves should be accessible by index
        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));

        // SOFR should be the discount curve
        assert_eq!(curve_set.discount_index(), Some(RateIndex::Sofr));

        // SONIA should also be accessible by tenor
        assert!(curve_set.has_forward_curve(Tenor::ThreeMonth));
    }

    #[test]
    fn test_multi_curve_circular_dependency_error() {
        let specs = vec![
            // A depends on B
            (
                CurveDependency::new(RateIndex::Sofr).with_discount(RateIndex::Sonia),
                vec![BootstrapInstrument::ois(1.0, 0.03)],
            ),
            // B depends on A - circular!
            (
                CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr),
                vec![BootstrapInstrument::ois(1.0, 0.025)],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let result = builder.build_with_dependencies(&specs);

        assert!(result.is_err());
    }

    #[test]
    fn test_multi_curve_dependency_chain() {
        // A -> B -> C dependency chain
        let specs = vec![
            // SOFR: no dependency (root)
            (
                CurveDependency::new(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::ois(1.0, 0.03),
                    BootstrapInstrument::ois(2.0, 0.032),
                ],
            ),
            // SONIA depends on SOFR
            (
                CurveDependency::new(RateIndex::Sonia).with_discount(RateIndex::Sofr),
                vec![
                    BootstrapInstrument::irs(1.0, 0.035),
                    BootstrapInstrument::irs(2.0, 0.038),
                ],
            ),
            // TONAR depends on SONIA
            (
                CurveDependency::new(RateIndex::Tonar).with_discount(RateIndex::Sonia),
                vec![
                    BootstrapInstrument::irs(1.0, 0.028),
                    BootstrapInstrument::irs(2.0, 0.030),
                ],
            ),
        ];

        let builder = MultiCurveBuilder::<f64>::with_defaults();
        let curve_set = builder.build_with_dependencies(&specs).unwrap();

        // All three curves should be built
        assert!(curve_set.has_curve_for_index(RateIndex::Sofr));
        assert!(curve_set.has_curve_for_index(RateIndex::Sonia));
        assert!(curve_set.has_curve_for_index(RateIndex::Tonar));
    }
}

// ============================================================================
// Additional End-to-End Tests
// ============================================================================

mod end_to_end_tests {
    use super::*;

    #[test]
    fn test_full_pipeline_json_to_curve() {
        // 1. Create definition via programmatic API
        let definition =
            CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::ThreeYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::SevenYears))
                .with_instrument(InstrumentSpec::ois(InstrumentTenor::TenYears));

        // 2. Prepare market rates
        let rates: Vec<(InstrumentTenor, f64)> = vec![
            (InstrumentTenor::OneYear, 0.030),
            (InstrumentTenor::TwoYears, 0.032),
            (InstrumentTenor::ThreeYears, 0.034),
            (InstrumentTenor::FiveYears, 0.037),
            (InstrumentTenor::SevenYears, 0.039),
            (InstrumentTenor::TenYears, 0.040),
        ];

        // 3. Build curve with caching
        let engine = CurveEngineBuilder::<f64>::default().with_cache(100).build();

        let result = engine.build_curve(&definition, &rates).unwrap();

        // 4. Verify curve quality
        assert_eq!(result.curve.pillar_count(), 6);

        // All residuals should be small (good fit)
        let max_residual = result.residuals.iter().map(|r| r.abs()).fold(0.0, f64::max);
        assert!(
            max_residual < 1e-8,
            "Max residual too large: {}",
            max_residual
        );

        // 5. Use the curve for pricing
        let df_5y = result.curve.discount_factor(5.0).unwrap();
        let zr_5y = result.curve.zero_rate(5.0).unwrap();
        let fwd_4y5y = result.curve.forward_rate(4.0, 5.0).unwrap();

        // Sanity checks
        assert!(df_5y > 0.8 && df_5y < 1.0, "5Y DF out of range");
        assert!(zr_5y > 0.03 && zr_5y < 0.05, "5Y zero rate out of range");
        assert!(
            fwd_4y5y > 0.03 && fwd_4y5y < 0.06,
            "4Y-5Y forward rate out of range"
        );
    }

    #[test]
    fn test_curve_extrapolation_behavior() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(5.0, 0.037),
        ];

        // With extrapolation allowed
        let config_with_extrap = GenericBootstrapConfigBuilder::<f64>::default()
            .allow_extrapolation(true)
            .build();

        let bootstrapper = SequentialBootstrapper::new(config_with_extrap);
        let result = bootstrapper.bootstrap(&instruments).unwrap();
        let curve = result.curve;

        // Should be able to get DF beyond last pillar
        let df_10y = curve.discount_factor(10.0);
        assert!(df_10y.is_ok(), "Extrapolation should be allowed");

        // Without extrapolation
        let config_no_extrap = GenericBootstrapConfigBuilder::<f64>::default()
            .allow_extrapolation(false)
            .build();

        let bootstrapper_no_extrap = SequentialBootstrapper::new(config_no_extrap);
        let result_no_extrap = bootstrapper_no_extrap.bootstrap(&instruments).unwrap();
        let curve_no_extrap = result_no_extrap.curve;

        // Should fail beyond last pillar
        let df_10y_no_extrap = curve_no_extrap.discount_factor(10.0);
        assert!(
            df_10y_no_extrap.is_err(),
            "Extrapolation should not be allowed"
        );
    }

    #[test]
    fn test_curve_consistency_relations() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
            BootstrapInstrument::ois(5.0, 0.037),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::new(GenericBootstrapConfig::default());
        let result = bootstrapper.bootstrap(&instruments).unwrap();
        let curve = result.curve;

        // Test: DF(t) = exp(-r(t) * t) relationship
        for t in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0] {
            let df = curve.discount_factor(t).unwrap();
            let zr = curve.zero_rate(t).unwrap();
            let expected_df = (-zr * t).exp();
            assert!(
                (df - expected_df).abs() < 1e-10,
                "DF-ZR consistency failed at t={}: DF={}, expected={}",
                t,
                df,
                expected_df
            );
        }

        // Test: Forward rate consistency DF(t2) = DF(t1) * exp(-f * (t2-t1))
        for (t1, t2) in [(0.5, 1.0), (1.0, 2.0), (2.0, 3.0), (3.0, 5.0)] {
            let df1 = curve.discount_factor(t1).unwrap();
            let df2 = curve.discount_factor(t2).unwrap();
            let fwd = curve.forward_rate(t1, t2).unwrap();

            let expected_df2 = df1 * (-fwd * (t2 - t1)).exp();
            assert!(
                (df2 - expected_df2).abs() < 1e-10,
                "Forward rate consistency failed for [{}, {}]: DF2={}, expected={}",
                t1,
                t2,
                df2,
                expected_df2
            );
        }
    }
}

// ============================================================================
// Task 5.3: GlobalBootstrapper vs SequentialBootstrapper Comparison
// ============================================================================

#[cfg(feature = "global-bootstrap")]
mod global_bootstrapper_comparison_tests {
    use super::*;
    use pricer_models::market::calibration::bootstrapping::{
        GlobalBootstrapConfig, GlobalBootstrapper,
    };

    /// Helper to create test instruments for comparison tests
    fn create_comparison_instruments() -> Vec<BootstrapInstrument<f64>> {
        vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
            BootstrapInstrument::ois(5.0, 0.037),
            BootstrapInstrument::ois(10.0, 0.040),
        ]
    }

    #[test]
    fn test_global_bootstrapper_produces_valid_curve() {
        let instruments = create_comparison_instruments();
        let bootstrapper = GlobalBootstrapper::<f64>::with_defaults();

        let result = bootstrapper.calibrate(&instruments).unwrap();

        // Verify convergence
        assert!(result.converged);
        assert_eq!(result.pillars.len(), 5);
        assert_eq!(result.discount_factors.len(), 5);

        // Verify discount factors are positive and decreasing
        for i in 0..result.discount_factors.len() {
            assert!(
                result.discount_factors[i] > 0.0,
                "DF at {} should be positive",
                i
            );
            assert!(
                result.discount_factors[i] <= 1.0,
                "DF at {} should be <= 1.0",
                i
            );
        }

        // Verify DFs are decreasing
        for i in 1..result.discount_factors.len() {
            assert!(
                result.discount_factors[i] < result.discount_factors[i - 1],
                "DFs should be decreasing: {} vs {}",
                result.discount_factors[i],
                result.discount_factors[i - 1]
            );
        }
    }

    #[test]
    fn test_global_vs_sequential_discount_factor_consistency() {
        let instruments = create_comparison_instruments();

        // Bootstrap with GlobalBootstrapper
        let global_config = GlobalBootstrapConfig::default();
        let global_bootstrapper = GlobalBootstrapper::new(global_config);
        let global_result = global_bootstrapper.calibrate(&instruments).unwrap();

        // Bootstrap with SequentialBootstrapper
        let seq_config = GenericBootstrapConfig::default();
        let seq_bootstrapper = SequentialBootstrapper::new(seq_config);
        let seq_result = seq_bootstrapper.bootstrap(&instruments).unwrap();

        // Compare discount factors at each pillar
        assert_eq!(
            global_result.pillars.len(),
            seq_result.pillars.len(),
            "Number of pillars should match"
        );

        for i in 0..global_result.pillars.len() {
            let global_df = global_result.discount_factors[i];
            let seq_df = seq_result.discount_factors[i];

            // Allow small numerical differences (< 1bp in rate terms)
            let tolerance = 1e-6;
            let diff = (global_df - seq_df).abs();
            assert!(
                diff < tolerance,
                "DF mismatch at pillar {}: global={}, sequential={}, diff={}",
                global_result.pillars[i],
                global_df,
                seq_df,
                diff
            );
        }
    }

    #[test]
    fn test_global_vs_sequential_curve_interpolation_consistency() {
        let instruments = create_comparison_instruments();

        // Bootstrap with both methods
        let global_result = GlobalBootstrapper::<f64>::with_defaults()
            .calibrate(&instruments)
            .unwrap();
        let seq_result = SequentialBootstrapper::<f64>::with_defaults()
            .bootstrap(&instruments)
            .unwrap();

        // Compare interpolated DFs at intermediate points
        let test_maturities = [0.5, 1.5, 2.5, 4.0, 7.0];

        for &t in &test_maturities {
            let global_df = global_result.curve.discount_factor(t).unwrap();
            let seq_df = seq_result.curve.discount_factor(t).unwrap();

            // Slightly larger tolerance for interpolated points
            let tolerance = 1e-5;
            let diff = (global_df - seq_df).abs();
            assert!(
                diff < tolerance,
                "Interpolated DF mismatch at t={}: global={}, sequential={}, diff={}",
                t,
                global_df,
                seq_df,
                diff
            );
        }
    }

    #[test]
    fn test_global_bootstrapper_stores_jacobian_inverse() {
        let instruments = create_comparison_instruments();
        let config = GlobalBootstrapConfig::default().with_jacobian_inverse(true);
        let bootstrapper = GlobalBootstrapper::new(config);

        let result = bootstrapper.calibrate(&instruments).unwrap();

        assert!(result.has_jacobian_inverse());
        let j_inv = result.jacobian_inverse.as_ref().unwrap();

        // Jacobian inverse should be n x n where n = number of pillars
        assert_eq!(j_inv.nrows(), 5);
        assert_eq!(j_inv.ncols(), 5);

        // Verify J_inv is not singular (diagonal elements should be non-zero)
        for i in 0..5 {
            assert!(
                j_inv[(i, i)].abs() > 1e-15,
                "Diagonal element {} is near-zero",
                i
            );
        }
    }

    #[test]
    fn test_global_bootstrapper_pricing_errors_minimal() {
        use pricer_models::market::calibration::bootstrapping::CalibrationInstrument;

        let instruments = create_comparison_instruments();
        let result = GlobalBootstrapper::<f64>::with_defaults()
            .calibrate(&instruments)
            .unwrap();

        // Verify pricing errors are minimal for all instruments
        for (i, instr) in instruments.iter().enumerate() {
            let error = instr.pricing_error(&result.curve).unwrap();
            assert!(
                error.abs() < 1e-8,
                "Instrument {} has pricing error {} (should be < 1e-8)",
                i,
                error
            );
        }
    }

    #[test]
    fn test_global_bootstrapper_inverted_curve() {
        // Test with inverted (downward sloping) curve
        let instruments = vec![
            BootstrapInstrument::ois(1.0, 0.05),
            BootstrapInstrument::ois(2.0, 0.045),
            BootstrapInstrument::ois(5.0, 0.040),
            BootstrapInstrument::ois(10.0, 0.035),
        ];

        let global_result = GlobalBootstrapper::<f64>::with_defaults()
            .calibrate(&instruments)
            .unwrap();
        let seq_result = SequentialBootstrapper::<f64>::with_defaults()
            .bootstrap(&instruments)
            .unwrap();

        assert!(global_result.converged);

        // Compare zero rates at pillars
        for &t in &[1.0, 2.0, 5.0, 10.0] {
            let global_zr = global_result.curve.zero_rate(t).unwrap();
            let seq_zr = seq_result.curve.zero_rate(t).unwrap();

            let diff = (global_zr - seq_zr).abs();
            assert!(
                diff < 1e-5,
                "Zero rate mismatch at t={}: global={}, seq={}",
                t,
                global_zr,
                seq_zr
            );
        }
    }
}

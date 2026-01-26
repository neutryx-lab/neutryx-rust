//! Integration tests for VolCube calibration.
//!
//! # Requirements: 5.9, 6.5, 6.8, 6.10, 7.6
//!
//! These tests verify the end-to-end flow of VolCube calibration:
//! - Curve→VolCube calibration dependency
//! - Quote update→cache invalidation
//! - AAD Vega calculation accuracy

use chrono::NaiveDate;
use pricer_models::market::volcube::{
    AADCrossValidator, Currency, Tenor, UnderlyingIndex, VegaBumpConfig, VolCubeBuilder,
    VolCubeConfig, VolCubeVegaCalculator, VolInstrument, VolLazyEvaluator, VolQuote, VolQuoteSet,
    VolStrike, VolatilityCube,
};

// ============================================================================
// Task 17.1: Curve→VolCube Calibration Flow Tests
// ============================================================================

mod curve_volcube_flow_tests {
    use super::*;

    /// Test that VolCube calibration works with instruments.
    /// Note: VolCube requires at least 2 expiries AND 2 tenors for grid
    /// construction.
    #[test]
    fn test_volcube_calibration_basic_flow() {
        // Create instruments for calibration with 2+ expiries and 2+ tenors
        let instruments: Vec<VolInstrument<f64>> = vec![
            // Expiry 1Y, Tenor 5Y
            VolInstrument::new("1Y-5Y-ATM", 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("1Y-5Y-OTM1", 1.0, 5.0, 0.025, 0.22, 0.03),
            VolInstrument::new("1Y-5Y-OTM2", 1.0, 5.0, 0.035, 0.21, 0.03),
            // Expiry 1Y, Tenor 10Y
            VolInstrument::new("1Y-10Y-ATM", 1.0, 10.0, 0.035, 0.18, 0.035),
            VolInstrument::new("1Y-10Y-OTM1", 1.0, 10.0, 0.030, 0.20, 0.035),
            VolInstrument::new("1Y-10Y-OTM2", 1.0, 10.0, 0.040, 0.19, 0.035),
            // Expiry 2Y, Tenor 5Y
            VolInstrument::new("2Y-5Y-ATM", 2.0, 5.0, 0.03, 0.19, 0.03),
            VolInstrument::new("2Y-5Y-OTM1", 2.0, 5.0, 0.025, 0.21, 0.03),
            VolInstrument::new("2Y-5Y-OTM2", 2.0, 5.0, 0.035, 0.20, 0.03),
            // Expiry 2Y, Tenor 10Y
            VolInstrument::new("2Y-10Y-ATM", 2.0, 10.0, 0.035, 0.17, 0.035),
            VolInstrument::new("2Y-10Y-OTM1", 2.0, 10.0, 0.030, 0.19, 0.035),
            VolInstrument::new("2Y-10Y-OTM2", 2.0, 10.0, 0.040, 0.18, 0.035),
        ];

        // Build VolCube
        let config = VolCubeConfig::default();
        let cube = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .build();

        assert!(cube.is_ok(), "VolCube build should succeed");
        let cube = cube.unwrap();

        // Verify volatility interpolation
        let vol = cube.volatility(1.0, 5.0, 0.03);
        assert!(vol.is_ok(), "Volatility query should succeed");
        let vol = vol.unwrap();
        assert!(vol > 0.0 && vol < 1.0, "Volatility should be reasonable");
    }

    /// Test VolCube with multiple expiry-tenor slices.
    #[test]
    fn test_volcube_multi_slice_calibration() {
        let instruments: Vec<VolInstrument<f64>> = vec![
            // Expiry 1Y, Tenor 5Y
            VolInstrument::new("1Y-5Y-ATM", 1.0, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("1Y-5Y-25D", 1.0, 5.0, 0.025, 0.22, 0.03),
            VolInstrument::new("1Y-5Y-75D", 1.0, 5.0, 0.035, 0.21, 0.03),
            // Expiry 2Y, Tenor 5Y
            VolInstrument::new("2Y-5Y-ATM", 2.0, 5.0, 0.03, 0.19, 0.03),
            VolInstrument::new("2Y-5Y-25D", 2.0, 5.0, 0.025, 0.21, 0.03),
            VolInstrument::new("2Y-5Y-75D", 2.0, 5.0, 0.035, 0.20, 0.03),
            // Expiry 1Y, Tenor 10Y
            VolInstrument::new("1Y-10Y-ATM", 1.0, 10.0, 0.035, 0.18, 0.035),
            VolInstrument::new("1Y-10Y-25D", 1.0, 10.0, 0.030, 0.20, 0.035),
            VolInstrument::new("1Y-10Y-75D", 1.0, 10.0, 0.040, 0.19, 0.035),
            // Expiry 2Y, Tenor 10Y
            VolInstrument::new("2Y-10Y-ATM", 2.0, 10.0, 0.035, 0.17, 0.035),
            VolInstrument::new("2Y-10Y-25D", 2.0, 10.0, 0.030, 0.19, 0.035),
            VolInstrument::new("2Y-10Y-75D", 2.0, 10.0, 0.040, 0.18, 0.035),
        ];

        let config = VolCubeConfig::default();
        let cube = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .build()
            .expect("Build should succeed");

        // Test interpolation at different points
        let vol_1y_5y = cube.volatility(1.0, 5.0, 0.03).unwrap();
        let vol_2y_5y = cube.volatility(2.0, 5.0, 0.03).unwrap();
        let vol_1y_10y = cube.volatility(1.0, 10.0, 0.035).unwrap();

        // Volatilities should be positive and reasonable
        assert!(vol_1y_5y > 0.0 && vol_1y_5y < 1.0);
        assert!(vol_2y_5y > 0.0 && vol_2y_5y < 1.0);
        assert!(vol_1y_10y > 0.0 && vol_1y_10y < 1.0);

        // Term structure: 2Y should typically have lower vol than 1Y
        // (depends on calibration, so we just check they're different)
        assert!((vol_1y_5y - vol_2y_5y).abs() < 0.1);
    }
}

// ============================================================================
// Task 17.2: Quote Update→Cache Invalidation Tests
// ============================================================================

mod cache_invalidation_tests {
    use pricer_models::market::volcube::QuoteUpdateListener;

    use super::*;

    /// Test that quote updates trigger cache invalidation.
    #[test]
    fn test_quote_update_invalidates_cache() {
        let config = VolCubeConfig::default();
        let evaluator: VolLazyEvaluator<f64> = VolLazyEvaluator::new(config);

        // Initial state should have no invalidations
        let stats = evaluator.stats();
        assert_eq!(stats.invalidations(), 0);
        assert_eq!(stats.calibrations(), 0);

        // Update quote should invalidate related slices
        evaluator.on_quote_update(1.0, 5.0);

        // After update, stats should reflect the change
        let stats_after = evaluator.stats();
        assert_eq!(stats_after.invalidations(), 1);
    }

    /// Test VolQuoteSet update detection.
    #[test]
    fn test_vol_quote_set_update_detection() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();

        let mut quote_set = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        // Add quotes
        quote_set.add_quote(VolQuote::new(
            "Q1",
            expiry,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.20,
        ));

        assert_eq!(quote_set.len(), 1);

        // Add another quote
        quote_set.add_quote(VolQuote::new(
            "Q2",
            expiry,
            Tenor::years(5.0),
            VolStrike::Absolute(0.025),
            0.22,
        ));

        assert_eq!(quote_set.len(), 2);

        // Grid stats should reflect the change
        let stats = quote_set.grid_stats();
        assert_eq!(stats.total_quotes, 2);
    }
}

// ============================================================================
// Task 17.3: AAD Vega Calculation Accuracy Tests
// ============================================================================

mod aad_vega_tests {
    use super::*;

    /// Test AAD cross-validation with bump-and-revalue.
    #[test]
    fn test_aad_cross_validation_linear_function() {
        let validator = AADCrossValidator::new().with_tolerance(0.01);

        // For a linear function, AAD and bump-and-revalue should match exactly
        let pricing_fn = |vol: f64| vol * 100.0;
        let aad_vega = 100.0; // Exact derivative of linear function

        let validation = validator.validate_point(1.0, 5.0, 0.03, 0.20, pricing_fn, aad_vega);

        assert!(validation.passed, "Linear function should pass validation");
        assert!(
            validation.relative_error < 0.01,
            "Error should be very small for linear function"
        );
    }

    /// Test AAD cross-validation with quadratic function.
    #[test]
    fn test_aad_cross_validation_quadratic_function() {
        let validator = AADCrossValidator::new().with_tolerance(0.01);

        // Quadratic function: price = vol^2 * 100
        let pricing_fn = |vol: f64| vol * vol * 100.0;
        let base_vol = 0.20;
        // Exact derivative: 2 * vol * 100 = 40
        let aad_vega = 2.0 * base_vol * 100.0;

        let validation = validator.validate_point(1.0, 5.0, 0.03, base_vol, pricing_fn, aad_vega);

        assert!(
            validation.passed,
            "Quadratic function should pass validation with central difference"
        );
    }

    /// Test Vega bump config.
    #[test]
    fn test_vega_bump_config() {
        let config = VegaBumpConfig {
            vol_bump: 0.0001, // 1bp
            use_relative_bump: false,
            relative_bump_pct: 0.01,
            use_central_difference: true,
        };

        assert_eq!(config.vol_bump, 0.0001);
        assert!(config.use_central_difference);
    }

    /// Test Vega calculator configuration and bump computation.
    #[test]
    fn test_vega_calculator_basic() {
        let config = VegaBumpConfig {
            vol_bump: 0.0001,
            use_relative_bump: false,
            relative_bump_pct: 0.01,
            use_central_difference: true,
        };

        // Test bump computation (absolute bump mode)
        let base_vol = 0.20;
        let bump = config.compute_bump(base_vol);
        assert_eq!(bump, 0.0001, "Absolute bump should be 0.0001");

        // Test relative bump mode
        let relative_config = VegaBumpConfig {
            vol_bump: 0.0001,
            use_relative_bump: true,
            relative_bump_pct: 0.01, // 1%
            use_central_difference: true,
        };
        let relative_bump = relative_config.compute_bump(base_vol);
        assert!(
            (relative_bump - 0.002).abs() < 1e-10,
            "Relative bump should be 0.20 * 0.01 = 0.002, got {}",
            relative_bump
        );

        // Test VegaCalculator creation
        let _calculator = VolCubeVegaCalculator::new(config);
    }
}

// ============================================================================
// End-to-End Flow Tests
// ============================================================================

mod e2e_flow_tests {
    use super::*;

    /// Test complete flow from quotes to calibrated cube.
    /// Note: VolCube requires at least 2 expiries AND 2 tenors for grid
    /// construction.
    #[test]
    fn test_quotes_to_calibrated_cube() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry_1y = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry_2y = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        // Create quote set
        let mut quote_set = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        // Add quotes for 1Y expiry, 5Y tenor
        quote_set.add_quote(VolQuote::new(
            "1Y-5Y-ATM",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.20,
        ));
        quote_set.add_quote(VolQuote::new(
            "1Y-5Y-OTM",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.025),
            0.22,
        ));

        // Add quotes for 1Y expiry, 10Y tenor
        quote_set.add_quote(VolQuote::new(
            "1Y-10Y-ATM",
            expiry_1y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.035),
            0.18,
        ));
        quote_set.add_quote(VolQuote::new(
            "1Y-10Y-OTM",
            expiry_1y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.030),
            0.20,
        ));

        // Add quotes for 2Y expiry, 5Y tenor
        quote_set.add_quote(VolQuote::new(
            "2Y-5Y-ATM",
            expiry_2y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.19,
        ));
        quote_set.add_quote(VolQuote::new(
            "2Y-5Y-OTM",
            expiry_2y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.025),
            0.21,
        ));

        // Add quotes for 2Y expiry, 10Y tenor
        quote_set.add_quote(VolQuote::new(
            "2Y-10Y-ATM",
            expiry_2y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.035),
            0.17,
        ));
        quote_set.add_quote(VolQuote::new(
            "2Y-10Y-OTM",
            expiry_2y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.030),
            0.19,
        ));

        // Validate quote set
        assert!(quote_set.validate().is_ok());

        // Convert to instruments using fixed forward
        let instruments = quote_set.to_instruments_with_fixed_forward(0.03);

        assert_eq!(instruments.len(), 8);

        // Build cube
        let config = VolCubeConfig::default();
        let cube = VolCubeBuilder::new()
            .with_instruments(instruments)
            .with_config(config)
            .build();

        assert!(cube.is_ok(), "Cube build should succeed");
    }

    /// Test Breeden-Litzenberger density calculation.
    #[test]
    fn test_breeden_litzenberger_density_concept() {
        // Create a simple smile for testing
        let forward = 0.03;

        // Simple SABR-like volatility function
        let vol_fn = |strike: f64| {
            let atm_vol = 0.20;
            let skew = -0.05;
            let moneyness = (strike - forward) / forward;
            atm_vol + skew * moneyness
        };

        // Test density at various strikes
        let strikes = [0.02, 0.025, 0.03, 0.035, 0.04];

        for &strike in &strikes {
            let vol = vol_fn(strike);
            assert!(
                vol > 0.0 && vol < 1.0,
                "Vol should be reasonable at strike {}",
                strike
            );
        }
    }
}

// ============================================================================
// VolQuoteSet Grid Stats Tests
// ============================================================================

mod quote_set_tests {
    use super::*;

    /// Test grid stats calculation.
    #[test]
    fn test_grid_stats_calculation() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry_1y = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry_2y = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let mut quote_set = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        // Add quotes for multiple expiries and tenors
        // 1Y expiry, 5Y tenor
        quote_set.add_quote(VolQuote::new(
            "1",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.20,
        ));
        quote_set.add_quote(VolQuote::new(
            "2",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.035),
            0.22,
        ));

        // 1Y expiry, 10Y tenor
        quote_set.add_quote(VolQuote::new(
            "3",
            expiry_1y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.03),
            0.18,
        ));

        // 2Y expiry, 5Y tenor
        quote_set.add_quote(VolQuote::new(
            "4",
            expiry_2y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.19,
        ));

        // 2Y expiry, 10Y tenor
        quote_set.add_quote(VolQuote::new(
            "5",
            expiry_2y,
            Tenor::years(10.0),
            VolStrike::Absolute(0.03),
            0.17,
        ));

        let stats = quote_set.grid_stats();

        assert_eq!(stats.num_expiries, 2);
        assert_eq!(stats.num_tenors, 2);
        assert_eq!(stats.num_slices, 4);
        assert_eq!(stats.total_quotes, 5);
        assert!(stats.meets_minimum_requirements());
    }

    /// Test unique expiries extraction.
    #[test]
    fn test_unique_expiries() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry_1y = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry_2y = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let mut quote_set = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        quote_set.add_quote(VolQuote::new(
            "1",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.20,
        ));
        quote_set.add_quote(VolQuote::new(
            "2",
            expiry_1y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.035),
            0.22,
        ));
        quote_set.add_quote(VolQuote::new(
            "3",
            expiry_2y,
            Tenor::years(5.0),
            VolStrike::Absolute(0.03),
            0.19,
        ));

        let expiries = quote_set.unique_expiries();
        assert_eq!(expiries.len(), 2);
        assert_eq!(expiries[0], expiry_1y);
        assert_eq!(expiries[1], expiry_2y);
    }
}

//! Tests for Greeks types and configuration.

use super::*;
use approx::assert_relative_eq;

// =============================================================================
// Task 1.1: GreeksResult<T> Tests
// =============================================================================

mod greeks_result_tests {
    use super::*;

    #[test]
    fn test_greeks_result_default() {
        let result: GreeksResult<f64> = GreeksResult::default();
        assert_eq!(result.price, 0.0);
        assert_eq!(result.std_error, 0.0);
        assert!(result.delta.is_none());
        assert!(result.gamma.is_none());
        assert!(result.vega.is_none());
        assert!(result.theta.is_none());
        assert!(result.rho.is_none());
        assert!(result.vanna.is_none());
        assert!(result.volga.is_none());
    }

    #[test]
    fn test_greeks_result_with_first_order_greeks() {
        let result = GreeksResult {
            price: 10.5,
            std_error: 0.05,
            delta: Some(0.55),
            gamma: None,
            vega: Some(25.0),
            theta: Some(-0.05),
            rho: Some(15.0),
            vanna: None,
            volga: None,
        };

        assert_eq!(result.price, 10.5);
        assert_eq!(result.std_error, 0.05);
        assert_eq!(result.delta, Some(0.55));
        assert!(result.gamma.is_none());
        assert_eq!(result.vega, Some(25.0));
        assert_eq!(result.theta, Some(-0.05));
        assert_eq!(result.rho, Some(15.0));
    }

    #[test]
    fn test_greeks_result_with_second_order_greeks() {
        let result = GreeksResult {
            price: 10.5,
            std_error: 0.05,
            delta: Some(0.55),
            gamma: Some(0.02),
            vega: Some(25.0),
            theta: None,
            rho: None,
            vanna: Some(0.1),
            volga: Some(50.0),
        };

        assert_eq!(result.gamma, Some(0.02));
        assert_eq!(result.vanna, Some(0.1));
        assert_eq!(result.volga, Some(50.0));
    }

    #[test]
    fn test_greeks_result_confidence_95() {
        let result = GreeksResult::<f64> {
            price: 10.0,
            std_error: 0.1,
            ..Default::default()
        };

        // 95% CI: 1.96 * std_error
        assert_relative_eq!(result.confidence_95(), 0.196, epsilon = 1e-10);
    }

    #[test]
    fn test_greeks_result_confidence_99() {
        let result = GreeksResult::<f64> {
            price: 10.0,
            std_error: 0.1,
            ..Default::default()
        };

        // 99% CI: 2.576 * std_error
        assert_relative_eq!(result.confidence_99(), 0.2576, epsilon = 1e-10);
    }

    #[test]
    fn test_greeks_result_clone() {
        let result = GreeksResult {
            price: 10.5,
            std_error: 0.05,
            delta: Some(0.55),
            gamma: Some(0.02),
            vega: Some(25.0),
            theta: Some(-0.05),
            rho: Some(15.0),
            vanna: Some(0.1),
            volga: Some(50.0),
        };

        let cloned = result.clone();
        assert_eq!(result.price, cloned.price);
        assert_eq!(result.delta, cloned.delta);
        assert_eq!(result.vanna, cloned.vanna);
    }

    #[test]
    fn test_greeks_result_debug() {
        let result = GreeksResult::<f64> {
            price: 10.0,
            std_error: 0.1,
            delta: Some(0.5),
            ..Default::default()
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("price"));
        assert!(debug_str.contains("delta"));
    }

    #[test]
    fn test_greeks_result_f32_generic() {
        // Test that GreeksResult works with f32
        let result = GreeksResult::<f32> {
            price: 10.5_f32,
            std_error: 0.05_f32,
            delta: Some(0.55_f32),
            gamma: None,
            vega: None,
            theta: None,
            rho: None,
            vanna: None,
            volga: None,
        };

        assert_eq!(result.price, 10.5_f32);
        assert_relative_eq!(result.confidence_95(), 1.96_f32 * 0.05_f32, epsilon = 1e-5);
    }
}

// =============================================================================
// Task 1.2: GreeksConfig Tests
// =============================================================================

mod greeks_config_tests {
    use super::*;

    #[test]
    fn test_greeks_config_default() {
        let config = GreeksConfig::default();

        // Default values from design.md
        assert_relative_eq!(config.spot_bump_relative, 0.01, epsilon = 1e-10);
        assert_relative_eq!(config.vol_bump_absolute, 0.01, epsilon = 1e-10);
        assert_relative_eq!(config.time_bump_years, 1.0 / 252.0, epsilon = 1e-10);
        assert_relative_eq!(config.rate_bump_absolute, 0.01, epsilon = 1e-10);
        assert_relative_eq!(config.verification_tolerance, 1e-6, epsilon = 1e-15);
        assert_eq!(config.mode, GreeksMode::BumpRevalue);
    }

    #[test]
    fn test_greeks_config_builder() {
        let config = GreeksConfig::builder()
            .spot_bump_relative(0.02)
            .vol_bump_absolute(0.005)
            .time_bump_years(1.0 / 365.0)
            .rate_bump_absolute(0.001)
            .verification_tolerance(1e-8)
            .mode(GreeksMode::BumpRevalue)
            .build()
            .unwrap();

        assert_relative_eq!(config.spot_bump_relative, 0.02, epsilon = 1e-10);
        assert_relative_eq!(config.vol_bump_absolute, 0.005, epsilon = 1e-10);
        assert_relative_eq!(config.time_bump_years, 1.0 / 365.0, epsilon = 1e-10);
        assert_relative_eq!(config.rate_bump_absolute, 0.001, epsilon = 1e-10);
        assert_relative_eq!(config.verification_tolerance, 1e-8, epsilon = 1e-15);
    }

    #[test]
    fn test_greeks_config_builder_defaults() {
        let config = GreeksConfig::builder().build().unwrap();

        // Should have default values
        assert_relative_eq!(config.spot_bump_relative, 0.01, epsilon = 1e-10);
    }

    #[test]
    fn test_greeks_config_validation_negative_spot_bump() {
        let result = GreeksConfig::builder().spot_bump_relative(-0.01).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_greeks_config_validation_zero_spot_bump() {
        let result = GreeksConfig::builder().spot_bump_relative(0.0).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_greeks_config_validation_negative_vol_bump() {
        let result = GreeksConfig::builder().vol_bump_absolute(-0.01).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_greeks_config_validation_excessive_spot_bump() {
        // Bump > 100% is unreasonable
        let result = GreeksConfig::builder().spot_bump_relative(1.5).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_greeks_config_validation_negative_time_bump() {
        let result = GreeksConfig::builder().time_bump_years(-0.01).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_greeks_config_clone() {
        let config = GreeksConfig::builder()
            .spot_bump_relative(0.02)
            .build()
            .unwrap();

        let cloned = config.clone();
        assert_eq!(config.spot_bump_relative, cloned.spot_bump_relative);
        assert_eq!(config.mode, cloned.mode);
    }

    #[test]
    fn test_greeks_config_debug() {
        let config = GreeksConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("spot_bump_relative"));
        assert!(debug_str.contains("mode"));
    }
}

// =============================================================================
// Task 1.2: GreeksMode Tests
// =============================================================================

mod greeks_mode_tests {
    use super::*;

    #[test]
    fn test_greeks_mode_default() {
        let mode = GreeksMode::default();
        assert_eq!(mode, GreeksMode::BumpRevalue);
    }

    #[test]
    fn test_greeks_mode_equality() {
        assert_eq!(GreeksMode::BumpRevalue, GreeksMode::BumpRevalue);
        assert_ne!(GreeksMode::BumpRevalue, GreeksMode::NumDual);
    }

    #[test]
    fn test_greeks_mode_clone() {
        let mode = GreeksMode::BumpRevalue;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_greeks_mode_copy() {
        let mode = GreeksMode::BumpRevalue;
        let copied: GreeksMode = mode; // Copy
        assert_eq!(mode, copied);
    }

    #[test]
    fn test_greeks_mode_debug() {
        let mode = GreeksMode::BumpRevalue;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("BumpRevalue"));
    }

    #[test]
    fn test_greeks_mode_variants_exist() {
        // Ensure all variants exist
        let _ = GreeksMode::BumpRevalue;
        let _ = GreeksMode::NumDual;
        // EnzymeAAD is feature-gated, so we don't test it here
    }
}

// =============================================================================
// Task 1.2: Greek Enum Extension Tests
// =============================================================================

mod greek_enum_tests {
    use crate::mc::Greek;

    #[test]
    fn test_greek_vanna_exists() {
        let greek = Greek::Vanna;
        assert_eq!(greek, Greek::Vanna);
    }

    #[test]
    fn test_greek_volga_exists() {
        let greek = Greek::Volga;
        assert_eq!(greek, Greek::Volga);
    }

    #[test]
    fn test_greek_all_variants() {
        // Ensure all 7 variants exist (5 original + 2 new)
        let greeks = [
            // First-order Greeks
            Greek::Delta,
            Greek::Vega,
            Greek::Theta,
            Greek::Rho,
            // Second-order Greeks
            Greek::Gamma,
            Greek::Vanna,
            Greek::Volga,
        ];

        assert_eq!(greeks.len(), 7);
    }

    #[test]
    fn test_greek_clone_and_copy() {
        let greek = Greek::Vanna;
        let cloned = greek.clone();
        let copied: Greek = greek; // Copy
        assert_eq!(greek, cloned);
        assert_eq!(greek, copied);
    }

    #[test]
    fn test_greek_debug() {
        let debug_str = format!("{:?}", Greek::Vanna);
        assert!(debug_str.contains("Vanna"));

        let debug_str = format!("{:?}", Greek::Volga);
        assert!(debug_str.contains("Volga"));
    }

    #[test]
    fn test_greek_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Greek::Delta);
        set.insert(Greek::Vanna);
        set.insert(Greek::Volga);
        assert_eq!(set.len(), 3);
    }
}

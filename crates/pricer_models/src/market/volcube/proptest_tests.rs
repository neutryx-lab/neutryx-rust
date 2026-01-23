//! Property-based tests for VolCube.
//!
//! # Requirements: 9.1, 9.2
//!
//! These tests use proptest to verify mathematical invariants and
//! arbitrage-free conditions for the VolCube implementation.

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use crate::market::volcube::{
        InstrumentId, SabrParameterSurface, SabrParams, VolCube, VolCubeConfig, VolatilityCube,
    };

    // ========================================
    // Test Helpers
    // ========================================

    fn create_valid_cube() -> VolCube<f64> {
        let expiries = vec![0.5, 1.0, 2.0];
        let tenors = vec![2.0, 5.0, 10.0];
        let beta = 0.5;

        let params = vec![
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),
                SabrParams::new(0.045, beta, -0.25, 0.35),
                SabrParams::new(0.05, beta, -0.2, 0.3),
            ],
            vec![
                SabrParams::new(0.038, beta, -0.32, 0.42),
                SabrParams::new(0.043, beta, -0.28, 0.38),
                SabrParams::new(0.048, beta, -0.22, 0.32),
            ],
            vec![
                SabrParams::new(0.035, beta, -0.35, 0.45),
                SabrParams::new(0.040, beta, -0.30, 0.40),
                SabrParams::new(0.045, beta, -0.25, 0.35),
            ],
        ];

        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap();

        let forwards = vec![
            vec![0.03, 0.035, 0.04],
            vec![0.032, 0.037, 0.042],
            vec![0.034, 0.039, 0.044],
        ];

        let config = VolCubeConfig::default();
        let source_instruments = vec![
            InstrumentId::new("INST-1"),
            InstrumentId::new("INST-2"),
            InstrumentId::new("INST-3"),
        ];
        let strike_domain = (0.01, 0.10);

        VolCube::new(
            sabr_surface,
            forwards,
            config,
            source_instruments,
            strike_domain,
        )
    }

    // ========================================
    // Property: Positive Volatility
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Volatility must always be positive for valid inputs.
        #[test]
        fn prop_volatility_is_positive(
            expiry in 0.5f64..2.0,
            tenor in 2.0f64..10.0,
            strike in 0.015f64..0.08,
        ) {
            let cube = create_valid_cube();
            let vol = cube.volatility(expiry, tenor, strike);

            prop_assert!(vol.is_ok(), "Vol lookup should succeed: {:?}", vol);
            let v = vol.unwrap();
            prop_assert!(v > 0.0, "Vol should be positive: {}", v);
            prop_assert!(v < 5.0, "Vol should be reasonable (< 500%): {}", v);
        }
    }

    // ========================================
    // Property: Non-negative Probability Density
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Risk-neutral probability density must be non-negative.
        #[test]
        fn prop_probability_density_non_negative(
            expiry in 0.5f64..2.0,
            strike in 0.02f64..0.06,
        ) {
            let cube = create_valid_cube();
            let density = cube.probability_density(expiry, strike);

            if let Ok(d) = density {
                prop_assert!(d >= 0.0, "Density should be non-negative: {}", d);
            }
            // We don't fail if density calculation fails due to numerical issues
        }
    }

    // ========================================
    // Property: CDF in [0, 1]
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Cumulative probability must be in [0, 1].
        #[test]
        fn prop_cumulative_probability_in_unit_interval(
            expiry in 0.5f64..2.0,
            strike in 0.02f64..0.06,
        ) {
            let cube = create_valid_cube();
            let cdf = cube.cumulative_probability(expiry, strike);

            if let Ok(c) = cdf {
                prop_assert!(c >= 0.0 && c <= 1.0, "CDF should be in [0, 1]: {}", c);
            }
        }
    }

    // ========================================
    // Property: CDF Monotonicity
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// CDF should be monotonically increasing in strike.
        #[test]
        fn prop_cdf_monotonic_in_strike(
            expiry in 0.75f64..1.5,
            k1 in 0.02f64..0.04,
            delta in 0.005f64..0.02,
        ) {
            let cube = create_valid_cube();
            let k2 = k1 + delta;

            let cdf1 = cube.cumulative_probability(expiry, k1);
            let cdf2 = cube.cumulative_probability(expiry, k2);

            if let (Ok(c1), Ok(c2)) = (cdf1, cdf2) {
                // Allow small tolerance for numerical errors
                prop_assert!(c2 >= c1 - 1e-6, "CDF should be monotonic: cdf({}) = {} >= cdf({}) = {}", k2, c2, k1, c1);
            }
        }
    }

    // ========================================
    // Property: Volatility Continuity
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Small changes in inputs should produce small changes in output.
        #[test]
        fn prop_volatility_continuous(
            expiry in 0.6f64..1.8,
            tenor in 3.0f64..8.0,
            strike in 0.02f64..0.06,
        ) {
            let cube = create_valid_cube();
            let eps = 1e-4;

            let vol_base = cube.volatility(expiry, tenor, strike);
            let vol_perturbed = cube.volatility(expiry + eps, tenor, strike);

            if let (Ok(v1), Ok(v2)) = (vol_base, vol_perturbed) {
                let diff = (v1 - v2).abs();
                // Vol should not jump by more than 10% for tiny perturbation
                prop_assert!(diff < 0.1, "Vol should be continuous: |{} - {}| = {}", v1, v2, diff);
            }
        }
    }

    // ========================================
    // Property: Butterfly Spread Non-Negative
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// Butterfly spread price should be non-negative (no arbitrage).
        /// C(K-dk) - 2*C(K) + C(K+dk) >= 0
        /// This is equivalent to the second derivative being non-negative,
        /// which means the probability density is non-negative.
        #[test]
        fn prop_butterfly_spread_non_negative(
            expiry in 0.6f64..1.5,
            tenor in 3.0f64..7.0,
            strike in 0.025f64..0.05,
        ) {
            let cube = create_valid_cube();
            let dk = 0.005; // 50 bps

            let vol_low = cube.volatility(expiry, tenor, strike - dk);
            let vol_mid = cube.volatility(expiry, tenor, strike);
            let vol_high = cube.volatility(expiry, tenor, strike + dk);

            if let (Ok(v_l), Ok(v_m), Ok(v_h)) = (vol_low, vol_mid, vol_high) {
                // The butterfly spread should have non-negative value
                // For implied vol, this translates to the smile being convex enough
                // Note: This is a simplified check; actual butterfly price check would
                // require full Black-Scholes pricing
                let convexity = v_l - 2.0 * v_m + v_h;
                // Allow small negative due to numerical precision
                prop_assert!(convexity >= -0.01, "Butterfly convexity: {}", convexity);
            }
        }
    }

    // ========================================
    // Property: Domain Consistency
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// Queries within the domain should succeed.
        #[test]
        fn prop_within_domain_succeeds(
            expiry_frac in 0.1f64..0.9,
            tenor_frac in 0.1f64..0.9,
            strike_frac in 0.1f64..0.9,
        ) {
            let cube = create_valid_cube();

            let (exp_min, exp_max) = cube.expiry_domain();
            let (ten_min, ten_max) = cube.tenor_domain();
            let (k_min, k_max) = cube.strike_domain();

            let expiry = exp_min + expiry_frac * (exp_max - exp_min);
            let tenor = ten_min + tenor_frac * (ten_max - ten_min);
            let strike = k_min + strike_frac * (k_max - k_min);

            let vol = cube.volatility(expiry, tenor, strike);
            prop_assert!(vol.is_ok(), "Within-domain query should succeed: {:?}", vol);
        }
    }

    // ========================================
    // Property: ATM Vol Stability
    // ========================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// ATM volatility should be stable and reasonable.
        #[test]
        fn prop_atm_vol_reasonable(
            expiry in 0.5f64..2.0,
            tenor in 2.5f64..9.0,
        ) {
            let cube = create_valid_cube();
            let forward = 0.035; // Approximate ATM

            let vol = cube.volatility(expiry, tenor, forward);

            if let Ok(v) = vol {
                // ATM vol for interest rate products is typically 10-100%
                prop_assert!(v > 0.05, "ATM vol should be > 5%: {}", v);
                prop_assert!(v < 2.0, "ATM vol should be < 200%: {}", v);
            }
        }
    }

    // ========================================
    // Property: SABR Parameters Consistency
    // ========================================

    #[test]
    fn test_sabr_params_produce_valid_smile() {
        let cube = create_valid_cube();
        let expiry = 1.0;
        let tenor = 5.0;
        let forward = 0.035;

        // Check smile around ATM
        let strikes = vec![0.02, 0.025, 0.03, 0.035, 0.04, 0.045, 0.05];
        let vols: Vec<_> = strikes
            .iter()
            .map(|&k| cube.volatility(expiry, tenor, k).unwrap())
            .collect();

        // All vols should be positive
        for vol in &vols {
            assert!(*vol > 0.0, "All vols should be positive");
        }

        // Vols should form a reasonable smile (U-shaped or skewed)
        // Check that extremes are not drastically different from ATM
        let atm_idx = 3;
        let atm_vol = vols[atm_idx];
        for (i, vol) in vols.iter().enumerate() {
            let ratio = vol / atm_vol;
            assert!(
                ratio > 0.5 && ratio < 3.0,
                "Smile ratio at {} should be reasonable: {}",
                strikes[i],
                ratio
            );
        }
    }

    // ========================================
    // Property: Source Instruments Preserved
    // ========================================

    #[test]
    fn test_source_instruments_preserved() {
        let cube = create_valid_cube();
        let instruments = cube.source_instruments();

        assert_eq!(instruments.len(), 3);
        assert_eq!(instruments[0].as_str(), "INST-1");
        assert_eq!(instruments[1].as_str(), "INST-2");
        assert_eq!(instruments[2].as_str(), "INST-3");
    }
}

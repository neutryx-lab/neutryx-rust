//! Integration tests for pricer_pricing module.
//!
//! These tests verify the integration between different pricing methods
//! and ensure consistency across the pricing infrastructure.

use infra_config::PricingMethod;
use pricer_pricing::{
    dispatcher::{DispatcherConfig, PricingMethodDispatcher},
    tree::{BinomialTree, TreeConfig, TreeMethod, TreeType},
};

// =============================================================================
// Task 11.1: Tree Method Integration Tests
// =============================================================================

/// Test that Tree method produces consistent results with different step
/// counts.
#[test]
fn test_tree_convergence_with_increasing_steps() {
    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;

    let mut prev_price: Option<f64> = None;
    let step_counts = [50, 100, 200, 500];

    for &steps in &step_counts {
        let tree = BinomialTree::new(spot, strike, expiry, rate, volatility, steps, true, false)
            .expect("Failed to create tree");
        let price = tree.price();

        if let Some(prev) = prev_price {
            // Price should converge (difference should decrease with more steps)
            assert!(
                (price - prev).abs() < 1.0,
                "Price should converge: {} vs {} with {} steps",
                price,
                prev,
                steps
            );
        }
        prev_price = Some(price);
    }
}

/// Test Tree method with TreeConfig builder pattern.
#[test]
fn test_tree_method_with_config_builder() {
    let config = TreeConfig::builder()
        .num_steps(200)
        .tree_type(TreeType::Binomial)
        .compute_greeks(true)
        .build()
        .expect("Failed to build config");

    let method = TreeMethod::new(config);
    let result = method
        .price(100.0, 100.0, 1.0, 0.05, 0.2, true, false)
        .expect("Failed to price");

    assert!(result.pv > 0.0);
    assert!(result.greeks.is_some());
    let greeks = result.greeks.unwrap();
    assert!(greeks.delta.is_some());
    assert!(greeks.gamma.is_some());
}

// =============================================================================
// Task 11.2: Dispatcher Integration Tests
// =============================================================================

/// Test that dispatcher routes correctly to all pricing methods.
#[test]
fn test_dispatcher_routes_all_methods() {
    let dispatcher = PricingMethodDispatcher::new();

    // Test Analytical
    let analytical = dispatcher
        .price_vanilla(
            PricingMethod::Analytical,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            None,
            None,
        )
        .expect("Analytical pricing failed");
    assert_eq!(analytical.method, PricingMethod::Analytical);
    assert!(analytical.pv > 0.0);

    // Test Tree
    let tree = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            Some(100),
            None,
        )
        .expect("Tree pricing failed");
    assert_eq!(tree.method, PricingMethod::Tree);
    assert!(tree.pv > 0.0);

    // Test Monte Carlo
    let mc = dispatcher
        .price_vanilla(
            PricingMethod::MonteCarlo,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            None,
            Some(5000),
        )
        .expect("MC pricing failed");
    assert_eq!(mc.method, PricingMethod::MonteCarlo);
    assert!(mc.pv > 0.0);
}

/// Test that all methods produce similar prices for European options.
#[test]
fn test_dispatcher_method_consistency() {
    let dispatcher = PricingMethodDispatcher::new();

    let spot = 100.0;
    let strike = 100.0;
    let expiry = 1.0;
    let rate = 0.05;
    let volatility = 0.2;

    // Get prices from all methods
    let analytical = dispatcher
        .price_vanilla(
            PricingMethod::Analytical,
            spot,
            strike,
            expiry,
            rate,
            volatility,
            true,
            false,
            None,
            None,
        )
        .unwrap();

    let tree = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            spot,
            strike,
            expiry,
            rate,
            volatility,
            true,
            false,
            Some(500),
            None,
        )
        .unwrap();

    let mc = dispatcher
        .price_vanilla(
            PricingMethod::MonteCarlo,
            spot,
            strike,
            expiry,
            rate,
            volatility,
            true,
            false,
            None,
            Some(50_000),
        )
        .unwrap();

    // All methods should produce similar prices (within 0.5 for ATM 1Y option)
    assert!(
        (analytical.pv - tree.pv).abs() < 0.1,
        "Analytical {} vs Tree {}: diff too large",
        analytical.pv,
        tree.pv
    );

    // MC has more variance, allow larger tolerance
    assert!(
        (analytical.pv - mc.pv).abs() < 0.5,
        "Analytical {} vs MC {}: diff too large",
        analytical.pv,
        mc.pv
    );
}

/// Test dispatcher with custom configuration.
#[test]
fn test_dispatcher_custom_config() {
    let config = DispatcherConfig {
        default_tree_steps: 500,
        default_mc_paths: 20_000,
        default_mc_steps: 100,
        compute_greeks: true,
    };

    let dispatcher = PricingMethodDispatcher::with_config(config);

    let result = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            None,
            None,
        )
        .unwrap();

    // Should use 500 steps from config
    assert_eq!(result.num_steps(), Some(500));
}

// =============================================================================
// Task 11.3: American vs European Verification Tests
// =============================================================================

/// Test that American put is always >= European put.
#[test]
fn test_american_put_geq_european() {
    let dispatcher = PricingMethodDispatcher::new();

    let test_cases = [
        (100.0, 100.0, 0.05, 0.2, 1.0), // ATM, 1Y
        (100.0, 110.0, 0.05, 0.2, 1.0), // ITM put, 1Y
        (100.0, 90.0, 0.05, 0.2, 1.0),  // OTM put, 1Y
        (100.0, 100.0, 0.05, 0.3, 0.5), // High vol, 6M
        (100.0, 100.0, 0.10, 0.2, 2.0), // High rate, 2Y
    ];

    for (spot, strike, rate, vol, expiry) in test_cases {
        let european = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                spot,
                strike,
                expiry,
                rate,
                vol,
                false,
                false,
                Some(300),
                None,
            )
            .unwrap();

        let american = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                spot,
                strike,
                expiry,
                rate,
                vol,
                false,
                true,
                Some(300),
                None,
            )
            .unwrap();

        assert!(
            american.pv >= european.pv - 1e-6,
            "American put {} should be >= European put {} for S={}, K={}, r={}, vol={}, T={}",
            american.pv,
            european.pv,
            spot,
            strike,
            rate,
            vol,
            expiry
        );
    }
}

/// Test that American call equals European call (no dividends).
#[test]
fn test_american_call_equals_european_no_dividend() {
    let dispatcher = PricingMethodDispatcher::new();

    let test_cases = [
        (100.0, 100.0, 0.05, 0.2, 1.0),
        (100.0, 90.0, 0.05, 0.2, 1.0),
        (100.0, 110.0, 0.05, 0.2, 1.0),
    ];

    for (spot, strike, rate, vol, expiry) in test_cases {
        let european = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                spot,
                strike,
                expiry,
                rate,
                vol,
                true,
                false,
                Some(500),
                None,
            )
            .unwrap();

        let american = dispatcher
            .price_vanilla(
                PricingMethod::Tree,
                spot,
                strike,
                expiry,
                rate,
                vol,
                true,
                true,
                Some(500),
                None,
            )
            .unwrap();

        // Should be equal within numerical tolerance
        assert!(
            (american.pv - european.pv).abs() < 1e-4,
            "American call {} should equal European call {} (no dividends)",
            american.pv,
            european.pv
        );
    }
}

/// Test put-call parity for European options.
#[test]
fn test_put_call_parity() {
    let dispatcher = PricingMethodDispatcher::new();

    let spot = 100.0;
    let strike = 100.0;
    let rate = 0.05;
    let vol = 0.2;
    let expiry = 1.0;

    let call = dispatcher
        .price_vanilla(
            PricingMethod::Analytical,
            spot,
            strike,
            expiry,
            rate,
            vol,
            true,
            false,
            None,
            None,
        )
        .unwrap();

    let put = dispatcher
        .price_vanilla(
            PricingMethod::Analytical,
            spot,
            strike,
            expiry,
            rate,
            vol,
            false,
            false,
            None,
            None,
        )
        .unwrap();

    // Put-call parity: C - P = S - K * exp(-r * T)
    let discount = (-rate * expiry).exp();
    let parity_rhs = spot - strike * discount;
    let parity_lhs = call.pv - put.pv;

    assert!(
        (parity_lhs - parity_rhs).abs() < 1e-6,
        "Put-call parity violated: C - P = {}, S - K*e^(-rT) = {}",
        parity_lhs,
        parity_rhs
    );
}

/// Test Greeks consistency between methods.
#[test]
fn test_greeks_consistency() {
    let dispatcher = PricingMethodDispatcher::new();

    let analytical = dispatcher
        .price_vanilla(
            PricingMethod::Analytical,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            None,
            None,
        )
        .unwrap();

    let tree = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            100.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            Some(500),
            None,
        )
        .unwrap();

    // Compare Delta
    let analytical_delta = analytical.delta().unwrap();
    let tree_delta = tree.delta().unwrap();

    assert!(
        (analytical_delta - tree_delta).abs() < 0.02,
        "Delta mismatch: Analytical {} vs Tree {}",
        analytical_delta,
        tree_delta
    );

    // Compare Gamma
    let analytical_gamma = analytical.gamma().unwrap();
    let tree_gamma = tree.gamma().unwrap();

    assert!(
        (analytical_gamma - tree_gamma).abs() < 0.005,
        "Gamma mismatch: Analytical {} vs Tree {}",
        analytical_gamma,
        tree_gamma
    );
}

/// Test extreme moneyness scenarios.
#[test]
fn test_extreme_moneyness() {
    let dispatcher = PricingMethodDispatcher::new();

    // Deep ITM call
    let deep_itm_call = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            150.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            Some(200),
            None,
        )
        .unwrap();

    // ITM call price should be approximately intrinsic + time value
    let intrinsic = 50.0;
    assert!(
        deep_itm_call.pv > intrinsic,
        "Deep ITM call {} should be > intrinsic {}",
        deep_itm_call.pv,
        intrinsic
    );

    // Deep OTM call
    let deep_otm_call = dispatcher
        .price_vanilla(
            PricingMethod::Tree,
            50.0,
            100.0,
            1.0,
            0.05,
            0.2,
            true,
            false,
            Some(200),
            None,
        )
        .unwrap();

    // OTM call price should be small but positive
    assert!(
        deep_otm_call.pv > 0.0 && deep_otm_call.pv < 5.0,
        "Deep OTM call {} should be small but positive",
        deep_otm_call.pv
    );
}

//! Integration tests for jump-aware curve functionality.
//!
//! This module tests the complete flow from JumpPillar definitions through
//! curve bootstrapping to jump-aware discount factor calculations.

use infra_master::market::definition::JumpPillar;
use infra_master::time::{Date, DayCounter};
use pricer_core::types::Limit;
use pricer_models::builder::CurveBootstrapper;
use pricer_models::market::curves::{BootstrappedCurve, MarketInstrument, YieldCurve};
use pricer_models::market::jumps::convert_jump_pillars;

// =============================================================================
// Integration Test Fixtures
// =============================================================================

fn test_valuation_date() -> Date {
    Date::from_ymd(2024, 1, 1).unwrap()
}

fn sample_instruments() -> Vec<MarketInstrument<f64>> {
    vec![
        MarketInstrument::ois(0.25, 0.025),
        MarketInstrument::ois(0.5, 0.028),
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
        MarketInstrument::ois(5.0, 0.04),
    ]
}

fn sample_jump_pillars() -> Vec<JumpPillar> {
    vec![
        // FOMC meeting in March
        JumpPillar::new(
            Date::from_ymd(2024, 3, 20).unwrap(),
            25.0,  // 25 bps expected hike
            0.8,   // 80% confidence
        ),
        // FOMC meeting in June
        JumpPillar::new(
            Date::from_ymd(2024, 6, 12).unwrap(),
            -25.0, // 25 bps expected cut
            0.6,   // 60% confidence
        ),
    ]
}

// =============================================================================
// Test: Backward Compatibility - No Jumps
// =============================================================================

#[test]
fn test_backward_compat_no_jumps_bootstrap_matches() {
    // Curves bootstrapped without jumps should produce identical results
    let instruments = sample_instruments();
    let bootstrapper = CurveBootstrapper::new();

    // Regular bootstrap
    let curve_regular = bootstrapper.bootstrap_to_curve(&instruments).unwrap();

    // Jump-aware bootstrap with no jumps
    let curve_with_jumps = bootstrapper
        .bootstrap_to_curve_with_jumps(&instruments, &[])
        .unwrap();

    // Discount factors should be identical
    for t in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0] {
        let df_reg = curve_regular.discount_factor(t).unwrap();
        let df_jump = curve_with_jumps.discount_factor(t).unwrap();
        assert!(
            (df_reg - df_jump).abs() < 1e-12,
            "DF mismatch at t={}: regular={}, with_jumps={}",
            t, df_reg, df_jump
        );
    }
}

// =============================================================================
// Test: End-to-End Flow
// =============================================================================

#[test]
fn test_end_to_end_jump_pillar_to_curve() {
    // Test complete flow: JumpPillar -> convert -> BootstrappedCurve
    let valuation = test_valuation_date();
    let instruments = sample_instruments();
    let jump_pillars = sample_jump_pillars();

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &jump_pillars,
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Verify jumps are attached
    assert!(curve.has_jumps());
    assert_eq!(curve.jumps().len(), 2);

    // Verify discount factor calculation works
    let df_1y = curve.discount_factor(1.0).unwrap();
    assert!(df_1y > 0.0 && df_1y < 1.0);
}

// =============================================================================
// Test: Left/Right Limit Behavior
// =============================================================================

#[test]
fn test_left_right_limit_at_jump() {
    let valuation = test_valuation_date();
    let instruments = sample_instruments();

    // Single jump at ~0.22 years (March 20, 2024)
    let jump = JumpPillar::new(
        Date::from_ymd(2024, 3, 20).unwrap(),
        25.0,
        1.0, // 100% confidence for clearer testing
    );

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump.clone()],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Get jump time
    let jump_time = DayCounter::Actual365Fixed.year_fraction(valuation, jump.jump_date());

    // At jump point
    let df_left = curve.discount_factor_with_limit(jump_time, Limit::Left).unwrap();
    let df_right = curve.discount_factor_with_limit(jump_time, Limit::Right).unwrap();
    let df_cont = curve.discount_factor_with_limit(jump_time, Limit::Continuous).unwrap();

    // Right and Continuous should be equal
    assert!(
        (df_right - df_cont).abs() < 1e-12,
        "Right and Continuous limits should be equal at jump"
    );

    // Left should be greater (no jump offset applied yet)
    assert!(
        df_left > df_right,
        "Left limit should be greater than right at positive jump"
    );

    // The ratio should reflect the jump offset
    // For 25 bps, offset = -0.0025 in log space
    let expected_ratio = (-0.0025_f64).exp();
    let actual_ratio = df_right / df_left;
    assert!(
        (actual_ratio - expected_ratio).abs() < 1e-8,
        "Jump ratio mismatch: expected {}, got {}",
        expected_ratio, actual_ratio
    );
}

#[test]
fn test_limits_between_jumps() {
    let valuation = test_valuation_date();
    let instruments = sample_instruments();
    let jump_pillars = sample_jump_pillars();

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &jump_pillars,
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Time between jumps (May 2024 - between March and June jumps)
    let t_between = 0.4;

    let df_left = curve.discount_factor_with_limit(t_between, Limit::Left).unwrap();
    let df_right = curve.discount_factor_with_limit(t_between, Limit::Right).unwrap();

    // Between jumps, left and right should be equal (no jump at this point)
    assert!(
        (df_left - df_right).abs() < 1e-12,
        "Left and right should be equal between jumps"
    );
}

// =============================================================================
// Test: Forward Rate Decomposition
// =============================================================================

#[test]
fn test_forward_rate_decomposition_spanning_jump() {
    let valuation = test_valuation_date();
    let instruments = sample_instruments();

    // Single jump for simpler testing
    let jump = JumpPillar::new(
        Date::from_ymd(2024, 3, 20).unwrap(),
        25.0,
        1.0,
    );

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Forward rate spanning the jump (0.1 to 0.5)
    let decomp = curve.decompose_forward_rate(0.1, 0.5).unwrap();

    // Total should equal continuous + jump
    assert!(
        (decomp.total - (decomp.continuous + decomp.jump)).abs() < 1e-10,
        "Total should equal continuous + jump"
    );

    // Jump component should be non-zero
    assert!(
        decomp.jump.abs() > 1e-6,
        "Jump component should be non-zero when spanning a jump"
    );
}

#[test]
fn test_forward_rate_decomposition_no_jump_in_range() {
    let valuation = test_valuation_date();
    let instruments = sample_instruments();

    // Jump at March 2024
    let jump = JumpPillar::new(
        Date::from_ymd(2024, 3, 20).unwrap(),
        25.0,
        1.0,
    );

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Forward rate not spanning the jump (0.5 to 1.0, jump is ~0.22)
    let decomp = curve.decompose_forward_rate(0.5, 1.0).unwrap();

    // Jump component should be ~0
    assert!(
        decomp.jump.abs() < 1e-10,
        "Jump component should be zero when not spanning a jump"
    );

    // Total should equal continuous
    assert!(
        (decomp.total - decomp.continuous).abs() < 1e-10,
        "Total should equal continuous when no jump in range"
    );
}

// =============================================================================
// Test: Performance (Binary Search O(log n))
// =============================================================================

#[test]
fn test_jump_lookup_performance() {
    use std::time::Instant;

    // Create many jumps
    let mut jumps: Vec<(f64, f64)> = Vec::new();
    let mut cumulative = 0.0;
    for i in 1..=100 {
        let t = i as f64 * 0.01; // 0.01, 0.02, ..., 1.0
        cumulative += 0.0001; // small increment
        jumps.push((t, cumulative));
    }

    let pillars = vec![0.5_f64, 1.0, 2.0];
    let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

    let curve = BootstrappedCurve::new(
        pillars,
        dfs,
        pricer_models::market::curves::BootstrapInterpolation::LogLinear,
        true,
    )
    .unwrap()
    .with_jumps(jumps);

    // Warm up
    for _ in 0..100 {
        let _ = curve.discount_factor(0.55);
    }

    // Timed run
    let start = Instant::now();
    let iterations = 10000;
    for _ in 0..iterations {
        let _ = curve.discount_factor_with_limit(0.55, Limit::Continuous);
    }
    let elapsed = start.elapsed();

    // Should complete in reasonable time (< 100ms for 10k iterations)
    assert!(
        elapsed.as_millis() < 100,
        "Performance issue: {} ms for {} iterations",
        elapsed.as_millis(),
        iterations
    );
}

// =============================================================================
// Test: Edge Cases
// =============================================================================

#[test]
fn test_jump_at_curve_start() {
    let valuation = test_valuation_date();
    let instruments = vec![
        MarketInstrument::ois(0.5, 0.028),
        MarketInstrument::ois(1.0, 0.03),
    ];

    // Jump very close to t=0
    let jump = JumpPillar::new(
        Date::from_ymd(2024, 1, 15).unwrap(), // 2 weeks from valuation
        10.0,
        1.0,
    );

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Should still work
    assert!(curve.has_jumps());
    let df = curve.discount_factor(0.5).unwrap();
    assert!(df > 0.0 && df < 1.0);
}

#[test]
fn test_jump_beyond_curve_end() {
    let valuation = test_valuation_date();
    let instruments = vec![
        MarketInstrument::ois(1.0, 0.03),
        MarketInstrument::ois(2.0, 0.035),
    ];

    // Jump beyond the last pillar
    let jump = JumpPillar::new(
        Date::from_ymd(2027, 6, 12).unwrap(), // 3.5 years out
        25.0,
        0.5,
    );

    let bootstrapper = CurveBootstrapper::new();
    let curve = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    assert!(curve.has_jumps());

    // Discount factor at 1.5Y should work and not include the jump
    let df = curve.discount_factor(1.5).unwrap();
    assert!(df > 0.0 && df < 1.0);
}

#[test]
fn test_zero_confidence_jump_ignored() {
    let valuation = test_valuation_date();
    let instruments = sample_instruments();

    // Jump with 0 confidence should have no effect
    let jump = JumpPillar::new(
        Date::from_ymd(2024, 3, 20).unwrap(),
        25.0,
        0.0, // Zero confidence
    );

    let bootstrapper = CurveBootstrapper::new();

    // With zero-confidence jump
    let curve_with_zero = bootstrapper
        .bootstrap_to_curve_with_jump_pillars(
            &instruments,
            &[jump],
            valuation,
            DayCounter::Actual365Fixed,
        )
        .unwrap();

    // Without any jump
    let curve_without = bootstrapper.bootstrap_to_curve(&instruments).unwrap();

    // Should produce same discount factors
    let df_with = curve_with_zero.discount_factor(0.5).unwrap();
    let df_without = curve_without.discount_factor(0.5).unwrap();

    assert!(
        (df_with - df_without).abs() < 1e-10,
        "Zero confidence jump should have no effect"
    );
}

// =============================================================================
// Test: Jump Conversion Utilities
// =============================================================================

#[test]
fn test_jump_entry_conversion() {
    let valuation = test_valuation_date();
    let pillars = sample_jump_pillars();

    let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

    assert_eq!(entries.len(), 2);

    // Should be sorted by time
    assert!(entries[0].time() < entries[1].time());

    // Cumulative offsets should accumulate (negative for rate hikes)
    // First (March, 25 bps hike): -25 * 0.8 / 10000 = -0.002
    // Second (June, -25 bps cut): -0.002 + -(-25) * 0.6 / 10000 = -0.002 + 0.0015 = -0.0005
    assert!((entries[0].cumulative_offset() - (-0.002)).abs() < 1e-10);
    assert!((entries[1].cumulative_offset() - (-0.0005)).abs() < 1e-10);
}

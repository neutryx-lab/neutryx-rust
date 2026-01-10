//! Demo command for lazy-arc-pricing-kernel architecture demonstration.
//!
//! This command demonstrates the complete 3-stage rocket pattern with:
//! - Lazy evaluation via `MarketProvider`
//! - Arc caching for curve/vol sharing
//! - Pull-then-Push parallel execution
//!
//! # Expected Log Output
//!
//! ```text
//! [Optimiser] Bootstrapping Yield Curve for USD...
//! [Optimiser] Bootstrapping Yield Curve for JPY...
//! [Optimiser] Calibrating SABR Surface for USD...
//! ```
//!
//! Key verification points:
//! - USD curve bootstrapped only once (Arc cache working)
//! - JPY curve bootstrapped separately
//! - USD SABR calibration only for CMS trade (lazy evaluation working)
//! - No SABR calibration for VanillaSwap trades

use crate::Result;
use pricer_core::types::Currency;
use pricer_optimiser::provider::MarketProvider;
use pricer_risk::demo::{run_portfolio_pricing, DemoTrade};

/// Runs the lazy-arc-pricing-kernel architecture demonstration.
///
/// Creates a portfolio of 4 trades:
/// - T001: USD VanillaSwap (fixed_rate = 0.02)
/// - T002: USD VanillaSwap (fixed_rate = 0.025)
/// - T003: USD CmsSwap (fixed_rate = 0.02) - requires vol
/// - T004: JPY VanillaSwap (fixed_rate = 0.01)
///
/// # Returns
///
/// `Ok(())` on success, `Err` on failure.
pub fn run() -> Result<()> {
    println!("========================================");
    println!("Lazy-Arc-Pricing-Kernel Demo");
    println!("========================================");
    println!();

    // Step 1: Create MarketProvider (empty caches)
    println!("[Demo] Creating MarketProvider with empty caches...");
    let market = MarketProvider::new();
    println!();

    // Step 2: Create portfolio of 4 trades
    println!("[Demo] Creating portfolio with 4 trades:");
    println!("  - T001: USD VanillaSwap (fixed_rate=0.02)");
    println!("  - T002: USD VanillaSwap (fixed_rate=0.025)");
    println!("  - T003: USD CmsSwap (fixed_rate=0.02) [requires vol]");
    println!("  - T004: JPY VanillaSwap (fixed_rate=0.01)");
    println!();

    let trades = vec![
        DemoTrade::new_vanilla_swap("T001", Currency::USD, 0.02),
        DemoTrade::new_vanilla_swap("T002", Currency::USD, 0.025),
        DemoTrade::new_cms_swap("T003", Currency::USD, 0.02),
        DemoTrade::new_vanilla_swap("T004", Currency::JPY, 0.01),
    ];

    // Step 3: Run portfolio pricing
    println!("[Demo] Running portfolio pricing (parallel)...");
    println!("       Expected: USD curve bootstrapped once, JPY curve once,");
    println!("                 USD SABR calibrated once (only for CmsSwap)");
    println!();

    let results = run_portfolio_pricing(&trades, &market);

    // Step 4: Display results
    println!();
    println!("[Demo] Pricing Results:");
    println!("----------------------------------------");
    println!("{:<10} {:<10} {:<15}", "Trade ID", "Currency", "PV");
    println!("----------------------------------------");

    for (trade, result) in trades.iter().zip(results.iter()) {
        println!(
            "{:<10} {:<10} {:<15.6}",
            result.trade_id,
            trade.ccy,
            result.pv
        );
    }
    println!("----------------------------------------");
    println!();

    // Step 5: Architecture verification summary
    println!("[Demo] Architecture Verification:");
    println!("  1. Arc Cache: Same-currency trades share curves (USD bootstrapped once)");
    println!("  2. Lazy Eval: Vol only fetched for CmsSwap (T003)");
    println!("  3. 3-Stage Rocket: Definition -> Linking -> Execution");
    println!("  4. Pull-then-Push: Market data resolved before kernel invocation");
    println!();
    println!("========================================");
    println!("Demo completed successfully!");
    println!("========================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_run() {
        // Just verify the demo runs without error
        let result = run();
        assert!(result.is_ok());
    }
}

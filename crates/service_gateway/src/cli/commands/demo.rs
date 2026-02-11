//! Demo command for lazy-arc-pricing-kernel architecture demonstration.
//!
//! This command demonstrates the complete 3-stage rocket pattern with:
//! - Lazy evaluation via `MarketProvider`
//! - Arc caching for curve/vol sharing
//! - Pull-then-Push parallel execution

/// Runs the lazy-arc-pricing-kernel architecture demonstration.
///
/// Creates a portfolio of 4 trades:
/// - T001: USD `VanillaSwap` (`fixed_rate` = 0.02)
/// - T002: USD `VanillaSwap` (`fixed_rate` = 0.025)
/// - T003: USD `CmsSwap` (`fixed_rate` = 0.02) - requires vol
/// - T004: JPY `VanillaSwap` (`fixed_rate` = 0.01)
pub fn run() {
    // TODO: pricer_risk temporarily disabled - needs refactoring
    println!("========================================");
    println!("Lazy-Arc-Pricing-Kernel Demo");
    println!("========================================");
    println!();
    println!("[Demo] Demo temporarily disabled - pricer_risk needs refactoring");
    println!("       to use infra_domain::trade types.");
    println!();
    println!("========================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_run() {
        // Just verify the demo runs without error
        run();
    }
}

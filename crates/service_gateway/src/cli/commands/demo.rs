//! Demo command for lazy-arc-pricing-kernel architecture demonstration.

/// Runs the lazy-arc-pricing-kernel architecture demonstration.
pub fn run() {
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
    fn test_demo_run() { run(); }
}

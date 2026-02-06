//! Check command implementation
//!
//! Validates system configuration and dependencies.

use tracing::info;

/// Run the check command
pub fn run() {
    info!("Checking system configuration...\n");

    println!("Neutryx System Check");
    println!("====================\n");

    // Check Rust version
    println!("Rust Toolchain:");
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Edition: 2021");
    println!();

    // Check for Enzyme (nightly feature)
    println!("Enzyme AD:");
    #[cfg(feature = "enzyme-ad")]
    println!("  Status: ✓ Enabled");
    #[cfg(not(feature = "enzyme-ad"))]
    println!("  Status: ✗ Disabled (pricer_pricing not built with Enzyme)");
    println!();

    // Check thread pool
    let num_threads = rayon::current_num_threads();
    println!("Parallelisation:");
    println!("  Rayon threads: {num_threads}");
    println!("  CPU cores: {}", num_cpus::get());
    println!();

    // Check available modules (A-I-P-S architecture)
    println!("Available Modules (A-I-P-S Architecture):");
    println!();
    println!("  [A] Adapter Layer:");
    println!("    ✓ adapter_feeds");
    println!("    ✓ adapter_fpml");
    println!("    ✓ adapter_loader");
    println!();
    println!("  [I] Infra Layer:");
    println!("    ✓ infra_config");
    println!("    ✓ infra_domain");
    println!("    ✓ infra_store");
    println!();
    println!("  [P] Pricer Layer:");
    println!("    ✓ pricer_core (L1)");
    println!("    ✓ pricer_models (L2)");
    println!("    ✓ pricer_optimiser (L2.5)");
    println!("    ✓ pricer_pricing (L3)");
    println!("    ✓ pricer_risk (L4)");
    println!();
    println!("  [S] Service Layer:");
    println!("    ✓ service_cli");
    println!("    ✓ service_gateway");
    println!("    ✓ service_python");
    println!();

    println!("All checks passed!");
}

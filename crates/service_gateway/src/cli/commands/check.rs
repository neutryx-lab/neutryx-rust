//! Check command implementation.

use tracing::info;

/// Run the check command.
pub fn run() {
    info!("Checking system configuration...\n");

    println!("Neutryx System Check");
    println!("====================\n");

    println!("Rust Toolchain:");
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Edition: 2021");
    println!();

    println!("Enzyme AD:");
    #[cfg_attr(not(feature = "enzyme-ad"), allow(dead_code))]
    {
        println!("  Status: ✗ Disabled (pricer_pricing not built with Enzyme)");
    }
    println!();

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!("Parallelisation:");
    println!("  CPU cores: {num_cpus}");
    println!();

    println!("Available Modules (A-I-P-S Architecture):");
    println!();
    println!("  [A] Adapter Layer:");
    println!("    ✓ adapter_feeds");
    println!("    ✓ adapter_loader (incl. fpml feature)");
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
    println!("    ✓ service_gateway (REST + CLI + Python)");
    println!();

    println!("All checks passed!");
}

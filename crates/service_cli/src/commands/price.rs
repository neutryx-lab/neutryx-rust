//! Price command implementation
//!
//! Prices a portfolio of trades using the pricer_pricing engine.

use tracing::info;

use crate::{CliError, Result};

/// Run the price command
pub fn run(portfolio: &str, date: Option<&str>, num_paths: usize, format: &str) -> Result<()> {
    info!("Starting pricing...");
    info!("  Portfolio: {}", portfolio);
    info!("  Date: {}", date.unwrap_or("today"));
    info!("  Monte Carlo paths: {}", num_paths);
    info!("  Output format: {}", format);

    // Validate portfolio file exists
    if !std::path::Path::new(portfolio).exists() {
        return Err(CliError::FileNotFound(portfolio.to_string()));
    }

    // Stub: portfolio loading, market data, and pricing integration pending.
    // Current implementation outputs format template only.

    match format {
        "json" => {
            info!("Outputting results as JSON...");
        }
        "csv" => {
            info!("Outputting results as CSV...");
        }
        "table" => {
            info!("Outputting results as table...");
            println!("\n┌────────────┬────────────┬────────────┐");
            println!("│ Trade ID   │ PV         │ Delta      │");
            println!("├────────────┼────────────┼────────────┤");
            println!("│ (no data)  │            │            │");
            println!("└────────────┴────────────┴────────────┘");
        }
        other => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown format: {}. Supported: json, csv, table",
                other
            )));
        }
    }

    info!("Pricing complete");
    Ok(())
}

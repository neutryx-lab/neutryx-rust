//! Price command implementation.

use tracing::info;

use crate::error::ServerError;

/// Run the price command.
pub fn run(
    portfolio: &str,
    date: Option<&str>,
    num_paths: usize,
    format: &str,
) -> Result<(), ServerError> {
    info!("Starting pricing...");
    info!("  Portfolio: {}", portfolio);
    info!("  Date: {}", date.unwrap_or("today"));
    info!("  Monte Carlo paths: {}", num_paths);
    info!("  Output format: {}", format);

    if !std::path::Path::new(portfolio).exists() {
        return Err(ServerError::NotFound(format!(
            "File not found: {portfolio}"
        )));
    }

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
            return Err(ServerError::InvalidRequest(format!(
                "Unknown format: {other}. Supported: json, csv, table"
            )));
        }
    }

    info!("Pricing complete");
    Ok(())
}

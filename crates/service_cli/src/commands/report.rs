//! Report command implementation
//!
//! Generates risk reports using the pricer_risk crate.

use tracing::info;

use crate::{CliError, Result};

/// Run the report command
pub fn run(report_type: &str, portfolio: &str, output_dir: &str) -> Result<()> {
    info!("Generating report...");
    info!("  Report type: {}", report_type);
    info!("  Portfolio: {}", portfolio);
    info!("  Output directory: {}", output_dir);

    // Validate portfolio file exists
    if !std::path::Path::new(portfolio).exists() {
        return Err(CliError::FileNotFound(portfolio.to_string()));
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    // Stub: portfolio loading and pricer_risk report generation pending.

    match report_type {
        "xva" => {
            info!("Generating XVA report...");
            // Use pricer_risk::xva
        }
        "exposure" => {
            info!("Generating exposure report...");
            // Use pricer_risk::exposure
        }
        "greeks" => {
            info!("Generating Greeks report...");
            // Use pricer_pricing::greeks
        }
        other => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown report type: {}. Supported: xva, exposure, greeks",
                other
            )));
        }
    }

    info!("Report generation complete");
    Ok(())
}

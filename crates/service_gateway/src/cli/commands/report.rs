//! Report command implementation.

use tracing::info;

use crate::error::ServerError;

/// Run the report command.
pub fn run(report_type: &str, portfolio: &str, output_dir: &str) -> Result<(), ServerError> {
    info!("Generating report...");
    info!("  Report type: {}", report_type);
    info!("  Portfolio: {}", portfolio);
    info!("  Output directory: {}", output_dir);

    if !std::path::Path::new(portfolio).exists() {
        return Err(ServerError::NotFound(format!(
            "File not found: {portfolio}"
        )));
    }

    std::fs::create_dir_all(output_dir)?;

    match report_type {
        "xva" => {
            info!("Generating XVA report...");
        }
        "exposure" => {
            info!("Generating exposure report...");
        }
        "greeks" => {
            info!("Generating Greeks report...");
        }
        other => {
            return Err(ServerError::InvalidRequest(format!(
                "Unknown report type: {other}. Supported: xva, exposure, greeks"
            )));
        }
    }

    info!("Report generation complete");
    Ok(())
}

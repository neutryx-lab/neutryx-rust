//! Calibrate command implementation
//!
//! Calibrates model parameters from market data using the pricer_optimiser crate.

use tracing::{info, warn};

use crate::{CliError, Result};

/// Run the calibrate command
pub fn run(market_data: &str, model_type: &str, output: Option<&str>) -> Result<()> {
    info!("Starting calibration...");
    info!("  Market data: {}", market_data);
    info!("  Model type: {}", model_type);

    // Validate market data file exists
    if !std::path::Path::new(market_data).exists() {
        return Err(CliError::FileNotFound(market_data.to_string()));
    }

    // Stub: market data loading and pricer_optimiser calibration pending.

    match model_type {
        "hull-white" => {
            info!("Calibrating Hull-White model...");
            // Use pricer_optimiser::calibration::hull_white
        }
        "cir" => {
            info!("Calibrating CIR model...");
            // Use pricer_optimiser::calibration::cir
        }
        other => {
            warn!("Unknown model type: {}", other);
            return Err(CliError::InvalidArgument(format!(
                "Unknown model type: {}. Supported: hull-white, cir",
                other
            )));
        }
    }

    if let Some(output_path) = output {
        info!("Writing calibrated parameters to: {}", output_path);
        // Stub: parameter serialisation pending.
    }

    info!("Calibration complete");
    Ok(())
}

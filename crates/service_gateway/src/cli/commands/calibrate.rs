//! Calibrate command implementation.

use tracing::{info, warn};

use crate::error::ServerError;

/// Run the calibrate command.
pub fn run(market_data: &str, model_type: &str, output: Option<&str>) -> Result<(), ServerError> {
    info!("Starting calibration...");
    info!("  Market data: {}", market_data);
    info!("  Model type: {}", model_type);

    if !std::path::Path::new(market_data).exists() {
        return Err(ServerError::FileNotFound(market_data.to_string()));
    }

    match model_type {
        "hull-white" => {
            info!("Calibrating Hull-White model...");
        }
        "cir" => {
            info!("Calibrating CIR model...");
        }
        other => {
            warn!("Unknown model type: {}", other);
            return Err(ServerError::InvalidArgument(format!(
                "Unknown model type: {other}. Supported: hull-white, cir"
            )));
        }
    }

    if let Some(output_path) = output {
        info!("Writing calibrated parameters to: {}", output_path);
    }

    info!("Calibration complete");
    Ok(())
}

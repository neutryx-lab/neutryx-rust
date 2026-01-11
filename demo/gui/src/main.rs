//! Demo TUI Entry Point

use anyhow::Result;
use demo_gui::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Note: Tracing is disabled for TUI to avoid terminal interference
    // For debugging, use RUST_LOG env var with a file logger

    // Create and run the TUI app
    let mut app = TuiApp::new()?;
    app.run().await?;

    Ok(())
}

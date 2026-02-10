//! CLI command-line interface
//!
//! Provides a `clap`-based CLI for operational tasks such as pricing,
//! calibration, reporting, and system checks.

pub mod commands;

use clap::{Parser, Subcommand};
use tracing::info;

use crate::error::ServerError;

/// Convenience alias for CLI results.
pub type Result<T> = std::result::Result<T, ServerError>;

/// Neutryx XVA Pricing Library CLI
#[derive(Parser)]
#[command(name = "neutryx")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true, default_value = "neutryx.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Calibrate model parameters from market data
    Calibrate {
        /// Path to market data file
        #[arg(short, long)]
        market_data: String,

        /// Model type to calibrate (e.g., hull-white, cir)
        #[arg(short = 't', long, default_value = "hull-white")]
        model_type: String,

        /// Output file for calibrated parameters
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Price a portfolio of trades
    Price {
        /// Path to portfolio file (CSV/JSON)
        #[arg(short, long)]
        portfolio: String,

        /// Pricing date (YYYY-MM-DD)
        #[arg(short, long)]
        date: Option<String>,

        /// Number of Monte Carlo paths
        #[arg(short, long, default_value = "10000")]
        num_paths: usize,

        /// Output format (json, csv, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Generate risk reports
    Report {
        /// Report type (xva, exposure, greeks)
        #[arg(short = 't', long, default_value = "xva")]
        report_type: String,

        /// Portfolio file
        #[arg(short, long)]
        portfolio: String,

        /// Output directory
        #[arg(short, long, default_value = "./reports")]
        output_dir: String,
    },

    /// Check system configuration and dependencies
    Check,

    /// Run lazy-arc-pricing-kernel architecture demonstration
    Demo,
}

/// Execute the parsed CLI command.
pub fn run(cli: Cli) -> Result<()> {
    if cli.verbose {
        info!("Verbose mode enabled");
    }

    match cli.command {
        Commands::Calibrate {
            market_data,
            model_type,
            output,
        } => commands::calibrate::run(&market_data, &model_type, output.as_deref()),
        Commands::Price {
            portfolio,
            date,
            num_paths,
            format,
        } => commands::price::run(&portfolio, date.as_deref(), num_paths, &format),
        Commands::Report {
            report_type,
            portfolio,
            output_dir,
        } => commands::report::run(&report_type, &portfolio, &output_dir),
        Commands::Check => {
            commands::check::run();
            Ok(())
        }
        Commands::Demo => {
            commands::demo::run();
            Ok(())
        }
    }
}

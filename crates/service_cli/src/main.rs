//! Neutryx CLI - Command Line Operations for XVA Pricing
//!
//! This is the operational entry point for the Neutryx XVA pricing library.
//!
//! # Commands
//!
//! - `neutryx calibrate` - Calibrate model parameters from market data
//! - `neutryx price --portfolio <file>` - Price a portfolio of trades
//! - `neutryx report` - Generate risk reports
//!
//! # Architecture
//!
//! As part of the **S**ervice layer in the A-I-P-S architecture, this crate
//! orchestrates all other layers to provide a unified command-line interface.

use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod config;
mod error;

pub use error::{CliError, Result};

/// Neutryx XVA Pricing Library CLI
#[derive(Parser)]
#[command(name = "neutryx")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true, default_value = "neutryx.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

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
        Commands::Check => commands::check::run(),
        Commands::Demo => commands::demo::run(),
    }
}

// Clippy configuration for demo_inputs
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::unused_async)]

//! # Upstream Systems
//!
//! Mock upstream systems that provide input data to the Neutryx Adapter layer.
//!
//! This crate simulates external market data providers, trade booking systems,
//! and file generators that would typically feed into the A-I-P-S architecture.
//!
//! ## Modules
//!
//! - [`market_data_provider`]: Simulates market data feeds (Reuters, Bloomberg
//!   style)
//! - [`trade_source`]: Simulates front office trade booking
//! - [`file_source`]: Generates CSV/Parquet files for batch processing

pub mod file_source;
pub mod market_data_provider;
pub mod trade_source;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        file_source::{BulkDataGenerator, CsvGenerator, FileGenerator, GenerationSummary},
        market_data_provider::{
            BloombergSim, MarketDataProvider, MeanReversionModel, PriceEvolutionModel,
            RandomWalkModel, ReutersSim, StreamingPriceGenerator, SyntheticGenerator,
        },
        trade_source::{FpmlGenerator, FrontOffice, TradeSource},
    };
}

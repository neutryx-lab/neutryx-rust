// Clippy configuration for demo_gui
// Demo code uses simpler patterns acceptable for non-production code
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::unused_async)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::semicolon_if_nothing_returned)]

//! # Demo GUI
//!
//! Web dashboard for the FrictionalBank demo.
//!
//! ## Web Mode
//! Uses axum for REST API and WebSocket support.
//!
//! ### Endpoints
//! - `GET /api/health` - Health check
//! - `GET /api/portfolio` - Portfolio data
//! - `GET /api/exposure` - Exposure metrics
//! - `GET /api/risk` - Risk metrics
//! - `GET /api/market/rates` - Market rates
//! - `WS /api/ws` - Real-time updates

pub mod web;

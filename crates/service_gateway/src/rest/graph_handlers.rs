//! Portfolio graph REST API handlers.
//!
//! Provides endpoints for Portfolio-level computation graph extraction
//! with shared node deduplication and subgraph filtering.
#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use adapter_loader::fpml::FpmlParser;
use axum::{
    extract::{Query, State},
    Json,
};
use infra_domain::trade::{Trade as FpmlTrade, TradeType};
use pricer_pricing::graph::{
    ComputationGraph, PortfolioComputationGraph, PortfolioGraphExtractable, PortfolioGraphExtractor,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::ServerError;

/// Query parameters for `/api/v1/portfolio/graph`
#[derive(Debug, Deserialize)]
pub struct GraphQueryParams {
    /// Comma-separated trade IDs for subgraph extraction
    /// Example: `?trade_ids=T001,T002,T003`
    pub trade_ids: Option<String>,
}

/// Portfolio computation graph response
#[derive(Serialize)]
pub struct PortfolioGraphResponse {
    /// Graph nodes
    pub nodes: Vec<GraphNodeDto>,
    /// Graph edges (as "links" for D3.js compatibility)
    #[serde(rename = "links")]
    pub edges: Vec<GraphEdgeDto>,
    /// Graph metadata
    pub metadata: PortfolioGraphMetadataDto,
}

/// Graph node DTO
#[derive(Serialize)]
pub struct GraphNodeDto {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub is_sensitivity_target: bool,
    pub group: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trade_ids: Vec<String>,
}

/// Graph edge DTO
#[derive(Serialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

/// Portfolio graph metadata DTO
#[derive(Serialize)]
pub struct PortfolioGraphMetadataDto {
    pub node_count: usize,
    pub edge_count: usize,
    pub depth: usize,
    pub generated_at: String,
    pub trade_count: usize,
    pub shared_node_count: usize,
    pub optimisation_ratio: f64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub large_graph_warning: bool,
}

/// Trade list query parameters
#[derive(Debug, Deserialize)]
pub struct TradeListQueryParams {
    /// Filter by instrument type (e.g., "vanilla", "forward")
    pub instrument_type: Option<String>,
    /// Filter by currency (e.g., "USD", "EUR")
    pub currency: Option<String>,
}

/// Trade summary for listing
#[derive(Serialize)]
pub struct TradeSummaryDto {
    pub id: String,
    pub instrument_type: String,
    pub currency: String,
    pub notional: f64,
    /// Maturity date (last cashflow date) as ISO 8601 string, e.g.,
    /// "2025-07-15"
    pub maturity: String,
    /// Counterparty name
    pub counterparty: String,
    /// Trading book
    pub book: String,
}

/// Trade list response
#[derive(Serialize)]
pub struct TradeListResponse {
    pub trades: Vec<TradeSummaryDto>,
    pub statistics: TradeStatisticsDto,
}

/// Trade statistics
#[derive(Serialize)]
pub struct TradeStatisticsDto {
    pub total_count: usize,
    pub by_instrument_type: HashMap<String, usize>,
    pub by_currency: HashMap<String, usize>,
    pub total_notional: f64,
}

/// Shared application state for graph handlers
pub struct GraphAppState {
    /// `FpML` trades loaded from files
    pub trades: Vec<FpmlTrade>,
    /// Graph cache with TTL
    pub graph_cache: RwLock<GraphCache>,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

/// Graph cache entry
struct CacheEntry {
    graph: PortfolioComputationGraph,
    created_at: Instant,
}

/// Simple graph cache with TTL
pub struct GraphCache {
    /// Full portfolio graph cache
    full_graph: Option<CacheEntry>,
    /// Subgraph cache by `trade_ids` key
    subgraphs: HashMap<String, CacheEntry>,
    /// TTL in seconds
    ttl_secs: u64,
}

impl GraphCache {
    /// Create a new cache with specified TTL
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            full_graph: None,
            subgraphs: HashMap::new(),
            ttl_secs,
        }
    }

    /// Get cached full graph if still valid
    fn get_full_graph(&self) -> Option<&PortfolioComputationGraph> {
        self.full_graph.as_ref().and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < self.ttl_secs {
                Some(&entry.graph)
            } else {
                None
            }
        })
    }

    /// Set full graph cache
    fn set_full_graph(&mut self, graph: PortfolioComputationGraph) {
        self.full_graph = Some(CacheEntry {
            graph,
            created_at: Instant::now(),
        });
    }

    /// Get cached subgraph if still valid
    fn get_subgraph(&self, key: &str) -> Option<&PortfolioComputationGraph> {
        self.subgraphs.get(key).and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < self.ttl_secs {
                Some(&entry.graph)
            } else {
                None
            }
        })
    }

    /// Set subgraph cache
    fn set_subgraph(&mut self, key: String, graph: PortfolioComputationGraph) {
        self.subgraphs.insert(
            key,
            CacheEntry {
                graph,
                created_at: Instant::now(),
            },
        );
    }

    /// Clean expired entries
    fn _cleanup_expired(&mut self) {
        if let Some(entry) = &self.full_graph {
            if entry.created_at.elapsed().as_secs() >= self.ttl_secs {
                self.full_graph = None;
            }
        }
        self.subgraphs
            .retain(|_, entry| entry.created_at.elapsed().as_secs() < self.ttl_secs);
    }
}

impl GraphAppState {
    /// Create a new app state by loading `FpML` trades from the demo directory.
    pub fn new_with_sample(_trade_count: usize, cache_ttl_secs: u64) -> Result<Self, ServerError> {
        let trades = load_fpml_trades()
            .map_err(|e| ServerError::Internal(format!("Failed to load FpML trades: {e}")))?;

        tracing::info!(
            "Loaded {} FpML trades from demo/data/trades/fpml/",
            trades.len()
        );

        Ok(Self {
            trades,
            graph_cache: RwLock::new(GraphCache::new(cache_ttl_secs)),
            cache_ttl_secs,
        })
    }

    /// Create with default settings (loads all `FpML` files, 5 second cache)
    pub fn default_sample() -> Result<Self, ServerError> { Self::new_with_sample(0, 5) }

    /// Returns the number of loaded trades.
    pub fn trade_count(&self) -> usize { self.trades.len() }
}

/// Load all `FpML` trades from the demo/data/trades/fpml/ directory.
fn load_fpml_trades() -> Result<Vec<FpmlTrade>, String> {
    let base_path = Path::new("demo/data/trades/fpml");

    if !base_path.exists() {
        return Err(format!("FpML directory not found: {}", base_path.display()));
    }

    let mut trades = Vec::new();

    // Directories to scan for FpML files
    let subdirs = ["rates", "fx", "equity", "credit", "commodity"];

    for subdir in &subdirs {
        let dir_path = base_path.join(subdir);
        if !dir_path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "xml") {
                    match load_fpml_file(&path) {
                        Ok(trade) => {
                            tracing::debug!("Loaded FpML trade: {} from {:?}", trade.id, path);
                            trades.push(trade);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse FpML file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    if trades.is_empty() {
        return Err("No FpML trades found".to_string());
    }

    Ok(trades)
}

/// Load a single `FpML` file.
fn load_fpml_file(path: &Path) -> Result<FpmlTrade, String> {
    let xml = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    FpmlParser::parse(&xml).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Extract portfolio computation graph with optional subgraph filtering.
pub async fn get_portfolio_graph(
    State(state): State<Arc<GraphAppState>>,
    Query(params): Query<GraphQueryParams>,
) -> Result<Json<PortfolioGraphResponse>, ServerError> {
    let timeout_ms = 500u64;
    let start_time = Instant::now();

    // Parse trade_ids if provided
    let trade_ids: Option<Vec<String>> = params
        .trade_ids
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

    // Check if we need a subgraph
    let cache_key = trade_ids.as_ref().map(|ids| {
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.join(",")
    });

    // Try to get from cache first
    {
        let cache = state.graph_cache.read().await;
        if let Some(key) = &cache_key {
            if let Some(cached) = cache.get_subgraph(key) {
                return Ok(Json(convert_to_response(cached)));
            }
        } else if let Some(cached) = cache.get_full_graph() {
            return Ok(Json(convert_to_response(cached)));
        }
    }

    // Extract graph with timeout protection
    let graph = tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        extract_fpml_portfolio_graph(&state.trades, trade_ids.as_deref())
    })
    .await
    .map_err(|_| ServerError::Timeout("Graph extraction timed out (>500ms)".to_string()))?
    .map_err(|e| match &e {
        pricer_pricing::graph::GraphError::TradeNotFound(id) => {
            ServerError::NotFound(format!("Trade not found: {id}"))
        }
        pricer_pricing::graph::GraphError::Timeout => {
            ServerError::Timeout("Graph extraction timed out".to_string())
        }
        pricer_pricing::graph::GraphError::ExtractionFailed(msg) => {
            ServerError::Internal(format!("Graph extraction failed: {msg}"))
        }
    })?;

    // Check for large graph warning
    let large_graph_warning = graph.metadata.node_count > 10_000;
    if large_graph_warning {
        tracing::warn!(
            "Large graph generated: {} nodes (>10,000). Consider using LOD mode.",
            graph.metadata.node_count
        );
    }

    // Cache the result
    {
        let mut cache = state.graph_cache.write().await;
        if let Some(key) = cache_key {
            cache.set_subgraph(key, graph.clone());
        } else {
            cache.set_full_graph(graph.clone());
        }
    }

    // Log extraction time
    let elapsed = start_time.elapsed();
    tracing::debug!(
        "Graph extraction completed in {:?} ({} nodes, {} edges)",
        elapsed,
        graph.metadata.node_count,
        graph.metadata.edge_count
    );

    Ok(Json(convert_to_response(&graph)))
}

/// List all trades in the portfolio with optional filtering.
pub async fn get_portfolio_trades(
    State(state): State<Arc<GraphAppState>>,
    Query(params): Query<TradeListQueryParams>,
) -> Result<Json<TradeListResponse>, ServerError> {
    let all_trades = &state.trades;

    let mut trades: Vec<TradeSummaryDto> = Vec::new();
    let mut by_instrument_type: HashMap<String, usize> = HashMap::new();
    let mut by_currency: HashMap<String, usize> = HashMap::new();
    let mut total_notional = 0.0;

    for trade in all_trades {
        let instrument_type = get_instrument_type_name(&trade.trade_type);
        let (currency, notional, maturity, counterparty, book) = get_trade_details(trade);

        // Apply filters
        if let Some(ref filter_type) = params.instrument_type {
            if instrument_type != *filter_type {
                // Still count for statistics
                *by_instrument_type
                    .entry(instrument_type.clone())
                    .or_insert(0) += 1;
                *by_currency.entry(currency.clone()).or_insert(0) += 1;
                total_notional += notional;
                continue;
            }
        }
        if let Some(ref filter_currency) = params.currency {
            if currency != *filter_currency {
                *by_instrument_type
                    .entry(instrument_type.clone())
                    .or_insert(0) += 1;
                *by_currency.entry(currency.clone()).or_insert(0) += 1;
                total_notional += notional;
                continue;
            }
        }

        // Update statistics
        *by_instrument_type
            .entry(instrument_type.clone())
            .or_insert(0) += 1;
        *by_currency.entry(currency.clone()).or_insert(0) += 1;
        total_notional += notional;

        trades.push(TradeSummaryDto {
            id: trade.id.to_string(),
            instrument_type,
            currency,
            notional,
            maturity,
            counterparty,
            book,
        });
    }

    Ok(Json(TradeListResponse {
        statistics: TradeStatisticsDto {
            total_count: all_trades.len(),
            by_instrument_type,
            by_currency,
            total_notional,
        },
        trades,
    }))
}

/// Get a human-readable instrument type name from `TradeType`.
fn get_instrument_type_name(trade_type: &TradeType) -> String {
    match trade_type {
        TradeType::Deposit => "Deposit".to_string(),
        TradeType::Fra => "FRA".to_string(),
        TradeType::Futures => "Futures".to_string(),
        TradeType::Swap => "IRS".to_string(),
        TradeType::Ois => "OIS".to_string(),
        TradeType::BasisSwap => "Basis Swap".to_string(),
        TradeType::CrossCurrencySwap => "XCCY Swap".to_string(),
        TradeType::Swaption { .. } => "Swaption".to_string(),
        TradeType::Bond { .. } => "Bond".to_string(),
        TradeType::CapFloor => "Cap/Floor".to_string(),
        TradeType::FxSpot => "FX Spot".to_string(),
        TradeType::FxForward => "FX Forward".to_string(),
        TradeType::FxSwap => "FX Swap".to_string(),
        TradeType::FxOption { .. } => "FX Option".to_string(),
        TradeType::FxBarrierOption { .. } => "FX Barrier".to_string(),
        TradeType::EquityForward { .. } => "Equity Forward".to_string(),
        TradeType::EquityOption { .. } => "Equity Option".to_string(),
        TradeType::EquitySwap { .. } => "Equity Swap".to_string(),
        TradeType::CreditDefaultSwap { .. } => "CDS".to_string(),
        TradeType::CreditDefaultSwapIndex { .. } => "CDX".to_string(),
        TradeType::CreditDefaultSwapOption { .. } => "CDS Option".to_string(),
        TradeType::CommodityForward { .. } => "Commodity Fwd".to_string(),
        TradeType::CommoditySwap { .. } => "Commodity Swap".to_string(),
        TradeType::CommodityOption { .. } => "Commodity Opt".to_string(),
        TradeType::Generic => "Generic".to_string(),
    }
}

/// Extract currency, notional, maturity date, counterparty, and book from an
/// `FpML` trade.
fn get_trade_details(trade: &FpmlTrade) -> (String, f64, String, String, String) {
    // Get currency and notional from first leg
    let (currency, notional) = trade
        .first_leg()
        .map(|leg| {
            let curr = format!("{:?}", leg.currency);
            let notl = leg
                .cashflows()
                .next()
                .map(|cf| cf.notional.abs())
                .unwrap_or(0.0);
            (curr, notl)
        })
        .unwrap_or_else(|| ("USD".to_string(), 0.0));

    // Get maturity as the last cashflow payment date (formatted as ISO 8601)
    let maturity = trade
        .all_cashflows()
        .map(|cf| cf.payment_date)
        .max()
        .map(|last_date| last_date.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    // Get counterparty from metadata
    let counterparty = trade
        .metadata
        .counterparty
        .as_ref()
        .map(|cp| cp.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Get book from metadata
    let book = trade
        .metadata
        .book
        .as_ref()
        .map(|b| b.as_str().to_string())
        .unwrap_or_else(|| "Unassigned".to_string());

    (currency, notional, maturity, counterparty, book)
}

/// Extract portfolio graph from `FpML` trades, optionally filtered to specific
/// trades
fn extract_fpml_portfolio_graph(
    trades: &[FpmlTrade],
    trade_ids: Option<&[String]>,
) -> Result<PortfolioComputationGraph, pricer_pricing::graph::GraphError> {
    let extractor = PortfolioGraphExtractor::new()
        .with_timeout(500)
        .with_capacity(5_000, 10_000);

    // First, extract individual trade graphs
    let mut trade_graphs: HashMap<String, ComputationGraph> = HashMap::new();

    // Get all trade IDs
    let all_trade_ids: Vec<String> = trades.iter().map(|t| t.id.to_string()).collect();

    // For each trade, create a graph based on its type
    for trade in trades {
        let trade_id = trade.id.to_string();
        let graph = create_fpml_trade_graph(&trade_id, trade);
        trade_graphs.insert(trade_id, graph);
    }

    // Extract the full portfolio graph
    let full_graph = extractor.extract_portfolio_graph(&all_trade_ids, &trade_graphs)?;

    // If specific trade_ids requested, extract subgraph
    if let Some(selected_ids) = trade_ids {
        extractor.extract_subgraph(&full_graph, selected_ids)
    } else {
        Ok(full_graph)
    }
}

/// Create a simplified computation graph for an `FpML` trade
fn create_fpml_trade_graph(trade_id: &str, trade: &FpmlTrade) -> ComputationGraph {
    use pricer_pricing::graph::{GraphBuilder, GraphEdge, GraphNode, NodeGroup, NodeType};

    let mk_node = |id: String, nt, label: &str, sens, group| GraphNode {
        id, node_type: nt, label: label.to_string(), value: None,
        is_sensitivity_target: sens, group, trade_ids: vec![trade_id.to_string()],
    };

    let mut builder = GraphBuilder::with_capacity(10, 15);

    let params: &[&str] = match &trade.trade_type {
        TradeType::Swap | TradeType::Ois | TradeType::BasisSwap => &["rate", "spread"],
        TradeType::Swaption { .. } | TradeType::CapFloor => &["rate", "vol", "strike"],
        TradeType::FxForward | TradeType::FxSpot | TradeType::FxSwap => &["spot", "rate_dom", "rate_for"],
        TradeType::FxOption { .. } | TradeType::FxBarrierOption { .. } => &["spot", "vol", "rate_dom", "rate_for", "strike"],
        TradeType::EquityOption { .. } => &["spot", "vol", "rate", "div", "strike"],
        TradeType::CreditDefaultSwap { .. } | TradeType::CreditDefaultSwapIndex { .. } => &["spread", "recovery", "rate"],
        TradeType::CommoditySwap { .. } => &["price", "rate"],
        _ => &["rate"],
    };

    let input_ids: Vec<String> = params.iter().map(|p| {
        let id = format!("{trade_id}_{p}");
        builder.add_node(mk_node(id.clone(), NodeType::Input, p, true, NodeGroup::Sensitivity));
        id
    }).collect();

    let calc_id = format!("{trade_id}_calc");
    builder.add_node(mk_node(calc_id.clone(), NodeType::Mul, "calculation", false, NodeGroup::Intermediate));
    for id in &input_ids {
        builder.add_edge(GraphEdge { source: id.clone(), target: calc_id.clone(), weight: None });
    }

    let out_id = format!("{trade_id}_price");
    builder.add_node(mk_node(out_id.clone(), NodeType::Output, "price", false, NodeGroup::Output));
    builder.add_edge(GraphEdge { source: calc_id, target: out_id, weight: None });

    builder.build(Some(trade_id.to_string()))
}

/// Convert `PortfolioComputationGraph` to response DTO
fn convert_to_response(graph: &PortfolioComputationGraph) -> PortfolioGraphResponse {
    let nodes: Vec<GraphNodeDto> = graph
        .nodes
        .iter()
        .map(|n| GraphNodeDto {
            id: n.id.clone(),
            node_type: format!("{:?}", n.node_type),
            label: n.label.clone(),
            value: n.value,
            is_sensitivity_target: n.is_sensitivity_target,
            group: format!("{:?}", n.group),
            trade_ids: n.trade_ids.clone(),
        })
        .collect();

    let edges: Vec<GraphEdgeDto> = graph
        .edges
        .iter()
        .map(|e| GraphEdgeDto {
            source: e.source.clone(),
            target: e.target.clone(),
            weight: e.weight,
        })
        .collect();

    let metadata = PortfolioGraphMetadataDto {
        node_count: graph.metadata.node_count,
        edge_count: graph.metadata.edge_count,
        depth: graph.metadata.depth,
        generated_at: graph.metadata.generated_at.clone(),
        trade_count: graph.metadata.trade_count,
        shared_node_count: graph.metadata.shared_node_count,
        optimisation_ratio: graph.metadata.optimisation_ratio,
        large_graph_warning: graph.metadata.node_count > 10_000,
    };

    PortfolioGraphResponse {
        nodes,
        edges,
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_cache_ttl() {
        let mut cache = GraphCache::new(1); // 1 second TTL

        // Create a minimal graph for testing
        let graph = PortfolioComputationGraph {
            nodes: vec![],
            edges: vec![],
            metadata: pricer_pricing::graph::PortfolioGraphMetadata {
                node_count: 0,
                edge_count: 0,
                depth: 0,
                generated_at: "test".to_string(),
                trade_count: 0,
                shared_node_count: 0,
                optimisation_ratio: 1.0,
            },
        };

        cache.set_full_graph(graph);
        assert!(cache.get_full_graph().is_some());

        // Wait for TTL to expire
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(cache.get_full_graph().is_none());
    }

    #[test]
    fn test_instrument_type_name() {
        assert_eq!(get_instrument_type_name(&TradeType::Swap), "IRS");
        assert_eq!(get_instrument_type_name(&TradeType::Ois), "OIS");
        assert_eq!(
            get_instrument_type_name(&TradeType::FxForward),
            "FX Forward"
        );
        assert_eq!(get_instrument_type_name(&TradeType::CapFloor), "Cap/Floor");
    }
}

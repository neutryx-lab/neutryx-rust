//! Portfolio graph REST API handlers.
//!
//! Provides endpoints for Portfolio-level computation graph extraction
//! with shared node deduplication and subgraph filtering.
//!
//! Note: This module is only used when `demo` feature is disabled.
#![allow(dead_code)]
//!
//! # Endpoints
//!
//! - `GET /api/v1/portfolio/graph` - Extract Portfolio computation graph
//! - `GET /api/v1/portfolio/trades` - List trades in Portfolio
//!
//! # Requirements Coverage
//!
//! - 4.1: `/api/v1/portfolio/graph` endpoint
//! - 4.2: Timeout and error handling
//! - 4.3: `/api/v1/portfolio/trades` endpoint
//! - 4.4: `GraphCache` implementation

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Query, State},
    Json,
};
use pricer_pricing::graph::{
    ComputationGraph, PortfolioComputationGraph, PortfolioGraphExtractable, PortfolioGraphExtractor,
};
use pricer_risk::portfolio::{Portfolio, SamplePortfolioBuilder, TradeId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::ServerError;

// ============================================================================
// Request/Response Types
// ============================================================================

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
    pub expiry: f64,
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

// ============================================================================
// Application State
// ============================================================================

/// Shared application state for graph handlers
pub struct GraphAppState {
    /// Cached sample portfolio
    pub portfolio: Portfolio,
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
    /// Create a new app state with a sample portfolio
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Internal` if the sample portfolio fails to build.
    pub fn new_with_sample(trade_count: usize, cache_ttl_secs: u64) -> Result<Self, ServerError> {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(trade_count)
            .build()
            .map_err(|e| {
                ServerError::Internal(format!("Failed to create sample portfolio: {e}"))
            })?;

        Ok(Self {
            portfolio,
            graph_cache: RwLock::new(GraphCache::new(cache_ttl_secs)),
            cache_ttl_secs,
        })
    }

    /// Create with default settings (50 trades, 5 second cache)
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Internal` if the sample portfolio fails to build.
    pub fn default_sample() -> Result<Self, ServerError> { Self::new_with_sample(50, 5) }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/v1/portfolio/graph
///
/// Extract the Portfolio computation graph with optional subgraph filtering.
///
/// # Query Parameters
///
/// - `trade_ids`: Comma-separated list of trade IDs for subgraph extraction
///
/// # Returns
///
/// - 200 OK: Portfolio graph in D3.js-compatible JSON format
/// - 404 Not Found: If any specified `trade_id` doesn't exist
/// - 500 Internal Server Error: If extraction fails
/// - 504 Gateway Timeout: If extraction exceeds 500ms
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
        extract_portfolio_graph(&state.portfolio, trade_ids.as_deref())
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

/// GET /api/v1/portfolio/trades
///
/// List all trades in the Portfolio with optional filtering.
///
/// # Query Parameters
///
/// - `instrument_type`: Filter by instrument type
/// - `currency`: Filter by currency
///
/// # Returns
///
/// - 200 OK: Trade list with statistics
pub async fn get_portfolio_trades(
    State(state): State<Arc<GraphAppState>>,
    Query(params): Query<TradeListQueryParams>,
) -> Result<Json<TradeListResponse>, ServerError> {
    let portfolio = &state.portfolio;

    let mut trades: Vec<TradeSummaryDto> = Vec::new();
    let mut by_instrument_type: HashMap<String, usize> = HashMap::new();
    let mut by_currency: HashMap<String, usize> = HashMap::new();
    let mut total_notional = 0.0;

    for trade in portfolio.trades() {
        let instrument_type = if trade.is_vanilla() {
            "vanilla"
        } else if trade.is_forward() {
            "forward"
        } else if trade.is_swap() {
            "swap"
        } else {
            "other"
        }
        .to_string();

        let currency = format!("{:?}", trade.currency());

        // Apply filters
        if let Some(ref filter_type) = params.instrument_type {
            if instrument_type != *filter_type {
                // Still count for statistics
                *by_instrument_type
                    .entry(instrument_type.clone())
                    .or_insert(0) += 1;
                *by_currency.entry(currency.clone()).or_insert(0) += 1;
                total_notional += trade.notional();
                continue;
            }
        }
        if let Some(ref filter_currency) = params.currency {
            if currency != *filter_currency {
                *by_instrument_type
                    .entry(instrument_type.clone())
                    .or_insert(0) += 1;
                *by_currency.entry(currency.clone()).or_insert(0) += 1;
                total_notional += trade.notional();
                continue;
            }
        }

        // Update statistics
        *by_instrument_type
            .entry(instrument_type.clone())
            .or_insert(0) += 1;
        *by_currency.entry(currency.clone()).or_insert(0) += 1;
        total_notional += trade.notional();

        trades.push(TradeSummaryDto {
            id: trade.id().to_string(),
            instrument_type,
            currency,
            notional: trade.notional(),
            expiry: trade.expiry(),
        });
    }

    Ok(Json(TradeListResponse {
        statistics: TradeStatisticsDto {
            total_count: portfolio.trade_count(),
            by_instrument_type,
            by_currency,
            total_notional,
        },
        trades,
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract portfolio graph, optionally filtered to specific trades
fn extract_portfolio_graph(
    portfolio: &Portfolio,
    trade_ids: Option<&[String]>,
) -> Result<PortfolioComputationGraph, pricer_pricing::graph::GraphError> {
    let extractor = PortfolioGraphExtractor::new()
        .with_timeout(500)
        .with_capacity(5_000, 10_000);

    // First, extract individual trade graphs
    let mut trade_graphs: HashMap<String, ComputationGraph> = HashMap::new();

    // Get all trade IDs
    let all_trade_ids: Vec<String> = portfolio
        .trade_ids()
        .map(std::string::ToString::to_string)
        .collect();

    // For each trade, create a mock graph (since we don't have real pricing
    // context) In production, this would come from the actual pricing engine
    for trade_id in &all_trade_ids {
        let trade = portfolio.trade(&TradeId::new(trade_id));
        if let Some(trade) = trade {
            // Create a simplified graph for the trade
            let graph = create_trade_graph(trade_id, trade);
            trade_graphs.insert(trade_id.clone(), graph);
        }
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

/// Create a simplified computation graph for a trade
pub(crate) fn create_trade_graph(
    trade_id: &str,
    trade: &pricer_risk::portfolio::Trade,
) -> ComputationGraph {
    use pricer_pricing::graph::{GraphBuilder, GraphEdge, GraphNode, NodeGroup, NodeType};

    let mut builder = GraphBuilder::with_capacity(10, 15);

    // Determine sensitivity params based on trade type
    let params = if trade.is_vanilla() {
        vec!["spot", "vol", "rate", "strike"]
    } else {
        // Forward and other trade types use spot and rate
        vec!["spot", "rate"]
    };

    // Create input nodes
    let mut input_ids = Vec::new();
    for param in &params {
        let node_id = format!("{trade_id}_{param}");
        let node = GraphNode {
            id: node_id.clone(),
            node_type: NodeType::Input,
            label: (*param).to_string(),
            value: None,
            is_sensitivity_target: true,
            group: NodeGroup::Sensitivity,
            trade_ids: vec![trade_id.to_string()],
        };
        builder.add_node(node);
        input_ids.push(node_id);
    }

    // Create intermediate node
    let intermediate_id = format!("{trade_id}_calc");
    let intermediate_node = GraphNode {
        id: intermediate_id.clone(),
        node_type: NodeType::Mul,
        label: "calculation".to_string(),
        value: None,
        is_sensitivity_target: false,
        group: NodeGroup::Intermediate,
        trade_ids: vec![trade_id.to_string()],
    };
    builder.add_node(intermediate_node);

    // Connect inputs to intermediate
    for input_id in &input_ids {
        builder.add_edge(GraphEdge {
            source: input_id.clone(),
            target: intermediate_id.clone(),
            weight: None,
        });
    }

    // Create output node
    let output_id = format!("{trade_id}_price");
    let output_node = GraphNode {
        id: output_id.clone(),
        node_type: NodeType::Output,
        label: "price".to_string(),
        value: None,
        is_sensitivity_target: false,
        group: NodeGroup::Output,
        trade_ids: vec![trade_id.to_string()],
    };
    builder.add_node(output_node);

    // Connect intermediate to output
    builder.add_edge(GraphEdge {
        source: intermediate_id,
        target: output_id,
        weight: None,
    });

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
    fn test_app_state_creation() {
        let state = GraphAppState::new_with_sample(10, 5);
        assert!(state.is_ok());
        let state = state.unwrap();
        assert_eq!(state.portfolio.trade_count(), 10);
    }
}

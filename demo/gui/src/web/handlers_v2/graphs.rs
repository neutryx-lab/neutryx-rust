//! Graph-related handlers and types.
//!
//! This module provides:
//! - Computation graph endpoints (`/api/graph`)
//! - Instrument graph endpoints (`/api/instrument-graph`)
//! - Portfolio graph endpoints (`/api/v1/portfolio/graph`)
//! - Graph caching with TTL support

use std::{collections::HashMap, sync::Arc, time::Instant};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::web::AppState;

// =============================================================================
// Graph Types (D3.js compatible)
// =============================================================================

/// Query parameters for graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQueryParams {
    /// Optional trade ID to filter graph extraction
    pub trade_id: Option<String>,
}

/// Graph node for API response (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeResponse {
    /// Unique identifier for the node
    pub id: String,
    /// Operation type (D3.js compatible: "type" field)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Human-readable label
    pub label: String,
    /// Current computed value
    pub value: Option<f64>,
    /// Whether this node is a sensitivity calculation target
    pub is_sensitivity_target: bool,
    /// Visual grouping for colour coding
    pub group: String,
}

/// Graph edge for API response (D3.js compatible: "links")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeResponse {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Optional edge weight
    pub weight: Option<f64>,
}

/// Graph metadata for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadataResponse {
    /// Trade ID (None for aggregate graphs)
    pub trade_id: Option<String>,
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Graph depth (longest path)
    pub depth: usize,
    /// Generation timestamp (ISO 8601)
    pub generated_at: String,
}

/// Graph API response (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    /// All nodes in the computation graph
    pub nodes: Vec<GraphNodeResponse>,
    /// All edges (D3.js compatible: "links")
    pub links: Vec<GraphEdgeResponse>,
    /// Graph metadata
    pub metadata: GraphMetadataResponse,
}

/// Error response for graph API
#[derive(Debug, Serialize)]
pub struct GraphErrorResponse {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// Graph Cache
// =============================================================================

/// Cached graph entry with timestamp
#[derive(Debug, Clone)]
pub struct CachedGraph {
    /// The cached graph response
    pub graph: GraphResponse,
    /// When the cache entry was created
    pub created_at: Instant,
}

/// Graph cache with TTL support
#[derive(Debug, Default)]
pub struct GraphCache {
    /// Cache entries by trade_id (None key = all trades)
    entries: HashMap<Option<String>, CachedGraph>,
}

impl GraphCache {
    /// Cache TTL in seconds
    const TTL_SECONDS: u64 = 5;

    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached graph if it exists and is not expired
    pub fn get(&self, trade_id: &Option<String>) -> Option<&GraphResponse> {
        self.entries.get(trade_id).and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS {
                Some(&entry.graph)
            } else {
                None
            }
        })
    }

    /// Insert a graph into the cache
    pub fn insert(&mut self, trade_id: Option<String>, graph: GraphResponse) {
        self.entries.insert(
            trade_id,
            CachedGraph {
                graph,
                created_at: Instant::now(),
            },
        );
    }

    /// Remove expired entries from the cache
    pub fn cleanup(&mut self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS);
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// =============================================================================
// Portfolio Graph Types
// =============================================================================

/// Query parameters for portfolio graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct PortfolioGraphQueryParams {
    /// Optional comma-separated list of trade IDs to filter
    pub trade_ids: Option<String>,
}

/// Portfolio graph node with trade ownership (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphNodeResponse {
    /// Unique identifier for the node
    pub id: String,
    /// Operation type (D3.js compatible: "type" field)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Human-readable label
    pub label: String,
    /// Current computed value
    pub value: Option<f64>,
    /// Whether this node is a sensitivity calculation target
    pub is_sensitivity_target: bool,
    /// Visual grouping for colour coding
    pub group: String,
    /// Trade IDs that share this node
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trade_ids: Vec<String>,
}

/// Portfolio graph metadata with optimisation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphMetadataResponse {
    /// Total number of nodes (after deduplication)
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Graph depth (longest path)
    pub depth: usize,
    /// Generation timestamp (ISO 8601)
    pub generated_at: String,
    /// Number of trades in the portfolio
    pub trade_count: usize,
    /// Number of shared (deduplicated) nodes
    pub shared_node_count: usize,
    /// Optimisation ratio (lower is better deduplication)
    pub optimisation_ratio: f64,
    /// Warning flag for large graphs (> 10,000 nodes)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub large_graph_warning: bool,
}

/// Portfolio graph API response (D3.js compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphResponse {
    /// All nodes in the portfolio computation graph
    pub nodes: Vec<PortfolioGraphNodeResponse>,
    /// All edges (D3.js compatible: "links")
    pub links: Vec<GraphEdgeResponse>,
    /// Portfolio graph metadata with optimisation statistics
    pub metadata: PortfolioGraphMetadataResponse,
}

// =============================================================================
// Portfolio Graph Cache
// =============================================================================

/// Cached portfolio graph entry with timestamp
#[derive(Debug, Clone)]
pub struct CachedPortfolioGraph {
    /// The cached portfolio graph response
    pub graph: PortfolioGraphResponse,
    /// When the cache entry was created
    pub created_at: Instant,
}

/// Portfolio graph cache with 5-second TTL
#[derive(Debug, Default)]
pub struct PortfolioGraphCache {
    /// Cache entries keyed by comma-sorted trade_ids (None = full graph)
    entries: HashMap<Option<String>, CachedPortfolioGraph>,
}

impl PortfolioGraphCache {
    /// Cache TTL in seconds
    const TTL_SECONDS: u64 = 5;

    /// Create a new empty portfolio graph cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Generate a cache key from optional trade_ids filter
    fn cache_key(trade_ids: Option<&[String]>) -> Option<String> {
        trade_ids.map(|ids| {
            let mut sorted: Vec<&String> = ids.iter().collect();
            sorted.sort();
            sorted
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
    }

    /// Get a cached portfolio graph if it exists and is not expired
    pub fn get(&self, trade_ids: Option<&[String]>) -> Option<&PortfolioGraphResponse> {
        let key = Self::cache_key(trade_ids);
        self.entries.get(&key).and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS {
                Some(&entry.graph)
            } else {
                None
            }
        })
    }

    /// Insert a portfolio graph into the cache
    pub fn insert(&mut self, trade_ids: Option<&[String]>, graph: PortfolioGraphResponse) {
        let key = Self::cache_key(trade_ids);
        self.entries.insert(
            key,
            CachedPortfolioGraph {
                graph,
                created_at: Instant::now(),
            },
        );
    }

    /// Remove expired entries from the cache
    pub fn cleanup(&mut self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed().as_secs() < Self::TTL_SECONDS);
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// =============================================================================
// Instrument Graph Types
// =============================================================================

/// Query parameters for instrument graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentGraphQueryParams {
    /// Optional currency to filter (default: USD)
    pub currency: Option<String>,
    /// Optional index type to filter (default: OIS)
    pub index_type: Option<String>,
}

/// Instrument graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentGraphNode {
    /// Unique identifier
    pub id: String,
    /// Node type (curve, instrument, quote)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Display label
    pub label: String,
    /// Node group for visualisation
    pub group: String,
    /// Current value if applicable
    pub value: Option<f64>,
    /// Instrument tenor if applicable
    pub tenor: Option<String>,
}

/// Instrument graph metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentGraphMetadata {
    /// Currency of the curve
    pub currency: String,
    /// Index type
    pub index_type: String,
    /// Total nodes
    pub node_count: usize,
    /// Total edges
    pub edge_count: usize,
    /// Generation timestamp
    pub generated_at: String,
}

/// Instrument graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentGraphResponse {
    /// Graph nodes
    pub nodes: Vec<InstrumentGraphNode>,
    /// Graph edges
    pub links: Vec<GraphEdgeResponse>,
    /// Metadata
    pub metadata: InstrumentGraphMetadata,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get parameters for a specific trade
fn get_trade_params(trade_id: &str) -> Vec<String> {
    match trade_id {
        "T001" => vec![
            "spot".to_string(),
            "vol".to_string(),
            "rate".to_string(),
            "time".to_string(),
        ],
        "T002" => vec![
            "fx_spot".to_string(),
            "dom_rate".to_string(),
            "for_rate".to_string(),
        ],
        "T003" => vec![
            "swap_rate".to_string(),
            "discount".to_string(),
            "notional".to_string(),
            "tenor".to_string(),
        ],
        "T004" => vec!["eur_usd".to_string(), "vol".to_string(), "rate".to_string()],
        "T005" => vec![
            "spread".to_string(),
            "recovery".to_string(),
            "hazard".to_string(),
        ],
        _ => vec!["param1".to_string(), "param2".to_string()],
    }
}

/// Check if a trade exists
fn trade_exists(trade_id: &str) -> bool {
    matches!(trade_id, "T001" | "T002" | "T003" | "T004" | "T005")
}

/// Generate a sample computation graph for a trade
fn generate_sample_graph(trade_id: Option<&str>) -> GraphResponse {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    let trades_data = if let Some(tid) = trade_id {
        vec![(tid.to_string(), get_trade_params(tid))]
    } else {
        vec![
            ("T001".to_string(), get_trade_params("T001")),
            ("T002".to_string(), get_trade_params("T002")),
            ("T003".to_string(), get_trade_params("T003")),
        ]
    };

    for (tid, params) in &trades_data {
        let mut intermediate_ids = Vec::new();

        // Create input nodes for each parameter
        for (i, param) in params.iter().enumerate() {
            let node_id = format!("{}_{}", tid, param);
            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: "input".to_string(),
                label: param.clone(),
                value: Some(100.0 + (i as f64) * 10.0),
                is_sensitivity_target: true,
                group: "sensitivity".to_string(),
            });
        }

        // Create intermediate computation nodes
        for (i, chunk) in params.chunks(2).enumerate() {
            let node_id = format!("{}_op_{}", tid, i);
            let label = if chunk.len() == 2 {
                format!("{} * {}", chunk[0], chunk[1])
            } else {
                format!("exp({})", chunk[0])
            };
            let node_type = if chunk.len() == 2 { "mul" } else { "exp" };

            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: node_type.to_string(),
                label,
                value: Some(25.0 + (i as f64) * 5.0),
                is_sensitivity_target: false,
                group: "intermediate".to_string(),
            });

            // Add edges from inputs to operation
            for param in chunk {
                links.push(GraphEdgeResponse {
                    source: format!("{}_{}", tid, param),
                    target: node_id.clone(),
                    weight: None,
                });
            }

            intermediate_ids.push(node_id);
        }

        // Create second level combination nodes
        let mut second_level_ids = Vec::new();
        for (i, chunk) in intermediate_ids.chunks(2).enumerate() {
            let node_id = format!("{}_combine_{}", tid, i);
            let label = if chunk.len() == 2 {
                format!("{} + {}", chunk[0], chunk[1])
            } else {
                format!("sqrt({})", chunk[0])
            };
            let node_type = if chunk.len() == 2 { "add" } else { "sqrt" };

            nodes.push(GraphNodeResponse {
                id: node_id.clone(),
                node_type: node_type.to_string(),
                label,
                value: Some(50.0 + (i as f64) * 10.0),
                is_sensitivity_target: false,
                group: "intermediate".to_string(),
            });

            for source in chunk {
                links.push(GraphEdgeResponse {
                    source: source.clone(),
                    target: node_id.clone(),
                    weight: None,
                });
            }

            second_level_ids.push(node_id);
        }

        // Create output node
        let output_id = format!("{}_price", tid);
        nodes.push(GraphNodeResponse {
            id: output_id.clone(),
            node_type: "output".to_string(),
            label: "price".to_string(),
            value: Some(125.5),
            is_sensitivity_target: false,
            group: "output".to_string(),
        });

        // Connect final nodes to output
        let final_sources = if second_level_ids.is_empty() {
            &intermediate_ids
        } else {
            &second_level_ids
        };
        for source in final_sources {
            links.push(GraphEdgeResponse {
                source: source.clone(),
                target: output_id.clone(),
                weight: None,
            });
        }
    }

    let depth = if nodes.is_empty() { 0 } else { 4 };
    let generated_at = chrono::Utc::now().to_rfc3339();

    GraphResponse {
        metadata: GraphMetadataResponse {
            trade_id: trade_id.map(String::from),
            node_count: nodes.len(),
            edge_count: links.len(),
            depth,
            generated_at,
        },
        nodes,
        links,
    }
}

/// Generate a sample instrument graph
fn generate_instrument_graph(currency: &str, index_type: &str) -> InstrumentGraphResponse {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Curve node
    let curve_id = format!("{}_{}_curve", currency, index_type);
    nodes.push(InstrumentGraphNode {
        id: curve_id.clone(),
        node_type: "curve".to_string(),
        label: format!("{} {} Curve", currency, index_type),
        group: "curve".to_string(),
        value: None,
        tenor: None,
    });

    // Instrument nodes
    let tenors = ["1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y", "30Y"];
    for tenor in &tenors {
        let inst_id = format!("{}_{}_{}", currency, index_type, tenor);
        nodes.push(InstrumentGraphNode {
            id: inst_id.clone(),
            node_type: "instrument".to_string(),
            label: format!("{} {}", index_type, tenor),
            group: "instrument".to_string(),
            value: Some(0.04 + 0.001 * tenors.iter().position(|t| t == tenor).unwrap() as f64),
            tenor: Some(tenor.to_string()),
        });

        links.push(GraphEdgeResponse {
            source: inst_id,
            target: curve_id.clone(),
            weight: None,
        });
    }

    let generated_at = chrono::Utc::now().to_rfc3339();

    InstrumentGraphResponse {
        nodes: nodes.clone(),
        links: links.clone(),
        metadata: InstrumentGraphMetadata {
            currency: currency.to_string(),
            index_type: index_type.to_string(),
            node_count: nodes.len(),
            edge_count: links.len(),
            generated_at,
        },
    }
}

/// Generate a sample portfolio graph
fn generate_sample_portfolio_graph(trade_ids_filter: Option<&[String]>) -> PortfolioGraphResponse {
    let all_trade_ids = [
        "T001".to_string(),
        "T002".to_string(),
        "T003".to_string(),
        "T004".to_string(),
        "T005".to_string(),
    ];

    let trade_ids: Vec<&String> = match trade_ids_filter {
        Some(filter) => all_trade_ids
            .iter()
            .filter(|tid| filter.contains(tid))
            .collect(),
        None => all_trade_ids.iter().collect(),
    };

    let mut nodes: Vec<PortfolioGraphNodeResponse> = Vec::new();
    let mut links: Vec<GraphEdgeResponse> = Vec::new();
    let mut shared_count = 0;

    // Shared USD Spot node
    let usd_spot_trades: Vec<String> = trade_ids
        .iter()
        .filter(|tid| matches!(tid.as_str(), "T001" | "T002" | "T003"))
        .map(|s| (*s).clone())
        .collect();
    if !usd_spot_trades.is_empty() {
        shared_count += 1;
        nodes.push(PortfolioGraphNodeResponse {
            id: "shared_usd_spot".to_string(),
            node_type: "input".to_string(),
            label: "USD Spot".to_string(),
            value: Some(100.0),
            is_sensitivity_target: true,
            group: "input".to_string(),
            trade_ids: usd_spot_trades,
        });
    }

    // Per-trade output nodes
    for tid in &trade_ids {
        let output_id = format!("{}_price", tid);
        nodes.push(PortfolioGraphNodeResponse {
            id: output_id.clone(),
            node_type: "output".to_string(),
            label: format!("{} Price", tid),
            value: Some(100.0),
            is_sensitivity_target: false,
            group: "output".to_string(),
            trade_ids: vec![(*tid).clone()],
        });

        // Link from shared node if applicable
        if matches!(tid.as_str(), "T001" | "T002" | "T003") {
            links.push(GraphEdgeResponse {
                source: "shared_usd_spot".to_string(),
                target: output_id,
                weight: None,
            });
        }
    }

    let generated_at = chrono::Utc::now().to_rfc3339();
    let node_count = nodes.len();
    let edge_count = links.len();

    PortfolioGraphResponse {
        nodes,
        links,
        metadata: PortfolioGraphMetadataResponse {
            node_count,
            edge_count,
            depth: 2,
            generated_at,
            trade_count: trade_ids.len(),
            shared_node_count: shared_count,
            optimisation_ratio: if trade_ids.is_empty() {
                1.0
            } else {
                node_count as f64 / (trade_ids.len() * 5) as f64
            },
            large_graph_warning: node_count > 10000,
        },
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// Get computation graph endpoint
///
/// GET /api/graph
pub async fn get_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphQueryParams>,
) -> Result<Json<GraphResponse>, (StatusCode, Json<GraphErrorResponse>)> {
    let start = Instant::now();

    // Validate trade exists
    if let Some(ref trade_id) = params.trade_id {
        if !trade_exists(trade_id) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(GraphErrorResponse {
                    error_type: "TradeNotFound".to_string(),
                    message: format!("Trade '{}' not found", trade_id),
                }),
            ));
        }
    }

    // Check cache
    {
        let cache = state.graph_cache.read().await;
        if let Some(cached) = cache.get(&params.trade_id) {
            let elapsed_us = start.elapsed().as_micros() as u64;
            state.metrics.record_graph_time(elapsed_us).await;
            return Ok(Json(cached.clone()));
        }
    }

    // Generate graph
    let graph = generate_sample_graph(params.trade_id.as_deref());

    // Update cache
    {
        let mut cache = state.graph_cache.write().await;
        cache.insert(params.trade_id.clone(), graph.clone());
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_graph_time(elapsed_us).await;

    Ok(Json(graph))
}

/// Get instrument graph endpoint
///
/// GET /api/instrument-graph
pub async fn get_instrument_graph(
    Query(params): Query<InstrumentGraphQueryParams>,
) -> Json<InstrumentGraphResponse> {
    let currency = params.currency.as_deref().unwrap_or("USD");
    let index_type = params.index_type.as_deref().unwrap_or("OIS");

    Json(generate_instrument_graph(currency, index_type))
}

/// Get portfolio graph endpoint
///
/// GET /api/v1/portfolio/graph
pub async fn get_portfolio_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PortfolioGraphQueryParams>,
) -> Json<PortfolioGraphResponse> {
    let start = Instant::now();

    // Parse trade_ids filter
    let trade_ids_filter: Option<Vec<String>> = params
        .trade_ids
        .as_ref()
        .map(|ids| ids.split(',').map(|s| s.trim().to_string()).collect());

    // Check cache
    {
        let cache = state.portfolio_graph_cache.read().await;
        if let Some(cached) = cache.get(trade_ids_filter.as_deref()) {
            let elapsed_us = start.elapsed().as_micros() as u64;
            state.metrics.record_graph_time(elapsed_us).await;
            return Json(cached.clone());
        }
    }

    // Generate graph
    let graph = generate_sample_portfolio_graph(trade_ids_filter.as_deref());

    // Update cache
    {
        let mut cache = state.portfolio_graph_cache.write().await;
        cache.insert(trade_ids_filter.as_deref(), graph.clone());
    }

    let elapsed_us = start.elapsed().as_micros() as u64;
    state.metrics.record_graph_time(elapsed_us).await;

    Json(graph)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_cache_ttl() {
        let mut cache = GraphCache::new();
        let graph = generate_sample_graph(Some("T001"));

        cache.insert(Some("T001".to_string()), graph);
        assert!(cache.get(&Some("T001".to_string())).is_some());
    }

    #[test]
    fn test_portfolio_graph_cache() {
        let mut cache = PortfolioGraphCache::new();
        assert!(cache.is_empty());

        let graph = generate_sample_portfolio_graph(None);
        cache.insert(None, graph);

        assert_eq!(cache.len(), 1);
        assert!(cache.get(None).is_some());
    }

    #[test]
    fn test_trade_exists() {
        assert!(trade_exists("T001"));
        assert!(trade_exists("T005"));
        assert!(!trade_exists("T999"));
    }

    #[test]
    fn test_generate_sample_graph() {
        let graph = generate_sample_graph(Some("T001"));
        assert!(!graph.nodes.is_empty());
        assert!(!graph.links.is_empty());
        assert_eq!(graph.metadata.trade_id, Some("T001".to_string()));
    }

    #[test]
    fn test_generate_instrument_graph() {
        let graph = generate_instrument_graph("USD", "OIS");
        assert!(!graph.nodes.is_empty());
        assert_eq!(graph.metadata.currency, "USD");
    }

    #[test]
    fn test_generate_portfolio_graph() {
        let graph = generate_sample_portfolio_graph(None);
        assert!(!graph.nodes.is_empty());
        assert_eq!(graph.metadata.trade_count, 5);
    }
}

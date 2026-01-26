//! Graph-related handlers.
//!
//! This module provides graph visualization handlers:
//! - Computation graph endpoints (`/api/graph`)
//! - Instrument graph endpoints (`/api/instrument-graph`)
//! - Portfolio graph endpoints (`/api/v1/portfolio/graph`)

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::web::AppState;

// =============================================================================
// Core Graph Types
// =============================================================================

/// Query parameters for the graph endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQueryParams {
    /// Optional trade ID to filter the graph
    pub trade_id: Option<String>,
}

/// Node in a computation graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeResponse {
    /// Unique node identifier
    pub id: String,
    /// Node type (input, output, mul, add, etc.)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Display label
    pub label: String,
    /// Current value if applicable
    pub value: Option<f64>,
    /// Whether this is a sensitivity target
    pub is_sensitivity_target: bool,
    /// Node group for visualisation
    pub group: String,
}

/// Edge in a computation graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeResponse {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Edge weight if applicable
    pub weight: Option<f64>,
}

/// Graph metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadataResponse {
    /// Trade ID if single-trade graph
    pub trade_id: Option<String>,
    /// Total node count
    pub node_count: usize,
    /// Total edge count
    pub edge_count: usize,
    /// Graph depth
    pub depth: usize,
    /// Generation timestamp
    pub generated_at: String,
}

/// Full graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    /// Graph metadata
    pub metadata: GraphMetadataResponse,
    /// Graph nodes
    pub nodes: Vec<GraphNodeResponse>,
    /// Graph edges
    pub links: Vec<GraphEdgeResponse>,
}

/// Error response for graph endpoints
#[derive(Debug, Clone, Serialize)]
pub struct GraphErrorResponse {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// Portfolio Graph Types
// =============================================================================

/// Query parameters for portfolio graph
#[derive(Debug, Clone, Deserialize)]
pub struct PortfolioGraphQueryParams {
    /// Comma-separated trade IDs to include
    pub trade_ids: Option<String>,
}

/// Node in a portfolio graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphNodeResponse {
    /// Unique node identifier
    pub id: String,
    /// Node type
    #[serde(rename = "type")]
    pub node_type: String,
    /// Display label
    pub label: String,
    /// Current value if applicable
    pub value: Option<f64>,
    /// Whether this is a sensitivity target
    pub is_sensitivity_target: bool,
    /// Node group
    pub group: String,
    /// Trade IDs that share this node
    pub trade_ids: Vec<String>,
}

/// Portfolio graph metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphMetadataResponse {
    /// Total node count
    pub node_count: usize,
    /// Total edge count
    pub edge_count: usize,
    /// Graph depth
    pub depth: usize,
    /// Generation timestamp
    pub generated_at: String,
    /// Number of trades
    pub trade_count: usize,
    /// Number of shared nodes
    pub shared_node_count: usize,
    /// Optimisation ratio
    pub optimisation_ratio: f64,
    /// Warning for large graphs
    pub large_graph_warning: bool,
}

/// Portfolio graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGraphResponse {
    /// Graph nodes
    pub nodes: Vec<PortfolioGraphNodeResponse>,
    /// Graph edges
    pub links: Vec<GraphEdgeResponse>,
    /// Metadata
    pub metadata: PortfolioGraphMetadataResponse,
}

// =============================================================================
// Cache Types
// =============================================================================

/// Cached graph entry
#[derive(Debug, Clone)]
pub struct CachedGraph {
    /// The cached graph response
    pub graph: GraphResponse,
    /// Cache timestamp
    pub cached_at: std::time::Instant,
}

/// Graph cache (trade_id -> graph)
pub type GraphCache = HashMap<Option<String>, GraphResponse>;

/// Cached portfolio graph entry
#[derive(Debug, Clone)]
pub struct CachedPortfolioGraph {
    /// The cached portfolio graph response
    pub graph: PortfolioGraphResponse,
    /// Cache timestamp
    pub cached_at: std::time::Instant,
}

/// Portfolio graph cache wrapper
#[derive(Debug, Clone, Default)]
pub struct PortfolioGraphCache {
    entries: HashMap<String, PortfolioGraphResponse>,
}

impl PortfolioGraphCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached entry by trade IDs
    pub fn get(&self, trade_ids: Option<&[String]>) -> Option<&PortfolioGraphResponse> {
        let key = Self::make_key(trade_ids);
        self.entries.get(&key)
    }

    /// Insert a new entry
    pub fn insert(&mut self, trade_ids: Option<&[String]>, graph: PortfolioGraphResponse) {
        let key = Self::make_key(trade_ids);
        self.entries.insert(key, graph);
    }

    fn make_key(trade_ids: Option<&[String]>) -> String {
        match trade_ids {
            Some(ids) => ids.join(","),
            None => "__all__".to_string(),
        }
    }
}

// =============================================================================
// Instrument Graph Types (new in handlers)
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
pub fn generate_sample_graph(trade_id: Option<&str>) -> GraphResponse {
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
pub fn generate_sample_portfolio_graph(
    trade_ids_filter: Option<&[String]>,
) -> PortfolioGraphResponse {
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
    let start = std::time::Instant::now();

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
    let start = std::time::Instant::now();

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

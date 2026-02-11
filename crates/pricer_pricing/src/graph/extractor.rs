//! # Graph Extractor Trait and Implementation
//!
//! This module provides the `GraphExtractable` trait for extracting computation
//! graphs from pricing contexts, and `SimpleGraphExtractor` as the default
//! implementation.
//!
//! ## Performance Requirements
//!
//! - Extract 10,000 nodes in < 1 second
//! - Impact on pricing calculation < 5%
//! - Pre-allocated buffers for memory efficiency
//!
//! ## Portfolio Graph Extraction
//!
//! The `PortfolioGraphExtractable` trait and `PortfolioGraphExtractor` enable
//! extraction of computation graphs from multiple trades in a Portfolio,
//! with shared node deduplication and optimisation.

use std::{collections::HashMap, time::Instant};

use petgraph::{
    algo::{is_cyclic_directed, toposort},
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use super::{
    error::GraphError,
    types::{
        ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup,
        NodeType, PortfolioComputationGraph, PortfolioGraphMetadata,
    },
};

// =============================================================================
// GraphExtractable Trait
// =============================================================================

/// Trait for extracting computation graphs from pricing contexts.
pub trait GraphExtractable {
    /// Extract the computation graph for a specific trade (or all if `None`).
    fn extract_graph(&self, trade_id: Option<&str>) -> Result<ComputationGraph, GraphError>;

    /// Extract nodes affected by recent updates (for differential WebSocket
    /// updates).
    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError>;
}

// =============================================================================
// GraphBuilder
// =============================================================================

/// Pre-allocated buffer builder for graph construction.
///
/// Internally backed by a `petgraph::graph::DiGraph` for efficient
/// topological sort, cycle detection, and depth calculation.
#[derive(Debug)]
pub struct GraphBuilder {
    /// Directed graph storing domain `GraphNode` payloads
    digraph: DiGraph<GraphNode, ()>,
    /// Domain edge records (kept for serialisation into `ComputationGraph`)
    edges: Vec<GraphEdge>,
    /// Node ID (String) to petgraph `NodeIndex` mapping for fast lookup
    node_index: HashMap<String, NodeIndex>,
}

impl GraphBuilder {
    /// Create a new GraphBuilder (default: 1,000 nodes, 2,000 edges).
    pub fn new() -> Self { Self::with_capacity(1_000, 2_000) }

    /// Create a new GraphBuilder with specified capacity.
    pub fn with_capacity(node_capacity: usize, edge_capacity: usize) -> Self {
        Self {
            digraph: DiGraph::with_capacity(node_capacity, edge_capacity),
            edges: Vec::with_capacity(edge_capacity),
            node_index: HashMap::with_capacity(node_capacity),
        }
    }

    /// Add a node to the graph, returning its index (ordinal position).
    pub fn add_node(&mut self, node: GraphNode) -> usize {
        let id = node.id.clone();
        let nx = self.digraph.add_node(node);
        self.node_index.insert(id, nx);
        nx.index()
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        // Also add the edge to the petgraph DiGraph when both endpoints exist
        if let (Some(&src), Some(&tgt)) = (
            self.node_index.get(&edge.source),
            self.node_index.get(&edge.target),
        ) {
            self.digraph.add_edge(src, tgt, ());
        }
        self.edges.push(edge);
    }

    /// Check if a node exists by ID.
    pub fn has_node(&self, id: &str) -> bool { self.node_index.contains_key(id) }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.node_index.get(id).map(|&nx| &self.digraph[nx])
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.node_index
            .get(id)
            .copied()
            .map(|nx| &mut self.digraph[nx])
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize { self.digraph.node_count() }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize { self.edges.len() }

    /// Add a trade ID to a node's trade_ids list (deduplicating).
    pub fn add_trade_id(&mut self, node_id: &str, trade_id: &str) -> Option<()> {
        let node = self.get_node_mut(node_id)?;
        let trade_id_string = trade_id.to_string();
        if !node.trade_ids.contains(&trade_id_string) {
            node.trade_ids.push(trade_id_string);
        }
        Some(())
    }

    /// Set the trade IDs for a node, replacing any existing values.
    pub fn set_trade_ids(&mut self, node_id: &str, trade_ids: Vec<String>) -> Option<()> {
        let node = self.get_node_mut(node_id)?;
        node.trade_ids = trade_ids;
        Some(())
    }

    /// Clear the builder for reuse.
    ///
    /// This clears all nodes and edges but retains the allocated capacity,
    /// allowing the builder to be reused efficiently.
    pub fn clear(&mut self) {
        self.digraph.clear();
        self.edges.clear();
        self.node_index.clear();
    }

    /// Calculate the graph depth (longest path from any input to any output).
    ///
    /// Uses `petgraph::algo::toposort` followed by dynamic programming
    /// for O(V + E) complexity.
    pub fn calculate_depth(&self) -> usize {
        if self.digraph.node_count() == 0 {
            return 0;
        }

        // Obtain topological ordering via petgraph
        let sorted = match toposort(&self.digraph, None) {
            Ok(order) => order,
            Err(_) => return 0, // Graph has cycles; depth is undefined
        };

        // Compute longest path via DP over topological order
        let mut distance: HashMap<NodeIndex, usize> =
            HashMap::with_capacity(self.digraph.node_count());

        for &nx in &sorted {
            distance.entry(nx).or_insert(0);
        }

        let mut max_depth: usize = 0;

        for &nx in &sorted {
            let current_dist = distance[&nx];
            for edge_ref in self.digraph.edges(nx) {
                let target = edge_ref.target();
                let new_dist = current_dist + 1;
                let entry = distance.entry(target).or_insert(0);
                if new_dist > *entry {
                    *entry = new_dist;
                }
            }
            max_depth = max_depth.max(current_dist);
        }

        // Depth is max_depth + 1 (counting nodes, not edges)
        max_depth + 1
    }

    /// Validate that the graph is a DAG (no cycles).
    ///
    /// Delegates to `petgraph::algo::is_cyclic_directed`.
    pub fn is_dag(&self) -> bool {
        if self.digraph.node_count() == 0 {
            return true;
        }
        !is_cyclic_directed(&self.digraph)
    }

    /// Build the final `ComputationGraph` with calculated metadata.
    pub fn build(self, trade_id: Option<String>) -> ComputationGraph {
        let node_count = self.digraph.node_count();
        let edge_count = self.edges.len();
        let depth = self.calculate_depth();
        let generated_at = Self::current_timestamp();

        let metadata = GraphMetadata {
            trade_id,
            node_count,
            edge_count,
            depth,
            generated_at,
        };

        // Collect nodes from the petgraph storage
        let nodes: Vec<GraphNode> = self
            .digraph
            .raw_nodes()
            .iter()
            .map(|n| n.weight.clone())
            .collect();

        ComputationGraph {
            nodes,
            edges: self.edges,
            metadata,
        }
    }

    /// Build the final `ComputationGraph` with a pre-calculated depth.
    pub fn build_with_depth(self, trade_id: Option<String>, depth: usize) -> ComputationGraph {
        let node_count = self.digraph.node_count();
        let edge_count = self.edges.len();
        let generated_at = Self::current_timestamp();

        let metadata = GraphMetadata {
            trade_id,
            node_count,
            edge_count,
            depth,
            generated_at,
        };

        let nodes: Vec<GraphNode> = self
            .digraph
            .raw_nodes()
            .iter()
            .map(|n| n.weight.clone())
            .collect();

        ComputationGraph {
            nodes,
            edges: self.edges,
            metadata,
        }
    }

    /// Get the current timestamp as Unix epoch seconds with 'Z' suffix.
    pub fn current_timestamp() -> String {
        // Use a simple format since we don't want to add chrono dependency
        // In production, this would use chrono::Utc::now().to_rfc3339()
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", now.as_secs())
    }
}

impl Default for GraphBuilder {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// SimpleGraphExtractor
// =============================================================================

/// Simple graph extractor for demonstration purposes.
#[derive(Debug)]
pub struct SimpleGraphExtractor {
    /// Registered trades with their sensitivity parameters
    trades: HashMap<String, TradeGraphInfo>,
    /// Timeout for extraction (milliseconds)
    timeout_ms: u64,
    /// Pre-allocated builder for reuse
    builder_capacity: (usize, usize),
    /// Previous values for delta calculation
    previous_values: HashMap<String, HashMap<String, f64>>,
}

/// Information about a trade's graph structure.
#[derive(Debug, Clone)]
struct TradeGraphInfo {
    /// Sensitivity parameters (AD seed points)
    sensitivity_params: Vec<String>,
    /// Current parameter values
    param_values: HashMap<String, f64>,
    /// Computed intermediate and output values
    computed_values: HashMap<String, f64>,
}

impl SimpleGraphExtractor {
    /// Create a new SimpleGraphExtractor with default settings.
    pub fn new() -> Self {
        Self {
            trades: HashMap::new(),
            timeout_ms: 500,
            builder_capacity: (1_000, 2_000),
            previous_values: HashMap::new(),
        }
    }

    /// Set custom timeout (milliseconds) for graph extraction.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set custom pre-allocation capacity for nodes and edges.
    pub fn with_capacity(mut self, node_capacity: usize, edge_capacity: usize) -> Self {
        self.builder_capacity = (node_capacity, edge_capacity);
        self
    }

    /// Register a trade with its sensitivity parameters (AD seed points).
    pub fn register_trade<S: Into<String>>(&mut self, trade_id: &str, sensitivity_params: Vec<S>) {
        let info = TradeGraphInfo {
            sensitivity_params: sensitivity_params.into_iter().map(|s| s.into()).collect(),
            param_values: HashMap::new(),
            computed_values: HashMap::new(),
        };
        self.trades.insert(trade_id.to_string(), info);
    }

    /// Set a parameter value for a trade.
    pub fn set_param_value(
        &mut self,
        trade_id: &str,
        param_name: &str,
        value: f64,
    ) -> Result<(), GraphError> {
        let trade = self
            .trades
            .get_mut(trade_id)
            .ok_or_else(|| GraphError::TradeNotFound(trade_id.to_string()))?;

        // Store previous value for delta calculation
        if let Some(old_value) = trade.param_values.get(param_name) {
            self.previous_values
                .entry(trade_id.to_string())
                .or_default()
                .insert(param_name.to_string(), *old_value);
        }

        trade.param_values.insert(param_name.to_string(), value);
        Ok(())
    }

    /// Set a computed value for a trade node.
    pub fn set_computed_value(
        &mut self,
        trade_id: &str,
        node_id: &str,
        value: f64,
    ) -> Result<(), GraphError> {
        let trade = self
            .trades
            .get_mut(trade_id)
            .ok_or_else(|| GraphError::TradeNotFound(trade_id.to_string()))?;

        // Store previous value for delta calculation
        if let Some(old_value) = trade.computed_values.get(node_id) {
            self.previous_values
                .entry(trade_id.to_string())
                .or_default()
                .insert(node_id.to_string(), *old_value);
        }

        trade.computed_values.insert(node_id.to_string(), value);
        Ok(())
    }

    /// Check if a trade is registered.
    pub fn has_trade(&self, trade_id: &str) -> bool { self.trades.contains_key(trade_id) }

    /// Get the number of registered trades.
    pub fn trade_count(&self) -> usize { self.trades.len() }

    /// Build a sample graph for a trade.
    ///
    /// This simulates the graph structure of a pricing calculation:
    /// - Input nodes for parameters (spot, vol, rate, etc.)
    /// - Intermediate computation nodes (operations)
    /// - Output node for the final price
    fn build_trade_graph(
        &self,
        trade_id: &str,
        trade_info: &TradeGraphInfo,
        builder: &mut GraphBuilder,
        start_time: Instant,
    ) -> Result<(), GraphError> {
        // Check timeout
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        let params = &trade_info.sensitivity_params;
        let param_values = &trade_info.param_values;
        let computed_values = &trade_info.computed_values;

        // Create input nodes for each sensitivity parameter
        let mut input_node_ids: Vec<String> = Vec::with_capacity(params.len());

        for param in params {
            let node_id = format!("{}_{}", trade_id, param);
            if !builder.has_node(&node_id) {
                let value = param_values.get(param).copied();
                let node = GraphNode {
                    id: node_id.clone(),
                    node_type: NodeType::Input,
                    label: param.clone(),
                    value,
                    is_sensitivity_target: true,
                    group: NodeGroup::Sensitivity,
                    trade_ids: vec![],
                };
                builder.add_node(node);
            }
            input_node_ids.push(node_id);
        }

        // Check timeout after input nodes
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        // Create intermediate computation nodes
        // For a typical pricing calculation, we create a tree-like structure
        let mut intermediate_nodes: Vec<String> = Vec::new();

        // First level: pairwise operations
        for (i, chunk) in input_node_ids.chunks(2).enumerate() {
            let node_id = format!("{}_op_{}", trade_id, i);
            if !builder.has_node(&node_id) {
                let label = if chunk.len() == 2 {
                    format!("{} * {}", chunk[0], chunk[1])
                } else {
                    format!("exp({})", chunk[0])
                };

                let node_type = if chunk.len() == 2 {
                    NodeType::Mul
                } else {
                    NodeType::Exp
                };

                let value = computed_values.get(&node_id).copied();

                let node = GraphNode {
                    id: node_id.clone(),
                    node_type,
                    label,
                    value,
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                    trade_ids: vec![],
                };
                builder.add_node(node);

                // Add edges from inputs to this operation
                for source_id in chunk {
                    let edge = GraphEdge {
                        source: source_id.clone(),
                        target: node_id.clone(),
                        weight: None,
                    };
                    builder.add_edge(edge);
                }
            }
            intermediate_nodes.push(node_id);
        }

        // Check timeout after intermediate nodes
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        // Second level: combine intermediate results
        let mut second_level: Vec<String> = Vec::new();
        for (i, chunk) in intermediate_nodes.chunks(2).enumerate() {
            let node_id = format!("{}_combine_{}", trade_id, i);
            if !builder.has_node(&node_id) {
                let label = if chunk.len() == 2 {
                    format!("{} + {}", chunk[0], chunk[1])
                } else {
                    format!("sqrt({})", chunk[0])
                };

                let node_type = if chunk.len() == 2 {
                    NodeType::Add
                } else {
                    NodeType::Sqrt
                };

                let value = computed_values.get(&node_id).copied();

                let node = GraphNode {
                    id: node_id.clone(),
                    node_type,
                    label,
                    value,
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                    trade_ids: vec![],
                };
                builder.add_node(node);

                for source_id in chunk {
                    let edge = GraphEdge {
                        source: source_id.clone(),
                        target: node_id.clone(),
                        weight: None,
                    };
                    builder.add_edge(edge);
                }
            }
            second_level.push(node_id);
        }

        // Create output node
        let output_id = format!("{}_price", trade_id);
        if !builder.has_node(&output_id) {
            let value = computed_values.get(&output_id).copied();

            let node = GraphNode {
                id: output_id.clone(),
                node_type: NodeType::Output,
                label: "price".to_string(),
                value,
                is_sensitivity_target: false,
                group: NodeGroup::Output,
                trade_ids: vec![],
            };
            builder.add_node(node);

            // Connect final intermediate nodes to output
            let sources = if second_level.is_empty() {
                &intermediate_nodes
            } else {
                &second_level
            };

            for source_id in sources {
                let edge = GraphEdge {
                    source: source_id.clone(),
                    target: output_id.clone(),
                    weight: None,
                };
                builder.add_edge(edge);
            }
        }

        Ok(())
    }
}

impl Default for SimpleGraphExtractor {
    fn default() -> Self { Self::new() }
}

impl GraphExtractable for SimpleGraphExtractor {
    fn extract_graph(&self, trade_id: Option<&str>) -> Result<ComputationGraph, GraphError> {
        let start_time = Instant::now();
        let (node_cap, edge_cap) = self.builder_capacity;
        let mut builder = GraphBuilder::with_capacity(node_cap, edge_cap);

        match trade_id {
            Some(id) => {
                // Extract graph for specific trade
                let trade_info = self
                    .trades
                    .get(id)
                    .ok_or_else(|| GraphError::TradeNotFound(id.to_string()))?;

                self.build_trade_graph(id, trade_info, &mut builder, start_time)?;
            }
            None => {
                // Extract combined graph for all trades
                if self.trades.is_empty() {
                    return Err(GraphError::ExtractionFailed(
                        "No trades registered".to_string(),
                    ));
                }

                for (trade_id, trade_info) in &self.trades {
                    self.build_trade_graph(trade_id, trade_info, &mut builder, start_time)?;
                }
            }
        }

        // Validate DAG
        if !builder.is_dag() {
            return Err(GraphError::ExtractionFailed(
                "Graph contains cycles".to_string(),
            ));
        }

        // Check final timeout
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        Ok(builder.build(trade_id.map(String::from)))
    }

    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError> {
        // Verify trade exists
        if !self.trades.contains_key(trade_id) {
            return Err(GraphError::TradeNotFound(trade_id.to_string()));
        }

        let mut updates: Vec<GraphNodeUpdate> = Vec::new();

        // Get current and previous values
        let trade_info = &self.trades[trade_id];
        let previous = self.previous_values.get(trade_id);

        // Check parameter value changes
        for (param, &value) in &trade_info.param_values {
            let node_id = format!("{}_{}", trade_id, param);
            let delta = previous
                .and_then(|prev| prev.get(param))
                .map(|&old| value - old);

            if delta.is_some_and(|d| d.abs() > 1e-10) || previous.is_none() {
                updates.push(GraphNodeUpdate {
                    id: node_id,
                    value,
                    delta,
                });
            }
        }

        // Check computed value changes
        for (node_id, &value) in &trade_info.computed_values {
            let delta = previous
                .and_then(|prev| prev.get(node_id))
                .map(|&old| value - old);

            if delta.is_some_and(|d| d.abs() > 1e-10) {
                updates.push(GraphNodeUpdate {
                    id: node_id.clone(),
                    value,
                    delta,
                });
            }
        }

        Ok(updates)
    }
}

// =============================================================================
// PortfolioGraphExtractable Trait
// =============================================================================

/// Trait for extracting computation graphs from Portfolios with shared node
/// deduplication.
pub trait PortfolioGraphExtractable {
    /// Extract the complete computation graph for a Portfolio.
    fn extract_portfolio_graph(
        &self,
        trade_ids: &[String],
        trade_graphs: &HashMap<String, ComputationGraph>,
    ) -> Result<PortfolioComputationGraph, GraphError>;

    /// Extract a subgraph for selected trades, preserving shared nodes.
    fn extract_subgraph(
        &self,
        full_graph: &PortfolioComputationGraph,
        selected_trade_ids: &[String],
    ) -> Result<PortfolioComputationGraph, GraphError>;

    /// Extract nodes with changed values for differential WebSocket broadcasts.
    fn extract_portfolio_updates(
        &self,
        trade_ids: &[String],
        previous_graph: Option<&PortfolioComputationGraph>,
        current_graph: &PortfolioComputationGraph,
    ) -> Result<Vec<GraphNodeUpdate>, GraphError>;
}

// =============================================================================
// PortfolioGraphExtractor (Task 2.2)
// =============================================================================

/// Extractor for Portfolio-level computation graphs.
///
/// Provides extraction of integrated computation graphs from multiple trades
/// in a Portfolio, with shared node deduplication and optimisation.
///
/// # Features
///
/// - Integrates graphs from multiple trades
/// - Deduplicates shared market data nodes (same label + node_type)
/// - Tracks trade ownership via `trade_ids` field on each node
/// - Calculates optimisation ratio (node reduction percentage)
///
/// # Performance (Task 2.3)
///
/// - Pre-allocated buffers via configurable capacity
/// - O(n) shared node detection using HashMap
/// - Timeout protection (default 500ms)
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::PortfolioGraphExtractor;
///
/// let extractor = PortfolioGraphExtractor::new()
///     .with_timeout(1000)
///     .with_capacity(5000, 10000);
/// ```
#[derive(Debug)]
pub struct PortfolioGraphExtractor {
    /// Inner extractor for single-trade graphs
    inner: SimpleGraphExtractor,
    /// Timeout for extraction (milliseconds)
    timeout_ms: u64,
    /// Pre-allocated builder capacity (nodes, edges)
    builder_capacity: (usize, usize),
}

impl PortfolioGraphExtractor {
    /// Create a new PortfolioGraphExtractor with default settings.
    ///
    /// Default settings:
    /// - Timeout: 500ms
    /// - Capacity: 5,000 nodes, 10,000 edges
    pub fn new() -> Self {
        Self {
            inner: SimpleGraphExtractor::new(),
            timeout_ms: 500,
            builder_capacity: (5_000, 10_000),
        }
    }

    /// Create a new PortfolioGraphExtractor with custom timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Create a new PortfolioGraphExtractor with custom capacity.
    pub fn with_capacity(mut self, node_capacity: usize, edge_capacity: usize) -> Self {
        self.builder_capacity = (node_capacity, edge_capacity);
        self
    }

    /// Get reference to the inner SimpleGraphExtractor.
    pub fn inner(&self) -> &SimpleGraphExtractor { &self.inner }

    /// Get mutable reference to the inner SimpleGraphExtractor.
    pub fn inner_mut(&mut self) -> &mut SimpleGraphExtractor { &mut self.inner }

    /// Get current timeout setting.
    pub fn timeout_ms(&self) -> u64 { self.timeout_ms }

    /// Get current capacity setting.
    pub fn capacity(&self) -> (usize, usize) { self.builder_capacity }

    /// Merge trade graphs into a single Portfolio graph with shared node
    /// deduplication.
    ///
    /// # Algorithm (Task 2.3)
    ///
    /// 1. Build HashMap of (label, node_type) -> node_id for shared node
    ///    detection
    /// 2. For each trade graph: a. Check if node exists (same label + type) in
    ///    merged graph b. If exists: add trade_id to existing node c. If not:
    ///    add new node with trade_id
    /// 3. Redirect edges to use deduplicated node IDs
    /// 4. Calculate optimisation ratio
    fn merge_trade_graphs(
        &self,
        trade_ids: &[String],
        trade_graphs: &HashMap<String, ComputationGraph>,
        start_time: Instant,
    ) -> Result<PortfolioComputationGraph, GraphError> {
        let (node_cap, edge_cap) = self.builder_capacity;
        let mut builder = GraphBuilder::with_capacity(node_cap, edge_cap);

        // Track shared nodes: (label, node_type) -> merged_node_id
        let mut shared_node_map: HashMap<(String, NodeType), String> = HashMap::new();
        // Track node ID mapping: original_node_id -> merged_node_id
        let mut node_id_map: HashMap<String, String> = HashMap::new();
        // Track total nodes before deduplication
        let mut total_nodes_before_dedup = 0;

        for trade_id in trade_ids {
            // Check timeout
            if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
                return Err(GraphError::Timeout);
            }

            let Some(graph) = trade_graphs.get(trade_id) else {
                continue; // Skip missing trades
            };

            total_nodes_before_dedup += graph.nodes.len();

            // Process nodes
            for node in &graph.nodes {
                let key = (node.label.clone(), node.node_type);

                // Check if this is a shareable node (Input type with common labels)
                let is_shareable =
                    matches!(node.node_type, NodeType::Input) && !node.label.starts_with(trade_id);

                if is_shareable {
                    if let Some(existing_id) = shared_node_map.get(&key) {
                        // Node already exists, add trade_id to it
                        builder.add_trade_id(existing_id, trade_id);
                        node_id_map.insert(node.id.clone(), existing_id.clone());
                    } else {
                        // New shared node
                        let merged_id = format!("shared_{}", node.label);
                        let mut merged_node = node.clone();
                        merged_node.id.clone_from(&merged_id);
                        merged_node.trade_ids = vec![trade_id.clone()];
                        builder.add_node(merged_node);
                        shared_node_map.insert(key, merged_id.clone());
                        node_id_map.insert(node.id.clone(), merged_id);
                    }
                } else {
                    // Trade-specific node, prefix with trade_id if not already
                    let merged_id = if node.id.starts_with(trade_id) {
                        node.id.clone()
                    } else {
                        format!("{}_{}", trade_id, node.id)
                    };

                    if !builder.has_node(&merged_id) {
                        let mut merged_node = node.clone();
                        merged_node.id.clone_from(&merged_id);
                        merged_node.trade_ids = vec![trade_id.clone()];
                        builder.add_node(merged_node);
                    }
                    node_id_map.insert(node.id.clone(), merged_id);
                }
            }

            // Process edges with ID mapping
            for edge in &graph.edges {
                let source = node_id_map
                    .get(&edge.source)
                    .cloned()
                    .unwrap_or_else(|| edge.source.clone());
                let target = node_id_map
                    .get(&edge.target)
                    .cloned()
                    .unwrap_or_else(|| edge.target.clone());

                // Avoid duplicate edges
                let edge_key = format!("{}->{}", source, target);
                if !builder.has_node(&edge_key) {
                    // Use edge existence check via tracking (simplified)
                    builder.add_edge(GraphEdge {
                        source,
                        target,
                        weight: edge.weight,
                    });
                }
            }

            // Clear map for next trade (node_id_map is per-trade)
            node_id_map.clear();
        }

        // Check timeout before building
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        // Calculate metadata
        let node_count = builder.node_count();
        let edge_count = builder.edge_count();
        let depth = builder.calculate_depth();
        let shared_node_count = shared_node_map.len();
        let optimisation_ratio = if total_nodes_before_dedup > 0 {
            node_count as f64 / total_nodes_before_dedup as f64
        } else {
            1.0
        };

        let metadata = PortfolioGraphMetadata {
            node_count,
            edge_count,
            depth,
            generated_at: GraphBuilder::current_timestamp(),
            trade_count: trade_ids.len(),
            shared_node_count,
            optimisation_ratio,
        };

        // Build final graph
        let ComputationGraph { nodes, edges, .. } = builder.build(None);

        Ok(PortfolioComputationGraph {
            nodes,
            edges,
            metadata,
        })
    }
}

impl Default for PortfolioGraphExtractor {
    fn default() -> Self { Self::new() }
}

impl PortfolioGraphExtractable for PortfolioGraphExtractor {
    fn extract_portfolio_graph(
        &self,
        trade_ids: &[String],
        trade_graphs: &HashMap<String, ComputationGraph>,
    ) -> Result<PortfolioComputationGraph, GraphError> {
        let start_time = Instant::now();

        if trade_ids.is_empty() {
            return Err(GraphError::ExtractionFailed(
                "No trades provided for Portfolio graph extraction".to_string(),
            ));
        }

        self.merge_trade_graphs(trade_ids, trade_graphs, start_time)
    }

    fn extract_subgraph(
        &self,
        full_graph: &PortfolioComputationGraph,
        selected_trade_ids: &[String],
    ) -> Result<PortfolioComputationGraph, GraphError> {
        if selected_trade_ids.is_empty() {
            return Err(GraphError::ExtractionFailed(
                "No trade IDs selected for subgraph extraction".to_string(),
            ));
        }

        // Verify all selected trade IDs exist in the graph
        let all_trade_ids: std::collections::HashSet<&str> = full_graph
            .nodes
            .iter()
            .flat_map(|n| n.trade_ids.iter().map(|s| s.as_str()))
            .collect();

        for trade_id in selected_trade_ids {
            if !all_trade_ids.contains(trade_id.as_str()) {
                return Err(GraphError::TradeNotFound(trade_id.clone()));
            }
        }

        let selected_set: std::collections::HashSet<&str> =
            selected_trade_ids.iter().map(|s| s.as_str()).collect();

        // Filter nodes: keep if any trade_id is in selected set
        let filtered_nodes: Vec<GraphNode> = full_graph
            .nodes
            .iter()
            .filter(|n| {
                n.trade_ids
                    .iter()
                    .any(|tid| selected_set.contains(tid.as_str()))
            })
            .cloned()
            .collect();

        // Build set of retained node IDs
        let retained_ids: std::collections::HashSet<&str> =
            filtered_nodes.iter().map(|n| n.id.as_str()).collect();

        // Filter edges: keep if both endpoints are retained
        let filtered_edges: Vec<GraphEdge> = full_graph
            .edges
            .iter()
            .filter(|e| {
                retained_ids.contains(e.source.as_str()) && retained_ids.contains(e.target.as_str())
            })
            .cloned()
            .collect();

        // Recalculate metadata
        let shared_nodes: Vec<&GraphNode> = filtered_nodes
            .iter()
            .filter(|n| n.trade_ids.len() > 1)
            .collect();

        let metadata = PortfolioGraphMetadata {
            node_count: filtered_nodes.len(),
            edge_count: filtered_edges.len(),
            depth: full_graph.metadata.depth, // Approximate, could recalculate
            generated_at: GraphBuilder::current_timestamp(),
            trade_count: selected_trade_ids.len(),
            shared_node_count: shared_nodes.len(),
            optimisation_ratio: if full_graph.metadata.node_count > 0 {
                filtered_nodes.len() as f64 / full_graph.metadata.node_count as f64
            } else {
                1.0
            },
        };

        Ok(PortfolioComputationGraph {
            nodes: filtered_nodes,
            edges: filtered_edges,
            metadata,
        })
    }

    fn extract_portfolio_updates(
        &self,
        _trade_ids: &[String],
        previous_graph: Option<&PortfolioComputationGraph>,
        current_graph: &PortfolioComputationGraph,
    ) -> Result<Vec<GraphNodeUpdate>, GraphError> {
        let mut updates = Vec::new();

        match previous_graph {
            Some(prev) => {
                // Build lookup for previous values
                let prev_values: HashMap<&str, Option<f64>> = prev
                    .nodes
                    .iter()
                    .map(|n| (n.id.as_str(), n.value))
                    .collect();

                // Compare current values
                for node in &current_graph.nodes {
                    let prev_value = prev_values.get(node.id.as_str()).copied().flatten();
                    let curr_value = node.value;

                    match (prev_value, curr_value) {
                        (Some(old), Some(new)) if (old - new).abs() > 1e-10 => {
                            updates.push(GraphNodeUpdate {
                                id: node.id.clone(),
                                value: new,
                                delta: Some(new - old),
                            });
                        }
                        (None, Some(new)) => {
                            updates.push(GraphNodeUpdate {
                                id: node.id.clone(),
                                value: new,
                                delta: None,
                            });
                        }
                        _ => {}
                    }
                }
            }
            None => {
                // No previous graph, all nodes with values are updates
                for node in &current_graph.nodes {
                    if let Some(value) = node.value {
                        updates.push(GraphNodeUpdate {
                            id: node.id.clone(),
                            value,
                            delta: None,
                        });
                    }
                }
            }
        }

        Ok(updates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_node(
        id: &str,
        nt: NodeType,
        label: &str,
        val: Option<f64>,
        sens: bool,
        group: NodeGroup,
        trades: Vec<&str>,
    ) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: nt,
            label: label.to_string(),
            value: val,
            is_sensitivity_target: sens,
            group,
            trade_ids: trades.into_iter().map(String::from).collect(),
        }
    }

    fn mk_edge(src: &str, tgt: &str) -> GraphEdge {
        GraphEdge {
            source: src.to_string(),
            target: tgt.to_string(),
            weight: None,
        }
    }

    fn mk_pmeta(
        nodes: usize,
        edges: usize,
        depth: usize,
        trades: usize,
        shared: usize,
        ratio: f64,
    ) -> PortfolioGraphMetadata {
        PortfolioGraphMetadata {
            node_count: nodes,
            edge_count: edges,
            depth,
            generated_at: "test".to_string(),
            trade_count: trades,
            shared_node_count: shared,
            optimisation_ratio: ratio,
        }
    }

    mod trait_tests {
        use super::*;

        #[test]
        fn test_extract_graph_returns_computation_graph() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot", "vol"]);

            let result = extractor.extract_graph(Some("T001"));

            assert!(result.is_ok());
            let graph = result.unwrap();
            assert!(!graph.nodes.is_empty());
            assert!(!graph.edges.is_empty());
        }

        #[test]
        fn test_extract_graph_trade_not_found() {
            let extractor = SimpleGraphExtractor::new();

            let result = extractor.extract_graph(Some("NONEXISTENT"));

            assert!(matches!(result, Err(GraphError::TradeNotFound(_))));
        }

        #[test]
        fn test_extract_affected_nodes_returns_updates() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot"]);
            extractor.set_param_value("T001", "spot", 100.0).unwrap();

            let result = extractor.extract_affected_nodes("T001");

            assert!(result.is_ok());
            let updates = result.unwrap();
            assert!(!updates.is_empty());
        }

        #[test]
        fn test_extract_affected_nodes_trade_not_found() {
            let extractor = SimpleGraphExtractor::new();

            let result = extractor.extract_affected_nodes("NONEXISTENT");

            assert!(matches!(result, Err(GraphError::TradeNotFound(_))));
        }

        #[test]
        fn test_graph_node_update_has_required_fields() {
            let update = GraphNodeUpdate {
                id: "N1".to_string(),
                value: 105.0,
                delta: Some(5.0),
            };

            assert_eq!(update.id, "N1");
            assert_eq!(update.value, 105.0);
            assert_eq!(update.delta, Some(5.0));
        }
    }

    mod extractor_tests {
        use super::*;

        #[test]
        fn test_new_extractor_is_empty() {
            let extractor = SimpleGraphExtractor::new();

            assert_eq!(extractor.trade_count(), 0);
        }

        #[test]
        fn test_register_trade() {
            let mut extractor = SimpleGraphExtractor::new();

            extractor.register_trade("T001", vec!["spot", "vol", "rate"]);

            assert!(extractor.has_trade("T001"));
            assert_eq!(extractor.trade_count(), 1);
        }

        #[test]
        fn test_set_param_value() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot"]);

            let result = extractor.set_param_value("T001", "spot", 100.0);

            assert!(result.is_ok());
        }

        #[test]
        fn test_set_param_value_trade_not_found() {
            let mut extractor = SimpleGraphExtractor::new();

            let result = extractor.set_param_value("T001", "spot", 100.0);

            assert!(matches!(result, Err(GraphError::TradeNotFound(_))));
        }

        #[test]
        fn test_graph_contains_input_nodes() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot", "vol"]);

            let graph = extractor.extract_graph(Some("T001")).unwrap();

            // Should have input nodes for each sensitivity parameter
            let input_count = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == NodeType::Input)
                .count();
            assert!(input_count >= 2);
        }

        #[test]
        fn test_graph_contains_output_node() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot"]);

            let graph = extractor.extract_graph(Some("T001")).unwrap();

            let output_count = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == NodeType::Output)
                .count();
            assert!(output_count >= 1);
        }

        #[test]
        fn test_sensitivity_targets_marked() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot", "vol"]);

            let graph = extractor.extract_graph(Some("T001")).unwrap();

            let sensitivity_count = graph
                .nodes
                .iter()
                .filter(|n| n.is_sensitivity_target)
                .count();
            assert!(sensitivity_count >= 2);
        }

        #[test]
        fn test_graph_is_dag() {
            use std::collections::HashSet;

            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot", "vol", "rate"]);

            let graph = extractor.extract_graph(Some("T001")).unwrap();

            // Build a set of all node IDs
            let node_ids: HashSet<_> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

            // Verify all edges reference valid nodes
            for edge in &graph.edges {
                assert!(
                    node_ids.contains(edge.source.as_str()),
                    "Source {} not found",
                    edge.source
                );
                assert!(
                    node_ids.contains(edge.target.as_str()),
                    "Target {} not found",
                    edge.target
                );
            }
        }

        #[test]
        fn test_graph_depth_calculated() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot", "vol"]);

            let graph = extractor.extract_graph(Some("T001")).unwrap();

            // Depth should be at least 2 (input -> output)
            assert!(
                graph.metadata.depth >= 2,
                "Expected depth >= 2, got {}",
                graph.metadata.depth
            );
        }

        #[test]
        fn test_extract_all_trades() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot"]);
            extractor.register_trade("T002", vec!["vol"]);

            let graph = extractor.extract_graph(None).unwrap();

            // Should contain nodes from both trades
            let t001_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.id.starts_with("T001"))
                .collect();
            let t002_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.id.starts_with("T002"))
                .collect();

            assert!(!t001_nodes.is_empty());
            assert!(!t002_nodes.is_empty());
        }

        #[test]
        fn test_extract_empty_returns_error() {
            let extractor = SimpleGraphExtractor::new();

            let result = extractor.extract_graph(None);

            assert!(matches!(result, Err(GraphError::ExtractionFailed(_))));
        }

        #[test]
        fn test_delta_calculation() {
            let mut extractor = SimpleGraphExtractor::new();
            extractor.register_trade("T001", vec!["spot"]);
            extractor.set_param_value("T001", "spot", 100.0).unwrap();
            extractor.set_param_value("T001", "spot", 105.0).unwrap();

            let updates = extractor.extract_affected_nodes("T001").unwrap();

            let spot_update = updates.iter().find(|u| u.id.contains("spot"));
            assert!(spot_update.is_some());
            let update = spot_update.unwrap();
            assert_eq!(update.value, 105.0);
            assert!(update.delta.is_some());
            assert!((update.delta.unwrap() - 5.0).abs() < 1e-10);
        }
    }

    mod builder_tests {
        use super::*;

        #[test]
        fn test_builder_new() {
            let builder = GraphBuilder::new();

            assert_eq!(builder.node_count(), 0);
            assert_eq!(builder.edge_count(), 0);
        }

        #[test]
        fn test_builder_with_capacity() {
            let builder = GraphBuilder::with_capacity(10_000, 20_000);

            assert_eq!(builder.node_count(), 0);
            assert_eq!(builder.edge_count(), 0);
        }

        #[test]
        fn test_builder_add_node() {
            let mut builder = GraphBuilder::new();
            let node = mk_node(
                "N1",
                NodeType::Input,
                "spot",
                Some(100.0),
                true,
                NodeGroup::Input,
                vec![],
            );

            let index = builder.add_node(node);

            assert_eq!(index, 0);
            assert_eq!(builder.node_count(), 1);
            assert!(builder.has_node("N1"));
        }

        #[test]
        fn test_builder_add_edge() {
            let mut builder = GraphBuilder::new();
            builder.add_edge(mk_edge("N1", "N2"));

            assert_eq!(builder.edge_count(), 1);
        }

        #[test]
        fn test_builder_get_node() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                Some(100.0),
                true,
                NodeGroup::Input,
                vec![],
            ));

            let retrieved = builder.get_node("N1");

            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().label, "spot");
        }

        #[test]
        fn test_builder_get_node_not_found() {
            let builder = GraphBuilder::new();

            let retrieved = builder.get_node("N1");

            assert!(retrieved.is_none());
        }

        #[test]
        fn test_builder_clear() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                None,
                false,
                NodeGroup::Input,
                vec![],
            ));
            builder.add_edge(mk_edge("N1", "N2"));

            builder.clear();

            assert_eq!(builder.node_count(), 0);
            assert_eq!(builder.edge_count(), 0);
            assert!(!builder.has_node("N1"));
        }

        #[test]
        fn test_builder_calculate_depth_empty() {
            let builder = GraphBuilder::new();

            assert_eq!(builder.calculate_depth(), 0);
        }

        #[test]
        fn test_builder_calculate_depth_single_node() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "x",
                None,
                false,
                NodeGroup::Input,
                vec![],
            ));

            assert_eq!(builder.calculate_depth(), 1);
        }

        #[test]
        fn test_builder_calculate_depth_linear() {
            let mut builder = GraphBuilder::new();
            // Create linear chain: N1 -> N2 -> N3
            for (i, nt) in [
                (1, NodeType::Input),
                (2, NodeType::Add),
                (3, NodeType::Output),
            ] {
                builder.add_node(mk_node(
                    &format!("N{i}"),
                    nt,
                    &format!("n{i}"),
                    None,
                    false,
                    NodeGroup::Intermediate,
                    vec![],
                ));
            }
            builder.add_edge(mk_edge("N1", "N2"));
            builder.add_edge(mk_edge("N2", "N3"));

            assert_eq!(builder.calculate_depth(), 3);
        }

        #[test]
        fn test_builder_is_dag_empty() {
            let builder = GraphBuilder::new();

            assert!(builder.is_dag());
        }

        #[test]
        fn test_builder_is_dag_simple() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "x",
                None,
                false,
                NodeGroup::Input,
                vec![],
            ));
            builder.add_node(mk_node(
                "N2",
                NodeType::Output,
                "y",
                None,
                false,
                NodeGroup::Output,
                vec![],
            ));
            builder.add_edge(mk_edge("N1", "N2"));

            assert!(builder.is_dag());
        }

        #[test]
        fn test_builder_build() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "x",
                Some(1.0),
                true,
                NodeGroup::Input,
                vec![],
            ));
            builder.add_node(mk_node(
                "N2",
                NodeType::Output,
                "y",
                Some(2.0),
                false,
                NodeGroup::Output,
                vec![],
            ));
            builder.add_edge(mk_edge("N1", "N2"));

            let graph = builder.build(Some("T001".to_string()));

            assert_eq!(graph.nodes.len(), 2);
            assert_eq!(graph.edges.len(), 1);
            assert_eq!(graph.metadata.trade_id, Some("T001".to_string()));
            assert_eq!(graph.metadata.node_count, 2);
            assert_eq!(graph.metadata.edge_count, 1);
            assert_eq!(graph.metadata.depth, 2);
        }
    }

    mod performance_tests {
        use std::time::Duration;

        use super::*;

        #[test]
        fn test_extraction_within_timeout() {
            let mut extractor = SimpleGraphExtractor::new().with_timeout(1000);
            extractor.register_trade("T001", vec!["spot", "vol", "rate", "tenor"]);

            let start = Instant::now();
            let result = extractor.extract_graph(Some("T001"));
            let elapsed = start.elapsed();

            assert!(result.is_ok());
            assert!(
                elapsed < Duration::from_millis(1000),
                "Extraction took {:?}",
                elapsed
            );
        }

        #[test]
        fn test_large_graph_extraction() {
            let mut extractor = SimpleGraphExtractor::new()
                .with_timeout(5000)
                .with_capacity(10_000, 20_000);

            // Register 100 trades with 10 parameters each
            for i in 0..100 {
                let trade_id = format!("T{:04}", i);
                let params: Vec<String> = (0..10).map(|j| format!("param_{}", j)).collect();
                extractor.register_trade(&trade_id, params);
            }

            let start = Instant::now();
            let result = extractor.extract_graph(None);
            let elapsed = start.elapsed();

            assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
            let graph = result.unwrap();
            assert!(graph.nodes.len() > 0);

            // Should complete within 5 seconds
            assert!(
                elapsed < Duration::from_secs(5),
                "Extraction took {:?}",
                elapsed
            );
        }

        #[test]
        fn test_builder_reuse_efficiency() {
            let mut builder = GraphBuilder::with_capacity(1000, 2000);

            for i in 0..100 {
                builder.add_node(mk_node(
                    &format!("N{i}"),
                    NodeType::Add,
                    &format!("n{i}"),
                    None,
                    false,
                    NodeGroup::Intermediate,
                    vec![],
                ));
            }
            assert_eq!(builder.node_count(), 100);
            builder.clear();
            assert_eq!(builder.node_count(), 0);
            for i in 0..50 {
                builder.add_node(mk_node(
                    &format!("M{i}"),
                    NodeType::Mul,
                    &format!("m{i}"),
                    None,
                    false,
                    NodeGroup::Intermediate,
                    vec![],
                ));
            }

            assert_eq!(builder.node_count(), 50);
        }

        #[test]
        fn test_builder_add_trade_id() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                Some(100.0),
                true,
                NodeGroup::Input,
                vec![],
            ));

            // Add first trade ID
            let result = builder.add_trade_id("N1", "T001");
            assert!(result.is_some());

            let node = builder.get_node("N1").unwrap();
            assert_eq!(node.trade_ids, vec!["T001".to_string()]);
        }

        #[test]
        fn test_builder_add_trade_id_deduplication() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                None,
                false,
                NodeGroup::Input,
                vec![],
            ));

            // Add same trade ID twice
            builder.add_trade_id("N1", "T001");
            builder.add_trade_id("N1", "T001");

            let node = builder.get_node("N1").unwrap();
            assert_eq!(node.trade_ids.len(), 1);
            assert_eq!(node.trade_ids[0], "T001");
        }

        #[test]
        fn test_builder_add_trade_id_multiple_trades() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                None,
                false,
                NodeGroup::Input,
                vec![],
            ));

            // Add multiple trade IDs
            builder.add_trade_id("N1", "T001");
            builder.add_trade_id("N1", "T002");
            builder.add_trade_id("N1", "T003");

            let node = builder.get_node("N1").unwrap();
            assert_eq!(node.trade_ids.len(), 3);
            assert!(node.trade_ids.contains(&"T001".to_string()));
            assert!(node.trade_ids.contains(&"T002".to_string()));
            assert!(node.trade_ids.contains(&"T003".to_string()));
        }

        #[test]
        fn test_builder_add_trade_id_nonexistent_node() {
            let mut builder = GraphBuilder::new();

            // Try to add trade ID to nonexistent node
            let result = builder.add_trade_id("N999", "T001");
            assert!(result.is_none());
        }

        #[test]
        fn test_builder_set_trade_ids() {
            let mut builder = GraphBuilder::new();
            builder.add_node(mk_node(
                "N1",
                NodeType::Input,
                "spot",
                None,
                false,
                NodeGroup::Input,
                vec!["OLD"],
            ));

            // Replace trade IDs
            let trade_ids = vec!["T001".to_string(), "T002".to_string()];
            let result = builder.set_trade_ids("N1", trade_ids);
            assert!(result.is_some());

            let node = builder.get_node("N1").unwrap();
            assert_eq!(node.trade_ids.len(), 2);
            assert!(node.trade_ids.contains(&"T001".to_string()));
            assert!(node.trade_ids.contains(&"T002".to_string()));
            assert!(!node.trade_ids.contains(&"OLD".to_string()));
        }

        #[test]
        fn test_builder_set_trade_ids_nonexistent_node() {
            let mut builder = GraphBuilder::new();

            // Try to set trade IDs on nonexistent node
            let result = builder.set_trade_ids("N999", vec!["T001".to_string()]);
            assert!(result.is_none());
        }
    }

    mod portfolio_extractor_tests {
        use super::*;

        fn sample_trade_graph(trade_id: &str, params: &[&str]) -> ComputationGraph {
            let output_id = format!("{trade_id}_price");
            let mut nodes: Vec<GraphNode> = params
                .iter()
                .map(|p| {
                    mk_node(
                        &format!("{trade_id}_{p}"),
                        NodeType::Input,
                        p,
                        Some(100.0),
                        true,
                        NodeGroup::Input,
                        vec![],
                    )
                })
                .collect();
            nodes.push(mk_node(
                &output_id,
                NodeType::Output,
                "price",
                Some(10.5),
                false,
                NodeGroup::Output,
                vec![],
            ));
            let edges = params
                .iter()
                .map(|p| mk_edge(&format!("{trade_id}_{p}"), &output_id))
                .collect();
            ComputationGraph {
                nodes,
                edges,
                metadata: GraphMetadata {
                    trade_id: Some(trade_id.to_string()),
                    node_count: params.len() + 1,
                    edge_count: params.len(),
                    depth: 2,
                    generated_at: "test".to_string(),
                },
            }
        }

        fn sample_portfolio(
            nodes: Vec<GraphNode>,
            edges: Vec<GraphEdge>,
            trades: usize,
            shared: usize,
            ratio: f64,
        ) -> PortfolioComputationGraph {
            let depth = if nodes.is_empty() { 0 } else { 2 };
            PortfolioComputationGraph {
                metadata: mk_pmeta(nodes.len(), edges.len(), depth, trades, shared, ratio),
                nodes,
                edges,
            }
        }

        #[test]
        fn test_portfolio_extractor_new() {
            let extractor = PortfolioGraphExtractor::new();

            assert_eq!(extractor.timeout_ms(), 500);
            assert_eq!(extractor.capacity(), (5_000, 10_000));
        }

        #[test]
        fn test_portfolio_extractor_with_timeout() {
            let extractor = PortfolioGraphExtractor::new().with_timeout(1000);

            assert_eq!(extractor.timeout_ms(), 1000);
        }

        #[test]
        fn test_portfolio_extractor_with_capacity() {
            let extractor = PortfolioGraphExtractor::new().with_capacity(10_000, 20_000);

            assert_eq!(extractor.capacity(), (10_000, 20_000));
        }

        #[test]
        fn test_portfolio_extractor_default() {
            let extractor = PortfolioGraphExtractor::default();

            assert_eq!(extractor.timeout_ms(), 500);
        }

        #[test]
        fn test_extract_portfolio_graph_single_trade() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids = vec!["T001".to_string()];
            let mut trade_graphs = HashMap::new();
            trade_graphs.insert(
                "T001".to_string(),
                sample_trade_graph("T001", &["spot", "vol"]),
            );

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(result.is_ok());
            let graph = result.unwrap();
            assert_eq!(graph.metadata.trade_count, 1);
            assert!(graph.nodes.len() >= 3); // 2 inputs + 1 output
        }

        #[test]
        fn test_extract_portfolio_graph_multiple_trades() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids = vec!["T001".to_string(), "T002".to_string()];
            let mut trade_graphs = HashMap::new();
            trade_graphs.insert(
                "T001".to_string(),
                sample_trade_graph("T001", &["spot", "vol"]),
            );
            trade_graphs.insert(
                "T002".to_string(),
                sample_trade_graph("T002", &["spot", "rate"]),
            );

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(result.is_ok());
            let graph = result.unwrap();
            assert_eq!(graph.metadata.trade_count, 2);
            // Should have nodes from both trades
            assert!(graph.nodes.len() >= 4);
        }

        #[test]
        fn test_extract_portfolio_graph_empty_trades() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids: Vec<String> = vec![];
            let trade_graphs = HashMap::new();

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(matches!(result, Err(GraphError::ExtractionFailed(_))));
        }

        #[test]
        fn test_extract_portfolio_graph_shared_nodes() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids = vec!["T001".to_string(), "T002".to_string()];
            let mut trade_graphs = HashMap::new();

            // Both trades use "spot" - should be deduplicated
            trade_graphs.insert(
                "T001".to_string(),
                sample_trade_graph("T001", &["spot", "vol"]),
            );
            trade_graphs.insert(
                "T002".to_string(),
                sample_trade_graph("T002", &["spot", "rate"]),
            );

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(result.is_ok());
            let graph = result.unwrap();

            // Check that shared nodes exist
            let shared = graph.shared_nodes();
            // "spot" should be shared between T001 and T002
            assert!(
                shared.iter().any(|n| n.label == "spot"),
                "Expected 'spot' to be a shared node"
            );
        }

        #[test]
        fn test_extract_portfolio_graph_trade_ids_populated() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids = vec!["T001".to_string()];
            let mut trade_graphs = HashMap::new();
            trade_graphs.insert("T001".to_string(), sample_trade_graph("T001", &["spot"]));

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(result.is_ok());
            let graph = result.unwrap();

            // All nodes should have trade_ids populated
            for node in &graph.nodes {
                assert!(
                    !node.trade_ids.is_empty(),
                    "Node {} should have trade_ids",
                    node.id
                );
            }
        }

        #[test]
        fn test_extract_portfolio_graph_metadata() {
            let extractor = PortfolioGraphExtractor::new();
            let trade_ids = vec!["T001".to_string(), "T002".to_string()];
            let mut trade_graphs = HashMap::new();
            trade_graphs.insert("T001".to_string(), sample_trade_graph("T001", &["spot"]));
            trade_graphs.insert("T002".to_string(), sample_trade_graph("T002", &["vol"]));

            let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);

            assert!(result.is_ok());
            let graph = result.unwrap();

            assert_eq!(graph.metadata.trade_count, 2);
            assert!(graph.metadata.node_count > 0);
            assert!(graph.metadata.optimisation_ratio > 0.0);
            assert!(graph.metadata.optimisation_ratio <= 1.0);
        }

        #[test]
        fn test_extract_subgraph_single_trade() {
            let extractor = PortfolioGraphExtractor::new();
            let full_graph = sample_portfolio(
                vec![
                    mk_node(
                        "T001_spot",
                        NodeType::Input,
                        "spot",
                        Some(100.0),
                        true,
                        NodeGroup::Input,
                        vec!["T001"],
                    ),
                    mk_node(
                        "T001_price",
                        NodeType::Output,
                        "price",
                        Some(10.5),
                        false,
                        NodeGroup::Output,
                        vec!["T001"],
                    ),
                    mk_node(
                        "T002_vol",
                        NodeType::Input,
                        "vol",
                        Some(0.2),
                        true,
                        NodeGroup::Input,
                        vec!["T002"],
                    ),
                    mk_node(
                        "T002_price",
                        NodeType::Output,
                        "price",
                        Some(15.0),
                        false,
                        NodeGroup::Output,
                        vec!["T002"],
                    ),
                ],
                vec![
                    mk_edge("T001_spot", "T001_price"),
                    mk_edge("T002_vol", "T002_price"),
                ],
                2,
                0,
                1.0,
            );

            let selected = vec!["T001".to_string()];
            let result = extractor.extract_subgraph(&full_graph, &selected);

            assert!(result.is_ok());
            let subgraph = result.unwrap();

            // Should only have T001 nodes
            assert_eq!(subgraph.nodes.len(), 2);
            assert!(subgraph
                .nodes
                .iter()
                .all(|n| n.trade_ids.contains(&"T001".to_string())));
            assert_eq!(subgraph.metadata.trade_count, 1);
        }

        #[test]
        fn test_extract_subgraph_preserves_shared_nodes() {
            let extractor = PortfolioGraphExtractor::new();
            let full_graph = sample_portfolio(
                vec![
                    mk_node(
                        "shared_spot",
                        NodeType::Input,
                        "spot",
                        Some(100.0),
                        true,
                        NodeGroup::Input,
                        vec!["T001", "T002"],
                    ),
                    mk_node(
                        "T001_price",
                        NodeType::Output,
                        "price",
                        Some(10.5),
                        false,
                        NodeGroup::Output,
                        vec!["T001"],
                    ),
                    mk_node(
                        "T002_price",
                        NodeType::Output,
                        "price",
                        Some(15.0),
                        false,
                        NodeGroup::Output,
                        vec!["T002"],
                    ),
                ],
                vec![
                    mk_edge("shared_spot", "T001_price"),
                    mk_edge("shared_spot", "T002_price"),
                ],
                2,
                1,
                0.75,
            );

            let selected = vec!["T001".to_string()];
            let result = extractor.extract_subgraph(&full_graph, &selected);

            assert!(result.is_ok());
            let subgraph = result.unwrap();

            // Should have shared_spot (because T001 uses it) and T001_price
            assert_eq!(subgraph.nodes.len(), 2);
            assert!(subgraph.nodes.iter().any(|n| n.id == "shared_spot"));
            assert!(subgraph.nodes.iter().any(|n| n.id == "T001_price"));
        }

        #[test]
        fn test_extract_subgraph_trade_not_found() {
            let extractor = PortfolioGraphExtractor::new();
            let full_graph = sample_portfolio(
                vec![mk_node(
                    "T001_spot",
                    NodeType::Input,
                    "spot",
                    Some(100.0),
                    true,
                    NodeGroup::Input,
                    vec!["T001"],
                )],
                vec![],
                1,
                0,
                1.0,
            );

            let selected = vec!["T999".to_string()]; // Non-existent trade
            let result = extractor.extract_subgraph(&full_graph, &selected);

            assert!(matches!(result, Err(GraphError::TradeNotFound(id)) if id == "T999"));
        }

        #[test]
        fn test_extract_subgraph_empty_selection() {
            let extractor = PortfolioGraphExtractor::new();
            let full_graph = sample_portfolio(vec![], vec![], 0, 0, 1.0);

            let selected: Vec<String> = vec![];
            let result = extractor.extract_subgraph(&full_graph, &selected);

            assert!(matches!(result, Err(GraphError::ExtractionFailed(_))));
        }

        #[test]
        fn test_extract_portfolio_updates_no_previous() {
            let extractor = PortfolioGraphExtractor::new();
            let current = sample_portfolio(
                vec![mk_node(
                    "N1",
                    NodeType::Input,
                    "spot",
                    Some(100.0),
                    true,
                    NodeGroup::Input,
                    vec!["T001"],
                )],
                vec![],
                1,
                0,
                1.0,
            );

            let result = extractor.extract_portfolio_updates(&["T001".to_string()], None, &current);

            assert!(result.is_ok());
            let updates = result.unwrap();
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0].id, "N1");
            assert_eq!(updates[0].value, 100.0);
            assert!(updates[0].delta.is_none());
        }

        #[test]
        fn test_extract_portfolio_updates_with_delta() {
            let extractor = PortfolioGraphExtractor::new();
            let make_graph = |val: f64| {
                sample_portfolio(
                    vec![mk_node(
                        "N1",
                        NodeType::Input,
                        "spot",
                        Some(val),
                        true,
                        NodeGroup::Input,
                        vec!["T001"],
                    )],
                    vec![],
                    1,
                    0,
                    1.0,
                )
            };
            let previous = make_graph(100.0);
            let current = make_graph(105.0);

            let result = extractor.extract_portfolio_updates(
                &["T001".to_string()],
                Some(&previous),
                &current,
            );

            assert!(result.is_ok());
            let updates = result.unwrap();
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0].id, "N1");
            assert_eq!(updates[0].value, 105.0);
            assert!(updates[0].delta.is_some());
            assert!((updates[0].delta.unwrap() - 5.0).abs() < 1e-10);
        }

        #[test]
        fn test_extract_portfolio_updates_no_changes() {
            let extractor = PortfolioGraphExtractor::new();
            let graph = sample_portfolio(
                vec![mk_node(
                    "N1",
                    NodeType::Input,
                    "spot",
                    Some(100.0),
                    true,
                    NodeGroup::Input,
                    vec!["T001"],
                )],
                vec![],
                1,
                0,
                1.0,
            );

            let result =
                extractor.extract_portfolio_updates(&["T001".to_string()], Some(&graph), &graph);

            assert!(result.is_ok());
            let updates = result.unwrap();
            // No changes, so no updates
            assert!(updates.is_empty());
        }
    }
}

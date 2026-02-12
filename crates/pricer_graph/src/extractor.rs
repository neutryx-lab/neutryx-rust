//! # Graph Extractor Trait and Implementation

use std::{collections::HashMap, time::Instant};

use petgraph::{
    algo::{is_cyclic_directed, toposort},
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    error::GraphError,
    types::{
        ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup,
        NodeType, PortfolioComputationGraph, PortfolioGraphMetadata,
    },
};

/// Trait for extracting computation graphs from pricing contexts.
pub trait GraphExtractable {
    /// Extract the computation graph for a specific trade (or all if `None`).
    fn extract_graph(&self, trade_id: Option<&str>) -> Result<ComputationGraph, GraphError>;

    /// Extract nodes affected by recent updates (for differential WebSocket
    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError>;
}

/// Pre-allocated buffer builder for graph construction.
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
    pub fn clear(&mut self) {
        self.digraph.clear();
        self.edges.clear();
        self.node_index.clear();
    }

    /// Calculate the graph depth (longest path from any input to any output).
    pub fn calculate_depth(&self) -> usize {
        if self.digraph.node_count() == 0 {
            return 0;
        }

        let sorted = match toposort(&self.digraph, None) {
            Ok(order) => order,
            Err(_) => return 0,
        };

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

        max_depth + 1
    }

    /// Validate that the graph is a DAG (no cycles).
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", now.as_secs())
    }
}

impl Default for GraphBuilder {
    fn default() -> Self { Self::new() }
}

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
    fn build_trade_graph(
        &self,
        trade_id: &str,
        trade_info: &TradeGraphInfo,
        builder: &mut GraphBuilder,
        start_time: Instant,
    ) -> Result<(), GraphError> {
        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        let params = &trade_info.sensitivity_params;
        let param_values = &trade_info.param_values;
        let computed_values = &trade_info.computed_values;

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

        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        let mut intermediate_nodes: Vec<String> = Vec::new();

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

        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

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
                let trade_info = self
                    .trades
                    .get(id)
                    .ok_or_else(|| GraphError::TradeNotFound(id.to_string()))?;

                self.build_trade_graph(id, trade_info, &mut builder, start_time)?;
            }
            None => {
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

        if !builder.is_dag() {
            return Err(GraphError::ExtractionFailed(
                "Graph contains cycles".to_string(),
            ));
        }

        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

        Ok(builder.build(trade_id.map(String::from)))
    }

    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError> {
        if !self.trades.contains_key(trade_id) {
            return Err(GraphError::TradeNotFound(trade_id.to_string()));
        }

        let mut updates: Vec<GraphNodeUpdate> = Vec::new();

        let trade_info = &self.trades[trade_id];
        let previous = self.previous_values.get(trade_id);

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

/// Trait for extracting computation graphs from Portfolios with shared node
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

/// Extractor for Portfolio-level computation graphs.
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
    fn merge_trade_graphs(
        &self,
        trade_ids: &[String],
        trade_graphs: &HashMap<String, ComputationGraph>,
        start_time: Instant,
    ) -> Result<PortfolioComputationGraph, GraphError> {
        let (node_cap, edge_cap) = self.builder_capacity;
        let mut builder = GraphBuilder::with_capacity(node_cap, edge_cap);

        let mut shared_node_map: HashMap<(String, NodeType), String> = HashMap::new();
        let mut node_id_map: HashMap<String, String> = HashMap::new();
        let mut total_nodes_before_dedup = 0;

        for trade_id in trade_ids {
            if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
                return Err(GraphError::Timeout);
            }

            let Some(graph) = trade_graphs.get(trade_id) else {
                continue;
            };

            total_nodes_before_dedup += graph.nodes.len();

            for node in &graph.nodes {
                let key = (node.label.clone(), node.node_type);

                let is_shareable =
                    matches!(node.node_type, NodeType::Input) && !node.label.starts_with(trade_id);

                if is_shareable {
                    if let Some(existing_id) = shared_node_map.get(&key) {
                        builder.add_trade_id(existing_id, trade_id);
                        node_id_map.insert(node.id.clone(), existing_id.clone());
                    } else {
                        let merged_id = format!("shared_{}", node.label);
                        let mut merged_node = node.clone();
                        merged_node.id.clone_from(&merged_id);
                        merged_node.trade_ids = vec![trade_id.clone()];
                        builder.add_node(merged_node);
                        shared_node_map.insert(key, merged_id.clone());
                        node_id_map.insert(node.id.clone(), merged_id);
                    }
                } else {
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

            for edge in &graph.edges {
                let source = node_id_map
                    .get(&edge.source)
                    .cloned()
                    .unwrap_or_else(|| edge.source.clone());
                let target = node_id_map
                    .get(&edge.target)
                    .cloned()
                    .unwrap_or_else(|| edge.target.clone());

                let edge_key = format!("{}->{}", source, target);
                if !builder.has_node(&edge_key) {
                    builder.add_edge(GraphEdge {
                        source,
                        target,
                        weight: edge.weight,
                    });
                }
            }

            node_id_map.clear();
        }

        if start_time.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(GraphError::Timeout);
        }

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

        let retained_ids: std::collections::HashSet<&str> =
            filtered_nodes.iter().map(|n| n.id.as_str()).collect();

        let filtered_edges: Vec<GraphEdge> = full_graph
            .edges
            .iter()
            .filter(|e| {
                retained_ids.contains(e.source.as_str()) && retained_ids.contains(e.target.as_str())
            })
            .cloned()
            .collect();

        let shared_nodes: Vec<&GraphNode> = filtered_nodes
            .iter()
            .filter(|n| n.trade_ids.len() > 1)
            .collect();

        let metadata = PortfolioGraphMetadata {
            node_count: filtered_nodes.len(),
            edge_count: filtered_edges.len(),
            depth: full_graph.metadata.depth,
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
                let prev_values: HashMap<&str, Option<f64>> = prev
                    .nodes
                    .iter()
                    .map(|n| (n.id.as_str(), n.value))
                    .collect();

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

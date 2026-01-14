//! # Graph Extractor Trait and Implementation
//!
//! This module provides the `GraphExtractable` trait for extracting computation
//! graphs from pricing contexts, and `SimpleGraphExtractor` as the default implementation.
//!
//! ## Performance Requirements
//!
//! - Extract 10,000 nodes in < 1 second
//! - Impact on pricing calculation < 5%
//! - Pre-allocated buffers for memory efficiency

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use super::error::GraphError;
use super::types::{
    ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup, NodeType,
};

// =============================================================================
// GraphExtractable Trait (Task 2.1)
// =============================================================================

/// Trait for extracting computation graphs from pricing contexts.
///
/// Implementors of this trait can extract the dependency graph of computations
/// performed during pricing, enabling visualisation of the computation structure.
///
/// # Requirements
///
/// - `extract_graph`: Extract the full graph for a trade (or all trades)
/// - `extract_affected_nodes`: Extract only nodes affected by updates (for WebSocket)
///
/// # Example
///
/// ```rust,ignore
/// use pricer_pricing::graph::{GraphExtractable, SimpleGraphExtractor};
///
/// let extractor = SimpleGraphExtractor::new();
/// let graph = extractor.extract_graph(Some("T001"))?;
/// println!("Graph has {} nodes", graph.nodes.len());
/// ```
pub trait GraphExtractable {
    /// Extract the computation graph for a specific trade or all trades.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Optional trade ID to filter extraction. If None, extracts
    ///   the combined graph for all trades.
    ///
    /// # Returns
    ///
    /// - `Ok(ComputationGraph)` - The extracted graph with nodes, edges, and metadata
    /// - `Err(GraphError::TradeNotFound)` - If the specified trade does not exist
    /// - `Err(GraphError::ExtractionFailed)` - If extraction fails for any reason
    /// - `Err(GraphError::Timeout)` - If extraction exceeds the time limit
    ///
    /// # Performance
    ///
    /// Should complete within 1 second for graphs up to 10,000 nodes.
    fn extract_graph(&self, trade_id: Option<&str>) -> Result<ComputationGraph, GraphError>;

    /// Extract nodes affected by recent updates (for differential WebSocket updates).
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade ID to check for affected nodes
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<GraphNodeUpdate>)` - List of nodes with updated values
    /// - `Err(GraphError::TradeNotFound)` - If the trade does not exist
    ///
    /// # Usage
    ///
    /// This method is called after market data updates to determine which
    /// nodes have changed values, enabling efficient WebSocket broadcasting.
    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError>;
}

// =============================================================================
// GraphBuilder - Pre-allocated Buffer for Performance (Task 2.3)
// =============================================================================

/// Pre-allocated buffer builder for graph construction.
///
/// Provides memory-efficient graph construction by pre-allocating
/// node and edge vectors to avoid repeated allocations.
///
/// # Performance
///
/// Pre-allocation reduces memory allocation overhead during graph
/// construction, meeting the 10,000 nodes in 1 second requirement.
#[derive(Debug)]
pub struct GraphBuilder {
    /// Pre-allocated node buffer
    nodes: Vec<GraphNode>,
    /// Pre-allocated edge buffer
    edges: Vec<GraphEdge>,
    /// Node ID to index mapping for fast lookup
    node_index: HashMap<String, usize>,
}

impl GraphBuilder {
    /// Create a new GraphBuilder with default capacity.
    ///
    /// Default capacity is 1,000 nodes and 2,000 edges.
    pub fn new() -> Self {
        Self::with_capacity(1_000, 2_000)
    }

    /// Create a new GraphBuilder with specified capacity.
    ///
    /// # Arguments
    ///
    /// * `node_capacity` - Initial capacity for nodes
    /// * `edge_capacity` - Initial capacity for edges
    ///
    /// # Performance
    ///
    /// Pre-allocating sufficient capacity avoids reallocations during
    /// graph construction.
    pub fn with_capacity(node_capacity: usize, edge_capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(node_capacity),
            edges: Vec::with_capacity(edge_capacity),
            node_index: HashMap::with_capacity(node_capacity),
        }
    }

    /// Add a node to the graph.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to add
    ///
    /// # Returns
    ///
    /// The index of the added node.
    pub fn add_node(&mut self, node: GraphNode) -> usize {
        let index = self.nodes.len();
        self.node_index.insert(node.id.clone(), index);
        self.nodes.push(node);
        index
    }

    /// Add an edge to the graph.
    ///
    /// # Arguments
    ///
    /// * `edge` - The edge to add
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Check if a node exists by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to check
    pub fn has_node(&self, id: &str) -> bool {
        self.node_index.contains_key(id)
    }

    /// Get a node by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to look up
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.node_index.get(id).map(|&idx| &self.nodes[idx])
    }

    /// Get a mutable reference to a node by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to look up
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.node_index
            .get(id)
            .copied()
            .map(|idx| &mut self.nodes[idx])
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Clear the builder for reuse.
    ///
    /// This clears all nodes and edges but retains the allocated capacity,
    /// allowing the builder to be reused efficiently.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.node_index.clear();
    }

    /// Calculate the graph depth (longest path from any input to any output).
    ///
    /// Uses topological sort and dynamic programming for O(V + E) complexity.
    pub fn calculate_depth(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }

        // Build adjacency list and in-degree
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::with_capacity(self.nodes.len());
        let mut in_degree: HashMap<&str, usize> = HashMap::with_capacity(self.nodes.len());

        for node in &self.nodes {
            adjacency.entry(node.id.as_str()).or_default();
            in_degree.entry(node.id.as_str()).or_insert(0);
        }

        for edge in &self.edges {
            adjacency
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
            *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
        }

        // Topological sort with distance tracking
        let mut queue: VecDeque<&str> = VecDeque::with_capacity(self.nodes.len());
        let mut distance: HashMap<&str, usize> = HashMap::with_capacity(self.nodes.len());

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*node);
                distance.insert(*node, 0);
            }
        }

        let mut max_depth: usize = 0;

        while let Some(current) = queue.pop_front() {
            let current_dist = *distance.get(current).unwrap_or(&0);
            max_depth = max_depth.max(current_dist);

            if let Some(neighbours) = adjacency.get(current) {
                for &neighbour in neighbours {
                    let new_dist = current_dist + 1;
                    let old_dist = distance.entry(neighbour).or_insert(0);
                    if new_dist > *old_dist {
                        *old_dist = new_dist;
                    }

                    let degree = in_degree.get_mut(neighbour).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        // Depth is max_depth + 1 (counting nodes, not edges)
        max_depth + 1
    }

    /// Validate that the graph is a DAG (no cycles).
    ///
    /// # Returns
    ///
    /// `true` if the graph is a valid DAG, `false` if cycles are detected.
    pub fn is_dag(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }

        // Build in-degree map
        let mut in_degree: HashMap<&str, usize> = HashMap::with_capacity(self.nodes.len());

        for node in &self.nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
        }

        for edge in &self.edges {
            *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
        }

        // Build adjacency list
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::with_capacity(self.nodes.len());
        for edge in &self.edges {
            adjacency
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
        }

        // Kahn's algorithm for topological sort
        let mut queue: VecDeque<&str> = VecDeque::new();
        let mut processed_count = 0;

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*node);
            }
        }

        while let Some(current) = queue.pop_front() {
            processed_count += 1;

            if let Some(neighbours) = adjacency.get(current) {
                for &neighbour in neighbours {
                    let degree = in_degree.get_mut(neighbour).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        // If all nodes were processed, graph is a DAG
        processed_count == self.nodes.len()
    }

    /// Build the final ComputationGraph.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Optional trade ID for metadata
    ///
    /// # Returns
    ///
    /// The completed `ComputationGraph` with calculated metadata.
    pub fn build(self, trade_id: Option<String>) -> ComputationGraph {
        let node_count = self.nodes.len();
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

        ComputationGraph {
            nodes: self.nodes,
            edges: self.edges,
            metadata,
        }
    }

    /// Build the final ComputationGraph, consuming the builder.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Optional trade ID for metadata
    /// * `depth` - Pre-calculated depth (for performance when already known)
    ///
    /// # Returns
    ///
    /// The completed `ComputationGraph` with provided metadata.
    pub fn build_with_depth(self, trade_id: Option<String>, depth: usize) -> ComputationGraph {
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let generated_at = Self::current_timestamp();

        let metadata = GraphMetadata {
            trade_id,
            node_count,
            edge_count,
            depth,
            generated_at,
        };

        ComputationGraph {
            nodes: self.nodes,
            edges: self.edges,
            metadata,
        }
    }

    /// Get the current timestamp in ISO 8601 format.
    fn current_timestamp() -> String {
        // Use a simple format since we don't want to add chrono dependency
        // In production, this would use chrono::Utc::now().to_rfc3339()
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", now.as_secs())
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SimpleGraphExtractor (Task 2.2)
// =============================================================================

/// Simple implementation of graph extraction for demonstration purposes.
///
/// This extractor simulates graph extraction from a pricing context,
/// building computation graphs that represent the dependency structure
/// of pricing calculations.
///
/// # Features
///
/// - Extract graphs for specific trades or all trades
/// - Track sensitivity targets for AD
/// - Calculate graph depth and validate DAG structure
///
/// # Performance (Task 2.3)
///
/// - Pre-allocated buffers via `GraphBuilder`
/// - O(V + E) graph construction
/// - Timeout protection (500ms default)
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::{SimpleGraphExtractor, GraphExtractable};
///
/// let mut extractor = SimpleGraphExtractor::new();
/// extractor.register_trade("T001", vec!["spot", "vol", "rate"]);
///
/// let graph = extractor.extract_graph(Some("T001")).unwrap();
/// assert!(graph.nodes.len() > 0);
/// ```
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

    /// Create a new SimpleGraphExtractor with custom timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Timeout in milliseconds for graph extraction
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Create a new SimpleGraphExtractor with custom capacity.
    ///
    /// # Arguments
    ///
    /// * `node_capacity` - Initial capacity for nodes
    /// * `edge_capacity` - Initial capacity for edges
    pub fn with_capacity(mut self, node_capacity: usize, edge_capacity: usize) -> Self {
        self.builder_capacity = (node_capacity, edge_capacity);
        self
    }

    /// Register a trade with its sensitivity parameters.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade identifier
    /// * `sensitivity_params` - List of parameter names that are AD seed points
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::graph::SimpleGraphExtractor;
    ///
    /// let mut extractor = SimpleGraphExtractor::new();
    /// extractor.register_trade("T001", vec!["spot", "vol", "rate"]);
    /// ```
    pub fn register_trade<S: Into<String>>(&mut self, trade_id: &str, sensitivity_params: Vec<S>) {
        let info = TradeGraphInfo {
            sensitivity_params: sensitivity_params.into_iter().map(|s| s.into()).collect(),
            param_values: HashMap::new(),
            computed_values: HashMap::new(),
        };
        self.trades.insert(trade_id.to_string(), info);
    }

    /// Set parameter values for a trade.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade identifier
    /// * `param_name` - The parameter name
    /// * `value` - The parameter value
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

    /// Set computed value for a trade.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The trade identifier
    /// * `node_id` - The node identifier
    /// * `value` - The computed value
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
    pub fn has_trade(&self, trade_id: &str) -> bool {
        self.trades.contains_key(trade_id)
    }

    /// Get the number of registered trades.
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

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
    fn default() -> Self {
        Self::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 2.1: GraphExtractable Trait Tests
    // =========================================================================

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

    // =========================================================================
    // Task 2.2: SimpleGraphExtractor Implementation Tests
    // =========================================================================

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

    // =========================================================================
    // Task 2.3: Performance and GraphBuilder Tests
    // =========================================================================

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
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
            };

            let index = builder.add_node(node);

            assert_eq!(index, 0);
            assert_eq!(builder.node_count(), 1);
            assert!(builder.has_node("N1"));
        }

        #[test]
        fn test_builder_add_edge() {
            let mut builder = GraphBuilder::new();

            let edge = GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            };
            builder.add_edge(edge);

            assert_eq!(builder.edge_count(), 1);
        }

        #[test]
        fn test_builder_get_node() {
            let mut builder = GraphBuilder::new();
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
            };
            builder.add_node(node);

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
            builder.add_node(GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: None,
                is_sensitivity_target: false,
                group: NodeGroup::Input,
            });
            builder.add_edge(GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            });

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
            builder.add_node(GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "x".to_string(),
                value: None,
                is_sensitivity_target: false,
                group: NodeGroup::Input,
            });

            assert_eq!(builder.calculate_depth(), 1);
        }

        #[test]
        fn test_builder_calculate_depth_linear() {
            let mut builder = GraphBuilder::new();

            // Create linear chain: N1 -> N2 -> N3
            for i in 1..=3 {
                builder.add_node(GraphNode {
                    id: format!("N{}", i),
                    node_type: if i == 1 {
                        NodeType::Input
                    } else if i == 3 {
                        NodeType::Output
                    } else {
                        NodeType::Add
                    },
                    label: format!("n{}", i),
                    value: None,
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                });
            }

            builder.add_edge(GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            });
            builder.add_edge(GraphEdge {
                source: "N2".to_string(),
                target: "N3".to_string(),
                weight: None,
            });

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

            builder.add_node(GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "x".to_string(),
                value: None,
                is_sensitivity_target: false,
                group: NodeGroup::Input,
            });
            builder.add_node(GraphNode {
                id: "N2".to_string(),
                node_type: NodeType::Output,
                label: "y".to_string(),
                value: None,
                is_sensitivity_target: false,
                group: NodeGroup::Output,
            });
            builder.add_edge(GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            });

            assert!(builder.is_dag());
        }

        #[test]
        fn test_builder_build() {
            let mut builder = GraphBuilder::new();
            builder.add_node(GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "x".to_string(),
                value: Some(1.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
            });
            builder.add_node(GraphNode {
                id: "N2".to_string(),
                node_type: NodeType::Output,
                label: "y".to_string(),
                value: Some(2.0),
                is_sensitivity_target: false,
                group: NodeGroup::Output,
            });
            builder.add_edge(GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            });

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
        use super::*;
        use std::time::Duration;

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

            // First use
            for i in 0..100 {
                builder.add_node(GraphNode {
                    id: format!("N{}", i),
                    node_type: NodeType::Add,
                    label: format!("n{}", i),
                    value: None,
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                });
            }

            assert_eq!(builder.node_count(), 100);

            // Clear and reuse
            builder.clear();

            assert_eq!(builder.node_count(), 0);

            // Second use - should not need reallocation
            for i in 0..50 {
                builder.add_node(GraphNode {
                    id: format!("M{}", i),
                    node_type: NodeType::Mul,
                    label: format!("m{}", i),
                    value: None,
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                });
            }

            assert_eq!(builder.node_count(), 50);
        }
    }
}

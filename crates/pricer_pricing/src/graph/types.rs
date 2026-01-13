//! # Computation Graph Data Types
//!
//! Core data structures for representing computation graphs extracted from
//! the Enzyme AD engine. These types are designed to be D3.js compatible
//! for force-directed graph visualisation in the FrictionalBank Web Dashboard.
//!
//! ## D3.js Compatibility
//!
//! - `ComputationGraph.edges` is serialised as `links` for D3.js
//! - `GraphNode.node_type` is serialised as `type` for D3.js
//! - All enum variants use lowercase serialisation (`#[serde(rename_all = "lowercase")]`)

use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "serde")]
use serde::Serialize;

// =============================================================================
// NodeType Enumeration (Task 1.1)
// =============================================================================

/// Operation type for a computation graph node.
///
/// Represents the type of mathematical operation performed by a node
/// in the Enzyme AD computation graph.
///
/// # D3.js Compatibility
///
/// All variants are serialised to lowercase (e.g., `Add` -> `"add"`).
///
/// # Supported Operations
///
/// - `Input`: Input variable (e.g., market data, model parameters)
/// - `Add`: Addition operation
/// - `Mul`: Multiplication operation
/// - `Exp`: Exponential function (e^x)
/// - `Log`: Natural logarithm (ln(x))
/// - `Sqrt`: Square root
/// - `Div`: Division operation
/// - `Output`: Final output value
/// - `Custom(u8)`: User-defined custom operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum NodeType {
    /// Input variable (market data, model parameters)
    Input,
    /// Addition operation
    Add,
    /// Multiplication operation
    Mul,
    /// Exponential function (e^x)
    Exp,
    /// Natural logarithm (ln(x))
    Log,
    /// Square root
    Sqrt,
    /// Division operation
    Div,
    /// Final output value
    Output,
    /// User-defined custom operation type
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_custom"))]
    Custom(u8),
}

#[cfg(feature = "serde")]
fn serialize_custom<S>(value: &u8, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("custom_{}", value))
}

// =============================================================================
// NodeGroup Enumeration (Task 1.1)
// =============================================================================

/// Visual grouping for a computation graph node.
///
/// Used for colour coding and layout organisation in the D3.js visualisation.
///
/// # Colour Mapping
///
/// - `Input`: Blue (#3b82f6)
/// - `Intermediate`: Grey (#6b7280)
/// - `Output`: Green (#22c55e)
/// - `Sensitivity`: Orange (#f97316)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum NodeGroup {
    /// Input nodes (market data, parameters)
    Input,
    /// Intermediate computation nodes
    Intermediate,
    /// Output nodes (pricing results)
    Output,
    /// Sensitivity target nodes (AD differentiation points)
    Sensitivity,
}

// =============================================================================
// GraphNode Structure (Task 1.1)
// =============================================================================

/// A node in the computation graph.
///
/// Represents a single computation step in the Enzyme AD graph,
/// including the operation type, current value, and metadata.
///
/// # D3.js Compatibility
///
/// The `node_type` field is serialised as `"type"` for D3.js compatibility.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::{GraphNode, NodeType, NodeGroup};
///
/// let node = GraphNode {
///     id: "N1".to_string(),
///     node_type: NodeType::Input,
///     label: "spot".to_string(),
///     value: Some(100.0),
///     is_sensitivity_target: true,
///     group: NodeGroup::Input,
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GraphNode {
    /// Unique identifier for the node
    pub id: String,

    /// Operation type performed by this node
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub node_type: NodeType,

    /// Human-readable label (variable name or operation description)
    pub label: String,

    /// Current computed value (None if not yet computed)
    pub value: Option<f64>,

    /// Whether this node is a sensitivity calculation target (AD seed point)
    pub is_sensitivity_target: bool,

    /// Visual grouping for colour coding
    pub group: NodeGroup,
}

// =============================================================================
// GraphEdge Structure (Task 1.1)
// =============================================================================

/// An edge connecting two nodes in the computation graph.
///
/// Represents a data dependency between two computation nodes.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::GraphEdge;
///
/// let edge = GraphEdge {
///     source: "N1".to_string(),
///     target: "N2".to_string(),
///     weight: Some(1.0),
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GraphEdge {
    /// Source node ID (input to the operation)
    pub source: String,

    /// Target node ID (output of the operation)
    pub target: String,

    /// Optional edge weight (for weighted graph analysis)
    pub weight: Option<f64>,
}

// =============================================================================
// GraphMetadata Structure (Task 1.2)
// =============================================================================

/// Metadata about a computation graph.
///
/// Contains summary statistics and identification information
/// for a computation graph extraction.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::GraphMetadata;
///
/// let metadata = GraphMetadata {
///     trade_id: Some("T001".to_string()),
///     node_count: 150,
///     edge_count: 200,
///     depth: 12,
///     generated_at: "2026-01-13T12:00:00Z".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GraphMetadata {
    /// Trade ID this graph belongs to (None for aggregate graphs)
    pub trade_id: Option<String>,

    /// Total number of nodes in the graph
    pub node_count: usize,

    /// Total number of edges in the graph
    pub edge_count: usize,

    /// Maximum depth of the graph (longest path from input to output)
    pub depth: usize,

    /// ISO 8601 timestamp of graph generation
    pub generated_at: String,
}

// =============================================================================
// ComputationGraph Structure (Task 1.2)
// =============================================================================

/// Complete computation graph representation.
///
/// Contains all nodes, edges, and metadata for a computation graph
/// extracted from the Enzyme AD engine.
///
/// # D3.js Compatibility
///
/// The `edges` field is serialised as `"links"` for D3.js force-directed
/// graph compatibility.
///
/// # Example JSON Output
///
/// ```json
/// {
///   "nodes": [...],
///   "links": [...],
///   "metadata": {...}
/// }
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ComputationGraph {
    /// All nodes in the computation graph
    pub nodes: Vec<GraphNode>,

    /// All edges in the computation graph (serialised as "links" for D3.js)
    #[cfg_attr(feature = "serde", serde(rename = "links"))]
    pub edges: Vec<GraphEdge>,

    /// Graph metadata (statistics, timestamps, identification)
    pub metadata: GraphMetadata,
}

impl ComputationGraph {
    /// Find a node by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to search for
    ///
    /// # Returns
    ///
    /// Reference to the node if found, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::graph::{ComputationGraph, GraphNode, GraphEdge, GraphMetadata, NodeType, NodeGroup};
    ///
    /// let graph = ComputationGraph {
    ///     nodes: vec![GraphNode {
    ///         id: "N1".to_string(),
    ///         node_type: NodeType::Input,
    ///         label: "spot".to_string(),
    ///         value: Some(100.0),
    ///         is_sensitivity_target: true,
    ///         group: NodeGroup::Input,
    ///     }],
    ///     edges: vec![],
    ///     metadata: GraphMetadata {
    ///         trade_id: None,
    ///         node_count: 1,
    ///         edge_count: 0,
    ///         depth: 1,
    ///         generated_at: "2026-01-13T12:00:00Z".to_string(),
    ///     },
    /// };
    ///
    /// let node = graph.find_node("N1");
    /// assert!(node.is_some());
    /// assert_eq!(node.unwrap().label, "spot");
    /// ```
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find a path between two nodes using BFS.
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID
    /// * `to` - Target node ID
    ///
    /// # Returns
    ///
    /// Vector of node IDs representing the path from source to target,
    /// or None if no path exists.
    ///
    /// # Algorithm
    ///
    /// Uses Breadth-First Search (BFS) to find the shortest path.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }

        // Build adjacency list for efficient traversal
        let adjacency: HashMap<&str, Vec<&str>> = self
            .edges
            .iter()
            .fold(HashMap::new(), |mut acc, edge| {
                acc.entry(edge.source.as_str())
                    .or_default()
                    .push(edge.target.as_str());
                acc
            });

        // BFS for shortest path
        let mut queue: VecDeque<Vec<String>> = VecDeque::new();
        let mut visited: HashSet<&str> = HashSet::new();

        queue.push_back(vec![from.to_string()]);
        visited.insert(from);

        while let Some(path) = queue.pop_front() {
            let current = path.last().unwrap().as_str();

            if let Some(neighbours) = adjacency.get(current) {
                for &neighbour in neighbours {
                    if neighbour == to {
                        let mut result = path.clone();
                        result.push(neighbour.to_string());
                        return Some(result);
                    }

                    if !visited.contains(neighbour) {
                        visited.insert(neighbour);
                        let mut new_path = path.clone();
                        new_path.push(neighbour.to_string());
                        queue.push_back(new_path);
                    }
                }
            }
        }

        None
    }

    /// Get the critical path (longest path) through the graph.
    ///
    /// The critical path is the longest dependency chain from any input
    /// to any output node. This is useful for identifying bottlenecks
    /// in the computation.
    ///
    /// # Algorithm
    ///
    /// Uses topological sort and dynamic programming to find the longest
    /// path in the DAG.
    ///
    /// # Returns
    ///
    /// Vector of node IDs representing the critical path.
    pub fn get_critical_path(&self) -> Vec<String> {
        if self.nodes.is_empty() {
            return vec![];
        }

        // Build adjacency list and in-degree map for topological sort
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        // Initialise all nodes with zero in-degree
        for node in &self.nodes {
            adjacency.entry(node.id.as_str()).or_default();
            in_degree.entry(node.id.as_str()).or_insert(0);
        }

        // Build graph structure
        for edge in &self.edges {
            adjacency
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
            *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
        }

        // Topological sort with longest path tracking
        let mut queue: VecDeque<&str> = VecDeque::new();
        let mut distance: HashMap<&str, usize> = HashMap::new();
        let mut predecessor: HashMap<&str, Option<&str>> = HashMap::new();

        // Start with nodes that have no incoming edges (sources)
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*node);
                distance.insert(*node, 0);
                predecessor.insert(*node, None);
            }
        }

        let mut last_node: Option<&str> = None;
        let mut max_distance: usize = 0;

        while let Some(current) = queue.pop_front() {
            let current_dist = *distance.get(current).unwrap_or(&0);

            if current_dist >= max_distance {
                max_distance = current_dist;
                last_node = Some(current);
            }

            if let Some(neighbours) = adjacency.get(current) {
                for &neighbour in neighbours {
                    let new_dist = current_dist + 1;
                    if new_dist > *distance.get(neighbour).unwrap_or(&0) {
                        distance.insert(neighbour, new_dist);
                        predecessor.insert(neighbour, Some(current));
                    }

                    let degree = in_degree.get_mut(neighbour).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        // Reconstruct the path
        let mut path: Vec<String> = Vec::new();
        let mut current = last_node;

        while let Some(node) = current {
            path.push(node.to_string());
            current = predecessor.get(node).and_then(|&p| p);
        }

        path.reverse();
        path
    }
}

// =============================================================================
// GraphNodeUpdate Structure (Task 2.1 partial - for WebSocket updates)
// =============================================================================

/// Update information for a single graph node.
///
/// Used for WebSocket real-time updates to send only the changed
/// nodes rather than the entire graph.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::graph::GraphNodeUpdate;
///
/// let update = GraphNodeUpdate {
///     id: "N1".to_string(),
///     value: 101.5,
///     delta: Some(1.5),
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GraphNodeUpdate {
    /// Node ID being updated
    pub id: String,

    /// New computed value
    pub value: f64,

    /// Change from previous value (for animation)
    pub delta: Option<f64>,
}

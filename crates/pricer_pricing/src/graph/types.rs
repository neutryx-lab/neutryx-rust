//! # Computation Graph Data Types

use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;

/// Operation type for a computation graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
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
    #[serde(serialize_with = "serialize_custom")]
    Custom(u8),
}

fn serialize_custom<S>(value: &u8, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("custom_{}", value))
}

/// Visual grouping for a computation graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
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

/// A node in the computation graph.
#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    /// Unique identifier for the node
    pub id: String,

    /// Operation type performed by this node
    #[serde(rename = "type")]
    pub node_type: NodeType,

    /// Human-readable label (variable name or operation description)
    pub label: String,

    /// Current computed value (None if not yet computed)
    pub value: Option<f64>,

    /// Whether this node is a sensitivity calculation target (AD seed point)
    pub is_sensitivity_target: bool,

    /// Visual grouping for colour coding
    pub group: NodeGroup,

    /// Trade IDs this node belongs to (Portfolio support)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trade_ids: Vec<String>,
}

impl Default for GraphNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            node_type: NodeType::Input,
            label: String::new(),
            value: None,
            is_sensitivity_target: false,
            group: NodeGroup::Intermediate,
            trade_ids: Vec::new(),
        }
    }
}

/// An edge connecting two nodes in the computation graph.
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    /// Source node ID (input to the operation)
    pub source: String,

    /// Target node ID (output of the operation)
    pub target: String,

    /// Optional edge weight (for weighted graph analysis)
    pub weight: Option<f64>,
}

/// Metadata about a computation graph.
#[derive(Debug, Clone, Serialize)]
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

/// Complete computation graph representation.
#[derive(Debug, Clone, Serialize)]
pub struct ComputationGraph {
    /// All nodes in the computation graph
    pub nodes: Vec<GraphNode>,

    /// All edges in the computation graph (serialised as "links" for D3.js)
    #[serde(rename = "links")]
    pub edges: Vec<GraphEdge>,

    /// Graph metadata (statistics, timestamps, identification)
    pub metadata: GraphMetadata,
}

impl ComputationGraph {
    /// Find a node by its ID.
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find a path between two nodes using BFS.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }

        let adjacency: HashMap<&str, Vec<&str>> =
            self.edges.iter().fold(HashMap::new(), |mut acc, edge| {
                acc.entry(edge.source.as_str())
                    .or_default()
                    .push(edge.target.as_str());
                acc
            });

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
    pub fn get_critical_path(&self) -> Vec<String> {
        if self.nodes.is_empty() {
            return vec![];
        }

        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

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

        let mut queue: VecDeque<&str> = VecDeque::new();
        let mut distance: HashMap<&str, usize> = HashMap::new();
        let mut predecessor: HashMap<&str, Option<&str>> = HashMap::new();

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

/// Update information for a single graph node.
#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeUpdate {
    /// Node ID being updated
    pub id: String,

    /// New computed value
    pub value: f64,

    /// Change from previous value (for animation)
    pub delta: Option<f64>,
}

/// Metadata for a Portfolio-level computation graph.
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioGraphMetadata {
    /// Total number of nodes in the graph
    pub node_count: usize,

    /// Total number of edges in the graph
    pub edge_count: usize,

    /// Maximum depth of the graph (longest path from input to output)
    pub depth: usize,

    /// ISO 8601 timestamp of graph generation
    pub generated_at: String,

    /// Number of trades in the Portfolio
    pub trade_count: usize,

    /// Number of nodes shared between multiple trades
    pub shared_node_count: usize,

    /// Optimisation ratio: nodes after deduplication / nodes before
    pub optimisation_ratio: f64,
}

/// Complete computation graph representation for a Portfolio.
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioComputationGraph {
    /// All nodes in the computation graph (with trade_ids populated)
    pub nodes: Vec<GraphNode>,

    /// All edges in the computation graph (serialised as "links" for D3.js)
    #[serde(rename = "links")]
    pub edges: Vec<GraphEdge>,

    /// Portfolio-specific metadata (statistics, timestamps)
    pub metadata: PortfolioGraphMetadata,
}

impl PortfolioComputationGraph {
    /// Find a node by its ID.
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all nodes belonging to a specific trade.
    pub fn nodes_for_trade(&self, trade_id: &str) -> Vec<&GraphNode> {
        self.nodes
            .iter()
            .filter(|n| n.trade_ids.contains(&trade_id.to_string()))
            .collect()
    }

    /// Get all shared nodes (nodes belonging to multiple trades).
    pub fn shared_nodes(&self) -> Vec<&GraphNode> {
        self.nodes
            .iter()
            .filter(|n| n.trade_ids.len() > 1)
            .collect()
    }
}

//! VolCube Graph Extractor for AAD visualization.
//!
//! # Requirements: 7.3, 7.4 (Task 10.1)
//!
//! This module provides `VolCubeGraphExtractor` which implements the
//! `GraphExtractable` trait for VolCube computation graphs.
//!
//! # Architecture
//!
//! ```text
//! pricer_models::market::volcube::graph  (L2 - data extraction)
//!                     ↓
//! pricer_pricing::graph::volcube_extractor  (L3 - trait implementation)
//!                     ↓
//! ComputationGraph for D3.js visualization
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use pricer_pricing::graph::{VolCubeGraphExtractor, GraphExtractable};
//! use pricer_models::market::volcube::VolCube;
//!
//! let cube: VolCube<f64> = /* ... */;
//! let extractor = VolCubeGraphExtractor::new(&cube, "CUBE-001");
//! let graph = extractor.extract_graph(None)?;
//! ```

use std::time::Instant;

use pricer_models::market::volcube::{
    graph::{VolCubeGraphData, VolCubeNodeType},
    VolCube,
};

use super::{
    error::GraphError,
    extractor::GraphExtractable,
    types::{
        ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup, NodeType,
    },
};

// =============================================================================
// VolCubeGraphExtractor
// =============================================================================

/// Graph extractor for VolCube computation graphs.
///
/// # Requirements: 7.3, 7.4
///
/// Extracts the calibration graph structure from a VolCube, converting
/// VolQuotes → SABRParams → InterpolatedVol dependencies into a format
/// suitable for D3.js visualization.
///
/// # Node Types
///
/// - Instrument nodes: Input volatility quotes
/// - SabrSlice nodes: Calibrated SABR parameters per expiry-tenor
/// - Cube node: Interpolated volatility output
///
/// # Example
///
/// ```ignore
/// let extractor = VolCubeGraphExtractor::new(&cube, "USD-SOFR-VOL");
/// let graph = extractor.extract_graph(None)?;
/// println!("Graph has {} nodes", graph.nodes.len());
/// ```
#[derive(Debug)]
pub struct VolCubeGraphExtractor<'a, T: num_traits::Float + Send + Sync> {
    /// Reference to the VolCube.
    cube: &'a VolCube<T>,
    /// Identifier for the cube (used as graph ID).
    cube_id: String,
    /// Cached graph data.
    graph_data: VolCubeGraphData,
}

impl<'a, T: num_traits::Float + Send + Sync> VolCubeGraphExtractor<'a, T> {
    /// Create a new VolCube graph extractor.
    ///
    /// # Arguments
    ///
    /// * `cube` - Reference to the VolCube
    /// * `cube_id` - Identifier for the cube (used as graph root node ID)
    ///
    /// # Returns
    ///
    /// A new `VolCubeGraphExtractor` instance.
    pub fn new(cube: &'a VolCube<T>, cube_id: impl Into<String>) -> Self {
        let cube_id = cube_id.into();
        let graph_data = VolCubeGraphData::from_cube(cube, &cube_id);
        Self {
            cube,
            cube_id,
            graph_data,
        }
    }

    /// Get the underlying VolCube reference.
    pub fn cube(&self) -> &'a VolCube<T> { self.cube }

    /// Get the cube identifier.
    pub fn cube_id(&self) -> &str { &self.cube_id }

    /// Get the raw graph data.
    pub fn graph_data(&self) -> &VolCubeGraphData { &self.graph_data }

    /// Convert VolCubeNodeType to NodeType.
    fn convert_node_type(volcube_type: VolCubeNodeType) -> NodeType {
        match volcube_type {
            VolCubeNodeType::Cube => NodeType::Output,
            VolCubeNodeType::SabrSlice => NodeType::Custom(1), // SABR calibration
            VolCubeNodeType::Instrument => NodeType::Input,
            VolCubeNodeType::Interpolation => NodeType::Custom(2), // Interpolation
        }
    }

    /// Convert VolCubeNodeType to NodeGroup.
    fn convert_node_group(volcube_type: VolCubeNodeType) -> NodeGroup {
        match volcube_type {
            VolCubeNodeType::Cube => NodeGroup::Output,
            VolCubeNodeType::SabrSlice => NodeGroup::Intermediate,
            VolCubeNodeType::Instrument => NodeGroup::Input,
            VolCubeNodeType::Interpolation => NodeGroup::Intermediate,
        }
    }

    /// Determine if a node is a sensitivity target.
    fn is_sensitivity_target(volcube_type: VolCubeNodeType) -> bool {
        matches!(volcube_type, VolCubeNodeType::Instrument)
    }

    /// Calculate graph depth using BFS.
    fn calculate_depth(&self) -> usize {
        use std::collections::{HashMap, VecDeque};

        // Build adjacency list (source -> targets)
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.graph_data.edges {
            adj.entry(&edge.source).or_default().push(&edge.target);
        }

        // Find input nodes (no incoming edges)
        let targets: std::collections::HashSet<_> = self
            .graph_data
            .edges
            .iter()
            .map(|e| e.target.as_str())
            .collect();
        let sources: Vec<_> = self
            .graph_data
            .nodes
            .iter()
            .filter(|n| !targets.contains(n.id.as_str()))
            .map(|n| n.id.as_str())
            .collect();

        // BFS to find max depth
        let mut max_depth = 0;
        let mut queue = VecDeque::new();
        let mut depths: HashMap<&str, usize> = HashMap::new();

        for source in sources {
            queue.push_back((source, 1));
            depths.insert(source, 1);
        }

        while let Some((node, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if !depths.contains_key(neighbor) || depths[neighbor] < depth + 1 {
                        depths.insert(neighbor, depth + 1);
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        max_depth
    }
}

impl<T: num_traits::Float + Send + Sync> GraphExtractable for VolCubeGraphExtractor<'_, T> {
    /// Extract the computation graph from the VolCube.
    ///
    /// # Arguments
    ///
    /// * `trade_id` - Optional trade ID (ignored for VolCube, uses cube_id)
    ///
    /// # Returns
    ///
    /// The extracted `ComputationGraph` with nodes, edges, and metadata.
    fn extract_graph(&self, _trade_id: Option<&str>) -> Result<ComputationGraph, GraphError> {
        let start_time = Instant::now();

        // Convert nodes
        let nodes: Vec<GraphNode> = self
            .graph_data
            .nodes
            .iter()
            .map(|n| GraphNode {
                id: n.id.clone(),
                node_type: Self::convert_node_type(n.node_type),
                label: n.label.clone(),
                value: n.value,
                is_sensitivity_target: Self::is_sensitivity_target(n.node_type),
                group: Self::convert_node_group(n.node_type),
                trade_ids: vec![],
            })
            .collect();

        // Convert edges
        let edges: Vec<GraphEdge> = self
            .graph_data
            .edges
            .iter()
            .map(|e| GraphEdge {
                source: e.source.clone(),
                target: e.target.clone(),
                weight: None,
            })
            .collect();

        // Calculate metadata
        let depth = self.calculate_depth();
        let generated_at = chrono::Utc::now().to_rfc3339();

        let metadata = GraphMetadata {
            trade_id: Some(self.cube_id.clone()),
            node_count: nodes.len(),
            edge_count: edges.len(),
            depth,
            generated_at,
        };

        // Check timeout (1 second limit)
        if start_time.elapsed().as_secs() > 1 {
            return Err(GraphError::Timeout);
        }

        Ok(ComputationGraph {
            nodes,
            edges,
            metadata,
        })
    }

    /// Extract affected nodes (for differential updates).
    ///
    /// # Arguments
    ///
    /// * `trade_id` - The cube ID to check
    ///
    /// # Returns
    ///
    /// Empty vector (VolCube doesn't track differential updates yet).
    fn extract_affected_nodes(&self, trade_id: &str) -> Result<Vec<GraphNodeUpdate>, GraphError> {
        if trade_id != self.cube_id {
            return Err(GraphError::TradeNotFound(trade_id.to_string()));
        }
        // No differential updates implemented yet
        Ok(vec![])
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use pricer_models::market::volcube::{
        InstrumentId, SabrParameterSurface, SabrParams, VolCubeConfig,
    };

    use super::*;

    fn create_test_cube() -> VolCube<f64> {
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0, 5.0];
        let beta = 0.5;

        let params = vec![
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),
                SabrParams::new(0.05, beta, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.045, beta, -0.35, 0.45),
                SabrParams::new(0.055, beta, -0.2, 0.3),
            ],
        ];

        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap();
        let forwards = vec![vec![0.03, 0.035], vec![0.032, 0.038]];
        let config = VolCubeConfig::default();
        let source_instruments = vec![
            InstrumentId::new("INST-1"),
            InstrumentId::new("INST-2"),
            InstrumentId::new("INST-3"),
        ];
        let strike_domain = (0.01, 0.10);

        VolCube::new(
            sabr_surface,
            forwards,
            config,
            source_instruments,
            strike_domain,
        )
    }

    #[test]
    fn test_extractor_creation() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        assert_eq!(extractor.cube_id(), "TEST-CUBE");
    }

    #[test]
    fn test_extract_graph() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Should have nodes
        assert!(!graph.nodes.is_empty());
        // Should have edges
        assert!(!graph.edges.is_empty());
        // Metadata should be populated
        assert_eq!(graph.metadata.trade_id, Some("TEST-CUBE".to_string()));
        assert_eq!(graph.metadata.node_count, graph.nodes.len());
        assert_eq!(graph.metadata.edge_count, graph.edges.len());
    }

    #[test]
    fn test_node_type_conversion() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Find different node types
        let input_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Input)
            .collect();
        let output_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Output)
            .collect();

        // Should have input nodes (instruments)
        assert!(!input_nodes.is_empty());
        // Should have output node (cube)
        assert!(!output_nodes.is_empty());
    }

    #[test]
    fn test_sensitivity_targets() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Input nodes (instruments) should be sensitivity targets
        let sensitivity_targets: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.is_sensitivity_target)
            .collect();

        assert!(!sensitivity_targets.is_empty());
        // All sensitivity targets should be Input type
        for node in &sensitivity_targets {
            assert_eq!(node.node_type, NodeType::Input);
        }
    }

    #[test]
    fn test_node_groups() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Check that we have different groups
        let input_group: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.group == NodeGroup::Input)
            .collect();
        let intermediate_group: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.group == NodeGroup::Intermediate)
            .collect();
        let output_group: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.group == NodeGroup::Output)
            .collect();

        assert!(!input_group.is_empty());
        assert!(!intermediate_group.is_empty());
        assert!(!output_group.is_empty());
    }

    #[test]
    fn test_extract_affected_nodes_not_found() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let result = extractor.extract_affected_nodes("WRONG-ID");
        assert!(matches!(result, Err(GraphError::TradeNotFound(_))));
    }

    #[test]
    fn test_extract_affected_nodes_empty() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let result = extractor.extract_affected_nodes("TEST-CUBE").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_graph_data_access() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let data = extractor.graph_data();
        assert_eq!(data.id, "TEST-CUBE");
        assert!(!data.nodes.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_d3_json_compatibility() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Serialize to JSON
        let json = serde_json::to_value(&graph).unwrap();

        // Should have "links" (not "edges") for D3.js compatibility
        assert!(json.get("links").is_some());
        assert!(json.get("edges").is_none());

        // Should have "nodes" array
        assert!(json.get("nodes").is_some());

        // Nodes should have "type" (not "node_type")
        let nodes = json.get("nodes").unwrap().as_array().unwrap();
        if !nodes.is_empty() {
            assert!(nodes[0].get("type").is_some());
            assert!(nodes[0].get("node_type").is_none());
        }
    }

    #[test]
    fn test_graph_depth_calculation() {
        let cube = create_test_cube();
        let extractor = VolCubeGraphExtractor::new(&cube, "TEST-CUBE");

        let graph = extractor.extract_graph(None).unwrap();

        // Depth should be at least 2 (input -> output)
        assert!(graph.metadata.depth >= 2);
    }
}

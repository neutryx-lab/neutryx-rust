//! # Computation Graph Visualisation
//!
//! This module provides data structures for extracting and representing
//! computation graphs from the Enzyme AD engine for visualisation purposes.
//!
//! ## Module Structure
//!
//! - `types`: Core data structures (GraphNode, GraphEdge, ComputationGraph)
//! - `error`: Error types for graph operations
//! - `extractor`: Graph extraction trait and implementations
//!
//! ## D3.js Compatibility
//!
//! All structures are designed to serialise to JSON format compatible with
//! D3.js force-directed graph visualisation:
//! - `edges` field is renamed to `links` in JSON output
//! - `node_type` field is renamed to `type` in JSON output
//!
//! ## Graph Extraction
//!
//! Use `GraphExtractable` trait and `SimpleGraphExtractor` to extract
//! computation graphs from pricing contexts:
//!
//! ```rust
//! use pricer_pricing::graph::{SimpleGraphExtractor, GraphExtractable};
//!
//! let mut extractor = SimpleGraphExtractor::new();
//! extractor.register_trade("T001", vec!["spot", "vol"]);
//!
//! let graph = extractor.extract_graph(Some("T001")).unwrap();
//! assert!(graph.nodes.len() > 0);
//! ```

mod error;
mod extractor;
mod types;

#[cfg(feature = "l1l2-integration")]
mod volcube_extractor;

pub use error::GraphError;
pub use extractor::{
    GraphBuilder, GraphExtractable, PortfolioGraphExtractable, PortfolioGraphExtractor,
    SimpleGraphExtractor,
};
pub use types::{
    ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup, NodeType,
    PortfolioComputationGraph, PortfolioGraphMetadata,
};
#[cfg(feature = "l1l2-integration")]
pub use volcube_extractor::VolCubeGraphExtractor;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 1.1: GraphNode and GraphEdge Tests
    // =========================================================================

    mod node_tests {
        use super::*;

        #[test]
        fn test_graph_node_creation() {
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec![],
            };

            assert_eq!(node.id, "N1");
            assert_eq!(node.node_type, NodeType::Input);
            assert_eq!(node.label, "spot");
            assert_eq!(node.value, Some(100.0));
            assert!(node.is_sensitivity_target);
            assert_eq!(node.group, NodeGroup::Input);
        }

        #[test]
        fn test_graph_node_clone() {
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Add,
                label: "a + b".to_string(),
                value: Some(42.0),
                is_sensitivity_target: false,
                group: NodeGroup::Intermediate,
                trade_ids: vec![],
            };

            let cloned = node.clone();
            assert_eq!(cloned.id, node.id);
            assert_eq!(cloned.node_type, node.node_type);
            assert_eq!(cloned.label, node.label);
            assert_eq!(cloned.value, node.value);
        }

        #[test]
        fn test_graph_edge_creation() {
            let edge = GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: Some(1.5),
            };

            assert_eq!(edge.source, "N1");
            assert_eq!(edge.target, "N2");
            assert_eq!(edge.weight, Some(1.5));
        }

        #[test]
        fn test_graph_edge_without_weight() {
            let edge = GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            };

            assert!(edge.weight.is_none());
        }
    }

    mod node_type_tests {
        use super::*;

        #[test]
        fn test_all_node_types() {
            // Verify all required node types exist
            let input = NodeType::Input;
            let add = NodeType::Add;
            let mul = NodeType::Mul;
            let exp = NodeType::Exp;
            let log = NodeType::Log;
            let sqrt = NodeType::Sqrt;
            let div = NodeType::Div;
            let output = NodeType::Output;
            let custom = NodeType::Custom(42);

            // Verify they are distinguishable
            assert_ne!(input, add);
            assert_ne!(add, mul);
            assert_ne!(exp, log);
            assert_ne!(sqrt, div);
            assert_ne!(output, custom);
            assert_eq!(NodeType::Custom(42), NodeType::Custom(42));
            assert_ne!(NodeType::Custom(1), NodeType::Custom(2));
        }

        #[test]
        fn test_node_type_copy() {
            let node_type = NodeType::Mul;
            let copied = node_type;
            assert_eq!(node_type, copied);
        }
    }

    mod node_group_tests {
        use super::*;

        #[test]
        fn test_all_node_groups() {
            // Verify all required node groups exist
            let input = NodeGroup::Input;
            let intermediate = NodeGroup::Intermediate;
            let output = NodeGroup::Output;
            let sensitivity = NodeGroup::Sensitivity;

            // Verify they are distinguishable
            assert_ne!(input, intermediate);
            assert_ne!(intermediate, output);
            assert_ne!(output, sensitivity);
        }

        #[test]
        fn test_node_group_copy() {
            let group = NodeGroup::Sensitivity;
            let copied = group;
            assert_eq!(group, copied);
        }
    }

    #[cfg(feature = "serde")]
    mod serialisation_tests {
        use super::*;

        #[test]
        fn test_graph_node_serialisation() {
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec![],
            };

            let json = serde_json::to_string(&node).unwrap();

            // Verify D3.js compatible field name: node_type -> type
            assert!(json.contains("\"type\":\"input\""));
            assert!(json.contains("\"id\":\"N1\""));
            assert!(json.contains("\"label\":\"spot\""));
            assert!(json.contains("\"value\":100.0"));
            assert!(json.contains("\"is_sensitivity_target\":true"));
            assert!(json.contains("\"group\":\"input\""));
        }

        #[test]
        fn test_graph_edge_serialisation() {
            let edge = GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: Some(1.5),
            };

            let json = serde_json::to_string(&edge).unwrap();

            assert!(json.contains("\"source\":\"N1\""));
            assert!(json.contains("\"target\":\"N2\""));
            assert!(json.contains("\"weight\":1.5"));
        }

        #[test]
        fn test_node_type_serialisation_lowercase() {
            // Verify all node types serialise to lowercase
            let types = vec![
                (NodeType::Input, "\"input\""),
                (NodeType::Add, "\"add\""),
                (NodeType::Mul, "\"mul\""),
                (NodeType::Exp, "\"exp\""),
                (NodeType::Log, "\"log\""),
                (NodeType::Sqrt, "\"sqrt\""),
                (NodeType::Div, "\"div\""),
                (NodeType::Output, "\"output\""),
            ];

            for (node_type, expected) in types {
                let json = serde_json::to_string(&node_type).unwrap();
                assert_eq!(
                    json, expected,
                    "NodeType {:?} should serialise to {}",
                    node_type, expected
                );
            }
        }

        #[test]
        fn test_node_group_serialisation_lowercase() {
            let groups = vec![
                (NodeGroup::Input, "\"input\""),
                (NodeGroup::Intermediate, "\"intermediate\""),
                (NodeGroup::Output, "\"output\""),
                (NodeGroup::Sensitivity, "\"sensitivity\""),
            ];

            for (group, expected) in groups {
                let json = serde_json::to_string(&group).unwrap();
                assert_eq!(
                    json, expected,
                    "NodeGroup {:?} should serialise to {}",
                    group, expected
                );
            }
        }
    }

    // =========================================================================
    // Task 1.2: ComputationGraph and GraphMetadata Tests
    // =========================================================================

    mod graph_tests {
        use super::*;

        fn create_sample_graph() -> ComputationGraph {
            let nodes = vec![
                GraphNode {
                    id: "N1".to_string(),
                    node_type: NodeType::Input,
                    label: "spot".to_string(),
                    value: Some(100.0),
                    is_sensitivity_target: true,
                    group: NodeGroup::Input,
                    trade_ids: vec![],
                },
                GraphNode {
                    id: "N2".to_string(),
                    node_type: NodeType::Input,
                    label: "vol".to_string(),
                    value: Some(0.25),
                    is_sensitivity_target: true,
                    group: NodeGroup::Input,
                    trade_ids: vec![],
                },
                GraphNode {
                    id: "N3".to_string(),
                    node_type: NodeType::Mul,
                    label: "spot * vol".to_string(),
                    value: Some(25.0),
                    is_sensitivity_target: false,
                    group: NodeGroup::Intermediate,
                    trade_ids: vec![],
                },
                GraphNode {
                    id: "N4".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(10.5),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec![],
                },
            ];

            let edges = vec![
                GraphEdge {
                    source: "N1".to_string(),
                    target: "N3".to_string(),
                    weight: None,
                },
                GraphEdge {
                    source: "N2".to_string(),
                    target: "N3".to_string(),
                    weight: None,
                },
                GraphEdge {
                    source: "N3".to_string(),
                    target: "N4".to_string(),
                    weight: None,
                },
            ];

            let metadata = GraphMetadata {
                trade_id: Some("T001".to_string()),
                node_count: 4,
                edge_count: 3,
                depth: 3,
                generated_at: "2026-01-13T12:00:00Z".to_string(),
            };

            ComputationGraph {
                nodes,
                edges,
                metadata,
            }
        }

        #[test]
        fn test_computation_graph_creation() {
            let graph = create_sample_graph();

            assert_eq!(graph.nodes.len(), 4);
            assert_eq!(graph.edges.len(), 3);
            assert_eq!(graph.metadata.trade_id, Some("T001".to_string()));
            assert_eq!(graph.metadata.node_count, 4);
            assert_eq!(graph.metadata.edge_count, 3);
            assert_eq!(graph.metadata.depth, 3);
        }

        #[test]
        fn test_find_node_existing() {
            let graph = create_sample_graph();

            let node = graph.find_node("N1");
            assert!(node.is_some());
            assert_eq!(node.unwrap().label, "spot");
        }

        #[test]
        fn test_find_node_non_existing() {
            let graph = create_sample_graph();

            let node = graph.find_node("N999");
            assert!(node.is_none());
        }

        #[test]
        fn test_find_path_direct() {
            let graph = create_sample_graph();

            // N3 -> N4 is a direct path
            let path = graph.find_path("N3", "N4");
            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path.len(), 2);
            assert_eq!(path[0], "N3");
            assert_eq!(path[1], "N4");
        }

        #[test]
        fn test_find_path_multi_hop() {
            let graph = create_sample_graph();

            // N1 -> N3 -> N4
            let path = graph.find_path("N1", "N4");
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.len() >= 2);
            assert_eq!(path[0], "N1");
            assert_eq!(*path.last().unwrap(), "N4");
        }

        #[test]
        fn test_find_path_no_path() {
            let graph = create_sample_graph();

            // N4 -> N1 should have no path (reverse direction)
            let path = graph.find_path("N4", "N1");
            assert!(path.is_none());
        }

        #[test]
        fn test_get_critical_path() {
            let graph = create_sample_graph();

            let critical_path = graph.get_critical_path();
            // Critical path should be the longest path through the graph
            assert!(!critical_path.is_empty());
            // Should include at least input -> intermediate -> output
            assert!(critical_path.len() >= 3);
        }

        #[test]
        fn test_graph_metadata_iso8601_format() {
            let metadata = GraphMetadata {
                trade_id: Some("T001".to_string()),
                node_count: 10,
                edge_count: 15,
                depth: 5,
                generated_at: "2026-01-13T12:00:00Z".to_string(),
            };

            // Verify ISO 8601 format
            assert!(metadata.generated_at.contains("T"));
            assert!(metadata.generated_at.ends_with("Z") || metadata.generated_at.contains("+"));
        }
    }

    #[cfg(feature = "serde")]
    mod graph_serialisation_tests {
        use super::*;

        #[test]
        fn test_computation_graph_serialisation() {
            let nodes = vec![GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec![],
            }];

            let edges = vec![GraphEdge {
                source: "N1".to_string(),
                target: "N2".to_string(),
                weight: None,
            }];

            let metadata = GraphMetadata {
                trade_id: Some("T001".to_string()),
                node_count: 1,
                edge_count: 1,
                depth: 2,
                generated_at: "2026-01-13T12:00:00Z".to_string(),
            };

            let graph = ComputationGraph {
                nodes,
                edges,
                metadata,
            };

            let json = serde_json::to_string(&graph).unwrap();

            // Verify D3.js compatible: edges -> links
            assert!(json.contains("\"links\":"));
            assert!(!json.contains("\"edges\":"));

            // Verify nodes array
            assert!(json.contains("\"nodes\":"));

            // Verify metadata
            assert!(json.contains("\"metadata\":"));
            assert!(json.contains("\"trade_id\":\"T001\""));
        }

        #[test]
        fn test_graph_metadata_serialisation() {
            let metadata = GraphMetadata {
                trade_id: Some("T001".to_string()),
                node_count: 150,
                edge_count: 200,
                depth: 12,
                generated_at: "2026-01-13T12:00:00Z".to_string(),
            };

            let json = serde_json::to_string(&metadata).unwrap();

            assert!(json.contains("\"trade_id\":\"T001\""));
            assert!(json.contains("\"node_count\":150"));
            assert!(json.contains("\"edge_count\":200"));
            assert!(json.contains("\"depth\":12"));
            assert!(json.contains("\"generated_at\":\"2026-01-13T12:00:00Z\""));
        }
    }

    // =========================================================================
    // Task 1.3: GraphError Tests
    // =========================================================================

    mod error_tests {
        use super::*;

        #[test]
        fn test_trade_not_found_error() {
            let error = GraphError::TradeNotFound("T001".to_string());

            match error {
                GraphError::TradeNotFound(trade_id) => {
                    assert_eq!(trade_id, "T001");
                }
                _ => panic!("Expected TradeNotFound variant"),
            }
        }

        #[test]
        fn test_extraction_failed_error() {
            let error = GraphError::ExtractionFailed("Some reason".to_string());

            match error {
                GraphError::ExtractionFailed(reason) => {
                    assert_eq!(reason, "Some reason");
                }
                _ => panic!("Expected ExtractionFailed variant"),
            }
        }

        #[test]
        fn test_timeout_error() {
            let error = GraphError::Timeout;

            assert!(matches!(error, GraphError::Timeout));
        }

        #[test]
        fn test_error_http_status_codes() {
            // TradeNotFound -> 404
            let not_found = GraphError::TradeNotFound("T001".to_string());
            assert_eq!(not_found.http_status_code(), 404);

            // ExtractionFailed -> 500
            let extraction_failed = GraphError::ExtractionFailed("reason".to_string());
            assert_eq!(extraction_failed.http_status_code(), 500);

            // Timeout -> 500
            let timeout = GraphError::Timeout;
            assert_eq!(timeout.http_status_code(), 500);
        }

        #[test]
        fn test_error_clone() {
            let error = GraphError::TradeNotFound("T001".to_string());
            let cloned = error.clone();

            match cloned {
                GraphError::TradeNotFound(trade_id) => {
                    assert_eq!(trade_id, "T001");
                }
                _ => panic!("Clone should preserve variant"),
            }
        }

        #[test]
        fn test_error_debug() {
            let error = GraphError::TradeNotFound("T001".to_string());
            let debug_str = format!("{:?}", error);
            assert!(debug_str.contains("TradeNotFound"));
            assert!(debug_str.contains("T001"));
        }
    }

    #[cfg(feature = "serde")]
    mod error_serialisation_tests {
        use super::*;

        #[test]
        fn test_error_serialisation() {
            let error = GraphError::TradeNotFound("T001".to_string());
            let json = serde_json::to_string(&error).unwrap();

            // Verify it can be serialised for error responses
            assert!(!json.is_empty());
        }
    }

    // =========================================================================
    // Task 2.1 (partial): GraphNodeUpdate Tests
    // =========================================================================

    mod node_update_tests {
        use super::*;

        #[test]
        fn test_graph_node_update_creation() {
            let update = GraphNodeUpdate {
                id: "N1".to_string(),
                value: 105.0,
                delta: Some(5.0),
            };

            assert_eq!(update.id, "N1");
            assert_eq!(update.value, 105.0);
            assert_eq!(update.delta, Some(5.0));
        }

        #[test]
        fn test_graph_node_update_without_delta() {
            let update = GraphNodeUpdate {
                id: "N1".to_string(),
                value: 105.0,
                delta: None,
            };

            assert!(update.delta.is_none());
        }
    }

    // =========================================================================
    // Portfolio Graph Types Tests
    // =========================================================================

    mod portfolio_graph_tests {
        use super::*;

        #[test]
        fn test_portfolio_graph_metadata_creation() {
            let metadata = PortfolioGraphMetadata {
                node_count: 150,
                edge_count: 200,
                depth: 12,
                generated_at: "2026-01-19T12:00:00Z".to_string(),
                trade_count: 10,
                shared_node_count: 25,
                optimisation_ratio: 0.83,
            };

            assert_eq!(metadata.node_count, 150);
            assert_eq!(metadata.edge_count, 200);
            assert_eq!(metadata.depth, 12);
            assert_eq!(metadata.trade_count, 10);
            assert_eq!(metadata.shared_node_count, 25);
            assert!((metadata.optimisation_ratio - 0.83).abs() < 1e-10);
        }

        #[test]
        fn test_portfolio_computation_graph_creation() {
            let nodes = vec![
                GraphNode {
                    id: "spot_USD".to_string(),
                    node_type: NodeType::Input,
                    label: "spot".to_string(),
                    value: Some(100.0),
                    is_sensitivity_target: true,
                    group: NodeGroup::Input,
                    trade_ids: vec!["T001".to_string(), "T002".to_string()],
                },
                GraphNode {
                    id: "T001_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(10.5),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T001".to_string()],
                },
                GraphNode {
                    id: "T002_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(15.0),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T002".to_string()],
                },
            ];

            let edges = vec![
                GraphEdge {
                    source: "spot_USD".to_string(),
                    target: "T001_price".to_string(),
                    weight: None,
                },
                GraphEdge {
                    source: "spot_USD".to_string(),
                    target: "T002_price".to_string(),
                    weight: None,
                },
            ];

            let metadata = PortfolioGraphMetadata {
                node_count: 3,
                edge_count: 2,
                depth: 2,
                generated_at: "2026-01-19T12:00:00Z".to_string(),
                trade_count: 2,
                shared_node_count: 1,
                optimisation_ratio: 0.75,
            };

            let graph = PortfolioComputationGraph {
                nodes,
                edges,
                metadata,
            };

            assert_eq!(graph.nodes.len(), 3);
            assert_eq!(graph.edges.len(), 2);
            assert_eq!(graph.metadata.trade_count, 2);
        }

        #[test]
        fn test_portfolio_graph_find_node() {
            let nodes = vec![GraphNode {
                id: "spot_USD".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec!["T001".to_string()],
            }];

            let graph = PortfolioComputationGraph {
                nodes,
                edges: vec![],
                metadata: PortfolioGraphMetadata {
                    node_count: 1,
                    edge_count: 0,
                    depth: 1,
                    generated_at: "2026-01-19T12:00:00Z".to_string(),
                    trade_count: 1,
                    shared_node_count: 0,
                    optimisation_ratio: 1.0,
                },
            };

            let node = graph.find_node("spot_USD");
            assert!(node.is_some());
            assert_eq!(node.unwrap().label, "spot");

            let not_found = graph.find_node("nonexistent");
            assert!(not_found.is_none());
        }

        #[test]
        fn test_portfolio_graph_nodes_for_trade() {
            let nodes = vec![
                GraphNode {
                    id: "spot_USD".to_string(),
                    node_type: NodeType::Input,
                    label: "spot".to_string(),
                    value: Some(100.0),
                    is_sensitivity_target: true,
                    group: NodeGroup::Input,
                    trade_ids: vec!["T001".to_string(), "T002".to_string()],
                },
                GraphNode {
                    id: "T001_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(10.5),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T001".to_string()],
                },
                GraphNode {
                    id: "T002_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(15.0),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T002".to_string()],
                },
            ];

            let graph = PortfolioComputationGraph {
                nodes,
                edges: vec![],
                metadata: PortfolioGraphMetadata {
                    node_count: 3,
                    edge_count: 0,
                    depth: 1,
                    generated_at: "2026-01-19T12:00:00Z".to_string(),
                    trade_count: 2,
                    shared_node_count: 1,
                    optimisation_ratio: 0.75,
                },
            };

            // T001 should have 2 nodes (spot_USD shared + T001_price)
            let t001_nodes = graph.nodes_for_trade("T001");
            assert_eq!(t001_nodes.len(), 2);

            // T002 should have 2 nodes (spot_USD shared + T002_price)
            let t002_nodes = graph.nodes_for_trade("T002");
            assert_eq!(t002_nodes.len(), 2);

            // T003 should have 0 nodes
            let t003_nodes = graph.nodes_for_trade("T003");
            assert_eq!(t003_nodes.len(), 0);
        }

        #[test]
        fn test_portfolio_graph_shared_nodes() {
            let nodes = vec![
                GraphNode {
                    id: "spot_USD".to_string(),
                    node_type: NodeType::Input,
                    label: "spot".to_string(),
                    value: Some(100.0),
                    is_sensitivity_target: true,
                    group: NodeGroup::Input,
                    trade_ids: vec!["T001".to_string(), "T002".to_string()],
                },
                GraphNode {
                    id: "T001_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(10.5),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T001".to_string()],
                },
            ];

            let graph = PortfolioComputationGraph {
                nodes,
                edges: vec![],
                metadata: PortfolioGraphMetadata {
                    node_count: 2,
                    edge_count: 0,
                    depth: 1,
                    generated_at: "2026-01-19T12:00:00Z".to_string(),
                    trade_count: 2,
                    shared_node_count: 1,
                    optimisation_ratio: 0.67,
                },
            };

            let shared = graph.shared_nodes();
            assert_eq!(shared.len(), 1);
            assert_eq!(shared[0].id, "spot_USD");
        }

        #[test]
        fn test_portfolio_graph_no_shared_nodes() {
            let nodes = vec![
                GraphNode {
                    id: "T001_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(10.5),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T001".to_string()],
                },
                GraphNode {
                    id: "T002_price".to_string(),
                    node_type: NodeType::Output,
                    label: "price".to_string(),
                    value: Some(15.0),
                    is_sensitivity_target: false,
                    group: NodeGroup::Output,
                    trade_ids: vec!["T002".to_string()],
                },
            ];

            let graph = PortfolioComputationGraph {
                nodes,
                edges: vec![],
                metadata: PortfolioGraphMetadata {
                    node_count: 2,
                    edge_count: 0,
                    depth: 1,
                    generated_at: "2026-01-19T12:00:00Z".to_string(),
                    trade_count: 2,
                    shared_node_count: 0,
                    optimisation_ratio: 1.0,
                },
            };

            let shared = graph.shared_nodes();
            assert_eq!(shared.len(), 0);
        }
    }

    mod graph_node_trade_ids_tests {
        use super::*;

        #[test]
        fn test_graph_node_with_trade_ids() {
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec!["T001".to_string(), "T002".to_string()],
            };

            assert_eq!(node.trade_ids.len(), 2);
            assert!(node.trade_ids.contains(&"T001".to_string()));
            assert!(node.trade_ids.contains(&"T002".to_string()));
        }

        #[test]
        fn test_graph_node_empty_trade_ids() {
            let node = GraphNode {
                id: "N1".to_string(),
                node_type: NodeType::Input,
                label: "spot".to_string(),
                value: Some(100.0),
                is_sensitivity_target: true,
                group: NodeGroup::Input,
                trade_ids: vec![],
            };

            assert!(node.trade_ids.is_empty());
        }

        #[test]
        fn test_graph_node_default() {
            let node = GraphNode::default();

            assert!(node.id.is_empty());
            assert_eq!(node.node_type, NodeType::Input);
            assert!(node.label.is_empty());
            assert!(node.value.is_none());
            assert!(!node.is_sensitivity_target);
            assert_eq!(node.group, NodeGroup::Intermediate);
            assert!(node.trade_ids.is_empty());
        }
    }
}

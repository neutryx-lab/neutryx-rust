#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::similar_names)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

//! # Computation Graph Visualisation
//!
//! Provides data types, builders and extractors for pricing computation
//! graph DAGs with serialisation support for D3.js visualisation.

mod error;
mod extractor;
mod types;

pub use error::GraphError;
pub use extractor::{
    GraphBuilder, GraphExtractable, PortfolioGraphExtractable, PortfolioGraphExtractor,
    SimpleGraphExtractor,
};
pub use types::{
    ComputationGraph, GraphEdge, GraphMetadata, GraphNode, GraphNodeUpdate, NodeGroup, NodeType,
    PortfolioComputationGraph, PortfolioGraphMetadata,
};

#[cfg(test)]
mod tests {
    use super::*;

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

    mod graph_node_default_tests {
        use super::*;

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

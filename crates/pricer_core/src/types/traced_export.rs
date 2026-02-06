//! # Execution Trace Export
//!
//! Provides functionality to export `ExecutionTrace` to D3.js-compatible
//! graph formats for visualisation.
//!
//! ## Feature Gate
//!
//! This module is only available when the `execution-trace` feature is enabled.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pricer_core::types::traced::{ExecutionTrace, set_trace_context, clear_trace_context};
//! use pricer_core::types::TracedFloat;
//! use pricer_core::types::traced_export::{export_graph, D3Graph};
//!
//! // Set up and compute
//! let trace = Rc::new(RefCell::new(ExecutionTrace::new()));
//! set_trace_context(Rc::clone(&trace));
//! let x = TracedFloat::input(100.0, "spot");
//! let y = TracedFloat::input(0.2, "vol");
//! let _z = x * y;
//! clear_trace_context();
//!
//! // Export to D3 format
//! let graph = export_graph(&trace.borrow(), DetailLevel::Operation);
//! let json = serde_json::to_string(&graph).unwrap();
//! ```

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::Serialize;

use super::traced::{DetailLevel, ExecutionTrace, NodeId, Operation, ScopeId};

// =============================================================================
// D3-Compatible Graph Types
// =============================================================================

/// D3.js-compatible node type enumeration.
///
/// This maps to lowercase strings for D3 visualisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum D3NodeType {
    /// Input variable (market data, parameters)
    Input,
    /// Constant value
    Constant,
    /// Addition operation
    Add,
    /// Subtraction operation
    Sub,
    /// Multiplication operation
    Mul,
    /// Division operation
    Div,
    /// Exponential function
    Exp,
    /// Natural logarithm
    Log,
    /// Square root
    Sqrt,
    /// Power function
    Pow,
    /// Trigonometric function
    Trig,
    /// Other operation
    Other,
    /// Output node
    Output,
    /// Aggregated scope (for scope-level detail)
    Scope,
}

impl From<Operation> for D3NodeType {
    fn from(op: Operation) -> Self {
        match op {
            Operation::Input => D3NodeType::Input,
            Operation::Constant => D3NodeType::Constant,
            Operation::Add => D3NodeType::Add,
            Operation::Sub => D3NodeType::Sub,
            Operation::Mul => D3NodeType::Mul,
            Operation::Div => D3NodeType::Div,
            Operation::Neg => D3NodeType::Sub, // Negation is similar to subtraction
            Operation::Rem => D3NodeType::Div, // Remainder is similar to division
            Operation::Sqrt => D3NodeType::Sqrt,
            Operation::Exp | Operation::Exp2 | Operation::ExpM1 => D3NodeType::Exp,
            Operation::Ln | Operation::Log2 | Operation::Log10 | Operation::Ln1p => D3NodeType::Log,
            Operation::Powf | Operation::Powi => D3NodeType::Pow,
            Operation::Sin
            | Operation::Cos
            | Operation::Tan
            | Operation::Asin
            | Operation::Acos
            | Operation::Atan
            | Operation::Atan2
            | Operation::Sinh
            | Operation::Cosh
            | Operation::Tanh
            | Operation::Asinh
            | Operation::Acosh
            | Operation::Atanh => D3NodeType::Trig,
            _ => D3NodeType::Other,
        }
    }
}

/// D3.js-compatible node group for colour coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum D3NodeGroup {
    /// Input nodes (blue)
    Input,
    /// Intermediate nodes (grey)
    Intermediate,
    /// Output nodes (green)
    Output,
    /// Scope nodes (purple)
    Scope,
}

/// A node in the D3-compatible graph.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct D3Node {
    /// Unique identifier
    pub id: String,

    /// Node type (serialised as "type" for D3.js)
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub node_type: D3NodeType,

    /// Human-readable label
    pub label: String,

    /// Computed value
    pub value: Option<f64>,

    /// Visual grouping for colour coding
    pub group: D3NodeGroup,

    /// Scope name this node belongs to (if any)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scope: Option<String>,

    /// Source location information
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub source: Option<String>,
}

/// An edge in the D3-compatible graph.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct D3Edge {
    /// Source node ID
    pub source: String,

    /// Target node ID
    pub target: String,
}

/// D3-compatible graph metadata.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct D3Metadata {
    /// Total number of nodes
    pub node_count: usize,

    /// Total number of edges
    pub edge_count: usize,

    /// Number of scopes
    pub scope_count: usize,

    /// Detail level used for export
    pub detail_level: DetailLevel,

    /// ISO 8601 timestamp
    pub generated_at: String,
}

/// D3.js-compatible computation graph.
///
/// This structure serialises to JSON format compatible with D3.js
/// force-directed graph visualisation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct D3Graph {
    /// All nodes in the graph
    pub nodes: Vec<D3Node>,

    /// All edges (serialised as "links" for D3.js)
    #[cfg_attr(feature = "serde", serde(rename = "links"))]
    pub edges: Vec<D3Edge>,

    /// Graph metadata
    pub metadata: D3Metadata,
}

// =============================================================================
// Export Functions
// =============================================================================

/// Exports an ExecutionTrace to a D3-compatible graph.
///
/// # Arguments
///
/// * `trace` - The execution trace to export
/// * `detail_level` - Level of detail for the export
///
/// # Returns
///
/// A D3-compatible graph structure.
#[must_use]
pub fn export_graph(trace: &ExecutionTrace, detail_level: DetailLevel) -> D3Graph {
    match detail_level {
        DetailLevel::Operation => export_operation_level(trace),
        DetailLevel::Scope => export_scope_level(trace),
    }
}

/// Export at operation level (most detailed).
fn export_operation_level(trace: &ExecutionTrace) -> D3Graph {
    // Build scope name lookup
    let scope_names: HashMap<ScopeId, &str> = trace
        .scopes()
        .iter()
        .map(|s| (s.id, s.name.as_str()))
        .collect();

    // Find output nodes (nodes with no outgoing edges)
    let nodes_with_outgoing: std::collections::HashSet<NodeId> =
        trace.edges().iter().map(|e| e.source).collect();

    // Convert nodes
    let nodes: Vec<D3Node> = trace
        .nodes()
        .iter()
        .map(|node| {
            let node_type = D3NodeType::from(node.operation);
            let is_output = !nodes_with_outgoing.contains(&node.id);

            let group = match node.operation {
                Operation::Input => D3NodeGroup::Input,
                Operation::Constant if node.input_ids.is_empty() => D3NodeGroup::Input,
                _ if is_output => D3NodeGroup::Output,
                _ => D3NodeGroup::Intermediate,
            };

            let label = node
                .label
                .clone()
                .unwrap_or_else(|| format!("{:?}", node.operation));

            D3Node {
                id: format!("N{}", node.id.value()),
                node_type,
                label,
                value: Some(node.value),
                group,
                scope: node
                    .scope_id
                    .and_then(|sid| scope_names.get(&sid).map(|s| (*s).to_string())),
                source: Some(node.source_location.to_string()),
            }
        })
        .collect();

    // Convert edges
    let edges: Vec<D3Edge> = trace
        .edges()
        .iter()
        .map(|edge| D3Edge {
            source: format!("N{}", edge.source.value()),
            target: format!("N{}", edge.target.value()),
        })
        .collect();

    // Generate timestamp
    let generated_at = chrono::Utc::now().to_rfc3339();

    D3Graph {
        nodes,
        edges,
        metadata: D3Metadata {
            node_count: trace.node_count(),
            edge_count: trace.edge_count(),
            scope_count: trace.scope_count(),
            detail_level: DetailLevel::Operation,
            generated_at,
        },
    }
}

/// Export at scope level (aggregated).
fn export_scope_level(trace: &ExecutionTrace) -> D3Graph {
    // If no scopes, fall back to operation level
    if trace.scope_count() == 0 {
        return export_operation_level(trace);
    }

    // Build scope hierarchy and membership
    let mut scope_nodes: HashMap<Option<ScopeId>, Vec<NodeId>> = HashMap::new();
    for node in trace.nodes() {
        scope_nodes.entry(node.scope_id).or_default().push(node.id);
    }

    // Create scope-level nodes
    let mut nodes: Vec<D3Node> = Vec::new();

    // Add a node for each scope
    for scope in trace.scopes() {
        let node_ids = scope_nodes
            .get(&Some(scope.id))
            .cloned()
            .unwrap_or_default();
        let scope_nodes_data: Vec<_> = node_ids
            .iter()
            .filter_map(|id| trace.nodes().iter().find(|n| n.id == *id))
            .collect();

        // Aggregate value (sum of all values in scope)
        let total_value: f64 = scope_nodes_data.iter().map(|n| n.value).sum();

        // Determine group based on contained operations
        let has_inputs = scope_nodes_data
            .iter()
            .any(|n| matches!(n.operation, Operation::Input));
        let group = if has_inputs {
            D3NodeGroup::Input
        } else {
            D3NodeGroup::Scope
        };

        nodes.push(D3Node {
            id: format!("S{}", scope.id.value()),
            node_type: D3NodeType::Scope,
            label: scope.name.clone(),
            value: Some(total_value),
            group,
            scope: scope.parent_id.and_then(|pid| {
                trace
                    .scopes()
                    .iter()
                    .find(|s| s.id == pid)
                    .map(|s| s.name.clone())
            }),
            source: None,
        });
    }

    // Add top-level nodes (nodes without scope)
    let top_level_nodes = scope_nodes.get(&None).cloned().unwrap_or_default();
    for node_id in &top_level_nodes {
        if let Some(node) = trace.nodes().iter().find(|n| n.id == *node_id) {
            let label = node
                .label
                .clone()
                .unwrap_or_else(|| format!("{:?}", node.operation));
            nodes.push(D3Node {
                id: format!("N{}", node.id.value()),
                node_type: D3NodeType::from(node.operation),
                label,
                value: Some(node.value),
                group: match node.operation {
                    Operation::Input | Operation::Constant => D3NodeGroup::Input,
                    _ => D3NodeGroup::Intermediate,
                },
                scope: None,
                source: Some(node.source_location.to_string()),
            });
        }
    }

    // Build edges between scopes
    let mut edges: Vec<D3Edge> = Vec::new();
    let mut seen_edges: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    for edge in trace.edges() {
        // Find scope for source and target nodes
        let source_node = trace.nodes().iter().find(|n| n.id == edge.source);
        let target_node = trace.nodes().iter().find(|n| n.id == edge.target);

        if let (Some(src), Some(tgt)) = (source_node, target_node) {
            let source_id = match src.scope_id {
                Some(sid) => format!("S{}", sid.value()),
                None => format!("N{}", src.id.value()),
            };
            let target_id = match tgt.scope_id {
                Some(sid) => format!("S{}", sid.value()),
                None => format!("N{}", tgt.id.value()),
            };

            // Skip self-loops and duplicates
            if source_id != target_id
                && !seen_edges.contains(&(source_id.clone(), target_id.clone()))
            {
                seen_edges.insert((source_id.clone(), target_id.clone()));
                edges.push(D3Edge {
                    source: source_id,
                    target: target_id,
                });
            }
        }
    }

    let generated_at = chrono::Utc::now().to_rfc3339();
    let node_count = nodes.len();
    let edge_count = edges.len();

    D3Graph {
        nodes,
        edges,
        metadata: D3Metadata {
            node_count,
            edge_count,
            scope_count: trace.scope_count(),
            detail_level: DetailLevel::Scope,
            generated_at,
        },
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use num_traits::Float;

    use super::*;
    use crate::types::{
        traced::{clear_trace_context, set_trace_context, ExecutionTrace, SourceLocation},
        TracedFloat,
    };

    fn setup_trace() -> Rc<RefCell<ExecutionTrace>> {
        let trace = Rc::new(RefCell::new(ExecutionTrace::new()));
        set_trace_context(Rc::clone(&trace));
        trace
    }

    fn teardown() { clear_trace_context(); }

    mod d3_node_type_tests {
        use super::*;

        #[test]
        fn test_from_operation() {
            assert_eq!(D3NodeType::from(Operation::Input), D3NodeType::Input);
            assert_eq!(D3NodeType::from(Operation::Add), D3NodeType::Add);
            assert_eq!(D3NodeType::from(Operation::Mul), D3NodeType::Mul);
            assert_eq!(D3NodeType::from(Operation::Sqrt), D3NodeType::Sqrt);
            assert_eq!(D3NodeType::from(Operation::Exp), D3NodeType::Exp);
            assert_eq!(D3NodeType::from(Operation::Ln), D3NodeType::Log);
            assert_eq!(D3NodeType::from(Operation::Sin), D3NodeType::Trig);
        }
    }

    mod export_tests {
        use super::*;

        #[test]
        fn test_export_simple_graph() {
            let trace = setup_trace();

            let a = TracedFloat::input(10.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let _c = a + b;

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);

            assert_eq!(graph.nodes.len(), 3);
            assert_eq!(graph.edges.len(), 2);
            assert_eq!(graph.metadata.node_count, 3);
            assert_eq!(graph.metadata.edge_count, 2);

            // Check node types
            assert_eq!(graph.nodes[0].node_type, D3NodeType::Input);
            assert_eq!(graph.nodes[1].node_type, D3NodeType::Input);
            assert_eq!(graph.nodes[2].node_type, D3NodeType::Add);

            // Check labels
            assert_eq!(graph.nodes[0].label, "a");
            assert_eq!(graph.nodes[1].label, "b");

            teardown();
        }

        #[test]
        fn test_export_with_values() {
            let trace = setup_trace();

            let x = TracedFloat::input(100.0, "x");
            let _y = x.sqrt();

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);

            assert_eq!(graph.nodes.len(), 2);
            assert_eq!(graph.nodes[0].value, Some(100.0));
            assert_eq!(graph.nodes[1].value, Some(10.0));

            teardown();
        }

        #[test]
        fn test_node_groups() {
            let trace = setup_trace();

            let a = TracedFloat::input(10.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let c = a + b;
            let _d = c * TracedFloat::input(2.0, "scale");

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);

            // Check groups
            assert_eq!(graph.nodes[0].group, D3NodeGroup::Input); // a
            assert_eq!(graph.nodes[1].group, D3NodeGroup::Input); // b
                                                                  // c is intermediate
                                                                  // d is output (no outgoing edges)

            teardown();
        }

        #[test]
        fn test_edge_structure() {
            let trace = setup_trace();

            let a = TracedFloat::input(1.0, "a");
            let b = TracedFloat::input(2.0, "b");
            let _c = a + b;

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);

            // Edges should connect inputs to add node
            assert_eq!(graph.edges[0].source, "N0");
            assert_eq!(graph.edges[0].target, "N2");
            assert_eq!(graph.edges[1].source, "N1");
            assert_eq!(graph.edges[1].target, "N2");

            teardown();
        }
    }

    mod scope_export_tests {
        use super::*;

        #[test]
        fn test_export_with_scope() {
            let trace = setup_trace();

            // Enter a scope
            trace.borrow_mut().enter_scope("calculate");
            let a = TracedFloat::input(10.0, "a");
            let b = TracedFloat::input(5.0, "b");
            let _c = a + b;
            trace.borrow_mut().exit_scope();

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);

            // All nodes should have scope = "calculate"
            assert!(graph
                .nodes
                .iter()
                .all(|n| n.scope == Some("calculate".to_string())));

            teardown();
        }

        #[test]
        fn test_scope_level_export() {
            let trace = setup_trace();

            trace.borrow_mut().enter_scope("calc");
            let a = TracedFloat::input(10.0, "a");
            let _b = a.sqrt();
            trace.borrow_mut().exit_scope();

            let graph = export_graph(&trace.borrow(), DetailLevel::Scope);

            // Should have a single scope node
            assert_eq!(graph.nodes.len(), 1);
            assert_eq!(graph.nodes[0].node_type, D3NodeType::Scope);
            assert_eq!(graph.nodes[0].label, "calc");

            teardown();
        }

        #[test]
        fn test_scope_level_empty_scope() {
            let trace = setup_trace();

            // No scopes, should fall back to operation level
            let a = TracedFloat::input(10.0, "a");
            let _b = a + a;

            let graph = export_graph(&trace.borrow(), DetailLevel::Scope);

            // Should export at operation level since no scopes exist
            assert_eq!(graph.nodes.len(), 2);

            teardown();
        }
    }

    #[cfg(feature = "serde")]
    mod serialisation_tests {
        use super::*;

        #[test]
        fn test_d3_graph_serialisation() {
            let trace = setup_trace();

            let a = TracedFloat::input(10.0, "spot");
            let b = TracedFloat::input(0.2, "vol");
            let _c = a * b;

            let graph = export_graph(&trace.borrow(), DetailLevel::Operation);
            let json = serde_json::to_string(&graph).unwrap();

            // Check D3-compatible field names
            assert!(json.contains("\"links\":")); // edges -> links
            assert!(json.contains("\"type\":")); // node_type -> type
            assert!(json.contains("\"nodes\":"));
            assert!(json.contains("\"metadata\":"));

            teardown();
        }

        #[test]
        fn test_node_type_serialisation() {
            let json = serde_json::to_string(&D3NodeType::Input).unwrap();
            assert_eq!(json, "\"input\"");

            let json = serde_json::to_string(&D3NodeType::Mul).unwrap();
            assert_eq!(json, "\"mul\"");
        }
    }
}

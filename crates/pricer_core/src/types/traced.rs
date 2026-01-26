//! # Execution Trace Types
//!
//! Core types for computation graph tracing and visualisation.
//! These types enable automatic extraction of computation graphs from
//! pricing calculations using `TracedFloat`.
//!
//! ## Feature Gate
//!
//! This module is only available when the `execution-trace` feature is enabled.
//!
//! ## Overview
//!
//! - `NodeId`: Unique identifier for graph nodes
//! - `ScopeId`: Unique identifier for scopes (logical groups)
//! - `Operation`: Type of mathematical operation performed
//! - `SourceLocation`: Source code location (file, line, column)
//! - `TraceNode`: A node in the computation graph
//! - `TraceEdge`: An edge connecting two nodes
//! - `Scope`: A logical scope for grouping nodes
//! - `DetailLevel`: Level of detail for graph export

use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;

#[cfg(feature = "serde")]
use serde::Serialize;

// =============================================================================
// NodeId and ScopeId Newtypes
// =============================================================================

/// Unique identifier for a node in the computation graph.
///
/// Nodes are assigned sequential IDs starting from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct NodeId(pub u64);

impl NodeId {
    /// Creates a new NodeId.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the inner ID value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N{}", self.0)
    }
}

/// Unique identifier for a scope in the computation graph.
///
/// Scopes are assigned sequential IDs starting from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ScopeId(pub u64);

impl ScopeId {
    /// Creates a new ScopeId.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the inner ID value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S{}", self.0)
    }
}

// =============================================================================
// Operation Enumeration
// =============================================================================

/// Operation type performed by a node in the computation graph.
///
/// Represents mathematical operations tracked during execution tracing.
/// This is more granular than `NodeType` in the graph module, covering
/// all operations supported by `num_traits::Float`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Operation {
    /// Input value (market data, model parameters)
    Input,
    /// Constant value
    Constant,
    /// Addition (a + b)
    Add,
    /// Subtraction (a - b)
    Sub,
    /// Multiplication (a * b)
    Mul,
    /// Division (a / b)
    Div,
    /// Negation (-a)
    Neg,
    /// Remainder (a % b)
    Rem,
    /// Square root
    Sqrt,
    /// Exponential (e^x)
    Exp,
    /// Exponential (2^x)
    Exp2,
    /// Exponential minus 1 (e^x - 1)
    ExpM1,
    /// Natural logarithm (ln(x))
    Ln,
    /// Logarithm base 2
    Log2,
    /// Logarithm base 10
    Log10,
    /// Natural log plus 1 (ln(x + 1))
    Ln1p,
    /// Power (x^y)
    Powf,
    /// Integer power (x^n)
    Powi,
    /// Sine
    Sin,
    /// Cosine
    Cos,
    /// Tangent
    Tan,
    /// Arcsine
    Asin,
    /// Arccosine
    Acos,
    /// Arctangent
    Atan,
    /// Two-argument arctangent (atan2)
    Atan2,
    /// Hyperbolic sine
    Sinh,
    /// Hyperbolic cosine
    Cosh,
    /// Hyperbolic tangent
    Tanh,
    /// Inverse hyperbolic sine
    Asinh,
    /// Inverse hyperbolic cosine
    Acosh,
    /// Inverse hyperbolic tangent
    Atanh,
    /// Absolute value
    Abs,
    /// Sign function (signum)
    Signum,
    /// Floor function
    Floor,
    /// Ceiling function
    Ceil,
    /// Round function
    Round,
    /// Truncate function
    Trunc,
    /// Fractional part
    Fract,
    /// Reciprocal (1/x)
    Recip,
    /// Maximum of two values
    Max,
    /// Minimum of two values
    Min,
    /// Absolute difference
    AbsDiffEq,
    /// Fused multiply-add (a * b + c)
    MulAdd,
    /// Hypot (sqrt(x^2 + y^2))
    Hypot,
    /// Copy sign
    Copysign,
}

impl Operation {
    /// Returns the number of input arguments for this operation.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Input | Self::Constant => 0,
            Self::Neg
            | Self::Sqrt
            | Self::Exp
            | Self::Exp2
            | Self::ExpM1
            | Self::Ln
            | Self::Log2
            | Self::Log10
            | Self::Ln1p
            | Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::Asin
            | Self::Acos
            | Self::Atan
            | Self::Sinh
            | Self::Cosh
            | Self::Tanh
            | Self::Asinh
            | Self::Acosh
            | Self::Atanh
            | Self::Abs
            | Self::Signum
            | Self::Floor
            | Self::Ceil
            | Self::Round
            | Self::Trunc
            | Self::Fract
            | Self::Recip => 1,
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::Rem
            | Self::Powf
            | Self::Powi
            | Self::Atan2
            | Self::Max
            | Self::Min
            | Self::AbsDiffEq
            | Self::Hypot
            | Self::Copysign => 2,
            Self::MulAdd => 3,
        }
    }
}

// =============================================================================
// SourceLocation
// =============================================================================

/// Source code location where an operation was performed.
///
/// Captured using `#[track_caller]` attribute for automatic location tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SourceLocation {
    /// File path
    pub file: &'static str,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

impl SourceLocation {
    /// Creates a new SourceLocation from a `std::panic::Location`.
    #[must_use]
    pub fn from_location(location: &'static Location<'static>) -> Self {
        Self {
            file: location.file(),
            line: location.line(),
            column: location.column(),
        }
    }

    /// Creates a new SourceLocation with explicit values.
    #[must_use]
    pub const fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

// =============================================================================
// TraceNode
// =============================================================================

/// A node in the execution trace computation graph.
///
/// Represents a single computation step with its operation, value,
/// source location, and scope membership.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TraceNode {
    /// Unique identifier for this node
    pub id: NodeId,
    /// Operation type performed
    pub operation: Operation,
    /// Computed value at this node
    pub value: f64,
    /// Source code location where this operation occurred
    pub source_location: SourceLocation,
    /// Scope this node belongs to (None if top-level)
    pub scope_id: Option<ScopeId>,
    /// IDs of input nodes (dependencies)
    pub input_ids: Vec<NodeId>,
    /// Optional label for input nodes (e.g., "spot", "vol")
    pub label: Option<String>,
}

impl TraceNode {
    /// Creates a new input node with a label.
    #[must_use]
    pub fn input(id: NodeId, value: f64, label: &str, location: SourceLocation) -> Self {
        Self {
            id,
            operation: Operation::Input,
            value,
            source_location: location,
            scope_id: None,
            input_ids: Vec::new(),
            label: Some(label.to_string()),
        }
    }

    /// Creates a new operation node.
    #[must_use]
    pub fn operation(
        id: NodeId,
        op: Operation,
        value: f64,
        location: SourceLocation,
        input_ids: Vec<NodeId>,
    ) -> Self {
        Self {
            id,
            operation: op,
            value,
            source_location: location,
            scope_id: None,
            input_ids,
            label: None,
        }
    }

    /// Sets the scope ID for this node.
    pub fn with_scope(mut self, scope_id: ScopeId) -> Self {
        self.scope_id = Some(scope_id);
        self
    }
}

// =============================================================================
// TraceEdge
// =============================================================================

/// An edge in the execution trace computation graph.
///
/// Represents a data dependency between two nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TraceEdge {
    /// Source node ID (input to the operation)
    pub source: NodeId,
    /// Target node ID (output of the operation)
    pub target: NodeId,
}

impl TraceEdge {
    /// Creates a new edge.
    #[must_use]
    pub const fn new(source: NodeId, target: NodeId) -> Self {
        Self { source, target }
    }
}

// =============================================================================
// Scope
// =============================================================================

/// A logical scope for grouping nodes in the computation graph.
///
/// Scopes are typically created at function boundaries using the
/// `#[traced_scope]` attribute macro.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Scope {
    /// Unique identifier for this scope
    pub id: ScopeId,
    /// Name of the scope (typically the function name)
    pub name: String,
    /// Parent scope ID (None if top-level)
    pub parent_id: Option<ScopeId>,
}

impl Scope {
    /// Creates a new scope.
    #[must_use]
    pub fn new(id: ScopeId, name: impl Into<String>, parent_id: Option<ScopeId>) -> Self {
        Self {
            id,
            name: name.into(),
            parent_id,
        }
    }
}

// =============================================================================
// DetailLevel
// =============================================================================

/// Level of detail for graph export.
///
/// Controls how much information is included in the exported graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DetailLevel {
    /// Show all individual operations (most detailed)
    #[default]
    Operation,
    /// Aggregate operations into scopes (less detailed)
    Scope,
}

// =============================================================================
// ExecutionTrace
// =============================================================================

/// Accumulator for execution trace data.
///
/// Collects nodes, edges, and scopes during computation with `TracedFloat`.
/// This is the main data structure used to build the computation graph.
#[derive(Debug, Default)]
pub struct ExecutionTrace {
    /// All nodes in the trace
    nodes: Vec<TraceNode>,
    /// All edges in the trace
    edges: Vec<TraceEdge>,
    /// All scopes in the trace
    scopes: Vec<Scope>,
    /// Stack of active scope IDs
    scope_stack: Vec<ScopeId>,
    /// Next node ID to assign
    next_node_id: u64,
    /// Next scope ID to assign
    next_scope_id: u64,
}

impl ExecutionTrace {
    /// Creates a new empty execution trace.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of nodes in the trace.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the trace.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the number of scopes in the trace.
    #[must_use]
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    /// Returns a reference to all nodes.
    #[must_use]
    pub fn nodes(&self) -> &[TraceNode] {
        &self.nodes
    }

    /// Returns a reference to all edges.
    #[must_use]
    pub fn edges(&self) -> &[TraceEdge] {
        &self.edges
    }

    /// Returns a reference to all scopes.
    #[must_use]
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// Returns the current active scope ID (if any).
    #[must_use]
    pub fn current_scope(&self) -> Option<ScopeId> {
        self.scope_stack.last().copied()
    }

    /// Adds an input node with a label.
    pub fn add_input(&mut self, value: f64, label: &str, location: SourceLocation) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let mut node = TraceNode::input(id, value, label, location);
        if let Some(scope_id) = self.current_scope() {
            node.scope_id = Some(scope_id);
        }

        self.nodes.push(node);
        id
    }

    /// Adds an operation node with the given inputs.
    pub fn add_node(
        &mut self,
        operation: Operation,
        value: f64,
        location: SourceLocation,
        input_ids: Vec<NodeId>,
    ) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        // Create edges from inputs to this node
        for &input_id in &input_ids {
            self.edges.push(TraceEdge::new(input_id, id));
        }

        let mut node = TraceNode::operation(id, operation, value, location, input_ids);
        if let Some(scope_id) = self.current_scope() {
            node.scope_id = Some(scope_id);
        }

        self.nodes.push(node);
        id
    }

    /// Adds a constant node (no inputs, no label).
    pub fn add_constant(&mut self, value: f64, location: SourceLocation) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let mut node = TraceNode {
            id,
            operation: Operation::Constant,
            value,
            source_location: location,
            scope_id: None,
            input_ids: Vec::new(),
            label: None,
        };

        if let Some(scope_id) = self.current_scope() {
            node.scope_id = Some(scope_id);
        }

        self.nodes.push(node);
        id
    }

    /// Enters a new scope with the given name.
    pub fn enter_scope(&mut self, name: &str) -> ScopeId {
        let id = ScopeId::new(self.next_scope_id);
        self.next_scope_id += 1;

        let parent_id = self.current_scope();
        let scope = Scope::new(id, name, parent_id);
        self.scopes.push(scope);
        self.scope_stack.push(id);

        id
    }

    /// Exits the current scope.
    pub fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Clears all trace data.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.scopes.clear();
        self.scope_stack.clear();
        self.next_node_id = 0;
        self.next_scope_id = 0;
    }
}

// Thread-local execution trace context.
// Provides a global access point for the current execution trace,
// used by TracedFloat and #[traced_scope] macro.
thread_local! {
    static TRACE_CONTEXT: RefCell<Option<Rc<RefCell<ExecutionTrace>>>> = const { RefCell::new(None) };
}

/// Sets the current thread's execution trace context.
pub fn set_trace_context(trace: Rc<RefCell<ExecutionTrace>>) {
    TRACE_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(trace);
    });
}

/// Clears the current thread's execution trace context.
pub fn clear_trace_context() {
    TRACE_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
}

/// Gets the current thread's execution trace context.
#[must_use]
pub fn get_trace_context() -> Option<Rc<RefCell<ExecutionTrace>>> {
    TRACE_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// RAII guard for scope management.
///
/// Automatically exits the scope when dropped.
pub struct ScopeGuard {
    trace: Rc<RefCell<ExecutionTrace>>,
}

impl ScopeGuard {
    /// Creates a new scope guard, entering the scope.
    #[must_use]
    pub fn new(trace: Rc<RefCell<ExecutionTrace>>, name: &str) -> Self {
        trace.borrow_mut().enter_scope(name);
        Self { trace }
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        self.trace.borrow_mut().exit_scope();
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod node_id_tests {
        use super::*;

        #[test]
        fn test_node_id_creation() {
            let id = NodeId::new(42);
            assert_eq!(id.value(), 42);
        }

        #[test]
        fn test_node_id_display() {
            let id = NodeId::new(1);
            assert_eq!(format!("{id}"), "N1");
        }

        #[test]
        fn test_node_id_equality() {
            let id1 = NodeId::new(1);
            let id2 = NodeId::new(1);
            let id3 = NodeId::new(2);
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn test_node_id_copy() {
            let id1 = NodeId::new(5);
            let id2 = id1;
            assert_eq!(id1, id2);
        }
    }

    mod scope_id_tests {
        use super::*;

        #[test]
        fn test_scope_id_creation() {
            let id = ScopeId::new(10);
            assert_eq!(id.value(), 10);
        }

        #[test]
        fn test_scope_id_display() {
            let id = ScopeId::new(3);
            assert_eq!(format!("{id}"), "S3");
        }
    }

    mod operation_tests {
        use super::*;

        #[test]
        fn test_operation_arity_input() {
            assert_eq!(Operation::Input.arity(), 0);
            assert_eq!(Operation::Constant.arity(), 0);
        }

        #[test]
        fn test_operation_arity_unary() {
            assert_eq!(Operation::Neg.arity(), 1);
            assert_eq!(Operation::Sqrt.arity(), 1);
            assert_eq!(Operation::Exp.arity(), 1);
            assert_eq!(Operation::Ln.arity(), 1);
            assert_eq!(Operation::Sin.arity(), 1);
            assert_eq!(Operation::Cos.arity(), 1);
            assert_eq!(Operation::Abs.arity(), 1);
        }

        #[test]
        fn test_operation_arity_binary() {
            assert_eq!(Operation::Add.arity(), 2);
            assert_eq!(Operation::Sub.arity(), 2);
            assert_eq!(Operation::Mul.arity(), 2);
            assert_eq!(Operation::Div.arity(), 2);
            assert_eq!(Operation::Powf.arity(), 2);
            assert_eq!(Operation::Max.arity(), 2);
            assert_eq!(Operation::Min.arity(), 2);
        }

        #[test]
        fn test_operation_arity_ternary() {
            assert_eq!(Operation::MulAdd.arity(), 3);
        }
    }

    mod source_location_tests {
        use super::*;

        #[test]
        fn test_source_location_new() {
            let loc = SourceLocation::new("test.rs", 42, 10);
            assert_eq!(loc.file, "test.rs");
            assert_eq!(loc.line, 42);
            assert_eq!(loc.column, 10);
        }

        #[test]
        fn test_source_location_display() {
            let loc = SourceLocation::new("src/lib.rs", 100, 5);
            assert_eq!(format!("{loc}"), "src/lib.rs:100:5");
        }

        #[test]
        #[track_caller]
        fn test_source_location_from_location() {
            let location = Location::caller();
            let loc = SourceLocation::from_location(location);
            assert!(loc.file.ends_with("traced.rs"));
            assert!(loc.line > 0);
        }
    }

    mod trace_node_tests {
        use super::*;

        #[test]
        fn test_trace_node_input() {
            let loc = SourceLocation::new("test.rs", 1, 1);
            let node = TraceNode::input(NodeId::new(0), 100.0, "spot", loc);

            assert_eq!(node.id, NodeId::new(0));
            assert_eq!(node.operation, Operation::Input);
            assert_eq!(node.value, 100.0);
            assert_eq!(node.label, Some("spot".to_string()));
            assert!(node.input_ids.is_empty());
        }

        #[test]
        fn test_trace_node_operation() {
            let loc = SourceLocation::new("test.rs", 2, 1);
            let node = TraceNode::operation(
                NodeId::new(2),
                Operation::Add,
                150.0,
                loc,
                vec![NodeId::new(0), NodeId::new(1)],
            );

            assert_eq!(node.id, NodeId::new(2));
            assert_eq!(node.operation, Operation::Add);
            assert_eq!(node.value, 150.0);
            assert_eq!(node.input_ids.len(), 2);
            assert!(node.label.is_none());
        }

        #[test]
        fn test_trace_node_with_scope() {
            let loc = SourceLocation::new("test.rs", 1, 1);
            let node = TraceNode::input(NodeId::new(0), 100.0, "spot", loc)
                .with_scope(ScopeId::new(5));

            assert_eq!(node.scope_id, Some(ScopeId::new(5)));
        }
    }

    mod trace_edge_tests {
        use super::*;

        #[test]
        fn test_trace_edge_creation() {
            let edge = TraceEdge::new(NodeId::new(0), NodeId::new(1));
            assert_eq!(edge.source, NodeId::new(0));
            assert_eq!(edge.target, NodeId::new(1));
        }
    }

    mod scope_tests {
        use super::*;

        #[test]
        fn test_scope_creation() {
            let scope = Scope::new(ScopeId::new(0), "calculate_price", None);
            assert_eq!(scope.id, ScopeId::new(0));
            assert_eq!(scope.name, "calculate_price");
            assert!(scope.parent_id.is_none());
        }

        #[test]
        fn test_scope_with_parent() {
            let scope = Scope::new(ScopeId::new(1), "inner_calc", Some(ScopeId::new(0)));
            assert_eq!(scope.parent_id, Some(ScopeId::new(0)));
        }
    }

    mod detail_level_tests {
        use super::*;

        #[test]
        fn test_detail_level_default() {
            let level = DetailLevel::default();
            assert_eq!(level, DetailLevel::Operation);
        }
    }

    mod execution_trace_tests {
        use super::*;

        #[test]
        fn test_execution_trace_new() {
            let trace = ExecutionTrace::new();
            assert_eq!(trace.node_count(), 0);
            assert_eq!(trace.edge_count(), 0);
            assert_eq!(trace.scope_count(), 0);
        }

        #[test]
        fn test_add_input() {
            let mut trace = ExecutionTrace::new();
            let loc = SourceLocation::new("test.rs", 1, 1);
            let id = trace.add_input(100.0, "spot", loc);

            assert_eq!(id, NodeId::new(0));
            assert_eq!(trace.node_count(), 1);
            assert_eq!(trace.nodes()[0].operation, Operation::Input);
            assert_eq!(trace.nodes()[0].label, Some("spot".to_string()));
        }

        #[test]
        fn test_add_node_creates_edges() {
            let mut trace = ExecutionTrace::new();
            let loc = SourceLocation::new("test.rs", 1, 1);

            let id0 = trace.add_input(100.0, "a", loc.clone());
            let id1 = trace.add_input(50.0, "b", loc.clone());
            let id2 = trace.add_node(Operation::Add, 150.0, loc, vec![id0, id1]);

            assert_eq!(trace.node_count(), 3);
            assert_eq!(trace.edge_count(), 2);
            assert_eq!(id2, NodeId::new(2));

            // Check edges
            assert_eq!(trace.edges()[0], TraceEdge::new(id0, id2));
            assert_eq!(trace.edges()[1], TraceEdge::new(id1, id2));
        }

        #[test]
        fn test_add_constant() {
            let mut trace = ExecutionTrace::new();
            let loc = SourceLocation::new("test.rs", 1, 1);
            let id = trace.add_constant(3.14159, loc);

            assert_eq!(id, NodeId::new(0));
            assert_eq!(trace.nodes()[0].operation, Operation::Constant);
            assert!(trace.nodes()[0].label.is_none());
        }

        #[test]
        fn test_scope_management() {
            let mut trace = ExecutionTrace::new();

            assert!(trace.current_scope().is_none());

            let s1 = trace.enter_scope("outer");
            assert_eq!(trace.current_scope(), Some(s1));
            assert_eq!(trace.scope_count(), 1);

            let s2 = trace.enter_scope("inner");
            assert_eq!(trace.current_scope(), Some(s2));
            assert_eq!(trace.scope_count(), 2);

            // Check parent relationship
            assert_eq!(trace.scopes()[1].parent_id, Some(s1));

            trace.exit_scope();
            assert_eq!(trace.current_scope(), Some(s1));

            trace.exit_scope();
            assert!(trace.current_scope().is_none());
        }

        #[test]
        fn test_nodes_inherit_scope() {
            let mut trace = ExecutionTrace::new();
            let loc = SourceLocation::new("test.rs", 1, 1);

            let s1 = trace.enter_scope("calc");
            let id = trace.add_input(100.0, "x", loc);

            assert_eq!(trace.nodes()[0].scope_id, Some(s1));
            assert_eq!(id, NodeId::new(0));
        }

        #[test]
        fn test_clear() {
            let mut trace = ExecutionTrace::new();
            let loc = SourceLocation::new("test.rs", 1, 1);

            trace.enter_scope("test");
            trace.add_input(100.0, "x", loc.clone());
            trace.add_constant(1.0, loc);

            trace.clear();

            assert_eq!(trace.node_count(), 0);
            assert_eq!(trace.edge_count(), 0);
            assert_eq!(trace.scope_count(), 0);
            assert!(trace.current_scope().is_none());
        }
    }

    mod thread_local_tests {
        use super::*;

        #[test]
        fn test_trace_context() {
            let trace = Rc::new(RefCell::new(ExecutionTrace::new()));

            set_trace_context(Rc::clone(&trace));
            assert!(get_trace_context().is_some());

            clear_trace_context();
            assert!(get_trace_context().is_none());
        }

        #[test]
        fn test_scope_guard() {
            let trace = Rc::new(RefCell::new(ExecutionTrace::new()));

            {
                let _guard = ScopeGuard::new(Rc::clone(&trace), "test_scope");
                assert_eq!(trace.borrow().scope_count(), 1);
                assert!(trace.borrow().current_scope().is_some());
            }

            // Guard dropped, scope exited
            assert!(trace.borrow().current_scope().is_none());
        }
    }

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_operation_serialisation() {
            let json = serde_json::to_string(&Operation::Add).unwrap();
            assert_eq!(json, "\"add\"");

            let json = serde_json::to_string(&Operation::ExpM1).unwrap();
            assert_eq!(json, "\"exp_m1\"");
        }

        #[test]
        fn test_detail_level_serialisation() {
            let json = serde_json::to_string(&DetailLevel::Operation).unwrap();
            assert_eq!(json, "\"operation\"");

            let json = serde_json::to_string(&DetailLevel::Scope).unwrap();
            assert_eq!(json, "\"scope\"");
        }
    }
}

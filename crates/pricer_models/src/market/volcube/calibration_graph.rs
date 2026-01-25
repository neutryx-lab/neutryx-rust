//! Calibration Dependency Graph for Curve→VolCube management.
//!
//! # Requirements: 6.8, 6.9, 6.10, 6.12
//!
//! This module provides a dependency graph for managing the calibration
//! order and state of market objects (Curves and VolCubes).
//!
//! # Architecture
//!
//! ```text
//! CalibrationGraph
//! ├── Nodes (CalibrationNode)
//! │   ├── Curve nodes (discount, projection)
//! │   └── VolCube nodes
//! ├── Edges (parent → child dependencies)
//! │   └── VolCube depends on Curve(s)
//! ├── State tracking (CalibrationState)
//! └── Lazy calibration support
//! ```
//!
//! # Lazy Calibration Example
//!
//! ```ignore
//! use pricer_models::market::volcube::calibration_graph::{
//!     CalibrationGraph, CalibrationNodeId, CalibrationExecutor, GraphCalibrationResult,
//! };
//!
//! // Define a calibration executor
//! struct MyCalibratorExecutor;
//! impl CalibrationExecutor for MyCalibratorExecutor {
//!     fn calibrate(&self, id: &CalibrationNodeId) -> GraphCalibrationResult {
//!         // Perform actual calibration...
//!         Ok(())
//!     }
//! }
//!
//! let mut graph = CalibrationGraph::new();
//! let curve_id = graph.add_curve("USD-SOFR", "USD SOFR Curve")?;
//! let vol_id = graph.add_volcube("USD-VOL", "USD Swaption Vol")?;
//! graph.add_dependency(&vol_id, &curve_id)?;
//!
//! // Request VolCube - auto-calibrates curve first
//! let executor = MyCalibratorExecutor;
//! graph.ensure_calibrated(&vol_id, &executor)?;
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

/// Unique identifier for a calibration node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CalibrationNodeId(String);

impl CalibrationNodeId {
    /// Create a new node ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Get the string representation.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for CalibrationNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl From<&str> for CalibrationNodeId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<String> for CalibrationNodeId {
    fn from(s: String) -> Self { Self(s) }
}

/// Kind of calibration node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Yield curve (discount or projection).
    Curve,
    /// Volatility cube.
    VolCube,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeKind::Curve => write!(f, "Curve"),
            NodeKind::VolCube => write!(f, "VolCube"),
        }
    }
}

/// Calibration state of a node.
///
/// # Requirements: 6.12
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CalibrationState {
    /// Not yet calibrated.
    #[default]
    Pending,
    /// Currently being calibrated.
    Computing,
    /// Successfully calibrated.
    Calibrated,
    /// Stale (dependency changed, needs recalibration).
    Stale,
    /// Failed calibration.
    Failed,
}

impl CalibrationState {
    /// Check if the node needs calibration.
    pub fn needs_calibration(&self) -> bool {
        matches!(self, Self::Pending | Self::Stale | Self::Failed)
    }

    /// Check if the node is successfully calibrated.
    pub fn is_calibrated(&self) -> bool { matches!(self, Self::Calibrated) }
}

impl std::fmt::Display for CalibrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalibrationState::Pending => write!(f, "Pending"),
            CalibrationState::Computing => write!(f, "Computing"),
            CalibrationState::Calibrated => write!(f, "Calibrated"),
            CalibrationState::Stale => write!(f, "Stale"),
            CalibrationState::Failed => write!(f, "Failed"),
        }
    }
}

/// A node in the calibration graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationNode {
    /// Unique node identifier.
    pub id: CalibrationNodeId,
    /// Human-readable name.
    pub name: String,
    /// Node kind (Curve or VolCube).
    pub kind: NodeKind,
    /// Current calibration state.
    pub state: CalibrationState,
    /// Timestamp of last calibration (Unix epoch seconds).
    pub last_calibrated: Option<u64>,
}

impl CalibrationNode {
    /// Create a new calibration node.
    pub fn new(id: impl Into<CalibrationNodeId>, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            state: CalibrationState::Pending,
            last_calibrated: None,
        }
    }

    /// Create a curve node.
    pub fn curve(id: impl Into<CalibrationNodeId>, name: impl Into<String>) -> Self {
        Self::new(id, name, NodeKind::Curve)
    }

    /// Create a VolCube node.
    pub fn volcube(id: impl Into<CalibrationNodeId>, name: impl Into<String>) -> Self {
        Self::new(id, name, NodeKind::VolCube)
    }

    /// Set the calibration state.
    pub fn with_state(mut self, state: CalibrationState) -> Self {
        self.state = state;
        self
    }

    /// Mark as computing.
    pub fn mark_computing(&mut self) { self.state = CalibrationState::Computing; }

    /// Mark as calibrated.
    pub fn mark_calibrated(&mut self) {
        self.state = CalibrationState::Calibrated;
        self.last_calibrated = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Mark as stale.
    pub fn mark_stale(&mut self) { self.state = CalibrationState::Stale; }

    /// Mark as failed.
    pub fn mark_failed(&mut self) { self.state = CalibrationState::Failed; }
}

/// Error type for calibration graph operations.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
    /// Node not found.
    NodeNotFound {
        /// The ID of the missing node.
        id: CalibrationNodeId,
    },
    /// Circular dependency detected.
    CyclicDependency {
        /// Nodes involved in the cycle.
        cycle: Vec<CalibrationNodeId>,
    },
    /// Duplicate node ID.
    DuplicateNode {
        /// The duplicate ID.
        id: CalibrationNodeId,
    },
    /// Invalid dependency (e.g., Curve depending on VolCube).
    InvalidDependency {
        /// The child node ID.
        child: CalibrationNodeId,
        /// The parent node ID.
        parent: CalibrationNodeId,
        /// Reason for the error.
        reason: String,
    },
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::NodeNotFound { id } => {
                write!(f, "Node not found: {}", id)
            }
            GraphError::CyclicDependency { cycle } => {
                let cycle_str: Vec<_> = cycle.iter().map(|id| id.as_str()).collect();
                write!(f, "Cyclic dependency detected: {}", cycle_str.join(" → "))
            }
            GraphError::DuplicateNode { id } => {
                write!(f, "Duplicate node ID: {}", id)
            }
            GraphError::InvalidDependency {
                child,
                parent,
                reason,
            } => {
                write!(
                    f,
                    "Invalid dependency from {} to {}: {}",
                    child, parent, reason
                )
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// Result type for graph calibration operations.
pub type GraphCalibrationResult = Result<(), GraphError>;

/// Trait for executing calibration of individual nodes.
///
/// # Requirements: 6.8, 6.10
///
/// Implement this trait to provide actual calibration logic for
/// curves and VolCubes. The graph uses this to perform lazy
/// calibration when a node is requested.
pub trait CalibrationExecutor: Send + Sync {
    /// Calibrate a single node.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to calibrate
    /// * `kind` - The kind of node (Curve or VolCube)
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or GraphError on failure.
    fn calibrate(&self, id: &CalibrationNodeId, kind: NodeKind) -> GraphCalibrationResult;
}

/// A no-op calibration executor for testing.
///
/// Always succeeds without performing actual calibration.
#[derive(Debug, Default)]
pub struct NoOpCalibrationExecutor;

impl CalibrationExecutor for NoOpCalibrationExecutor {
    fn calibrate(&self, _id: &CalibrationNodeId, _kind: NodeKind) -> GraphCalibrationResult {
        Ok(())
    }
}

/// Calibration dependency graph.
///
/// # Requirements: 6.9, 6.12
///
/// Manages dependencies between Curves and VolCubes for ordered calibration.
#[derive(Debug, Clone, Default)]
pub struct CalibrationGraph {
    /// All nodes in the graph.
    nodes: HashMap<CalibrationNodeId, CalibrationNode>,
    /// Child → Parents mapping (what this node depends on).
    parents: HashMap<CalibrationNodeId, HashSet<CalibrationNodeId>>,
    /// Parent → Children mapping (what depends on this node).
    children: HashMap<CalibrationNodeId, HashSet<CalibrationNodeId>>,
}

impl CalibrationGraph {
    /// Create an empty calibration graph.
    pub fn new() -> Self { Self::default() }

    /// Add a node to the graph.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to add
    ///
    /// # Returns
    ///
    /// The node ID, or error if duplicate.
    pub fn add_node(&mut self, node: CalibrationNode) -> Result<CalibrationNodeId, GraphError> {
        let id = node.id.clone();

        if self.nodes.contains_key(&id) {
            return Err(GraphError::DuplicateNode { id });
        }

        self.nodes.insert(id.clone(), node);
        self.parents.insert(id.clone(), HashSet::new());
        self.children.insert(id.clone(), HashSet::new());

        Ok(id)
    }

    /// Add a curve node.
    pub fn add_curve(
        &mut self,
        id: impl Into<CalibrationNodeId>,
        name: impl Into<String>,
    ) -> Result<CalibrationNodeId, GraphError> {
        self.add_node(CalibrationNode::curve(id, name))
    }

    /// Add a VolCube node.
    pub fn add_volcube(
        &mut self,
        id: impl Into<CalibrationNodeId>,
        name: impl Into<String>,
    ) -> Result<CalibrationNodeId, GraphError> {
        self.add_node(CalibrationNode::volcube(id, name))
    }

    /// Add a dependency edge (child depends on parent).
    ///
    /// # Arguments
    ///
    /// * `child` - The node that depends on another
    /// * `parent` - The node being depended upon
    ///
    /// # Returns
    ///
    /// Ok if successful, or error if nodes don't exist or dependency is
    /// invalid.
    pub fn add_dependency(
        &mut self,
        child: &CalibrationNodeId,
        parent: &CalibrationNodeId,
    ) -> Result<(), GraphError> {
        // Verify both nodes exist
        if !self.nodes.contains_key(child) {
            return Err(GraphError::NodeNotFound { id: child.clone() });
        }
        if !self.nodes.contains_key(parent) {
            return Err(GraphError::NodeNotFound { id: parent.clone() });
        }

        // Validate dependency direction
        let child_node = &self.nodes[child];
        let parent_node = &self.nodes[parent];

        // Curves should not depend on VolCubes
        if child_node.kind == NodeKind::Curve && parent_node.kind == NodeKind::VolCube {
            return Err(GraphError::InvalidDependency {
                child: child.clone(),
                parent: parent.clone(),
                reason: "Curves cannot depend on VolCubes".to_string(),
            });
        }

        // Add the edge
        self.parents.get_mut(child).unwrap().insert(parent.clone());
        self.children.get_mut(parent).unwrap().insert(child.clone());

        // Check for cycles
        if self.has_cycle() {
            // Rollback
            self.parents.get_mut(child).unwrap().remove(parent);
            self.children.get_mut(parent).unwrap().remove(child);
            return Err(GraphError::CyclicDependency {
                cycle: vec![child.clone(), parent.clone()],
            });
        }

        Ok(())
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &CalibrationNodeId) -> Option<&CalibrationNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: &CalibrationNodeId) -> Option<&mut CalibrationNode> {
        self.nodes.get_mut(id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &CalibrationNode> { self.nodes.values() }

    /// Get node count.
    pub fn node_count(&self) -> usize { self.nodes.len() }

    /// Get edge count.
    pub fn edge_count(&self) -> usize { self.parents.values().map(|p| p.len()).sum() }

    /// Get parent nodes (dependencies) for a node.
    pub fn get_parents(&self, id: &CalibrationNodeId) -> Option<&HashSet<CalibrationNodeId>> {
        self.parents.get(id)
    }

    /// Get child nodes (dependents) for a node.
    pub fn get_children(&self, id: &CalibrationNodeId) -> Option<&HashSet<CalibrationNodeId>> {
        self.children.get(id)
    }

    /// Check if the graph has a cycle using DFS.
    fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for id in self.nodes.keys() {
            if self.has_cycle_dfs(id, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    /// DFS helper for cycle detection.
    fn has_cycle_dfs(
        &self,
        node: &CalibrationNodeId,
        visited: &mut HashSet<CalibrationNodeId>,
        rec_stack: &mut HashSet<CalibrationNodeId>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }

        if visited.contains(node) {
            return false;
        }

        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(parents) = self.parents.get(node) {
            for parent in parents {
                if self.has_cycle_dfs(parent, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Compute calibration order using Kahn's algorithm (topological sort).
    ///
    /// # Requirements: 6.9
    ///
    /// Returns nodes in order such that all dependencies are calibrated before
    /// their dependents.
    ///
    /// # Returns
    ///
    /// Vec of node IDs in calibration order, or error if cycle detected.
    pub fn calibration_order(&self) -> Result<Vec<CalibrationNodeId>, GraphError> {
        let mut in_degree: HashMap<&CalibrationNodeId, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Calculate in-degree for each node
        for id in self.nodes.keys() {
            let degree = self.parents.get(id).map(|p| p.len()).unwrap_or(0);
            in_degree.insert(id, degree);

            if degree == 0 {
                queue.push_back(id);
            }
        }

        // Process nodes with zero in-degree
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            // Reduce in-degree for children
            if let Some(children) = self.children.get(node) {
                for child in children {
                    if let Some(degree) = in_degree.get_mut(child) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        // Check if all nodes were processed (no cycle)
        if result.len() != self.nodes.len() {
            // Find nodes in cycle
            let cycle: Vec<_> = self
                .nodes
                .keys()
                .filter(|id| !result.contains(id))
                .cloned()
                .collect();
            return Err(GraphError::CyclicDependency { cycle });
        }

        Ok(result)
    }

    /// Get nodes that need calibration.
    pub fn nodes_needing_calibration(&self) -> Vec<&CalibrationNodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.state.needs_calibration())
            .map(|(id, _)| id)
            .collect()
    }

    /// Cascade invalidation: mark a node and all its descendants as stale.
    ///
    /// # Requirements: 6.12
    ///
    /// When a node becomes stale (e.g., market data update), all nodes that
    /// depend on it transitively also become stale.
    pub fn cascade_invalidate(&mut self, id: &CalibrationNodeId) -> Result<usize, GraphError> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound { id: id.clone() });
        }

        let mut count = 0;
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(id.clone());

        while let Some(node_id) = queue.pop_front() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id.clone());

            // Mark as stale
            if let Some(node) = self.nodes.get_mut(&node_id) {
                if node.state.is_calibrated() {
                    node.mark_stale();
                    count += 1;
                }
            }

            // Add children to queue
            if let Some(children) = self.children.get(&node_id) {
                for child in children {
                    if !visited.contains(child) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        Ok(count)
    }

    /// Update node state.
    pub fn set_state(
        &mut self,
        id: &CalibrationNodeId,
        state: CalibrationState,
    ) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or_else(|| GraphError::NodeNotFound { id: id.clone() })?;
        node.state = state;
        Ok(())
    }

    /// Get calibration order for nodes that need calibration.
    ///
    /// Returns only the nodes that need calibration, in proper order.
    pub fn pending_calibration_order(&self) -> Result<Vec<CalibrationNodeId>, GraphError> {
        let full_order = self.calibration_order()?;

        Ok(full_order
            .into_iter()
            .filter(|id| {
                self.nodes
                    .get(id)
                    .map(|n| n.state.needs_calibration())
                    .unwrap_or(false)
            })
            .collect())
    }

    // ========================================
    // Lazy Calibration Support (Task 7.3)
    // ========================================

    /// Check if all dependencies of a node are calibrated.
    ///
    /// # Requirements: 6.8
    ///
    /// Returns true if the node can be calibrated (all parents calibrated).
    pub fn can_calibrate(&self, id: &CalibrationNodeId) -> Result<bool, GraphError> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound { id: id.clone() });
        }

        let parents = self.parents.get(id).map(|p| p.iter()).into_iter().flatten();

        for parent_id in parents {
            if let Some(parent) = self.nodes.get(parent_id) {
                if !parent.state.is_calibrated() {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Get uncalibrated dependencies for a node.
    ///
    /// # Requirements: 6.8, 6.10
    ///
    /// Returns the IDs of parent nodes that need calibration, in calibration
    /// order.
    pub fn uncalibrated_dependencies(
        &self,
        id: &CalibrationNodeId,
    ) -> Result<Vec<CalibrationNodeId>, GraphError> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound { id: id.clone() });
        }

        // Get all transitive dependencies
        let deps = self.transitive_dependencies(id);

        // Filter to uncalibrated and sort by calibration order
        let full_order = self.calibration_order()?;
        let uncalibrated: HashSet<_> = deps
            .into_iter()
            .filter(|dep_id| {
                self.nodes
                    .get(dep_id)
                    .map(|n| n.state.needs_calibration())
                    .unwrap_or(false)
            })
            .collect();

        Ok(full_order
            .into_iter()
            .filter(|id| uncalibrated.contains(id))
            .collect())
    }

    /// Get all transitive dependencies (parents, grandparents, etc.).
    fn transitive_dependencies(&self, id: &CalibrationNodeId) -> Vec<CalibrationNodeId> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct parents
        if let Some(parents) = self.parents.get(id) {
            for parent in parents {
                queue.push_back(parent.clone());
            }
        }

        while let Some(node_id) = queue.pop_front() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id.clone());
            result.push(node_id.clone());

            // Add grandparents
            if let Some(parents) = self.parents.get(&node_id) {
                for parent in parents {
                    if !visited.contains(parent) {
                        queue.push_back(parent.clone());
                    }
                }
            }
        }

        result
    }

    /// Get the calibration sequence needed to calibrate a specific node.
    ///
    /// # Requirements: 6.8, 6.10
    ///
    /// Returns a sequence of node IDs that need to be calibrated (in order)
    /// before and including the target node. Only includes nodes that need
    /// calibration.
    ///
    /// This is the key method for lazy calibration: when a VolCube is
    /// requested, call this method to get the list of curves that need to
    /// be calibrated first.
    pub fn calibration_sequence_for(
        &self,
        id: &CalibrationNodeId,
    ) -> Result<Vec<CalibrationNodeId>, GraphError> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound { id: id.clone() });
        }

        // Get uncalibrated dependencies
        let mut sequence = self.uncalibrated_dependencies(id)?;

        // Add the target node itself if it needs calibration
        if let Some(node) = self.nodes.get(id) {
            if node.state.needs_calibration() {
                sequence.push(id.clone());
            }
        }

        Ok(sequence)
    }

    /// Check if a node is ready for calibration (not computing, dependencies
    /// met).
    ///
    /// A node is ready if:
    /// - It needs calibration (Pending, Stale, or Failed)
    /// - It's not currently being computed
    /// - All its dependencies are calibrated
    pub fn is_ready_for_calibration(&self, id: &CalibrationNodeId) -> Result<bool, GraphError> {
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| GraphError::NodeNotFound { id: id.clone() })?;

        // Must need calibration
        if !node.state.needs_calibration() {
            return Ok(false);
        }

        // Must not be computing
        if node.state == CalibrationState::Computing {
            return Ok(false);
        }

        // All dependencies must be calibrated
        self.can_calibrate(id)
    }

    /// Get all nodes that are ready for calibration.
    ///
    /// Returns nodes that need calibration and have all dependencies
    /// satisfied.
    pub fn ready_nodes(&self) -> Vec<CalibrationNodeId> {
        self.nodes
            .keys()
            .filter(|id| self.is_ready_for_calibration(id).unwrap_or(false))
            .cloned()
            .collect()
    }

    // ========================================
    // Lazy Calibration Execution (Task 7.3)
    // ========================================

    /// Ensure a node is calibrated, auto-calibrating dependencies if needed.
    ///
    /// # Requirements: 6.8, 6.10
    ///
    /// This is the main entry point for lazy calibration. When a VolCube
    /// is requested, call this method to ensure all dependencies are
    /// calibrated first.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to ensure is calibrated
    /// * `executor` - The calibration executor to use
    ///
    /// # Returns
    ///
    /// Ok(()) if the node (and all dependencies) were successfully calibrated,
    /// or Err if any calibration failed.
    pub fn ensure_calibrated(
        &mut self,
        id: &CalibrationNodeId,
        executor: &dyn CalibrationExecutor,
    ) -> GraphCalibrationResult {
        // Get the calibration sequence
        let sequence = self.calibration_sequence_for(id)?;

        // Calibrate each node in order
        for node_id in sequence {
            self.calibrate_node(&node_id, executor)?;
        }

        Ok(())
    }

    /// Calibrate a single node using the executor.
    ///
    /// Updates the node state based on calibration result.
    fn calibrate_node(
        &mut self,
        id: &CalibrationNodeId,
        executor: &dyn CalibrationExecutor,
    ) -> GraphCalibrationResult {
        // Get node kind
        let kind = {
            let node = self
                .nodes
                .get(id)
                .ok_or_else(|| GraphError::NodeNotFound { id: id.clone() })?;
            node.kind
        };

        // Mark as computing
        if let Some(node) = self.nodes.get_mut(id) {
            node.mark_computing();
        }

        // Execute calibration
        let result = executor.calibrate(id, kind);

        // Update state based on result
        if let Some(node) = self.nodes.get_mut(id) {
            match &result {
                Ok(()) => node.mark_calibrated(),
                Err(_) => node.mark_failed(),
            }
        }

        result
    }

    /// Ensure multiple nodes are calibrated.
    ///
    /// Optimises by computing the minimal calibration sequence for all
    /// requested nodes.
    pub fn ensure_calibrated_all(
        &mut self,
        ids: &[CalibrationNodeId],
        executor: &dyn CalibrationExecutor,
    ) -> GraphCalibrationResult {
        // Collect all needed calibrations
        let mut all_needed: HashSet<CalibrationNodeId> = HashSet::new();

        for id in ids {
            let seq = self.calibration_sequence_for(id)?;
            all_needed.extend(seq);
        }

        // Sort by calibration order
        let full_order = self.calibration_order()?;
        let ordered: Vec<_> = full_order
            .into_iter()
            .filter(|id| all_needed.contains(id))
            .collect();

        // Calibrate in order
        for node_id in ordered {
            self.calibrate_node(&node_id, executor)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Node ID Tests
    // ========================================

    #[test]
    fn test_node_id_creation() {
        let id = CalibrationNodeId::new("test-curve");
        assert_eq!(id.as_str(), "test-curve");
    }

    #[test]
    fn test_node_id_display() {
        let id = CalibrationNodeId::new("USD-SOFR");
        assert_eq!(format!("{}", id), "USD-SOFR");
    }

    #[test]
    fn test_node_id_from_str() {
        let id: CalibrationNodeId = "EUR-ESTR".into();
        assert_eq!(id.as_str(), "EUR-ESTR");
    }

    // ========================================
    // CalibrationState Tests
    // ========================================

    #[test]
    fn test_calibration_state_needs_calibration() {
        assert!(CalibrationState::Pending.needs_calibration());
        assert!(CalibrationState::Stale.needs_calibration());
        assert!(CalibrationState::Failed.needs_calibration());
        assert!(!CalibrationState::Calibrated.needs_calibration());
        assert!(!CalibrationState::Computing.needs_calibration());
    }

    #[test]
    fn test_calibration_state_is_calibrated() {
        assert!(CalibrationState::Calibrated.is_calibrated());
        assert!(!CalibrationState::Pending.is_calibrated());
        assert!(!CalibrationState::Stale.is_calibrated());
    }

    // ========================================
    // CalibrationNode Tests
    // ========================================

    #[test]
    fn test_node_creation() {
        let node = CalibrationNode::curve("USD-SOFR", "USD SOFR Discount Curve");

        assert_eq!(node.id.as_str(), "USD-SOFR");
        assert_eq!(node.name, "USD SOFR Discount Curve");
        assert_eq!(node.kind, NodeKind::Curve);
        assert_eq!(node.state, CalibrationState::Pending);
    }

    #[test]
    fn test_node_state_transitions() {
        let mut node = CalibrationNode::volcube("USD-VOL", "USD Swaption Vol Cube");

        assert_eq!(node.state, CalibrationState::Pending);

        node.mark_computing();
        assert_eq!(node.state, CalibrationState::Computing);

        node.mark_calibrated();
        assert_eq!(node.state, CalibrationState::Calibrated);
        assert!(node.last_calibrated.is_some());

        node.mark_stale();
        assert_eq!(node.state, CalibrationState::Stale);
    }

    // ========================================
    // Graph Construction Tests
    // ========================================

    #[test]
    fn test_graph_add_nodes() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("USD-SOFR", "USD SOFR Curve").unwrap();
        let volcube_id = graph
            .add_volcube("USD-VOL", "USD Swaption VolCube")
            .unwrap();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.get_node(&curve_id).is_some());
        assert!(graph.get_node(&volcube_id).is_some());
    }

    #[test]
    fn test_graph_duplicate_node() {
        let mut graph = CalibrationGraph::new();

        graph.add_curve("USD-SOFR", "First").unwrap();
        let result = graph.add_curve("USD-SOFR", "Second");

        assert!(matches!(result, Err(GraphError::DuplicateNode { .. })));
    }

    #[test]
    fn test_graph_add_dependency() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("USD-SOFR", "USD SOFR").unwrap();
        let volcube_id = graph.add_volcube("USD-VOL", "USD Vol").unwrap();

        graph.add_dependency(&volcube_id, &curve_id).unwrap();

        assert_eq!(graph.edge_count(), 1);

        let parents = graph.get_parents(&volcube_id).unwrap();
        assert!(parents.contains(&curve_id));

        let children = graph.get_children(&curve_id).unwrap();
        assert!(children.contains(&volcube_id));
    }

    #[test]
    fn test_graph_invalid_dependency_curve_on_volcube() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("USD-SOFR", "USD SOFR").unwrap();
        let volcube_id = graph.add_volcube("USD-VOL", "USD Vol").unwrap();

        // Curve should not depend on VolCube
        let result = graph.add_dependency(&curve_id, &volcube_id);

        assert!(matches!(result, Err(GraphError::InvalidDependency { .. })));
    }

    // ========================================
    // Topological Sort Tests
    // ========================================

    #[test]
    fn test_calibration_order_simple() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "VolCube").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        let order = graph.calibration_order().unwrap();

        // Curve should come before VolCube
        let curve_pos = order.iter().position(|id| *id == curve_id).unwrap();
        let vol_pos = order.iter().position(|id| *id == vol_id).unwrap();
        assert!(curve_pos < vol_pos);
    }

    #[test]
    fn test_calibration_order_complex() {
        let mut graph = CalibrationGraph::new();

        // Create: DISC -> PROJ -> VOL
        //              \-> VOL
        let disc_id = graph.add_curve("DISC", "Discount").unwrap();
        let proj_id = graph.add_curve("PROJ", "Projection").unwrap();
        let vol_id = graph.add_volcube("VOL", "VolCube").unwrap();

        graph.add_dependency(&proj_id, &disc_id).unwrap();
        graph.add_dependency(&vol_id, &disc_id).unwrap();
        graph.add_dependency(&vol_id, &proj_id).unwrap();

        let order = graph.calibration_order().unwrap();

        // Check order: DISC < PROJ < VOL
        let disc_pos = order.iter().position(|id| *id == disc_id).unwrap();
        let proj_pos = order.iter().position(|id| *id == proj_id).unwrap();
        let vol_pos = order.iter().position(|id| *id == vol_id).unwrap();

        assert!(disc_pos < proj_pos);
        assert!(proj_pos < vol_pos);
    }

    #[test]
    fn test_calibration_order_cycle_detection() {
        let mut graph = CalibrationGraph::new();

        let v1 = graph.add_volcube("V1", "Vol1").unwrap();
        let v2 = graph.add_volcube("V2", "Vol2").unwrap();

        graph.add_dependency(&v2, &v1).unwrap();

        // This should detect the cycle
        let result = graph.add_dependency(&v1, &v2);
        assert!(matches!(result, Err(GraphError::CyclicDependency { .. })));
    }

    // ========================================
    // Cascade Invalidation Tests
    // ========================================

    #[test]
    fn test_cascade_invalidate() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Mark both as calibrated
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();
        graph.get_node_mut(&vol_id).unwrap().mark_calibrated();

        // Invalidate the curve
        let count = graph.cascade_invalidate(&curve_id).unwrap();

        // Both should be stale (curve + vol)
        assert_eq!(count, 2);
        assert_eq!(
            graph.get_node(&curve_id).unwrap().state,
            CalibrationState::Stale
        );
        assert_eq!(
            graph.get_node(&vol_id).unwrap().state,
            CalibrationState::Stale
        );
    }

    #[test]
    fn test_cascade_invalidate_deep() {
        let mut graph = CalibrationGraph::new();

        let c1 = graph.add_curve("C1", "Curve1").unwrap();
        let c2 = graph.add_curve("C2", "Curve2").unwrap();
        let v1 = graph.add_volcube("V1", "Vol1").unwrap();
        let v2 = graph.add_volcube("V2", "Vol2").unwrap();

        // C1 <- C2 <- V1 <- V2
        graph.add_dependency(&c2, &c1).unwrap();
        graph.add_dependency(&v1, &c2).unwrap();
        graph.add_dependency(&v2, &v1).unwrap();

        // Mark all as calibrated
        for id in [&c1, &c2, &v1, &v2] {
            graph.get_node_mut(id).unwrap().mark_calibrated();
        }

        // Invalidate C1 - should cascade to all
        let count = graph.cascade_invalidate(&c1).unwrap();
        assert_eq!(count, 4);
    }

    // ========================================
    // Pending Calibration Order Tests
    // ========================================

    #[test]
    fn test_pending_calibration_order() {
        let mut graph = CalibrationGraph::new();

        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Mark curve as calibrated
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        // Only vol should need calibration
        let pending = graph.pending_calibration_order().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0], vol_id);
    }

    // ========================================
    // GraphError Tests
    // ========================================

    #[test]
    fn test_graph_error_display() {
        let err = GraphError::NodeNotFound {
            id: CalibrationNodeId::new("MISSING"),
        };
        assert!(err.to_string().contains("MISSING"));

        let err2 = GraphError::CyclicDependency {
            cycle: vec![CalibrationNodeId::new("A"), CalibrationNodeId::new("B")],
        };
        assert!(err2.to_string().contains("Cyclic"));
    }

    // ========================================
    // Lazy Calibration Tests (Task 7.3)
    // ========================================

    #[test]
    fn test_can_calibrate_no_dependencies() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        // No dependencies, can calibrate immediately
        assert!(graph.can_calibrate(&curve_id).unwrap());
    }

    #[test]
    fn test_can_calibrate_dependency_not_calibrated() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Curve is pending, vol cannot be calibrated
        assert!(!graph.can_calibrate(&vol_id).unwrap());
    }

    #[test]
    fn test_can_calibrate_dependency_calibrated() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Calibrate the curve
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        // Now vol can be calibrated
        assert!(graph.can_calibrate(&vol_id).unwrap());
    }

    #[test]
    fn test_can_calibrate_missing_node() {
        let graph = CalibrationGraph::new();
        let missing = CalibrationNodeId::new("MISSING");

        let result = graph.can_calibrate(&missing);
        assert!(matches!(result, Err(GraphError::NodeNotFound { .. })));
    }

    #[test]
    fn test_uncalibrated_dependencies_none() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Calibrate the curve
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        // No uncalibrated dependencies
        let deps = graph.uncalibrated_dependencies(&vol_id).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_uncalibrated_dependencies_one() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Curve is pending
        let deps = graph.uncalibrated_dependencies(&vol_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], curve_id);
    }

    #[test]
    fn test_uncalibrated_dependencies_chain() {
        let mut graph = CalibrationGraph::new();

        // C1 <- C2 <- V
        let c1 = graph.add_curve("C1", "Curve1").unwrap();
        let c2 = graph.add_curve("C2", "Curve2").unwrap();
        let vol = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&c2, &c1).unwrap();
        graph.add_dependency(&vol, &c2).unwrap();

        // All pending
        let deps = graph.uncalibrated_dependencies(&vol).unwrap();
        assert_eq!(deps.len(), 2);
        // Should be in order: C1, C2
        assert_eq!(deps[0], c1);
        assert_eq!(deps[1], c2);
    }

    #[test]
    fn test_calibration_sequence_for_simple() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Get sequence for vol
        let seq = graph.calibration_sequence_for(&vol_id).unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0], curve_id);
        assert_eq!(seq[1], vol_id);
    }

    #[test]
    fn test_calibration_sequence_for_partially_calibrated() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Calibrate the curve
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        // Only vol needs calibration
        let seq = graph.calibration_sequence_for(&vol_id).unwrap();
        assert_eq!(seq.len(), 1);
        assert_eq!(seq[0], vol_id);
    }

    #[test]
    fn test_calibration_sequence_for_fully_calibrated() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Calibrate both
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();
        graph.get_node_mut(&vol_id).unwrap().mark_calibrated();

        // Nothing needs calibration
        let seq = graph.calibration_sequence_for(&vol_id).unwrap();
        assert!(seq.is_empty());
    }

    #[test]
    fn test_calibration_sequence_for_complex() {
        let mut graph = CalibrationGraph::new();

        // DISC <- PROJ <- VOL
        //      \--------- VOL
        let disc = graph.add_curve("DISC", "Discount").unwrap();
        let proj = graph.add_curve("PROJ", "Projection").unwrap();
        let vol = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&proj, &disc).unwrap();
        graph.add_dependency(&vol, &disc).unwrap();
        graph.add_dependency(&vol, &proj).unwrap();

        let seq = graph.calibration_sequence_for(&vol).unwrap();
        assert_eq!(seq.len(), 3);

        // DISC must come before PROJ, PROJ must come before VOL
        let disc_pos = seq.iter().position(|id| *id == disc).unwrap();
        let proj_pos = seq.iter().position(|id| *id == proj).unwrap();
        let vol_pos = seq.iter().position(|id| *id == vol).unwrap();

        assert!(disc_pos < proj_pos);
        assert!(proj_pos < vol_pos);
    }

    #[test]
    fn test_is_ready_for_calibration() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Curve is ready (no dependencies, needs calibration)
        assert!(graph.is_ready_for_calibration(&curve_id).unwrap());

        // Vol is not ready (dependency not calibrated)
        assert!(!graph.is_ready_for_calibration(&vol_id).unwrap());

        // Calibrate curve
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        // Curve no longer ready (already calibrated)
        assert!(!graph.is_ready_for_calibration(&curve_id).unwrap());

        // Vol is now ready
        assert!(graph.is_ready_for_calibration(&vol_id).unwrap());
    }

    #[test]
    fn test_is_ready_for_calibration_computing() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        // Mark as computing
        graph.get_node_mut(&curve_id).unwrap().mark_computing();

        // Not ready while computing
        assert!(!graph.is_ready_for_calibration(&curve_id).unwrap());
    }

    #[test]
    fn test_ready_nodes() {
        let mut graph = CalibrationGraph::new();

        let c1 = graph.add_curve("C1", "Curve1").unwrap();
        let c2 = graph.add_curve("C2", "Curve2").unwrap();
        let vol = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol, &c1).unwrap();
        graph.add_dependency(&vol, &c2).unwrap();

        // Initially, both curves are ready
        let ready = graph.ready_nodes();
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&c1));
        assert!(ready.contains(&c2));

        // Calibrate c1
        graph.get_node_mut(&c1).unwrap().mark_calibrated();

        // Now only c2 is ready
        let ready = graph.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&c2));

        // Calibrate c2
        graph.get_node_mut(&c2).unwrap().mark_calibrated();

        // Now vol is ready
        let ready = graph.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&vol));
    }

    #[test]
    fn test_ready_nodes_stale() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        // Calibrate then mark stale
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();
        graph.get_node_mut(&curve_id).unwrap().mark_stale();

        // Stale node is ready for recalibration
        let ready = graph.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&curve_id));
    }

    // ========================================
    // Lazy Calibration Execution Tests
    // ========================================

    use std::sync::Arc;

    use parking_lot::Mutex;

    /// Test executor that records calibration calls.
    struct RecordingExecutor {
        calls: Arc<Mutex<Vec<(CalibrationNodeId, NodeKind)>>>,
    }

    impl RecordingExecutor {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<(CalibrationNodeId, NodeKind)> { self.calls.lock().clone() }
    }

    impl CalibrationExecutor for RecordingExecutor {
        fn calibrate(&self, id: &CalibrationNodeId, kind: NodeKind) -> GraphCalibrationResult {
            self.calls.lock().push((id.clone(), kind));
            Ok(())
        }
    }

    #[test]
    fn test_ensure_calibrated_simple() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        let executor = RecordingExecutor::new();
        graph.ensure_calibrated(&curve_id, &executor).unwrap();

        // Should have calibrated the curve
        let calls = executor.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, curve_id);
        assert_eq!(calls[0].1, NodeKind::Curve);

        // Node should be calibrated
        assert!(graph.get_node(&curve_id).unwrap().state.is_calibrated());
    }

    #[test]
    fn test_ensure_calibrated_with_dependency() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        let executor = RecordingExecutor::new();
        graph.ensure_calibrated(&vol_id, &executor).unwrap();

        // Should have calibrated curve first, then vol
        let calls = executor.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, curve_id);
        assert_eq!(calls[0].1, NodeKind::Curve);
        assert_eq!(calls[1].0, vol_id);
        assert_eq!(calls[1].1, NodeKind::VolCube);

        // Both should be calibrated
        assert!(graph.get_node(&curve_id).unwrap().state.is_calibrated());
        assert!(graph.get_node(&vol_id).unwrap().state.is_calibrated());
    }

    #[test]
    fn test_ensure_calibrated_already_calibrated() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        // Pre-calibrate
        graph.get_node_mut(&curve_id).unwrap().mark_calibrated();

        let executor = RecordingExecutor::new();
        graph.ensure_calibrated(&curve_id, &executor).unwrap();

        // Should not have called executor (already calibrated)
        let calls = executor.calls();
        assert!(calls.is_empty());
    }

    #[test]
    fn test_ensure_calibrated_complex_chain() {
        let mut graph = CalibrationGraph::new();

        // DISC <- PROJ <- VOL
        let disc = graph.add_curve("DISC", "Discount").unwrap();
        let proj = graph.add_curve("PROJ", "Projection").unwrap();
        let vol = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&proj, &disc).unwrap();
        graph.add_dependency(&vol, &proj).unwrap();

        let executor = RecordingExecutor::new();
        graph.ensure_calibrated(&vol, &executor).unwrap();

        // Should have calibrated in order: DISC, PROJ, VOL
        let calls = executor.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, disc);
        assert_eq!(calls[1].0, proj);
        assert_eq!(calls[2].0, vol);
    }

    #[test]
    fn test_ensure_calibrated_partial_chain() {
        let mut graph = CalibrationGraph::new();

        // DISC <- PROJ <- VOL
        let disc = graph.add_curve("DISC", "Discount").unwrap();
        let proj = graph.add_curve("PROJ", "Projection").unwrap();
        let vol = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&proj, &disc).unwrap();
        graph.add_dependency(&vol, &proj).unwrap();

        // Pre-calibrate DISC
        graph.get_node_mut(&disc).unwrap().mark_calibrated();

        let executor = RecordingExecutor::new();
        graph.ensure_calibrated(&vol, &executor).unwrap();

        // Should have calibrated only PROJ and VOL
        let calls = executor.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, proj);
        assert_eq!(calls[1].0, vol);
    }

    #[test]
    fn test_ensure_calibrated_all() {
        let mut graph = CalibrationGraph::new();

        // Two independent VolCubes with shared curve
        // DISC <- VOL1
        // DISC <- VOL2
        let disc = graph.add_curve("DISC", "Discount").unwrap();
        let vol1 = graph.add_volcube("VOL1", "Vol1").unwrap();
        let vol2 = graph.add_volcube("VOL2", "Vol2").unwrap();

        graph.add_dependency(&vol1, &disc).unwrap();
        graph.add_dependency(&vol2, &disc).unwrap();

        let executor = RecordingExecutor::new();
        graph
            .ensure_calibrated_all(&[vol1.clone(), vol2.clone()], &executor)
            .unwrap();

        // Should have calibrated DISC once, then both vols
        let calls = executor.calls();
        assert_eq!(calls.len(), 3);
        // DISC should be first
        assert_eq!(calls[0].0, disc);
    }

    /// Test executor that fails on specific nodes.
    struct FailingExecutor {
        fail_on: HashSet<CalibrationNodeId>,
    }

    impl FailingExecutor {
        fn failing_on(ids: &[CalibrationNodeId]) -> Self {
            Self {
                fail_on: ids.iter().cloned().collect(),
            }
        }
    }

    impl CalibrationExecutor for FailingExecutor {
        fn calibrate(&self, id: &CalibrationNodeId, _kind: NodeKind) -> GraphCalibrationResult {
            if self.fail_on.contains(id) {
                Err(GraphError::NodeNotFound { id: id.clone() }) // Using as a
                                                                 // generic error
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_ensure_calibrated_failure_marks_failed() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        let executor = FailingExecutor::failing_on(&[curve_id.clone()]);
        let result = graph.ensure_calibrated(&curve_id, &executor);

        // Should have failed
        assert!(result.is_err());

        // Node should be marked as failed
        assert_eq!(
            graph.get_node(&curve_id).unwrap().state,
            CalibrationState::Failed
        );
    }

    #[test]
    fn test_ensure_calibrated_stops_on_failure() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();
        let vol_id = graph.add_volcube("VOL", "Vol").unwrap();

        graph.add_dependency(&vol_id, &curve_id).unwrap();

        // Fail on curve
        let executor = FailingExecutor::failing_on(&[curve_id.clone()]);
        let result = graph.ensure_calibrated(&vol_id, &executor);

        // Should have failed
        assert!(result.is_err());

        // Curve should be failed, vol should still be pending
        assert_eq!(
            graph.get_node(&curve_id).unwrap().state,
            CalibrationState::Failed
        );
        assert_eq!(
            graph.get_node(&vol_id).unwrap().state,
            CalibrationState::Pending
        );
    }

    #[test]
    fn test_noop_executor() {
        let mut graph = CalibrationGraph::new();
        let curve_id = graph.add_curve("CURVE", "Curve").unwrap();

        let executor = NoOpCalibrationExecutor;
        graph.ensure_calibrated(&curve_id, &executor).unwrap();

        // Should be calibrated (no-op succeeds)
        assert!(graph.get_node(&curve_id).unwrap().state.is_calibrated());
    }
}

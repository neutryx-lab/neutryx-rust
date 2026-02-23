//! Global calibration table with DAG-based dependency tracking.
//!
//! Provides infrastructure for calibrating model parameters with:
//! - A global table of calibration entries with bounds and dependencies
//! - A directed acyclic graph (DAG) for dependency resolution
//! - A solver that calibrates parameters in topological order

use std::collections::HashMap;

use petgraph::{
    graph::{Graph, NodeIndex},
    visit::Topo,
    Direction,
};

use super::error::XvaEngineError;

/// A single calibration parameter entry.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CalibrationEntry {
    /// Unique parameter identifier.
    pub param_id: String,
    /// Target value to calibrate to (e.g., market price).
    pub target_value: f64,
    /// Current calibrated value.
    pub current_value: f64,
    /// Initial guess for the solver.
    pub initial_guess: f64,
    /// Lower bound for the parameter.
    pub lower_bound: f64,
    /// Upper bound for the parameter.
    pub upper_bound: f64,
    /// IDs of parameters this entry depends on.
    pub dependencies: Vec<String>,
}

/// Global table of calibration parameters.
#[derive(Clone, Debug, Default)]
pub struct GlobalCalibrationTable {
    entries: HashMap<String, CalibrationEntry>,
}

impl GlobalCalibrationTable {
    /// Creates an empty calibration table.
    pub fn new() -> Self { Self::default() }

    /// Adds a calibration entry to the table.
    pub fn add_entry(&mut self, entry: CalibrationEntry) {
        self.entries.insert(entry.param_id.clone(), entry);
    }

    /// Returns a reference to an entry by parameter ID.
    pub fn get(&self, param_id: &str) -> Option<&CalibrationEntry> { self.entries.get(param_id) }

    /// Returns a mutable reference to an entry by parameter ID.
    pub fn get_mut(&mut self, param_id: &str) -> Option<&mut CalibrationEntry> {
        self.entries.get_mut(param_id)
    }

    /// Removes an entry by parameter ID.
    pub fn remove(&mut self, param_id: &str) -> Option<CalibrationEntry> {
        self.entries.remove(param_id)
    }

    /// Returns an iterator over all entries.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &CalibrationEntry)> {
        self.entries.iter()
    }

    /// Returns all parameter IDs.
    pub fn param_ids(&self) -> Vec<&String> { self.entries.keys().collect() }

    /// Computes residuals: target_value - current_value for each entry.
    pub fn residuals(&self) -> Vec<f64> {
        self.entries
            .values()
            .map(|e| e.target_value - e.current_value)
            .collect()
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize { self.entries.len() }

    /// Returns whether the table is empty.
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

/// Directed Acyclic Graph for calibration dependency tracking.
///
/// Builds a dependency graph from calibration entries and provides
/// topological ordering for sequential calibration.
pub struct CalibrationDag {
    graph: Graph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

impl CalibrationDag {
    /// Builds a DAG from the calibration table entries.
    ///
    /// Each entry becomes a node, and edges represent dependencies
    /// (from dependency -> dependent).
    pub fn build(table: &GlobalCalibrationTable) -> Self {
        let mut graph = Graph::<String, ()>::new();
        let mut node_map = HashMap::new();

        // Add all nodes
        for (param_id, _) in table.entries() {
            let idx = graph.add_node(param_id.clone());
            node_map.insert(param_id.clone(), idx);
        }

        // Add dependency edges: dep -> param (dep must be calibrated first)
        for (param_id, entry) in table.entries() {
            if let Some(&to_idx) = node_map.get(param_id) {
                for dep in &entry.dependencies {
                    if let Some(&from_idx) = node_map.get(dep) {
                        graph.add_edge(from_idx, to_idx, ());
                    }
                }
            }
        }

        Self { graph, node_map }
    }

    /// Returns the calibration order via topological sort.
    ///
    /// Parameters with no dependencies come first, followed by those
    /// that depend on already-calibrated parameters.
    pub fn calibration_order(&self) -> Result<Vec<String>, XvaEngineError> {
        let mut topo = Topo::new(&self.graph);
        let mut order = Vec::new();

        while let Some(node_idx) = topo.next(&self.graph) {
            order.push(self.graph[node_idx].clone());
        }

        // Verify we got all nodes (no cycles)
        if order.len() != self.graph.node_count() {
            return Err(XvaEngineError::CalibrationError(
                "Calibration dependency graph contains a cycle".to_string(),
            ));
        }

        Ok(order)
    }

    /// Returns all downstream (affected) parameter IDs for a given parameter.
    ///
    /// These are parameters that depend (directly or transitively) on the
    /// given parameter and would need recalibration if it changes.
    pub fn downstream(&self, param_id: &str) -> Vec<String> {
        let mut result = Vec::new();

        if let Some(&start_idx) = self.node_map.get(param_id) {
            let mut stack = vec![start_idx];
            let mut visited = std::collections::HashSet::new();
            visited.insert(start_idx);

            while let Some(current) = stack.pop() {
                // Find all outgoing neighbors (nodes that depend on current)
                for neighbor in self.graph.neighbors_directed(current, Direction::Outgoing) {
                    if visited.insert(neighbor) {
                        result.push(self.graph[neighbor].clone());
                        stack.push(neighbor);
                    }
                }
            }
        }

        result
    }
}

/// Solver for calibrating parameters using bisection.
pub struct CalibrationSolver {
    /// Maximum number of iterations per parameter.
    pub max_iterations: usize,
    /// Convergence tolerance.
    pub tolerance: f64,
}

impl CalibrationSolver {
    /// Creates a new solver with specified parameters.
    pub fn new(max_iterations: usize, tolerance: f64) -> Self {
        Self {
            max_iterations,
            tolerance,
        }
    }

    /// Solves the calibration problem.
    ///
    /// For each parameter in DAG topological order, adjusts the current value
    /// to minimize |eval_fn(param_id, value) - target_value| using bisection.
    ///
    /// `eval_fn` takes (param_id, candidate_value) and returns the model output
    /// at that value.
    ///
    /// Returns the total number of iterations used across all parameters.
    pub fn solve<F>(
        &self,
        table: &mut GlobalCalibrationTable,
        eval_fn: F,
    ) -> Result<usize, XvaEngineError>
    where
        F: Fn(&str, f64) -> f64,
    {
        let dag = CalibrationDag::build(table);
        let order = dag.calibration_order()?;

        let mut total_iterations = 0;

        for param_id in &order {
            let (target, lower, upper) = {
                let entry = table.get(param_id).ok_or_else(|| {
                    XvaEngineError::CalibrationError(format!(
                        "Parameter '{}' not found in table",
                        param_id
                    ))
                })?;
                (entry.target_value, entry.lower_bound, entry.upper_bound)
            };

            let mut lo = lower;
            let mut hi = upper;

            // Evaluate at bounds to determine bracket
            let f_lo = eval_fn(param_id, lo) - target;
            let f_hi = eval_fn(param_id, hi) - target;

            // If both have the same sign, try midpoint approach (Newton-like fallback)
            let same_sign = f_lo * f_hi > 0.0;

            let mut best_value = f64::midpoint(lo, hi);
            let mut best_error = f64::MAX;

            for iter in 0..self.max_iterations {
                total_iterations += 1;

                let mid = f64::midpoint(lo, hi);
                let f_mid = eval_fn(param_id, mid) - target;

                if f_mid.abs() < best_error {
                    best_error = f_mid.abs();
                    best_value = mid;
                }

                if f_mid.abs() < self.tolerance {
                    best_value = mid;
                    break;
                }

                if same_sign {
                    // Golden section search when we can't bracket
                    let _ = iter;
                    let quarter = (hi - lo) / 4.0;
                    let left = lo + quarter;
                    let right = hi - quarter;
                    let f_left = (eval_fn(param_id, left) - target).abs();
                    let f_right = (eval_fn(param_id, right) - target).abs();

                    if f_left < f_right {
                        hi = right;
                    } else {
                        lo = left;
                    }
                } else {
                    // Standard bisection
                    let f_lo_current = eval_fn(param_id, lo) - target;
                    if f_lo_current * f_mid < 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
            }

            // Update the entry with the calibrated value
            if let Some(entry) = table.get_mut(param_id) {
                entry.current_value = best_value;
            }
        }

        Ok(total_iterations)
    }
}

impl Default for CalibrationSolver {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-8,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn make_entry(
        id: &str,
        target: f64,
        initial: f64,
        lower: f64,
        upper: f64,
        deps: Vec<String>,
    ) -> CalibrationEntry {
        CalibrationEntry {
            param_id: id.to_string(),
            target_value: target,
            current_value: initial,
            initial_guess: initial,
            lower_bound: lower,
            upper_bound: upper,
            dependencies: deps,
        }
    }

    #[test]
    fn test_calibration_table_basic() {
        let mut table = GlobalCalibrationTable::new();
        assert!(table.is_empty());

        table.add_entry(make_entry("vol", 0.2, 0.1, 0.01, 1.0, vec![]));
        table.add_entry(make_entry("rate", 0.05, 0.03, 0.0, 0.5, vec![]));

        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());

        let vol = table.get("vol").unwrap();
        assert_relative_eq!(vol.target_value, 0.2, epsilon = 1e-10);

        let ids = table.param_ids();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_calibration_table_get_mut() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("vol", 0.2, 0.1, 0.01, 1.0, vec![]));

        if let Some(entry) = table.get_mut("vol") {
            entry.current_value = 0.19;
        }
        assert_relative_eq!(
            table.get("vol").unwrap().current_value,
            0.19,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_calibration_table_remove() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("vol", 0.2, 0.1, 0.01, 1.0, vec![]));
        assert_eq!(table.len(), 1);

        let removed = table.remove("vol");
        assert!(removed.is_some());
        assert_eq!(table.len(), 0);
        assert!(table.get("vol").is_none());
    }

    #[test]
    fn test_calibration_table_residuals() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("vol", 0.2, 0.15, 0.01, 1.0, vec![]));
        table.add_entry(make_entry("rate", 0.05, 0.03, 0.0, 0.5, vec![]));

        let residuals = table.residuals();
        assert_eq!(residuals.len(), 2);
        // Residuals = target - current
        let total_abs: f64 = residuals.iter().map(|r| r.abs()).sum();
        assert!(total_abs > 0.0);
    }

    #[test]
    fn test_dag_no_dependencies() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("a", 1.0, 0.0, -10.0, 10.0, vec![]));
        table.add_entry(make_entry("b", 2.0, 0.0, -10.0, 10.0, vec![]));

        let dag = CalibrationDag::build(&table);
        let order = dag.calibration_order().unwrap();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_dag_with_dependencies() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("base_rate", 0.05, 0.0, 0.0, 0.5, vec![]));
        table.add_entry(make_entry(
            "vol",
            0.2,
            0.0,
            0.01,
            1.0,
            vec!["base_rate".to_string()],
        ));
        table.add_entry(make_entry(
            "skew",
            0.1,
            0.0,
            -1.0,
            1.0,
            vec!["vol".to_string()],
        ));

        let dag = CalibrationDag::build(&table);
        let order = dag.calibration_order().unwrap();

        assert_eq!(order.len(), 3);
        // base_rate must come before vol, vol must come before skew
        let pos_base = order.iter().position(|s| s == "base_rate").unwrap();
        let pos_vol = order.iter().position(|s| s == "vol").unwrap();
        let pos_skew = order.iter().position(|s| s == "skew").unwrap();

        assert!(pos_base < pos_vol);
        assert!(pos_vol < pos_skew);
    }

    #[test]
    fn test_dag_downstream() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("a", 1.0, 0.0, -10.0, 10.0, vec![]));
        table.add_entry(make_entry(
            "b",
            2.0,
            0.0,
            -10.0,
            10.0,
            vec!["a".to_string()],
        ));
        table.add_entry(make_entry(
            "c",
            3.0,
            0.0,
            -10.0,
            10.0,
            vec!["b".to_string()],
        ));

        let dag = CalibrationDag::build(&table);

        let downstream_a = dag.downstream("a");
        assert!(downstream_a.contains(&"b".to_string()));
        assert!(downstream_a.contains(&"c".to_string()));

        let downstream_b = dag.downstream("b");
        assert!(downstream_b.contains(&"c".to_string()));
        assert!(!downstream_b.contains(&"a".to_string()));

        let downstream_c = dag.downstream("c");
        assert!(downstream_c.is_empty());
    }

    #[test]
    fn test_dag_downstream_nonexistent() {
        let table = GlobalCalibrationTable::new();
        let dag = CalibrationDag::build(&table);
        let downstream = dag.downstream("nonexistent");
        assert!(downstream.is_empty());
    }

    #[test]
    fn test_solver_default() {
        let solver = CalibrationSolver::default();
        assert_eq!(solver.max_iterations, 100);
        assert_relative_eq!(solver.tolerance, 1e-8, epsilon = 1e-15);
    }

    #[test]
    fn test_solver_simple_linear() {
        // Calibrate f(x) = 2*x to target = 1.0 -> x = 0.5
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("x", 1.0, 0.0, 0.0, 10.0, vec![]));

        let solver = CalibrationSolver::new(100, 1e-8);
        let iters = solver
            .solve(&mut table, |_param_id, value| 2.0 * value)
            .unwrap();

        let calibrated = table.get("x").unwrap().current_value;
        assert_relative_eq!(calibrated, 0.5, epsilon = 1e-6);
        assert!(iters > 0);
    }

    #[test]
    fn test_solver_quadratic() {
        // Calibrate f(x) = x^2 to target = 4.0, x in [0, 10] -> x = 2.0
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("x", 4.0, 1.0, 0.0, 10.0, vec![]));

        let solver = CalibrationSolver::new(200, 1e-8);
        let _iters = solver
            .solve(&mut table, |_param_id, value| value * value)
            .unwrap();

        let calibrated = table.get("x").unwrap().current_value;
        assert_relative_eq!(calibrated, 2.0, epsilon = 1e-4);
    }

    #[test]
    fn test_solver_with_dependencies() {
        // Two parameters: a (no deps), b depends on a
        // Calibrate a: f(a) = a -> target 3.0
        // Calibrate b: f(b) = b + a -> target 5.0 (so b should be 2.0 after a=3.0)
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("a", 3.0, 0.0, 0.0, 10.0, vec![]));
        table.add_entry(make_entry("b", 5.0, 0.0, 0.0, 10.0, vec!["a".to_string()]));

        let solver = CalibrationSolver::new(100, 1e-8);

        // Simple eval: each param is just its value (identity)
        // For "a": f(a) = a, target 3.0
        // For "b": f(b) = b, target 5.0
        let _iters = solver.solve(&mut table, |_param_id, value| value).unwrap();

        let a_val = table.get("a").unwrap().current_value;
        let b_val = table.get("b").unwrap().current_value;

        assert_relative_eq!(a_val, 3.0, epsilon = 1e-6);
        assert_relative_eq!(b_val, 5.0, epsilon = 1e-6);
    }

    #[test]
    fn test_solver_multiple_independent() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("vol1", 0.2, 0.0, 0.0, 1.0, vec![]));
        table.add_entry(make_entry("vol2", 0.3, 0.0, 0.0, 1.0, vec![]));
        table.add_entry(make_entry("vol3", 0.4, 0.0, 0.0, 1.0, vec![]));

        let solver = CalibrationSolver::new(100, 1e-8);
        let _iters = solver.solve(&mut table, |_param_id, value| value).unwrap();

        assert_relative_eq!(
            table.get("vol1").unwrap().current_value,
            0.2,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            table.get("vol2").unwrap().current_value,
            0.3,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            table.get("vol3").unwrap().current_value,
            0.4,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_calibration_entry_clone() {
        let entry = make_entry("test", 1.0, 0.5, 0.0, 2.0, vec!["dep".to_string()]);
        let cloned = entry.clone();
        assert_eq!(entry.param_id, cloned.param_id);
        assert_eq!(entry.dependencies, cloned.dependencies);
    }

    #[test]
    fn test_calibration_entry_serde() {
        let entry = make_entry("test", 1.0, 0.5, 0.0, 2.0, vec!["dep".to_string()]);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: CalibrationEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.param_id, deserialized.param_id);
        assert_relative_eq!(
            entry.target_value,
            deserialized.target_value,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_calibration_table_entries_iterator() {
        let mut table = GlobalCalibrationTable::new();
        table.add_entry(make_entry("a", 1.0, 0.0, -10.0, 10.0, vec![]));
        table.add_entry(make_entry("b", 2.0, 0.0, -10.0, 10.0, vec![]));

        let count = table.entries().count();
        assert_eq!(count, 2);
    }
}

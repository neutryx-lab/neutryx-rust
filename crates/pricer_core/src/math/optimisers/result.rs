//! Result types for optimisation algorithms.

/// Result of an optimisation run.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimisationResult {
    /// Optimal parameter values found.
    pub params: Vec<f64>,
    /// Final objective function value.
    pub value: f64,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Number of function evaluations.
    pub func_evals: usize,
    /// Whether the optimisation converged.
    pub converged: bool,
    /// Optional convergence message.
    pub message: Option<String>,
}

impl OptimisationResult {
    /// Create a new optimisation result.
    #[must_use]
    pub fn new(
        params: Vec<f64>,
        value: f64,
        iterations: usize,
        func_evals: usize,
        converged: bool,
    ) -> Self {
        Self {
            params,
            value,
            iterations,
            func_evals,
            converged,
            message: None,
        }
    }

    /// Set a convergence message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the optimal parameters.
    #[must_use]
    pub fn params(&self) -> &[f64] { &self.params }

    /// Get the optimal value.
    #[must_use]
    pub fn value(&self) -> f64 { self.value }

    /// Check if optimisation converged.
    #[must_use]
    pub fn converged(&self) -> bool { self.converged }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_result() {
        let result = OptimisationResult::new(vec![1.0, 2.0], 0.5, 100, 200, true);
        assert_eq!(result.params, vec![1.0, 2.0]);
        assert!((result.value - 0.5).abs() < 1e-15);
        assert_eq!(result.iterations, 100);
        assert_eq!(result.func_evals, 200);
        assert!(result.converged);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_with_message() {
        let result = OptimisationResult::new(vec![1.0], 0.0, 10, 20, true).with_message("Success!");
        assert_eq!(result.message, Some("Success!".to_string()));
    }

    #[test]
    fn test_accessors() {
        let result = OptimisationResult::new(vec![1.0, 2.0, 3.0], 1.5, 50, 100, false);
        assert_eq!(result.params().len(), 3);
        assert!((result.value() - 1.5).abs() < 1e-15);
        assert!(!result.converged());
    }

    #[test]
    fn test_clone() {
        let result1 = OptimisationResult::new(vec![1.0], 0.5, 10, 20, true);
        let result2 = result1.clone();
        assert_eq!(result1, result2);
    }
}

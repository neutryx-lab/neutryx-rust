//! FX Volatility Surface Sensitivity calculations.
//!
//! This module provides AAD-compatible sensitivity calculations for
//! FX volatility surfaces, supporting both forward and reverse mode
//! automatic differentiation.

use num_traits::Float;
use serde::{Deserialize, Serialize};

/// Volatility surface sensitivities with respect to input quotes.
///
/// This structure captures the partial derivatives of output volatilities
/// with respect to input market quotes (ATM, Butterfly, Risk Reversal).
///
/// # Example
///
/// ```ignore
/// let sensitivity = VolSurfaceSensitivity::new(
///     d_vol_d_atm,     // ∂σ/∂ATM
///     d_vol_d_bf,      // ∂σ/∂BF
///     d_vol_d_rr,      // ∂σ/∂RR
/// );
///
/// // Chain with vega for complete sensitivity
/// let d_price_d_atm = sensitivity.d_vol_d_atm * vega;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolSurfaceSensitivity<T: Float> {
    /// Derivative of volatility with respect to ATM quote: ∂σ/∂ATM
    pub d_vol_d_atm: T,
    /// Derivative of volatility with respect to 25-delta Butterfly: ∂σ/∂BF
    pub d_vol_d_bf: T,
    /// Derivative of volatility with respect to 25-delta Risk Reversal: ∂σ/∂RR
    pub d_vol_d_rr: T,
}

impl<T: Float> VolSurfaceSensitivity<T> {
    /// Creates a new sensitivity structure.
    pub fn new(d_vol_d_atm: T, d_vol_d_bf: T, d_vol_d_rr: T) -> Self {
        Self {
            d_vol_d_atm,
            d_vol_d_bf,
            d_vol_d_rr,
        }
    }

    /// Creates zero sensitivities (for initialisation).
    pub fn zero() -> Self {
        Self {
            d_vol_d_atm: T::zero(),
            d_vol_d_bf: T::zero(),
            d_vol_d_rr: T::zero(),
        }
    }

    /// Scales all sensitivities by a factor.
    ///
    /// This is useful for chain rule application (e.g., multiplying by vega).
    pub fn scale(self, factor: T) -> Self {
        Self {
            d_vol_d_atm: self.d_vol_d_atm * factor,
            d_vol_d_bf: self.d_vol_d_bf * factor,
            d_vol_d_rr: self.d_vol_d_rr * factor,
        }
    }

    /// Adds two sensitivity structures element-wise.
    pub fn add(self, other: Self) -> Self {
        Self {
            d_vol_d_atm: self.d_vol_d_atm + other.d_vol_d_atm,
            d_vol_d_bf: self.d_vol_d_bf + other.d_vol_d_bf,
            d_vol_d_rr: self.d_vol_d_rr + other.d_vol_d_rr,
        }
    }
}

impl<T: Float + Default> Default for VolSurfaceSensitivity<T> {
    fn default() -> Self { Self::zero() }
}

/// Quote sensitivities at a specific expiry.
///
/// Captures how the smile at a given expiry depends on its input quotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpirySensitivity<T: Float> {
    /// Time to expiry in years.
    pub expiry: T,
    /// ATM sensitivity: how volatility at each delta responds to ATM change.
    pub atm_sens: T,
    /// Butterfly sensitivity: symmetric wing response.
    pub bf_sens: T,
    /// Risk reversal sensitivity: skew response.
    pub rr_sens: T,
}

impl<T: Float> ExpirySensitivity<T> {
    /// Creates a new expiry sensitivity.
    pub fn new(expiry: T, atm_sens: T, bf_sens: T, rr_sens: T) -> Self {
        Self {
            expiry,
            atm_sens,
            bf_sens,
            rr_sens,
        }
    }

    /// Converts to a VolSurfaceSensitivity at a given delta.
    ///
    /// # Arguments
    /// * `delta` - The delta at which to compute sensitivities (0 < delta < 1)
    /// * `is_call_side` - True for call side (delta > 0.5), false for put side
    pub fn to_vol_sensitivity(&self, delta: T, is_call_side: bool) -> VolSurfaceSensitivity<T> {
        // ATM always affects vol
        let d_vol_d_atm = self.atm_sens;

        // Butterfly is symmetric: affects both wings equally
        let d_vol_d_bf = self.bf_sens;

        // Risk reversal: asymmetric
        // Positive RR increases call-side vol, decreases put-side vol
        let half = T::from(0.5).unwrap();
        let d_vol_d_rr = if is_call_side {
            self.rr_sens * half
        } else {
            self.rr_sens * (-half)
        };

        VolSurfaceSensitivity::new(d_vol_d_atm, d_vol_d_bf, d_vol_d_rr)
    }
}

/// Smooth approximations for discontinuous operations.
///
/// These functions replace discontinuous operations with smooth versions
/// for AD compatibility.
pub mod smooth {
    use num_traits::Float;

    /// Default smoothing epsilon for AD compatibility.
    pub const DEFAULT_EPSILON: f64 = 1e-6;

    /// Smooth maximum function using log-sum-exp.
    ///
    /// Approximates max(a, b) using:
    /// smooth_max(a, b, ε) = ε * ln(exp(a/ε) + exp(b/ε))
    ///
    /// For large ε, approaches max(a, b) asymptotically.
    #[inline]
    pub fn smooth_max<T: Float>(a: T, b: T, epsilon: T) -> T {
        // For numerical stability, shift by the larger value
        let max_ab = if a > b { a } else { b };
        let shifted_a = (a - max_ab) / epsilon;
        let shifted_b = (b - max_ab) / epsilon;

        max_ab + epsilon * (shifted_a.exp() + shifted_b.exp()).ln()
    }

    /// Smooth minimum function.
    ///
    /// Approximates min(a, b) = -max(-a, -b)
    #[inline]
    pub fn smooth_min<T: Float>(a: T, b: T, epsilon: T) -> T { -smooth_max(-a, -b, epsilon) }

    /// Smooth indicator function (sigmoid).
    ///
    /// Approximates the step function:
    /// smooth_indicator(x, ε) = 1 / (1 + exp(-x/ε))
    ///
    /// Returns ~0 for x << 0, ~1 for x >> 0.
    #[inline]
    pub fn smooth_indicator<T: Float>(x: T, epsilon: T) -> T {
        T::one() / (T::one() + (-x / epsilon).exp())
    }

    /// Smooth absolute value function.
    ///
    /// Approximates |x| using sqrt(x² + ε²)
    #[inline]
    pub fn smooth_abs<T: Float>(x: T, epsilon: T) -> T { (x * x + epsilon * epsilon).sqrt() }

    /// Smooth clamp function.
    ///
    /// Smoothly clamps x to [low, high] range.
    #[inline]
    pub fn smooth_clamp<T: Float>(x: T, low: T, high: T, epsilon: T) -> T {
        let clamped_low = smooth_max(x, low, epsilon);
        smooth_min(clamped_low, high, epsilon)
    }
}

/// Sensitivity computation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SensitivityMode {
    /// Use analytical sensitivities where available.
    #[default]
    Analytical,
    /// Use finite differences for verification.
    FiniteDifference,
    /// Use automatic differentiation (Enzyme).
    AutomaticDifferentiation,
}

/// Configuration for sensitivity computation.
#[derive(Debug, Clone)]
pub struct SensitivityConfig {
    /// Computation mode.
    pub mode: SensitivityMode,
    /// Bump size for finite differences (as fraction, e.g., 0.0001 = 1bp).
    pub bump_size: f64,
    /// Smoothing epsilon for AD compatibility.
    pub smoothing_epsilon: f64,
}

impl Default for SensitivityConfig {
    fn default() -> Self {
        Self {
            mode: SensitivityMode::Analytical,
            bump_size: 0.0001, // 1bp
            smoothing_epsilon: smooth::DEFAULT_EPSILON,
        }
    }
}

impl SensitivityConfig {
    /// Creates configuration for finite difference mode.
    pub fn finite_difference(bump_size: f64) -> Self {
        Self {
            mode: SensitivityMode::FiniteDifference,
            bump_size,
            ..Default::default()
        }
    }

    /// Creates configuration for AD mode.
    pub fn automatic_differentiation(smoothing_epsilon: f64) -> Self {
        Self {
            mode: SensitivityMode::AutomaticDifferentiation,
            smoothing_epsilon,
            ..Default::default()
        }
    }
}

/// D3.js compatible JSON output for computation graph visualisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationGraphNode {
    /// Unique node identifier.
    pub id: String,
    /// Node type (input, intermediate, output).
    pub node_type: String,
    /// Node label for display.
    pub label: String,
    /// Node value (if available).
    pub value: Option<f64>,
}

/// Edge in the computation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationGraphEdge {
    /// Source node ID.
    pub source: String,
    /// Target node ID.
    pub target: String,
    /// Partial derivative value (if computed).
    pub derivative: Option<f64>,
}

/// D3.js compatible computation graph for visualisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationGraph {
    /// Nodes in the graph.
    pub nodes: Vec<ComputationGraphNode>,
    /// Edges connecting nodes.
    pub edges: Vec<ComputationGraphEdge>,
}

impl ComputationGraph {
    /// Creates a new empty computation graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds an input node to the graph.
    pub fn add_input(&mut self, id: &str, label: &str, value: Option<f64>) {
        self.nodes.push(ComputationGraphNode {
            id: id.to_string(),
            node_type: "input".to_string(),
            label: label.to_string(),
            value,
        });
    }

    /// Adds an output node to the graph.
    pub fn add_output(&mut self, id: &str, label: &str, value: Option<f64>) {
        self.nodes.push(ComputationGraphNode {
            id: id.to_string(),
            node_type: "output".to_string(),
            label: label.to_string(),
            value,
        });
    }

    /// Adds an intermediate node to the graph.
    pub fn add_intermediate(&mut self, id: &str, label: &str, value: Option<f64>) {
        self.nodes.push(ComputationGraphNode {
            id: id.to_string(),
            node_type: "intermediate".to_string(),
            label: label.to_string(),
            value,
        });
    }

    /// Adds an edge with derivative.
    pub fn add_edge(&mut self, source: &str, target: &str, derivative: Option<f64>) {
        self.edges.push(ComputationGraphEdge {
            source: source.to_string(),
            target: target.to_string(),
            derivative,
        });
    }

    /// Exports to JSON for D3.js visualisation.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl Default for ComputationGraph {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_surface_sensitivity_new() {
        let sens: VolSurfaceSensitivity<f64> = VolSurfaceSensitivity::new(1.0, 0.5, -0.2);

        assert!((sens.d_vol_d_atm - 1.0).abs() < 1e-10);
        assert!((sens.d_vol_d_bf - 0.5).abs() < 1e-10);
        assert!((sens.d_vol_d_rr - (-0.2)).abs() < 1e-10);
    }

    #[test]
    fn test_vol_surface_sensitivity_zero() {
        let sens: VolSurfaceSensitivity<f64> = VolSurfaceSensitivity::zero();

        assert_eq!(sens.d_vol_d_atm, 0.0);
        assert_eq!(sens.d_vol_d_bf, 0.0);
        assert_eq!(sens.d_vol_d_rr, 0.0);
    }

    #[test]
    fn test_vol_surface_sensitivity_scale() {
        let sens: VolSurfaceSensitivity<f64> = VolSurfaceSensitivity::new(1.0, 2.0, 3.0);
        let scaled = sens.scale(2.0);

        assert!((scaled.d_vol_d_atm - 2.0).abs() < 1e-10);
        assert!((scaled.d_vol_d_bf - 4.0).abs() < 1e-10);
        assert!((scaled.d_vol_d_rr - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_surface_sensitivity_add() {
        let sens1: VolSurfaceSensitivity<f64> = VolSurfaceSensitivity::new(1.0, 2.0, 3.0);
        let sens2: VolSurfaceSensitivity<f64> = VolSurfaceSensitivity::new(0.5, 1.0, -1.0);
        let sum = sens1.add(sens2);

        assert!((sum.d_vol_d_atm - 1.5).abs() < 1e-10);
        assert!((sum.d_vol_d_bf - 3.0).abs() < 1e-10);
        assert!((sum.d_vol_d_rr - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_expiry_sensitivity_to_vol_sensitivity_call() {
        let exp_sens = ExpirySensitivity::new(0.25, 1.0, 0.5, 0.4);
        let vol_sens = exp_sens.to_vol_sensitivity(0.75, true); // call side

        assert!((vol_sens.d_vol_d_atm - 1.0).abs() < 1e-10);
        assert!((vol_sens.d_vol_d_bf - 0.5).abs() < 1e-10);
        assert!((vol_sens.d_vol_d_rr - 0.2).abs() < 1e-10); // half of RR
    }

    #[test]
    fn test_expiry_sensitivity_to_vol_sensitivity_put() {
        let exp_sens = ExpirySensitivity::new(0.25, 1.0, 0.5, 0.4);
        let vol_sens = exp_sens.to_vol_sensitivity(0.25, false); // put side

        assert!((vol_sens.d_vol_d_atm - 1.0).abs() < 1e-10);
        assert!((vol_sens.d_vol_d_bf - 0.5).abs() < 1e-10);
        assert!((vol_sens.d_vol_d_rr - (-0.2)).abs() < 1e-10); // negative half
    }

    #[test]
    fn test_smooth_max() {
        let epsilon = 0.01;
        let result = smooth::smooth_max(1.0_f64, 2.0, epsilon);
        // Should be close to 2.0
        assert!((result - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_smooth_indicator() {
        let epsilon = 0.1;

        // Large positive -> ~1
        let pos = smooth::smooth_indicator(10.0_f64, epsilon);
        assert!(pos > 0.99);

        // Large negative -> ~0
        let neg = smooth::smooth_indicator(-10.0_f64, epsilon);
        assert!(neg < 0.01);

        // Zero -> 0.5
        let zero = smooth::smooth_indicator(0.0_f64, epsilon);
        assert!((zero - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_smooth_abs() {
        let epsilon = 0.001;
        let result = smooth::smooth_abs(-5.0_f64, epsilon);
        // Should be close to 5.0
        assert!((result - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_sensitivity_config_default() {
        let config = SensitivityConfig::default();
        assert_eq!(config.mode, SensitivityMode::Analytical);
        assert!((config.bump_size - 0.0001).abs() < 1e-10);
    }

    #[test]
    fn test_computation_graph() {
        let mut graph = ComputationGraph::new();

        graph.add_input("atm", "ATM Vol", Some(0.10));
        graph.add_input("bf", "Butterfly", Some(0.005));
        graph.add_output("vol_25d", "Vol at 25Δ", Some(0.105));
        graph.add_edge("atm", "vol_25d", Some(1.0));
        graph.add_edge("bf", "vol_25d", Some(1.0));

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        let json = graph.to_json().unwrap();
        assert!(json.contains("atm"));
        assert!(json.contains("vol_25d"));
    }

    #[test]
    fn test_sensitivity_config_finite_difference() {
        let config = SensitivityConfig::finite_difference(0.0005);
        assert_eq!(config.mode, SensitivityMode::FiniteDifference);
        assert!((config.bump_size - 0.0005).abs() < 1e-10);
    }

    #[test]
    fn test_sensitivity_config_ad() {
        let config = SensitivityConfig::automatic_differentiation(1e-8);
        assert_eq!(config.mode, SensitivityMode::AutomaticDifferentiation);
        assert!((config.smoothing_epsilon - 1e-8).abs() < 1e-12);
    }
}

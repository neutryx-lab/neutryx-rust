//! Sensitivity Path構築とCurve経由間接感応度計算。
//!
//! # Requirements: 6.11, 7.8, 7.9, 7.10
//!
//! このモジュールはCurveQuote→Price完全パスのAADグラフを構築し、
//! ∂SwaptionPrice/∂CurveQuoteのような間接感応度を計算する。
//!
//! # アーキテクチャ
//!
//! ```text
//! SensitivityPath
//! ├── CurveQuote層（入力）
//! │   └── Discount/Projection Rate Quotes
//! ├── Curve層（中間）
//! │   └── Calibrated Yield Curves
//! ├── ForwardRate層（中間）
//! │   └── Forward Swap Rates from Curves
//! ├── VolCube層（中間）
//! │   └── Calibrated Volatility Cube
//! └── Price層（出力）
//!     └── Swaption/CapFloor Prices
//! ```
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::{SensitivityPathBuilder, IndirectSensitivity};
//!
//! let path = SensitivityPathBuilder::new()
//!     .add_curve_quote("USD-SOFR-1Y", 0.05)
//!     .add_curve_quote("USD-SOFR-2Y", 0.045)
//!     .build_curve("USD-SOFR")
//!     .build_volcube("USD-SWAPTION-VOL")
//!     .add_price("SWAPTION-001", 1_000_000.0)
//!     .build()?;
//!
//! let sensitivity = path.calculate_indirect_sensitivity(
//!     "SWAPTION-001",
//!     "USD-SOFR-1Y",
//! )?;
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::vega::{VegaBumpConfig, VegaError};

// =============================================================================
// Sensitivity Node Types
// =============================================================================

/// 感応度パスのノード種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitivityNodeKind {
    /// Curve Quote（入力）。
    CurveQuote,
    /// Yield Curve。
    Curve,
    /// Forward Rate。
    ForwardRate,
    /// Vol Quote（入力）。
    VolQuote,
    /// VolCube。
    VolCube,
    /// Option Price（出力）。
    Price,
}

impl std::fmt::Display for SensitivityNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurveQuote => write!(f, "CurveQuote"),
            Self::Curve => write!(f, "Curve"),
            Self::ForwardRate => write!(f, "ForwardRate"),
            Self::VolQuote => write!(f, "VolQuote"),
            Self::VolCube => write!(f, "VolCube"),
            Self::Price => write!(f, "Price"),
        }
    }
}

// =============================================================================
// Sensitivity Node
// =============================================================================

/// 感応度パスのノード。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityNode {
    /// ノードID。
    pub id: String,
    /// ノード種別。
    pub kind: SensitivityNodeKind,
    /// ノード名。
    pub name: String,
    /// 現在の値。
    pub value: f64,
    /// 追加メタデータ。
    pub metadata: HashMap<String, String>,
}

impl SensitivityNode {
    /// 新しいノードを作成。
    pub fn new(
        id: impl Into<String>,
        kind: SensitivityNodeKind,
        name: impl Into<String>,
        value: f64,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            name: name.into(),
            value,
            metadata: HashMap::new(),
        }
    }

    /// CurveQuoteノードを作成。
    pub fn curve_quote(id: impl Into<String>, name: impl Into<String>, quote: f64) -> Self {
        Self::new(id, SensitivityNodeKind::CurveQuote, name, quote)
    }

    /// Curveノードを作成。
    pub fn curve(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(id, SensitivityNodeKind::Curve, name, 0.0)
    }

    /// ForwardRateノードを作成。
    pub fn forward_rate(id: impl Into<String>, expiry: f64, tenor: f64, rate: f64) -> Self {
        let name = format!("Fwd({:.1}Y,{:.0}Y)", expiry, tenor);
        let mut node = Self::new(id, SensitivityNodeKind::ForwardRate, name, rate);
        node.metadata
            .insert("expiry".to_string(), expiry.to_string());
        node.metadata.insert("tenor".to_string(), tenor.to_string());
        node
    }

    /// VolQuoteノードを作成。
    pub fn vol_quote(id: impl Into<String>, name: impl Into<String>, vol: f64) -> Self {
        Self::new(id, SensitivityNodeKind::VolQuote, name, vol)
    }

    /// VolCubeノードを作成。
    pub fn volcube(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(id, SensitivityNodeKind::VolCube, name, 0.0)
    }

    /// Priceノードを作成。
    pub fn price(id: impl Into<String>, name: impl Into<String>, price: f64) -> Self {
        Self::new(id, SensitivityNodeKind::Price, name, price)
    }

    /// メタデータを追加。
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// =============================================================================
// Sensitivity Edge
// =============================================================================

/// 感応度パスのエッジ（依存関係）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityEdge {
    /// ソースノードID。
    pub source: String,
    /// ターゲットノードID。
    pub target: String,
    /// エッジの重み（感応度）。
    pub weight: Option<f64>,
    /// エッジのラベル。
    pub label: Option<String>,
}

impl SensitivityEdge {
    /// 新しいエッジを作成。
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            weight: None,
            label: None,
        }
    }

    /// 重みを設定。
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// ラベルを設定。
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

// =============================================================================
// Sensitivity Path
// =============================================================================

/// 感応度パス（完全なAADグラフ）。
///
/// # Requirements: 6.11, 7.8
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensitivityPath {
    /// ノードマップ。
    nodes: HashMap<String, SensitivityNode>,
    /// エッジリスト。
    edges: Vec<SensitivityEdge>,
    /// ノードIDの順序（追加順）。
    node_order: Vec<String>,
}

impl SensitivityPath {
    /// 新しい空のパスを作成。
    pub fn new() -> Self { Self::default() }

    /// ノードを追加。
    pub fn add_node(&mut self, node: SensitivityNode) -> &mut Self {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        if !self.node_order.contains(&id) {
            self.node_order.push(id);
        }
        self
    }

    /// エッジを追加。
    pub fn add_edge(&mut self, edge: SensitivityEdge) -> &mut Self {
        self.edges.push(edge);
        self
    }

    /// ノードを取得。
    pub fn get_node(&self, id: &str) -> Option<&SensitivityNode> { self.nodes.get(id) }

    /// ノードを変更可能で取得。
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut SensitivityNode> {
        self.nodes.get_mut(id)
    }

    /// すべてのノードを取得。
    pub fn nodes(&self) -> impl Iterator<Item = &SensitivityNode> { self.nodes.values() }

    /// すべてのエッジを取得。
    pub fn edges(&self) -> &[SensitivityEdge] { &self.edges }

    /// ノード数を取得。
    pub fn node_count(&self) -> usize { self.nodes.len() }

    /// エッジ数を取得。
    pub fn edge_count(&self) -> usize { self.edges.len() }

    /// 特定の種別のノードを取得。
    pub fn nodes_of_kind(&self, kind: SensitivityNodeKind) -> Vec<&SensitivityNode> {
        self.nodes.values().filter(|n| n.kind == kind).collect()
    }

    /// 入力ノード（CurveQuote, VolQuote）を取得。
    pub fn input_nodes(&self) -> Vec<&SensitivityNode> {
        self.nodes
            .values()
            .filter(|n| {
                matches!(
                    n.kind,
                    SensitivityNodeKind::CurveQuote | SensitivityNodeKind::VolQuote
                )
            })
            .collect()
    }

    /// 出力ノード（Price）を取得。
    pub fn output_nodes(&self) -> Vec<&SensitivityNode> {
        self.nodes_of_kind(SensitivityNodeKind::Price)
    }

    /// ソースからターゲットへのパスを検索。
    pub fn find_path(&self, source: &str, target: &str) -> Option<Vec<String>> {
        use std::collections::{HashSet, VecDeque};

        if !self.nodes.contains_key(source) || !self.nodes.contains_key(target) {
            return None;
        }

        // BFSでパスを探索
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        queue.push_back(source.to_string());
        visited.insert(source.to_string());

        while let Some(current) = queue.pop_front() {
            if current == target {
                // パスを再構築
                let mut path = vec![target.to_string()];
                let mut node = target.to_string();
                while let Some(p) = parent.get(&node) {
                    path.push(p.clone());
                    node = p.clone();
                }
                path.reverse();
                return Some(path);
            }

            // 隣接ノードを探索
            for edge in &self.edges {
                if edge.source == current && !visited.contains(&edge.target) {
                    visited.insert(edge.target.clone());
                    parent.insert(edge.target.clone(), current.clone());
                    queue.push_back(edge.target.clone());
                }
            }
        }

        None
    }
}

// =============================================================================
// Indirect Sensitivity Calculator
// =============================================================================

/// 間接感応度計算結果。
///
/// # Requirements: 7.9, 7.10
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndirectSensitivity {
    /// 入力ノードID（例：CurveQuote）。
    pub input_id: String,
    /// 出力ノードID（例：Price）。
    pub output_id: String,
    /// 感応度値（∂Output/∂Input）。
    pub sensitivity: f64,
    /// 経路上のノードID。
    pub path: Vec<String>,
    /// 経路上の各ステップの感応度。
    pub path_sensitivities: Vec<f64>,
}

impl IndirectSensitivity {
    /// 新しい間接感応度を作成。
    pub fn new(
        input_id: impl Into<String>,
        output_id: impl Into<String>,
        sensitivity: f64,
    ) -> Self {
        Self {
            input_id: input_id.into(),
            output_id: output_id.into(),
            sensitivity,
            path: Vec::new(),
            path_sensitivities: Vec::new(),
        }
    }

    /// パスを設定。
    pub fn with_path(mut self, path: Vec<String>, sensitivities: Vec<f64>) -> Self {
        self.path = path;
        self.path_sensitivities = sensitivities;
        self
    }
}

/// 間接感応度計算器。
///
/// # Requirements: 7.9, 7.10
///
/// Bump-and-revalueまたはAAD（将来）を使用して
/// CurveQuote→Priceの間接感応度を計算する。
#[derive(Debug, Clone)]
pub struct IndirectSensitivityCalculator {
    /// バンプ設定。
    config: VegaBumpConfig,
}

impl IndirectSensitivityCalculator {
    /// 新しい計算器を作成。
    pub fn new(config: VegaBumpConfig) -> Self { Self { config } }

    /// デフォルト設定で計算器を作成。
    pub fn with_defaults() -> Self { Self::new(VegaBumpConfig::default()) }

    /// 間接感応度を計算（bump-and-revalue）。
    ///
    /// # Arguments
    ///
    /// * `path` - 感応度パス
    /// * `input_id` - 入力ノードID（バンプ対象）
    /// * `output_id` - 出力ノードID（感応度計算対象）
    /// * `revalue_fn` - 再評価関数 f(input_value) -> output_value
    ///
    /// # Returns
    ///
    /// 間接感応度結果。
    pub fn calculate<F>(
        &self,
        path: &SensitivityPath,
        input_id: &str,
        output_id: &str,
        revalue_fn: F,
    ) -> Result<IndirectSensitivity, VegaError>
    where
        F: Fn(f64) -> f64,
    {
        // 入力ノードを取得
        let input_node = path.get_node(input_id).ok_or_else(|| {
            VegaError::CalculationError(format!("Input node not found: {}", input_id))
        })?;

        // 出力ノードを取得
        let _output_node = path.get_node(output_id).ok_or_else(|| {
            VegaError::CalculationError(format!("Output node not found: {}", output_id))
        })?;

        // パスを検索
        let node_path = path.find_path(input_id, output_id).ok_or_else(|| {
            VegaError::CalculationError(format!("No path from {} to {}", input_id, output_id))
        })?;

        // バンプサイズを計算
        let base_value = input_node.value;
        let bump = self.config.compute_bump(base_value);

        // 感応度を計算
        let sensitivity = if self.config.use_central_difference {
            let output_up = revalue_fn(base_value + bump);
            let output_down = revalue_fn((base_value - bump).max(1e-10));
            (output_up - output_down) / (2.0 * bump)
        } else {
            let base_output = revalue_fn(base_value);
            let output_up = revalue_fn(base_value + bump);
            (output_up - base_output) / bump
        };

        Ok(IndirectSensitivity::new(input_id, output_id, sensitivity)
            .with_path(node_path, vec![sensitivity]))
    }

    /// 複数入力に対する間接感応度を計算。
    pub fn calculate_all<F>(
        &self,
        path: &SensitivityPath,
        output_id: &str,
        revalue_fn: F,
    ) -> Result<Vec<IndirectSensitivity>, VegaError>
    where
        F: Fn(&str, f64) -> f64,
    {
        let mut results = Vec::new();

        for input_node in path.input_nodes() {
            let input_id = &input_node.id;
            let base_value = input_node.value;
            let bump = self.config.compute_bump(base_value);

            // 感応度を計算
            let sensitivity = if self.config.use_central_difference {
                let output_up = revalue_fn(input_id, base_value + bump);
                let output_down = revalue_fn(input_id, (base_value - bump).max(1e-10));
                (output_up - output_down) / (2.0 * bump)
            } else {
                let base_output = revalue_fn(input_id, base_value);
                let output_up = revalue_fn(input_id, base_value + bump);
                (output_up - base_output) / bump
            };

            let node_path = path.find_path(input_id, output_id).unwrap_or_default();

            results.push(
                IndirectSensitivity::new(input_id, output_id, sensitivity)
                    .with_path(node_path, vec![sensitivity]),
            );
        }

        Ok(results)
    }
}

impl Default for IndirectSensitivityCalculator {
    fn default() -> Self { Self::with_defaults() }
}

// =============================================================================
// Sensitivity Path Builder
// =============================================================================

/// 感応度パスビルダー。
///
/// # Requirements: 7.8
#[derive(Debug, Clone, Default)]
pub struct SensitivityPathBuilder {
    path: SensitivityPath,
    current_curve: Option<String>,
    current_volcube: Option<String>,
}

impl SensitivityPathBuilder {
    /// 新しいビルダーを作成。
    pub fn new() -> Self { Self::default() }

    /// CurveQuoteを追加。
    pub fn add_curve_quote(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        quote: f64,
    ) -> Self {
        let node = SensitivityNode::curve_quote(id, name, quote);
        self.path.add_node(node);
        self
    }

    /// Curveを追加。
    pub fn add_curve(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        let curve_id: String = id.into();
        let node = SensitivityNode::curve(&curve_id, name);
        self.path.add_node(node);
        self.current_curve = Some(curve_id);
        self
    }

    /// CurveQuoteからCurveへの依存を追加。
    pub fn link_quote_to_curve(
        mut self,
        quote_id: impl Into<String>,
        curve_id: impl Into<String>,
    ) -> Self {
        let edge = SensitivityEdge::new(quote_id, curve_id);
        self.path.add_edge(edge);
        self
    }

    /// ForwardRateを追加。
    pub fn add_forward_rate(
        mut self,
        id: impl Into<String>,
        expiry: f64,
        tenor: f64,
        rate: f64,
    ) -> Self {
        let fwd_id: String = id.into();
        let node = SensitivityNode::forward_rate(&fwd_id, expiry, tenor, rate);
        self.path.add_node(node);

        // 現在のCurveからリンク
        if let Some(curve_id) = &self.current_curve {
            let edge = SensitivityEdge::new(curve_id.clone(), &fwd_id);
            self.path.add_edge(edge);
        }
        self
    }

    /// VolQuoteを追加。
    pub fn add_vol_quote(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        vol: f64,
    ) -> Self {
        let node = SensitivityNode::vol_quote(id, name, vol);
        self.path.add_node(node);
        self
    }

    /// VolCubeを追加。
    pub fn add_volcube(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        let volcube_id: String = id.into();
        let node = SensitivityNode::volcube(&volcube_id, name);
        self.path.add_node(node);
        self.current_volcube = Some(volcube_id);
        self
    }

    /// ForwardRateからVolCubeへの依存を追加。
    pub fn link_forward_to_volcube(
        mut self,
        forward_id: impl Into<String>,
        volcube_id: impl Into<String>,
    ) -> Self {
        let edge = SensitivityEdge::new(forward_id, volcube_id);
        self.path.add_edge(edge);
        self
    }

    /// VolQuoteからVolCubeへの依存を追加。
    pub fn link_vol_to_volcube(
        mut self,
        vol_id: impl Into<String>,
        volcube_id: impl Into<String>,
    ) -> Self {
        let edge = SensitivityEdge::new(vol_id, volcube_id);
        self.path.add_edge(edge);
        self
    }

    /// Priceを追加。
    pub fn add_price(mut self, id: impl Into<String>, name: impl Into<String>, price: f64) -> Self {
        let price_id: String = id.into();
        let node = SensitivityNode::price(&price_id, name, price);
        self.path.add_node(node);

        // 現在のVolCubeからリンク
        if let Some(volcube_id) = &self.current_volcube {
            let edge = SensitivityEdge::new(volcube_id.clone(), &price_id);
            self.path.add_edge(edge);
        }
        self
    }

    /// パスを構築。
    pub fn build(self) -> SensitivityPath { self.path }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitivity_node_creation() {
        let node = SensitivityNode::curve_quote("USD-1Y", "USD 1Y Rate", 0.05);
        assert_eq!(node.id, "USD-1Y");
        assert_eq!(node.kind, SensitivityNodeKind::CurveQuote);
        assert!((node.value - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_sensitivity_node_forward_rate() {
        let node = SensitivityNode::forward_rate("FWD-1Y5Y", 1.0, 5.0, 0.03);
        assert_eq!(node.kind, SensitivityNodeKind::ForwardRate);
        assert_eq!(node.metadata.get("expiry"), Some(&"1".to_string()));
        assert_eq!(node.metadata.get("tenor"), Some(&"5".to_string()));
    }

    #[test]
    fn test_sensitivity_path_add_nodes() {
        let mut path = SensitivityPath::new();

        path.add_node(SensitivityNode::curve_quote("Q1", "Quote 1", 0.05));
        path.add_node(SensitivityNode::curve("C1", "Curve 1"));
        path.add_edge(SensitivityEdge::new("Q1", "C1"));

        assert_eq!(path.node_count(), 2);
        assert_eq!(path.edge_count(), 1);
    }

    #[test]
    fn test_sensitivity_path_find_path() {
        let mut path = SensitivityPath::new();

        // Q1 -> C1 -> F1 -> V1 -> P1
        path.add_node(SensitivityNode::curve_quote("Q1", "Quote", 0.05));
        path.add_node(SensitivityNode::curve("C1", "Curve"));
        path.add_node(SensitivityNode::forward_rate("F1", 1.0, 5.0, 0.03));
        path.add_node(SensitivityNode::volcube("V1", "VolCube"));
        path.add_node(SensitivityNode::price("P1", "Price", 100.0));

        path.add_edge(SensitivityEdge::new("Q1", "C1"));
        path.add_edge(SensitivityEdge::new("C1", "F1"));
        path.add_edge(SensitivityEdge::new("F1", "V1"));
        path.add_edge(SensitivityEdge::new("V1", "P1"));

        let found_path = path.find_path("Q1", "P1");
        assert!(found_path.is_some());

        let p = found_path.unwrap();
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], "Q1");
        assert_eq!(p[4], "P1");
    }

    #[test]
    fn test_sensitivity_path_no_path() {
        let mut path = SensitivityPath::new();

        path.add_node(SensitivityNode::curve_quote("Q1", "Quote", 0.05));
        path.add_node(SensitivityNode::price("P1", "Price", 100.0));
        // No edge between them

        let found_path = path.find_path("Q1", "P1");
        assert!(found_path.is_none());
    }

    #[test]
    fn test_indirect_sensitivity_creation() {
        let sens = IndirectSensitivity::new("Q1", "P1", 1000.0);
        assert_eq!(sens.input_id, "Q1");
        assert_eq!(sens.output_id, "P1");
        assert!((sens.sensitivity - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_indirect_sensitivity_calculator() {
        let mut path = SensitivityPath::new();

        path.add_node(SensitivityNode::curve_quote("Q1", "Quote", 0.05));
        path.add_node(SensitivityNode::price("P1", "Price", 100.0));
        path.add_edge(SensitivityEdge::new("Q1", "P1"));

        let calc = IndirectSensitivityCalculator::with_defaults();

        // Simple linear relationship: price = quote * 2000
        let revalue_fn = |quote: f64| quote * 2000.0;

        let result = calc.calculate(&path, "Q1", "P1", revalue_fn);
        assert!(result.is_ok());

        let sens = result.unwrap();
        // Sensitivity should be ~2000
        assert!((sens.sensitivity - 2000.0).abs() < 1.0);
    }

    #[test]
    fn test_indirect_sensitivity_calculator_quadratic() {
        let mut path = SensitivityPath::new();

        path.add_node(SensitivityNode::curve_quote("Q1", "Quote", 0.05));
        path.add_node(SensitivityNode::price("P1", "Price", 100.0));
        path.add_edge(SensitivityEdge::new("Q1", "P1"));

        let calc = IndirectSensitivityCalculator::with_defaults();

        // Quadratic relationship: price = quote^2 * 10000
        let revalue_fn = |quote: f64| quote * quote * 10000.0;

        let result = calc.calculate(&path, "Q1", "P1", revalue_fn);
        assert!(result.is_ok());

        let sens = result.unwrap();
        // Sensitivity at quote=0.05: d/dq(q^2 * 10000) = 2q * 10000 = 1000
        assert!((sens.sensitivity - 1000.0).abs() < 10.0);
    }

    #[test]
    fn test_sensitivity_path_builder() {
        let path = SensitivityPathBuilder::new()
            .add_curve_quote("Q1", "USD 1Y", 0.05)
            .add_curve_quote("Q2", "USD 2Y", 0.045)
            .add_curve("C1", "USD-SOFR")
            .link_quote_to_curve("Q1", "C1")
            .link_quote_to_curve("Q2", "C1")
            .add_forward_rate("F1", 1.0, 5.0, 0.03)
            .add_volcube("V1", "USD-SWAPTION-VOL")
            .link_forward_to_volcube("F1", "V1")
            .add_price("P1", "SWAPTION-001", 100000.0)
            .build();

        assert_eq!(path.node_count(), 6);
        assert!(path.edge_count() >= 4);

        // Check input nodes
        let inputs = path.input_nodes();
        assert_eq!(inputs.len(), 2);

        // Check output nodes
        let outputs = path.output_nodes();
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn test_nodes_of_kind() {
        let path = SensitivityPathBuilder::new()
            .add_curve_quote("Q1", "Quote 1", 0.05)
            .add_curve_quote("Q2", "Quote 2", 0.045)
            .add_curve("C1", "Curve 1")
            .add_volcube("V1", "VolCube 1")
            .add_price("P1", "Price 1", 100.0)
            .build();

        assert_eq!(path.nodes_of_kind(SensitivityNodeKind::CurveQuote).len(), 2);
        assert_eq!(path.nodes_of_kind(SensitivityNodeKind::Curve).len(), 1);
        assert_eq!(path.nodes_of_kind(SensitivityNodeKind::VolCube).len(), 1);
        assert_eq!(path.nodes_of_kind(SensitivityNodeKind::Price).len(), 1);
    }

    #[test]
    fn test_sensitivity_edge_with_weight() {
        let edge = SensitivityEdge::new("A", "B")
            .with_weight(0.5)
            .with_label("calibration");

        assert_eq!(edge.source, "A");
        assert_eq!(edge.target, "B");
        assert_eq!(edge.weight, Some(0.5));
        assert_eq!(edge.label, Some("calibration".to_string()));
    }

    #[test]
    fn test_sensitivity_node_kind_display() {
        assert_eq!(format!("{}", SensitivityNodeKind::CurveQuote), "CurveQuote");
        assert_eq!(format!("{}", SensitivityNodeKind::Price), "Price");
    }
}

//! VolCube計算グラフ抽出サポート。
//!
//! # Requirements: 4.2, 4.3, 4.4
//!
//! このモジュールはVolCubeの計算グラフ情報を抽出するための
//! データ構造とヘルパー関数を提供する。
//!
//! # 設計
//!
//! `pricer_pricing::graph::GraphExtractable`トレイトの実装は
//! `pricer_pricing`側で行い、本モジュールは必要なデータを提供する。
//! これによりL2（pricer_models）からL3（pricer_pricing）への
//! 依存を回避する。
//!
//! # 使用例
//!
//! ```ignore
//! use pricer_models::market::volcube::graph::VolCubeGraphData;
//!
//! let cube: VolCube<f64> = /* ... */;
//! let graph_data = VolCubeGraphData::from_cube(&cube, "CUBE-001");
//! ```

use std::collections::HashMap;

use num_traits::Float;
use serde::{Deserialize, Serialize};

use super::{cube::VolCube, types::InstrumentId};

/// 計算グラフノード種別。
///
/// VolCube関連のノードタイプを定義。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolCubeNodeType {
    /// VolCube本体ノード。
    Cube,
    /// SABRパラメータノード（expiry-tenor単位）。
    SabrSlice,
    /// ソースInstrumentノード。
    Instrument,
    /// 補間パラメータノード。
    Interpolation,
}

/// 計算グラフエッジ種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolCubeEdgeType {
    /// ソースからキューブへの依存。
    Source,
    /// カリブレーション依存。
    Calibration,
    /// 補間依存。
    Interpolation,
}

/// グラフノード情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolCubeGraphNode {
    /// ノードID。
    pub id: String,
    /// ノード表示名。
    pub label: String,
    /// ノード種別。
    pub node_type: VolCubeNodeType,
    /// 関連する数値（vol値など）。
    pub value: Option<f64>,
    /// 追加メタデータ。
    pub metadata: HashMap<String, String>,
}

impl VolCubeGraphNode {
    /// 新しいノードを作成。
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        node_type: VolCubeNodeType,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            node_type,
            value: None,
            metadata: HashMap::new(),
        }
    }

    /// 値を設定。
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// メタデータを追加。
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// グラフエッジ情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolCubeGraphEdge {
    /// エッジID。
    pub id: String,
    /// ソースノードID。
    pub source: String,
    /// ターゲットノードID。
    pub target: String,
    /// エッジ種別。
    pub edge_type: VolCubeEdgeType,
    /// エッジラベル（オプション）。
    pub label: Option<String>,
}

impl VolCubeGraphEdge {
    /// 新しいエッジを作成。
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        edge_type: VolCubeEdgeType,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            edge_type,
            label: None,
        }
    }

    /// ラベルを設定。
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// VolCubeグラフデータ。
///
/// D3.js互換のノード・エッジ構造を提供。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolCubeGraphData {
    /// グラフID。
    pub id: String,
    /// グラフ名。
    pub name: String,
    /// ノードリスト。
    pub nodes: Vec<VolCubeGraphNode>,
    /// エッジリスト。
    pub edges: Vec<VolCubeGraphEdge>,
    /// メタデータ。
    pub metadata: HashMap<String, String>,
}

impl VolCubeGraphData {
    /// VolCubeからグラフデータを生成。
    ///
    /// # Arguments
    ///
    /// * `cube` - VolCubeインスタンス
    /// * `cube_id` - グラフのルートノードID
    pub fn from_cube<T: Float + Send + Sync>(cube: &VolCube<T>, cube_id: &str) -> Self {
        use super::cube::VolatilityCube;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut edge_counter = 0;

        // ルートノード（VolCube本体）
        let root_node = VolCubeGraphNode::new(
            cube_id,
            format!("VolCube {}", cube_id),
            VolCubeNodeType::Cube,
        )
        .with_metadata(
            "expiry_min",
            format!("{:.2}", cube.expiry_domain().0.to_f64().unwrap_or(0.0)),
        )
        .with_metadata(
            "expiry_max",
            format!("{:.2}", cube.expiry_domain().1.to_f64().unwrap_or(0.0)),
        )
        .with_metadata(
            "tenor_min",
            format!("{:.2}", cube.tenor_domain().0.to_f64().unwrap_or(0.0)),
        )
        .with_metadata(
            "tenor_max",
            format!("{:.2}", cube.tenor_domain().1.to_f64().unwrap_or(0.0)),
        );

        nodes.push(root_node);

        // ソースInstrumentノード
        for inst_id in cube.source_instruments() {
            let node_id = format!("{}:{}", cube_id, inst_id.as_str());
            let node =
                VolCubeGraphNode::new(&node_id, inst_id.as_str(), VolCubeNodeType::Instrument);
            nodes.push(node);

            // ソースからキューブへのエッジ
            let edge = VolCubeGraphEdge::new(
                format!("e{}", edge_counter),
                &node_id,
                cube_id,
                VolCubeEdgeType::Source,
            );
            edges.push(edge);
            edge_counter += 1;
        }

        // SABRパラメータスライスノード（expiry-tenor格子点）
        let expiries = cube.sabr_params().expiries();
        let tenors = cube.sabr_params().tenors();

        for (ei, exp) in expiries.iter().enumerate() {
            for (ti, ten) in tenors.iter().enumerate() {
                let slice_id = format!("{}:SABR:{}x{}", cube_id, ei, ti);
                let slice_label = format!(
                    "SABR({:.1}Y,{:.0}Y)",
                    exp.to_f64().unwrap_or(0.0),
                    ten.to_f64().unwrap_or(0.0)
                );

                let node =
                    VolCubeGraphNode::new(&slice_id, slice_label, VolCubeNodeType::SabrSlice)
                        .with_metadata("expiry", exp.to_f64().unwrap_or(0.0).to_string())
                        .with_metadata("tenor", ten.to_f64().unwrap_or(0.0).to_string());

                nodes.push(node);

                // スライスからキューブへのエッジ
                let edge = VolCubeGraphEdge::new(
                    format!("e{}", edge_counter),
                    &slice_id,
                    cube_id,
                    VolCubeEdgeType::Calibration,
                );
                edges.push(edge);
                edge_counter += 1;
            }
        }

        let mut metadata = HashMap::new();
        metadata.insert("node_count".to_string(), nodes.len().to_string());
        metadata.insert("edge_count".to_string(), edges.len().to_string());
        metadata.insert(
            "source_instruments".to_string(),
            cube.source_instruments().len().to_string(),
        );

        Self {
            id: cube_id.to_string(),
            name: format!("VolCube {}", cube_id),
            nodes,
            edges,
            metadata,
        }
    }

    /// D3.js互換のJSON形式で出力。
    pub fn to_d3_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "label": n.label,
                    "type": format!("{:?}", n.node_type),
                    "value": n.value,
                    "metadata": n.metadata,
                })
            }).collect::<Vec<_>>(),
            "links": self.edges.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "source": e.source,
                    "target": e.target,
                    "type": format!("{:?}", e.edge_type),
                    "label": e.label,
                })
            }).collect::<Vec<_>>(),
        })
    }

    /// ノード数を取得。
    pub fn node_count(&self) -> usize { self.nodes.len() }

    /// エッジ数を取得。
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

/// AAD感度情報。
///
/// Vega, Volga, Vannaなどのグリーク計算に必要な情報を保持。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolCubeSensitivityInfo {
    /// ソースInstrument ID。
    pub instrument_id: InstrumentId,
    /// 感度の種類（"vega", "volga", "vanna"など）。
    pub sensitivity_type: String,
    /// 感度値（AD計算結果）。
    pub value: f64,
    /// Expiry。
    pub expiry: f64,
    /// Tenor。
    pub tenor: f64,
    /// Strike。
    pub strike: f64,
}

impl VolCubeSensitivityInfo {
    /// 新しい感度情報を作成。
    pub fn new(
        instrument_id: InstrumentId,
        sensitivity_type: impl Into<String>,
        value: f64,
        expiry: f64,
        tenor: f64,
        strike: f64,
    ) -> Self {
        Self {
            instrument_id,
            sensitivity_type: sensitivity_type.into(),
            value,
            expiry,
            tenor,
            strike,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::{SabrParameterSurface, VolCubeConfig};

    fn create_test_cube() -> VolCube<f64> {
        use crate::market::volcube::SabrParams;

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
    fn test_graph_node_creation() {
        let node = VolCubeGraphNode::new("node1", "Test Node", VolCubeNodeType::Cube);
        assert_eq!(node.id, "node1");
        assert_eq!(node.label, "Test Node");
        assert_eq!(node.node_type, VolCubeNodeType::Cube);
        assert!(node.value.is_none());
    }

    #[test]
    fn test_graph_node_with_value() {
        let node =
            VolCubeGraphNode::new("node1", "Test", VolCubeNodeType::Instrument).with_value(0.25);
        assert_eq!(node.value, Some(0.25));
    }

    #[test]
    fn test_graph_node_with_metadata() {
        let node = VolCubeGraphNode::new("node1", "Test", VolCubeNodeType::SabrSlice)
            .with_metadata("expiry", "1.0")
            .with_metadata("tenor", "5.0");

        assert_eq!(node.metadata.get("expiry"), Some(&"1.0".to_string()));
        assert_eq!(node.metadata.get("tenor"), Some(&"5.0".to_string()));
    }

    #[test]
    fn test_graph_edge_creation() {
        let edge = VolCubeGraphEdge::new("e1", "source", "target", VolCubeEdgeType::Source);
        assert_eq!(edge.id, "e1");
        assert_eq!(edge.source, "source");
        assert_eq!(edge.target, "target");
        assert_eq!(edge.edge_type, VolCubeEdgeType::Source);
        assert!(edge.label.is_none());
    }

    #[test]
    fn test_graph_edge_with_label() {
        let edge = VolCubeGraphEdge::new("e1", "a", "b", VolCubeEdgeType::Calibration)
            .with_label("calibrates");
        assert_eq!(edge.label, Some("calibrates".to_string()));
    }

    #[test]
    fn test_volcube_graph_data_from_cube() {
        let cube = create_test_cube();
        let graph_data = VolCubeGraphData::from_cube(&cube, "CUBE-001");

        assert_eq!(graph_data.id, "CUBE-001");

        // 1 root + 3 instruments + 4 SABR slices = 8 nodes
        assert_eq!(graph_data.node_count(), 8);

        // 3 instrument->cube edges + 4 sabr->cube edges = 7 edges
        assert_eq!(graph_data.edge_count(), 7);
    }

    #[test]
    fn test_volcube_graph_data_to_d3_json() {
        let cube = create_test_cube();
        let graph_data = VolCubeGraphData::from_cube(&cube, "CUBE-001");
        let json = graph_data.to_d3_json();

        assert!(json.get("nodes").is_some());
        assert!(json.get("links").is_some());

        let nodes = json.get("nodes").unwrap().as_array().unwrap();
        assert_eq!(nodes.len(), 8);

        let links = json.get("links").unwrap().as_array().unwrap();
        assert_eq!(links.len(), 7);
    }

    #[test]
    fn test_sensitivity_info_creation() {
        let info =
            VolCubeSensitivityInfo::new(InstrumentId::new("INST-1"), "vega", 0.05, 1.0, 5.0, 0.03);

        assert_eq!(info.instrument_id.as_str(), "INST-1");
        assert_eq!(info.sensitivity_type, "vega");
        assert_eq!(info.value, 0.05);
        assert_eq!(info.expiry, 1.0);
        assert_eq!(info.tenor, 5.0);
        assert_eq!(info.strike, 0.03);
    }

    #[test]
    fn test_node_types() {
        assert_ne!(VolCubeNodeType::Cube, VolCubeNodeType::Instrument);
        assert_ne!(VolCubeNodeType::SabrSlice, VolCubeNodeType::Interpolation);
    }

    #[test]
    fn test_edge_types() {
        assert_ne!(VolCubeEdgeType::Source, VolCubeEdgeType::Calibration);
        assert_ne!(VolCubeEdgeType::Calibration, VolCubeEdgeType::Interpolation);
    }
}

//! 可視化モジュール: ベンチマーク結果のチャート表示機能
//!
//! # Task Coverage
//!
//! - Task 7.1: 計算フロー概念図の実装
//!   - AADとBump-and-Revalueの計算フローの概念図をTUI/Webで表示
//!   - 両手法の違いを視覚的に説明する図を実装
//!
//! - Task 7.2: 速度比較チャートの実装
//!   - ベンチマーク結果を速度比較バーチャートとして表示
//!   - TUIモードではratatuiのChartウィジェットを使用
//!   - Webモードではchart.js互換JSONデータを出力
//!
//! # Requirements Coverage
//!
//! - Requirement 7.1: 計算フローの概念図を表示
//! - Requirement 7.2: 速度比較のバーチャートを表示
//! - Requirement 7.4: TUIモードではratatuiのチャートウィジェットを使用
//! - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力

use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Paragraph},
};

use serde::Serialize;

// =============================================================================
// 速度比較データ構造
// =============================================================================

/// 速度比較のためのベンチマークデータ
///
/// # Requirements Coverage
///
/// - Requirement 7.2: 速度比較のバーチャートを表示
#[derive(Clone, Debug)]
pub struct SpeedComparisonData {
    /// AADの平均計算時間（ナノ秒）
    pub aad_mean_ns: f64,
    /// Bump-and-Revalueの平均計算時間（ナノ秒）
    pub bump_mean_ns: f64,
    /// 高速化率（bump / aad）
    pub speedup_ratio: f64,
    /// テナーポイント数
    pub tenor_count: usize,
}

impl SpeedComparisonData {
    /// 新しいSpeedComparisonDataを生成
    pub fn new(aad_mean_ns: f64, bump_mean_ns: f64, tenor_count: usize) -> Self {
        let speedup_ratio = if aad_mean_ns > 0.0 {
            bump_mean_ns / aad_mean_ns
        } else {
            1.0
        };

        Self {
            aad_mean_ns,
            bump_mean_ns,
            speedup_ratio,
            tenor_count,
        }
    }

    /// ナノ秒からマイクロ秒に変換したAAD時間を取得
    pub fn aad_mean_us(&self) -> f64 {
        self.aad_mean_ns / 1000.0
    }

    /// ナノ秒からマイクロ秒に変換したBump時間を取得
    pub fn bump_mean_us(&self) -> f64 {
        self.bump_mean_ns / 1000.0
    }

    /// サンプルデータを生成（デモ用）
    pub fn sample() -> Self {
        Self::new(15_000.0, 300_000.0, 20)
    }
}

// =============================================================================
// Chart.js互換JSON出力
// =============================================================================

/// Chart.jsのデータセット構造
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartJsDataset {
    /// データセットのラベル
    pub label: String,
    /// データ値の配列
    pub data: Vec<f64>,
    /// 背景色の配列
    pub background_color: Vec<String>,
}

/// Chart.jsのデータ構造
#[derive(Clone, Debug, Serialize)]
pub struct ChartJsData {
    /// X軸のラベル
    pub labels: Vec<String>,
    /// データセットの配列
    pub datasets: Vec<ChartJsDataset>,
}

/// Chart.jsのタイトルオプション
#[derive(Clone, Debug, Serialize)]
pub struct ChartJsTitleOptions {
    /// タイトル表示フラグ
    pub display: bool,
    /// タイトルテキスト
    pub text: String,
}

/// Chart.jsのオプション
#[derive(Clone, Debug, Serialize)]
pub struct ChartJsOptions {
    /// タイトル設定
    pub title: ChartJsTitleOptions,
}

/// Chart.js互換のバーチャート構造
///
/// # Requirements Coverage
///
/// - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力
#[derive(Clone, Debug, Serialize)]
pub struct ChartJsBarChart {
    /// チャートタイプ
    #[serde(rename = "type")]
    pub chart_type: String,
    /// チャートデータ
    pub data: ChartJsData,
    /// チャートオプション
    pub options: ChartJsOptions,
}

// =============================================================================
// BenchmarkVisualiser
// =============================================================================

/// ベンチマーク結果の可視化を担当する構造体
///
/// # Task Coverage
///
/// - Task 7.2: 速度比較チャートの実装
///
/// # Requirements Coverage
///
/// - Requirement 7.2: 速度比較のバーチャートを表示
/// - Requirement 7.4: TUIモードではratatuiのチャートウィジェットを使用
/// - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力
pub struct BenchmarkVisualiser;

impl BenchmarkVisualiser {
    /// 新しいBenchmarkVisualiserを生成
    pub fn new() -> Self {
        Self
    }

    /// TUI用の速度比較バーチャートを描画
    ///
    /// # Arguments
    ///
    /// * `frame` - ratatuiのフレーム
    /// * `area` - 描画領域
    /// * `data` - 速度比較データ
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 7.2: 速度比較のバーチャートを表示
    /// - Requirement 7.4: TUIモードではratatuiのチャートウィジェットを使用
    pub fn draw_speed_comparison_bar(
        &self,
        frame: &mut Frame,
        area: Rect,
        data: &SpeedComparisonData,
    ) {
        // レイアウト: チャート領域 + サマリー領域
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(area);

        // バーチャート用データ（マイクロ秒単位で表示）
        let aad_us = data.aad_mean_us();
        let bump_us = data.bump_mean_us();

        // ratatuiのBarChartでは、データは u64 にする必要がある
        // マイクロ秒で表示するため適切にスケーリング
        let bar_data: Vec<(&str, u64)> = vec![
            ("AAD", aad_us.round() as u64),
            ("Bump", bump_us.round() as u64),
        ];

        let bar_chart = BarChart::default()
            .block(
                Block::default()
                    .title(format!(
                        " 速度比較 ({}xスピードアップ) ",
                        format_ratio(data.speedup_ratio)
                    ))
                    .borders(Borders::ALL),
            )
            .bar_width(15)
            .bar_gap(3)
            .group_gap(2)
            .bar_style(Style::default().fg(Color::Green))
            .value_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .label_style(Style::default().fg(Color::Yellow))
            .data(&bar_data);

        frame.render_widget(bar_chart, chunks[0]);

        // サマリー情報
        let summary_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  AAD: ", Style::default().fg(Color::Green)),
                Span::raw(format_time_us(aad_us)),
                Span::raw("  |  "),
                Span::styled("Bump-and-Revalue: ", Style::default().fg(Color::Red)),
                Span::raw(format_time_us(bump_us)),
                Span::raw("  |  "),
                Span::styled("テナー数: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", data.tenor_count)),
            ]),
        ];

        let summary = Paragraph::new(summary_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" サマリー "));

        frame.render_widget(summary, chunks[1]);
    }

    /// Chart.js互換のJSONデータを生成
    ///
    /// # Arguments
    ///
    /// * `data` - 速度比較データ
    ///
    /// # Returns
    ///
    /// Chart.js互換のJSONデータを含む構造体
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力
    pub fn to_chartjs_json(&self, data: &SpeedComparisonData) -> ChartJsBarChart {
        let aad_us = data.aad_mean_us();
        let bump_us = data.bump_mean_us();

        ChartJsBarChart {
            chart_type: "bar".to_string(),
            data: ChartJsData {
                labels: vec!["AAD".to_string(), "Bump-and-Revalue".to_string()],
                datasets: vec![ChartJsDataset {
                    label: "Computation Time (μs)".to_string(),
                    data: vec![aad_us, bump_us],
                    background_color: vec!["#4CAF50".to_string(), "#FF5722".to_string()],
                }],
            },
            options: ChartJsOptions {
                title: ChartJsTitleOptions {
                    display: true,
                    text: format!(
                        "Speed Comparison ({}x speedup)",
                        format_ratio(data.speedup_ratio)
                    ),
                },
            },
        }
    }

    /// Chart.js互換のJSON文字列を生成
    ///
    /// # Arguments
    ///
    /// * `data` - 速度比較データ
    ///
    /// # Returns
    ///
    /// Chart.js互換のJSON文字列
    pub fn to_chartjs_json_string(&self, data: &SpeedComparisonData) -> String {
        let chart = self.to_chartjs_json(data);
        serde_json::to_string_pretty(&chart).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for BenchmarkVisualiser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Task 7.1: 計算フロー概念図
// =============================================================================

/// 計算フローの概念図を描画するための構造体
///
/// AAD（Adjoint Algorithmic Differentiation）とBump-and-Revalueの
/// 計算フローの違いを視覚的に説明する。
///
/// # Task Coverage
///
/// - Task 7.1: 計算フロー概念図の実装
///
/// # Requirements Coverage
///
/// - Requirement 7.1: 計算フローの概念図を表示
pub struct ComputationFlowDiagram;

impl ComputationFlowDiagram {
    /// 新しいComputationFlowDiagramを生成
    pub fn new() -> Self {
        Self
    }

    /// AADとBump-and-Revalueの比較図をTUI上に描画
    ///
    /// # Arguments
    ///
    /// * `frame` - ratatuiのフレーム
    /// * `area` - 描画領域
    pub fn draw_comparison(frame: &mut Frame, area: Rect) {
        // 左右に分割してAADとBumpを並べて表示
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        Self::draw_aad_flow(frame, chunks[0]);
        Self::draw_bump_flow(frame, chunks[1]);
    }

    /// AADの計算フロー図を描画
    fn draw_aad_flow(frame: &mut Frame, area: Rect) {
        let flow_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  AAD (Adjoint Algorithmic Differentiation)",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  ┌─────────────────────────────────┐"),
            Line::from("  │    Forward Pass (順伝播)        │"),
            Line::from("  │    f(x₁, x₂, ..., xₙ) → y       │"),
            Line::from("  │    + テープ記録                 │"),
            Line::from("  └────────────────┬────────────────┘"),
            Line::from("                   │"),
            Line::from("                   ▼"),
            Line::from("  ┌─────────────────────────────────┐"),
            Line::from("  │   Backward Pass (逆伝播)        │"),
            Line::from("  │   ∂y/∂x₁, ∂y/∂x₂, ..., ∂y/∂xₙ  │"),
            Line::from("  │   一度で全微分を計算            │"),
            Line::from("  └─────────────────────────────────┘"),
            Line::from(""),
            Line::from(Span::styled(
                "  計算量: O(1) × Forward Pass Cost",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                "  メモリ: O(m) テープサイズ",
                Style::default().fg(Color::Yellow),
            )),
        ];

        let paragraph = Paragraph::new(flow_text).block(
            Block::default()
                .title(" AAD Flow ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        frame.render_widget(paragraph, area);
    }

    /// Bump-and-Revalueの計算フロー図を描画
    fn draw_bump_flow(frame: &mut Frame, area: Rect) {
        let flow_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Bump-and-Revalue (有限差分法)",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  ┌─────────────────────────────────┐"),
            Line::from("  │  For i = 1 to n:                │"),
            Line::from("  │    ┌───────────────────────┐    │"),
            Line::from("  │    │ xᵢ → xᵢ + ε (bump)   │    │"),
            Line::from("  │    │ f(x) → y'             │    │"),
            Line::from("  │    │ ∂y/∂xᵢ ≈ (y'-y)/ε    │    │"),
            Line::from("  │    └───────────────────────┘    │"),
            Line::from("  │  End for                        │"),
            Line::from("  └─────────────────────────────────┘"),
            Line::from(""),
            Line::from(Span::styled(
                "  計算量: O(n) × Forward Pass Cost",
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "  メモリ: O(1) 追加メモリなし",
                Style::default().fg(Color::Green),
            )),
        ];

        let paragraph = Paragraph::new(flow_text).block(
            Block::default()
                .title(" Bump-and-Revalue Flow ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        );

        frame.render_widget(paragraph, area);
    }

    /// スケーリング特性の説明を描画
    pub fn draw_scaling_explanation(frame: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  計算量のスケーリング比較",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  n = パラメータ数（テナーポイント数）"),
            Line::from(""),
            Line::from(vec![
                Span::styled("  AAD:  ", Style::default().fg(Color::Cyan)),
                Span::raw("O(1) - パラメータ数に依存しない"),
            ]),
            Line::from(vec![
                Span::styled("  Bump: ", Style::default().fg(Color::Red)),
                Span::raw("O(n) - パラメータ数に比例"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  スピードアップ: ", Style::default().fg(Color::Green)),
                Span::raw("≈ n倍 (テナー数が増えるほど効果大)"),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .title(" Scaling Characteristics ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

        frame.render_widget(paragraph, area);
    }

    /// Web用のJSON形式で計算フロー情報を出力
    pub fn to_flow_json() -> serde_json::Value {
        serde_json::json!({
            "aad": {
                "name": "AAD (Adjoint Algorithmic Differentiation)",
                "steps": [
                    {"name": "Forward Pass", "description": "Compute f(x) and record tape"},
                    {"name": "Backward Pass", "description": "Compute all derivatives in one sweep"}
                ],
                "complexity": {
                    "time": "O(1) × Forward Pass",
                    "space": "O(m) tape size"
                }
            },
            "bump": {
                "name": "Bump-and-Revalue (Finite Difference)",
                "steps": [
                    {"name": "Loop", "description": "For each parameter i"},
                    {"name": "Bump", "description": "x_i -> x_i + epsilon"},
                    {"name": "Revalue", "description": "Compute f(x')"},
                    {"name": "Difference", "description": "dy/dx_i ≈ (y' - y) / epsilon"}
                ],
                "complexity": {
                    "time": "O(n) × Forward Pass",
                    "space": "O(1)"
                }
            },
            "comparison": {
                "speedup": "~n times faster with AAD",
                "note": "n = number of parameters (tenor points)"
            }
        })
    }
}

impl Default for ComputationFlowDiagram {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Task 7.3: スケーラビリティグラフ
// =============================================================================

/// スケーラビリティデータポイント
///
/// テナー数に対する計算時間のデータを保持
#[derive(Clone, Debug)]
pub struct ScalabilityDataPoint {
    /// テナー数
    pub tenor_count: usize,
    /// AAD計算時間（ナノ秒）
    pub aad_time_ns: f64,
    /// Bump計算時間（ナノ秒）
    pub bump_time_ns: f64,
}

/// スケーラビリティデータセット
///
/// 複数のデータポイントを保持し、折れ線グラフ用のデータを提供
#[derive(Clone, Debug, Default)]
pub struct ScalabilityData {
    /// データポイント配列
    pub points: Vec<ScalabilityDataPoint>,
}

impl ScalabilityData {
    /// 新しいScalabilityDataを生成
    pub fn new() -> Self {
        Self::default()
    }

    /// データポイントを追加
    pub fn add_point(&mut self, tenor_count: usize, aad_time_ns: f64, bump_time_ns: f64) {
        self.points.push(ScalabilityDataPoint {
            tenor_count,
            aad_time_ns,
            bump_time_ns,
        });
    }

    /// AADデータをチャート用の座標に変換
    pub fn aad_chart_data(&self) -> Vec<(f64, f64)> {
        self.points
            .iter()
            .map(|p| (p.tenor_count as f64, p.aad_time_ns / 1000.0)) // ns -> us
            .collect()
    }

    /// Bumpデータをチャート用の座標に変換
    pub fn bump_chart_data(&self) -> Vec<(f64, f64)> {
        self.points
            .iter()
            .map(|p| (p.tenor_count as f64, p.bump_time_ns / 1000.0)) // ns -> us
            .collect()
    }

    /// 最大テナー数を取得
    pub fn max_tenor_count(&self) -> f64 {
        self.points.iter().map(|p| p.tenor_count).max().unwrap_or(1) as f64
    }

    /// 最大計算時間（マイクロ秒）を取得
    pub fn max_time_us(&self) -> f64 {
        self.points
            .iter()
            .map(|p| p.aad_time_ns.max(p.bump_time_ns) / 1000.0)
            .fold(0.0_f64, |a, b| a.max(b))
    }

    /// サンプルデータを生成（デモ用）
    ///
    /// AADはO(1)、BumpはO(n)のスケーリング特性を模擬
    pub fn sample() -> Self {
        let mut data = Self::new();
        let base_aad = 15_000.0; // 基準AAD時間（ns）
        let base_bump = 15_000.0; // 基準Bump時間（ns）

        for tenor in [2, 4, 8, 12, 16, 20] {
            // AAD: ほぼ一定（わずかな増加）
            let aad_time = base_aad + (tenor as f64 * 100.0);
            // Bump: テナー数に比例
            let bump_time = base_bump * tenor as f64;
            data.add_point(tenor, aad_time, bump_time);
        }
        data
    }
}

/// スケーラビリティグラフを描画する構造体
///
/// # Task Coverage
///
/// - Task 7.3: スケーラビリティグラフの実装
///
/// # Requirements Coverage
///
/// - Requirement 7.3: テナー数増加に伴う計算時間の比較折れ線グラフ
pub struct ScalabilityVisualiser;

impl ScalabilityVisualiser {
    /// 新しいScalabilityVisualiserを生成
    pub fn new() -> Self {
        Self
    }

    /// TUI用のスケーラビリティ折れ線グラフを描画
    ///
    /// # Arguments
    ///
    /// * `frame` - ratatuiのフレーム
    /// * `area` - 描画領域
    /// * `data` - スケーラビリティデータ
    pub fn draw_scalability_chart(frame: &mut Frame, area: Rect, data: &ScalabilityData) {
        use ratatui::symbols;
        use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

        if data.points.is_empty() {
            let empty = Paragraph::new("No scalability data available")
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Scalability ")
                        .borders(Borders::ALL),
                );
            frame.render_widget(empty, area);
            return;
        }

        let aad_data = data.aad_chart_data();
        let bump_data = data.bump_chart_data();

        let max_x = data.max_tenor_count() * 1.1;
        let max_y = data.max_time_us() * 1.2;

        let datasets = vec![
            Dataset::default()
                .name("AAD")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&aad_data),
            Dataset::default()
                .name("Bump")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Red))
                .data(&bump_data),
        ];

        let x_labels = vec![
            Span::raw("0"),
            Span::raw(format!("{:.0}", max_x / 2.0)),
            Span::raw(format!("{:.0}", max_x)),
        ];

        let y_labels = vec![
            Span::raw("0"),
            Span::raw(format!("{:.0}", max_y / 2.0)),
            Span::raw(format!("{:.0} us", max_y)),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(" Scalability: AAD vs Bump-and-Revalue ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .x_axis(
                Axis::default()
                    .title("Tenor Count")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, max_x])
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Time (us)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, max_y])
                    .labels(y_labels),
            );

        frame.render_widget(chart, area);
    }

    /// Chart.js互換のJSON形式でスケーラビリティデータを出力
    pub fn to_chartjs_json(data: &ScalabilityData) -> serde_json::Value {
        let tenor_labels: Vec<String> = data
            .points
            .iter()
            .map(|p| p.tenor_count.to_string())
            .collect();

        let aad_times: Vec<f64> = data.points.iter().map(|p| p.aad_time_ns / 1000.0).collect();

        let bump_times: Vec<f64> = data
            .points
            .iter()
            .map(|p| p.bump_time_ns / 1000.0)
            .collect();

        serde_json::json!({
            "type": "line",
            "data": {
                "labels": tenor_labels,
                "datasets": [
                    {
                        "label": "AAD",
                        "data": aad_times,
                        "borderColor": "#4CAF50",
                        "fill": false
                    },
                    {
                        "label": "Bump-and-Revalue",
                        "data": bump_times,
                        "borderColor": "#FF5722",
                        "fill": false
                    }
                ]
            },
            "options": {
                "title": {
                    "display": true,
                    "text": "Scalability: AAD vs Bump-and-Revalue"
                },
                "scales": {
                    "xAxes": [{ "scaleLabel": { "display": true, "labelString": "Tenor Count" }}],
                    "yAxes": [{ "scaleLabel": { "display": true, "labelString": "Time (μs)" }}]
                }
            }
        })
    }
}

impl Default for ScalabilityVisualiser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Task 7.4: 精度検証結果表示
// =============================================================================

/// 精度検証結果データ
///
/// AADとBump-and-Revalueの計算結果の差を保持
#[derive(Clone, Debug)]
pub struct AccuracyVerificationData {
    /// テナーポイント
    pub tenors: Vec<f64>,
    /// AADデルタ値
    pub aad_deltas: Vec<f64>,
    /// Bumpデルタ値
    pub bump_deltas: Vec<f64>,
    /// 相対誤差
    pub relative_errors: Vec<f64>,
}

impl AccuracyVerificationData {
    /// 新しいAccuracyVerificationDataを生成
    pub fn new(tenors: Vec<f64>, aad_deltas: Vec<f64>, bump_deltas: Vec<f64>) -> Self {
        let relative_errors: Vec<f64> = aad_deltas
            .iter()
            .zip(bump_deltas.iter())
            .map(|(aad, bump)| {
                if bump.abs() > 1e-12 {
                    ((aad - bump) / bump).abs()
                } else {
                    0.0
                }
            })
            .collect();

        Self {
            tenors,
            aad_deltas,
            bump_deltas,
            relative_errors,
        }
    }

    /// 最大相対誤差を取得
    pub fn max_relative_error(&self) -> f64 {
        self.relative_errors.iter().fold(0.0_f64, |a, &b| a.max(b))
    }

    /// 平均相対誤差を取得
    pub fn mean_relative_error(&self) -> f64 {
        if self.relative_errors.is_empty() {
            0.0
        } else {
            self.relative_errors.iter().sum::<f64>() / self.relative_errors.len() as f64
        }
    }

    /// サンプルデータを生成
    pub fn sample() -> Self {
        let tenors = vec![1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        let bump_deltas = vec![100.5, 195.2, 280.8, 420.3, 550.1, 680.9];
        let aad_deltas = vec![
            100.5000001,
            195.2000002,
            280.8000001,
            420.3000003,
            550.1000002,
            680.9000001,
        ];
        Self::new(tenors, aad_deltas, bump_deltas)
    }
}

/// 精度検証結果を表示する構造体
///
/// # Task Coverage
///
/// - Task 7.4: 精度検証結果表示の実装
///
/// # Requirements Coverage
///
/// - Requirement 7.6: 数値精度検証結果のテーブル表示
pub struct AccuracyVisualiser;

impl AccuracyVisualiser {
    /// 新しいAccuracyVisualiserを生成
    pub fn new() -> Self {
        Self
    }

    /// TUI用の精度検証結果テーブルを描画
    pub fn draw_accuracy_table(frame: &mut Frame, area: Rect, data: &AccuracyVerificationData) {
        use ratatui::widgets::{Cell, Row, Table};

        let header_cells = ["Tenor", "AAD Delta", "Bump Delta", "Rel. Error"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = data
            .tenors
            .iter()
            .zip(data.aad_deltas.iter())
            .zip(data.bump_deltas.iter())
            .zip(data.relative_errors.iter())
            .map(|(((tenor, aad), bump), error)| {
                let error_color = if *error < 1e-10 {
                    Color::Green
                } else if *error < 1e-6 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                Row::new(vec![
                    Cell::from(format!("{:.1}Y", tenor)),
                    Cell::from(format!("{:.6}", aad)),
                    Cell::from(format!("{:.6}", bump)),
                    Cell::from(format!("{:.2e}", error)).style(Style::default().fg(error_color)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(15),
        ];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .title(format!(
                    " Accuracy Verification (Max Error: {:.2e}) ",
                    data.max_relative_error()
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        frame.render_widget(table, area);
    }

    /// 精度検証サマリーを描画
    pub fn draw_accuracy_summary(frame: &mut Frame, area: Rect, data: &AccuracyVerificationData) {
        let max_error = data.max_relative_error();
        let mean_error = data.mean_relative_error();

        let status_color = if max_error < 1e-10 {
            Color::Green
        } else if max_error < 1e-6 {
            Color::Yellow
        } else {
            Color::Red
        };

        let status_text = if max_error < 1e-10 {
            "EXCELLENT"
        } else if max_error < 1e-6 {
            "GOOD"
        } else {
            "NEEDS REVIEW"
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    status_text,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Max Relative Error:  ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.2e}", max_error),
                    Style::default().fg(status_color),
                ),
            ]),
            Line::from(vec![
                Span::styled("Mean Relative Error: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.2e}", mean_error),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("Tenor Points:        ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", data.tenors.len())),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .title(" Accuracy Summary ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        frame.render_widget(paragraph, area);
    }

    /// JSON形式で精度検証結果を出力
    pub fn to_json(data: &AccuracyVerificationData) -> serde_json::Value {
        let points: Vec<serde_json::Value> = data
            .tenors
            .iter()
            .zip(data.aad_deltas.iter())
            .zip(data.bump_deltas.iter())
            .zip(data.relative_errors.iter())
            .map(|(((tenor, aad), bump), error)| {
                serde_json::json!({
                    "tenor": tenor,
                    "aad_delta": aad,
                    "bump_delta": bump,
                    "relative_error": error
                })
            })
            .collect();

        serde_json::json!({
            "summary": {
                "max_relative_error": data.max_relative_error(),
                "mean_relative_error": data.mean_relative_error(),
                "tenor_count": data.tenors.len()
            },
            "points": points
        })
    }
}

impl Default for AccuracyVisualiser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ヘルパー関数
// =============================================================================

/// 時間をマイクロ秒単位でフォーマット
fn format_time_us(time_us: f64) -> String {
    if time_us >= 1000.0 {
        format!("{:.2} ms", time_us / 1000.0)
    } else {
        format!("{:.2} μs", time_us)
    }
}

/// スピードアップ率をフォーマット
fn format_ratio(ratio: f64) -> String {
    if ratio >= 100.0 {
        format!("{:.0}", ratio)
    } else if ratio >= 10.0 {
        format!("{:.1}", ratio)
    } else {
        format!("{:.2}", ratio)
    }
}

// =============================================================================
// チャートデータ変換ヘルパー
// =============================================================================

/// TUI表示用のバーチャートデータを生成
///
/// ratatuiのChartウィジェット用のデータポイント形式に変換
pub fn to_chart_data_points(data: &SpeedComparisonData) -> Vec<(f64, f64)> {
    vec![(0.0, data.aad_mean_us()), (1.0, data.bump_mean_us())]
}

/// チャート用のY軸境界を計算
pub fn compute_y_bounds(data: &SpeedComparisonData) -> [f64; 2] {
    let max_val = data.bump_mean_us().max(data.aad_mean_us());
    [0.0, max_val * 1.2] // 20%のマージンを追加
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SpeedComparisonData Tests
    // =========================================================================

    mod speed_comparison_data_tests {
        use super::*;

        #[test]
        fn test_new() {
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            assert!((data.aad_mean_ns - 15_000.0).abs() < 1e-10);
            assert!((data.bump_mean_ns - 300_000.0).abs() < 1e-10);
            assert!((data.speedup_ratio - 20.0).abs() < 1e-10);
            assert_eq!(data.tenor_count, 20);
        }

        #[test]
        fn test_new_zero_aad() {
            let data = SpeedComparisonData::new(0.0, 300_000.0, 20);

            assert!((data.speedup_ratio - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_aad_mean_us() {
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            assert!((data.aad_mean_us() - 15.0).abs() < 1e-10);
        }

        #[test]
        fn test_bump_mean_us() {
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            assert!((data.bump_mean_us() - 300.0).abs() < 1e-10);
        }

        #[test]
        fn test_sample() {
            let data = SpeedComparisonData::sample();

            assert!(data.aad_mean_ns > 0.0);
            assert!(data.bump_mean_ns > 0.0);
            assert!(data.speedup_ratio > 1.0);
            assert!(data.tenor_count > 0);
        }
    }

    // =========================================================================
    // BenchmarkVisualiser Tests
    // =========================================================================

    mod benchmark_visualiser_tests {
        use super::*;

        #[test]
        fn test_new() {
            let visualiser = BenchmarkVisualiser::new();
            // ステートレスな構造体なので、生成できることを確認
            let _ = visualiser;
        }

        #[test]
        fn test_default() {
            let visualiser = BenchmarkVisualiser::default();
            let _ = visualiser;
        }

        #[test]
        fn test_to_chartjs_json() {
            let visualiser = BenchmarkVisualiser::new();
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            let chart = visualiser.to_chartjs_json(&data);

            assert_eq!(chart.chart_type, "bar");
            assert_eq!(chart.data.labels.len(), 2);
            assert_eq!(chart.data.labels[0], "AAD");
            assert_eq!(chart.data.labels[1], "Bump-and-Revalue");
            assert_eq!(chart.data.datasets.len(), 1);
            assert_eq!(chart.data.datasets[0].data.len(), 2);
            assert!((chart.data.datasets[0].data[0] - 15.0).abs() < 1e-10);
            assert!((chart.data.datasets[0].data[1] - 300.0).abs() < 1e-10);
        }

        #[test]
        fn test_to_chartjs_json_string() {
            let visualiser = BenchmarkVisualiser::new();
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            let json = visualiser.to_chartjs_json_string(&data);

            assert!(json.contains("\"type\": \"bar\""));
            assert!(json.contains("\"labels\""));
            assert!(json.contains("\"AAD\""));
            assert!(json.contains("\"Bump-and-Revalue\""));
            assert!(json.contains("\"datasets\""));
            assert!(json.contains("Computation Time"));
            assert!(json.contains("#4CAF50")); // AAD color
            assert!(json.contains("#FF5722")); // Bump color
        }

        #[test]
        fn test_to_chartjs_json_string_contains_speedup() {
            let visualiser = BenchmarkVisualiser::new();
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);

            let json = visualiser.to_chartjs_json_string(&data);

            assert!(json.contains("Speed Comparison"));
            assert!(json.contains("speedup"));
        }

        #[test]
        fn test_to_chartjs_json_string_structure() {
            let visualiser = BenchmarkVisualiser::new();
            let data = SpeedComparisonData::sample();

            let json = visualiser.to_chartjs_json_string(&data);

            // JSONとして有効かを確認
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
            assert!(parsed.is_ok(), "Generated JSON should be valid");

            let value = parsed.unwrap();
            assert!(value.get("type").is_some());
            assert!(value.get("data").is_some());
            assert!(value.get("options").is_some());
        }
    }

    // =========================================================================
    // ヘルパー関数 Tests
    // =========================================================================

    mod helper_tests {
        use super::*;

        #[test]
        fn test_format_time_us_microseconds() {
            let result = format_time_us(15.0);
            assert!(result.contains("μs"));
            assert!(result.contains("15.00"));
        }

        #[test]
        fn test_format_time_us_milliseconds() {
            let result = format_time_us(1500.0);
            assert!(result.contains("ms"));
            assert!(result.contains("1.50"));
        }

        #[test]
        fn test_format_ratio_small() {
            let result = format_ratio(5.5);
            assert_eq!(result, "5.50");
        }

        #[test]
        fn test_format_ratio_medium() {
            let result = format_ratio(15.5);
            assert_eq!(result, "15.5");
        }

        #[test]
        fn test_format_ratio_large() {
            // Note: Rust uses banker's rounding (round half to even)
            let result = format_ratio(150.6);
            assert_eq!(result, "151");
        }

        #[test]
        fn test_to_chart_data_points() {
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);
            let points = to_chart_data_points(&data);

            assert_eq!(points.len(), 2);
            assert!((points[0].0 - 0.0).abs() < 1e-10);
            assert!((points[0].1 - 15.0).abs() < 1e-10);
            assert!((points[1].0 - 1.0).abs() < 1e-10);
            assert!((points[1].1 - 300.0).abs() < 1e-10);
        }

        #[test]
        fn test_compute_y_bounds() {
            let data = SpeedComparisonData::new(15_000.0, 300_000.0, 20);
            let bounds = compute_y_bounds(&data);

            assert!((bounds[0] - 0.0).abs() < 1e-10);
            assert!((bounds[1] - 360.0).abs() < 1e-10); // 300 * 1.2
        }
    }

    // =========================================================================
    // Chart.js JSON構造 Tests
    // =========================================================================

    mod chartjs_structure_tests {
        use super::*;

        #[test]
        fn test_chartjs_dataset_serialization() {
            let dataset = ChartJsDataset {
                label: "Test".to_string(),
                data: vec![10.0, 20.0],
                background_color: vec!["#AAA".to_string(), "#BBB".to_string()],
            };

            let json = serde_json::to_string(&dataset).unwrap();
            assert!(json.contains("\"label\""));
            assert!(json.contains("\"data\""));
            assert!(json.contains("\"backgroundColor\"")); // camelCase
        }

        #[test]
        fn test_chartjs_data_serialization() {
            let data = ChartJsData {
                labels: vec!["A".to_string(), "B".to_string()],
                datasets: vec![],
            };

            let json = serde_json::to_string(&data).unwrap();
            assert!(json.contains("\"labels\""));
            assert!(json.contains("\"datasets\""));
        }

        #[test]
        fn test_chartjs_options_serialization() {
            let options = ChartJsOptions {
                title: ChartJsTitleOptions {
                    display: true,
                    text: "Test Title".to_string(),
                },
            };

            let json = serde_json::to_string(&options).unwrap();
            assert!(json.contains("\"title\""));
            assert!(json.contains("\"display\""));
            assert!(json.contains("\"text\""));
        }

        #[test]
        fn test_chartjs_bar_chart_serialization() {
            let chart = ChartJsBarChart {
                chart_type: "bar".to_string(),
                data: ChartJsData {
                    labels: vec!["AAD".to_string()],
                    datasets: vec![],
                },
                options: ChartJsOptions {
                    title: ChartJsTitleOptions {
                        display: true,
                        text: "Test".to_string(),
                    },
                },
            };

            let json = serde_json::to_string(&chart).unwrap();
            // serde_json compact format: check for the presence of type and bar
            assert!(json.contains("\"type\""));
            assert!(json.contains("bar"));
        }
    }

    // =========================================================================
    // Task 7.3: ScalabilityData Tests
    // =========================================================================

    mod scalability_tests {
        use super::*;

        #[test]
        fn test_scalability_data_new() {
            let data = ScalabilityData::new();
            assert!(data.points.is_empty());
        }

        #[test]
        fn test_scalability_data_add_point() {
            let mut data = ScalabilityData::new();
            data.add_point(8, 15000.0, 120000.0);
            assert_eq!(data.points.len(), 1);
            assert_eq!(data.points[0].tenor_count, 8);
        }

        #[test]
        fn test_scalability_data_sample() {
            let data = ScalabilityData::sample();
            assert!(!data.points.is_empty());
            assert_eq!(data.points.len(), 6);
        }

        #[test]
        fn test_scalability_data_chart_data() {
            let mut data = ScalabilityData::new();
            data.add_point(4, 16000.0, 60000.0);
            data.add_point(8, 16500.0, 120000.0);

            let aad_data = data.aad_chart_data();
            let bump_data = data.bump_chart_data();

            assert_eq!(aad_data.len(), 2);
            assert_eq!(bump_data.len(), 2);
            assert!((aad_data[0].0 - 4.0).abs() < 1e-10);
            assert!((aad_data[0].1 - 16.0).abs() < 1e-10); // ns -> us
        }

        #[test]
        fn test_scalability_max_values() {
            let data = ScalabilityData::sample();
            assert!(data.max_tenor_count() > 0.0);
            assert!(data.max_time_us() > 0.0);
        }

        #[test]
        fn test_scalability_chartjs_json() {
            let data = ScalabilityData::sample();
            let json = ScalabilityVisualiser::to_chartjs_json(&data);

            assert_eq!(json["type"], "line");
            assert!(json["data"]["labels"].as_array().is_some());
            assert!(json["data"]["datasets"].as_array().is_some());
        }
    }

    // =========================================================================
    // Task 7.4: AccuracyVerificationData Tests
    // =========================================================================

    mod accuracy_tests {
        use super::*;

        #[test]
        fn test_accuracy_data_new() {
            let tenors = vec![1.0, 2.0];
            let aad = vec![100.0, 200.0];
            let bump = vec![100.0, 200.0];

            let data = AccuracyVerificationData::new(tenors, aad, bump);
            assert_eq!(data.tenors.len(), 2);
            assert_eq!(data.relative_errors.len(), 2);
        }

        #[test]
        fn test_accuracy_data_error_calculation() {
            let tenors = vec![1.0];
            let aad = vec![100.1];
            let bump = vec![100.0];

            let data = AccuracyVerificationData::new(tenors, aad, bump);
            // Relative error should be (100.1 - 100.0) / 100.0 = 0.001
            assert!((data.relative_errors[0] - 0.001).abs() < 1e-10);
        }

        #[test]
        fn test_accuracy_data_sample() {
            let data = AccuracyVerificationData::sample();
            assert!(!data.tenors.is_empty());
            assert_eq!(data.tenors.len(), 6);
            // Sample data should have very small errors
            assert!(data.max_relative_error() < 1e-6);
        }

        #[test]
        fn test_accuracy_max_error() {
            let tenors = vec![1.0, 2.0];
            let aad = vec![100.0, 200.2];
            let bump = vec![100.0, 200.0];

            let data = AccuracyVerificationData::new(tenors, aad, bump);
            assert!(data.max_relative_error() > 0.0);
        }

        #[test]
        fn test_accuracy_mean_error() {
            let tenors = vec![1.0];
            let aad = vec![100.1];
            let bump = vec![100.0];

            let data = AccuracyVerificationData::new(tenors, aad, bump);
            assert!((data.mean_relative_error() - 0.001).abs() < 1e-10);
        }

        #[test]
        fn test_accuracy_json() {
            let data = AccuracyVerificationData::sample();
            let json = AccuracyVisualiser::to_json(&data);

            assert!(json["summary"]["max_relative_error"].as_f64().is_some());
            assert!(json["summary"]["mean_relative_error"].as_f64().is_some());
            assert!(json["points"].as_array().is_some());
        }
    }

    // =========================================================================
    // Task 7.1: ComputationFlowDiagram Tests
    // =========================================================================

    mod computation_flow_diagram_tests {
        use super::*;

        #[test]
        fn test_new() {
            let diagram = ComputationFlowDiagram::new();
            let _ = diagram;
        }

        #[test]
        fn test_default() {
            let diagram = ComputationFlowDiagram::default();
            let _ = diagram;
        }

        #[test]
        fn test_to_flow_json_structure() {
            let json = ComputationFlowDiagram::to_flow_json();

            // Check AAD section
            assert!(json.get("aad").is_some());
            assert!(json["aad"]["name"].as_str().unwrap().contains("AAD"));
            assert!(json["aad"]["steps"].as_array().is_some());
            assert!(json["aad"]["complexity"]["time"].as_str().is_some());
            assert!(json["aad"]["complexity"]["space"].as_str().is_some());

            // Check Bump section
            assert!(json.get("bump").is_some());
            assert!(json["bump"]["name"].as_str().unwrap().contains("Bump"));
            assert!(json["bump"]["steps"].as_array().is_some());

            // Check comparison section
            assert!(json.get("comparison").is_some());
            assert!(json["comparison"]["speedup"].as_str().is_some());
        }

        #[test]
        fn test_to_flow_json_aad_steps() {
            let json = ComputationFlowDiagram::to_flow_json();
            let steps = json["aad"]["steps"].as_array().unwrap();

            assert_eq!(steps.len(), 2);
            assert!(steps[0]["name"].as_str().unwrap().contains("Forward"));
            assert!(steps[1]["name"].as_str().unwrap().contains("Backward"));
        }

        #[test]
        fn test_to_flow_json_bump_steps() {
            let json = ComputationFlowDiagram::to_flow_json();
            let steps = json["bump"]["steps"].as_array().unwrap();

            assert_eq!(steps.len(), 4);
            assert!(steps[0]["name"].as_str().unwrap().contains("Loop"));
            assert!(steps[1]["name"].as_str().unwrap().contains("Bump"));
            assert!(steps[2]["name"].as_str().unwrap().contains("Revalue"));
            assert!(steps[3]["name"].as_str().unwrap().contains("Difference"));
        }

        #[test]
        fn test_to_flow_json_complexity() {
            let json = ComputationFlowDiagram::to_flow_json();

            // AAD is O(1) time
            assert!(json["aad"]["complexity"]["time"]
                .as_str()
                .unwrap()
                .contains("O(1)"));

            // Bump is O(n) time
            assert!(json["bump"]["complexity"]["time"]
                .as_str()
                .unwrap()
                .contains("O(n)"));
        }
    }
}

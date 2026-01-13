//! 可視化モジュール: ベンチマーク結果のチャート表示機能
//!
//! # Task Coverage
//!
//! - Task 7.2: 速度比較チャートの実装
//!   - ベンチマーク結果を速度比較バーチャートとして表示
//!   - TUIモードではratatuiのChartウィジェットを使用
//!   - Webモードではchart.js互換JSONデータを出力
//!
//! # Requirements Coverage
//!
//! - Requirement 7.2: 速度比較のバーチャートを表示
//! - Requirement 7.4: TUIモードではratatuiのチャートウィジェットを使用
//! - Requirement 7.5: Webモードではchart.js互換のJSONデータを出力

use ratatui::{
    prelude::*,
    symbols,
    widgets::{Axis, BarChart, Block, Borders, Chart, Dataset, GraphType, Paragraph},
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
            .value_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
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
    vec![
        (0.0, data.aad_mean_us()),
        (1.0, data.bump_mean_us()),
    ]
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
            let result = format_ratio(150.5);
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
            assert!(json.contains("\"type\": \"bar\""));
        }
    }
}

//! IRS AAD Demo画面のレンダリング機能。
//!
//! このモジュールは、IRS AAD（Adjoint Algorithmic Differentiation）デモ用の
//! TUI画面レンダリング機能を提供する。
//!
//! # タスクカバレッジ
//!
//! - タスク 6.2: TUI画面の実装
//!   - IRS AAD Demo用のTUI画面（IrsAadScreen）
//!   - IRSパラメータ入力フォーム
//!   - PV、Greeks、計算時間の結果表示
//!   - パラメータ変更時の即時再計算と結果更新
//!
//! # 要件カバレッジ
//!
//! - 要件 6.2: IRSパラメータ入力処理
//! - 要件 6.3: PV、Greeks、計算時間の結果表示
//! - 要件 6.6: パラメータ変更時の即時再計算

// enzyme-ad feature is defined in pricer_pricing, not in this crate
#![allow(unexpected_cfgs)]

use ratatui::{
    prelude::*,
    symbols,
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table},
};

use pricer_pricing::greeks::GreeksMode;
#[allow(unused_imports)]
use pricer_pricing::irs_greeks::{DeltaBenchmarkResult, TimingStats};

// =============================================================================
// データ構造
// =============================================================================

/// IRS AAD画面用のIRSパラメータ。
///
/// frictional_bank::workflow::IrsParamsの表示用サブセット。
#[derive(Clone, Debug)]
pub struct IrsDisplayParams {
    /// 想定元本。
    pub notional: f64,
    /// 固定レート（年率）。
    pub fixed_rate: f64,
    /// テナー（年数）。
    pub tenor_years: f64,
    /// 通貨コード。
    pub currency: String,
    /// 固定払いフラグ。
    pub pay_fixed: bool,
}

impl Default for IrsDisplayParams {
    fn default() -> Self {
        Self {
            notional: 1_000_000.0,
            fixed_rate: 0.03,
            tenor_years: 5.0,
            currency: "USD".to_string(),
            pay_fixed: true,
        }
    }
}

/// IRS計算結果の表示用構造体。
#[derive(Clone, Debug)]
pub struct IrsDisplayResult {
    /// 正味現在価値（NPV）。
    pub npv: f64,
    /// DV01（1bp金利変動に対するPV変化）。
    pub dv01: f64,
    /// テナーポイント。
    pub tenors: Vec<f64>,
    /// テナー別デルタ値。
    pub tenor_deltas: Vec<f64>,
    /// 計算時間（ナノ秒）。
    pub compute_time_ns: u64,
    /// 計算モード。
    pub mode: GreeksMode,
}

impl Default for IrsDisplayResult {
    fn default() -> Self {
        Self {
            npv: 0.0,
            dv01: 0.0,
            tenors: Vec::new(),
            tenor_deltas: Vec::new(),
            compute_time_ns: 0,
            mode: GreeksMode::BumpRevalue,
        }
    }
}

/// IRS AAD画面のデータ。
#[derive(Clone, Debug)]
pub struct IrsAadScreenData {
    /// IRSパラメータ。
    pub params: IrsDisplayParams,
    /// 計算結果（計算済みの場合）。
    pub result: Option<IrsDisplayResult>,
    /// ベンチマーク結果（実行済みの場合）。
    pub benchmark: Option<DeltaBenchmarkResult>,
    /// 計算モード。
    pub mode: GreeksMode,
    /// フォーカスされているフィールドのインデックス。
    pub selected_field: usize,
}

impl Default for IrsAadScreenData {
    fn default() -> Self {
        Self {
            params: IrsDisplayParams::default(),
            result: None,
            benchmark: None,
            mode: GreeksMode::BumpRevalue,
            selected_field: 0,
        }
    }
}

impl IrsAadScreenData {
    /// 新しいIRS AAD画面データを作成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// パラメータを設定する。
    pub fn with_params(mut self, params: IrsDisplayParams) -> Self {
        self.params = params;
        self
    }

    /// 計算結果を設定する。
    pub fn with_result(mut self, result: IrsDisplayResult) -> Self {
        self.result = Some(result);
        self
    }

    /// ベンチマーク結果を設定する。
    pub fn with_benchmark(mut self, benchmark: DeltaBenchmarkResult) -> Self {
        self.benchmark = Some(benchmark);
        self
    }

    /// 計算モードを設定する。
    pub fn with_mode(mut self, mode: GreeksMode) -> Self {
        self.mode = mode;
        self
    }
}

// =============================================================================
// 描画関数
// =============================================================================

/// IRS AADデモ画面を描画する。
///
/// # 引数
///
/// * `frame` - レンダリングフレーム
/// * `area` - 描画領域
/// * `data` - 画面データ
///
/// # 要件カバレッジ
///
/// - 要件 6.2: IRSパラメータ入力処理
/// - 要件 6.3: PV、Greeks、計算時間の結果表示
pub fn draw_irs_aad_screen(frame: &mut Frame, area: Rect, data: &IrsAadScreenData) {
    // 縦3分割レイアウト
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // パラメータセクション
            Constraint::Length(12), // 結果セクション
            Constraint::Min(0),     // テナーデルタテーブル
        ])
        .split(area);

    // 上部を左右に分割
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // パラメータセクション
    draw_parameters_section(frame, top_chunks[0], &data.params, data.selected_field);

    // モード選択セクション
    draw_mode_section(frame, top_chunks[1], data.mode, data.benchmark.as_ref());

    // 結果セクション
    draw_results_section(frame, chunks[1], data.result.as_ref());

    // テナーデルタテーブル
    draw_tenor_delta_table(frame, chunks[2], data.result.as_ref());
}

/// パラメータセクションを描画する。
fn draw_parameters_section(
    frame: &mut Frame,
    area: Rect,
    params: &IrsDisplayParams,
    selected_field: usize,
) {
    let direction_text = if params.pay_fixed {
        "Pay Fixed"
    } else {
        "Receive Fixed"
    };

    let fields = vec![
        Line::from(vec![
            Span::styled("Notional:    ", Style::default().fg(Color::Yellow)),
            Span::raw(format_with_commas(params.notional)),
            Span::styled(" ", Style::default()),
            Span::styled(&params.currency, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Fixed Rate:  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:.2}%", params.fixed_rate * 100.0),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tenor:       ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.0}Y", params.tenor_years)),
        ]),
        Line::from(vec![
            Span::styled("Direction:   ", Style::default().fg(Color::Yellow)),
            Span::styled(
                direction_text,
                Style::default().fg(if params.pay_fixed {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
    ];

    // 選択されたフィールドをハイライト
    let mut content = vec![Line::from("")];
    for (i, field) in fields.into_iter().enumerate() {
        // フィールドのスパンを取得して先頭にマーカーを追加
        let mut spans: Vec<Span> = field.spans;
        if i == selected_field {
            spans.insert(0, Span::styled("> ", Style::default().fg(Color::Cyan)));
        } else {
            spans.insert(0, Span::raw("  "));
        }
        content.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" IRS Parameters ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// モード選択セクションを描画する。
fn draw_mode_section(
    frame: &mut Frame,
    area: Rect,
    mode: GreeksMode,
    benchmark: Option<&DeltaBenchmarkResult>,
) {
    let mode_str = match mode {
        GreeksMode::BumpRevalue => "Bump-and-Revalue",
        GreeksMode::NumDual => "NumDual (Forward AD)",
        #[cfg(feature = "enzyme-ad")]
        GreeksMode::EnzymeAAD => "Enzyme AAD",
        #[allow(unreachable_patterns)]
        _ => "Bump-and-Revalue",
    };

    let mut content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(Color::Yellow)),
            Span::styled(mode_str, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
    ];

    // ベンチマーク結果がある場合は速度比を表示
    if let Some(bench) = benchmark {
        content.push(Line::from(""));
        content.push(Line::from(vec![Span::styled(
            "--- Benchmark Results ---",
            Style::default().fg(Color::Cyan),
        )]));
        content.push(Line::from(vec![
            Span::styled("Speedup: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:.1}x", bench.speedup_ratio),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" faster with AAD", Style::default().fg(Color::DarkGray)),
        ]));
        content.push(Line::from(vec![
            Span::styled("Tenor count: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}", bench.tenor_count)),
        ]));
    }

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Calculation Mode ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// 結果セクションを描画する。
fn draw_results_section(frame: &mut Frame, area: Rect, result: Option<&IrsDisplayResult>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // 価格評価結果
    let valuation_content = if let Some(r) = result {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("NPV:         ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format_with_commas(r.npv),
                    Style::default().fg(if r.npv >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("DV01:        ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.4}", r.dv01),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("DV01 (1MM):  ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.2}", r.dv01 * 10_000.0),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Press [Enter] to calculate",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let valuation = Paragraph::new(valuation_content).block(
        Block::default()
            .title(" Valuation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(valuation, chunks[0]);

    // 計算時間
    let timing_content = if let Some(r) = result {
        let time_us = r.compute_time_ns as f64 / 1000.0;
        let time_ms = time_us / 1000.0;

        let mode_str = match r.mode {
            GreeksMode::BumpRevalue => "Bump",
            GreeksMode::NumDual => "NumDual",
            #[cfg(feature = "enzyme-ad")]
            GreeksMode::EnzymeAAD => "AAD",
            #[allow(unreachable_patterns)]
            _ => "Bump",
        };

        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Mode:        ", Style::default().fg(Color::Yellow)),
                Span::raw(mode_str),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Time:        ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    if time_ms >= 1.0 {
                        format!("{:.2} ms", time_ms)
                    } else {
                        format!("{:.1} us", time_us)
                    },
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Tenor points:", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {}", r.tenors.len())),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No timing data",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let timing = Paragraph::new(timing_content).block(
        Block::default()
            .title(" Computation Time ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(timing, chunks[1]);
}

/// テナーデルタテーブルを描画する。
fn draw_tenor_delta_table(frame: &mut Frame, area: Rect, result: Option<&IrsDisplayResult>) {
    let header_cells = ["Tenor", "Delta", "Delta/bp"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = if let Some(r) = result {
        r.tenors
            .iter()
            .zip(r.tenor_deltas.iter())
            .map(|(tenor, delta)| {
                let delta_color = if *delta >= 0.0 {
                    Color::Green
                } else {
                    Color::Red
                };
                Row::new(vec![
                    Cell::from(format!("{:.2}Y", tenor)),
                    Cell::from(format!("{:.4}", delta)).style(Style::default().fg(delta_color)),
                    Cell::from(format!("{:.2}", delta * 10_000.0))
                        .style(Style::default().fg(delta_color)),
                ])
            })
            .collect()
    } else {
        vec![Row::new(vec![Cell::from(Span::styled(
            "No delta data available",
            Style::default().fg(Color::DarkGray),
        ))])]
    };

    let widths = [
        Constraint::Length(10),
        Constraint::Length(15),
        Constraint::Length(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" Tenor Deltas ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, area);
}

/// ベンチマーク速度比較チャートを描画する。
///
/// # 引数
///
/// * `frame` - レンダリングフレーム
/// * `area` - 描画領域
/// * `benchmark` - ベンチマーク結果
///
/// # 要件カバレッジ
///
/// - 要件 7.2: 速度比較チャート
pub fn draw_irs_benchmark_chart(
    frame: &mut Frame,
    area: Rect,
    benchmark: Option<&DeltaBenchmarkResult>,
) {
    if let Some(bench) = benchmark {
        // AADとBumpの計算時間を比較するバーチャートデータ
        let aad_time_us = bench.aad_stats.mean_ns / 1000.0;
        let bump_time_us = bench.bump_stats.mean_ns / 1000.0;

        // 棒グラフを折れ線で近似表現
        // 各手法を2点で表現（開始点と終了点）
        let aad_data: Vec<(f64, f64)> = vec![(0.0, aad_time_us), (1.0, aad_time_us)];
        let bump_data: Vec<(f64, f64)> = vec![(2.0, bump_time_us), (3.0, bump_time_us)];

        let max_time = aad_time_us.max(bump_time_us) * 1.2;

        let datasets = vec![
            Dataset::default()
                .name("AAD")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&aad_data),
            Dataset::default()
                .name("Bump")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Red))
                .data(&bump_data),
        ];

        let x_labels = vec![
            Span::raw("AAD"),
            Span::raw(""),
            Span::raw("Bump"),
            Span::raw(""),
        ];

        let y_labels = vec![
            Span::raw("0"),
            Span::raw(format!("{:.0}", max_time / 2.0)),
            Span::raw(format!("{:.0} us", max_time)),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(format!(
                        " Speed Comparison (Speedup: {:.1}x) ",
                        bench.speedup_ratio
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .x_axis(
                Axis::default()
                    .title("Method")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 4.0])
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Time (us)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, max_time])
                    .labels(y_labels),
            );

        frame.render_widget(chart, area);
    } else {
        let paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Run benchmark to see speed comparison",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Speed Comparison ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        frame.render_widget(paragraph, area);
    }
}

// =============================================================================
// ユーティリティ関数
// =============================================================================

/// 数値をカンマ区切りでフォーマットする。
fn format_with_commas(n: f64) -> String {
    let abs_n = n.abs();
    let sign = if n < 0.0 { "-" } else { "" };

    if abs_n >= 1_000_000.0 {
        format!("{}{:.2}M", sign, abs_n / 1_000_000.0)
    } else if abs_n >= 1_000.0 {
        format!("{}{:.2}K", sign, abs_n / 1_000.0)
    } else {
        format!("{}{:.2}", sign, abs_n)
    }
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // IrsDisplayParams テスト
    // =========================================================================

    mod params_tests {
        use super::*;

        #[test]
        fn test_default_params() {
            let params = IrsDisplayParams::default();
            assert!((params.notional - 1_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.03).abs() < 1e-10);
            assert!((params.tenor_years - 5.0).abs() < 1e-10);
            assert_eq!(params.currency, "USD");
            assert!(params.pay_fixed);
        }

        #[test]
        fn test_params_with_custom_values() {
            let params = IrsDisplayParams {
                notional: 10_000_000.0,
                fixed_rate: 0.025,
                tenor_years: 10.0,
                currency: "EUR".to_string(),
                pay_fixed: false,
            };
            assert!((params.notional - 10_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.025).abs() < 1e-10);
            assert!((params.tenor_years - 10.0).abs() < 1e-10);
            assert_eq!(params.currency, "EUR");
            assert!(!params.pay_fixed);
        }
    }

    // =========================================================================
    // IrsDisplayResult テスト
    // =========================================================================

    mod result_tests {
        use super::*;

        #[test]
        fn test_default_result() {
            let result = IrsDisplayResult::default();
            assert!((result.npv - 0.0).abs() < 1e-10);
            assert!((result.dv01 - 0.0).abs() < 1e-10);
            assert!(result.tenors.is_empty());
            assert!(result.tenor_deltas.is_empty());
            assert_eq!(result.compute_time_ns, 0);
            assert_eq!(result.mode, GreeksMode::BumpRevalue);
        }

        #[test]
        fn test_result_with_values() {
            let result = IrsDisplayResult {
                npv: 25000.0,
                dv01: 0.0045,
                tenors: vec![1.0, 2.0, 5.0, 10.0],
                tenor_deltas: vec![100.0, 200.0, 450.0, 500.0],
                compute_time_ns: 1_500_000,
                mode: GreeksMode::BumpRevalue,
            };
            assert!((result.npv - 25000.0).abs() < 1e-10);
            assert!((result.dv01 - 0.0045).abs() < 1e-10);
            assert_eq!(result.tenors.len(), 4);
            assert_eq!(result.tenor_deltas.len(), 4);
            assert_eq!(result.compute_time_ns, 1_500_000);
        }
    }

    // =========================================================================
    // IrsAadScreenData テスト
    // =========================================================================

    mod screen_data_tests {
        use super::*;

        #[test]
        fn test_default_screen_data() {
            let data = IrsAadScreenData::default();
            assert!(data.result.is_none());
            assert!(data.benchmark.is_none());
            assert_eq!(data.mode, GreeksMode::BumpRevalue);
            assert_eq!(data.selected_field, 0);
        }

        #[test]
        fn test_screen_data_builder() {
            let params = IrsDisplayParams {
                notional: 5_000_000.0,
                ..Default::default()
            };
            let result = IrsDisplayResult {
                npv: 50000.0,
                dv01: 0.005,
                ..Default::default()
            };

            let data = IrsAadScreenData::new()
                .with_params(params)
                .with_result(result)
                .with_mode(GreeksMode::NumDual);

            assert!((data.params.notional - 5_000_000.0).abs() < 1e-10);
            assert!(data.result.is_some());
            assert!((data.result.unwrap().npv - 50000.0).abs() < 1e-10);
            assert_eq!(data.mode, GreeksMode::NumDual);
        }

        #[test]
        fn test_screen_data_with_benchmark() {
            let benchmark = DeltaBenchmarkResult {
                aad_stats: TimingStats {
                    mean_ns: 1000.0,
                    std_dev_ns: 100.0,
                    min_ns: 800,
                    max_ns: 1200,
                    sample_count: 100,
                },
                bump_stats: TimingStats {
                    mean_ns: 10000.0,
                    std_dev_ns: 500.0,
                    min_ns: 9000,
                    max_ns: 11000,
                    sample_count: 100,
                },
                speedup_ratio: 10.0,
                tenor_count: 8,
            };

            let data = IrsAadScreenData::new().with_benchmark(benchmark);
            assert!(data.benchmark.is_some());
            let bench = data.benchmark.unwrap();
            assert!((bench.speedup_ratio - 10.0).abs() < 1e-10);
            assert_eq!(bench.tenor_count, 8);
        }
    }

    // =========================================================================
    // フォーマット関数テスト
    // =========================================================================

    mod format_tests {
        use super::*;

        #[test]
        fn test_format_with_commas_small() {
            let result = format_with_commas(123.45);
            assert_eq!(result, "123.45");
        }

        #[test]
        fn test_format_with_commas_thousands() {
            let result = format_with_commas(12345.67);
            assert_eq!(result, "12.35K");
        }

        #[test]
        fn test_format_with_commas_millions() {
            let result = format_with_commas(1234567.89);
            assert_eq!(result, "1.23M");
        }

        #[test]
        fn test_format_with_commas_negative() {
            let result = format_with_commas(-50000.0);
            assert_eq!(result, "-50.00K");
        }
    }

    // =========================================================================
    // 描画関数テスト（レンダリング可能かの検証）
    // =========================================================================

    mod draw_tests {
        use super::*;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        fn create_test_terminal() -> Terminal<TestBackend> {
            let backend = TestBackend::new(80, 40);
            Terminal::new(backend).unwrap()
        }

        #[test]
        fn test_draw_irs_aad_screen_no_result() {
            let mut terminal = create_test_terminal();
            let data = IrsAadScreenData::default();

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_aad_screen(frame, area, &data);
                })
                .unwrap();

            // 描画が成功することを確認（パニックしない）
        }

        #[test]
        fn test_draw_irs_aad_screen_with_result() {
            let mut terminal = create_test_terminal();
            let result = IrsDisplayResult {
                npv: 25000.0,
                dv01: 0.0045,
                tenors: vec![1.0, 2.0, 5.0],
                tenor_deltas: vec![100.0, 200.0, 450.0],
                compute_time_ns: 1_500_000,
                mode: GreeksMode::BumpRevalue,
            };
            let data = IrsAadScreenData::new().with_result(result);

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_aad_screen(frame, area, &data);
                })
                .unwrap();

            // 描画が成功することを確認
        }

        #[test]
        fn test_draw_irs_aad_screen_with_benchmark() {
            let mut terminal = create_test_terminal();
            let benchmark = DeltaBenchmarkResult {
                aad_stats: TimingStats {
                    mean_ns: 1000.0,
                    std_dev_ns: 100.0,
                    min_ns: 800,
                    max_ns: 1200,
                    sample_count: 100,
                },
                bump_stats: TimingStats {
                    mean_ns: 10000.0,
                    std_dev_ns: 500.0,
                    min_ns: 9000,
                    max_ns: 11000,
                    sample_count: 100,
                },
                speedup_ratio: 10.0,
                tenor_count: 8,
            };
            let data = IrsAadScreenData::new().with_benchmark(benchmark);

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_aad_screen(frame, area, &data);
                })
                .unwrap();

            // 描画が成功することを確認
        }

        #[test]
        fn test_draw_benchmark_chart_no_data() {
            let mut terminal = create_test_terminal();

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_benchmark_chart(frame, area, None);
                })
                .unwrap();

            // 描画が成功することを確認
        }

        #[test]
        fn test_draw_benchmark_chart_with_data() {
            let mut terminal = create_test_terminal();
            let benchmark = DeltaBenchmarkResult {
                aad_stats: TimingStats {
                    mean_ns: 1000.0,
                    std_dev_ns: 100.0,
                    min_ns: 800,
                    max_ns: 1200,
                    sample_count: 100,
                },
                bump_stats: TimingStats {
                    mean_ns: 10000.0,
                    std_dev_ns: 500.0,
                    min_ns: 9000,
                    max_ns: 11000,
                    sample_count: 100,
                },
                speedup_ratio: 10.0,
                tenor_count: 8,
            };

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_benchmark_chart(frame, area, Some(&benchmark));
                })
                .unwrap();

            // 描画が成功することを確認
        }

        #[test]
        fn test_draw_with_negative_npv() {
            let mut terminal = create_test_terminal();
            let result = IrsDisplayResult {
                npv: -15000.0,
                dv01: -0.003,
                tenors: vec![1.0, 2.0],
                tenor_deltas: vec![-50.0, -100.0],
                compute_time_ns: 500_000,
                mode: GreeksMode::BumpRevalue,
            };
            let data = IrsAadScreenData::new().with_result(result);

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_aad_screen(frame, area, &data);
                })
                .unwrap();

            // 負の値でも描画が成功することを確認
        }

        #[test]
        fn test_draw_with_empty_deltas() {
            let mut terminal = create_test_terminal();
            let result = IrsDisplayResult {
                npv: 10000.0,
                dv01: 0.002,
                tenors: Vec::new(),
                tenor_deltas: Vec::new(),
                compute_time_ns: 100_000,
                mode: GreeksMode::BumpRevalue,
            };
            let data = IrsAadScreenData::new().with_result(result);

            terminal
                .draw(|frame| {
                    let area = frame.size();
                    draw_irs_aad_screen(frame, area, &data);
                })
                .unwrap();

            // 空のデルタリストでも描画が成功することを確認
        }
    }
}

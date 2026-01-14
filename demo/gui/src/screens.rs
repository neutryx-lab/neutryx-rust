//! Screen rendering functions for the TUI.

mod irs_aad;

pub use irs_aad::{
    draw_irs_aad_screen, draw_irs_benchmark_chart, IrsAadScreenData, IrsDisplayParams,
    IrsDisplayResult,
};

use crate::app::{ExposureTimeSeries, IrsAadDemoState, RiskMetrics, TradeRow};
use ratatui::{
    prelude::*,
    symbols,
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table},
};

/// Format a number with thousands separators
fn format_number(n: f64, decimals: usize) -> String {
    if decimals == 0 {
        format!("{:.0}", n)
    } else {
        format!("{:.1$}", n, decimals)
    }
}

/// Draw dashboard screen
pub fn draw_dashboard(frame: &mut Frame, area: Rect, metrics: &RiskMetrics) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Portfolio summary
    let portfolio_text = vec![
        Line::from(vec![
            Span::raw("Total PV: "),
            Span::styled(
                format_number(metrics.total_pv, 2),
                Style::default().fg(if metrics.total_pv >= 0.0 {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::raw("XVA Adjustments:")]),
        Line::from(vec![
            Span::raw("  CVA: "),
            Span::styled(
                format_number(metrics.cva, 2),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::raw("  DVA: "),
            Span::styled(
                format_number(metrics.dva, 2),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("  FVA: "),
            Span::styled(
                format_number(metrics.fva, 2),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];

    let portfolio = Paragraph::new(portfolio_text).block(
        Block::default()
            .title(" Portfolio Summary ")
            .borders(Borders::ALL),
    );
    frame.render_widget(portfolio, chunks[0]);

    // Risk metrics
    let risk_text = vec![
        Line::from(vec![
            Span::raw("Expected Exposure (EE): "),
            Span::styled(
                format_number(metrics.ee, 2),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Expected Positive Exp: "),
            Span::styled(
                format_number(metrics.epe, 2),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Potential Future Exp:  "),
            Span::styled(
                format_number(metrics.pfe, 2),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];

    let risk = Paragraph::new(risk_text).block(
        Block::default()
            .title(" Exposure Metrics ")
            .borders(Borders::ALL),
    );
    frame.render_widget(risk, chunks[1]);
}

/// Draw portfolio screen
pub fn draw_portfolio(frame: &mut Frame, area: Rect, trades: &[TradeRow], selected: usize) {
    let header_cells = [
        "ID",
        "Instrument",
        "Notional",
        "PV",
        "Delta",
        "Gamma",
        "Vega",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells).height(1);

    let rows = trades.iter().enumerate().map(|(idx, trade)| {
        let style = if idx == selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(trade.id.clone()),
            Cell::from(trade.instrument.clone()),
            Cell::from(format_number(trade.notional, 0)),
            Cell::from(format_number(trade.pv, 2)).style(Style::default().fg(if trade.pv >= 0.0 {
                Color::Green
            } else {
                Color::Red
            })),
            Cell::from(format!("{:.4}", trade.delta)),
            Cell::from(format!("{:.4}", trade.gamma)),
            Cell::from(format!("{:.4}", trade.vega)),
        ])
        .style(style)
    });

    let widths = [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(15),
        Constraint::Length(15),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(" Portfolio ").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, area);
}

/// Draw risk screen
pub fn draw_risk(frame: &mut Frame, area: Rect, metrics: &RiskMetrics) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // XVA metrics
    let xva_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  CVA (Credit Value Adjustment):    ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.cva),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  DVA (Debit Value Adjustment):     ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.dva),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  FVA (Funding Value Adjustment):   ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.fva),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Total XVA:                        ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.cva + metrics.dva + metrics.fva),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let xva = Paragraph::new(xva_text).block(
        Block::default()
            .title(" XVA Metrics ")
            .borders(Borders::ALL),
    );
    frame.render_widget(xva, chunks[0]);

    // Exposure metrics
    let exposure_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  EE (Expected Exposure):           ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.ee),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  EPE (Expected Positive Exposure): ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.epe),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  PFE (Potential Future Exposure):  ",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{:>12.2}", metrics.pfe),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];

    let exposure = Paragraph::new(exposure_text).block(
        Block::default()
            .title(" Exposure Metrics ")
            .borders(Borders::ALL),
    );
    frame.render_widget(exposure, chunks[1]);
}

/// Draw trade blotter screen
pub fn draw_trade_blotter(frame: &mut Frame, area: Rect, trade: Option<&TradeRow>) {
    let content = if let Some(t) = trade {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Trade ID:    ", Style::default().fg(Color::Yellow)),
                Span::raw(&t.id),
            ]),
            Line::from(vec![
                Span::styled("Instrument:  ", Style::default().fg(Color::Yellow)),
                Span::raw(&t.instrument),
            ]),
            Line::from(vec![
                Span::styled("Notional:    ", Style::default().fg(Color::Yellow)),
                Span::raw(format_number(t.notional, 2)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "--- Valuation ---",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(vec![
                Span::styled("PV:          ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format_number(t.pv, 2),
                    Style::default().fg(if t.pv >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "--- Greeks ---",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(vec![
                Span::styled("Delta:       ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:.6}", t.delta)),
            ]),
            Line::from(vec![
                Span::styled("Gamma:       ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:.6}", t.gamma)),
            ]),
            Line::from(vec![
                Span::styled("Vega:        ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:.6}", t.vega)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No trade selected",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let blotter = Paragraph::new(content).block(
        Block::default()
            .title(" Trade Details ")
            .borders(Borders::ALL),
    );
    frame.render_widget(blotter, area);
}

/// Draw exposure chart screen with time series
pub fn draw_exposure_chart(frame: &mut Frame, area: Rect, series: &ExposureTimeSeries) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(area);

    // Prepare chart data
    let ee_data: Vec<(f64, f64)> = series.ee_data();
    let epe_data: Vec<(f64, f64)> = series.epe_data();
    let pfe_data: Vec<(f64, f64)> = series.pfe_data();
    let ene_data: Vec<(f64, f64)> = series.ene_data();

    let x_bounds = series.x_bounds();
    let y_bounds = series.y_bounds();

    // Create datasets for the chart
    let datasets = vec![
        Dataset::default()
            .name("PFE (95%)")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&pfe_data),
        Dataset::default()
            .name("EE")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&ee_data),
        Dataset::default()
            .name("EPE")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&epe_data),
        Dataset::default()
            .name("ENE")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&ene_data),
    ];

    // Create X axis labels
    let x_labels: Vec<Span> = (0..=10).map(|i| Span::raw(format!("{}Y", i))).collect();

    // Create Y axis labels (format large numbers)
    let y_min = y_bounds[0];
    let y_max = y_bounds[1];
    let y_labels: Vec<Span> = vec![
        Span::raw(format_k(y_min)),
        Span::raw(format_k((y_min + y_max) / 2.0)),
        Span::raw(format_k(y_max)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(" Exposure Profile Over Time ")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (Years)")
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Exposure")
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(y_labels),
        );

    frame.render_widget(chart, chunks[0]);

    // Legend
    let legend = Paragraph::new(vec![Line::from(vec![
        Span::styled(" PFE(95%) ", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(" EE ", Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(" EPE ", Style::default().fg(Color::Green)),
        Span::raw(" | "),
        Span::styled(" ENE ", Style::default().fg(Color::Red)),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Legend "));
    frame.render_widget(legend, chunks[1]);
}

/// Format large numbers with K/M suffix
fn format_k(n: f64) -> String {
    let abs_n = n.abs();
    if abs_n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if abs_n >= 1_000.0 {
        format!("{:.0}K", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}

// =============================================================================
// Task 6.2: IRS AAD Demo Screen (using IrsAadDemoState from app.rs)
// =============================================================================

/// Draw the IRS AAD Demo screen (Task 6.2)
///
/// This function renders the IRS AAD Demo screen using the state
/// from the TUI application. It displays:
/// - Parameter input form (notional, fixed rate, tenor, num tenors)
/// - Calculation mode selector (Bump/AAD/Both)
/// - Results (NPV, DV01, tenor deltas)
/// - Benchmark timing comparison
///
/// # Requirements Coverage
///
/// - Requirement 6.2: IRS parameter input processing
/// - Requirement 6.3: PV, Greeks, calculation time display
/// - Requirement 6.6: Immediate recalculation on parameter change
pub fn draw_irs_aad_demo(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    // Main layout: 3 rows
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Top: params + mode
            Constraint::Length(10), // Middle: results
            Constraint::Min(0),     // Bottom: benchmark/deltas
        ])
        .split(area);

    // Top section: left = params, right = mode
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    draw_irs_demo_params(frame, top_chunks[0], state);
    draw_irs_demo_mode(frame, top_chunks[1], state);

    // Middle section: results
    draw_irs_demo_results(frame, chunks[1], state);

    // Bottom section: benchmark + deltas chart
    if state.benchmark.is_some() {
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        draw_irs_demo_benchmark(frame, bottom_chunks[0], state);
        draw_irs_demo_delta_chart(frame, bottom_chunks[1], state);
    } else {
        draw_irs_demo_help(frame, chunks[2]);
    }
}

/// Draw IRS parameters section
fn draw_irs_demo_params(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    let params = &state.params;
    let selected = state.selected_field;

    let field_labels = [
        ("Notional:", format_k(params.notional)),
        ("Fixed Rate:", format!("{:.2}%", params.fixed_rate * 100.0)),
        ("Tenor:", format!("{} years", params.tenor_years)),
        ("Tenor Points:", format!("{}", params.num_tenors)),
    ];

    let mut lines = vec![Line::from("")];

    for (i, (label, value)) in field_labels.iter().enumerate() {
        let prefix = if i == selected { "> " } else { "  " };
        let style = if i == selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{:<14}", label), Style::default().fg(Color::Yellow)),
            Span::styled(value.clone(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Left/Right] Adjust  [Up/Down] Select",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" IRS Parameters ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// Draw calculation mode section
fn draw_irs_demo_mode(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    let mode_names = ["Bump-and-Revalue", "AAD (Enzyme)", "Both (Compare)"];
    let mode_str = mode_names.get(state.calc_mode).unwrap_or(&"Unknown");

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                *mode_str,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [Tab] Cycle mode",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    if state.is_calculating {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Calculating...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::SLOW_BLINK),
        )));
    }

    if let Some(ref err) = state.error_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" Calculation Mode ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// Draw results section
fn draw_irs_demo_results(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Valuation results
    let val_content = if let Some(ref result) = state.result {
        let npv_color = if result.npv >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };

        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("NPV:      ", Style::default().fg(Color::Yellow)),
                Span::styled(format_k(result.npv), Style::default().fg(npv_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("DV01:     ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.4}", result.dv01),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("DV01/MM:  ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.2}", result.dv01 * 10_000.0),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Press [Enter] to calculate",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let valuation = Paragraph::new(val_content).block(
        Block::default()
            .title(" Valuation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(valuation, chunks[0]);

    // Timing results
    let time_content = if let Some(ref result) = state.result {
        let time_us = result.compute_time_ns as f64 / 1000.0;
        let time_str = if time_us >= 1000.0 {
            format!("{:.2} ms", time_us / 1000.0)
        } else {
            format!("{:.1} us", time_us)
        };

        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Mode:     ", Style::default().fg(Color::Yellow)),
                Span::raw(&result.mode),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Time:     ", Style::default().fg(Color::Yellow)),
                Span::styled(time_str, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Tenors:   ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", result.tenors.len())),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No timing data",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let timing = Paragraph::new(time_content).block(
        Block::default()
            .title(" Computation Time ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(timing, chunks[1]);
}

/// Draw benchmark comparison section
fn draw_irs_demo_benchmark(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    let content = if let Some(ref bench) = state.benchmark {
        let aad_us = bench.aad_mean_ns / 1000.0;
        let bump_us = bench.bump_mean_ns / 1000.0;

        vec![
            Line::from(""),
            Line::from(Span::styled(
                "--- Speed Comparison ---",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("AAD Time:   ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.1} us", aad_us),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Bump Time:  ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.1} us", bump_us),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Speedup:    ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:.1}x", bench.speedup_ratio),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" faster", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("Tenors:     ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", bench.tenor_count)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No benchmark data",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Benchmark Results ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// Draw delta comparison chart
fn draw_irs_demo_delta_chart(frame: &mut Frame, area: Rect, state: &IrsAadDemoState) {
    if let Some(ref result) = state.result {
        if !result.tenors.is_empty() && !result.deltas.is_empty() {
            // Create chart data
            let data: Vec<(f64, f64)> = result
                .tenors
                .iter()
                .zip(result.deltas.iter())
                .map(|(t, d)| (*t, *d))
                .collect();

            let max_tenor = result.tenors.iter().fold(0.0_f64, |a, &b| a.max(b));
            let max_delta = result.deltas.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
            let min_delta = result.deltas.iter().fold(0.0_f64, |a, &b| a.min(b));

            let y_max = max_delta * 1.2;
            let y_min = if min_delta < 0.0 {
                min_delta * 1.2
            } else {
                0.0
            };

            let datasets = vec![Dataset::default()
                .name("Delta")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&data)];

            let x_labels: Vec<Span> = (0..=max_tenor as u32)
                .step_by(((max_tenor / 4.0).max(1.0)) as usize)
                .map(|i| Span::raw(format!("{}Y", i)))
                .collect();

            let y_labels = vec![
                Span::raw(format_k(y_min)),
                Span::raw(format_k((y_min + y_max) / 2.0)),
                Span::raw(format_k(y_max)),
            ];

            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .title(" Tenor Deltas ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .x_axis(
                    Axis::default()
                        .title("Tenor")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([0.0, max_tenor * 1.1])
                        .labels(x_labels),
                )
                .y_axis(
                    Axis::default()
                        .title("Delta")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([y_min, y_max])
                        .labels(y_labels),
                );

            frame.render_widget(chart, area);
            return;
        }
    }

    // Fallback: no data
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No delta data available",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title(" Tenor Deltas ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

/// Draw help section when no benchmark data
fn draw_irs_demo_help(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "--- IRS AAD Demo Help ---",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from("  This demo compares AAD (Adjoint Algorithmic Differentiation)"),
        Line::from("  with traditional Bump-and-Revalue for IRS Greeks calculation."),
        Line::from(""),
        Line::from(Span::styled(
            "  Controls:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  [Up/Down]    Select parameter"),
        Line::from("  [Left/Right] Adjust parameter value"),
        Line::from("  [Tab]        Cycle calculation mode"),
        Line::from("  [Enter]      Run calculation"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [Enter] to run the calculation",
            Style::default().fg(Color::Green),
        )),
    ];

    let paragraph = Paragraph::new(help_text).block(
        Block::default()
            .title(" IRS AAD Demo ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(paragraph, area);
}

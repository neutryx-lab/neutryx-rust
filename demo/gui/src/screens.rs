//! Screen rendering functions for the TUI.

use crate::app::{ExposureTimeSeries, RiskMetrics, TradeRow};
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
            Span::styled(format_number(metrics.cva, 2), Style::default().fg(Color::Red)),
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

    let portfolio = Paragraph::new(portfolio_text)
        .block(Block::default().title(" Portfolio Summary ").borders(Borders::ALL));
    frame.render_widget(portfolio, chunks[0]);

    // Risk metrics
    let risk_text = vec![
        Line::from(vec![
            Span::raw("Expected Exposure (EE): "),
            Span::styled(format_number(metrics.ee, 2), Style::default().fg(Color::Cyan)),
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

    let risk = Paragraph::new(risk_text)
        .block(Block::default().title(" Exposure Metrics ").borders(Borders::ALL));
    frame.render_widget(risk, chunks[1]);
}

/// Draw portfolio screen
pub fn draw_portfolio(frame: &mut Frame, area: Rect, trades: &[TradeRow], selected: usize) {
    let header_cells = ["ID", "Instrument", "Notional", "PV", "Delta", "Gamma", "Vega"]
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
            Cell::from(format_number(trade.pv, 2)).style(Style::default().fg(
                if trade.pv >= 0.0 {
                    Color::Green
                } else {
                    Color::Red
                },
            )),
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

    let xva = Paragraph::new(xva_text)
        .block(Block::default().title(" XVA Metrics ").borders(Borders::ALL));
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

    let exposure = Paragraph::new(exposure_text)
        .block(Block::default().title(" Exposure Metrics ").borders(Borders::ALL));
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

    let blotter = Paragraph::new(content)
        .block(Block::default().title(" Trade Details ").borders(Borders::ALL));
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
    let x_labels: Vec<Span> = (0..=10)
        .map(|i| Span::raw(format!("{}Y", i)))
        .collect();

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

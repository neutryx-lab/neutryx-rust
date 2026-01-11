//! TUI Application state and event handling.

use crate::api_client::ApiClient;
use crate::screens;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::io::{self, Stdout};
use std::time::Duration;

/// Available screens in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Dashboard with portfolio summary
    Dashboard,
    /// Portfolio view with trade list
    Portfolio,
    /// Risk metrics view
    Risk,
    /// Trade blotter with details
    TradeBlotter,
    /// Exposure time series chart
    Chart,
}

impl Screen {
    /// Get screen title
    pub fn title(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Portfolio => "Portfolio",
            Self::Risk => "Risk",
            Self::TradeBlotter => "Trade Blotter",
            Self::Chart => "Exposure Chart",
        }
    }
}

/// A single data point in the exposure time series
#[derive(Debug, Clone)]
pub struct ExposureDataPoint {
    /// Time in years from valuation date
    pub time: f64,
    /// Expected Exposure
    pub ee: f64,
    /// Expected Positive Exposure
    pub epe: f64,
    /// Potential Future Exposure (95th percentile)
    pub pfe: f64,
    /// Expected Negative Exposure
    pub ene: f64,
}

/// Time series of exposure data for charting
#[derive(Debug, Clone)]
pub struct ExposureTimeSeries {
    /// Collection of data points over time
    pub data_points: Vec<ExposureDataPoint>,
}

impl Default for ExposureTimeSeries {
    fn default() -> Self {
        // Generate sample exposure profile over 10 years
        let data_points: Vec<ExposureDataPoint> = (0..=40)
            .map(|i| {
                let t = i as f64 * 0.25; // quarterly intervals
                // Exposure typically rises then falls (tent-shaped profile)
                let decay = (-0.15 * t).exp();
                let growth = 1.0 - (-0.8 * t).exp();
                let profile = growth * decay;

                ExposureDataPoint {
                    time: t,
                    ee: 500_000.0 * profile + 100_000.0,
                    epe: 450_000.0 * profile + 80_000.0,
                    pfe: 900_000.0 * profile + 150_000.0,
                    ene: -200_000.0 * profile - 50_000.0,
                }
            })
            .collect();

        Self { data_points }
    }
}

impl ExposureTimeSeries {
    /// Convert to chart data format for Expected Exposure
    pub fn ee_data(&self) -> Vec<(f64, f64)> {
        self.data_points
            .iter()
            .map(|p| (p.time, p.ee))
            .collect()
    }

    /// Convert to chart data format for Expected Positive Exposure
    pub fn epe_data(&self) -> Vec<(f64, f64)> {
        self.data_points
            .iter()
            .map(|p| (p.time, p.epe))
            .collect()
    }

    /// Convert to chart data format for Potential Future Exposure
    pub fn pfe_data(&self) -> Vec<(f64, f64)> {
        self.data_points
            .iter()
            .map(|p| (p.time, p.pfe))
            .collect()
    }

    /// Convert to chart data format for Expected Negative Exposure
    pub fn ene_data(&self) -> Vec<(f64, f64)> {
        self.data_points
            .iter()
            .map(|p| (p.time, p.ene))
            .collect()
    }

    /// Get min/max values for Y axis bounds
    pub fn y_bounds(&self) -> [f64; 2] {
        let min = self.data_points.iter().map(|p| p.ene).fold(f64::MAX, f64::min);
        let max = self.data_points.iter().map(|p| p.pfe).fold(f64::MIN, f64::max);
        [min * 1.1, max * 1.1]
    }

    /// Get max time for X axis bounds
    pub fn x_bounds(&self) -> [f64; 2] {
        let max_time = self.data_points.iter().map(|p| p.time).fold(0.0_f64, f64::max);
        [0.0, max_time]
    }
}

/// Trade row for display
#[derive(Debug, Clone)]
pub struct TradeRow {
    pub id: String,
    pub instrument: String,
    pub notional: f64,
    pub pv: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
}

/// Risk metrics for display
#[derive(Debug, Clone, Default)]
pub struct RiskMetrics {
    pub total_pv: f64,
    pub cva: f64,
    pub dva: f64,
    pub fva: f64,
    pub ee: f64,
    pub epe: f64,
    pub pfe: f64,
}

/// Rendering state snapshot
struct RenderState {
    current_screen: Screen,
    trades: Vec<TradeRow>,
    selected_trade: usize,
    risk_metrics: RiskMetrics,
    exposure_series: ExposureTimeSeries,
}

/// TUI Application state
pub struct TuiApp {
    /// Current screen
    current_screen: Screen,
    /// Trade list
    trades: Vec<TradeRow>,
    /// Selected trade index
    selected_trade: usize,
    /// Risk metrics
    risk_metrics: RiskMetrics,
    /// Exposure time series for charting
    exposure_series: ExposureTimeSeries,
    /// Exit flag
    should_quit: bool,
    /// API client
    #[allow(dead_code)]
    api_client: ApiClient,
    /// Terminal
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            current_screen: Screen::Dashboard,
            trades: Self::sample_trades(),
            selected_trade: 0,
            risk_metrics: Self::sample_risk_metrics(),
            exposure_series: ExposureTimeSeries::default(),
            should_quit: false,
            api_client: ApiClient::new("http://localhost:8080".to_string()),
            terminal,
        })
    }

    /// Generate sample trades for demo
    fn sample_trades() -> Vec<TradeRow> {
        vec![
            TradeRow {
                id: "T001".to_string(),
                instrument: "AAPL Call 200".to_string(),
                notional: 1_000_000.0,
                pv: 125_000.0,
                delta: 0.65,
                gamma: 0.02,
                vega: 0.15,
            },
            TradeRow {
                id: "T002".to_string(),
                instrument: "USD/JPY Forward".to_string(),
                notional: 5_000_000.0,
                pv: -45_000.0,
                delta: 0.98,
                gamma: 0.0,
                vega: 0.0,
            },
            TradeRow {
                id: "T003".to_string(),
                instrument: "5Y IRS Pay".to_string(),
                notional: 10_000_000.0,
                pv: 250_000.0,
                delta: 4.5,
                gamma: 0.0,
                vega: 0.0,
            },
        ]
    }

    /// Generate sample risk metrics for demo
    fn sample_risk_metrics() -> RiskMetrics {
        RiskMetrics {
            total_pv: 330_000.0,
            cva: -15_000.0,
            dva: 5_000.0,
            fva: -8_000.0,
            ee: 500_000.0,
            epe: 450_000.0,
            pfe: 800_000.0,
        }
    }

    /// Get a snapshot of the render state
    fn render_state(&self) -> RenderState {
        RenderState {
            current_screen: self.current_screen,
            trades: self.trades.clone(),
            selected_trade: self.selected_trade,
            risk_metrics: self.risk_metrics.clone(),
            exposure_series: self.exposure_series.clone(),
        }
    }

    /// Run the TUI event loop
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Take a snapshot of the state for rendering
            let state = self.render_state();

            // Draw the current screen
            self.terminal.draw(|frame| {
                Self::draw(frame, &state);
            })?;

            // Handle events with timeout for async refresh
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => self.current_screen = Screen::Dashboard,
            KeyCode::Char('2') => self.current_screen = Screen::Portfolio,
            KeyCode::Char('3') => self.current_screen = Screen::Risk,
            KeyCode::Char('4') => self.current_screen = Screen::TradeBlotter,
            KeyCode::Char('5') => self.current_screen = Screen::Chart,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_trade > 0 {
                    self.selected_trade -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_trade < self.trades.len().saturating_sub(1) {
                    self.selected_trade += 1;
                }
            }
            _ => {}
        }
    }

    /// Draw the current screen
    fn draw(frame: &mut Frame, state: &RenderState) {
        let area = frame.size();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Draw header
        Self::draw_header(frame, chunks[0], state.current_screen);

        // Draw content based on current screen
        match state.current_screen {
            Screen::Dashboard => screens::draw_dashboard(frame, chunks[1], &state.risk_metrics),
            Screen::Portfolio => {
                screens::draw_portfolio(frame, chunks[1], &state.trades, state.selected_trade)
            }
            Screen::Risk => screens::draw_risk(frame, chunks[1], &state.risk_metrics),
            Screen::TradeBlotter => {
                let trade = state.trades.get(state.selected_trade);
                screens::draw_trade_blotter(frame, chunks[1], trade);
            }
            Screen::Chart => screens::draw_exposure_chart(frame, chunks[1], &state.exposure_series),
        }

        // Draw footer
        Self::draw_footer(frame, chunks[2]);
    }

    /// Draw header
    fn draw_header(frame: &mut Frame, area: Rect, screen: Screen) {
        let title = format!(" FrictionalBank - {} ", screen.title());
        let header = Paragraph::new(title)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, area);
    }

    /// Draw footer with keybindings
    fn draw_footer(frame: &mut Frame, area: Rect) {
        let footer_text =
            " [1]Dashboard [2]Portfolio [3]Risk [4]Blotter [5]Chart | [Up/Down]Navigate | [q]Quit ";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, area);
    }

    /// Refresh data from API
    #[allow(dead_code)]
    pub async fn refresh_data(&mut self) -> Result<()> {
        // TODO: Fetch data from service_gateway
        // let portfolio = self.api_client.get_portfolio().await?;
        // self.trades = portfolio.to_trade_rows();
        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_titles() {
        assert_eq!(Screen::Dashboard.title(), "Dashboard");
        assert_eq!(Screen::Portfolio.title(), "Portfolio");
    }

    #[test]
    fn test_chart_screen_exists() {
        // Test that Chart screen has correct title
        assert_eq!(Screen::Chart.title(), "Exposure Chart");
    }

    #[test]
    fn test_exposure_time_series_default() {
        // Test that exposure time series has default data points
        let series = ExposureTimeSeries::default();
        assert!(!series.data_points.is_empty());
        assert!(series.data_points.len() >= 10);
    }

    #[test]
    fn test_exposure_time_series_to_chart_data() {
        // Test conversion to chart data format
        let series = ExposureTimeSeries::default();
        let ee_data = series.ee_data();
        let pfe_data = series.pfe_data();

        assert_eq!(ee_data.len(), series.data_points.len());
        assert_eq!(pfe_data.len(), series.data_points.len());
    }
}

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
    /// IRS AAD Demo screen (Task 6.2)
    IrsAadDemo,
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
            Self::IrsAadDemo => "IRS AAD Demo",
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
        self.data_points.iter().map(|p| (p.time, p.ee)).collect()
    }

    /// Convert to chart data format for Expected Positive Exposure
    pub fn epe_data(&self) -> Vec<(f64, f64)> {
        self.data_points.iter().map(|p| (p.time, p.epe)).collect()
    }

    /// Convert to chart data format for Potential Future Exposure
    pub fn pfe_data(&self) -> Vec<(f64, f64)> {
        self.data_points.iter().map(|p| (p.time, p.pfe)).collect()
    }

    /// Convert to chart data format for Expected Negative Exposure
    pub fn ene_data(&self) -> Vec<(f64, f64)> {
        self.data_points.iter().map(|p| (p.time, p.ene)).collect()
    }

    /// Get min/max values for Y axis bounds
    pub fn y_bounds(&self) -> [f64; 2] {
        let min = self
            .data_points
            .iter()
            .map(|p| p.ene)
            .fold(f64::MAX, f64::min);
        let max = self
            .data_points
            .iter()
            .map(|p| p.pfe)
            .fold(f64::MIN, f64::max);
        [min * 1.1, max * 1.1]
    }

    /// Get max time for X axis bounds
    pub fn x_bounds(&self) -> [f64; 2] {
        let max_time = self
            .data_points
            .iter()
            .map(|p| p.time)
            .fold(0.0_f64, f64::max);
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

// =============================================================================
// Task 6.2: IRS AAD Demo Data Structures
// =============================================================================

/// IRS AAD Demo state (Task 6.2)
///
/// Holds all data needed for the IRS AAD Demo screen, including
/// input parameters, calculation results, and benchmark data.
#[derive(Debug, Clone)]
pub struct IrsAadDemoState {
    /// Current IRS parameters
    pub params: IrsAadParams,
    /// Latest calculation result
    pub result: Option<IrsAadResult>,
    /// Latest benchmark result
    pub benchmark: Option<IrsAadBenchmark>,
    /// Currently selected input field (for editing)
    pub selected_field: usize,
    /// Whether the demo is currently calculating
    pub is_calculating: bool,
    /// Error message if any
    pub error_message: Option<String>,
    /// Selected calculation mode (0=Bump, 1=AAD, 2=Both)
    pub calc_mode: usize,
}

impl Default for IrsAadDemoState {
    fn default() -> Self {
        Self {
            params: IrsAadParams::default(),
            result: None,
            benchmark: None,
            selected_field: 0,
            is_calculating: false,
            error_message: None,
            calc_mode: 2, // Default to "Both" mode
        }
    }
}

/// IRS input parameters for the demo (Task 6.2)
#[derive(Debug, Clone)]
pub struct IrsAadParams {
    /// Notional amount
    pub notional: f64,
    /// Fixed rate (annualised)
    pub fixed_rate: f64,
    /// Tenor in years
    pub tenor_years: u32,
    /// Number of tenor points for delta calculation
    pub num_tenors: usize,
}

impl Default for IrsAadParams {
    fn default() -> Self {
        Self {
            notional: 1_000_000.0,
            fixed_rate: 0.03,
            tenor_years: 5,
            num_tenors: 8,
        }
    }
}

/// IRS calculation result (Task 6.2)
#[derive(Debug, Clone)]
pub struct IrsAadResult {
    /// Net Present Value
    pub npv: f64,
    /// DV01 (1bp parallel shift sensitivity)
    pub dv01: f64,
    /// Tenor points
    pub tenors: Vec<f64>,
    /// Delta values at each tenor
    pub deltas: Vec<f64>,
    /// Computation time in nanoseconds
    pub compute_time_ns: u64,
    /// Mode used for calculation
    pub mode: String,
}

impl Default for IrsAadResult {
    fn default() -> Self {
        Self {
            npv: 0.0,
            dv01: 0.0,
            tenors: Vec::new(),
            deltas: Vec::new(),
            compute_time_ns: 0,
            mode: "Bump".to_string(),
        }
    }
}

/// IRS AAD vs Bump benchmark result (Task 6.2)
#[derive(Debug, Clone)]
pub struct IrsAadBenchmark {
    /// AAD timing stats
    pub aad_mean_ns: f64,
    pub aad_std_ns: f64,
    /// Bump timing stats
    pub bump_mean_ns: f64,
    pub bump_std_ns: f64,
    /// Speedup ratio
    pub speedup_ratio: f64,
    /// Number of tenors used
    pub tenor_count: usize,
    /// Accuracy check results (relative errors)
    pub accuracy_errors: Vec<f64>,
}

impl Default for IrsAadBenchmark {
    fn default() -> Self {
        Self {
            aad_mean_ns: 0.0,
            aad_std_ns: 0.0,
            bump_mean_ns: 0.0,
            bump_std_ns: 0.0,
            speedup_ratio: 1.0,
            tenor_count: 0,
            accuracy_errors: Vec::new(),
        }
    }
}

/// Rendering state snapshot
struct RenderState {
    current_screen: Screen,
    trades: Vec<TradeRow>,
    selected_trade: usize,
    risk_metrics: RiskMetrics,
    exposure_series: ExposureTimeSeries,
    /// IRS AAD Demo state (Task 6.2)
    irs_aad_state: IrsAadDemoState,
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
    /// IRS AAD Demo state (Task 6.2)
    irs_aad_state: IrsAadDemoState,
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
            irs_aad_state: IrsAadDemoState::default(),
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
            irs_aad_state: self.irs_aad_state.clone(),
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
            KeyCode::Char('6') => self.current_screen = Screen::IrsAadDemo,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.current_screen == Screen::IrsAadDemo {
                    // Navigate IRS AAD Demo fields (Task 6.2)
                    if self.irs_aad_state.selected_field > 0 {
                        self.irs_aad_state.selected_field -= 1;
                    }
                } else if self.selected_trade > 0 {
                    self.selected_trade -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.current_screen == Screen::IrsAadDemo {
                    // Navigate IRS AAD Demo fields (Task 6.2)
                    if self.irs_aad_state.selected_field < 3 {
                        self.irs_aad_state.selected_field += 1;
                    }
                } else if self.selected_trade < self.trades.len().saturating_sub(1) {
                    self.selected_trade += 1;
                }
            }
            // IRS AAD Demo specific keys (Task 6.2)
            KeyCode::Tab => {
                if self.current_screen == Screen::IrsAadDemo {
                    // Cycle calculation mode: Bump -> AAD -> Both
                    self.irs_aad_state.calc_mode = (self.irs_aad_state.calc_mode + 1) % 3;
                }
            }
            KeyCode::Enter => {
                if self.current_screen == Screen::IrsAadDemo {
                    // Trigger calculation (mark as calculating)
                    self.irs_aad_state.is_calculating = true;
                    // Note: Actual calculation would be done in async context
                    self.trigger_irs_calculation();
                }
            }
            KeyCode::Left => {
                if self.current_screen == Screen::IrsAadDemo {
                    self.adjust_irs_param(-1);
                }
            }
            KeyCode::Right => {
                if self.current_screen == Screen::IrsAadDemo {
                    self.adjust_irs_param(1);
                }
            }
            _ => {}
        }
    }

    /// Adjust IRS parameter based on selected field (Task 6.2)
    fn adjust_irs_param(&mut self, direction: i32) {
        match self.irs_aad_state.selected_field {
            0 => {
                // Notional: adjust by 100,000
                let delta = 100_000.0 * direction as f64;
                self.irs_aad_state.params.notional =
                    (self.irs_aad_state.params.notional + delta).max(100_000.0);
            }
            1 => {
                // Fixed rate: adjust by 0.25%
                let delta = 0.0025 * direction as f64;
                self.irs_aad_state.params.fixed_rate =
                    (self.irs_aad_state.params.fixed_rate + delta).clamp(0.001, 0.2);
            }
            2 => {
                // Tenor years: adjust by 1
                self.irs_aad_state.params.tenor_years =
                    ((self.irs_aad_state.params.tenor_years as i32 + direction).max(1) as u32)
                        .min(30);
            }
            3 => {
                // Num tenors: adjust by 1
                self.irs_aad_state.params.num_tenors =
                    ((self.irs_aad_state.params.num_tenors as i32 + direction).max(2) as usize)
                        .min(20);
            }
            _ => {}
        }
    }

    /// Trigger IRS calculation with demo data (Task 6.2)
    fn trigger_irs_calculation(&mut self) {
        // Generate demo result (in production, this would call the actual workflow)
        let params = &self.irs_aad_state.params;

        // Demo: Generate realistic-looking results
        let tenors: Vec<f64> = (0..params.num_tenors)
            .map(|i| (i + 1) as f64 * params.tenor_years as f64 / params.num_tenors as f64)
            .collect();

        let deltas: Vec<f64> = tenors
            .iter()
            .map(|t| params.notional * 0.0001 * t.sqrt() * (-0.1 * t).exp())
            .collect();

        let npv = params.notional * (params.fixed_rate - 0.035) * params.tenor_years as f64 * 0.95;
        let dv01 = params.notional * params.tenor_years as f64 * 0.0001 * 0.98;

        let mode = match self.irs_aad_state.calc_mode {
            0 => "Bump",
            1 => "AAD",
            _ => "Both",
        };

        self.irs_aad_state.result = Some(IrsAadResult {
            npv,
            dv01,
            tenors: tenors.clone(),
            deltas: deltas.clone(),
            compute_time_ns: if self.irs_aad_state.calc_mode == 1 {
                15_000
            } else {
                300_000
            },
            mode: mode.to_string(),
        });

        // Generate benchmark result
        self.irs_aad_state.benchmark = Some(IrsAadBenchmark {
            aad_mean_ns: 15_000.0 + 500.0 * params.num_tenors as f64,
            aad_std_ns: 500.0,
            bump_mean_ns: 15_000.0 * params.num_tenors as f64,
            bump_std_ns: 2000.0,
            speedup_ratio: params.num_tenors as f64 * 0.95,
            tenor_count: params.num_tenors,
            accuracy_errors: tenors.iter().map(|_| 1e-8).collect(),
        });

        self.irs_aad_state.is_calculating = false;
        self.irs_aad_state.error_message = None;
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
            Screen::IrsAadDemo => {
                screens::draw_irs_aad_demo(frame, chunks[1], &state.irs_aad_state)
            }
        }

        // Draw footer
        Self::draw_footer(frame, chunks[2]);
    }

    /// Draw header
    fn draw_header(frame: &mut Frame, area: Rect, screen: Screen) {
        let title = format!(" FrictionalBank - {} ", screen.title());
        let header = Paragraph::new(title)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, area);
    }

    /// Draw footer with keybindings
    fn draw_footer(frame: &mut Frame, area: Rect) {
        let footer_text =
            " [1]Dashboard [2]Portfolio [3]Risk [4]Blotter [5]Chart [6]IRS AAD | [Up/Down]Nav | [q]Quit ";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, area);
    }

    /// Refresh data from API
    #[allow(dead_code)]
    pub async fn refresh_data(&mut self) -> Result<()> {
        // Stub: service_gateway API integration pending.
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

    // =========================================================================
    // Task 6.2: IRS AAD Demo Screen Tests
    // =========================================================================

    mod irs_aad_demo_tests {
        use super::*;

        #[test]
        fn test_irs_aad_screen_title() {
            assert_eq!(Screen::IrsAadDemo.title(), "IRS AAD Demo");
        }

        #[test]
        fn test_irs_aad_demo_state_default() {
            let state = IrsAadDemoState::default();
            assert!((state.params.notional - 1_000_000.0).abs() < 1e-10);
            assert!((state.params.fixed_rate - 0.03).abs() < 1e-10);
            assert_eq!(state.params.tenor_years, 5);
            assert_eq!(state.params.num_tenors, 8);
            assert_eq!(state.calc_mode, 2); // Default to "Both"
            assert!(!state.is_calculating);
            assert!(state.result.is_none());
            assert!(state.benchmark.is_none());
        }

        #[test]
        fn test_irs_aad_params_default() {
            let params = IrsAadParams::default();
            assert!((params.notional - 1_000_000.0).abs() < 1e-10);
            assert!((params.fixed_rate - 0.03).abs() < 1e-10);
            assert_eq!(params.tenor_years, 5);
            assert_eq!(params.num_tenors, 8);
        }

        #[test]
        fn test_irs_aad_result_default() {
            let result = IrsAadResult::default();
            assert!((result.npv - 0.0).abs() < 1e-10);
            assert!((result.dv01 - 0.0).abs() < 1e-10);
            assert!(result.tenors.is_empty());
            assert!(result.deltas.is_empty());
            assert_eq!(result.compute_time_ns, 0);
            assert_eq!(result.mode, "Bump");
        }

        #[test]
        fn test_irs_aad_benchmark_default() {
            let benchmark = IrsAadBenchmark::default();
            assert!((benchmark.aad_mean_ns - 0.0).abs() < 1e-10);
            assert!((benchmark.bump_mean_ns - 0.0).abs() < 1e-10);
            assert!((benchmark.speedup_ratio - 1.0).abs() < 1e-10);
            assert_eq!(benchmark.tenor_count, 0);
            assert!(benchmark.accuracy_errors.is_empty());
        }

        #[test]
        fn test_irs_aad_state_field_navigation() {
            let mut state = IrsAadDemoState::default();
            assert_eq!(state.selected_field, 0);

            state.selected_field = 1;
            assert_eq!(state.selected_field, 1);

            state.selected_field = 3;
            assert_eq!(state.selected_field, 3);
        }

        #[test]
        fn test_irs_aad_calc_mode_cycle() {
            let mut state = IrsAadDemoState::default();
            assert_eq!(state.calc_mode, 2);

            state.calc_mode = (state.calc_mode + 1) % 3;
            assert_eq!(state.calc_mode, 0); // Bump

            state.calc_mode = (state.calc_mode + 1) % 3;
            assert_eq!(state.calc_mode, 1); // AAD

            state.calc_mode = (state.calc_mode + 1) % 3;
            assert_eq!(state.calc_mode, 2); // Both
        }
    }
}

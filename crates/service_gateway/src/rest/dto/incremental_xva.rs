//! DTOs for the Incremental XVA engine.
//!
//! Request and response types for the `/api/incremental-xva/` endpoints.

use serde::{Deserialize, Serialize};
use validator::Validate;

// ─── Swap Definition ────────────────────────────────────────────────────────

/// Definition of a vanilla interest rate swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapDefinitionDto {
    /// Unique trade identifier.
    pub trade_id: String,
    /// Notional amount.
    pub notional: f64,
    /// Contractual fixed rate (decimal, e.g. 0.03 for 3%).
    pub fixed_rate: f64,
    /// Swap tenor in years.
    pub tenor_years: f64,
    /// Payment frequency: "quarterly", "semi-annual", "annual".
    #[serde(default = "default_payment_freq")]
    pub payment_frequency: String,
    /// True for payer swap (pay fixed, receive floating).
    pub is_payer: bool,
}

fn default_payment_freq() -> String { "semi-annual".to_string() }

// ─── Exotic Definition ──────────────────────────────────────────────────────

/// Definition of an exotic product for MFM grid-cache construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExoticDefinitionDto {
    /// Unique trade identifier.
    pub trade_id: String,
    /// Product type: "bermudan", "tarn", "cif".
    pub product_type: String,
    /// Notional amount.
    pub notional: f64,

    // ── MFM calibration parameters ──
    /// Mean reversion for the MFM Gaussian process.
    pub mfm_mean_reversion: f64,
    /// Volatility for the MFM Gaussian process.
    pub mfm_volatility: f64,
    /// Number of grid points (odd, >= 3). Default: 41.
    pub mfm_grid_points: Option<usize>,
    /// Number of standard deviations for grid extent. Default: 5.0.
    pub mfm_num_std_devs: Option<f64>,

    // ── Schedule ──
    /// Exercise / coupon observation times (year fractions from today).
    pub exercise_times: Vec<f64>,
    /// Swap tenors corresponding to each exercise time.
    pub swap_tenors: Vec<f64>,
    /// Payment frequency in years. Default: 0.5 (semi-annual).
    pub payment_frequency: Option<f64>,

    // ── Funding / coupon curve ──
    /// Flat funding rate for discounting. Default: 0.03.
    pub funding_rate: Option<f64>,
    /// Flat coupon rate for projection. Default: 0.03.
    pub coupon_rate: Option<f64>,
    /// Flat swaption normal vol (bps). Default: 50.0.
    pub flat_vol_bps: Option<f64>,

    // ── Product-specific parameters ──
    /// Fixed rate (for Bermudan, CIF).
    pub fixed_rate: Option<f64>,
    /// Callable flag (for Bermudan). Default: true.
    pub is_callable: Option<bool>,
    /// TARN target amount.
    pub tarn_target: Option<f64>,
    /// TARN coupon grid points. Default: 20.
    pub tarn_coupon_grid_points: Option<usize>,
    /// CIF leverage.
    pub leverage: Option<f64>,
    /// CIF floor rate.
    pub floor_rate: Option<f64>,
    /// CIF cap rate.
    pub cap_rate: Option<f64>,
}

// ─── Inflation Swap Definition ──────────────────────────────────────────────

/// Definition of a zero-coupon inflation swap for JY model pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InflationSwapDefinitionDto {
    /// Unique trade identifier.
    pub trade_id: String,
    /// Notional amount.
    pub notional: f64,
    /// Contractual fixed rate (annual, e.g. 0.02 for 2%).
    pub fixed_rate: f64,
    /// Swap maturity in years.
    pub maturity_years: f64,
    /// Base inflation index level I(0). Default: 100.0.
    #[serde(default = "default_base_index")]
    pub base_index: f64,
}

fn default_base_index() -> f64 { 100.0 }

// ─── Incremental Trade ──────────────────────────────────────────────────────

/// The incremental trade to be added to the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum IncrementalTradeDto {
    /// A vanilla swap.
    #[serde(rename = "swap")]
    Swap(SwapDefinitionDto),
    /// An exotic product.
    #[serde(rename = "exotic")]
    Exotic(ExoticDefinitionDto),
    /// A zero-coupon inflation swap.
    #[serde(rename = "inflationSwap")]
    InflationSwap(InflationSwapDefinitionDto),
}

// ─── Request ────────────────────────────────────────────────────────────────

/// Request body for `POST /api/incremental-xva/run`.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalXvaRequest {
    // ── Monte Carlo configuration ──
    /// Number of simulation paths (100–1,000,000). Default: 10,000.
    #[validate(range(min = 100, max = 1_000_000))]
    pub n_paths: Option<usize>,
    /// Simulation horizon in years. Default: 10.0.
    pub horizon_years: Option<f64>,
    /// Time step: "monthly", "quarterly", "semi-annual". Default: "quarterly".
    pub time_step: Option<String>,
    /// Random seed for reproducibility.
    pub seed: Option<u64>,
    /// Use antithetic variates. Default: true.
    pub antithetic: Option<bool>,
    /// Bilateral CVA/DVA. Default: true.
    pub bilateral: Option<bool>,
    /// Compute FVA. Default: true.
    pub compute_fva: Option<bool>,

    // ── HW1F model parameters ──
    /// Mean reversion speed (a > 0).
    pub hw_mean_reversion: f64,
    /// Volatility of short rate (σ > 0).
    pub hw_volatility: f64,
    /// Initial short rate r(0).
    pub hw_initial_rate: f64,

    // ── Model coupling ──
    /// Coupling method: "swap_rate" (Approach A, default) or "zscore" (Approach
    /// B).
    #[serde(default = "default_coupling_method")]
    pub coupling_method: String,
    /// Benchmark swap tenor for Approach A (years). Default: 10.0.
    pub coupling_swap_tenor: Option<f64>,
    /// Payment frequency for benchmark swap in Approach A (years). Default:
    /// 0.5.
    pub coupling_payment_freq: Option<f64>,

    // ── Counterparty credit ──
    /// Counterparty hazard rate (decimal).
    pub hazard_rate: f64,
    /// Counterparty loss given default (0–1).
    pub lgd: f64,
    /// Own hazard rate. Default: 0.01.
    pub own_hazard_rate: Option<f64>,
    /// Own LGD. Default: 0.4.
    pub own_lgd: Option<f64>,
    /// Funding spread (borrow, decimal). Default: 0.005.
    pub funding_spread: Option<f64>,

    // ── JY inflation model (optional, required when inflation trades present) ──
    /// Real rate mean reversion speed.
    pub jy_real_mean_reversion: Option<f64>,
    /// Real rate volatility.
    pub jy_real_volatility: Option<f64>,
    /// Initial real short rate.
    pub jy_initial_real_rate: Option<f64>,
    /// Inflation index volatility.
    pub jy_inflation_volatility: Option<f64>,
    /// Initial inflation index level.
    pub jy_initial_index: Option<f64>,
    /// Correlation: nominal–real.
    pub jy_rho_nominal_real: Option<f64>,
    /// Correlation: nominal–inflation.
    pub jy_rho_nominal_inflation: Option<f64>,
    /// Correlation: real–inflation.
    pub jy_rho_real_inflation: Option<f64>,

    // ── Portfolio ──
    /// Base portfolio: vanilla swaps.
    #[serde(default)]
    pub base_swaps: Vec<SwapDefinitionDto>,
    /// Base portfolio: exotic trades.
    #[serde(default)]
    pub base_exotics: Vec<ExoticDefinitionDto>,
    /// Base portfolio: inflation swaps.
    #[serde(default)]
    pub base_inflation_swaps: Vec<InflationSwapDefinitionDto>,
    /// The incremental trade being evaluated.
    pub incremental_trade: IncrementalTradeDto,
}

fn default_coupling_method() -> String { "swap_rate".to_string() }

// ─── Response ───────────────────────────────────────────────────────────────

/// XVA metrics (CVA, DVA, FVA).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaMetricsDto {
    /// Unilateral CVA.
    pub ucva: f64,
    /// Bilateral CVA.
    pub bcva: f64,
    /// Unilateral DVA.
    pub udva: f64,
    /// Bilateral DVA.
    pub bdva: f64,
    /// Funding Cost Adjustment.
    pub fca: f64,
    /// Funding Benefit Adjustment.
    pub fba: f64,
    /// Total FVA = FCA - FBA.
    pub fva: f64,
    /// Total XVA = BCVA - BDVA + FVA.
    pub total: f64,
}

/// Response body for `POST /api/incremental-xva/run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalXvaResponse {
    /// Time grid used in simulation.
    pub time_grid: Vec<f64>,
    /// Actual number of paths simulated.
    pub n_paths: usize,

    /// XVA of the base portfolio (before incremental trade).
    pub base_xva: XvaMetricsDto,
    /// XVA of the full portfolio (after adding incremental trade).
    pub full_xva: XvaMetricsDto,
    /// Incremental XVA = full - base.
    pub incremental_xva: XvaMetricsDto,

    /// Base portfolio EPE profile.
    pub base_epe: Vec<f64>,
    /// Base portfolio ENE profile.
    pub base_ene: Vec<f64>,
    /// Full portfolio EPE profile.
    pub full_epe: Vec<f64>,
    /// Full portfolio ENE profile.
    pub full_ene: Vec<f64>,

    /// Coupling method used ("swap_rate" or "zscore").
    pub coupling_method: String,
    /// Wall-clock computation time in milliseconds.
    pub computation_time_ms: f64,
}

/// Default configuration response for `GET /api/incremental-xva/config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalXvaDefaultConfig {
    /// Default MC paths.
    pub n_paths: usize,
    /// Default horizon.
    pub horizon_years: f64,
    /// Default time step.
    pub time_step: String,
    /// Default antithetic flag.
    pub antithetic: bool,
    /// Default bilateral flag.
    pub bilateral: bool,
    /// Default compute_fva flag.
    pub compute_fva: bool,

    /// Default HW1F mean reversion.
    pub hw_mean_reversion: f64,
    /// Default HW1F volatility.
    pub hw_volatility: f64,
    /// Default HW1F initial rate.
    pub hw_initial_rate: f64,

    /// Default coupling method.
    pub coupling_method: String,

    /// Default counterparty credit.
    pub hazard_rate: f64,
    /// Default LGD.
    pub lgd: f64,

    // ── JY inflation defaults (optional) ──
    /// Default real rate mean reversion.
    pub jy_real_mean_reversion: Option<f64>,
    /// Default real rate volatility.
    pub jy_real_volatility: Option<f64>,
    /// Default initial real rate.
    pub jy_initial_real_rate: Option<f64>,
    /// Default inflation index volatility.
    pub jy_inflation_volatility: Option<f64>,
    /// Default initial index level.
    pub jy_initial_index: Option<f64>,
    /// Default nominal–real correlation.
    pub jy_rho_nominal_real: Option<f64>,
    /// Default nominal–inflation correlation.
    pub jy_rho_nominal_inflation: Option<f64>,
    /// Default real–inflation correlation.
    pub jy_rho_real_inflation: Option<f64>,

    /// Pre-populated base swaps.
    pub base_swaps: Vec<SwapDefinitionDto>,
    /// Pre-populated base exotics.
    pub base_exotics: Vec<ExoticDefinitionDto>,
    /// Pre-populated base inflation swaps.
    pub base_inflation_swaps: Vec<InflationSwapDefinitionDto>,
    /// Pre-populated incremental trade.
    pub incremental_trade: IncrementalTradeDto,
}

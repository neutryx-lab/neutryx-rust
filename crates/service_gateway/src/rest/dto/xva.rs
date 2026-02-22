//! XVA Engine DTOs for demo GUI.

use serde::{Deserialize, Serialize};
use validator::Validate;

// ── Request DTOs ──

/// Request to run XVA simulation.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct XvaSimulationRequest {
    /// Number of Monte Carlo paths (default: 10000).
    #[validate(range(min = 100, max = 1000000))]
    pub n_paths: Option<usize>,
    /// Time horizon in years (default: 5.0).
    pub horizon_years: Option<f64>,
    /// Time step frequency: "quarterly", "monthly", "semi-annual" (default: "quarterly").
    pub time_step: Option<String>,
    /// Random seed for reproducibility.
    pub seed: Option<u64>,
    /// Use antithetic variates (default: true).
    pub antithetic: Option<bool>,
    /// PFE percentiles (default: [0.95, 0.975, 0.99]).
    pub pfe_percentiles: Option<Vec<f64>>,
    /// Compute bilateral CVA/DVA (default: true).
    pub bilateral: Option<bool>,
    /// Compute FVA (default: true).
    pub compute_fva: Option<bool>,
    /// Counterparty ID to simulate (if None, uses demo default).
    pub counterparty_id: Option<String>,
}

/// Request to compute bilateral XVA for given exposure profiles.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct XvaBilateralRequest {
    /// Expected Positive Exposure profile.
    pub epe: Vec<f64>,
    /// Expected Negative Exposure profile.
    pub ene: Vec<f64>,
    /// Time grid in year fractions.
    pub time_grid: Vec<f64>,
    /// Counterparty hazard rate.
    #[validate(range(min = 0.0, max = 1.0))]
    pub hazard_rate: f64,
    /// Counterparty LGD.
    #[validate(range(min = 0.0, max = 1.0))]
    pub lgd: f64,
    /// Own hazard rate.
    #[validate(range(min = 0.0, max = 1.0))]
    pub own_hazard_rate: f64,
    /// Own LGD.
    #[validate(range(min = 0.0, max = 1.0))]
    pub own_lgd: f64,
    /// Funding spread for FVA.
    pub funding_spread: Option<f64>,
    /// Cross-currency basis spread.
    pub xccy_basis: Option<f64>,
}

// ── Response DTOs ──

/// Full XVA simulation result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaSimulationResponse {
    /// Simulation configuration summary.
    pub config: XvaConfigSummary,
    /// Time grid used.
    pub time_grid: Vec<f64>,
    /// Number of paths simulated.
    pub n_paths: usize,
    /// Netting set results.
    pub netting_sets: Vec<NettingSetResult>,
    /// Counterparty-level XVA results.
    pub counterparty_results: Vec<CounterpartyXvaResult>,
    /// Portfolio hierarchy summary.
    pub hierarchy: HierarchySummary,
    /// Computation time in milliseconds.
    pub computation_time_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaConfigSummary {
    pub n_paths: usize,
    pub time_points: usize,
    pub horizon_years: f64,
    pub antithetic: bool,
    pub bilateral: bool,
    pub compute_fva: bool,
    pub pfe_percentiles: Vec<f64>,
}

/// Exposure profile result for a single netting set.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NettingSetResult {
    pub netting_set_id: String,
    pub epe: Vec<f64>,
    pub ene: Vec<f64>,
    pub ecb: Vec<f64>,
    pub pfe: Vec<PfeProfile>,
    pub peak_epe: f64,
    pub peak_ene: f64,
    pub avg_epe: f64,
    pub avg_ene: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PfeProfile {
    pub percentile: f64,
    pub label: String,
    pub values: Vec<f64>,
    pub peak: f64,
}

/// Counterparty-level XVA results.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterpartyXvaResult {
    pub counterparty_id: String,
    pub credit_rating: String,
    pub hazard_rate: f64,
    pub lgd: f64,
    /// Unilateral CVA.
    pub ucva: f64,
    /// Unilateral DVA.
    pub udva: f64,
    /// Bilateral CVA (first-to-default).
    pub bcva: f64,
    /// Bilateral DVA (first-to-default).
    pub bdva: f64,
    /// Funding Cost Adjustment.
    pub fca: f64,
    /// Funding Benefit Adjustment.
    pub fba: f64,
    /// Net FVA.
    pub fva: f64,
    /// Net total XVA = BCVA - BDVA + FVA.
    pub total_xva: f64,
    /// Number of netting sets.
    pub netting_set_count: usize,
    /// Number of trades.
    pub trade_count: usize,
}

/// Portfolio hierarchy summary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchySummary {
    pub counterparties: Vec<HierarchyCounterparty>,
    pub total_counterparties: usize,
    pub total_netting_sets: usize,
    pub total_trades: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyCounterparty {
    pub id: String,
    pub credit_rating: String,
    pub isda_agreements: Vec<HierarchyIsda>,
    pub no_doc_trade_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyIsda {
    pub netting_set_id: String,
    pub vm_csas: Vec<HierarchyVmCsa>,
    pub non_csa_trade_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyVmCsa {
    pub csa_id: String,
    pub threshold_self: f64,
    pub threshold_ctpy: f64,
    pub mta_self: f64,
    pub mta_ctpy: f64,
    pub mpor_days: u32,
    pub trade_count: usize,
}

/// XVA bilateral calculation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaBilateralResponse {
    pub ucva: f64,
    pub udva: f64,
    pub bcva: f64,
    pub bdva: f64,
    pub fca: f64,
    pub fba: f64,
    pub fva: f64,
    pub total_xva: f64,
    pub computation_time_ms: f64,
}

/// Default XVA configuration response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaDefaultConfigResponse {
    pub n_paths: usize,
    pub horizon_years: f64,
    pub time_step: String,
    pub antithetic: bool,
    pub bilateral: bool,
    pub compute_fva: bool,
    pub pfe_percentiles: Vec<f64>,
    pub counterparties: Vec<DemoCounterparty>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoCounterparty {
    pub id: String,
    pub name: String,
    pub credit_rating: String,
    pub hazard_rate: f64,
    pub lgd: f64,
    pub netting_sets: Vec<DemoNettingSet>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoNettingSet {
    pub id: String,
    pub has_csa: bool,
    pub trade_count: usize,
    pub trade_types: Vec<String>,
}

/// CSV export response body (raw text).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XvaCsvExportResponse {
    pub csv_data: String,
    pub netting_set_id: String,
    pub row_count: usize,
}

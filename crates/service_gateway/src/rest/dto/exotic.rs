//! DTOs for exotic product pricing endpoints.

use serde::{Deserialize, Serialize};

/// Request for pricing exotic products.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "productType", rename_all = "snake_case")]
pub enum ExoticProductRequest {
    /// Target Accrual Redemption Forward.
    Tarf(TarfRequest),
    /// Autocallable structured note.
    Autocallable(AutocallableRequest),
}

/// TARF (Target Accrual Redemption Forward) request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TarfRequest {
    /// Currency pair (e.g. "EURUSD").
    pub currency_pair: String,
    /// Notional per fixing.
    pub notional_per_fixing: f64,
    /// Forward strike.
    pub strike: f64,
    /// Maximum accumulated profit before early termination.
    pub target_profit: f64,
    /// Leverage ratio on the downside (e.g. 2.0 for 2x).
    pub leverage: f64,
    /// Maturity in years (used to generate fixing schedule).
    pub maturity: f64,
    /// Number of fixings over the maturity period.
    #[serde(default = "default_num_fixings")]
    pub num_fixings: u32,
    /// Explicit fixing dates (year fractions). Auto-generated from
    /// maturity/num_fixings when absent.
    pub fixing_dates: Option<Vec<f64>>,
    /// Spot price.
    pub spot: f64,
    /// Domestic risk-free rate.
    pub domestic_rate: f64,
    /// Foreign risk-free rate.
    pub foreign_rate: f64,
    /// Implied volatility.
    pub volatility: f64,
    /// Number of Monte Carlo paths (optional).
    pub num_paths: Option<u32>,
}

fn default_num_fixings() -> u32 { 12 }

impl TarfRequest {
    /// Returns the fixing schedule, auto-generating evenly-spaced dates if not
    /// provided.
    pub fn effective_fixing_dates(&self) -> Vec<f64> {
        if let Some(dates) = &self.fixing_dates {
            return dates.clone();
        }
        let n = self.num_fixings.max(1) as usize;
        (1..=n)
            .map(|i| self.maturity * i as f64 / n as f64)
            .collect()
    }
}

/// Autocallable request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutocallableRequest {
    /// Underlying identifier (e.g. "SPX").
    pub underlying: String,
    /// Notional amount.
    pub notional: f64,
    /// Spot price.
    pub spot: f64,
    /// Autocall barrier level (absolute).
    pub autocall_barrier: f64,
    /// Coupon rate per observation period.
    pub coupon_rate: f64,
    /// Knock-in barrier for downside protection.
    pub ki_barrier: f64,
    /// Number of observations over the maturity period.
    #[serde(default = "default_num_observations")]
    pub num_observations: u32,
    /// Explicit observation dates (year fractions). Auto-generated when absent.
    pub observation_dates: Option<Vec<f64>>,
    /// Maturity in years.
    pub maturity: f64,
    /// Risk-free rate.
    pub rate: f64,
    /// Implied volatility.
    pub volatility: f64,
    /// Number of Monte Carlo paths (optional).
    pub num_paths: Option<u32>,
}

fn default_num_observations() -> u32 { 4 }

impl AutocallableRequest {
    /// Returns the observation schedule, auto-generating quarterly dates if not
    /// provided.
    pub fn effective_observation_dates(&self) -> Vec<f64> {
        if let Some(dates) = &self.observation_dates {
            return dates.clone();
        }
        let n = self.num_observations.max(1) as usize;
        (1..=n)
            .map(|i| self.maturity * i as f64 / n as f64)
            .collect()
    }
}

/// Response for exotic product pricing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExoticPricingResponse {
    /// Present value.
    pub price: f64,
    /// Currency or underlying identifier.
    pub currency: String,
    /// Product type identifier.
    pub product_type: String,
    /// Monte Carlo statistics (when applicable).
    pub mc_stats: Option<MonteCarloStats>,
    /// Calculation wall-clock time in milliseconds.
    pub calculation_time_ms: u64,
}

/// Monte Carlo simulation statistics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloStats {
    /// Number of simulated paths.
    pub num_paths: u32,
    /// Standard error of the estimate.
    pub std_error: f64,
    /// 95% confidence interval [lower, upper].
    pub confidence_95: [f64; 2],
}

/// Available exotic product definition for dynamic form rendering.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExoticProductDef {
    /// Product type identifier (e.g. "tarf", "autocallable").
    pub product_type: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Product description.
    pub description: String,
    /// Parameter definitions for UI form generation.
    pub parameters: Vec<ParameterDef>,
}

/// Parameter definition for dynamic form rendering.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDef {
    /// Parameter name (camelCase, matches JSON field).
    pub name: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Field type ("string", "number", etc.).
    pub field_type: String,
    /// Whether the parameter is required.
    pub required: bool,
    /// Default value for the parameter.
    pub default_value: Option<serde_json::Value>,
    /// Optional description / help text.
    pub description: Option<String>,
}

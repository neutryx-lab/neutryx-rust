//! Model-related DTOs for stochastic model configuration and pricing
//!
//! Request/Response types for `ModelService` endpoints.

use serde::{Deserialize, Serialize};

// ============================================================================
// Model Parameter DTOs
// ============================================================================

/// GBM (Geometric Brownian Motion) parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GbmParamsDto {
    /// Drift rate (mu)
    pub drift: f64,
    /// Volatility (sigma)
    pub volatility: f64,
}

/// Heston stochastic volatility parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HestonParamsDto {
    /// Initial variance (v0)
    pub v0: f64,
    /// Mean reversion speed (kappa)
    pub kappa: f64,
    /// Long-term variance (theta)
    pub theta: f64,
    /// Volatility of volatility (sigma)
    pub sigma: f64,
    /// Correlation between spot and variance (rho)
    pub rho: f64,
}

/// Hull-White interest rate model parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HullWhiteParamsDto {
    /// Mean reversion speed (a)
    pub mean_reversion: f64,
    /// Volatility (sigma)
    pub volatility: f64,
}

/// CIR (Cox-Ingersoll-Ross) model parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CirParamsDto {
    /// Mean reversion speed (kappa)
    pub kappa: f64,
    /// Long-term mean (theta)
    pub theta: f64,
    /// Volatility (sigma)
    pub sigma: f64,
}

/// SABR model parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SabrParamsDto {
    /// Initial volatility (alpha)
    pub alpha: f64,
    /// Beta exponent
    pub beta: f64,
    /// Correlation (rho)
    pub rho: f64,
    /// Vol-of-vol (nu)
    pub nu: f64,
}

// ============================================================================
// Model CRUD DTOs
// ============================================================================

/// Request to create a new model
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "model_type", rename_all = "snake_case")]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub enum CreateModelRequest {
    /// Geometric Brownian Motion
    Gbm {
        /// Model name
        #[serde(default)]
        name: Option<String>,
        /// GBM parameters
        params: GbmParamsDto,
    },
    /// Heston stochastic volatility
    Heston {
        /// Model name
        #[serde(default)]
        name: Option<String>,
        /// Heston parameters
        params: HestonParamsDto,
    },
    /// Hull-White interest rate
    HullWhite {
        /// Model name
        #[serde(default)]
        name: Option<String>,
        /// Hull-White parameters
        params: HullWhiteParamsDto,
    },
    /// Cox-Ingersoll-Ross
    Cir {
        /// Model name
        #[serde(default)]
        name: Option<String>,
        /// CIR parameters
        params: CirParamsDto,
    },
    /// SABR
    Sabr {
        /// Model name
        #[serde(default)]
        name: Option<String>,
        /// SABR parameters
        params: SabrParamsDto,
    },
}

/// Response for model creation
#[derive(Debug, Clone, Serialize)]
pub struct CreateModelResponse {
    /// Generated model ID
    pub model_id: String,
    /// Model type
    pub model_type: String,
    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Validation results
    pub validation: ModelValidationDto,
}

/// Model validation result
#[derive(Debug, Clone, Serialize)]
pub struct ModelValidationDto {
    /// Is the model valid?
    pub valid: bool,
    /// Validation warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Validation errors
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Response for model retrieval
#[derive(Debug, Clone, Serialize)]
pub struct GetModelResponse {
    /// Model ID
    pub model_id: String,
    /// Model type
    pub model_type: String,
    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Model parameters (as JSON object)
    pub params: serde_json::Value,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
}

// ============================================================================
// Model Pricing DTOs
// ============================================================================

/// Pricing method
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PricingMethodDto {
    /// Closed-form analytical solution (if available)
    #[default]
    Analytical,
    /// Monte Carlo simulation
    MonteCarlo,
    /// Tree method (binomial/trinomial)
    Tree,
}

/// Instrument type for pricing
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstrumentDto {
    /// Vanilla European option
    VanillaOption {
        /// Underlying spot price
        spot: f64,
        /// Strike price
        strike: f64,
        /// Time to maturity (years)
        maturity: f64,
        /// Is call option?
        is_call: bool,
    },
    /// Forward contract
    Forward {
        /// Underlying spot price
        spot: f64,
        /// Forward price
        forward_price: f64,
        /// Time to maturity (years)
        maturity: f64,
    },
    /// Asian option (arithmetic average)
    AsianOption {
        /// Underlying spot price
        spot: f64,
        /// Strike price
        strike: f64,
        /// Time to maturity (years)
        maturity: f64,
        /// Number of averaging periods
        num_periods: usize,
        /// Is call option?
        is_call: bool,
    },
    /// Barrier option
    BarrierOption {
        /// Underlying spot price
        spot: f64,
        /// Strike price
        strike: f64,
        /// Barrier level
        barrier: f64,
        /// Time to maturity (years)
        maturity: f64,
        /// Barrier type
        barrier_type: String,
        /// Is call option?
        is_call: bool,
    },
}

/// Request for model-based pricing
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization
pub struct ModelPricingRequest {
    /// Pricing method to use
    #[serde(default)]
    pub method: PricingMethodDto,
    /// Instrument to price
    pub instrument: InstrumentDto,
    /// Number of Monte Carlo paths (optional)
    #[serde(default)]
    pub num_paths: Option<usize>,
    /// Number of time steps (optional)
    #[serde(default)]
    pub num_steps: Option<usize>,
    /// Risk-free rate (optional, overrides model)
    #[serde(default)]
    pub risk_free_rate: Option<f64>,
}

/// Response for model-based pricing
#[derive(Debug, Clone, Serialize)]
pub struct ModelPricingResponse {
    /// Model ID used
    pub model_id: String,
    /// Model type
    pub model_type: String,
    /// Pricing method used
    pub method: PricingMethodDto,
    /// Calculated price
    pub price: f64,
    /// Price currency
    pub currency: String,
    /// Greeks (if calculated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeks: Option<PricingGreeksDto>,
    /// Number of paths used (for MC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_paths: Option<usize>,
    /// Standard error (for MC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub std_error: Option<f64>,
    /// Calculation time in milliseconds
    pub calculation_time_ms: f64,
}

/// Greeks from pricing
#[derive(Debug, Clone, Serialize)]
pub struct PricingGreeksDto {
    /// Delta
    pub delta: f64,
    /// Gamma
    pub gamma: f64,
    /// Vega
    pub vega: f64,
    /// Theta
    pub theta: f64,
    /// Rho
    pub rho: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_gbm_model_request() {
        let json = r#"{
            "model_type": "gbm",
            "name": "Test GBM",
            "params": {
                "drift": 0.05,
                "volatility": 0.2
            }
        }"#;
        let request: CreateModelRequest = serde_json::from_str(json).unwrap();
        match request {
            CreateModelRequest::Gbm { name, params } => {
                assert_eq!(name, Some("Test GBM".to_string()));
                assert!((params.drift - 0.05).abs() < f64::EPSILON);
                assert!((params.volatility - 0.2).abs() < f64::EPSILON);
            }
            _ => panic!("Expected GBM model"),
        }
    }

    #[test]
    fn test_create_heston_model_request() {
        let json = r#"{
            "model_type": "heston",
            "params": {
                "v0": 0.04,
                "kappa": 2.0,
                "theta": 0.04,
                "sigma": 0.3,
                "rho": -0.7
            }
        }"#;
        let request: CreateModelRequest = serde_json::from_str(json).unwrap();
        match request {
            CreateModelRequest::Heston { name, params } => {
                assert!(name.is_none());
                assert!((params.v0 - 0.04).abs() < f64::EPSILON);
                assert!((params.kappa - 2.0).abs() < f64::EPSILON);
                assert!((params.rho - -0.7).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Heston model"),
        }
    }

    #[test]
    fn test_instrument_vanilla_option() {
        let json = r#"{
            "type": "vanilla_option",
            "spot": 100.0,
            "strike": 105.0,
            "maturity": 1.0,
            "is_call": true
        }"#;
        let instrument: InstrumentDto = serde_json::from_str(json).unwrap();
        match instrument {
            InstrumentDto::VanillaOption {
                spot,
                strike,
                maturity,
                is_call,
            } => {
                assert!((spot - 100.0).abs() < f64::EPSILON);
                assert!((strike - 105.0).abs() < f64::EPSILON);
                assert!((maturity - 1.0).abs() < f64::EPSILON);
                assert!(is_call);
            }
            _ => panic!("Expected VanillaOption"),
        }
    }

}

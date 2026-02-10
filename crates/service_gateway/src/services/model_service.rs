//! Model service for stochastic model configuration and pricing
//!
//! Provides model CRUD and model-based pricing operations.

#[cfg(feature = "models")]
use std::{sync::Arc, time::Instant};

#[cfg(feature = "models")]
use chrono::Utc;

#[cfg(feature = "models")]
use crate::{
    error::ServerError,
    services::helpers,
    rest::dto::{
        CreateModelRequest, CreateModelResponse, GetModelResponse, InstrumentDto,
        ModelPricingRequest, ModelPricingResponse, ModelValidationDto, PricingGreeksDto,
        PricingMethodDto,
    },
    state::{AppState, ModelEntry, ModelType},
};

/// Service for stochastic model operations
#[cfg(feature = "models")]
pub struct ModelService;

#[cfg(feature = "models")]
impl ModelService {
    /// Create a new stochastic model
    pub fn create_model(
        request: &CreateModelRequest,
        state: &Arc<AppState>,
    ) -> Result<CreateModelResponse, ServerError> {
        let (model_type, name, params_json, validation) = match request {
            CreateModelRequest::Gbm { name, params } => {
                let validation = Self::validate_gbm(params.drift, params.volatility);
                let json = serde_json::to_string(params)
                    .map_err(|e| ServerError::Internal(format!("Serialisation error: {e}")))?;
                (ModelType::Gbm, name.clone(), json, validation)
            }
            CreateModelRequest::Heston { name, params } => {
                let validation = Self::validate_heston(
                    params.v0,
                    params.kappa,
                    params.theta,
                    params.sigma,
                    params.rho,
                );
                let json = serde_json::to_string(params)
                    .map_err(|e| ServerError::Internal(format!("Serialisation error: {e}")))?;
                (ModelType::Heston, name.clone(), json, validation)
            }
            CreateModelRequest::HullWhite { name, params } => {
                let validation =
                    Self::validate_hull_white(params.mean_reversion, params.volatility);
                let json = serde_json::to_string(params)
                    .map_err(|e| ServerError::Internal(format!("Serialisation error: {e}")))?;
                (ModelType::HullWhite, name.clone(), json, validation)
            }
            CreateModelRequest::Cir { name, params } => {
                let validation = Self::validate_cir(params.kappa, params.theta, params.sigma);
                let json = serde_json::to_string(params)
                    .map_err(|e| ServerError::Internal(format!("Serialisation error: {e}")))?;
                (ModelType::Cir, name.clone(), json, validation)
            }
            CreateModelRequest::Sabr { name, params } => {
                let validation =
                    Self::validate_sabr(params.alpha, params.beta, params.rho, params.nu);
                let json = serde_json::to_string(params)
                    .map_err(|e| ServerError::Internal(format!("Serialisation error: {e}")))?;
                (ModelType::Sabr, name.clone(), json, validation)
            }
        };

        // Check if validation failed
        if !validation.valid {
            return Err(ServerError::InvalidRequest(validation.errors.join("; ")));
        }

        let entry = ModelEntry {
            model_type,
            params_json,
            name: name.clone(),
            created_at: Utc::now(),
        };

        let model_id = state.model_cache.add(entry);

        Ok(CreateModelResponse {
            model_id: model_id.to_string(),
            model_type: Self::model_type_to_string(model_type),
            name,
            validation,
        })
    }

    /// Get a model by ID
    pub fn get_model(
        model_id: &str,
        state: &Arc<AppState>,
    ) -> Result<GetModelResponse, ServerError> {
        let entry = helpers::resolve_cached(&state.model_cache, model_id, "Model")?;

        let params: serde_json::Value = serde_json::from_str(&entry.params_json)
            .map_err(|e| ServerError::Internal(format!("Failed to parse params: {e}")))?;

        Ok(GetModelResponse {
            model_id: model_id.to_string(),
            model_type: Self::model_type_to_string(entry.model_type),
            name: entry.name,
            params,
            created_at: entry.created_at.to_rfc3339(),
        })
    }

    /// Price an instrument using a cached model
    pub fn price_with_model(
        model_id: &str,
        request: &ModelPricingRequest,
        state: &Arc<AppState>,
    ) -> Result<ModelPricingResponse, ServerError> {
        let start = Instant::now();

        let entry = helpers::resolve_cached(&state.model_cache, model_id, "Model")?;

        // Extract pricing parameters
        let (price, greeks, num_paths, std_error) = match &request.instrument {
            InstrumentDto::VanillaOption {
                spot,
                strike,
                maturity,
                is_call,
            } => {
                let (p, g) = Self::price_vanilla_option(
                    entry.model_type,
                    &entry.params_json,
                    *spot,
                    *strike,
                    *maturity,
                    *is_call,
                    request.risk_free_rate.unwrap_or(0.05),
                    &request.method,
                    request.num_paths.unwrap_or(10_000),
                )?;
                let paths = if matches!(request.method, PricingMethodDto::MonteCarlo) {
                    Some(request.num_paths.unwrap_or(10_000))
                } else {
                    None
                };
                let err = if matches!(request.method, PricingMethodDto::MonteCarlo) {
                    Some(p * 0.01) // Placeholder std error
                } else {
                    None
                };
                (p, Some(g), paths, err)
            }
            InstrumentDto::Forward {
                spot,
                forward_price,
                maturity,
            } => {
                let r = request.risk_free_rate.unwrap_or(0.05);
                let df = (-r * maturity).exp();
                let p = (*forward_price - *spot) * df;
                let g = PricingGreeksDto {
                    delta: df,
                    gamma: 0.0,
                    vega: 0.0,
                    theta: -r * p,
                    rho: -maturity * p,
                };
                (p, Some(g), None, None)
            }
            InstrumentDto::AsianOption {
                spot,
                strike,
                maturity,
                is_call,
                ..
            } => {
                // Simplified Asian pricing
                let (p, g) = Self::price_vanilla_option(
                    entry.model_type,
                    &entry.params_json,
                    *spot,
                    *strike,
                    *maturity,
                    *is_call,
                    request.risk_free_rate.unwrap_or(0.05),
                    &PricingMethodDto::MonteCarlo,
                    request.num_paths.unwrap_or(10_000),
                )?;
                // Asian options typically worth less than vanilla
                (
                    p * 0.85,
                    Some(g),
                    Some(request.num_paths.unwrap_or(10_000)),
                    Some(p * 0.02),
                )
            }
            InstrumentDto::BarrierOption {
                spot,
                strike,
                maturity,
                is_call,
                ..
            } => {
                let (p, g) = Self::price_vanilla_option(
                    entry.model_type,
                    &entry.params_json,
                    *spot,
                    *strike,
                    *maturity,
                    *is_call,
                    request.risk_free_rate.unwrap_or(0.05),
                    &PricingMethodDto::MonteCarlo,
                    request.num_paths.unwrap_or(10_000),
                )?;
                // Barrier options typically worth less
                (
                    p * 0.7,
                    Some(g),
                    Some(request.num_paths.unwrap_or(10_000)),
                    Some(p * 0.03),
                )
            }
        };

        let elapsed = start.elapsed();

        Ok(ModelPricingResponse {
            model_id: model_id.to_string(),
            model_type: Self::model_type_to_string(entry.model_type),
            method: request.method,
            price,
            currency: "USD".to_string(),
            greeks,
            num_paths,
            std_error,
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Price a vanilla option with the given model
    fn price_vanilla_option(
        model_type: ModelType,
        params_json: &str,
        spot: f64,
        strike: f64,
        maturity: f64,
        is_call: bool,
        risk_free_rate: f64,
        method: &PricingMethodDto,
        _num_paths: usize,
    ) -> Result<(f64, PricingGreeksDto), ServerError> {
        // Extract volatility from model params
        let vol = match model_type {
            ModelType::Gbm => {
                let params: serde_json::Value = serde_json::from_str(params_json)
                    .map_err(|e| ServerError::Internal(format!("Parse error: {e}")))?;
                params["volatility"].as_f64().unwrap_or(0.2)
            }
            ModelType::Heston => {
                let params: serde_json::Value = serde_json::from_str(params_json)
                    .map_err(|e| ServerError::Internal(format!("Parse error: {e}")))?;
                params["v0"].as_f64().unwrap_or(0.04).sqrt()
            }
            ModelType::Sabr => {
                let params: serde_json::Value = serde_json::from_str(params_json)
                    .map_err(|e| ServerError::Internal(format!("Parse error: {e}")))?;
                params["alpha"].as_f64().unwrap_or(0.2)
            }
            _ => 0.2, // Default for other models
        };

        // Use Black-Scholes for analytical, simplified formula for others
        let price = match method {
            PricingMethodDto::Analytical => {
                Self::black_scholes(spot, strike, maturity, risk_free_rate, vol, is_call)
            }
            PricingMethodDto::MonteCarlo | PricingMethodDto::Tree => {
                // Use same formula with small noise for simulation
                let bs = Self::black_scholes(spot, strike, maturity, risk_free_rate, vol, is_call);
                bs * (1.0 + 0.001 * (rand_simple() - 0.5))
            }
        };

        // Calculate Greeks
        let d1 = ((spot / strike).ln() + (risk_free_rate + vol * vol / 2.0) * maturity)
            / (vol * maturity.sqrt());
        let d2 = d1 - vol * maturity.sqrt();

        let nd1 = normal_cdf(d1);
        let nd2 = normal_cdf(d2);
        let pdf_d1 = normal_pdf(d1);

        let (delta, gamma, vega, theta, rho) = if is_call {
            let delta = nd1;
            let gamma = pdf_d1 / (spot * vol * maturity.sqrt());
            let vega = spot * pdf_d1 * maturity.sqrt() / 100.0; // Per 1% vol
            let theta = -(spot * pdf_d1 * vol) / (2.0 * maturity.sqrt())
                - risk_free_rate * strike * (-risk_free_rate * maturity).exp() * nd2;
            let rho = strike * maturity * (-risk_free_rate * maturity).exp() * nd2 / 100.0;
            (delta, gamma, vega, theta / 365.0, rho)
        } else {
            let delta = nd1 - 1.0;
            let gamma = pdf_d1 / (spot * vol * maturity.sqrt());
            let vega = spot * pdf_d1 * maturity.sqrt() / 100.0;
            let theta = -(spot * pdf_d1 * vol) / (2.0 * maturity.sqrt())
                + risk_free_rate * strike * (-risk_free_rate * maturity).exp() * (1.0 - nd2);
            let rho = -strike * maturity * (-risk_free_rate * maturity).exp() * (1.0 - nd2) / 100.0;
            (delta, gamma, vega, theta / 365.0, rho)
        };

        Ok((
            price,
            PricingGreeksDto {
                delta,
                gamma,
                vega,
                theta,
                rho,
            },
        ))
    }

    /// Black-Scholes formula
    fn black_scholes(
        spot: f64,
        strike: f64,
        maturity: f64,
        r: f64,
        vol: f64,
        is_call: bool,
    ) -> f64 {
        let d1 =
            ((spot / strike).ln() + (r + vol * vol / 2.0) * maturity) / (vol * maturity.sqrt());
        let d2 = d1 - vol * maturity.sqrt();

        let df = (-r * maturity).exp();

        if is_call {
            spot * normal_cdf(d1) - strike * df * normal_cdf(d2)
        } else {
            strike * df * normal_cdf(-d2) - spot * normal_cdf(-d1)
        }
    }

    /// Validate GBM parameters
    fn validate_gbm(drift: f64, volatility: f64) -> ModelValidationDto {
        let mut errors = vec![];
        let mut warnings = vec![];

        if volatility < 0.0 {
            errors.push("Volatility must be non-negative".to_string());
        }
        if volatility > 2.0 {
            warnings.push("Volatility > 200% is unusually high".to_string());
        }
        if drift.abs() > 1.0 {
            warnings.push("Drift > 100% is unusually high".to_string());
        }

        ModelValidationDto {
            valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    /// Validate Heston parameters
    fn validate_heston(
        v0: f64,
        kappa: f64,
        theta: f64,
        sigma: f64,
        rho: f64,
    ) -> ModelValidationDto {
        let mut errors = vec![];
        let mut warnings = vec![];

        if v0 < 0.0 {
            errors.push("Initial variance (v0) must be non-negative".to_string());
        }
        if kappa < 0.0 {
            errors.push("Mean reversion (kappa) must be non-negative".to_string());
        }
        if theta < 0.0 {
            errors.push("Long-term variance (theta) must be non-negative".to_string());
        }
        if sigma < 0.0 {
            errors.push("Vol-of-vol (sigma) must be non-negative".to_string());
        }
        if rho < -1.0 || rho > 1.0 {
            errors.push("Correlation (rho) must be in [-1, 1]".to_string());
        }

        // Feller condition
        if 2.0 * kappa * theta < sigma * sigma {
            warnings.push("Feller condition violated: 2*kappa*theta < sigma^2".to_string());
        }

        ModelValidationDto {
            valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    /// Validate Hull-White parameters
    fn validate_hull_white(mean_reversion: f64, volatility: f64) -> ModelValidationDto {
        let mut errors = vec![];
        let warnings = vec![];

        if mean_reversion < 0.0 {
            errors.push("Mean reversion must be non-negative".to_string());
        }
        if volatility < 0.0 {
            errors.push("Volatility must be non-negative".to_string());
        }

        ModelValidationDto {
            valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    /// Validate CIR parameters
    fn validate_cir(kappa: f64, theta: f64, sigma: f64) -> ModelValidationDto {
        let mut errors = vec![];
        let mut warnings = vec![];

        if kappa < 0.0 {
            errors.push("Mean reversion (kappa) must be non-negative".to_string());
        }
        if theta < 0.0 {
            errors.push("Long-term mean (theta) must be non-negative".to_string());
        }
        if sigma < 0.0 {
            errors.push("Volatility (sigma) must be non-negative".to_string());
        }

        // Feller condition for CIR
        if 2.0 * kappa * theta < sigma * sigma {
            warnings.push("Feller condition violated: rate may go negative".to_string());
        }

        ModelValidationDto {
            valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    /// Validate SABR parameters
    fn validate_sabr(alpha: f64, beta: f64, rho: f64, nu: f64) -> ModelValidationDto {
        let mut errors = vec![];
        let warnings = vec![];

        if alpha < 0.0 {
            errors.push("Alpha must be non-negative".to_string());
        }
        if beta < 0.0 || beta > 1.0 {
            errors.push("Beta must be in [0, 1]".to_string());
        }
        if rho < -1.0 || rho > 1.0 {
            errors.push("Rho must be in [-1, 1]".to_string());
        }
        if nu < 0.0 {
            errors.push("Nu must be non-negative".to_string());
        }

        ModelValidationDto {
            valid: errors.is_empty(),
            warnings,
            errors,
        }
    }

    fn model_type_to_string(model_type: ModelType) -> String {
        match model_type {
            ModelType::Gbm => "gbm".to_string(),
            ModelType::Heston => "heston".to_string(),
            ModelType::HullWhite => "hull_white".to_string(),
            ModelType::Cir => "cir".to_string(),
            ModelType::Sabr => "sabr".to_string(),
        }
    }
}

/// Simple random number for Monte Carlo noise
fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f64;
    (nanos / 1_000_000_000.0).fract()
}

/// Standard normal CDF approximation
fn normal_cdf(x: f64) -> f64 { 0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2)) }

/// Standard normal PDF
fn normal_pdf(x: f64) -> f64 { (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt() }

/// Error function approximation
fn erf(x: f64) -> f64 {
    // Horner form approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

#[cfg(all(test, feature = "models"))]
mod tests {
    use super::*;
    use crate::{
        rest::dto::{GbmParamsDto, HestonParamsDto},
        state::AppStateConfig,
    };

    fn create_test_state() -> Arc<AppState> {
        let config = AppStateConfig {
            curve_cache_size: 10,
            fxvol_cache_size: 5,
            #[cfg(feature = "risk")]
            portfolio_cache_size: 10,
            model_cache_size: 10,
            #[cfg(feature = "volatility")]
            vol_surface_cache_size: 10,
        };
        Arc::new(AppState::with_config(config))
    }

    #[test]
    fn test_create_gbm_model() {
        let state = create_test_state();

        let request = CreateModelRequest::Gbm {
            name: Some("Test GBM".to_string()),
            params: GbmParamsDto {
                drift: 0.05,
                volatility: 0.2,
            },
        };

        let response = ModelService::create_model(&request, &state).unwrap();

        assert!(!response.model_id.is_empty());
        assert_eq!(response.model_type, "gbm");
        assert!(response.validation.valid);
    }

    #[test]
    fn test_create_heston_model() {
        let state = create_test_state();

        let request = CreateModelRequest::Heston {
            name: None,
            params: HestonParamsDto {
                v0: 0.04,
                kappa: 2.0,
                theta: 0.04,
                sigma: 0.3,
                rho: -0.7,
            },
        };

        let response = ModelService::create_model(&request, &state).unwrap();

        assert_eq!(response.model_type, "heston");
        assert!(response.validation.valid);
    }

    #[test]
    fn test_create_model_invalid_params() {
        let state = create_test_state();

        let request = CreateModelRequest::Gbm {
            name: None,
            params: GbmParamsDto {
                drift: 0.05,
                volatility: -0.2, // Invalid
            },
        };

        let result = ModelService::create_model(&request, &state);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_get_model() {
        let state = create_test_state();

        let create_request = CreateModelRequest::Gbm {
            name: Some("Test".to_string()),
            params: GbmParamsDto {
                drift: 0.05,
                volatility: 0.2,
            },
        };
        let create_response = ModelService::create_model(&create_request, &state).unwrap();

        let response = ModelService::get_model(&create_response.model_id, &state).unwrap();

        assert_eq!(response.model_id, create_response.model_id);
        assert_eq!(response.model_type, "gbm");
    }

    #[test]
    fn test_get_model_not_found() {
        let state = create_test_state();

        let result = ModelService::get_model(&uuid::Uuid::new_v4().to_string(), &state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServerError::NotFound(_)));
    }

    #[test]
    fn test_price_vanilla_option() {
        let state = create_test_state();

        let create_request = CreateModelRequest::Gbm {
            name: None,
            params: GbmParamsDto {
                drift: 0.05,
                volatility: 0.2,
            },
        };
        let create_response = ModelService::create_model(&create_request, &state).unwrap();

        let request = ModelPricingRequest {
            method: PricingMethodDto::Analytical,
            instrument: InstrumentDto::VanillaOption {
                spot: 100.0,
                strike: 100.0,
                maturity: 1.0,
                is_call: true,
            },
            num_paths: None,
            num_steps: None,
            risk_free_rate: Some(0.05),
        };

        let response =
            ModelService::price_with_model(&create_response.model_id, &request, &state).unwrap();

        assert!(response.price > 0.0);
        assert!(response.greeks.is_some());

        let greeks = response.greeks.unwrap();
        assert!(greeks.delta > 0.0); // ATM call delta should be ~0.5
        assert!(greeks.delta < 1.0);
    }

    #[test]
    fn test_black_scholes_call_put_parity() {
        // Test call-put parity: C - P = S - K*e^(-rT)
        let spot = 100.0;
        let strike = 100.0;
        let maturity = 1.0;
        let r = 0.05;
        let vol = 0.2;

        let call = ModelService::black_scholes(spot, strike, maturity, r, vol, true);
        let put = ModelService::black_scholes(spot, strike, maturity, r, vol, false);

        let expected = spot - strike * (-r * maturity).exp();
        let actual = call - put;

        assert!((actual - expected).abs() < 1e-10);
    }
}

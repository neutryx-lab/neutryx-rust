//! Volatility service for Vol Surface/Cube operations
//!
//! Provides volatility surface construction and implied vol queries.

#[cfg(feature = "volatility")]
use std::{sync::Arc, time::Instant};

#[cfg(feature = "volatility")]
use chrono::Utc;

#[cfg(feature = "volatility")]
use crate::{
    error::ServerError,
    services::helpers,
    rest::dto::{
        BuildFxVolSurfaceRequest, BuildFxVolSurfaceResponse, BuildVolCubeRequest,
        BuildVolCubeResponse, CalibrationQualityDto, GetImpliedVolRequest, GetImpliedVolResponse,
        SabrCalibrationDto, StrikeTypeDto,
    },
    state::{AppState, SabrParams, VolSurfaceEntry, VolSurfaceType},
};

/// Convert calibration DTOs to cache params.
#[cfg(feature = "volatility")]
fn to_cache_params(sabr: &[SabrCalibrationDto]) -> Vec<SabrParams> {
    sabr.iter()
        .map(|p| SabrParams { expiry: p.expiry, alpha: p.alpha, beta: p.beta, rho: p.rho, nu: p.nu })
        .collect()
}

/// Service for volatility surface operations
#[cfg(feature = "volatility")]
pub struct VolatilityService;

#[cfg(feature = "volatility")]
impl VolatilityService {
    /// Build an FX volatility surface from quotes
    pub fn build_fx_vol_surface(
        request: &BuildFxVolSurfaceRequest,
        state: &Arc<AppState>,
    ) -> Result<BuildFxVolSurfaceResponse, ServerError> {
        let start = Instant::now();

        // Validate input
        if request.quotes.is_empty() {
            return Err(ServerError::InvalidRequest(
                "At least one volatility quote is required".to_string(),
            ));
        }

        if request.fx_spot <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "FX spot must be positive".to_string(),
            ));
        }

        // Group quotes by expiry for slice-wise calibration
        let mut expiry_quotes: std::collections::BTreeMap<
            u64,
            Vec<&crate::rest::dto::VolQuoteDto>,
        > = std::collections::BTreeMap::new();

        for quote in &request.quotes {
            // Use integer key for grouping (expiry * 1000 to handle floating point)
            let key = (quote.expiry * 1000.0) as u64;
            expiry_quotes.entry(key).or_default().push(quote);
        }

        // Calibrate SABR for each expiry slice
        let mut sabr_params = Vec::new();
        let mut total_residual = 0.0;
        let mut max_residual = 0.0;

        for (key, quotes) in &expiry_quotes {
            let expiry = *key as f64 / 1000.0;

            // Calculate ATM vol and smile parameters from quotes
            let atm_vol = quotes
                .iter()
                .find(|q| matches!(q.quote_type, crate::rest::dto::VolQuoteTypeDto::Atm))
                .map(|q| q.vol)
                .unwrap_or_else(|| quotes.iter().map(|q| q.vol).sum::<f64>() / quotes.len() as f64);

            // Simplified SABR calibration
            let (alpha, rho, nu, residual) =
                Self::calibrate_sabr_slice(atm_vol, quotes, request.beta, expiry);

            total_residual += residual;
            if residual > max_residual {
                max_residual = residual;
            }

            sabr_params.push(SabrCalibrationDto {
                expiry,
                alpha,
                beta: request.beta,
                rho,
                nu,
                residual,
            });
        }

        let expiry_count = sabr_params.len();

        let entry = VolSurfaceEntry {
            surface_type: VolSurfaceType::FxSurface,
            underlying: request.currency_pair.clone(),
            sabr_params: to_cache_params(&sabr_params),
            expiry_count,
            residual_ss: Some(total_residual),
            created_at: Utc::now(),
        };

        let surface_id = state.vol_surface_cache.add(entry);

        let elapsed = start.elapsed();

        Ok(BuildFxVolSurfaceResponse {
            surface_id: surface_id.to_string(),
            currency_pair: request.currency_pair.clone(),
            expiry_count,
            sabr_params,
            calibration_quality: CalibrationQualityDto {
                converged: total_residual < 1e-4,
                total_residual_ss: total_residual,
                max_residual,
                iterations: Some(10), // Placeholder
            },
            calibration_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Build an IR volatility cube from ATM vols and smile quotes
    pub fn build_vol_cube(
        request: &BuildVolCubeRequest,
        state: &Arc<AppState>,
    ) -> Result<BuildVolCubeResponse, ServerError> {
        let start = Instant::now();

        // Validate dimensions
        if request.expiries.is_empty() || request.tenors.is_empty() {
            return Err(ServerError::InvalidRequest(
                "Expiries and tenors must not be empty".to_string(),
            ));
        }

        if request.atm_vols.len() != request.expiries.len() {
            return Err(ServerError::InvalidRequest(
                "ATM vols dimensions must match expiries".to_string(),
            ));
        }

        for (i, row) in request.atm_vols.iter().enumerate() {
            if row.len() != request.tenors.len() {
                return Err(ServerError::InvalidRequest(format!(
                    "ATM vols row {} has {} elements, expected {}",
                    i,
                    row.len(),
                    request.tenors.len()
                )));
            }
        }

        // Parse tenors to years
        let expiry_years: Vec<f64> = request
            .expiries
            .iter()
            .map(|t| Self::parse_tenor(t))
            .collect();

        // Calibrate SABR for each (expiry, tenor) cell
        let mut sabr_params = Vec::new();
        let mut total_residual = 0.0;
        let mut max_residual = 0.0;

        for (i, expiry) in expiry_years.iter().enumerate() {
            for (j, _tenor) in request.tenors.iter().enumerate() {
                let atm_vol = request.atm_vols[i][j];

                // Use smile quotes if available, otherwise use ATM only
                let (alpha, rho, nu, residual) = if let Some(ref smile) = request.smile_quotes {
                    let smile_vols: Vec<f64> = smile.vols[i][j].clone();
                    Self::calibrate_sabr_with_smile(atm_vol, &smile_vols, request.beta, *expiry)
                } else {
                    // No smile - use simple calibration
                    (atm_vol, 0.0, 0.3, 0.0)
                };

                total_residual += residual;
                if residual > max_residual {
                    max_residual = residual;
                }

                sabr_params.push(SabrCalibrationDto {
                    expiry: *expiry,
                    alpha,
                    beta: request.beta,
                    rho,
                    nu,
                    residual,
                });
            }
        }

        let expiry_count = request.expiries.len();
        let tenor_count = request.tenors.len();

        let entry = VolSurfaceEntry {
            surface_type: VolSurfaceType::IrCube,
            underlying: request.index.clone(),
            sabr_params: to_cache_params(&sabr_params),
            expiry_count: sabr_params.len(),
            residual_ss: Some(total_residual),
            created_at: Utc::now(),
        };

        let cube_id = state.vol_surface_cache.add(entry);

        let elapsed = start.elapsed();

        Ok(BuildVolCubeResponse {
            cube_id: cube_id.to_string(),
            index: request.index.clone(),
            expiry_count,
            tenor_count,
            sabr_params,
            calibration_quality: CalibrationQualityDto {
                converged: total_residual < 1e-3,
                total_residual_ss: total_residual,
                max_residual,
                iterations: Some(15), // Placeholder
            },
            calibration_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Get implied volatility from a cached surface
    pub fn get_implied_vol(
        surface_id: &str,
        request: &GetImpliedVolRequest,
        state: &Arc<AppState>,
    ) -> Result<GetImpliedVolResponse, ServerError> {
        let entry =
            helpers::resolve_cached(&state.vol_surface_cache, surface_id, "Surface")?;

        // Convert strike based on type
        let strike = match request.strike_type {
            StrikeTypeDto::Absolute => request.strike,
            StrikeTypeDto::Moneyness => {
                let forward = request.forward.ok_or_else(|| {
                    ServerError::InvalidRequest(
                        "Forward is required for moneyness strike type".to_string(),
                    )
                })?;
                request.strike * forward
            }
            StrikeTypeDto::LogMoneyness => {
                let forward = request.forward.ok_or_else(|| {
                    ServerError::InvalidRequest(
                        "Forward is required for log-moneyness strike type".to_string(),
                    )
                })?;
                forward * request.strike.exp()
            }
            StrikeTypeDto::Delta => {
                // For delta, we need to invert the BS formula
                // Simplified: assume ATM delta = 0.5
                let forward = request.forward.unwrap_or(1.0);
                forward * (1.0 + (request.strike - 0.5) * 0.5)
            }
        };

        // Find nearest expiry slice(s) for interpolation
        let (sabr_params, implied_vol) =
            Self::interpolate_vol(&entry.sabr_params, request.expiry, strike)?;

        Ok(GetImpliedVolResponse {
            surface_id: surface_id.to_string(),
            expiry: request.expiry,
            strike,
            implied_vol,
            sabr_params: Some(SabrCalibrationDto {
                expiry: sabr_params.expiry,
                alpha: sabr_params.alpha,
                beta: sabr_params.beta,
                rho: sabr_params.rho,
                nu: sabr_params.nu,
                residual: 0.0, // Not applicable for query
            }),
        })
    }

    /// Calibrate SABR parameters for a single expiry slice
    fn calibrate_sabr_slice(
        atm_vol: f64,
        quotes: &[&crate::rest::dto::VolQuoteDto],
        beta: f64,
        _expiry: f64,
    ) -> (f64, f64, f64, f64) {
        // Simplified SABR calibration
        // In production, would use pricer_models::builder::vol::SabrSliceCalibrator

        let alpha = atm_vol;

        // Estimate rho from risk reversal if available
        let rho = quotes
            .iter()
            .find(|q| {
                matches!(
                    q.quote_type,
                    crate::rest::dto::VolQuoteTypeDto::RiskReversal
                )
            })
            .map(|q| (q.vol / atm_vol).clamp(-0.9, 0.9))
            .unwrap_or(0.0);

        // Estimate nu from butterfly if available
        let nu = quotes
            .iter()
            .find(|q| matches!(q.quote_type, crate::rest::dto::VolQuoteTypeDto::Butterfly))
            .map(|q| (q.vol / atm_vol * 2.0).clamp(0.1, 2.0))
            .unwrap_or(0.3);

        // Calculate residual
        let residual = quotes
            .iter()
            .map(|q| {
                let model_vol = Self::sabr_vol(alpha, beta, rho, nu, 1.0, q.delta_or_strike);
                (q.vol - model_vol).powi(2)
            })
            .sum::<f64>();

        (alpha, rho, nu, residual)
    }

    /// Calibrate SABR with explicit smile vols
    fn calibrate_sabr_with_smile(
        atm_vol: f64,
        smile_vols: &[f64],
        _beta: f64,
        _expiry: f64,
    ) -> (f64, f64, f64, f64) {
        let alpha = atm_vol;

        // Estimate rho from smile asymmetry
        let rho = if smile_vols.len() >= 2 {
            let left = smile_vols[0];
            let right = smile_vols.last().copied().unwrap_or(left);
            ((right - left) / atm_vol).clamp(-0.9, 0.9)
        } else {
            0.0
        };

        // Estimate nu from smile curvature
        let nu = if smile_vols.len() >= 3 {
            let avg_wing = (smile_vols[0] + smile_vols.last().copied().unwrap_or(0.0)) / 2.0;
            ((avg_wing - atm_vol).abs() / atm_vol * 3.0).clamp(0.1, 2.0)
        } else {
            0.3
        };

        let residual = smile_vols
            .iter()
            .map(|v| (v - atm_vol).powi(2))
            .sum::<f64>();

        (alpha, rho, nu, residual)
    }

    /// Interpolate volatility from SABR parameters
    fn interpolate_vol(
        params: &[SabrParams],
        expiry: f64,
        strike: f64,
    ) -> Result<(SabrParams, f64), ServerError> {
        if params.is_empty() {
            return Err(ServerError::NotFound(
                "No calibrated parameters available".to_string(),
            ));
        }

        // Find bracketing expiries
        let mut lower = &params[0];
        let mut upper = &params[0];

        for p in params {
            if p.expiry <= expiry {
                lower = p;
            }
            if p.expiry >= expiry && upper.expiry < expiry {
                upper = p;
            }
        }

        // Linear interpolation of SABR params
        let t = if (upper.expiry - lower.expiry).abs() > f64::EPSILON {
            (expiry - lower.expiry) / (upper.expiry - lower.expiry)
        } else {
            0.5
        };

        let interp_params = SabrParams {
            expiry,
            alpha: lower.alpha + t * (upper.alpha - lower.alpha),
            beta: lower.beta + t * (upper.beta - lower.beta),
            rho: lower.rho + t * (upper.rho - lower.rho),
            nu: lower.nu + t * (upper.nu - lower.nu),
        };

        // Calculate implied vol using SABR formula
        let forward = 1.0; // Normalised
        let implied_vol = Self::sabr_vol(
            interp_params.alpha,
            interp_params.beta,
            interp_params.rho,
            interp_params.nu,
            forward,
            strike,
        );

        Ok((interp_params, implied_vol))
    }

    /// SABR implied volatility formula (Hagan et al. approximation)
    fn sabr_vol(alpha: f64, beta: f64, rho: f64, nu: f64, forward: f64, strike: f64) -> f64 {
        if (forward - strike).abs() < 1e-10 {
            // ATM approximation
            return alpha;
        }

        let f = forward;
        let k = strike;
        let fk_beta = (f * k).powf((1.0 - beta) / 2.0);

        let log_fk = (f / k).ln();
        let z = nu / alpha * fk_beta * log_fk;

        let x = if z.abs() < 1e-10 {
            z
        } else {
            let sqrt_term = ((1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho).ln();
            z * sqrt_term / (1.0 - rho)
        };

        let factor1 = alpha / fk_beta;
        let factor2 = 1.0
            + ((1.0 - beta).powi(2) / 24.0 * log_fk.powi(2)
                + (1.0 - beta).powi(4) / 1920.0 * log_fk.powi(4));
        let factor3 = if x.abs() < 1e-10 { 1.0 } else { z / x };

        factor1 * factor2 * factor3
    }

    /// Parse tenor string to years
    fn parse_tenor(tenor: &str) -> f64 {
        let tenor = tenor.trim().to_uppercase();

        if let Some(num_str) = tenor.strip_suffix('Y') {
            return num_str.parse().unwrap_or(1.0);
        }
        if let Some(num_str) = tenor.strip_suffix('M') {
            return num_str.parse::<f64>().unwrap_or(1.0) / 12.0;
        }
        if let Some(num_str) = tenor.strip_suffix('W') {
            return num_str.parse::<f64>().unwrap_or(1.0) / 52.0;
        }
        if let Some(num_str) = tenor.strip_suffix('D') {
            return num_str.parse::<f64>().unwrap_or(1.0) / 365.0;
        }

        // Default to years
        tenor.parse().unwrap_or(1.0)
    }
}

#[cfg(all(test, feature = "volatility"))]
mod tests {
    use super::*;
    use crate::{
        rest::dto::{VolQuoteDto, VolQuoteTypeDto},
        state::AppStateConfig,
    };

    fn create_test_state() -> Arc<AppState> {
        let config = AppStateConfig {
            curve_cache_size: 10,
            fxvol_cache_size: 5,
            #[cfg(feature = "risk")]
            portfolio_cache_size: 10,
            #[cfg(feature = "models")]
            model_cache_size: 10,
            vol_surface_cache_size: 10,
        };
        Arc::new(AppState::with_config(config))
    }

    #[test]
    fn test_build_fx_vol_surface() {
        let state = create_test_state();

        let request = BuildFxVolSurfaceRequest {
            currency_pair: "USDJPY".to_string(),
            quotes: vec![
                VolQuoteDto {
                    expiry: 0.25,
                    delta_or_strike: 0.0,
                    quote_type: VolQuoteTypeDto::Atm,
                    vol: 0.10,
                },
                VolQuoteDto {
                    expiry: 0.5,
                    delta_or_strike: 0.0,
                    quote_type: VolQuoteTypeDto::Atm,
                    vol: 0.11,
                },
                VolQuoteDto {
                    expiry: 1.0,
                    delta_or_strike: 0.0,
                    quote_type: VolQuoteTypeDto::Atm,
                    vol: 0.12,
                },
            ],
            fx_spot: 150.0,
            domestic_rate: 0.05,
            foreign_rate: 0.01,
            beta: 0.5,
        };

        let response = VolatilityService::build_fx_vol_surface(&request, &state).unwrap();

        assert!(!response.surface_id.is_empty());
        assert_eq!(response.currency_pair, "USDJPY");
        assert_eq!(response.expiry_count, 3);
        assert_eq!(response.sabr_params.len(), 3);
    }

    #[test]
    fn test_build_fx_vol_surface_empty_quotes() {
        let state = create_test_state();

        let request = BuildFxVolSurfaceRequest {
            currency_pair: "EURUSD".to_string(),
            quotes: vec![],
            fx_spot: 1.10,
            domestic_rate: 0.05,
            foreign_rate: 0.03,
            beta: 0.5,
        };

        let result = VolatilityService::build_fx_vol_surface(&request, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_vol_cube() {
        let state = create_test_state();

        let request = BuildVolCubeRequest {
            index: "USD-SOFR".to_string(),
            expiries: vec!["1Y".to_string(), "2Y".to_string()],
            tenors: vec!["1Y".to_string(), "5Y".to_string(), "10Y".to_string()],
            atm_vols: vec![vec![0.20, 0.22, 0.24], vec![0.21, 0.23, 0.25]],
            smile_quotes: None,
            beta: 0.5,
        };

        let response = VolatilityService::build_vol_cube(&request, &state).unwrap();

        assert!(!response.cube_id.is_empty());
        assert_eq!(response.index, "USD-SOFR");
        assert_eq!(response.expiry_count, 2);
        assert_eq!(response.tenor_count, 3);
        assert_eq!(response.sabr_params.len(), 6); // 2 * 3
    }

    #[test]
    fn test_get_implied_vol() {
        let state = create_test_state();

        // First create a surface
        let build_request = BuildFxVolSurfaceRequest {
            currency_pair: "USDJPY".to_string(),
            quotes: vec![VolQuoteDto {
                expiry: 0.5,
                delta_or_strike: 0.0,
                quote_type: VolQuoteTypeDto::Atm,
                vol: 0.15,
            }],
            fx_spot: 150.0,
            domestic_rate: 0.05,
            foreign_rate: 0.01,
            beta: 0.5,
        };
        let build_response =
            VolatilityService::build_fx_vol_surface(&build_request, &state).unwrap();

        // Query implied vol using normalised moneyness (ATM = 1.0)
        let request = GetImpliedVolRequest {
            expiry: 0.5,
            strike: 1.0, // ATM in normalised moneyness
            strike_type: StrikeTypeDto::Moneyness,
            forward: Some(1.0), // Normalised forward
        };

        let response =
            VolatilityService::get_implied_vol(&build_response.surface_id, &request, &state)
                .unwrap();

        assert!(response.implied_vol > 0.0);
        assert!(response.sabr_params.is_some());
        // ATM vol should be close to alpha (0.15)
        assert!((response.implied_vol - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_get_implied_vol_not_found() {
        let state = create_test_state();

        let request = GetImpliedVolRequest {
            expiry: 0.5,
            strike: 100.0,
            strike_type: StrikeTypeDto::Absolute,
            forward: None,
        };

        let result =
            VolatilityService::get_implied_vol(&uuid::Uuid::new_v4().to_string(), &request, &state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServerError::NotFound(_)));
    }

    #[test]
    fn test_parse_tenor() {
        assert!((VolatilityService::parse_tenor("1Y") - 1.0).abs() < f64::EPSILON);
        assert!((VolatilityService::parse_tenor("6M") - 0.5).abs() < f64::EPSILON);
        assert!((VolatilityService::parse_tenor("3M") - 0.25).abs() < f64::EPSILON);
        assert!((VolatilityService::parse_tenor("1W") - 1.0 / 52.0).abs() < 0.001);
    }

    #[test]
    fn test_sabr_vol_atm() {
        // At ATM, SABR vol should equal alpha
        let alpha = 0.2;
        let vol = VolatilityService::sabr_vol(alpha, 0.5, 0.0, 0.3, 1.0, 1.0);
        assert!((vol - alpha).abs() < 0.01);
    }
}

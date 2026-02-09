//! Volatility cube service for IR vol, FX vol, and implied PDF operations
//!
//! Extracted from `demo_service` to provide a focused service for
//! volatility surface calibration and related computations.

use std::{collections::HashMap, path::Path, sync::Arc};

use adapter_loader::{parse_instruments, InstrumentSpec};
use infra_domain::time::parse_tenor_to_years;
use pricer_models::{
    builder::{BootstrapConfig, CurveBootstrapper, InterpolationMethod as BuilderInterpolation},
    market::YieldCurve,
    builder::vol::{SliceCalibrationConfig, VolCubeBuilder},
};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        CalibrationMetadata, CalibrationParameters, CellDiagnostics, FxVolCalibrateRequest,
        FxVolPair, FxVolPairsResponse, FxVolQuote, FxVolQuotesResponse, ImpliedPdfRequest,
        ImpliedPdfResponse, IrVolCurrenciesResponse, IrVolCurrency, IrVolQuote,
        IrVolQuotesResponse, SabrSmileRequest, SabrSmileResponse, SmilePoint,
        SwaptionInstrument, VolcubeCalibrateRequest, VolcubeCalibrateResponse,
        VolcubeIndicesResponse, VolcubeInstrumentsResponse, VolcubeModelsResponse,
    },
    state::AppState,
};

/// Service for volatility cube operations (IR vol, FX vol, implied PDF)
pub struct VolcubeService;

impl VolcubeService {
    // =========================================================================
    // IR Volatility API
    // =========================================================================

    /// Get IR vol currencies
    pub fn get_ir_vol_currencies(
        _state: &Arc<AppState>,
    ) -> Result<IrVolCurrenciesResponse, ServerError> {
        let vol_path = Path::new("demo/data/config/vol_surfaces.json");
        if vol_path.exists() {
            let content = std::fs::read_to_string(vol_path).map_err(|e| {
                ServerError::Internal(format!("Failed to read vol_surfaces.json: {e}"))
            })?;
            let vol_data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                ServerError::Internal(format!("Failed to parse vol_surfaces.json: {e}"))
            })?;

            if let Some(irvol_items) = vol_data.get("irVol").and_then(|i| i.as_array()) {
                let currencies: Vec<IrVolCurrency> = irvol_items
                    .iter()
                    .filter_map(|item| {
                        item.get("currency")
                            .and_then(|c| c.as_str())
                            .map(|currency| IrVolCurrency {
                                currency: currency.to_string(),
                            })
                    })
                    .collect();

                // Remove duplicates (keep first occurrence)
                let mut seen = std::collections::HashSet::new();
                let unique_currencies: Vec<IrVolCurrency> = currencies
                    .into_iter()
                    .filter(|c| seen.insert(c.currency.clone()))
                    .collect();

                return Ok(IrVolCurrenciesResponse {
                    currencies: unique_currencies,
                });
            }
        }

        // Fallback to hardcoded list
        Ok(IrVolCurrenciesResponse {
            currencies: vec![
                IrVolCurrency {
                    currency: "USD".to_string(),
                },
                IrVolCurrency {
                    currency: "EUR".to_string(),
                },
                IrVolCurrency {
                    currency: "JPY".to_string(),
                },
                IrVolCurrency {
                    currency: "GBP".to_string(),
                },
            ],
        })
    }

    /// Get IR vol quotes for a currency
    pub fn get_ir_vol_quotes(
        currency: &str,
        _state: &Arc<AppState>,
    ) -> Result<IrVolQuotesResponse, ServerError> {
        let file_path = format!("demo/data/input/irvol/{}.json", currency.to_lowercase());
        let path = Path::new(&file_path);

        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| ServerError::Internal(format!("Failed to read IR vol file: {e}")))?;
            let data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ServerError::Internal(format!("Failed to parse IR vol file: {e}")))?;

            let mut quotes = Vec::new();
            if let Some(quotes_arr) = data.get("quotes").and_then(|q| q.as_array()) {
                for quote in quotes_arr {
                    quotes.push(IrVolQuote {
                        expiry: quote
                            .get("expiry")
                            .and_then(|e| e.as_str())
                            .unwrap_or("")
                            .to_string(),
                        tenor: quote
                            .get("tenor")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                        atm_vol: quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        smile: None,
                    });
                }
            }

            return Ok(IrVolQuotesResponse {
                quotes,
                vol_type: Some("normal".to_string()),
                source: Some("Internal".to_string()),
            });
        }

        // Return mock data if file not found
        Ok(IrVolQuotesResponse {
            quotes: vec![
                IrVolQuote {
                    expiry: "1M".to_string(),
                    tenor: "1Y".to_string(),
                    atm_vol: 0.0050,
                    smile: Some(vec![
                        SmilePoint {
                            strike_offset_bp: -50.0,
                            vol: 0.0055,
                        },
                        SmilePoint {
                            strike_offset_bp: 50.0,
                            vol: 0.0045,
                        },
                    ]),
                },
                IrVolQuote {
                    expiry: "1Y".to_string(),
                    tenor: "5Y".to_string(),
                    atm_vol: 0.0065,
                    smile: None,
                },
            ],
            vol_type: Some("normal".to_string()),
            source: Some("Internal".to_string()),
        })
    }

    // =========================================================================
    // FX Volatility API
    // =========================================================================

    /// Get FX vol pairs
    pub fn get_fx_vol_pairs(_state: &Arc<AppState>) -> Result<FxVolPairsResponse, ServerError> {
        let vol_path = Path::new("demo/data/config/vol_surfaces.json");
        if vol_path.exists() {
            let content = std::fs::read_to_string(vol_path).map_err(|e| {
                ServerError::Internal(format!("Failed to read vol_surfaces.json: {e}"))
            })?;
            let vol_data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                ServerError::Internal(format!("Failed to parse vol_surfaces.json: {e}"))
            })?;

            if let Some(fxvol_items) = vol_data.get("fxVol").and_then(|i| i.as_array()) {
                let pairs: Vec<FxVolPair> = fxvol_items
                    .iter()
                    .filter_map(|item| {
                        item.get("currencyPair")
                            .and_then(|p| p.as_str())
                            .map(|pair| FxVolPair {
                                pair: pair.to_string(),
                            })
                    })
                    .collect();

                return Ok(FxVolPairsResponse { pairs });
            }
        }

        // Fallback
        Ok(FxVolPairsResponse {
            pairs: vec![
                FxVolPair {
                    pair: "EURUSD".to_string(),
                },
                FxVolPair {
                    pair: "USDJPY".to_string(),
                },
            ],
        })
    }

    /// Get FX vol quotes for a pair, including computed FX forwards
    pub fn get_fx_vol_quotes(
        pair: &str,
        _state: &Arc<AppState>,
    ) -> Result<FxVolQuotesResponse, ServerError> {
        let file_path = format!("demo/data/input/fxvol/{}.json", pair.to_lowercase());
        let path = Path::new(&file_path);

        if !path.exists() {
            return Err(ServerError::NotFound(format!(
                "FX vol data not found for pair: {}",
                pair
            )));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ServerError::Internal(format!("Failed to read FX vol file: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse FX vol file: {e}")))?;

        let spot = data.get("spot").and_then(|s| s.as_f64());
        let domestic_rate = data.get("domesticRate").and_then(|r| r.as_f64());
        let foreign_rate = data.get("foreignRate").and_then(|r| r.as_f64());

        // Look up base/quote currencies from fx_pairs.json
        let (base_ccy, quote_ccy) = Self::lookup_fx_pair_currencies(pair);

        // Build discount curves for both currencies and compute FX forwards
        let forwards = if let Some(spot_val) = spot {
            Self::compute_fx_forwards(
                &base_ccy,
                &quote_ccy,
                spot_val,
                &data,
            )
            .ok()
        } else {
            None
        };

        let mut quotes = Vec::new();
        if let Some(quotes_arr) = data.get("quotes").and_then(|q| q.as_array()) {
            for quote in quotes_arr {
                let expiry = quote.get("expiry").and_then(|e| e.as_f64()).unwrap_or(0.0);
                let expiry_label = quote
                    .get("tenor")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".to_string());

                let forward = forwards
                    .as_ref()
                    .and_then(|fwd_map| fwd_map.get(&expiry_label).copied());

                quotes.push(FxVolQuote {
                    expiry,
                    expiry_label,
                    atm_vol: quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    rr25d: quote.get("rr25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    bf25d: quote.get("bf25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    rr10d: quote.get("rr10d").and_then(|v| v.as_f64()),
                    bf10d: quote.get("bf10d").and_then(|v| v.as_f64()),
                    forward,
                });
            }
        }

        Ok(FxVolQuotesResponse {
            quotes,
            spot,
            domestic_rate,
            foreign_rate,
        })
    }

    // =========================================================================
    // Volcube API
    // =========================================================================

    /// Get volcube indices (rate index identifiers)
    pub fn get_volcube_indices(
        _state: &Arc<AppState>,
    ) -> Result<VolcubeIndicesResponse, ServerError> {
        let vol_path = Path::new("demo/data/config/vol_surfaces.json");
        let content = std::fs::read_to_string(vol_path)
            .map_err(|e| ServerError::Internal(format!("Failed to read vol_surfaces.json: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            ServerError::Internal(format!("Failed to parse vol_surfaces.json: {e}"))
        })?;

        let indices: Vec<String> = data
            .get("irVol")
            .and_then(|i| i.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("rateIndex").and_then(|c| c.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(VolcubeIndicesResponse { indices })
    }

    /// Get available volcube calibration models
    pub fn get_volcube_models(
        _state: &Arc<AppState>,
    ) -> Result<VolcubeModelsResponse, ServerError> {
        Ok(VolcubeModelsResponse {
            models: vec![
                "SABR".to_string(),
                "Normal SABR".to_string(),
            ],
        })
    }

    /// Get swaption instruments for volcube calibration
    pub fn get_volcube_instruments(
        currency: &str,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeInstrumentsResponse, ServerError> {
        let vol_path =
            Path::new("demo/data/input/irvol").join(format!("{}.json", currency.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("Swaption vol data not found for: {}", currency))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse vol data: {e}")))?;

        let mut instruments = Vec::new();

        if let Some(quotes) = data.get("quotes").and_then(|q| q.as_array()) {
            for quote in quotes {
                let expiry = quote
                    .get("expiry")
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .to_string();
                let tenor = quote
                    .get("tenor")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let atm_vol = quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let smile = quote
                    .get("smile")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|pt| {
                                let offset = pt.get("strikeOffsetBp").and_then(|o| o.as_f64())?;
                                let vol = pt.get("vol").and_then(|v| v.as_f64())?;
                                Some(SmilePoint {
                                    strike_offset_bp: offset,
                                    vol,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                instruments.push(SwaptionInstrument {
                    expiry,
                    tenor,
                    strike: "ATM".to_string(),
                    atm_vol,
                    smile,
                    enabled: true,
                });
            }
        }

        // Extract reference date from metadata.lastUpdated
        let reference_date = data
            .get("metadata")
            .and_then(|m| m.get("lastUpdated"))
            .and_then(|d| d.as_str())
            .map(|s| s.split('T').next().unwrap_or(s).to_string());

        Ok(VolcubeInstrumentsResponse {
            instruments,
            reference_date,
        })
    }

    /// Calibrate volcube (swaption vol surface) using real SABR calibration
    /// via `pricer_models::builder::vol::VolCubeBuilder` (Levenberg-Marquardt).
    pub fn calibrate_volcube(
        request: &VolcubeCalibrateRequest,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeCalibrateResponse, ServerError> {
        let start = std::time::Instant::now();

        // 1. Load vol data from file
        let vol_path = Path::new("demo/data/input/irvol")
            .join(format!("{}.json", request.index.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("Vol data not found for: {}", request.index))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse vol data: {e}")))?;

        let quotes = data
            .get("quotes")
            .and_then(|q| q.as_array())
            .cloned()
            .unwrap_or_default();
        let instrument_count = quotes.len();

        // 2. Resolve forward rates from request or use fallback
        let forward_rates = request
            .forward_rates
            .clone()
            .unwrap_or_default();
        let default_forward: f64 = 0.04;

        // 3. Determine vol type and select config based on model selection
        let is_normal_vol = data
            .get("metadata")
            .and_then(|m| m.get("volType"))
            .and_then(|v| v.as_str())
            .map_or(false, |v| v == "normal");

        let use_normal_sabr = request
            .model
            .as_deref()
            .map_or(is_normal_vol, |m| m == "Normal SABR");

        let config = if use_normal_sabr {
            SliceCalibrationConfig::normal()
        } else {
            SliceCalibrationConfig::rates()
        };
        let beta = config.fixed_beta.unwrap_or(0.5);

        // 4. Build VolCubeBuilder with real quotes
        let mut builder = VolCubeBuilder::with_config(config);

        // Track string keys for result lookup
        let mut cell_keys: Vec<(String, String, f64, f64)> = Vec::new();

        for quote in &quotes {
            let expiry_str = quote.get("expiry").and_then(|v| v.as_str()).unwrap_or("");
            let tenor_str = quote.get("tenor").and_then(|v| v.as_str()).unwrap_or("");
            let atm_vol_raw = quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0);

            if expiry_str.is_empty() || tenor_str.is_empty() || atm_vol_raw == 0.0 {
                continue;
            }

            let expiry_years = parse_tenor_to_years(expiry_str).map_err(|e| {
                ServerError::Internal(format!("Invalid expiry '{expiry_str}': {e}"))
            })?;
            let tenor_years = parse_tenor_to_years(tenor_str).map_err(|e| {
                ServerError::Internal(format!("Invalid tenor '{tenor_str}': {e}"))
            })?;

            let key = format!("{expiry_str}|{tenor_str}");
            let forward = forward_rates.get(&key).copied().unwrap_or(default_forward);

            // For normal vol: convert from percentage-like units to decimal (0.68 → 0.0068)
            // For lognormal vol: use as-is
            let atm_vol = if is_normal_vol {
                atm_vol_raw / 100.0
            } else {
                atm_vol_raw
            };

            // ATM quote
            builder.add_quote(expiry_years, tenor_years, forward, atm_vol, forward);

            // Smile quotes
            let smile = quote
                .get("smile")
                .and_then(|s| s.as_array())
                .cloned()
                .unwrap_or_default();
            for pt in &smile {
                let offset_bp = pt
                    .get("strikeOffsetBp")
                    .and_then(|o| o.as_f64())
                    .unwrap_or(0.0);
                let vol_raw = pt.get("vol").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let strike = forward + offset_bp / 10_000.0;
                let vol = if is_normal_vol { vol_raw / 100.0 } else { vol_raw };

                builder.add_quote(expiry_years, tenor_years, strike, vol, forward);
            }

            cell_keys.push((
                expiry_str.to_string(),
                tenor_str.to_string(),
                expiry_years,
                tenor_years,
            ));
        }

        // 5. Calibrate
        let cube_result = builder.calibrate().map_err(|e| {
            ServerError::Internal(format!("SABR calibration failed: {e}"))
        })?;

        // 6. Convert results to response DTOs
        let mut cell_parameters = HashMap::new();
        let mut cell_diagnostics_map = HashMap::new();
        let mut alpha_sum = 0.0_f64;
        let mut rho_sum = 0.0_f64;
        let mut nu_sum = 0.0_f64;
        let mut count = 0_usize;

        for (expiry_str, tenor_str, expiry_years, tenor_years) in &cell_keys {
            let key = format!("{expiry_str}|{tenor_str}");

            if let Some(params) = cube_result.get(*expiry_years, *tenor_years) {
                cell_parameters.insert(
                    key.clone(),
                    CalibrationParameters {
                        alpha: round4(params.alpha),
                        beta: round4(params.beta),
                        rho: round4(params.rho),
                        nu: round4(params.nu),
                    },
                );
                alpha_sum += params.alpha;
                rho_sum += params.rho;
                nu_sum += params.nu;
                count += 1;
            }

            if let Some(diag) = cube_result.get_diagnostics(*expiry_years, *tenor_years) {
                cell_diagnostics_map.insert(
                    key,
                    CellDiagnostics {
                        converged: diag.converged,
                        iterations: diag.iterations,
                        rmse: diag.rmse,
                    },
                );
            }
        }

        // 7. Global (average) parameters
        let global_params = if count > 0 {
            CalibrationParameters {
                alpha: round4(alpha_sum / count as f64),
                beta: round4(beta),
                rho: round4(rho_sum / count as f64),
                nu: round4(nu_sum / count as f64),
            }
        } else {
            CalibrationParameters {
                alpha: 0.02,
                beta: 0.5,
                rho: -0.15,
                nu: 0.4,
            }
        };

        let converged_count = cell_diagnostics_map.values().filter(|d| d.converged).count();
        let elapsed = start.elapsed();
        let model = request.model.clone().unwrap_or_else(|| "SABR".to_string());

        Ok(VolcubeCalibrateResponse {
            model,
            metadata: CalibrationMetadata {
                instrument_count,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
                converged_count: Some(converged_count),
                max_rmse: Some(cube_result.max_rmse()),
            },
            parameters: global_params,
            cell_parameters,
            cell_diagnostics: Some(cell_diagnostics_map),
        })
    }

    /// Calibrate FX vol surface
    pub fn calibrate_fxvol(
        request: &FxVolCalibrateRequest,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeCalibrateResponse, ServerError> {
        let start = std::time::Instant::now();

        let vol_path = Path::new("demo/data/input/fxvol")
            .join(format!("{}.json", request.pair.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("FX vol data not found for: {}", request.pair))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse FX vol data: {e}")))?;

        let instrument_count = data
            .get("smiles")
            .and_then(|s| s.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let elapsed = start.elapsed();

        // Mock SABR parameters for FX vol
        Ok(VolcubeCalibrateResponse {
            model: "SABR".to_string(),
            metadata: CalibrationMetadata {
                instrument_count,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
                converged_count: None,
                max_rmse: None,
            },
            parameters: CalibrationParameters {
                alpha: 0.15,
                beta: 0.5,
                rho: -0.20,
                nu: 0.35,
            },
            cell_parameters: std::collections::HashMap::new(),
            cell_diagnostics: None,
        })
    }

    // =========================================================================
    // Implied PDF API
    // =========================================================================

    /// Compute implied probability density via Breeden-Litzenberger (d²C/dk²)
    pub fn compute_implied_pdf(
        request: &ImpliedPdfRequest,
    ) -> Result<ImpliedPdfResponse, ServerError> {
        let expiry = request.expiry_years;
        if expiry <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "expiry_years must be positive".to_string(),
            ));
        }

        // Build sorted smile points including ATM at k=0
        let mut smile_pts: Vec<(f64, f64)> = vec![(0.0, request.atm_vol)];
        for pt in &request.smile {
            smile_pts.push((pt.strike_offset_bp, pt.vol));
        }
        smile_pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        // Remove duplicates at k=0 (keep first = ATM)
        smile_pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12);

        let dk_bp = request.step_bp;
        let dk = dk_bp / 10_000.0; // bp → decimal

        let range = request.range_bp;
        let n_steps = (2.0 * range / dk_bp).round() as i64;

        let mut offsets = Vec::with_capacity(n_steps as usize);
        let mut density = Vec::with_capacity(n_steps as usize);

        for i in 0..=n_steps {
            let k_bp = -range + i as f64 * dk_bp;
            let k = k_bp / 10_000.0;

            // Interpolate smile vol at k-dk, k, k+dk
            let vol_lo = interpolate_smile_vol(&smile_pts, k_bp - dk_bp) / 100.0;
            let vol_mid = interpolate_smile_vol(&smile_pts, k_bp) / 100.0;
            let vol_hi = interpolate_smile_vol(&smile_pts, k_bp + dk_bp) / 100.0;

            // Bachelier call prices (F=0 for strike-offset formulation)
            let c_lo = bachelier_call(k - dk, vol_lo, expiry)?;
            let c_mid = bachelier_call(k, vol_mid, expiry)?;
            let c_hi = bachelier_call(k + dk, vol_hi, expiry)?;

            // Finite difference: d²C/dk²
            let d2c = (c_lo - 2.0 * c_mid + c_hi) / (dk * dk);

            offsets.push(k_bp);
            density.push(d2c.max(0.0));
        }

        Ok(ImpliedPdfResponse { offsets, density })
    }

    // =========================================================================
    // SABR Smile + Density (from calibrated parameters)
    // =========================================================================

    /// Compute a smooth SABR smile and implied density from calibrated parameters.
    ///
    /// Returns `n_points` evenly spaced points in `[-range_bp, +range_bp]`.
    /// Vols are returned in the same percentage scale as market data
    /// (i.e. multiply by 100 on the frontend to get bp display).
    pub fn compute_sabr_smile(
        request: &SabrSmileRequest,
    ) -> Result<SabrSmileResponse, ServerError> {
        use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};

        let forward = request.forward;
        let expiry = request.expiry_years;
        if forward <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "forward must be positive".to_string(),
            ));
        }
        if expiry <= 0.0 {
            return Err(ServerError::InvalidRequest(
                "expiry_years must be positive".to_string(),
            ));
        }

        let sabr_params = SabrImpliedVolParams::new(
            forward,
            request.alpha,
            request.beta,
            request.nu,
            request.rho,
            expiry,
        );

        let n = request.n_points.max(3);
        let range = request.range_bp;
        let step = 2.0 * range / (n - 1) as f64;

        let mut offsets = Vec::with_capacity(n);
        let mut vols = Vec::with_capacity(n);

        for i in 0..n {
            let offset_bp = -range + i as f64 * step;
            let strike = forward + offset_bp / 10_000.0;
            // Clamp strike to positive (SABR requires K > 0 for β > 0)
            let strike = strike.max(1e-8);

            let black_vol = sabr_implied_vol(&sabr_params, strike)
                .unwrap_or(request.alpha);

            // Convert Black vol → normal vol (percentage scale matching market data)
            // σ_Normal ≈ σ_Black × F^β
            let normal_vol_pct = black_vol * forward.powf(request.beta);

            offsets.push(offset_bp);
            vols.push(normal_vol_pct);
        }

        // Compute density via Breeden-Litzenberger (d²C/dk²) using Bachelier
        let dk_bp = step;
        let dk = dk_bp / 10_000.0;
        let mut density = Vec::with_capacity(n);

        for i in 0..n {
            if i == 0 || i == n - 1 {
                density.push(0.0);
                continue;
            }

            let vol_lo = vols[i - 1]; // already in decimal (percentage / 1)
            let vol_mid = vols[i];
            let vol_hi = vols[i + 1];

            let k_lo = forward + offsets[i - 1] / 10_000.0;
            let k_mid = forward + offsets[i] / 10_000.0;
            let k_hi = forward + offsets[i + 1] / 10_000.0;

            let c_lo = bachelier_call_fwd(forward, k_lo, vol_lo, expiry);
            let c_mid = bachelier_call_fwd(forward, k_mid, vol_mid, expiry);
            let c_hi = bachelier_call_fwd(forward, k_hi, vol_hi, expiry);

            let d2c = (c_lo - 2.0 * c_mid + c_hi) / (dk * dk);
            density.push(d2c.max(0.0));
        }

        Ok(SabrSmileResponse {
            offsets,
            vols,
            density,
        })
    }

    // =========================================================================
    // FX Forward Computation Helpers
    // =========================================================================

    /// Look up base/quote currencies for an FX pair from config
    fn lookup_fx_pair_currencies(pair: &str) -> (String, String) {
        let config_path = Path::new("demo/data/config/fx_pairs.json");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(pairs) = data.get("fxPairs").and_then(|p| p.as_array()) {
                    for p in pairs {
                        let id = p.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        if id.eq_ignore_ascii_case(pair) {
                            let base = p
                                .get("baseCurrency")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let quote = p
                                .get("quoteCurrency")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            return (base, quote);
                        }
                    }
                }
            }
        }
        // Fallback: first 3 chars = base, last 3 = quote
        let pair_upper = pair.to_uppercase();
        (pair_upper[..3].to_string(), pair_upper[3..].to_string())
    }

    /// Look up the rate index name for a currency from currencies.json
    fn lookup_rate_index(currency: &str) -> Option<String> {
        let config_path = Path::new("demo/data/config/currencies.json");
        let content = std::fs::read_to_string(config_path).ok()?;
        let data: serde_json::Value = serde_json::from_str(&content).ok()?;
        let currencies = data.get("currencies")?.as_array()?;
        for ccy in currencies {
            let code = ccy.get("code").and_then(|c| c.as_str())?;
            if code.eq_ignore_ascii_case(currency) {
                let index = ccy.get("index").and_then(|i| i.as_str())?;
                return Some(format!("{}-{}", currency.to_lowercase(), index.to_lowercase()));
            }
        }
        None
    }

    /// Build a discount curve from a rate data file
    fn build_discount_curve_for_currency(
        rate_index: &str,
    ) -> Result<Box<dyn YieldCurve<f64>>, ServerError> {
        let file_path = format!("demo/data/input/rates/{}.json", rate_index);
        let path = Path::new(&file_path);
        if !path.exists() {
            return Err(ServerError::NotFound(format!(
                "Rate data not found for index: {}",
                rate_index
            )));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ServerError::Internal(format!("Failed to read rate file: {e}")))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse rate file: {e}")))?;

        let instruments = data
            .get("instruments")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();

        // Use deposits + OIS only (matching the swaption tab pattern)
        let allowed_types: std::collections::HashSet<&str> =
            ["deposit", "ois"].iter().copied().collect();

        let specs: Vec<InstrumentSpec> = instruments
            .iter()
            .filter_map(|i| {
                let itype = i.get("type").and_then(|t| t.as_str())?;
                if !allowed_types.contains(itype) {
                    return None;
                }
                Some(InstrumentSpec {
                    instrument_type: itype.to_string(),
                    tenor: i.get("tenor").and_then(|t| t.as_str())?.to_string(),
                    rate: i.get("rate").and_then(|r| r.as_f64())?,
                    event_date: None,
                    expected_rate_spike: None,
                })
            })
            .collect();

        if specs.is_empty() {
            return Err(ServerError::Internal(format!(
                "No instruments found for index: {}",
                rate_index
            )));
        }

        let market_instruments = parse_instruments(&specs)
            .map_err(|e| ServerError::Internal(format!("Instrument parsing failed: {e}")))?;

        let config = BootstrapConfig::new(1e-10, 100)
            .with_interpolation(BuilderInterpolation::LogLinear);
        let bootstrapper = CurveBootstrapper::with_config(config);

        let (curve, _) = bootstrapper
            .bootstrap_to_curve_with_jacobian(&market_instruments, &[])
            .map_err(|e| ServerError::Internal(format!("Curve bootstrap failed: {e}")))?;

        Ok(Box::new(curve))
    }

    /// Compute FX forward rates for each tenor in the vol quotes
    ///
    /// F(T) = Spot × DF_base(T) / DF_quote(T)
    /// where base = foreign currency, quote = domestic currency
    fn compute_fx_forwards(
        base_ccy: &str,
        quote_ccy: &str,
        spot: f64,
        vol_data: &serde_json::Value,
    ) -> Result<HashMap<String, f64>, ServerError> {
        // Try to build curves from rate data files
        let base_index = Self::lookup_rate_index(base_ccy);
        let quote_index = Self::lookup_rate_index(quote_ccy);

        let (base_curve, quote_curve) = match (base_index, quote_index) {
            (Some(bi), Some(qi)) => {
                let bc = Self::build_discount_curve_for_currency(&bi)?;
                let qc = Self::build_discount_curve_for_currency(&qi)?;
                (Some(bc), Some(qc))
            }
            _ => (None, None),
        };

        let mut forwards = HashMap::new();

        if let Some(quotes) = vol_data.get("quotes").and_then(|q| q.as_array()) {
            for quote in quotes {
                let tenor_label = quote
                    .get("tenor")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                let expiry_years = quote.get("expiry").and_then(|e| e.as_f64()).unwrap_or(0.0);

                if tenor_label.is_empty() || expiry_years <= 0.0 {
                    continue;
                }

                let fwd = if let (Some(ref bc), Some(ref qc)) = (&base_curve, &quote_curve) {
                    // Use bootstrapped discount curves
                    let df_base = bc.discount_factor(expiry_years).unwrap_or(1.0);
                    let df_quote = qc.discount_factor(expiry_years).unwrap_or(1.0);
                    spot * df_base / df_quote
                } else {
                    // Fallback to simple continuous compounding from fxvol file rates
                    let dom_rate = vol_data
                        .get("domesticRate")
                        .and_then(|r| r.as_f64())
                        .unwrap_or(0.0);
                    let for_rate = vol_data
                        .get("foreignRate")
                        .and_then(|r| r.as_f64())
                        .unwrap_or(0.0);
                    spot * (-for_rate * expiry_years).exp() / (-dom_rate * expiry_years).exp()
                };

                forwards.insert(tenor_label.to_string(), fwd);
            }
        }

        Ok(forwards)
    }
}

/// Bachelier call price with explicit forward
fn bachelier_call_fwd(forward: f64, strike: f64, vol: f64, expiry: f64) -> f64 {
    use pricer_core::math::formulas::Bachelier;
    if vol <= 0.0 {
        return (forward - strike).max(0.0);
    }
    match Bachelier::new(forward, vol) {
        Ok(model) => model.price_call(strike, expiry),
        Err(_) => (forward - strike).max(0.0),
    }
}

/// Linear interpolation on smile points, flat extrapolation outside range
fn interpolate_smile_vol(pts: &[(f64, f64)], k_bp: f64) -> f64 {
    if pts.is_empty() {
        return 0.0;
    }
    if pts.len() == 1 || k_bp <= pts[0].0 {
        return pts[0].1;
    }
    if k_bp >= pts[pts.len() - 1].0 {
        return pts[pts.len() - 1].1;
    }
    // Find bracketing interval
    for w in pts.windows(2) {
        if k_bp >= w[0].0 && k_bp <= w[1].0 {
            let t = (k_bp - w[0].0) / (w[1].0 - w[0].0);
            return w[0].1 + t * (w[1].1 - w[0].1);
        }
    }
    pts[pts.len() - 1].1
}

/// Compute Bachelier call price with F=0 (strike-offset formulation)
fn bachelier_call(strike: f64, vol: f64, expiry: f64) -> Result<f64, ServerError> {
    use pricer_core::math::formulas::Bachelier;
    if vol <= 0.0 {
        // Zero vol → intrinsic value
        return Ok(if strike < 0.0 { -strike } else { 0.0 });
    }
    let model = Bachelier::new(0.0_f64, vol)
        .map_err(|e| ServerError::Pricing(format!("Bachelier model error: {e}")))?;
    Ok(model.price_call(strike, expiry))
}

/// Round to 4 decimal places.
fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

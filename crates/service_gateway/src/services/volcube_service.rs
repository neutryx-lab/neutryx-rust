//! Volatility cube service for IR vol, FX vol, and implied PDF operations
//!
//! Extracted from `demo_service` to provide a focused service for
//! volatility surface calibration and related computations.

use std::{path::Path, sync::Arc};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        CalibrationMetadata, CalibrationParameters, FxVolCalibrateRequest, FxVolPair,
        FxVolPairsResponse, FxVolQuote, FxVolQuotesResponse, ImpliedPdfRequest,
        ImpliedPdfResponse, IrVolCurrenciesResponse, IrVolCurrency, IrVolQuote,
        IrVolQuotesResponse, SmilePoint, SwaptionInstrument, VolcubeCalibrateRequest,
        VolcubeCalibrateResponse, VolcubeIndicesResponse, VolcubeInstrumentsResponse,
        VolcubeModelsResponse,
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

    /// Get FX vol quotes for a pair
    pub fn get_fx_vol_quotes(
        pair: &str,
        _state: &Arc<AppState>,
    ) -> Result<FxVolQuotesResponse, ServerError> {
        let file_path = format!("demo/data/input/fxvol/{}.json", pair.to_lowercase());
        let path = Path::new(&file_path);

        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| ServerError::Internal(format!("Failed to read FX vol file: {e}")))?;
            let data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| ServerError::Internal(format!("Failed to parse FX vol file: {e}")))?;

            let mut quotes = Vec::new();
            if let Some(quotes_arr) = data.get("quotes").and_then(|q| q.as_array()) {
                for quote in quotes_arr {
                    let expiry = quote.get("expiry").and_then(|e| e.as_f64()).unwrap_or(0.0);
                    // Require explicit tenor field in input data
                    let expiry_label = quote
                        .get("tenor")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    quotes.push(FxVolQuote {
                        expiry,
                        expiry_label,
                        atm_vol: quote.get("atmVol").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        rr25d: quote.get("rr25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        bf25d: quote.get("bf25d").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        rr10d: quote.get("rr10d").and_then(|v| v.as_f64()),
                        bf10d: quote.get("bf10d").and_then(|v| v.as_f64()),
                    });
                }
            }

            let spot = data.get("spot").and_then(|s| s.as_f64());

            return Ok(FxVolQuotesResponse { quotes, spot });
        }

        Err(ServerError::NotFound(format!(
            "FX vol data not found for pair: {}",
            pair
        )))
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
                "SABR-LMM".to_string(),
                "Heston".to_string(),
                "Local Vol".to_string(),
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

    /// Calibrate volcube (swaption vol surface)
    pub fn calibrate_volcube(
        request: &VolcubeCalibrateRequest,
        _state: &Arc<AppState>,
    ) -> Result<VolcubeCalibrateResponse, ServerError> {
        let start = std::time::Instant::now();

        let vol_path = Path::new("demo/data/input/irvol")
            .join(format!("{}.json", request.index.to_lowercase()));

        let content = std::fs::read_to_string(&vol_path).map_err(|_| {
            ServerError::NotFound(format!("Vol data not found for: {}", request.index))
        })?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ServerError::Internal(format!("Failed to parse vol data: {e}")))?;

        let instrument_count = data
            .get("quotes")
            .and_then(|q| q.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        // Get SABR parameters from the file or use defaults
        let params = data.get("smileParameters");
        let alpha = params
            .and_then(|p| p.get("defaultAlpha"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.02);
        let beta = params
            .and_then(|p| p.get("defaultBeta"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let rho = params
            .and_then(|p| p.get("defaultRho"))
            .and_then(|v| v.as_f64())
            .unwrap_or(-0.15);
        let nu = params
            .and_then(|p| p.get("defaultNu"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.4);

        let elapsed = start.elapsed();
        let model = request.model.clone().unwrap_or_else(|| "SABR".to_string());

        Ok(VolcubeCalibrateResponse {
            model,
            metadata: CalibrationMetadata {
                instrument_count,
                processing_time_ms: elapsed.as_secs_f64() * 1000.0,
            },
            parameters: CalibrationParameters {
                alpha,
                beta,
                rho,
                nu,
            },
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
            },
            parameters: CalibrationParameters {
                alpha: 0.15,
                beta: 0.5,
                rho: -0.20,
                nu: 0.35,
            },
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

//! XVA engine service for demo GUI.
//!
//! Provides a stateless service layer that orchestrates the XVA simulation
//! engine, bilateral CVA/DVA calculations, and FVA computations for the
//! demo portfolio hierarchy.

use std::{collections::HashMap, sync::OnceLock, time::Instant};

use infra_domain::counterparty::VmCsa;
use pricer_risk::{
    portfolio::{
        xva::{BilateralXvaCalculator, OwnCreditParams},
        CounterpartyId, CreditParams, NettingSetId, TradeId,
    },
    xva_engine::{
        IsdaAgreement, VmCsaNode, XvaCounterparty, XvaEngineConfig, XvaHierarchy,
        XvaRiskIndicators, XvaSimulator,
    },
};

use crate::{
    error::ServerError,
    rest::dto::xva::{
        CounterpartyXvaResult, DemoCounterparty, DemoNettingSet, HierarchyCounterparty,
        HierarchyIsda, HierarchySummary, HierarchyVmCsa, NettingSetResult, PfeProfile,
        XvaBilateralRequest, XvaBilateralResponse, XvaConfigSummary, XvaCsvExportResponse,
        XvaDefaultConfigResponse, XvaSimulationRequest, XvaSimulationResponse,
    },
};

/// Cached last simulation result for CSV export.
static LAST_SIMULATION: OnceLock<parking_lot::Mutex<Option<CachedSimulation>>> = OnceLock::new();

/// Internal cache for the last simulation result.
struct CachedSimulation {
    risk_indicators: HashMap<String, XvaRiskIndicators>,
}

/// Counterparty definition for the demo portfolio.
struct DemoCpDef {
    id: &'static str,
    name: &'static str,
    rating: &'static str,
    hazard_rate: f64,
    lgd: f64,
    ns_id: &'static str,
    csa_id: &'static str,
    trade_ids: Vec<&'static str>,
    trade_types: Vec<&'static str>,
}

/// Stateless XVA service with static methods.
pub struct XvaService;

impl XvaService {
    /// Returns default demo XVA configuration with 3 counterparties.
    pub fn get_default_config() -> Result<XvaDefaultConfigResponse, ServerError> {
        let defs = Self::demo_counterparty_defs();

        let counterparties = defs
            .iter()
            .map(|d| DemoCounterparty {
                id: d.id.to_string(),
                name: d.name.to_string(),
                credit_rating: d.rating.to_string(),
                hazard_rate: d.hazard_rate,
                lgd: d.lgd,
                netting_sets: vec![DemoNettingSet {
                    id: d.ns_id.to_string(),
                    has_csa: true,
                    trade_count: d.trade_ids.len(),
                    trade_types: d.trade_types.iter().map(|s| s.to_string()).collect(),
                }],
            })
            .collect();

        Ok(XvaDefaultConfigResponse {
            n_paths: 10_000,
            horizon_years: 5.0,
            time_step: "quarterly".to_string(),
            antithetic: true,
            bilateral: true,
            compute_fva: true,
            pfe_percentiles: vec![0.95, 0.975, 0.99],
            counterparties,
        })
    }

    /// Runs a full Monte Carlo XVA simulation with the demo portfolio.
    pub fn run_simulation(
        request: &XvaSimulationRequest,
    ) -> Result<XvaSimulationResponse, ServerError> {
        let start = Instant::now();

        // ── Build engine config ──
        let n_paths = request.n_paths.unwrap_or(10_000);
        let horizon = request.horizon_years.unwrap_or(5.0);
        let antithetic = request.antithetic.unwrap_or(true);
        let bilateral = request.bilateral.unwrap_or(true);
        let compute_fva = request.compute_fva.unwrap_or(true);
        let pfe_percentiles = request
            .pfe_percentiles
            .clone()
            .unwrap_or_else(|| vec![0.95, 0.975, 0.99]);

        let time_grid =
            Self::build_time_grid(horizon, request.time_step.as_deref().unwrap_or("quarterly"));

        let mut builder = XvaEngineConfig::builder()
            .n_paths(n_paths)
            .time_grid(time_grid.clone())
            .antithetic(antithetic)
            .pfe_percentiles(pfe_percentiles.clone())
            .bilateral(bilateral)
            .compute_fva(compute_fva)
            .compute_ecb(true);

        if let Some(seed) = request.seed {
            builder = builder.seed(seed);
        }

        let config = builder
            .build()
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid XVA config: {e}")))?;

        // ── Build demo hierarchy ──
        let (hierarchy, cp_defs) = Self::build_demo_hierarchy();

        // ── Simulate correlated GBM paths (2 factors: equity + rates) ──
        let simulator = XvaSimulator::new(config.clone());

        let n_factors = 2;
        let drift = vec![0.03, 0.01]; // equity drift, rates drift
        let vol = vec![0.20, 0.05]; // equity vol, rates vol
        let correlation = vec![vec![1.0, 0.3], vec![0.3, 1.0]];

        let paths = simulator
            .simulate_paths(n_factors, &drift, &vol, &correlation)
            .map_err(|e| ServerError::Pricing(format!("Simulation error: {e}")))?;

        let n_times = time_grid.len();
        let actual_paths = paths[0][0].len();

        // ── Compute netted trade values for each netting set ──
        let mut netted_values: HashMap<NettingSetId, Vec<Vec<f64>>> = HashMap::new();

        for def in &cp_defs {
            let ns_id = NettingSetId::new(def.ns_id);
            let n_trades = def.trade_ids.len();

            // Generate synthetic trade values using simulated paths.
            // Each trade gets a weighted combination of the two factors with
            // different notionals and signs to create realistic exposure profiles.
            let mut ns_values = vec![vec![0.0; actual_paths]; n_times];

            for (trade_idx, _) in def.trade_ids.iter().enumerate() {
                let notional = match trade_idx % 3 {
                    0 => 1_000_000.0,
                    1 => -500_000.0,
                    _ => 750_000.0,
                };
                let factor_weight_0 = if trade_idx % 2 == 0 { 0.7 } else { 0.3 };
                let factor_weight_1 = 1.0 - factor_weight_0;

                for t in 0..n_times {
                    for p in 0..actual_paths {
                        // Synthetic PV = notional * (weighted_factor - 1.0)
                        let factor_value =
                            factor_weight_0 * paths[0][t][p] + factor_weight_1 * paths[1][t][p];
                        let trade_pv = notional * (factor_value - 1.0);
                        ns_values[t][p] += trade_pv;
                    }
                }
            }

            // Scale by number of trades to get reasonable exposures
            let scale = 1.0 / (n_trades as f64).sqrt();
            for t in 0..n_times {
                for p in 0..actual_paths {
                    ns_values[t][p] *= scale;
                }
            }

            netted_values.insert(ns_id, ns_values);
        }

        // ── Compute exposure profiles ──
        let (epe_profiles, ene_profiles) = simulator.compute_exposure_profiles(&netted_values);

        // ── Compute risk indicators (PFE, ECB) ──
        let mut risk_indicators: HashMap<NettingSetId, XvaRiskIndicators> = HashMap::new();

        for (ns_id, values) in &netted_values {
            let epe = epe_profiles.get(ns_id).cloned().unwrap_or_default();
            let ene = ene_profiles.get(ns_id).cloned().unwrap_or_default();

            let mut ri = XvaRiskIndicators::new(time_grid.clone());
            ri.epe = epe;
            ri.ene = ene;

            // Compute ECB as difference between EPE and ENE (simplified)
            ri.ecb = ri
                .epe
                .iter()
                .zip(ri.ene.iter())
                .map(|(e, n)| (e - n).abs())
                .collect();

            // Compute PFE at each requested percentile
            for &pct in &pfe_percentiles {
                let pfe = XvaRiskIndicators::compute_pfe(values, pct);
                ri.pfe.insert(format!("{:.1}", pct * 100.0), pfe);
            }

            risk_indicators.insert(ns_id.clone(), ri);
        }

        // ── Build netting set results ──
        let mut netting_set_results: Vec<NettingSetResult> = Vec::new();

        for (ns_id, ri) in &risk_indicators {
            let peak_epe = ri.epe.iter().cloned().fold(0.0_f64, f64::max);
            let peak_ene = ri.ene.iter().cloned().fold(0.0_f64, f64::max);
            let avg_epe = if ri.epe.is_empty() {
                0.0
            } else {
                ri.epe.iter().sum::<f64>() / ri.epe.len() as f64
            };
            let avg_ene = if ri.ene.is_empty() {
                0.0
            } else {
                ri.ene.iter().sum::<f64>() / ri.ene.len() as f64
            };

            let mut pfe_profiles: Vec<PfeProfile> = Vec::new();
            let mut pfe_keys: Vec<&String> = ri.pfe.keys().collect();
            pfe_keys.sort();
            for key in pfe_keys {
                let values = ri.pfe.get(key).cloned().unwrap_or_default();
                let peak = values.iter().cloned().fold(0.0_f64, f64::max);
                pfe_profiles.push(PfeProfile {
                    percentile: key.parse::<f64>().unwrap_or(95.0),
                    label: format!("PFE {}%", key),
                    values,
                    peak,
                });
            }

            netting_set_results.push(NettingSetResult {
                netting_set_id: ns_id.as_str().to_string(),
                epe: ri.epe.clone(),
                ene: ri.ene.clone(),
                ecb: ri.ecb.clone(),
                pfe: pfe_profiles,
                peak_epe,
                peak_ene,
                avg_epe,
                avg_ene,
            });
        }

        // Sort by netting set ID for deterministic ordering
        netting_set_results.sort_by(|a, b| a.netting_set_id.cmp(&b.netting_set_id));

        // ── Compute bilateral CVA/DVA and FVA per counterparty ──
        let own_credit = OwnCreditParams::new(0.01, 0.4)
            .map_err(|e| ServerError::Pricing(format!("Own credit params error: {e}")))?;

        let discount_factors: Vec<f64> = time_grid.iter().map(|&t| (-0.03 * t).exp()).collect();

        let mut counterparty_results: Vec<CounterpartyXvaResult> = Vec::new();

        for def in &cp_defs {
            let ns_id = NettingSetId::new(def.ns_id);
            let epe = epe_profiles.get(&ns_id).cloned().unwrap_or_default();
            let ene = ene_profiles.get(&ns_id).cloned().unwrap_or_default();

            let credit_params = CreditParams::new(def.hazard_rate, def.lgd)
                .map_err(|e| ServerError::Pricing(format!("Credit params error: {e}")))?;

            // Bilateral CVA/DVA
            let bilateral_result = BilateralXvaCalculator::compute_bilateral_cva(
                &epe,
                &ene,
                &time_grid,
                &credit_params,
                &own_credit,
            );

            // FVA with cross-currency basis
            let funding_spread = 0.005; // 50bps borrowing spread
            let lending_spread = 0.003; // 30bps lending spread

            let survival_both: Vec<f64> = time_grid
                .iter()
                .map(|&t| credit_params.survival_prob(t) * own_credit.survival_prob(t))
                .collect();

            let fva_result = BilateralXvaCalculator::compute_fva_with_basis(
                &epe,
                &ene,
                &time_grid,
                funding_spread,
                lending_spread,
                &discount_factors,
                &survival_both,
                None,
            );

            let total_xva = bilateral_result.bcva - bilateral_result.bdva + fva_result.fva;

            counterparty_results.push(CounterpartyXvaResult {
                counterparty_id: def.id.to_string(),
                credit_rating: def.rating.to_string(),
                hazard_rate: def.hazard_rate,
                lgd: def.lgd,
                ucva: bilateral_result.ucva,
                udva: bilateral_result.udva,
                bcva: bilateral_result.bcva,
                bdva: bilateral_result.bdva,
                fca: fva_result.fca,
                fba: fva_result.fba,
                fva: fva_result.fva,
                total_xva,
                netting_set_count: 1,
                trade_count: def.trade_ids.len(),
            });
        }

        // ── Build hierarchy summary ──
        let hierarchy_summary = Self::build_hierarchy_summary(&hierarchy);

        // ── Cache for CSV export ──
        let cached = CachedSimulation {
            risk_indicators: risk_indicators
                .into_iter()
                .map(|(k, v)| (k.as_str().to_string(), v))
                .collect(),
        };

        let lock = LAST_SIMULATION.get_or_init(|| parking_lot::Mutex::new(None));
        *lock.lock() = Some(cached);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(XvaSimulationResponse {
            config: XvaConfigSummary {
                n_paths,
                time_points: time_grid.len(),
                horizon_years: horizon,
                antithetic,
                bilateral,
                compute_fva,
                pfe_percentiles,
            },
            time_grid,
            n_paths: actual_paths,
            netting_sets: netting_set_results,
            counterparty_results,
            hierarchy: hierarchy_summary,
            computation_time_ms: elapsed_ms,
        })
    }

    /// Computes bilateral CVA/DVA and FVA from given exposure profiles.
    pub fn compute_bilateral(
        request: &XvaBilateralRequest,
    ) -> Result<XvaBilateralResponse, ServerError> {
        let start = Instant::now();

        if request.epe.len() != request.time_grid.len() {
            return Err(ServerError::InvalidRequest(
                "EPE length must match time_grid length".to_string(),
            ));
        }
        if request.ene.len() != request.time_grid.len() {
            return Err(ServerError::InvalidRequest(
                "ENE length must match time_grid length".to_string(),
            ));
        }

        let credit_params = CreditParams::new(request.hazard_rate, request.lgd)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid credit params: {e}")))?;

        let own_credit = OwnCreditParams::new(request.own_hazard_rate, request.own_lgd)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid own credit params: {e}")))?;

        let bilateral_result = BilateralXvaCalculator::compute_bilateral_cva(
            &request.epe,
            &request.ene,
            &request.time_grid,
            &credit_params,
            &own_credit,
        );

        // FVA computation
        let funding_spread = request.funding_spread.unwrap_or(0.005);
        let lending_spread = funding_spread * 0.6; // lending at 60% of borrowing spread

        let discount_factors: Vec<f64> = request
            .time_grid
            .iter()
            .map(|&t| (-0.03 * t).exp())
            .collect();

        let survival_both: Vec<f64> = request
            .time_grid
            .iter()
            .map(|&t| credit_params.survival_prob(t) * own_credit.survival_prob(t))
            .collect();

        let xccy_basis_vec: Vec<f64>;
        let xccy_basis_ref = if let Some(basis) = request.xccy_basis {
            xccy_basis_vec = vec![basis; request.time_grid.len()];
            Some(xccy_basis_vec.as_slice())
        } else {
            None
        };

        let fva_result = BilateralXvaCalculator::compute_fva_with_basis(
            &request.epe,
            &request.ene,
            &request.time_grid,
            funding_spread,
            lending_spread,
            &discount_factors,
            &survival_both,
            xccy_basis_ref,
        );

        let total_xva = bilateral_result.bcva - bilateral_result.bdva + fva_result.fva;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(XvaBilateralResponse {
            ucva: bilateral_result.ucva,
            udva: bilateral_result.udva,
            bcva: bilateral_result.bcva,
            bdva: bilateral_result.bdva,
            fca: fva_result.fca,
            fba: fva_result.fba,
            fva: fva_result.fva,
            total_xva,
            computation_time_ms: elapsed_ms,
        })
    }

    /// Exports risk indicators as CSV for a given netting set.
    pub fn export_csv(ns_id: &str) -> Result<XvaCsvExportResponse, ServerError> {
        let lock = LAST_SIMULATION.get_or_init(|| parking_lot::Mutex::new(None));
        let guard = lock.lock();

        let cached = guard
            .as_ref()
            .ok_or_else(|| ServerError::NotFound("No simulation has been run yet".to_string()))?;

        let ri = cached.risk_indicators.get(ns_id).ok_or_else(|| {
            ServerError::NotFound(format!(
                "Netting set '{}' not found in last simulation",
                ns_id
            ))
        })?;

        let mut csv_buf = Vec::new();
        ri.to_csv(&mut csv_buf)
            .map_err(|e| ServerError::Internal(format!("CSV write error: {e}")))?;

        let csv_data = String::from_utf8(csv_buf)
            .map_err(|e| ServerError::Internal(format!("CSV encoding error: {e}")))?;

        let row_count = ri.time_grid.len();

        Ok(XvaCsvExportResponse {
            csv_data,
            netting_set_id: ns_id.to_string(),
            row_count,
        })
    }

    // ── Private helpers ──

    /// Returns demo counterparty definitions.
    fn demo_counterparty_defs() -> Vec<DemoCpDef> {
        vec![
            DemoCpDef {
                id: "BANK_A",
                name: "Global Bank A",
                rating: "AA",
                hazard_rate: 0.005,
                lgd: 0.4,
                ns_id: "NS_BANK_A",
                csa_id: "CSA_BANK_A",
                trade_ids: vec!["T_BA_1", "T_BA_2", "T_BA_3"],
                trade_types: vec!["IRS", "CCS", "FRA"],
            },
            DemoCpDef {
                id: "HEDGE_FUND_B",
                name: "Alpha Hedge Fund B",
                rating: "BBB",
                hazard_rate: 0.02,
                lgd: 0.6,
                ns_id: "NS_HEDGE_FUND_B",
                csa_id: "CSA_HF_B",
                trade_ids: vec!["T_HB_1", "T_HB_2"],
                trade_types: vec!["IRS", "Swaption"],
            },
            DemoCpDef {
                id: "CORP_C",
                name: "Industrial Corp C",
                rating: "BB",
                hazard_rate: 0.05,
                lgd: 0.5,
                ns_id: "NS_CORP_C",
                csa_id: "CSA_CORP_C",
                trade_ids: vec!["T_CC_1", "T_CC_2"],
                trade_types: vec!["FxForward", "IRS"],
            },
        ]
    }

    /// Builds the demo XVA portfolio hierarchy.
    fn build_demo_hierarchy() -> (XvaHierarchy, Vec<DemoCpDef>) {
        let defs = Self::demo_counterparty_defs();
        let mut hierarchy = XvaHierarchy::new();

        for def in &defs {
            let credit_params = CreditParams::new(def.hazard_rate, def.lgd)
                .expect("Demo credit params should be valid");

            let mut cp = XvaCounterparty::new(CounterpartyId::new(def.id), credit_params);

            let mut isda = IsdaAgreement::new(NettingSetId::new(def.ns_id));

            let vm_csa = VmCsa::builder()
                .threshold_self(500_000.0)
                .threshold_ctpy(500_000.0)
                .mta_self(25_000.0)
                .mta_ctpy(25_000.0)
                .mpor_days(10)
                .build();

            let mut csa_node = VmCsaNode::new(def.csa_id, vm_csa);

            for &trade_id in &def.trade_ids {
                csa_node.add_trade(TradeId::new(trade_id));
            }

            isda.add_vm_csa(csa_node);
            cp.add_isda(isda);
            hierarchy.add_counterparty(cp);
        }

        (hierarchy, defs)
    }

    /// Builds a time grid from horizon and step frequency.
    fn build_time_grid(horizon_years: f64, time_step: &str) -> Vec<f64> {
        let step = match time_step {
            "monthly" => 1.0 / 12.0,
            "semi-annual" => 0.5,
            _ => 0.25, // quarterly (default)
        };

        let n_steps = (horizon_years / step).ceil() as usize;
        (1..=n_steps)
            .map(|i| (i as f64 * step).min(horizon_years))
            .collect()
    }

    /// Builds a hierarchy summary DTO from the XVA hierarchy.
    fn build_hierarchy_summary(hierarchy: &XvaHierarchy) -> HierarchySummary {
        let mut counterparties = Vec::new();
        let mut total_netting_sets = 0_usize;
        let mut total_trades = 0_usize;

        for cp in hierarchy.counterparties() {
            let mut isda_summaries = Vec::new();

            for isda in cp.isda_agreements() {
                let mut vm_csas = Vec::new();

                for vm_csa_node in isda.vm_csas() {
                    let csa = vm_csa_node.vm_csa();
                    let trade_count = vm_csa_node.trade_ids().len();
                    total_trades += trade_count;

                    vm_csas.push(HierarchyVmCsa {
                        csa_id: vm_csa_node.csa_id().to_string(),
                        threshold_self: csa.threshold_self(),
                        threshold_ctpy: csa.threshold_ctpy(),
                        mta_self: csa.mta_self(),
                        mta_ctpy: csa.mta_ctpy(),
                        mpor_days: csa.mpor_days(),
                        trade_count,
                    });
                }

                let non_csa_count = isda.non_csa_trade_ids().len();
                total_trades += non_csa_count;
                total_netting_sets += 1;

                isda_summaries.push(HierarchyIsda {
                    netting_set_id: isda.id().as_str().to_string(),
                    vm_csas,
                    non_csa_trade_count: non_csa_count,
                });
            }

            let no_doc_count = cp.no_doc_trade_ids().len();
            total_trades += no_doc_count;

            let rating_str = cp
                .credit_params()
                .rating()
                .map(|r| format!("{:?}", r))
                .unwrap_or_else(|| "NR".to_string());

            counterparties.push(HierarchyCounterparty {
                id: cp.id().as_str().to_string(),
                credit_rating: rating_str,
                isda_agreements: isda_summaries,
                no_doc_trade_count: no_doc_count,
            });
        }

        // Sort counterparties by ID for deterministic ordering
        counterparties.sort_by(|a, b| a.id.cmp(&b.id));

        let total_counterparties = counterparties.len();

        HierarchySummary {
            counterparties,
            total_counterparties,
            total_netting_sets,
            total_trades,
        }
    }
}

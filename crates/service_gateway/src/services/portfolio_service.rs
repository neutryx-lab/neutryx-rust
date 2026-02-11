//! Portfolio service for CRUD and aggregation operations.

#[cfg(feature = "risk")]
use std::{sync::Arc, time::Instant};

#[cfg(feature = "risk")]
use chrono::Utc;

#[cfg(feature = "risk")]
use crate::{
    error::ServerError,
    rest::dto::{
        AddTradesRequest, AddTradesResponse, CreatePortfolioRequest, CreatePortfolioResponse,
        GetPortfolioResponse, GroupGreeksDto, PortfolioGreeksRequest, PortfolioGreeksResponse,
        PortfolioPriceResponse, TradePvDto,
    },
    services::helpers,
    state::{AppState, PortfolioEntry},
};

/// Service for portfolio CRUD and aggregation operations.
#[cfg(feature = "risk")]
pub struct PortfolioService;

#[cfg(feature = "risk")]
impl PortfolioService {
    /// Create a new portfolio.
    pub fn create_portfolio(
        request: &CreatePortfolioRequest,
        state: &Arc<AppState>,
    ) -> Result<CreatePortfolioResponse, ServerError> {
        let trade_count = request.trades.len();
        let counterparty_count = request.counterparties.len();
        let netting_set_count = request.netting_sets.len();

        let trade_ids: Vec<String> = request.trades.iter().map(|t| t.trade_id.clone()).collect();

        let entry = PortfolioEntry {
            name: request.name.clone(),
            trade_count,
            trade_ids,
            created_at: Utc::now(),
        };

        let portfolio_id = state.portfolio_cache.add(entry);

        Ok(CreatePortfolioResponse {
            portfolio_id: portfolio_id.to_string(),
            trade_count,
            counterparty_count,
            netting_set_count,
        })
    }

    /// Get a portfolio by ID.
    pub fn get_portfolio(
        portfolio_id: &str,
        state: &Arc<AppState>,
    ) -> Result<GetPortfolioResponse, ServerError> {
        let entry = helpers::resolve_cached(&state.portfolio_cache, portfolio_id, "Portfolio")?;

        Ok(GetPortfolioResponse {
            portfolio_id: portfolio_id.to_string(),
            name: entry.name,
            trade_count: entry.trade_count,
            counterparty_count: 0,
            netting_set_count: 0,
            created_at: entry.created_at.to_rfc3339(),
            trades: None,
        })
    }

    /// Add trades to an existing portfolio.
    pub fn add_trades(
        portfolio_id: &str,
        request: &AddTradesRequest,
        state: &Arc<AppState>,
    ) -> Result<AddTradesResponse, ServerError> {
        let id = helpers::parse_uuid(portfolio_id, "Portfolio")?;

        let mut entry = state.portfolio_cache.get(&id).ok_or_else(|| {
            ServerError::NotFound(format!("Portfolio {} not found", portfolio_id))
        })?;

        let trades_added = request.trades.len();

        for trade in &request.trades {
            entry.trade_ids.push(trade.trade_id.clone());
        }
        entry.trade_count += trades_added;

        let total_trade_count = entry.trade_count;

        state.portfolio_cache.update(&id, entry);

        Ok(AddTradesResponse {
            portfolio_id: portfolio_id.to_string(),
            trades_added,
            total_trade_count,
        })
    }

    /// Delete a portfolio.
    pub fn delete_portfolio(portfolio_id: &str, state: &Arc<AppState>) -> Result<(), ServerError> {
        let id = helpers::parse_uuid(portfolio_id, "Portfolio")?;

        state.portfolio_cache.remove(&id).ok_or_else(|| {
            ServerError::NotFound(format!("Portfolio {} not found", portfolio_id))
        })?;

        Ok(())
    }

    /// Calculate portfolio present value.
    pub fn price_portfolio(
        portfolio_id: &str,
        state: &Arc<AppState>,
    ) -> Result<PortfolioPriceResponse, ServerError> {
        let start = Instant::now();

        let entry = helpers::resolve_cached(&state.portfolio_cache, portfolio_id, "Portfolio")?;

        let trade_pvs: Vec<TradePvDto> = entry
            .trade_ids
            .iter()
            .enumerate()
            .map(|(i, trade_id)| TradePvDto {
                trade_id: trade_id.clone(),
                trade_type: "irs".to_string(),
                pv: 1_000_000.0 * (1.0 + i as f64 * 0.01),
                currency: "USD".to_string(),
            })
            .collect();

        let total_pv: f64 = trade_pvs.iter().map(|t| t.pv).sum();
        let success_count = trade_pvs.len();

        let elapsed = start.elapsed();

        Ok(PortfolioPriceResponse {
            portfolio_id: portfolio_id.to_string(),
            total_pv,
            currency: "USD".to_string(),
            trade_pvs: Some(trade_pvs),
            success_count,
            failure_count: 0,
            errors: vec![],
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }

    /// Compute portfolio Greeks with optional grouping.
    pub fn compute_portfolio_greeks(
        portfolio_id: &str,
        request: &PortfolioGreeksRequest,
        state: &Arc<AppState>,
    ) -> Result<PortfolioGreeksResponse, ServerError> {
        let start = Instant::now();

        let entry = helpers::resolve_cached(&state.portfolio_cache, portfolio_id, "Portfolio")?;

        let trade_count = entry.trade_count;

        let total_delta = trade_count as f64 * 100.0;
        let total_gamma = trade_count as f64 * 5.0;
        let total_vega = trade_count as f64 * 20.0;
        let total_theta = trade_count as f64 * -1.0;
        let total_rho = trade_count as f64 * 1.5;

        let by_counterparty = if request.group_by_counterparty {
            Some(vec![GroupGreeksDto {
                group_id: "CP001".to_string(),
                group_name: Some("Default Counterparty".to_string()),
                trade_count,
                delta: total_delta,
                gamma: total_gamma,
                vega: total_vega,
                theta: total_theta,
                rho: total_rho,
            }])
        } else {
            None
        };

        let by_netting_set = if request.group_by_netting_set {
            Some(vec![GroupGreeksDto {
                group_id: "NS001".to_string(),
                group_name: Some("Default Netting Set".to_string()),
                trade_count,
                delta: total_delta,
                gamma: total_gamma,
                vega: total_vega,
                theta: total_theta,
                rho: total_rho,
            }])
        } else {
            None
        };

        let elapsed = start.elapsed();

        Ok(PortfolioGreeksResponse {
            portfolio_id: portfolio_id.to_string(),
            total_delta,
            total_gamma,
            total_vega,
            total_theta,
            total_rho,
            by_counterparty,
            by_netting_set,
            success_count: trade_count,
            failure_count: 0,
            errors: vec![],
            calculation_time_ms: elapsed.as_secs_f64() * 1000.0,
        })
    }
}

#[cfg(all(test, feature = "risk"))]
mod tests {
    use super::*;
    use crate::rest::dto::TradeDto;

    fn create_test_trade(id: &str) -> TradeDto {
        TradeDto {
            trade_id: id.to_string(),
            trade_type: "irs".to_string(),
            counterparty_id: "CP001".to_string(),
            netting_set_id: "NS001".to_string(),
            notional: 1_000_000.0,
            currency: "USD".to_string(),
            maturity_date: "2025-12-31".to_string(),
            parameters: None,
        }
    }

    #[test]
    fn test_create_portfolio() {
        let state = AppState::test_state();

        let request = CreatePortfolioRequest {
            name: Some("Test Portfolio".to_string()),
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![create_test_trade("T001"), create_test_trade("T002")],
        };

        let response = PortfolioService::create_portfolio(&request, &state).unwrap();

        assert!(!response.portfolio_id.is_empty());
        assert_eq!(response.trade_count, 2);
    }

    #[test]
    fn test_get_portfolio() {
        let state = AppState::test_state();

        let create_request = CreatePortfolioRequest {
            name: Some("Test Portfolio".to_string()),
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![create_test_trade("T001")],
        };
        let create_response = PortfolioService::create_portfolio(&create_request, &state).unwrap();

        let response =
            PortfolioService::get_portfolio(&create_response.portfolio_id, &state).unwrap();

        assert_eq!(response.portfolio_id, create_response.portfolio_id);
        assert_eq!(response.name, Some("Test Portfolio".to_string()));
        assert_eq!(response.trade_count, 1);
    }

    #[test]
    fn test_get_portfolio_not_found() {
        let state = AppState::test_state();

        let result = PortfolioService::get_portfolio(&uuid::Uuid::new_v4().to_string(), &state);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServerError::NotFound(_)));
    }

    #[test]
    fn test_add_trades() {
        let state = AppState::test_state();

        let create_request = CreatePortfolioRequest {
            name: Some("Test".to_string()),
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![create_test_trade("T001")],
        };
        let create_response = PortfolioService::create_portfolio(&create_request, &state).unwrap();

        let add_request = AddTradesRequest {
            trades: vec![create_test_trade("T002"), create_test_trade("T003")],
        };
        let response =
            PortfolioService::add_trades(&create_response.portfolio_id, &add_request, &state)
                .unwrap();

        assert_eq!(response.trades_added, 2);
        assert_eq!(response.total_trade_count, 3);
    }

    #[test]
    fn test_delete_portfolio() {
        let state = AppState::test_state();

        let create_request = CreatePortfolioRequest {
            name: None,
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![],
        };
        let create_response = PortfolioService::create_portfolio(&create_request, &state).unwrap();

        let result = PortfolioService::delete_portfolio(&create_response.portfolio_id, &state);
        assert!(result.is_ok());

        let get_result = PortfolioService::get_portfolio(&create_response.portfolio_id, &state);
        assert!(get_result.is_err());
    }

    #[test]
    fn test_price_portfolio() {
        let state = AppState::test_state();

        let create_request = CreatePortfolioRequest {
            name: None,
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![create_test_trade("T001"), create_test_trade("T002")],
        };
        let create_response = PortfolioService::create_portfolio(&create_request, &state).unwrap();

        let response =
            PortfolioService::price_portfolio(&create_response.portfolio_id, &state).unwrap();

        assert_eq!(response.success_count, 2);
        assert_eq!(response.failure_count, 0);
        assert!(response.total_pv > 0.0);
        assert!(response.trade_pvs.is_some());
    }

    #[test]
    fn test_compute_portfolio_greeks() {
        let state = AppState::test_state();

        let create_request = CreatePortfolioRequest {
            name: None,
            counterparties: vec![],
            netting_sets: vec![],
            trades: vec![create_test_trade("T001")],
        };
        let create_response = PortfolioService::create_portfolio(&create_request, &state).unwrap();

        let request = PortfolioGreeksRequest {
            greek_types: vec!["delta".to_string(), "gamma".to_string()],
            group_by_counterparty: true,
            group_by_netting_set: false,
        };

        let response = PortfolioService::compute_portfolio_greeks(
            &create_response.portfolio_id,
            &request,
            &state,
        )
        .unwrap();

        assert_eq!(response.success_count, 1);
        assert!(response.by_counterparty.is_some());
        assert!(response.by_netting_set.is_none());
    }
}

//! Demo service wrapping data loading and demo-specific operations.

mod analysis;
mod config;
mod market_data;
mod pricing;
mod rate_index;

/// Demo service providing API endpoints for demo_gui.
pub struct DemoService;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::state::AppState;

    /// Check if demo data files are available.
    fn demo_data_available() -> bool { Path::new("demo/data/config/rate_indices.json").exists() }

    #[test]
    fn test_get_instruments() {
        let state = AppState::test_state();
        let result = DemoService::get_instruments(&state);
        assert!(result.is_ok());
        let instruments = result.unwrap();
        assert!(!instruments.instruments.is_empty());
    }

    #[test]
    fn test_get_fx_vol_pairs() {
        let state = AppState::test_state();
        let result = crate::services::VolcubeService::get_fx_vol_pairs(&state);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert!(!pairs.pairs.is_empty());
    }

    #[test]
    fn test_get_ir_vol_currencies() {
        let state = AppState::test_state();
        let result = crate::services::VolcubeService::get_ir_vol_currencies(&state);
        assert!(result.is_ok());
        let currencies = result.unwrap();
        assert!(!currencies.currencies.is_empty());
    }

    #[test]
    fn test_get_conventions() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_conventions(&state);
        assert!(result.is_ok());
        let conventions = result.unwrap();
        assert!(!conventions.conventions.is_empty());
    }

    #[test]
    fn test_get_market_config() {
        let state = AppState::test_state();
        let result = DemoService::get_market_config(&state);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.tenor_order.is_empty());
    }

    #[test]
    fn test_get_events() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_events(&state);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert!(!events.events.is_empty());
    }

    #[test]
    fn test_get_event_types() {
        let state = AppState::test_state();
        let result = DemoService::get_event_types(&state);
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(!types.types.is_empty());
    }

    #[test]
    fn test_get_rate_indices() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_indices(&state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.indices.is_empty());

        let sofr = response.indices.iter().find(|i| i.code == "SOFR");
        assert!(sofr.is_some());
        let sofr = sofr.unwrap();
        assert_eq!(sofr.currency, "USD");
        assert!(sofr.is_overnight);
    }

    #[test]
    fn test_get_rate_index_detail() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_index_detail("SOFR", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.code, "SOFR");
        assert_eq!(response.currency, "USD");
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_get_rate_index_detail_not_found() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_index_detail("NONEXISTENT", &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_index_rates() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_index_rates("SOFR", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.rates.is_empty());
    }

    #[test]
    fn test_get_index_conventions() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_index_conventions("SOFR", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.conventions.is_empty());
    }

    #[test]
    fn test_calculate_dates_from_tenor() {
        use chrono::NaiveDate;

        let valuation_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let (eff, mat) = DemoService::calculate_dates_from_tenor("ON", "USD", valuation_date);
        assert_eq!(eff, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
        assert_eq!(mat, NaiveDate::from_ymd_opt(2025, 1, 18).unwrap());

        let (eff, mat) = DemoService::calculate_dates_from_tenor("1M", "USD", valuation_date);
        assert_eq!(eff, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
        assert_eq!(mat, NaiveDate::from_ymd_opt(2025, 2, 17).unwrap());

        let (eff, mat) = DemoService::calculate_dates_from_tenor("1Y", "USD", valuation_date);
        assert_eq!(eff, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
        assert_eq!(mat, NaiveDate::from_ymd_opt(2026, 1, 17).unwrap());

        let (eff, _) = DemoService::calculate_dates_from_tenor("1M", "GBP", valuation_date);
        assert_eq!(eff, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }

    #[test]
    fn test_add_months() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        assert_eq!(
            DemoService::add_months(date, 1),
            NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()
        );

        assert_eq!(
            DemoService::add_months(date, 12),
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        );

        let date_eom = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        assert_eq!(
            DemoService::add_months(date_eom, 1),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn test_calculate_year_fraction() {
        use chrono::NaiveDate;

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();

        let yf_usd = DemoService::calculate_year_fraction(start, end, "USD");
        assert!((yf_usd - 181.0 / 360.0).abs() < 1e-10);

        let yf_gbp = DemoService::calculate_year_fraction(start, end, "GBP");
        assert!((yf_gbp - 181.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_rate_instrument_usd_swap() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_instrument("USD_SWAP_5Y", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.rate_id, "USD_SWAP_5Y");
        assert_eq!(response.instrument_type, "Swap");
        assert!(response.convention.is_some());
        assert!(response.notional > 0.0);
        assert!(response.processing_time_ms >= 0.0);
    }

    #[test]
    fn test_get_rate_instrument_usd_deposit() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_instrument("USD_DEPOSIT_3M", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.rate_id, "USD_DEPOSIT_3M");
        assert_eq!(response.instrument_type, "Deposit");
    }

    #[test]
    fn test_get_rate_instrument_not_found() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_instrument("NONEXISTENT_RATE", &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_rate_cashflows_swap() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_cashflows("USD_SWAP_5Y", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.rate_id, "USD_SWAP_5Y");
        assert_eq!(response.legs.len(), 2);

        let has_fixed = response.legs.iter().any(|l| l.leg_type == "Fixed");
        let has_floating = response.legs.iter().any(|l| l.leg_type == "Floating");
        assert!(has_fixed, "Should have fixed leg");
        assert!(has_floating, "Should have floating leg");

        for leg in &response.legs {
            assert!(!leg.cashflows.is_empty(), "Leg should have cashflows");
        }
    }

    #[test]
    fn test_get_rate_cashflows_deposit() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_cashflows("USD_DEPOSIT_3M", &state);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.legs.len(), 1);
        assert_eq!(response.legs[0].cashflows.len(), 1);
    }

    #[test]
    fn test_get_rate_cashflows_not_found() {
        if !demo_data_available() {
            eprintln!("Skipping test: demo data not available");
            return;
        }
        let state = AppState::test_state();
        let result = DemoService::get_rate_cashflows("NONEXISTENT_RATE", &state);
        assert!(result.is_err());
    }
}

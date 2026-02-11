//! Rate index listing and detail endpoints.

use std::{path::Path, sync::Arc};

use crate::{
    error::ServerError,
    rest::dto::demo::{
        IndexConventionsResponse, IndexRatesResponse, MarketRate, RateIndexDetailResponse,
        RateIndexInfo, RateIndexMetadata, RateIndicesResponse,
    },
    services::helpers,
    state::AppState,
};

use super::DemoService;

impl DemoService {
    /// Get all rate indices.
    pub fn get_rate_indices(state: &Arc<AppState>) -> Result<RateIndicesResponse, ServerError> {
        let rate_indices_path = Path::new("demo/data/config/rate_indices.json");
        let data: serde_json::Value =
            helpers::load_json_value(rate_indices_path, "rate_indices.json")?;

        let mut indices = Vec::new();

        if let Some(rate_items) = data.get("rateIndices").and_then(|i| i.as_array()) {
            let rates_response = Self::get_market_rates(state).ok();
            let conventions_response = Self::get_conventions(state).ok();

            for item in rate_items {
                let index_code = item
                    .get("indexType")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                let currency = item
                    .get("currency")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let tenor = item
                    .get("tenor")
                    .and_then(|t| t.as_str())
                    .unwrap_or("ON")
                    .to_string();
                let day_counter = item
                    .get("dayCounter")
                    .and_then(|d| d.as_str())
                    .map(String::from);
                let is_overnight = tenor == "ON";

                let associated_rates_count = rates_response
                    .as_ref()
                    .map(|r| {
                        r.rates
                            .iter()
                            .filter(|rate| {
                                rate.rate_index.as_deref() == Some(&index_code)
                                    || rate.currency == currency
                            })
                            .count()
                    })
                    .unwrap_or(0);

                let associated_conventions_count = conventions_response
                    .as_ref()
                    .map(|c| {
                        c.conventions
                            .iter()
                            .filter(|conv| conv.currency == currency)
                            .count()
                    })
                    .unwrap_or(0);

                let name = format!("{} ({})", index_code, currency);

                indices.push(RateIndexInfo {
                    code: index_code,
                    name,
                    currency,
                    tenor,
                    day_counter,
                    is_overnight,
                    associated_rates_count,
                    associated_conventions_count,
                });
            }
        }

        Ok(RateIndicesResponse { indices })
    }

    /// Get rate index detail.
    pub fn get_rate_index_detail(
        code: &str,
        state: &Arc<AppState>,
    ) -> Result<RateIndexDetailResponse, ServerError> {
        let rate_indices_path = Path::new("demo/data/config/rate_indices.json");
        let data: serde_json::Value =
            helpers::load_json_value(rate_indices_path, "rate_indices.json")?;

        let rate_items = data
            .get("rateIndices")
            .and_then(|i| i.as_array())
            .ok_or_else(|| ServerError::NotFound(format!("Index {} not found", code)))?;

        let item = rate_items
            .iter()
            .find(|i| {
                i.get("indexType")
                    .and_then(|idx| idx.as_str())
                    .map(|s| s == code)
                    .unwrap_or(false)
            })
            .ok_or_else(|| ServerError::NotFound(format!("Index {} not found", code)))?;

        let currency = item
            .get("currency")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let tenor = item
            .get("tenor")
            .and_then(|t| t.as_str())
            .unwrap_or("ON")
            .to_string();
        let name = format!("{} ({})", code, currency);

        let conventions = item.get("conventions");
        let metadata = Some(RateIndexMetadata {
            fixing_lag: conventions
                .and_then(|c| c.get("fixingLag"))
                .and_then(|f| f.as_u64())
                .map(|f| f as u32),
            settlement_lag: conventions
                .and_then(|c| c.get("settlementLag"))
                .and_then(|s| s.as_u64())
                .map(|s| s as u32),
            compounding_method: conventions
                .and_then(|c| c.get("compoundingMethod"))
                .and_then(|c| c.as_str())
                .map(String::from),
            fixing_calendar: conventions
                .and_then(|c| c.get("fixingCalendar"))
                .and_then(|c| c.as_str())
                .map(String::from),
        });

        let rates_response = Self::get_market_rates(state)?;
        let associated_rates: Vec<String> = rates_response
            .rates
            .iter()
            .filter(|rate| rate.rate_index.as_deref() == Some(code) || rate.currency == currency)
            .map(|r| r.id.clone())
            .collect();

        let conventions_response = Self::get_conventions(state)?;
        let associated_conventions: Vec<String> = conventions_response
            .conventions
            .iter()
            .filter(|conv| conv.currency == currency)
            .map(|c| c.id.clone())
            .collect();

        Ok(RateIndexDetailResponse {
            code: code.to_string(),
            name,
            currency,
            tenor,
            metadata,
            associated_rates,
            associated_conventions,
        })
    }

    /// Get rates for a rate index.
    pub fn get_index_rates(
        code: &str,
        state: &Arc<AppState>,
    ) -> Result<IndexRatesResponse, ServerError> {
        let index_detail = Self::get_rate_index_detail(code, state)?;

        let rates_response = Self::get_market_rates(state)?;
        let rates: Vec<MarketRate> = rates_response
            .rates
            .into_iter()
            .filter(|rate| {
                rate.rate_index.as_deref() == Some(code) || rate.currency == index_detail.currency
            })
            .collect();

        Ok(IndexRatesResponse { rates })
    }

    /// Get conventions for a rate index.
    pub fn get_index_conventions(
        code: &str,
        state: &Arc<AppState>,
    ) -> Result<IndexConventionsResponse, ServerError> {
        let index_detail = Self::get_rate_index_detail(code, state)?;

        let conventions_response = Self::get_conventions(state)?;
        let conventions: Vec<crate::rest::dto::demo::Convention> = conventions_response
            .conventions
            .into_iter()
            .filter(|conv| conv.currency == index_detail.currency)
            .collect();

        Ok(IndexConventionsResponse { conventions })
    }
}

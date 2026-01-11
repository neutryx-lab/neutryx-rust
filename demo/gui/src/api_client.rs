//! API client for communicating with service_gateway.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// API client for service_gateway
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

/// Portfolio request
#[derive(Debug, Serialize)]
pub struct PortfolioRequest {
    pub instruments: Vec<PriceRequest>,
    pub compute_greeks: Option<bool>,
}

/// Price request for a single instrument
#[derive(Debug, Serialize)]
pub struct PriceRequest {
    pub instrument_id: String,
    pub spot: f64,
    pub rate: f64,
    pub vol: f64,
}

/// Portfolio response
#[derive(Debug, Deserialize)]
pub struct PortfolioResponse {
    pub results: Vec<PriceResponse>,
    pub total_value: f64,
}

/// Price response for a single instrument
#[derive(Debug, Deserialize)]
pub struct PriceResponse {
    pub instrument_id: String,
    pub price: f64,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
}

/// Exposure response
#[derive(Debug, Deserialize)]
pub struct ExposureResponse {
    pub ee: f64,
    pub epe: f64,
    pub ene: f64,
    pub pfe: f64,
    pub eepe: f64,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Get portfolio pricing
    #[allow(dead_code)]
    pub async fn get_portfolio(&self, request: PortfolioRequest) -> Result<PortfolioResponse> {
        let url = format!("{}/portfolio", self.base_url);
        let response = self.client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            anyhow::bail!("API error: {}", response.status())
        }
    }

    /// Get exposure metrics
    #[allow(dead_code)]
    pub async fn get_exposure(&self) -> Result<ExposureResponse> {
        let url = format!("{}/exposure", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            anyhow::bail!("API error: {}", response.status())
        }
    }

    /// Health check
    #[allow(dead_code)]
    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://localhost:8080".to_string());
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}

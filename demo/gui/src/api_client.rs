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
    /// List of instruments to price
    pub instruments: Vec<PriceRequest>,
    /// Whether to compute Greeks
    pub compute_greeks: Option<bool>,
}

/// Price request for a single instrument
#[derive(Debug, Serialize)]
pub struct PriceRequest {
    /// Instrument identifier
    pub instrument_id: String,
    /// Spot price
    pub spot: f64,
    /// Risk-free rate
    pub rate: f64,
    /// Volatility
    pub vol: f64,
}

/// Portfolio response
#[derive(Debug, Deserialize)]
pub struct PortfolioResponse {
    /// Pricing results per instrument
    pub results: Vec<PriceResponse>,
    /// Total portfolio value
    pub total_value: f64,
}

/// Price response for a single instrument
#[derive(Debug, Deserialize)]
pub struct PriceResponse {
    /// Instrument identifier
    pub instrument_id: String,
    /// Calculated price
    pub price: f64,
    /// Delta (rate of change of price with respect to underlying)
    pub delta: Option<f64>,
    /// Gamma (rate of change of delta with respect to underlying)
    pub gamma: Option<f64>,
    /// Vega (sensitivity to volatility)
    pub vega: Option<f64>,
}

/// Exposure response
#[derive(Debug, Deserialize)]
pub struct ExposureResponse {
    /// Expected Exposure
    pub ee: f64,
    /// Expected Positive Exposure
    pub epe: f64,
    /// Expected Negative Exposure
    pub ene: f64,
    /// Potential Future Exposure
    pub pfe: f64,
    /// Effective Expected Positive Exposure
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

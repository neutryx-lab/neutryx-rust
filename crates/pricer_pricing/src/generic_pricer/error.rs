//! Error types for Generic Pricer Engine.
//!
//! This module defines structured error types for:
//! - Pricing operations (`PricingError`)
//! - Configuration validation (`ConfigError`)
//! - Market data extensions (`GenericPricerMarketError`)

use thiserror::Error;

#[cfg(feature = "l1l2-integration")]
use infra_master::market::Currency;

/// Pricing operation errors.
///
/// Provides structured error handling for pricing operations including
/// market data resolution and instrument validation.
///
/// # Variants
///
/// - `MissingMarketData`: Required market data not available
/// - `UnsupportedInstrument`: Instrument type not supported
/// - `MarketDataResolution`: Failed to resolve market data
/// - `FxRateNotFound`: FX rate not available for currency pair
/// - `InvalidTrade`: Trade structure is invalid
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PricingError {
    /// Required market data is missing.
    #[error("Missing market data: {description} (trade_id: {trade_id:?})")]
    MissingMarketData {
        /// Description of missing data.
        description: String,
        /// Optional trade ID for context.
        trade_id: Option<String>,
    },

    /// Instrument type is not supported.
    #[error("Unsupported instrument type: {instrument_type} (trade_id: {trade_id:?})")]
    UnsupportedInstrument {
        /// The unsupported instrument type.
        instrument_type: String,
        /// Optional trade ID for context.
        trade_id: Option<String>,
    },

    /// Market data resolution failed.
    #[error("Market data resolution failed: {reason}")]
    MarketDataResolution {
        /// Reason for the failure.
        reason: String,
    },

    /// FX rate not found for currency pair.
    #[cfg(feature = "l1l2-integration")]
    #[error("FX rate not found: {base}/{quote}")]
    FxRateNotFound {
        /// Base currency.
        base: Currency,
        /// Quote currency.
        quote: Currency,
    },

    /// FX rate not found for currency pair (without l1l2-integration).
    #[cfg(not(feature = "l1l2-integration"))]
    #[error("FX rate not found: {base}/{quote}")]
    FxRateNotFound {
        /// Base currency code.
        base: String,
        /// Quote currency code.
        quote: String,
    },

    /// Trade structure is invalid.
    #[error("Invalid trade: {reason} (trade_id: {trade_id:?})")]
    InvalidTrade {
        /// Reason the trade is invalid.
        reason: String,
        /// Optional trade ID for context.
        trade_id: Option<String>,
    },

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PricingError {
    /// Creates a missing market data error.
    pub fn missing_market_data(description: impl Into<String>) -> Self {
        Self::MissingMarketData {
            description: description.into(),
            trade_id: None,
        }
    }

    /// Creates a missing market data error with trade ID.
    pub fn missing_market_data_with_trade(
        description: impl Into<String>,
        trade_id: impl Into<String>,
    ) -> Self {
        Self::MissingMarketData {
            description: description.into(),
            trade_id: Some(trade_id.into()),
        }
    }

    /// Creates an unsupported instrument error.
    pub fn unsupported_instrument(instrument_type: impl Into<String>) -> Self {
        Self::UnsupportedInstrument {
            instrument_type: instrument_type.into(),
            trade_id: None,
        }
    }

    /// Creates an unsupported instrument error with trade ID.
    pub fn unsupported_instrument_with_trade(
        instrument_type: impl Into<String>,
        trade_id: impl Into<String>,
    ) -> Self {
        Self::UnsupportedInstrument {
            instrument_type: instrument_type.into(),
            trade_id: Some(trade_id.into()),
        }
    }

    /// Creates a market data resolution error.
    pub fn market_data_resolution(reason: impl Into<String>) -> Self {
        Self::MarketDataResolution {
            reason: reason.into(),
        }
    }

    /// Creates an FX rate not found error.
    #[cfg(feature = "l1l2-integration")]
    pub fn fx_rate_not_found(base: Currency, quote: Currency) -> Self {
        Self::FxRateNotFound { base, quote }
    }

    /// Creates an FX rate not found error (without l1l2-integration).
    #[cfg(not(feature = "l1l2-integration"))]
    pub fn fx_rate_not_found(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self::FxRateNotFound {
            base: base.into(),
            quote: quote.into(),
        }
    }

    /// Creates an invalid trade error.
    pub fn invalid_trade(reason: impl Into<String>) -> Self {
        Self::InvalidTrade {
            reason: reason.into(),
            trade_id: None,
        }
    }

    /// Creates an invalid trade error with trade ID.
    pub fn invalid_trade_with_id(
        reason: impl Into<String>,
        trade_id: impl Into<String>,
    ) -> Self {
        Self::InvalidTrade {
            reason: reason.into(),
            trade_id: Some(trade_id.into()),
        }
    }

    /// Creates an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Returns true if this is a market data error.
    pub fn is_market_data_error(&self) -> bool {
        matches!(
            self,
            Self::MissingMarketData { .. }
                | Self::MarketDataResolution { .. }
                | Self::FxRateNotFound { .. }
        )
    }

    /// Returns true if this is an instrument error.
    pub fn is_instrument_error(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedInstrument { .. } | Self::InvalidTrade { .. }
        )
    }
}

/// Configuration validation errors.
///
/// Errors that occur during construction when invalid parameters are provided.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigError {
    /// Invalid model parameter.
    #[error("Invalid model parameter '{name}': {reason}")]
    InvalidModelParameter {
        /// Parameter name.
        name: &'static str,
        /// Reason the parameter is invalid.
        reason: String,
    },

    /// Invalid pricer configuration.
    #[error("Invalid pricer configuration: {reason}")]
    InvalidPricerConfig {
        /// Reason the configuration is invalid.
        reason: String,
    },

    /// Required field is missing.
    #[error("Required field missing: {field}")]
    MissingField {
        /// The missing field name.
        field: &'static str,
    },
}

impl ConfigError {
    /// Creates an invalid model parameter error.
    pub fn invalid_model_parameter(name: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidModelParameter {
            name,
            reason: reason.into(),
        }
    }

    /// Creates an invalid pricer config error.
    pub fn invalid_pricer_config(reason: impl Into<String>) -> Self {
        Self::InvalidPricerConfig {
            reason: reason.into(),
        }
    }

    /// Creates a missing field error.
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingField { field }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // PricingError Tests (Task 1.1)
    // =========================================================================

    #[test]
    fn test_pricing_error_missing_market_data() {
        let err = PricingError::missing_market_data("SOFR curve not found");
        assert!(err.to_string().contains("Missing market data"));
        assert!(err.to_string().contains("SOFR curve"));
        assert!(err.is_market_data_error());
        assert!(!err.is_instrument_error());
    }

    #[test]
    fn test_pricing_error_missing_market_data_with_trade() {
        let err =
            PricingError::missing_market_data_with_trade("EUR discount curve", "TRADE-001");
        assert!(err.to_string().contains("TRADE-001"));
        assert!(err.is_market_data_error());
    }

    #[test]
    fn test_pricing_error_unsupported_instrument() {
        let err = PricingError::unsupported_instrument("ExoticBarrier");
        assert!(err.to_string().contains("Unsupported instrument"));
        assert!(err.to_string().contains("ExoticBarrier"));
        assert!(err.is_instrument_error());
        assert!(!err.is_market_data_error());
    }

    #[test]
    fn test_pricing_error_unsupported_instrument_with_trade() {
        let err =
            PricingError::unsupported_instrument_with_trade("DigitalOption", "TRADE-002");
        assert!(err.to_string().contains("TRADE-002"));
        assert!(err.is_instrument_error());
    }

    #[test]
    fn test_pricing_error_market_data_resolution() {
        let err = PricingError::market_data_resolution("Curve interpolation failed");
        assert!(err.to_string().contains("Market data resolution"));
        assert!(err.is_market_data_error());
    }

    #[test]
    fn test_pricing_error_fx_rate_not_found() {
        let err = PricingError::fx_rate_not_found("EUR", "USD");
        assert!(err.to_string().contains("FX rate not found"));
        assert!(err.to_string().contains("EUR"));
        assert!(err.to_string().contains("USD"));
        assert!(err.is_market_data_error());
    }

    #[test]
    fn test_pricing_error_invalid_trade() {
        let err = PricingError::invalid_trade("Missing notional");
        assert!(err.to_string().contains("Invalid trade"));
        assert!(err.to_string().contains("Missing notional"));
        assert!(err.is_instrument_error());
    }

    #[test]
    fn test_pricing_error_invalid_trade_with_id() {
        let err = PricingError::invalid_trade_with_id("Invalid leg structure", "TRADE-003");
        assert!(err.to_string().contains("TRADE-003"));
        assert!(err.is_instrument_error());
    }

    #[test]
    fn test_pricing_error_internal() {
        let err = PricingError::internal("Unexpected state");
        assert!(err.to_string().contains("Internal error"));
        assert!(err.to_string().contains("Unexpected state"));
    }

    #[test]
    fn test_pricing_error_clone_and_equality() {
        let err1 = PricingError::missing_market_data("test");
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_pricing_error_trait_implementation() {
        let err = PricingError::missing_market_data("test");
        let _: &dyn std::error::Error = &err;
    }

    // =========================================================================
    // ConfigError Tests (Task 1.2)
    // =========================================================================

    #[test]
    fn test_config_error_invalid_model_parameter() {
        let err = ConfigError::invalid_model_parameter("num_paths", "must be > 0");
        assert!(err.to_string().contains("Invalid model parameter"));
        assert!(err.to_string().contains("num_paths"));
        assert!(err.to_string().contains("must be > 0"));
    }

    #[test]
    fn test_config_error_invalid_pricer_config() {
        let err = ConfigError::invalid_pricer_config("Greeks mode not compatible");
        assert!(err.to_string().contains("Invalid pricer configuration"));
        assert!(err.to_string().contains("Greeks mode"));
    }

    #[test]
    fn test_config_error_missing_field() {
        let err = ConfigError::missing_field("model");
        assert!(err.to_string().contains("Required field missing"));
        assert!(err.to_string().contains("model"));
    }

    #[test]
    fn test_config_error_clone_and_equality() {
        let err1 = ConfigError::invalid_model_parameter("test", "reason");
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_config_error_trait_implementation() {
        let err = ConfigError::invalid_model_parameter("test", "reason");
        let _: &dyn std::error::Error = &err;
    }
}

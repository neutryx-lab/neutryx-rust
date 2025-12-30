//! Portfolio error types.
//!
//! This module provides structured error types for portfolio operations
//! using `thiserror` for derivation.

use thiserror::Error;

/// Errors that can occur during portfolio operations.
#[derive(Debug, Error)]
pub enum PortfolioError {
    /// Trade not found in portfolio.
    #[error("Trade not found: {0}")]
    TradeNotFound(String),

    /// Counterparty not found in portfolio.
    #[error("Counterparty not found: {0}")]
    CounterpartyNotFound(String),

    /// Netting set not found in portfolio.
    #[error("Netting set not found: {0}")]
    NettingSetNotFound(String),

    /// Duplicate trade ID encountered.
    #[error("Duplicate trade ID: {0}")]
    DuplicateTrade(String),

    /// Duplicate counterparty ID encountered.
    #[error("Duplicate counterparty ID: {0}")]
    DuplicateCounterparty(String),

    /// Duplicate netting set ID encountered.
    #[error("Duplicate netting set ID: {0}")]
    DuplicateNettingSet(String),

    /// Invalid credit parameters.
    #[error("Invalid credit parameters: {0}")]
    InvalidCreditParams(String),

    /// Invalid collateral agreement parameters.
    #[error("Invalid collateral agreement: {0}")]
    InvalidCollateralAgreement(String),

    /// Trade references an unknown counterparty.
    #[error("Trade references unknown counterparty: trade={0}, counterparty={1}")]
    UnknownCounterpartyReference(String, String),

    /// Trade references an unknown netting set.
    #[error("Trade references unknown netting set: trade={0}, netting_set={1}")]
    UnknownNettingSetReference(String, String),

    /// Netting set references an unknown counterparty.
    #[error("Netting set references unknown counterparty: netting_set={0}, counterparty={1}")]
    NettingSetUnknownCounterparty(String, String),

    /// Builder error during portfolio construction.
    #[error("Builder error: {0}")]
    BuilderError(String),

    /// Empty portfolio (no trades).
    #[error("Portfolio is empty")]
    EmptyPortfolio,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_trade_not_found() {
        let err = PortfolioError::TradeNotFound("TRADE001".to_string());
        assert_eq!(format!("{}", err), "Trade not found: TRADE001");
    }

    #[test]
    fn test_error_display_counterparty_not_found() {
        let err = PortfolioError::CounterpartyNotFound("CP001".to_string());
        assert_eq!(format!("{}", err), "Counterparty not found: CP001");
    }

    #[test]
    fn test_error_display_netting_set_not_found() {
        let err = PortfolioError::NettingSetNotFound("NS001".to_string());
        assert_eq!(format!("{}", err), "Netting set not found: NS001");
    }

    #[test]
    fn test_error_display_duplicate_trade() {
        let err = PortfolioError::DuplicateTrade("TRADE001".to_string());
        assert_eq!(format!("{}", err), "Duplicate trade ID: TRADE001");
    }

    #[test]
    fn test_error_display_invalid_credit_params() {
        let err = PortfolioError::InvalidCreditParams("LGD must be in [0, 1]".to_string());
        assert_eq!(
            format!("{}", err),
            "Invalid credit parameters: LGD must be in [0, 1]"
        );
    }

    #[test]
    fn test_error_display_unknown_counterparty_reference() {
        let err =
            PortfolioError::UnknownCounterpartyReference("T1".to_string(), "CP99".to_string());
        assert_eq!(
            format!("{}", err),
            "Trade references unknown counterparty: trade=T1, counterparty=CP99"
        );
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(PortfolioError::EmptyPortfolio);
        assert!(err.to_string().contains("empty"));
    }
}

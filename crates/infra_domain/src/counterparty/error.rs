//! CounterParty module errors.
//!
//! This module defines structured error types for the counterparty module,
//! covering validation failures for IDs, credit parameters, and margin terms.

use thiserror::Error;

/// CounterParty module errors.
///
/// Provides structured error handling for counterparty-related operations
/// including validation of IDs, LEIs, credit parameters, and margin terms.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::CounterPartyError;
///
/// let err = CounterPartyError::InvalidLei("ABC".to_string());
/// assert!(err.to_string().contains("20 alphanumeric"));
/// ```
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CounterPartyError {
    /// Invalid CounterParty ID.
    #[error("Invalid CounterParty ID: {0}")]
    InvalidCounterPartyId(String),

    /// Invalid NettingSet ID.
    #[error("Invalid NettingSet ID: {0}")]
    InvalidNettingSetId(String),

    /// Invalid LEI (must be 20 alphanumeric characters per ISO 17442).
    #[error("Invalid LEI (must be 20 alphanumeric characters): {0}")]
    InvalidLei(String),

    /// Missing CSA terms for netting set that requires them.
    #[error("Missing CSA terms for netting set")]
    MissingCsaTerms,

    /// Invalid credit rating string.
    #[error("Invalid credit rating: {0}")]
    InvalidRating(String),

    /// Invalid credit parameters (e.g., negative hazard rate, LGD out of
    /// range).
    #[error("Invalid credit parameters: {0}")]
    InvalidCreditParams(String),

    /// Invalid margin terms configuration.
    #[error("Invalid margin terms: {0}")]
    InvalidMarginTerms(String),

    /// Invalid haircut rate (must be in [0, 1]).
    #[error("Invalid haircut rate: {0} (must be in [0, 1])")]
    InvalidHaircut(f64),

    /// Counterparty mismatch in netting set or ISDA agreement.
    #[error("Counterparty mismatch: expected {expected}, got {actual}")]
    CounterpartyMismatch {
        /// The expected counterparty identifier.
        expected: String,
        /// The actual counterparty identifier encountered.
        actual: String,
    },
}

// Integration with MasterDataError
impl From<CounterPartyError> for crate::error::MasterDataError {
    fn from(e: CounterPartyError) -> Self {
        crate::error::MasterDataError::CounterParty(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_counterparty_id_error() {
        let err = CounterPartyError::InvalidCounterPartyId("".to_string());
        assert_eq!(format!("{}", err), "Invalid CounterParty ID: ");
    }

    #[test]
    fn test_invalid_netting_set_id_error() {
        let err = CounterPartyError::InvalidNettingSetId("bad-id".to_string());
        assert_eq!(format!("{}", err), "Invalid NettingSet ID: bad-id");
    }

    #[test]
    fn test_invalid_lei_error() {
        let err = CounterPartyError::InvalidLei("ABC".to_string());
        assert!(err.to_string().contains("20 alphanumeric"));
        assert!(err.to_string().contains("ABC"));
    }

    #[test]
    fn test_missing_csa_terms_error() {
        let err = CounterPartyError::MissingCsaTerms;
        assert_eq!(format!("{}", err), "Missing CSA terms for netting set");
    }

    #[test]
    fn test_invalid_rating_error() {
        let err = CounterPartyError::InvalidRating("XYZ".to_string());
        assert_eq!(format!("{}", err), "Invalid credit rating: XYZ");
    }

    #[test]
    fn test_invalid_credit_params_error() {
        let err =
            CounterPartyError::InvalidCreditParams("Hazard rate must be non-negative".to_string());
        assert!(err.to_string().contains("Hazard rate"));
    }

    #[test]
    fn test_invalid_margin_terms_error() {
        let err = CounterPartyError::InvalidMarginTerms("Invalid IM model".to_string());
        assert!(err.to_string().contains("Invalid IM model"));
    }

    #[test]
    fn test_invalid_haircut_error() {
        let err = CounterPartyError::InvalidHaircut(1.5);
        assert!(err.to_string().contains("1.5"));
        assert!(err.to_string().contains("[0, 1]"));
    }

    #[test]
    fn test_std_error_trait_implementation() {
        let err = CounterPartyError::MissingCsaTerms;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_clone_and_equality() {
        let err1 = CounterPartyError::InvalidLei("TEST".to_string());
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_debug_implementation() {
        let err = CounterPartyError::InvalidHaircut(0.5);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidHaircut"));
    }
}

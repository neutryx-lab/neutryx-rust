//! CSA (Credit Support Annex) terms and netting set configuration.
//!
//! This module re-exports types from `infra_master` for backward compatibility.
//! New code should import directly from `infra_master`.
//!
//! # Migration
//!
//! ```rust,ignore
//! // Old (deprecated):
//! use adapter_loader::{CsaTerms, NettingSetConfig};
//!
//! // New (preferred):
//! use infra_master::{CsaTerms, NettingSetConfig};
//! ```

// Re-export from infra_master for backward compatibility
pub use infra_master::{CsaTerms, NettingSetConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netting_set_config() {
        let config = NettingSetConfig::new("NS001", "CP001");
        assert_eq!(config.netting_set_id, "NS001");
        assert_eq!(config.counterparty_id, "CP001");
        assert!(config.closeout_netting);
    }
}

//! CSA (Credit Support Annex) terms and netting set configuration.

// Re-export from infra_domain::counterparty
pub use infra_domain::counterparty::{CsaTerms, NettingSet};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netting_set() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();
        assert_eq!(ns.id().as_str(), "NS001");
        assert_eq!(ns.counterparty_id().as_str(), "CP001");
        assert!(ns.has_closeout_netting());
    }

    #[test]
    fn test_csa_terms() {
        let csa = CsaTerms::builder().threshold(1_000_000.0).build();
        assert!((csa.threshold() - 1_000_000.0).abs() < f64::EPSILON);
    }
}

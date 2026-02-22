//! Three-tier portfolio hierarchy for XVA: Counterparty -> ISDA -> VM CSA.
//!
//! The XVA hierarchy models the real-world legal structure:
//! - **Counterparty** (top level): the legal entity we trade with
//! - **ISDA Master Agreement** (middle): netting agreement governing multiple trades
//! - **VM CSA** (bottom): collateral agreement under a specific ISDA
//!
//! Trades may also exist outside any ISDA ("no-doc" trades, fully fenced) or
//! under an ISDA but without a CSA ("non-CSA" trades).

use std::collections::HashMap;

use infra_domain::counterparty::VmCsa;

use crate::portfolio::{CounterpartyId, CreditParams, NettingSetId, TradeId};

/// Top-level XVA portfolio hierarchy containing all counterparties.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XvaHierarchy {
    counterparties: HashMap<CounterpartyId, XvaCounterparty>,
}

impl XvaHierarchy {
    /// Creates an empty hierarchy.
    pub fn new() -> Self {
        Self {
            counterparties: HashMap::new(),
        }
    }

    /// Adds a counterparty to the hierarchy.
    pub fn add_counterparty(&mut self, cp: XvaCounterparty) {
        self.counterparties.insert(cp.id.clone(), cp);
    }

    /// Returns a reference to a counterparty by ID.
    pub fn counterparty(&self, id: &CounterpartyId) -> Option<&XvaCounterparty> {
        self.counterparties.get(id)
    }

    /// Returns an iterator over all counterparties.
    pub fn counterparties(&self) -> impl Iterator<Item = &XvaCounterparty> {
        self.counterparties.values()
    }

    /// Returns the number of counterparties in the hierarchy.
    pub fn counterparty_count(&self) -> usize {
        self.counterparties.len()
    }

    /// Collects all trade IDs across the entire hierarchy.
    pub fn all_trade_ids(&self) -> Vec<&TradeId> {
        let mut ids = Vec::new();
        for cp in self.counterparties.values() {
            // Trades under ISDA agreements
            for isda in &cp.isda_agreements {
                ids.extend(isda.all_trade_ids());
            }
            // No-doc trades (fenced, no netting)
            for tid in &cp.no_doc_trade_ids {
                ids.push(tid);
            }
        }
        ids
    }
}

impl Default for XvaHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

/// A counterparty node in the XVA hierarchy.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XvaCounterparty {
    /// Counterparty identifier.
    pub id: CounterpartyId,
    /// Credit parameters (hazard rate, LGD, rating).
    pub credit_params: CreditParams,
    /// ISDA Master Agreements with this counterparty.
    pub isda_agreements: Vec<IsdaAgreement>,
    /// Trades without any ISDA agreement (fenced, no netting benefit).
    pub no_doc_trade_ids: Vec<TradeId>,
}

impl XvaCounterparty {
    /// Creates a new counterparty node.
    pub fn new(id: CounterpartyId, credit_params: CreditParams) -> Self {
        Self {
            id,
            credit_params,
            isda_agreements: Vec::new(),
            no_doc_trade_ids: Vec::new(),
        }
    }

    /// Returns the counterparty ID.
    pub fn id(&self) -> &CounterpartyId {
        &self.id
    }

    /// Returns the credit parameters.
    pub fn credit_params(&self) -> &CreditParams {
        &self.credit_params
    }

    /// Returns the ISDA agreements.
    pub fn isda_agreements(&self) -> &[IsdaAgreement] {
        &self.isda_agreements
    }

    /// Returns the no-doc trade IDs.
    pub fn no_doc_trade_ids(&self) -> &[TradeId] {
        &self.no_doc_trade_ids
    }

    /// Adds an ISDA Master Agreement.
    pub fn add_isda(&mut self, isda: IsdaAgreement) {
        self.isda_agreements.push(isda);
    }

    /// Adds a no-doc trade (not under any ISDA).
    pub fn add_no_doc_trade(&mut self, trade_id: TradeId) {
        self.no_doc_trade_ids.push(trade_id);
    }
}

/// An ISDA Master Agreement node in the XVA hierarchy.
///
/// An ISDA agreement defines a netting set and may contain one or more
/// VM CSA nodes as well as trades that are under the ISDA but not covered
/// by any CSA.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IsdaAgreement {
    /// The netting set ID associated with this ISDA.
    pub id: NettingSetId,
    /// VM CSA nodes under this ISDA agreement.
    pub vm_csas: Vec<VmCsaNode>,
    /// Trades under this ISDA but not covered by any CSA.
    pub non_csa_trade_ids: Vec<TradeId>,
}

impl IsdaAgreement {
    /// Creates a new ISDA agreement.
    pub fn new(id: NettingSetId) -> Self {
        Self {
            id,
            vm_csas: Vec::new(),
            non_csa_trade_ids: Vec::new(),
        }
    }

    /// Returns the netting set ID.
    pub fn id(&self) -> &NettingSetId {
        &self.id
    }

    /// Returns the VM CSA nodes.
    pub fn vm_csas(&self) -> &[VmCsaNode] {
        &self.vm_csas
    }

    /// Returns the non-CSA trade IDs.
    pub fn non_csa_trade_ids(&self) -> &[TradeId] {
        &self.non_csa_trade_ids
    }

    /// Adds a VM CSA node under this ISDA.
    pub fn add_vm_csa(&mut self, vm_csa: VmCsaNode) {
        self.vm_csas.push(vm_csa);
    }

    /// Adds a trade that is under this ISDA but not covered by any CSA.
    pub fn add_non_csa_trade(&mut self, trade_id: TradeId) {
        self.non_csa_trade_ids.push(trade_id);
    }

    /// Returns all trade IDs under this ISDA (both CSA and non-CSA trades).
    pub fn all_trade_ids(&self) -> Vec<&TradeId> {
        let mut ids = Vec::new();
        for vm_csa in &self.vm_csas {
            for tid in &vm_csa.trade_ids {
                ids.push(tid);
            }
        }
        for tid in &self.non_csa_trade_ids {
            ids.push(tid);
        }
        ids
    }
}

/// A VM CSA (Variation Margin Credit Support Annex) node in the hierarchy.
///
/// Represents a specific collateral agreement under an ISDA, containing
/// the CSA terms and the trades governed by this CSA.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VmCsaNode {
    /// CSA identifier.
    pub csa_id: String,
    /// The VM CSA terms.
    pub vm_csa: VmCsa,
    /// Trade IDs governed by this CSA.
    pub trade_ids: Vec<TradeId>,
}

impl VmCsaNode {
    /// Creates a new VM CSA node.
    pub fn new(csa_id: impl Into<String>, vm_csa: VmCsa) -> Self {
        Self {
            csa_id: csa_id.into(),
            vm_csa,
            trade_ids: Vec::new(),
        }
    }

    /// Returns the CSA ID.
    pub fn csa_id(&self) -> &str {
        &self.csa_id
    }

    /// Returns the VM CSA terms.
    pub fn vm_csa(&self) -> &VmCsa {
        &self.vm_csa
    }

    /// Returns the trade IDs.
    pub fn trade_ids(&self) -> &[TradeId] {
        &self.trade_ids
    }

    /// Adds a trade to this CSA node.
    pub fn add_trade(&mut self, trade_id: TradeId) {
        self.trade_ids.push(trade_id);
    }
}

/// Pre-computed exposure paths from external systems.
///
/// Used to merge exposure from trades priced externally (e.g., exotic
/// instruments priced in a different engine) into the XVA simulation.
///
/// The data is stored as `paths[time_idx][path_idx]`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OtherExposurePaths {
    /// Exposure paths indexed as [time_idx][path_idx].
    pub paths: Vec<Vec<f64>>,
}

impl OtherExposurePaths {
    /// Creates a new set of exposure paths initialized to zero.
    pub fn new(n_times: usize, n_paths: usize) -> Self {
        Self {
            paths: vec![vec![0.0; n_paths]; n_times],
        }
    }

    /// Gets the exposure value at a specific time and path index.
    pub fn get(&self, time_idx: usize, path_idx: usize) -> f64 {
        self.paths[time_idx][path_idx]
    }

    /// Sets the exposure value at a specific time and path index.
    pub fn set(&mut self, time_idx: usize, path_idx: usize, value: f64) {
        self.paths[time_idx][path_idx] = value;
    }

    /// Merges external exposure paths into the given netted exposure array.
    ///
    /// Adds `other.paths[t][p]` to `exposure[t][p]` for all t and p.
    pub fn add_to_exposure(&self, exposure: &mut [Vec<f64>]) {
        for (t, (exp_t, other_t)) in exposure.iter_mut().zip(self.paths.iter()).enumerate() {
            let _ = t;
            for (p, (exp_p, other_p)) in exp_t.iter_mut().zip(other_t.iter()).enumerate() {
                let _ = p;
                *exp_p += other_p;
            }
        }
    }

    /// Returns the number of time points.
    pub fn n_times(&self) -> usize {
        self.paths.len()
    }

    /// Returns the number of paths.
    pub fn n_paths(&self) -> usize {
        if self.paths.is_empty() {
            0
        } else {
            self.paths[0].len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_credit_params() -> CreditParams {
        CreditParams::new(0.02, 0.4).unwrap()
    }

    fn make_vm_csa() -> VmCsa {
        VmCsa::builder().build()
    }

    #[test]
    fn test_simple_hierarchy() {
        // 1 CP, 1 ISDA, 1 CSA
        let mut hierarchy = XvaHierarchy::new();

        let mut cp = XvaCounterparty::new(CounterpartyId::new("CP1"), make_credit_params());
        let mut isda = IsdaAgreement::new(NettingSetId::new("NS1"));
        let mut csa = VmCsaNode::new("CSA1", make_vm_csa());

        csa.add_trade(TradeId::new("T1"));
        csa.add_trade(TradeId::new("T2"));
        isda.add_vm_csa(csa);
        cp.add_isda(isda);
        hierarchy.add_counterparty(cp);

        assert_eq!(hierarchy.counterparty_count(), 1);
        let cp_ref = hierarchy.counterparty(&CounterpartyId::new("CP1")).unwrap();
        assert_eq!(cp_ref.isda_agreements().len(), 1);
        assert_eq!(cp_ref.isda_agreements()[0].vm_csas().len(), 1);
        assert_eq!(cp_ref.isda_agreements()[0].vm_csas()[0].trade_ids().len(), 2);

        let all_ids = hierarchy.all_trade_ids();
        assert_eq!(all_ids.len(), 2);
    }

    #[test]
    fn test_multi_level_hierarchy() {
        // 2 CPs, each with 2 ISDAs
        let mut hierarchy = XvaHierarchy::new();

        for cp_idx in 0..2 {
            let cp_id = CounterpartyId::new(&format!("CP{}", cp_idx));
            let mut cp = XvaCounterparty::new(cp_id, make_credit_params());

            for isda_idx in 0..2 {
                let ns_id = NettingSetId::new(&format!("NS_{}_{}", cp_idx, isda_idx));
                let mut isda = IsdaAgreement::new(ns_id);
                let mut csa = VmCsaNode::new(
                    format!("CSA_{}_{}",cp_idx, isda_idx),
                    make_vm_csa(),
                );
                csa.add_trade(TradeId::new(&format!("T_{}_{}", cp_idx, isda_idx)));
                isda.add_vm_csa(csa);
                cp.add_isda(isda);
            }

            hierarchy.add_counterparty(cp);
        }

        assert_eq!(hierarchy.counterparty_count(), 2);
        let all_ids = hierarchy.all_trade_ids();
        assert_eq!(all_ids.len(), 4);

        // Each CP should have 2 ISDAs
        for cp in hierarchy.counterparties() {
            assert_eq!(cp.isda_agreements().len(), 2);
        }
    }

    #[test]
    fn test_no_doc_trades_fenced() {
        let mut hierarchy = XvaHierarchy::new();
        let mut cp = XvaCounterparty::new(CounterpartyId::new("CP1"), make_credit_params());

        // Add a normal ISDA trade
        let mut isda = IsdaAgreement::new(NettingSetId::new("NS1"));
        let mut csa = VmCsaNode::new("CSA1", make_vm_csa());
        csa.add_trade(TradeId::new("T_ISDA"));
        isda.add_vm_csa(csa);
        cp.add_isda(isda);

        // Add no-doc trades (fenced, no netting)
        cp.add_no_doc_trade(TradeId::new("T_NODOC_1"));
        cp.add_no_doc_trade(TradeId::new("T_NODOC_2"));

        hierarchy.add_counterparty(cp);

        let cp_ref = hierarchy.counterparty(&CounterpartyId::new("CP1")).unwrap();
        assert_eq!(cp_ref.no_doc_trade_ids().len(), 2);
        assert_eq!(cp_ref.no_doc_trade_ids()[0], TradeId::new("T_NODOC_1"));
        assert_eq!(cp_ref.no_doc_trade_ids()[1], TradeId::new("T_NODOC_2"));

        // all_trade_ids should include both ISDA and no-doc trades
        let all_ids = hierarchy.all_trade_ids();
        assert_eq!(all_ids.len(), 3);
    }

    #[test]
    fn test_non_csa_trades_in_isda() {
        let mut hierarchy = XvaHierarchy::new();
        let mut cp = XvaCounterparty::new(CounterpartyId::new("CP1"), make_credit_params());

        let mut isda = IsdaAgreement::new(NettingSetId::new("NS1"));

        // Trades under CSA
        let mut csa = VmCsaNode::new("CSA1", make_vm_csa());
        csa.add_trade(TradeId::new("T_CSA"));
        isda.add_vm_csa(csa);

        // Trades under ISDA but no CSA
        isda.add_non_csa_trade(TradeId::new("T_NONCSA_1"));
        isda.add_non_csa_trade(TradeId::new("T_NONCSA_2"));

        cp.add_isda(isda);
        hierarchy.add_counterparty(cp);

        let cp_ref = hierarchy.counterparty(&CounterpartyId::new("CP1")).unwrap();
        let isda_ref = &cp_ref.isda_agreements()[0];

        assert_eq!(isda_ref.non_csa_trade_ids().len(), 2);
        assert_eq!(isda_ref.non_csa_trade_ids()[0], TradeId::new("T_NONCSA_1"));

        let isda_all = isda_ref.all_trade_ids();
        assert_eq!(isda_all.len(), 3); // 1 CSA + 2 non-CSA
    }

    #[test]
    fn test_other_exposure_paths_merge() {
        let n_times = 3;
        let n_paths = 4;

        let mut other = OtherExposurePaths::new(n_times, n_paths);
        assert_eq!(other.n_times(), n_times);
        assert_eq!(other.n_paths(), n_paths);

        // Set some values
        other.set(0, 0, 1.0);
        other.set(1, 1, 2.0);
        other.set(2, 2, 3.0);
        assert!((other.get(0, 0) - 1.0).abs() < f64::EPSILON);
        assert!((other.get(1, 1) - 2.0).abs() < f64::EPSILON);

        // Create exposure array and merge
        let mut exposure = vec![vec![10.0; n_paths]; n_times];
        other.add_to_exposure(&mut exposure);

        assert!((exposure[0][0] - 11.0).abs() < f64::EPSILON); // 10 + 1
        assert!((exposure[1][1] - 12.0).abs() < f64::EPSILON); // 10 + 2
        assert!((exposure[2][2] - 13.0).abs() < f64::EPSILON); // 10 + 3
        assert!((exposure[0][1] - 10.0).abs() < f64::EPSILON); // 10 + 0 (unchanged)
    }

    #[test]
    fn test_all_trade_ids_collects_across_hierarchy() {
        let mut hierarchy = XvaHierarchy::new();

        // CP1: ISDA with CSA trades + non-CSA trades + no-doc trades
        let mut cp1 = XvaCounterparty::new(CounterpartyId::new("CP1"), make_credit_params());
        let mut isda1 = IsdaAgreement::new(NettingSetId::new("NS1"));
        let mut csa1 = VmCsaNode::new("CSA1", make_vm_csa());
        csa1.add_trade(TradeId::new("T1"));
        csa1.add_trade(TradeId::new("T2"));
        isda1.add_vm_csa(csa1);
        isda1.add_non_csa_trade(TradeId::new("T3"));
        cp1.add_isda(isda1);
        cp1.add_no_doc_trade(TradeId::new("T4"));
        hierarchy.add_counterparty(cp1);

        // CP2: simple ISDA
        let mut cp2 = XvaCounterparty::new(CounterpartyId::new("CP2"), make_credit_params());
        let mut isda2 = IsdaAgreement::new(NettingSetId::new("NS2"));
        let mut csa2 = VmCsaNode::new("CSA2", make_vm_csa());
        csa2.add_trade(TradeId::new("T5"));
        isda2.add_vm_csa(csa2);
        cp2.add_isda(isda2);
        hierarchy.add_counterparty(cp2);

        let all_ids = hierarchy.all_trade_ids();
        assert_eq!(all_ids.len(), 5); // T1, T2, T3 (non-CSA), T4 (no-doc), T5
    }

    #[test]
    fn test_hierarchy_default() {
        let hierarchy = XvaHierarchy::default();
        assert_eq!(hierarchy.counterparty_count(), 0);
        assert!(hierarchy.all_trade_ids().is_empty());
    }

    #[test]
    fn test_vm_csa_node_accessors() {
        let csa = VmCsaNode::new("CSA_TEST", make_vm_csa());
        assert_eq!(csa.csa_id(), "CSA_TEST");
        assert!(csa.trade_ids().is_empty());
    }

    #[test]
    fn test_counterparty_accessors() {
        let cp = XvaCounterparty::new(CounterpartyId::new("CP_TEST"), make_credit_params());
        assert_eq!(cp.id().as_str(), "CP_TEST");
        assert!(cp.isda_agreements().is_empty());
        assert!(cp.no_doc_trade_ids().is_empty());
    }

    #[test]
    fn test_isda_agreement_accessors() {
        let isda = IsdaAgreement::new(NettingSetId::new("NS_TEST"));
        assert_eq!(isda.id().as_str(), "NS_TEST");
        assert!(isda.vm_csas().is_empty());
        assert!(isda.non_csa_trade_ids().is_empty());
        assert!(isda.all_trade_ids().is_empty());
    }

    #[test]
    fn test_other_exposure_paths_empty() {
        let other = OtherExposurePaths::new(0, 0);
        assert_eq!(other.n_times(), 0);
        assert_eq!(other.n_paths(), 0);
    }

    #[test]
    fn test_counterparty_iterator() {
        let mut hierarchy = XvaHierarchy::new();
        hierarchy.add_counterparty(XvaCounterparty::new(
            CounterpartyId::new("CP1"),
            make_credit_params(),
        ));
        hierarchy.add_counterparty(XvaCounterparty::new(
            CounterpartyId::new("CP2"),
            make_credit_params(),
        ));

        let count = hierarchy.counterparties().count();
        assert_eq!(count, 2);
    }
}

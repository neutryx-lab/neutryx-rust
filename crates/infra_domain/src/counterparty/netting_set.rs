//! NettingSet definitions.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::{
    Ccp, CcpId, CounterPartyError, CounterPartyId, CrossBookNettingAgreementId, CsaTerms,
    LegalEntityId, MarginTerms,
};
use crate::{ids::BookId, time::Date};

/// Netting type classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, strum::Display, serde::Serialize, serde::Deserialize)]
pub enum NettingType {
    /// Bilateral (OTC) - direct with counterparty.
    #[default]
    Bilateral,
    /// Cleared via CCP as a clearing member.
    #[strum(serialize = "Cleared (CCP)")]
    ClearedCcp,
    /// Cleared via CCP as a client of a clearing member.
    #[strum(serialize = "Cleared (Client)")]
    ClearedClient,
}

impl NettingType {
    /// Returns whether this is a cleared transaction.
    pub fn is_cleared(&self) -> bool {
        matches!(self, NettingType::ClearedCcp | NettingType::ClearedClient)
    }
}

/// NettingSet entity.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct NettingSet {
    netting_set_id: NettingSetId,
    counterparty_id: CounterPartyId,
    legal_entity_id: Option<LegalEntityId>,
    netting_type: NettingType,
    closeout_netting: bool,
    csa_terms: Option<CsaTerms>,
    margin_terms: Option<MarginTerms>,
    ccp_id: Option<CcpId>,
    /// Book IDs allowed for cross-book netting within this set.
    book_ids: Vec<BookId>,
}

impl NettingSet {
    /// Creates a new NettingSet builder.
    pub fn builder(
        id: impl Into<NettingSetId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> NettingSetBuilder {
        NettingSetBuilder::new(id, counterparty_id)
    }

    /// Returns the netting set ID.
    pub fn id(&self) -> &NettingSetId { &self.netting_set_id }

    /// Returns the counterparty ID.
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the legal entity ID if set.
    pub fn legal_entity_id(&self) -> Option<&LegalEntityId> { self.legal_entity_id.as_ref() }

    /// Returns the netting type.
    pub fn netting_type(&self) -> NettingType { self.netting_type }

    /// Returns whether close-out netting applies.
    pub fn has_closeout_netting(&self) -> bool { self.closeout_netting }

    /// Returns the CSA terms if set.
    pub fn csa_terms(&self) -> Option<&CsaTerms> { self.csa_terms.as_ref() }

    /// Returns the margin terms if set.
    pub fn margin_terms(&self) -> Option<&MarginTerms> { self.margin_terms.as_ref() }

    /// Returns the CCP ID if this is a cleared transaction.
    pub fn ccp_id(&self) -> Option<&CcpId> { self.ccp_id.as_ref() }

    /// Returns the book IDs allowed for cross-book netting.
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns true if cross-book netting is enabled (multiple books.
    pub fn allows_cross_book_netting(&self) -> bool { self.book_ids.len() > 1 }

    /// Returns true if the specified book is allowed in this netting set.
    pub fn allows_book(&self, book_id: &BookId) -> bool {
        self.book_ids.is_empty() || self.book_ids.contains(book_id)
    }

    /// Returns whether this is a cleared transaction.
    pub fn is_cleared(&self) -> bool { self.netting_type.is_cleared() }

    /// Returns whether this has collateral (CSA terms set).
    pub fn is_collateralised(&self) -> bool { self.csa_terms.is_some() }

    /// Returns the MPOR (Margin Period of Risk) in business days.
    pub fn mpor_days(&self) -> u32 {
        match self.netting_type {
            NettingType::ClearedCcp | NettingType::ClearedClient => Ccp::CLEARED_MPOR_DAYS,
            NettingType::Bilateral => self.csa_terms.as_ref().map(|c| c.mpor_days()).unwrap_or(10),
        }
    }
}

/// Type-safe NettingSet identifier (re-exported from ids module).
pub use super::ids::NettingSetId;

/// Builder for [`NettingSet`].
pub struct NettingSetBuilder {
    netting_set_id: NettingSetId,
    counterparty_id: CounterPartyId,
    legal_entity_id: Option<LegalEntityId>,
    netting_type: NettingType,
    closeout_netting: bool,
    csa_terms: Option<CsaTerms>,
    margin_terms: Option<MarginTerms>,
    ccp_id: Option<CcpId>,
    book_ids: Vec<BookId>,
}

impl NettingSetBuilder {
    /// Creates a new builder with required fields.
    pub fn new(id: impl Into<NettingSetId>, counterparty_id: impl Into<CounterPartyId>) -> Self {
        Self {
            netting_set_id: id.into(),
            counterparty_id: counterparty_id.into(),
            legal_entity_id: None,
            netting_type: NettingType::default(),
            closeout_netting: true,
            csa_terms: None,
            margin_terms: None,
            ccp_id: None,
            book_ids: Vec::new(),
        }
    }

    /// Sets the legal entity ID.
    pub fn legal_entity_id(mut self, lei: LegalEntityId) -> Self {
        self.legal_entity_id = Some(lei);
        self
    }

    /// Sets the netting type.
    pub fn netting_type(mut self, t: NettingType) -> Self {
        self.netting_type = t;
        self
    }

    /// Sets whether close-out netting applies.
    pub fn closeout_netting(mut self, v: bool) -> Self {
        self.closeout_netting = v;
        self
    }

    /// Sets the CSA terms.
    pub fn csa_terms(mut self, terms: CsaTerms) -> Self {
        self.csa_terms = Some(terms);
        self
    }

    /// Sets the margin terms.
    pub fn margin_terms(mut self, terms: MarginTerms) -> Self {
        self.margin_terms = Some(terms);
        self
    }

    /// Sets the CCP ID (for cleared transactions).
    pub fn ccp_id(mut self, id: impl Into<CcpId>) -> Self {
        self.ccp_id = Some(id.into());
        self
    }

    /// Adds a book ID to the allowed books list.
    pub fn add_book(mut self, book_id: impl Into<BookId>) -> Self {
        let id = book_id.into();
        if !self.book_ids.contains(&id) {
            self.book_ids.push(id);
        }
        self
    }

    /// Sets the allowed book IDs for cross-book netting.
    pub fn book_ids(mut self, book_ids: impl IntoIterator<Item = impl Into<BookId>>) -> Self {
        self.book_ids = book_ids.into_iter().map(Into::into).collect();
        self
    }

    /// Builds the NettingSet.
    pub fn build(self) -> Result<NettingSet, CounterPartyError> {
        Ok(NettingSet {
            netting_set_id: self.netting_set_id,
            counterparty_id: self.counterparty_id,
            legal_entity_id: self.legal_entity_id,
            netting_type: self.netting_type,
            closeout_netting: self.closeout_netting,
            csa_terms: self.csa_terms,
            margin_terms: self.margin_terms,
            ccp_id: self.ccp_id,
            book_ids: self.book_ids,
        })
    }
}

/// Cross-book netting agreement.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CrossBookNettingAgreement {
    id: CrossBookNettingAgreementId,
    counterparty_id: CounterPartyId,
    book_ids: Vec<BookId>,
    eligible_products: Vec<String>,
    effective_date: Option<Date>,
    termination_date: Option<Date>,
    description: Option<String>,
}

impl CrossBookNettingAgreement {
    /// Creates a new builder.
    pub fn builder(
        id: impl Into<CrossBookNettingAgreementId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> CrossBookNettingAgreementBuilder {
        CrossBookNettingAgreementBuilder::new(id, counterparty_id)
    }

    /// Returns the agreement ID.
    pub fn id(&self) -> &CrossBookNettingAgreementId { &self.id }

    /// Returns the counterparty ID.
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the book IDs in this agreement.
    pub fn book_ids(&self) -> &[BookId] { &self.book_ids }

    /// Returns the eligible products (empty = all products eligible).
    pub fn eligible_products(&self) -> &[String] { &self.eligible_products }

    /// Returns the effective date.
    pub fn effective_date(&self) -> Option<Date> { self.effective_date }

    /// Returns the termination date.
    pub fn termination_date(&self) -> Option<Date> { self.termination_date }

    /// Returns the description.
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }

    /// Returns true if the specified book is included in this agreement.
    pub fn is_book_eligible(&self, book_id: &BookId) -> bool { self.book_ids.contains(book_id) }

    /// Returns true if the specified product is eligible for cross-book.
    pub fn is_product_eligible(&self, product: &str) -> bool {
        self.eligible_products.is_empty() || self.eligible_products.iter().any(|p| p == product)
    }

    /// Returns the number of books in this agreement.
    pub fn book_count(&self) -> usize { self.book_ids.len() }
}

/// Builder for [`CrossBookNettingAgreement`].
pub struct CrossBookNettingAgreementBuilder {
    id: CrossBookNettingAgreementId,
    counterparty_id: CounterPartyId,
    book_ids: Vec<BookId>,
    eligible_products: Vec<String>,
    effective_date: Option<Date>,
    termination_date: Option<Date>,
    description: Option<String>,
}

impl CrossBookNettingAgreementBuilder {
    /// Creates a new builder with required fields.
    pub fn new(
        id: impl Into<CrossBookNettingAgreementId>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> Self {
        Self {
            id: id.into(),
            counterparty_id: counterparty_id.into(),
            book_ids: Vec::new(),
            eligible_products: Vec::new(),
            effective_date: None,
            termination_date: None,
            description: None,
        }
    }

    /// Adds a book to the agreement.
    pub fn add_book(mut self, book_id: impl Into<BookId>) -> Self {
        let id = book_id.into();
        if !self.book_ids.contains(&id) {
            self.book_ids.push(id);
        }
        self
    }

    /// Sets the book IDs.
    pub fn book_ids(mut self, book_ids: impl IntoIterator<Item = impl Into<BookId>>) -> Self {
        self.book_ids = book_ids.into_iter().map(Into::into).collect();
        self
    }

    /// Adds an eligible product type.
    pub fn add_eligible_product(mut self, product: impl Into<String>) -> Self {
        let p = product.into();
        if !self.eligible_products.contains(&p) {
            self.eligible_products.push(p);
        }
        self
    }

    /// Sets the eligible products.
    pub fn eligible_products(
        mut self,
        products: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.eligible_products = products.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the effective date.
    pub fn effective_date(mut self, date: Date) -> Self {
        self.effective_date = Some(date);
        self
    }

    /// Sets the termination date.
    pub fn termination_date(mut self, date: Date) -> Self {
        self.termination_date = Some(date);
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builds the CrossBookNettingAgreement.
    pub fn build(self) -> Result<CrossBookNettingAgreement, CounterPartyError> {
        if self.book_ids.len() < 2 {
            return Err(CounterPartyError::InvalidNettingSetId(format!(
                "Cross-book netting agreement requires at least 2 books, got {}",
                self.book_ids.len()
            )));
        }

        Ok(CrossBookNettingAgreement {
            id: self.id,
            counterparty_id: self.counterparty_id,
            book_ids: self.book_ids,
            eligible_products: self.eligible_products,
            effective_date: self.effective_date,
            termination_date: self.termination_date,
            description: self.description,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::Currency;

    #[test]
    fn test_netting_type_is_cleared() {
        assert!(!NettingType::Bilateral.is_cleared());
        assert!(NettingType::ClearedCcp.is_cleared());
        assert!(NettingType::ClearedClient.is_cleared());
    }

    #[test]
    fn test_netting_set_builder_minimal() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();
        assert_eq!(ns.id().as_str(), "NS001");
        assert_eq!(ns.counterparty_id().as_str(), "CP001");
        assert_eq!(ns.netting_type(), NettingType::Bilateral);
        assert!(ns.has_closeout_netting());
        assert!(ns.csa_terms().is_none());
    }

    #[test]
    fn test_netting_set_builder_full() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let csa = CsaTerms::builder()
            .threshold(1_000_000.0)
            .mpor_days(14)
            .margin_currency(Currency::EUR)
            .build();
        let margin = MarginTerms::vm_only(super::super::VmTerms::default());

        let ns = NettingSet::builder("NS001", "CP001")
            .legal_entity_id(lei)
            .netting_type(NettingType::Bilateral)
            .closeout_netting(true)
            .csa_terms(csa)
            .margin_terms(margin)
            .build()
            .unwrap();

        assert!(ns.legal_entity_id().is_some());
        assert!(ns.csa_terms().is_some());
        assert!(ns.margin_terms().is_some());
    }

    #[test]
    fn test_netting_set_cleared_ccp() {
        let ns = NettingSet::builder("NS001", "CP001")
            .netting_type(NettingType::ClearedCcp)
            .ccp_id("LCH")
            .build()
            .unwrap();

        assert!(ns.is_cleared());
        assert_eq!(ns.ccp_id().map(|c| c.as_str()), Some("LCH"));
        assert_eq!(ns.mpor_days(), 5);
    }

    #[test]
    fn test_netting_set_mpor_days() {
        let ns = NettingSet::builder("NS001", "CP001").build().unwrap();
        assert_eq!(ns.mpor_days(), 10);

        let csa = CsaTerms::builder().mpor_days(14).build();
        let ns = NettingSet::builder("NS002", "CP001")
            .csa_terms(csa)
            .build()
            .unwrap();
        assert_eq!(ns.mpor_days(), 14);
    }

    #[test]
    fn test_netting_set_book_ids() {
        let ns = NettingSet::builder("NS001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .add_book("B001")
            .build()
            .unwrap();

        assert_eq!(ns.book_ids().len(), 2);
        assert!(ns.allows_cross_book_netting());
        assert!(ns.allows_book(&BookId::new("B001")));
        assert!(!ns.allows_book(&BookId::new("B003")));
    }

    #[test]
    fn test_cross_book_netting_agreement() {
        let agreement = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .add_book("B002")
            .add_eligible_product("IRS")
            .build()
            .unwrap();

        assert_eq!(agreement.id().as_str(), "CBNA001");
        assert!(agreement.is_book_eligible(&BookId::new("B001")));
        assert!(agreement.is_product_eligible("IRS"));
        assert!(!agreement.is_product_eligible("FX"));
    }

    #[test]
    fn test_cross_book_netting_agreement_requires_multiple_books() {
        let result = CrossBookNettingAgreement::builder("CBNA001", "CP001")
            .add_book("B001")
            .build();
        assert!(result.is_err());
    }
}

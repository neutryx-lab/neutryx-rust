//! Index mapping for PricingKernel IR.
//!
//! This module provides `IndexMapper` which converts rate indices,
//! currencies, and curves to numeric IDs for the SoA kernel format.

use std::collections::HashMap;

use infra_master::{Currency, RateIndex};
use pricer_core::ir::CompileError;

/// Maps rate indices, currencies, and curves to numeric IDs.
///
/// `IndexMapper` maintains bidirectional mappings between:
/// - `RateIndex` ↔ `u16` forward index IDs
/// - `Currency` ↔ `u8` currency IDs
/// - Curve names ↔ `u8` discount curve IDs
///
/// # ID Conventions
///
/// - `fwd_index_id = 0`: Dummy index returning 0.0 (for fixed legs)
/// - `fx_index_id = 0`: Dummy FX returning 1.0 (for single currency)
/// - `currency_id = 0`: Base currency (typically USD)
///
/// # Examples
///
/// ```
/// use pricer_models::compiler::IndexMapper;
/// use infra_master::{RateIndex, Currency};
///
/// let mut mapper = IndexMapper::new();
///
/// // Register indices
/// let sofr_id = mapper.register_forward_index(RateIndex::Sofr);
/// assert_eq!(sofr_id, 1); // 0 is reserved for dummy
///
/// // Register currencies
/// let usd_id = mapper.register_currency(Currency::USD);
/// assert_eq!(usd_id, 0); // First currency is base (0)
/// ```
#[derive(Debug, Clone, Default)]
pub struct IndexMapper {
    // Forward index mapping (RateIndex → u16)
    fwd_index_to_id: HashMap<RateIndex, u16>,
    id_to_fwd_index: Vec<Option<RateIndex>>,

    // Currency mapping (Currency → u8)
    currency_to_id: HashMap<Currency, u8>,
    id_to_currency: Vec<Currency>,

    // Discount curve mapping (String → u8)
    discount_curve_to_id: HashMap<String, u8>,
    id_to_discount_curve: Vec<String>,
}

impl IndexMapper {
    /// Creates a new empty `IndexMapper`.
    ///
    /// The mapper is initialised with:
    /// - ID 0 reserved for dummy forward index
    /// - No currencies registered (first registered becomes base)
    #[must_use]
    pub fn new() -> Self {
        Self {
            fwd_index_to_id: HashMap::new(),
            id_to_fwd_index: vec![None], // 0 = dummy
            currency_to_id: HashMap::new(),
            id_to_currency: Vec::new(),
            discount_curve_to_id: HashMap::new(),
            id_to_discount_curve: Vec::new(),
        }
    }

    /// Creates a mapper pre-configured with common rate indices.
    ///
    /// Pre-registers: SOFR, ESTR, SONIA, TONAR, SARON, EURIBOR3M, EURIBOR6M
    #[must_use]
    pub fn with_common_indices() -> Self {
        let mut mapper = Self::new();

        // Register common overnight rates
        mapper.register_forward_index(RateIndex::Sofr);
        mapper.register_forward_index(RateIndex::Estr);
        mapper.register_forward_index(RateIndex::Sonia);
        mapper.register_forward_index(RateIndex::Tonar);
        mapper.register_forward_index(RateIndex::Saron);

        // Register common IBOR indices
        mapper.register_forward_index(RateIndex::Euribor3M);
        mapper.register_forward_index(RateIndex::Euribor6M);

        mapper
    }

    // =========================================================================
    // Forward Index Methods
    // =========================================================================

    /// Registers a forward rate index and returns its ID.
    ///
    /// If the index is already registered, returns the existing ID.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to register
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this index (1+, 0 is reserved).
    pub fn register_forward_index(&mut self, index: RateIndex) -> u16 {
        if let Some(&id) = self.fwd_index_to_id.get(&index) {
            return id;
        }

        let id = self.id_to_fwd_index.len() as u16;
        self.fwd_index_to_id.insert(index, id);
        self.id_to_fwd_index.push(Some(index));
        id
    }

    /// Gets the ID for a forward rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to look up
    ///
    /// # Returns
    ///
    /// * `Some(id)` - The index's ID
    /// * `None` - Index not registered
    #[must_use]
    pub fn get_forward_index_id(&self, index: RateIndex) -> Option<u16> {
        self.fwd_index_to_id.get(&index).copied()
    }

    /// Gets or registers a forward rate index.
    ///
    /// If not registered, registers it first.
    pub fn get_or_register_forward_index(&mut self, index: RateIndex) -> u16 {
        if let Some(&id) = self.fwd_index_to_id.get(&index) {
            id
        } else {
            self.register_forward_index(index)
        }
    }

    /// Gets the rate index for a given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to look up
    ///
    /// # Returns
    ///
    /// * `Some(RateIndex)` - The index (None for ID 0 = dummy)
    /// * `None` - ID out of range
    #[must_use]
    pub fn get_forward_index(&self, id: u16) -> Option<Option<RateIndex>> {
        self.id_to_fwd_index.get(id as usize).copied()
    }

    /// Returns the number of registered forward indices (excluding dummy).
    #[must_use]
    pub fn forward_index_count(&self) -> usize {
        self.id_to_fwd_index.len() - 1 // Subtract dummy
    }

    /// Returns the ID for a fixed leg (dummy index returning 0.0).
    #[must_use]
    pub const fn fixed_leg_index_id(&self) -> u16 {
        0 // Dummy index
    }

    // =========================================================================
    // Currency Methods
    // =========================================================================

    /// Registers a currency and returns its ID.
    ///
    /// If the currency is already registered, returns the existing ID.
    /// The first registered currency becomes the base currency (ID 0).
    ///
    /// # Arguments
    ///
    /// * `currency` - The currency to register
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this currency.
    pub fn register_currency(&mut self, currency: Currency) -> u8 {
        if let Some(&id) = self.currency_to_id.get(&currency) {
            return id;
        }

        let id = self.id_to_currency.len() as u8;
        self.currency_to_id.insert(currency, id);
        self.id_to_currency.push(currency);
        id
    }

    /// Gets the ID for a currency.
    #[must_use]
    pub fn get_currency_id(&self, currency: Currency) -> Option<u8> {
        self.currency_to_id.get(&currency).copied()
    }

    /// Gets or registers a currency.
    pub fn get_or_register_currency(&mut self, currency: Currency) -> u8 {
        if let Some(&id) = self.currency_to_id.get(&currency) {
            id
        } else {
            self.register_currency(currency)
        }
    }

    /// Gets the currency for a given ID.
    #[must_use]
    pub fn get_currency(&self, id: u8) -> Option<Currency> {
        self.id_to_currency.get(id as usize).copied()
    }

    /// Returns the number of registered currencies.
    #[must_use]
    pub fn currency_count(&self) -> usize { self.id_to_currency.len() }

    // =========================================================================
    // Discount Curve Methods
    // =========================================================================

    /// Registers a discount curve and returns its ID.
    ///
    /// # Arguments
    ///
    /// * `curve_name` - The curve name to register
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this curve.
    pub fn register_discount_curve(&mut self, curve_name: impl Into<String>) -> u8 {
        let name = curve_name.into();
        if let Some(&id) = self.discount_curve_to_id.get(&name) {
            return id;
        }

        let id = self.id_to_discount_curve.len() as u8;
        self.discount_curve_to_id.insert(name.clone(), id);
        self.id_to_discount_curve.push(name);
        id
    }

    /// Gets the ID for a discount curve.
    #[must_use]
    pub fn get_discount_curve_id(&self, curve_name: &str) -> Option<u8> {
        self.discount_curve_to_id.get(curve_name).copied()
    }

    /// Gets or registers a discount curve.
    pub fn get_or_register_discount_curve(&mut self, curve_name: impl Into<String>) -> u8 {
        let name = curve_name.into();
        if let Some(&id) = self.discount_curve_to_id.get(&name) {
            id
        } else {
            self.register_discount_curve(name)
        }
    }

    /// Gets the curve name for a given ID.
    #[must_use]
    pub fn get_discount_curve(&self, id: u8) -> Option<&str> {
        self.id_to_discount_curve
            .get(id as usize)
            .map(String::as_str)
    }

    /// Returns the number of registered discount curves.
    #[must_use]
    pub fn discount_curve_count(&self) -> usize { self.id_to_discount_curve.len() }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validates that a forward index ID is valid.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::UnknownIndex` if the ID is out of range.
    pub fn validate_forward_index_id(&self, id: u16) -> Result<(), CompileError> {
        if (id as usize) < self.id_to_fwd_index.len() {
            Ok(())
        } else {
            Err(CompileError::unknown_index(format!(
                "Invalid forward index ID: {id}"
            )))
        }
    }

    /// Validates that a currency ID is valid.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::UnknownCurrency` if the ID is out of range.
    pub fn validate_currency_id(&self, id: u8) -> Result<(), CompileError> {
        if (id as usize) < self.id_to_currency.len() {
            Ok(())
        } else {
            Err(CompileError::UnknownCurrency(format!(
                "Invalid currency ID: {id}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_mapper_new() {
        let mapper = IndexMapper::new();
        assert_eq!(mapper.forward_index_count(), 0);
        assert_eq!(mapper.currency_count(), 0);
        assert_eq!(mapper.discount_curve_count(), 0);
    }

    #[test]
    fn test_register_forward_index() {
        let mut mapper = IndexMapper::new();

        let sofr_id = mapper.register_forward_index(RateIndex::Sofr);
        assert_eq!(sofr_id, 1); // 0 is reserved for dummy

        let estr_id = mapper.register_forward_index(RateIndex::Estr);
        assert_eq!(estr_id, 2);

        // Re-registering returns same ID
        let sofr_id2 = mapper.register_forward_index(RateIndex::Sofr);
        assert_eq!(sofr_id2, 1);

        assert_eq!(mapper.forward_index_count(), 2);
    }

    #[test]
    fn test_get_forward_index_id() {
        let mut mapper = IndexMapper::new();
        mapper.register_forward_index(RateIndex::Sofr);

        assert_eq!(mapper.get_forward_index_id(RateIndex::Sofr), Some(1));
        assert_eq!(mapper.get_forward_index_id(RateIndex::Estr), None);
    }

    #[test]
    fn test_get_forward_index() {
        let mut mapper = IndexMapper::new();
        mapper.register_forward_index(RateIndex::Sofr);

        // ID 0 is dummy (None)
        assert_eq!(mapper.get_forward_index(0), Some(None));

        // ID 1 is SOFR
        assert_eq!(mapper.get_forward_index(1), Some(Some(RateIndex::Sofr)));

        // ID 2 is out of range
        assert_eq!(mapper.get_forward_index(2), None);
    }

    #[test]
    fn test_fixed_leg_index_id() {
        let mapper = IndexMapper::new();
        assert_eq!(mapper.fixed_leg_index_id(), 0);
    }

    #[test]
    fn test_register_currency() {
        let mut mapper = IndexMapper::new();

        let usd_id = mapper.register_currency(Currency::USD);
        assert_eq!(usd_id, 0); // First is base

        let eur_id = mapper.register_currency(Currency::EUR);
        assert_eq!(eur_id, 1);

        // Re-registering returns same ID
        let usd_id2 = mapper.register_currency(Currency::USD);
        assert_eq!(usd_id2, 0);

        assert_eq!(mapper.currency_count(), 2);
    }

    #[test]
    fn test_get_currency() {
        let mut mapper = IndexMapper::new();
        mapper.register_currency(Currency::USD);
        mapper.register_currency(Currency::EUR);

        assert_eq!(mapper.get_currency(0), Some(Currency::USD));
        assert_eq!(mapper.get_currency(1), Some(Currency::EUR));
        assert_eq!(mapper.get_currency(2), None);
    }

    #[test]
    fn test_register_discount_curve() {
        let mut mapper = IndexMapper::new();

        let ois_id = mapper.register_discount_curve("OIS");
        assert_eq!(ois_id, 0);

        let sofr_id = mapper.register_discount_curve("SOFR");
        assert_eq!(sofr_id, 1);

        // Re-registering returns same ID
        let ois_id2 = mapper.register_discount_curve("OIS");
        assert_eq!(ois_id2, 0);

        assert_eq!(mapper.discount_curve_count(), 2);
    }

    #[test]
    fn test_get_discount_curve() {
        let mut mapper = IndexMapper::new();
        mapper.register_discount_curve("OIS");

        assert_eq!(mapper.get_discount_curve(0), Some("OIS"));
        assert_eq!(mapper.get_discount_curve(1), None);
    }

    #[test]
    fn test_with_common_indices() {
        let mapper = IndexMapper::with_common_indices();

        // Should have 7 common indices registered
        assert_eq!(mapper.forward_index_count(), 7);

        // All common indices should be registered
        assert!(mapper.get_forward_index_id(RateIndex::Sofr).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Estr).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Sonia).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Tonar).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Saron).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Euribor3M).is_some());
        assert!(mapper.get_forward_index_id(RateIndex::Euribor6M).is_some());
    }

    #[test]
    fn test_get_or_register_forward_index() {
        let mut mapper = IndexMapper::new();

        // First call registers
        let id1 = mapper.get_or_register_forward_index(RateIndex::Sofr);
        assert_eq!(id1, 1);

        // Second call returns existing
        let id2 = mapper.get_or_register_forward_index(RateIndex::Sofr);
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_get_or_register_currency() {
        let mut mapper = IndexMapper::new();

        let id1 = mapper.get_or_register_currency(Currency::USD);
        let id2 = mapper.get_or_register_currency(Currency::USD);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_get_or_register_discount_curve() {
        let mut mapper = IndexMapper::new();

        let id1 = mapper.get_or_register_discount_curve("OIS");
        let id2 = mapper.get_or_register_discount_curve("OIS");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_validate_forward_index_id() {
        let mut mapper = IndexMapper::new();
        mapper.register_forward_index(RateIndex::Sofr);

        assert!(mapper.validate_forward_index_id(0).is_ok()); // Dummy
        assert!(mapper.validate_forward_index_id(1).is_ok()); // SOFR
        assert!(mapper.validate_forward_index_id(2).is_err()); // Invalid
    }

    #[test]
    fn test_validate_currency_id() {
        let mut mapper = IndexMapper::new();
        mapper.register_currency(Currency::USD);

        assert!(mapper.validate_currency_id(0).is_ok());
        assert!(mapper.validate_currency_id(1).is_err());
    }

    #[test]
    fn test_index_mapper_clone() {
        let mut mapper = IndexMapper::new();
        mapper.register_forward_index(RateIndex::Sofr);
        mapper.register_currency(Currency::USD);

        let cloned = mapper.clone();
        assert_eq!(cloned.forward_index_count(), 1);
        assert_eq!(cloned.currency_count(), 1);
    }
}

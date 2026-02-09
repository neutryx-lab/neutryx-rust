//! Index mapping for PricingKernel IR.
//!
//! This module provides `IndexMapper` which converts rate indices,
//! currencies, and curves to numeric IDs for the SoA kernel format.
//!
//! # CMS Index Support
//!
//! CMS (Constant Maturity Swap) indices are supported via [`CmsIndex`].
//! CMS indices are registered in the same ID space as forward indices,
//! allowing unified processing in the pricing kernel. The market data
//! provider is responsible for applying convexity adjustments when
//! returning rates for CMS index IDs.
//!
//! ```
//! use pricer_models::compiler::{IndexMapper, CmsIndex};
//! use infra_domain::time::Tenor;
//! use infra_domain::market::Currency;
//!
//! let mut mapper = IndexMapper::new();
//!
//! // Register a CMS index (10Y USD swap rate)
//! let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
//! let cms_id = mapper.register_cms_index(cms10y);
//!
//! // CMS indices share the same ID space as forward indices
//! assert!(cms_id > 0); // 0 is reserved for dummy
//! ```

use std::collections::HashMap;

use infra_domain::{
    market::{Currency, RateIndex},
    time::Tenor,
};
use pricer_core::{kernel::CompileError, types::FxPair};

/// CMS (Constant Maturity Swap) index definition.
///
/// Represents a CMS rate index for a specific currency and swap tenor.
/// CMS rates require convexity adjustment when used in pricing, which
/// is handled transparently by the `CurveProvider`.
///
/// # Examples
///
/// ```
/// use pricer_models::compiler::CmsIndex;
/// use infra_domain::{time::Tenor, market::Currency};
///
/// // 10Y USD CMS rate
/// let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
/// assert_eq!(cms10y.currency(), Currency::USD);
/// assert_eq!(cms10y.swap_tenor(), Tenor::TenYears);
///
/// // 5Y EUR CMS rate
/// let cms5y = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
/// assert!(cms5y.requires_convexity_adjustment());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CmsIndex {
    /// Currency of the underlying swap.
    currency: Currency,
    /// Tenor of the underlying swap rate.
    swap_tenor: Tenor,
}

impl CmsIndex {
    /// Creates a new CMS index.
    ///
    /// # Arguments
    ///
    /// * `currency` - Currency of the underlying swap
    /// * `swap_tenor` - Tenor of the underlying swap rate (e.g., 10Y)
    #[must_use]
    pub const fn new(currency: Currency, swap_tenor: Tenor) -> Self {
        Self {
            currency,
            swap_tenor,
        }
    }

    /// Returns the currency of the underlying swap.
    #[must_use]
    pub const fn currency(&self) -> Currency { self.currency }

    /// Returns the swap tenor for this CMS index.
    #[must_use]
    pub const fn swap_tenor(&self) -> Tenor { self.swap_tenor }

    /// Returns true (CMS rates always require convexity adjustment).
    #[must_use]
    pub const fn requires_convexity_adjustment(&self) -> bool { true }

    /// Returns a display name for this CMS index.
    #[must_use]
    pub fn name(&self) -> String { format!("CMS-{}-{}", self.currency, self.swap_tenor) }
}

impl std::fmt::Display for CmsIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CMS-{}-{}", self.currency, self.swap_tenor)
    }
}

/// Forward index type: either a standard rate index or a CMS index.
///
/// This enum allows unified handling of both simple forward rates and
/// CMS rates in the `IndexMapper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForwardIndexType {
    /// Standard rate index (SOFR, EURIBOR, etc.)
    Rate(RateIndex),
    /// CMS index (requires convexity adjustment)
    Cms(CmsIndex),
}

/// Maps rate indices, currencies, and curves to numeric IDs.
///
/// `IndexMapper` maintains bidirectional mappings between:
/// - `RateIndex` ↔ `u16` forward index IDs
/// - `Currency` ↔ `u8` currency IDs
/// - Curve names ↔ `u8` discount curve IDs
/// - `FxPair` ↔ `u16` FX index IDs
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
/// use infra_domain::market::{RateIndex, Currency};
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

    // CMS index mapping (CmsIndex → u16)
    // CMS indices share the same ID space as forward indices
    cms_index_to_id: HashMap<CmsIndex, u16>,
    id_to_cms_index: HashMap<u16, CmsIndex>,

    // Currency mapping (Currency → u8)
    currency_to_id: HashMap<Currency, u8>,
    id_to_currency: Vec<Currency>,

    // Discount curve mapping (String → u8)
    discount_curve_to_id: HashMap<String, u8>,
    id_to_discount_curve: Vec<String>,

    // FX pair mapping (FxPair → u16)
    fx_pair_to_id: HashMap<FxPair, u16>,
    id_to_fx_pair: Vec<Option<FxPair>>,
}

impl IndexMapper {
    /// Creates a new empty `IndexMapper`.
    ///
    /// The mapper is initialised with:
    /// - ID 0 reserved for dummy forward index
    /// - ID 0 reserved for dummy FX index (returns 1.0)
    /// - No currencies registered (first registered becomes base)
    #[must_use]
    pub fn new() -> Self {
        Self {
            fwd_index_to_id: HashMap::new(),
            id_to_fwd_index: vec![None], // 0 = dummy
            cms_index_to_id: HashMap::new(),
            id_to_cms_index: HashMap::new(),
            currency_to_id: HashMap::new(),
            id_to_currency: Vec::new(),
            discount_curve_to_id: HashMap::new(),
            id_to_discount_curve: Vec::new(),
            fx_pair_to_id: HashMap::new(),
            id_to_fx_pair: vec![None], // 0 = dummy (no FX conversion)
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
    // CMS Index Methods
    // =========================================================================

    /// Registers a CMS index and returns its ID.
    ///
    /// CMS indices share the same ID space as forward indices, allowing
    /// unified processing. The `CurveProvider` is responsible for returning
    /// convexity-adjusted rates for CMS index IDs.
    ///
    /// # Arguments
    ///
    /// * `cms_index` - The CMS index to register
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this CMS index (shares space with forward
    /// indices).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::compiler::{IndexMapper, CmsIndex};
    /// use infra_domain::{time::Tenor, market::Currency};
    ///
    /// let mut mapper = IndexMapper::new();
    /// let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
    /// let id = mapper.register_cms_index(cms10y);
    /// assert!(id > 0); // 0 is reserved for dummy
    /// ```
    pub fn register_cms_index(&mut self, cms_index: CmsIndex) -> u16 {
        if let Some(&id) = self.cms_index_to_id.get(&cms_index) {
            return id;
        }

        // CMS indices use the same ID space as forward indices
        let id = self.id_to_fwd_index.len() as u16;
        self.cms_index_to_id.insert(cms_index, id);
        self.id_to_cms_index.insert(id, cms_index);
        self.id_to_fwd_index.push(None); // No RateIndex for CMS
        id
    }

    /// Gets the ID for a CMS index.
    ///
    /// # Arguments
    ///
    /// * `cms_index` - The CMS index to look up
    ///
    /// # Returns
    ///
    /// * `Some(id)` - The CMS index's ID
    /// * `None` - CMS index not registered
    #[must_use]
    pub fn get_cms_index_id(&self, cms_index: CmsIndex) -> Option<u16> {
        self.cms_index_to_id.get(&cms_index).copied()
    }

    /// Gets or registers a CMS index.
    ///
    /// If not registered, registers it first.
    pub fn get_or_register_cms_index(&mut self, cms_index: CmsIndex) -> u16 {
        if let Some(&id) = self.cms_index_to_id.get(&cms_index) {
            id
        } else {
            self.register_cms_index(cms_index)
        }
    }

    /// Gets the CMS index for a given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to look up
    ///
    /// # Returns
    ///
    /// * `Some(CmsIndex)` - The CMS index
    /// * `None` - ID is not a CMS index
    #[must_use]
    pub fn get_cms_index(&self, id: u16) -> Option<CmsIndex> {
        self.id_to_cms_index.get(&id).copied()
    }

    /// Returns true if the given ID refers to a CMS index.
    ///
    /// CMS indices require convexity adjustment in the market data provider.
    #[must_use]
    pub fn is_cms_index(&self, id: u16) -> bool { self.id_to_cms_index.contains_key(&id) }

    /// Returns the number of registered CMS indices.
    #[must_use]
    pub fn cms_index_count(&self) -> usize { self.cms_index_to_id.len() }

    /// Gets the forward index type for a given ID.
    ///
    /// Returns the appropriate type (Rate or CMS) for unified handling.
    ///
    /// # Arguments
    ///
    /// * `id` - The forward index ID
    ///
    /// # Returns
    ///
    /// * `Some(ForwardIndexType::Rate(..))` - Standard rate index
    /// * `Some(ForwardIndexType::Cms(..))` - CMS index
    /// * `None` - ID is dummy (0) or out of range
    #[must_use]
    pub fn get_forward_index_type(&self, id: u16) -> Option<ForwardIndexType> {
        if id == 0 {
            return None; // Dummy index
        }

        // Check if it's a CMS index
        if let Some(&cms) = self.id_to_cms_index.get(&id) {
            return Some(ForwardIndexType::Cms(cms));
        }

        // Check if it's a standard rate index
        if let Some(Some(rate)) = self.id_to_fwd_index.get(id as usize) {
            return Some(ForwardIndexType::Rate(*rate));
        }

        None
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
    // FX Pair Methods
    // =========================================================================

    /// Registers an FX pair and returns its ID.
    ///
    /// If the pair is already registered, returns the existing ID.
    /// ID 0 is reserved for dummy (no FX conversion).
    ///
    /// # Arguments
    ///
    /// * `fx_pair` - The FX pair to register (e.g., EUR/USD)
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this FX pair (1+, 0 is reserved).
    pub fn register_fx_pair(&mut self, fx_pair: FxPair) -> u16 {
        if let Some(&id) = self.fx_pair_to_id.get(&fx_pair) {
            return id;
        }

        let id = self.id_to_fx_pair.len() as u16;
        self.fx_pair_to_id.insert(fx_pair, id);
        self.id_to_fx_pair.push(Some(fx_pair));
        id
    }

    /// Gets the ID for an FX pair.
    ///
    /// # Arguments
    ///
    /// * `fx_pair` - The FX pair to look up
    ///
    /// # Returns
    ///
    /// * `Some(id)` - The pair's ID
    /// * `None` - Pair not registered
    #[must_use]
    pub fn get_fx_pair_id(&self, fx_pair: FxPair) -> Option<u16> {
        self.fx_pair_to_id.get(&fx_pair).copied()
    }

    /// Gets or registers an FX pair.
    ///
    /// If not registered, registers it first.
    pub fn get_or_register_fx_pair(&mut self, fx_pair: FxPair) -> u16 {
        if let Some(&id) = self.fx_pair_to_id.get(&fx_pair) {
            id
        } else {
            self.register_fx_pair(fx_pair)
        }
    }

    /// Gets the FX pair for a given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to look up
    ///
    /// # Returns
    ///
    /// * `Some(FxPair)` - The FX pair (None for ID 0 = dummy)
    /// * `None` - ID out of range
    #[must_use]
    pub fn get_fx_pair(&self, id: u16) -> Option<Option<FxPair>> {
        self.id_to_fx_pair.get(id as usize).copied()
    }

    /// Returns the number of registered FX pairs (excluding dummy).
    #[must_use]
    pub fn fx_pair_count(&self) -> usize {
        self.id_to_fx_pair.len() - 1 // Subtract dummy
    }

    /// Returns the ID for single currency (dummy FX returning 1.0).
    #[must_use]
    pub const fn single_currency_fx_id(&self) -> u16 {
        0 // Dummy FX
    }

    /// Registers an FX pair from two currencies.
    ///
    /// Convenience method that creates an FxPair from base and quote
    /// currencies.
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency (e.g., EUR)
    /// * `quote` - Quote currency (e.g., USD)
    ///
    /// # Returns
    ///
    /// The numeric ID assigned to this FX pair.
    pub fn register_fx_pair_from_currencies(&mut self, base: Currency, quote: Currency) -> u16 {
        let fx_pair = FxPair::new(base, quote);
        self.register_fx_pair(fx_pair)
    }

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

    /// Validates that an FX pair ID is valid.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::UnknownIndex` if the ID is out of range.
    pub fn validate_fx_pair_id(&self, id: u16) -> Result<(), CompileError> {
        if (id as usize) < self.id_to_fx_pair.len() {
            Ok(())
        } else {
            Err(CompileError::unknown_index(format!(
                "Invalid FX pair ID: {id}"
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

    // =========================================================================
    // FX Pair Tests (Task 7.1)
    // =========================================================================

    use pricer_core::types::FxPair;

    #[test]
    fn test_register_fx_pair() {
        let mut mapper = IndexMapper::new();

        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        let id = mapper.register_fx_pair(eurusd);
        assert_eq!(id, 1); // 0 is reserved for dummy

        let gbpusd = FxPair::new(Currency::GBP, Currency::USD);
        let id2 = mapper.register_fx_pair(gbpusd);
        assert_eq!(id2, 2);

        // Re-registering returns same ID
        let id3 = mapper.register_fx_pair(eurusd);
        assert_eq!(id3, 1);

        assert_eq!(mapper.fx_pair_count(), 2);
    }

    #[test]
    fn test_get_fx_pair_id() {
        let mut mapper = IndexMapper::new();
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        mapper.register_fx_pair(eurusd);

        assert_eq!(mapper.get_fx_pair_id(eurusd), Some(1));

        let gbpusd = FxPair::new(Currency::GBP, Currency::USD);
        assert_eq!(mapper.get_fx_pair_id(gbpusd), None);
    }

    #[test]
    fn test_get_fx_pair() {
        let mut mapper = IndexMapper::new();
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        mapper.register_fx_pair(eurusd);

        // ID 0 is dummy (None)
        assert_eq!(mapper.get_fx_pair(0), Some(None));

        // ID 1 is EUR/USD
        assert_eq!(mapper.get_fx_pair(1), Some(Some(eurusd)));

        // ID 2 is out of range
        assert_eq!(mapper.get_fx_pair(2), None);
    }

    #[test]
    fn test_single_currency_fx_id() {
        let mapper = IndexMapper::new();
        assert_eq!(mapper.single_currency_fx_id(), 0);
    }

    #[test]
    fn test_get_or_register_fx_pair() {
        let mut mapper = IndexMapper::new();
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);

        // First call registers
        let id1 = mapper.get_or_register_fx_pair(eurusd);
        assert_eq!(id1, 1);

        // Second call returns existing
        let id2 = mapper.get_or_register_fx_pair(eurusd);
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_register_fx_pair_from_currencies() {
        let mut mapper = IndexMapper::new();

        let id = mapper.register_fx_pair_from_currencies(Currency::EUR, Currency::USD);
        assert_eq!(id, 1);

        // Should be equivalent to registering FxPair directly
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        let id2 = mapper.get_fx_pair_id(eurusd);
        assert_eq!(id2, Some(1));
    }

    #[test]
    fn test_validate_fx_pair_id() {
        let mut mapper = IndexMapper::new();
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        mapper.register_fx_pair(eurusd);

        assert!(mapper.validate_fx_pair_id(0).is_ok()); // Dummy
        assert!(mapper.validate_fx_pair_id(1).is_ok()); // EUR/USD
        assert!(mapper.validate_fx_pair_id(2).is_err()); // Invalid
    }

    #[test]
    fn test_fx_pair_preserves_direction() {
        let mut mapper = IndexMapper::new();

        // EUR/USD and USD/EUR should be different pairs
        let eurusd = FxPair::new(Currency::EUR, Currency::USD);
        let usdeur = FxPair::new(Currency::USD, Currency::EUR);

        let id1 = mapper.register_fx_pair(eurusd);
        let id2 = mapper.register_fx_pair(usdeur);

        assert_ne!(
            id1, id2,
            "Different FX directions should have different IDs"
        );
        assert_eq!(mapper.fx_pair_count(), 2);
    }

    // =========================================================================
    // CMS Index Tests (Task 8.1)
    // =========================================================================

    use infra_domain::time::Tenor;

    use super::{CmsIndex, ForwardIndexType};

    #[test]
    fn test_cms_index_new() {
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        assert_eq!(cms10y.currency(), Currency::USD);
        assert_eq!(cms10y.swap_tenor(), Tenor::TenYears);
        assert!(cms10y.requires_convexity_adjustment());
    }

    #[test]
    fn test_cms_index_name() {
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        assert_eq!(cms10y.name(), "CMS-USD-10Y");

        let cms5y = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
        assert_eq!(cms5y.name(), "CMS-EUR-5Y");
    }

    #[test]
    fn test_cms_index_display() {
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        assert_eq!(format!("{}", cms10y), "CMS-USD-10Y");
    }

    #[test]
    fn test_register_cms_index() {
        let mut mapper = IndexMapper::new();

        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let id = mapper.register_cms_index(cms10y);
        assert!(id > 0); // 0 is reserved for dummy

        let cms5y = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
        let id2 = mapper.register_cms_index(cms5y);
        assert_eq!(id2, id + 1);

        // Re-registering returns same ID
        let id3 = mapper.register_cms_index(cms10y);
        assert_eq!(id3, id);

        assert_eq!(mapper.cms_index_count(), 2);
    }

    #[test]
    fn test_get_cms_index_id() {
        let mut mapper = IndexMapper::new();
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let id = mapper.register_cms_index(cms10y);

        assert_eq!(mapper.get_cms_index_id(cms10y), Some(id));

        let cms5y = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
        assert_eq!(mapper.get_cms_index_id(cms5y), None);
    }

    #[test]
    fn test_get_cms_index() {
        let mut mapper = IndexMapper::new();
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let id = mapper.register_cms_index(cms10y);

        assert_eq!(mapper.get_cms_index(id), Some(cms10y));
        assert_eq!(mapper.get_cms_index(0), None); // Dummy
        assert_eq!(mapper.get_cms_index(999), None); // Out of range
    }

    #[test]
    fn test_is_cms_index() {
        let mut mapper = IndexMapper::new();

        // Register a forward index
        let sofr_id = mapper.register_forward_index(RateIndex::Sofr);

        // Register a CMS index
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms_id = mapper.register_cms_index(cms10y);

        assert!(!mapper.is_cms_index(0)); // Dummy
        assert!(!mapper.is_cms_index(sofr_id)); // SOFR
        assert!(mapper.is_cms_index(cms_id)); // CMS
    }

    #[test]
    fn test_get_or_register_cms_index() {
        let mut mapper = IndexMapper::new();
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);

        // First call registers
        let id1 = mapper.get_or_register_cms_index(cms10y);
        assert!(id1 > 0);

        // Second call returns existing
        let id2 = mapper.get_or_register_cms_index(cms10y);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_cms_and_forward_share_id_space() {
        let mut mapper = IndexMapper::new();

        // Register a forward index
        let sofr_id = mapper.register_forward_index(RateIndex::Sofr);

        // Register a CMS index - should get next ID
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms_id = mapper.register_cms_index(cms10y);

        assert_eq!(
            cms_id,
            sofr_id + 1,
            "CMS should use next ID in shared space"
        );

        // Register another forward index - should get next ID
        let estr_id = mapper.register_forward_index(RateIndex::Estr);
        assert_eq!(estr_id, cms_id + 1);
    }

    #[test]
    fn test_get_forward_index_type() {
        let mut mapper = IndexMapper::new();

        // Dummy index
        assert_eq!(mapper.get_forward_index_type(0), None);

        // Register forward index
        let sofr_id = mapper.register_forward_index(RateIndex::Sofr);
        assert_eq!(
            mapper.get_forward_index_type(sofr_id),
            Some(ForwardIndexType::Rate(RateIndex::Sofr))
        );

        // Register CMS index
        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms_id = mapper.register_cms_index(cms10y);
        assert_eq!(
            mapper.get_forward_index_type(cms_id),
            Some(ForwardIndexType::Cms(cms10y))
        );

        // Out of range
        assert_eq!(mapper.get_forward_index_type(999), None);
    }

    #[test]
    fn test_cms_index_hash() {
        use std::collections::HashSet;

        let cms10y = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms5y = CmsIndex::new(Currency::EUR, Tenor::FiveYears);
        let cms10y_dup = CmsIndex::new(Currency::USD, Tenor::TenYears);

        let mut set = HashSet::new();
        set.insert(cms10y);
        set.insert(cms5y);
        set.insert(cms10y_dup); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_cms_index_equality() {
        let cms10y_a = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms10y_b = CmsIndex::new(Currency::USD, Tenor::TenYears);
        let cms5y = CmsIndex::new(Currency::USD, Tenor::FiveYears);
        let cms10y_eur = CmsIndex::new(Currency::EUR, Tenor::TenYears);

        assert_eq!(cms10y_a, cms10y_b);
        assert_ne!(cms10y_a, cms5y);
        assert_ne!(cms10y_a, cms10y_eur);
    }
}

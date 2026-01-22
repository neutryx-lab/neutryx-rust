//! Convention set container for CF expansion.
//!
//! This module provides a container that aggregates all convention types
//! needed for cashflow expansion of financial instruments.

use super::{
    BondConvention, CapFloorConvention, CdsConvention, FraConvention, FxConvention, SwapConvention,
};
use crate::trade::instrument_def::InstrumentError;

/// Container for market conventions used in CF expansion.
///
/// Holds optional references to various convention types.
/// Use `get_*()` methods to retrieve conventions with proper error handling.
///
/// # Example
///
/// ```rust,ignore
/// use infra_master::trade::convention::{ConventionSet, SwapConvention};
///
/// // Create a convention set using builder pattern
/// let conventions = ConventionSet::new()
///     .with_swap(SwapConvention::usd_sofr())
///     .with_fx(FxConvention::usd_default());
///
/// // Or use a standard preset
/// let usd_conventions = ConventionSet::usd_standard();
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConventionSet {
    /// Swap convention.
    pub swap: Option<SwapConvention>,
    /// FRA convention.
    pub fra: Option<FraConvention>,
    /// Cap/Floor convention.
    pub cap_floor: Option<CapFloorConvention>,
    /// FX convention.
    pub fx: Option<FxConvention>,
    /// CDS convention.
    pub cds: Option<CdsConvention>,
    /// Bond convention.
    pub bond: Option<BondConvention>,
}

impl ConventionSet {
    /// Creates a new empty convention set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ---- Builder methods ----

    /// Sets the swap convention.
    #[must_use]
    pub fn with_swap(mut self, conv: SwapConvention) -> Self {
        self.swap = Some(conv);
        self
    }

    /// Sets the FRA convention.
    #[must_use]
    pub fn with_fra(mut self, conv: FraConvention) -> Self {
        self.fra = Some(conv);
        self
    }

    /// Sets the cap/floor convention.
    #[must_use]
    pub fn with_cap_floor(mut self, conv: CapFloorConvention) -> Self {
        self.cap_floor = Some(conv);
        self
    }

    /// Sets the FX convention.
    #[must_use]
    pub fn with_fx(mut self, conv: FxConvention) -> Self {
        self.fx = Some(conv);
        self
    }

    /// Sets the CDS convention.
    #[must_use]
    pub fn with_cds(mut self, conv: CdsConvention) -> Self {
        self.cds = Some(conv);
        self
    }

    /// Sets the bond convention.
    #[must_use]
    pub fn with_bond(mut self, conv: BondConvention) -> Self {
        self.bond = Some(conv);
        self
    }

    // ---- Getter methods ----

    /// Returns the swap convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if swap convention is not set.
    pub fn get_swap(&self) -> Result<&SwapConvention, InstrumentError> {
        self.swap
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Swap"))
    }

    /// Returns the FRA convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if FRA convention is not set.
    pub fn get_fra(&self) -> Result<&FraConvention, InstrumentError> {
        self.fra
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("FRA"))
    }

    /// Returns the cap/floor convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if cap/floor convention is not set.
    pub fn get_cap_floor(&self) -> Result<&CapFloorConvention, InstrumentError> {
        self.cap_floor
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("CapFloor"))
    }

    /// Returns the FX convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if FX convention is not set.
    pub fn get_fx(&self) -> Result<&FxConvention, InstrumentError> {
        self.fx
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("FX"))
    }

    /// Returns the CDS convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if CDS convention is not set.
    pub fn get_cds(&self) -> Result<&CdsConvention, InstrumentError> {
        self.cds
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("CDS"))
    }

    /// Returns the bond convention, or an error if not set.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError::MissingConvention` if bond convention is not set.
    pub fn get_bond(&self) -> Result<&BondConvention, InstrumentError> {
        self.bond
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Bond"))
    }

    // ---- Standard presets ----

    /// Returns a standard USD market convention set.
    ///
    /// Includes:
    /// - USD SOFR swap convention
    /// - USD FX convention
    /// - North American CDS convention
    #[must_use]
    pub fn usd_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_cds(CdsConvention::isda_na())
    }

    /// Returns a standard EUR market convention set.
    ///
    /// Includes:
    /// - EUR EURIBOR 6M swap convention
    /// - EUR FX convention
    /// - European CDS convention
    #[must_use]
    pub fn eur_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::eur_euribor_6m())
            .with_fx(FxConvention::eur_default())
            .with_cds(CdsConvention::isda_eu())
    }

    /// Returns a standard GBP market convention set.
    ///
    /// Includes:
    /// - GBP SONIA swap convention
    /// - GBP FX convention
    #[must_use]
    pub fn gbp_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::gbp_sonia())
            .with_fx(FxConvention::gbp_default())
    }

    /// Returns a standard JPY market convention set.
    ///
    /// Includes:
    /// - JPY TONAR swap convention
    /// - JPY FX convention
    #[must_use]
    pub fn jpy_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::jpy_tonar())
            .with_fx(FxConvention::jpy_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convention_set_new() {
        let set = ConventionSet::new();
        assert!(set.swap.is_none());
        assert!(set.fx.is_none());
        assert!(set.cds.is_none());
    }

    #[test]
    fn test_convention_set_with_swap() {
        let set = ConventionSet::new().with_swap(SwapConvention::usd_sofr());
        assert!(set.swap.is_some());
        assert!(set.get_swap().is_ok());
    }

    #[test]
    fn test_convention_set_get_swap_error() {
        let set = ConventionSet::new();
        let result = set.get_swap();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));
    }

    #[test]
    fn test_convention_set_with_fx() {
        let set = ConventionSet::new().with_fx(FxConvention::usd_default());
        assert!(set.fx.is_some());
        assert!(set.get_fx().is_ok());
    }

    #[test]
    fn test_convention_set_get_fx_error() {
        let set = ConventionSet::new();
        let result = set.get_fx();
        assert!(result.is_err());
    }

    #[test]
    fn test_convention_set_with_cds() {
        let set = ConventionSet::new().with_cds(CdsConvention::isda_na());
        assert!(set.cds.is_some());
        assert!(set.get_cds().is_ok());
    }

    #[test]
    fn test_convention_set_get_cds_error() {
        let set = ConventionSet::new();
        let result = set.get_cds();
        assert!(result.is_err());
    }

    #[test]
    fn test_convention_set_usd_standard() {
        let set = ConventionSet::usd_standard();
        assert!(set.swap.is_some());
        assert!(set.fx.is_some());
        assert!(set.cds.is_some());
    }

    #[test]
    fn test_convention_set_eur_standard() {
        let set = ConventionSet::eur_standard();
        assert!(set.swap.is_some());
        assert!(set.fx.is_some());
        assert!(set.cds.is_some());
    }

    #[test]
    fn test_convention_set_gbp_standard() {
        let set = ConventionSet::gbp_standard();
        assert!(set.swap.is_some());
        assert!(set.fx.is_some());
    }

    #[test]
    fn test_convention_set_jpy_standard() {
        let set = ConventionSet::jpy_standard();
        assert!(set.swap.is_some());
        assert!(set.fx.is_some());
    }

    #[test]
    fn test_convention_set_builder_chain() {
        let set = ConventionSet::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_cds(CdsConvention::isda_na());

        assert!(set.swap.is_some());
        assert!(set.fx.is_some());
        assert!(set.cds.is_some());
        assert!(set.fra.is_none());
    }

    #[test]
    fn test_convention_set_clone() {
        let set = ConventionSet::usd_standard();
        let cloned = set.clone();
        assert!(cloned.swap.is_some());
        assert!(cloned.fx.is_some());
    }

    #[test]
    fn test_convention_set_debug() {
        let set = ConventionSet::new();
        let debug = format!("{:?}", set);
        assert!(debug.contains("ConventionSet"));
    }
}

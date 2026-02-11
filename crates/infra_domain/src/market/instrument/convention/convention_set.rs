//! Convention set container for CF expansion.

use super::{
    BondConvention, CapFloorConvention, CdsConvention, CommodityConvention, EquityConvention,
    FraConvention, FxConvention, FxOptionConvention, InflationSwapConvention, SwapConvention,
    SwaptionConvention,
};
use crate::trade::instrument_def::InstrumentError;

/// Container for market conventions used in CF expansion.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConventionSet {
    /// Swap convention.
    pub swap: Option<SwapConvention>,
    /// Swaption convention.
    pub swaption: Option<SwaptionConvention>,
    /// FRA convention.
    pub fra: Option<FraConvention>,
    /// Cap/Floor convention.
    pub cap_floor: Option<CapFloorConvention>,
    /// Inflation swap convention.
    pub inflation_swap: Option<InflationSwapConvention>,

    /// FX convention.
    pub fx: Option<FxConvention>,
    /// FX option convention.
    pub fx_option: Option<FxOptionConvention>,

    /// CDS convention.
    pub cds: Option<CdsConvention>,

    /// Equity convention.
    pub equity: Option<EquityConvention>,

    /// Commodity convention.
    pub commodity: Option<CommodityConvention>,

    /// Bond convention.
    pub bond: Option<BondConvention>,
}

impl ConventionSet {
    /// Creates a new empty convention set.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the swap convention.
    #[must_use]
    pub fn with_swap(mut self, conv: SwapConvention) -> Self {
        self.swap = Some(conv);
        self
    }

    /// Sets the swaption convention.
    #[must_use]
    pub fn with_swaption(mut self, conv: SwaptionConvention) -> Self {
        self.swaption = Some(conv);
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

    /// Sets the inflation swap convention.
    #[must_use]
    pub fn with_inflation_swap(mut self, conv: InflationSwapConvention) -> Self {
        self.inflation_swap = Some(conv);
        self
    }

    /// Sets the FX convention.
    #[must_use]
    pub fn with_fx(mut self, conv: FxConvention) -> Self {
        self.fx = Some(conv);
        self
    }

    /// Sets the FX option convention.
    #[must_use]
    pub fn with_fx_option(mut self, conv: FxOptionConvention) -> Self {
        self.fx_option = Some(conv);
        self
    }

    /// Sets the CDS convention.
    #[must_use]
    pub fn with_cds(mut self, conv: CdsConvention) -> Self {
        self.cds = Some(conv);
        self
    }

    /// Sets the equity convention.
    #[must_use]
    pub fn with_equity(mut self, conv: EquityConvention) -> Self {
        self.equity = Some(conv);
        self
    }

    /// Sets the commodity convention.
    #[must_use]
    pub fn with_commodity(mut self, conv: CommodityConvention) -> Self {
        self.commodity = Some(conv);
        self
    }

    /// Sets the bond convention.
    #[must_use]
    pub fn with_bond(mut self, conv: BondConvention) -> Self {
        self.bond = Some(conv);
        self
    }

    /// Returns the swap convention, or an error if not set.
    pub fn get_swap(&self) -> Result<&SwapConvention, InstrumentError> {
        self.swap
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Swap"))
    }

    /// Returns the swaption convention, or an error if not set.
    pub fn get_swaption(&self) -> Result<&SwaptionConvention, InstrumentError> {
        self.swaption
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Swaption"))
    }

    /// Returns the FRA convention, or an error if not set.
    pub fn get_fra(&self) -> Result<&FraConvention, InstrumentError> {
        self.fra
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("FRA"))
    }

    /// Returns the cap/floor convention, or an error if not set.
    pub fn get_cap_floor(&self) -> Result<&CapFloorConvention, InstrumentError> {
        self.cap_floor
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("CapFloor"))
    }

    /// Returns the inflation swap convention, or an error if not set.
    pub fn get_inflation_swap(&self) -> Result<&InflationSwapConvention, InstrumentError> {
        self.inflation_swap
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("InflationSwap"))
    }

    /// Returns the FX convention, or an error if not set.
    pub fn get_fx(&self) -> Result<&FxConvention, InstrumentError> {
        self.fx
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("FX"))
    }

    /// Returns the FX option convention, or an error if not set.
    pub fn get_fx_option(&self) -> Result<&FxOptionConvention, InstrumentError> {
        self.fx_option
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("FxOption"))
    }

    /// Returns the CDS convention, or an error if not set.
    pub fn get_cds(&self) -> Result<&CdsConvention, InstrumentError> {
        self.cds
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("CDS"))
    }

    /// Returns the equity convention, or an error if not set.
    pub fn get_equity(&self) -> Result<&EquityConvention, InstrumentError> {
        self.equity
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Equity"))
    }

    /// Returns the commodity convention, or an error if not set.
    pub fn get_commodity(&self) -> Result<&CommodityConvention, InstrumentError> {
        self.commodity
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Commodity"))
    }

    /// Returns the bond convention, or an error if not set.
    pub fn get_bond(&self) -> Result<&BondConvention, InstrumentError> {
        self.bond
            .as_ref()
            .ok_or_else(|| InstrumentError::missing_convention("Bond"))
    }

    /// Returns a standard USD market convention set.
    #[must_use]
    pub fn usd_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_swaption(SwaptionConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_fx_option(FxOptionConvention::g10_standard())
            .with_cds(CdsConvention::isda_na())
            .with_equity(EquityConvention::us_equity())
            .with_inflation_swap(InflationSwapConvention::us_cpi_zc())
            .with_commodity(CommodityConvention::wti_crude())
    }

    /// Returns a standard EUR market convention set.
    #[must_use]
    pub fn eur_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::eur_euribor_6m())
            .with_swaption(SwaptionConvention::eur_euribor())
            .with_fx(FxConvention::eur_default())
            .with_fx_option(FxOptionConvention::eur_usd())
            .with_cds(CdsConvention::isda_eu())
            .with_equity(EquityConvention::eu_equity())
            .with_inflation_swap(InflationSwapConvention::eur_hicp_zc())
    }

    /// Returns a standard GBP market convention set.
    #[must_use]
    pub fn gbp_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::gbp_sonia())
            .with_swaption(SwaptionConvention::gbp_sonia())
            .with_fx(FxConvention::gbp_default())
            .with_fx_option(FxOptionConvention::gbp_usd())
            .with_equity(EquityConvention::uk_equity())
            .with_inflation_swap(InflationSwapConvention::uk_rpi_zc())
    }

    /// Returns a standard JPY market convention set.
    #[must_use]
    pub fn jpy_standard() -> Self {
        Self::new()
            .with_swap(SwapConvention::jpy_tonar())
            .with_swaption(SwaptionConvention::jpy_tonar())
            .with_fx(FxConvention::jpy_default())
            .with_fx_option(FxOptionConvention::usd_jpy())
            .with_equity(EquityConvention::jp_equity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_set_returns_errors() {
        let set = ConventionSet::new();
        assert!(set.get_swap().is_err());
        assert!(set.get_swaption().is_err());
        assert!(set.get_fra().is_err());
        assert!(set.get_cap_floor().is_err());
        assert!(set.get_inflation_swap().is_err());
        assert!(set.get_fx().is_err());
        assert!(set.get_fx_option().is_err());
        assert!(set.get_cds().is_err());
        assert!(set.get_equity().is_err());
        assert!(set.get_commodity().is_err());
        assert!(set.get_bond().is_err());
        assert!(matches!(
            set.get_swap().unwrap_err(),
            InstrumentError::MissingConvention { .. }
        ));
    }

    #[test]
    fn test_builder_chain() {
        let set = ConventionSet::new()
            .with_swap(SwapConvention::usd_sofr())
            .with_swaption(SwaptionConvention::usd_sofr())
            .with_fx(FxConvention::usd_default())
            .with_fx_option(FxOptionConvention::g10_standard())
            .with_cds(CdsConvention::isda_na())
            .with_equity(EquityConvention::us_equity())
            .with_commodity(CommodityConvention::wti_crude())
            .with_inflation_swap(InflationSwapConvention::us_cpi_zc());

        assert!(set.get_swap().is_ok());
        assert!(set.get_swaption().is_ok());
        assert!(set.get_fx().is_ok());
        assert!(set.get_fx_option().is_ok());
        assert!(set.get_cds().is_ok());
        assert!(set.get_equity().is_ok());
        assert!(set.get_commodity().is_ok());
        assert!(set.get_inflation_swap().is_ok());
        assert!(set.fra.is_none());
    }

    #[test]
    fn test_standard_presets() {
        let usd = ConventionSet::usd_standard();
        assert!(usd.get_swap().is_ok());
        assert!(usd.get_cds().is_ok());
        assert!(usd.get_commodity().is_ok());

        let eur = ConventionSet::eur_standard();
        assert!(eur.get_swap().is_ok());
        assert!(eur.get_inflation_swap().is_ok());

        let gbp = ConventionSet::gbp_standard();
        assert!(gbp.get_swap().is_ok());
        assert!(gbp.get_equity().is_ok());

        let jpy = ConventionSet::jpy_standard();
        assert!(jpy.get_swap().is_ok());
        assert!(jpy.get_fx().is_ok());
    }
}

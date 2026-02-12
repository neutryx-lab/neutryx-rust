//! Convention set container for CF expansion.

use super::{
    BondConvention, CapFloorConvention, CdsConvention, CommodityConvention, EquityConvention,
    FraConvention, FxConvention, FxOptionConvention, InflationSwapConvention, SwapConvention,
    SwaptionConvention,
};
use crate::trade::instrument_def::InstrumentError;

/// Container for market conventions used in CF expansion.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
        Self {
            swap: Some(SwapConvention::usd_sofr()),
            swaption: Some(SwaptionConvention::usd_sofr()),
            fx: Some(FxConvention::usd_default()),
            fx_option: Some(FxOptionConvention::g10_standard()),
            cds: Some(CdsConvention::isda_na()),
            equity: Some(EquityConvention::us_equity()),
            inflation_swap: Some(InflationSwapConvention::us_cpi_zc()),
            commodity: Some(CommodityConvention::wti_crude()),
            ..Default::default()
        }
    }

    /// Returns a standard EUR market convention set.
    #[must_use]
    pub fn eur_standard() -> Self {
        Self {
            swap: Some(SwapConvention::eur_euribor_6m()),
            swaption: Some(SwaptionConvention::eur_euribor()),
            fx: Some(FxConvention::eur_default()),
            fx_option: Some(FxOptionConvention::eur_usd()),
            cds: Some(CdsConvention::isda_eu()),
            equity: Some(EquityConvention::eu_equity()),
            inflation_swap: Some(InflationSwapConvention::eur_hicp_zc()),
            ..Default::default()
        }
    }

    /// Returns a standard GBP market convention set.
    #[must_use]
    pub fn gbp_standard() -> Self {
        Self {
            swap: Some(SwapConvention::gbp_sonia()),
            swaption: Some(SwaptionConvention::gbp_sonia()),
            fx: Some(FxConvention::gbp_default()),
            fx_option: Some(FxOptionConvention::gbp_usd()),
            equity: Some(EquityConvention::uk_equity()),
            inflation_swap: Some(InflationSwapConvention::uk_rpi_zc()),
            ..Default::default()
        }
    }

    /// Returns a standard JPY market convention set.
    #[must_use]
    pub fn jpy_standard() -> Self {
        Self {
            swap: Some(SwapConvention::jpy_tonar()),
            swaption: Some(SwaptionConvention::jpy_tonar()),
            fx: Some(FxConvention::jpy_default()),
            fx_option: Some(FxOptionConvention::usd_jpy()),
            equity: Some(EquityConvention::jp_equity()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_set_returns_errors() {
        let set = ConventionSet::default();
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
    fn test_struct_construction() {
        let set = ConventionSet {
            swap: Some(SwapConvention::usd_sofr()),
            swaption: Some(SwaptionConvention::usd_sofr()),
            fx: Some(FxConvention::usd_default()),
            fx_option: Some(FxOptionConvention::g10_standard()),
            cds: Some(CdsConvention::isda_na()),
            equity: Some(EquityConvention::us_equity()),
            commodity: Some(CommodityConvention::wti_crude()),
            inflation_swap: Some(InflationSwapConvention::us_cpi_zc()),
            ..Default::default()
        };

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

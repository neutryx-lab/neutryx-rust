//! Commodity convention definitions.

use crate::time::CalendarId;

/// Delivery convention for commodity contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DeliveryConvention {
    /// Physical delivery of the commodity.
    Physical,
    /// Cash settlement at expiry.
    Cash,
    /// Financial settlement (index-based).
    Financial,
}

/// Price quotation convention for commodities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PriceQuotation {
    /// Price per barrel (crude oil).
    PerBarrel,
    /// Price per metric tonne.
    PerMetricTonne,
    /// Price per troy ounce (precious metals).
    PerTroyOunce,
    /// Price per bushel (grains).
    PerBushel,
    /// Price per MMBtu (natural gas).
    PerMMBtu,
    /// Price per MWh (electricity).
    PerMWh,
}

/// Convention for commodity derivatives.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CommodityConvention {
    /// Delivery convention.
    pub delivery_convention: DeliveryConvention,
    /// Price quotation convention.
    pub price_quotation: PriceQuotation,
    /// Pricing calendar (for averaging, settlement).
    pub pricing_calendar: CalendarId,
    /// Number of business days to settlement.
    pub settlement_days: u32,
    /// Delivery location identifier.
    pub delivery_location: Option<String>,
    /// Contract size (standard lot).
    pub contract_size: f64,
}


super::define_convention_factories! {
    for CommodityConvention;
    /// Returns the WTI Crude Oil convention (Physical, Cushing OK, 1000 bbl).
    wti_crude => {
        delivery_convention: DeliveryConvention::Physical, price_quotation: PriceQuotation::PerBarrel,
        pricing_calendar: CalendarId::NewYork, settlement_days: 2,
        delivery_location: Some("Cushing, OK".to_string()), contract_size: 1000.0,
    };
    /// Returns the Brent Crude Oil convention (Cash, ICE, 1000 bbl).
    brent_crude => {
        delivery_convention: DeliveryConvention::Cash, price_quotation: PriceQuotation::PerBarrel,
        pricing_calendar: CalendarId::London, settlement_days: 2,
        delivery_location: None, contract_size: 1000.0,
    };
    /// Returns the Henry Hub Natural Gas convention (Physical, 10000 MMBtu).
    henry_hub_gas => {
        delivery_convention: DeliveryConvention::Physical, price_quotation: PriceQuotation::PerMMBtu,
        pricing_calendar: CalendarId::NewYork, settlement_days: 2,
        delivery_location: Some("Henry Hub, LA".to_string()), contract_size: 10000.0,
    };
    /// Returns the Gold (COMEX) convention (Physical, 100 troy oz).
    comex_gold => {
        delivery_convention: DeliveryConvention::Physical, price_quotation: PriceQuotation::PerTroyOunce,
        pricing_calendar: CalendarId::NewYork, settlement_days: 2,
        delivery_location: Some("COMEX Warehouse".to_string()), contract_size: 100.0,
    };
    /// Returns the LME Copper convention (Physical, 25 tonnes).
    lme_copper => {
        delivery_convention: DeliveryConvention::Physical, price_quotation: PriceQuotation::PerMetricTonne,
        pricing_calendar: CalendarId::London, settlement_days: 2,
        delivery_location: Some("LME Warehouse".to_string()), contract_size: 25.0,
    };
    /// Returns the CBOT Corn convention (Physical, 5000 bushels).
    cbot_corn => {
        delivery_convention: DeliveryConvention::Physical, price_quotation: PriceQuotation::PerBushel,
        pricing_calendar: CalendarId::NewYork, settlement_days: 2,
        delivery_location: Some("Chicago".to_string()), contract_size: 5000.0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commodity_presets() {
        let wti = CommodityConvention::wti_crude();
        assert_eq!(wti.delivery_convention, DeliveryConvention::Physical);
        assert_eq!(wti.price_quotation, PriceQuotation::PerBarrel);
        assert_eq!(wti.delivery_location, Some("Cushing, OK".to_string()));

        let brent = CommodityConvention::brent_crude();
        assert_eq!(brent.delivery_convention, DeliveryConvention::Cash);
        assert!(brent.delivery_location.is_none());

        assert_eq!(
            CommodityConvention::henry_hub_gas().price_quotation,
            PriceQuotation::PerMMBtu
        );
        assert_eq!(
            CommodityConvention::comex_gold().price_quotation,
            PriceQuotation::PerTroyOunce
        );
        assert_eq!(
            CommodityConvention::lme_copper().price_quotation,
            PriceQuotation::PerMetricTonne
        );
        assert_eq!(
            CommodityConvention::cbot_corn().price_quotation,
            PriceQuotation::PerBushel
        );
    }
}

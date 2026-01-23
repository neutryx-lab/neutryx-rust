//! Commodity convention definitions.
//!
//! This module provides types for representing commodity market conventions.

use crate::CalendarId;

/// Delivery convention for commodity contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeliveryConvention {
    /// Physical delivery of the commodity.
    Physical,
    /// Cash settlement at expiry.
    Cash,
    /// Financial settlement (index-based).
    Financial,
}

/// Price quotation convention for commodities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
///
/// Represents the market conventions for pricing and settling commodity
/// derivatives.
///
/// # Example
///
/// ```rust
/// use infra_master::trade::convention::{
///     CommodityConvention, DeliveryConvention, PriceQuotation,
/// };
/// use infra_master::CalendarId;
///
/// let conv = CommodityConvention::wti_crude();
/// assert_eq!(conv.delivery_convention, DeliveryConvention::Physical);
/// assert_eq!(conv.price_quotation, PriceQuotation::PerBarrel);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl CommodityConvention {
    /// Creates a new commodity convention.
    #[must_use]
    pub fn new(
        delivery_convention: DeliveryConvention,
        price_quotation: PriceQuotation,
        pricing_calendar: CalendarId,
        settlement_days: u32,
        delivery_location: Option<String>,
        contract_size: f64,
    ) -> Self {
        Self {
            delivery_convention,
            price_quotation,
            pricing_calendar,
            settlement_days,
            delivery_location,
            contract_size,
        }
    }

    /// Returns the WTI Crude Oil convention.
    ///
    /// - Delivery: Physical at Cushing, OK
    /// - Price: Per barrel
    /// - Calendar: NYMEX
    /// - Contract size: 1,000 barrels
    #[must_use]
    pub fn wti_crude() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Physical,
            price_quotation: PriceQuotation::PerBarrel,
            pricing_calendar: CalendarId::NewYork,
            settlement_days: 2,
            delivery_location: Some("Cushing, OK".to_string()),
            contract_size: 1000.0,
        }
    }

    /// Returns the Brent Crude Oil convention.
    ///
    /// - Delivery: Cash (ICE)
    /// - Price: Per barrel
    /// - Calendar: ICE
    /// - Contract size: 1,000 barrels
    #[must_use]
    pub fn brent_crude() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Cash,
            price_quotation: PriceQuotation::PerBarrel,
            pricing_calendar: CalendarId::London,
            settlement_days: 2,
            delivery_location: None,
            contract_size: 1000.0,
        }
    }

    /// Returns the Henry Hub Natural Gas convention.
    ///
    /// - Delivery: Physical at Henry Hub
    /// - Price: Per MMBtu
    /// - Calendar: NYMEX
    /// - Contract size: 10,000 MMBtu
    #[must_use]
    pub fn henry_hub_gas() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Physical,
            price_quotation: PriceQuotation::PerMMBtu,
            pricing_calendar: CalendarId::NewYork,
            settlement_days: 2,
            delivery_location: Some("Henry Hub, LA".to_string()),
            contract_size: 10000.0,
        }
    }

    /// Returns the Gold (COMEX) convention.
    ///
    /// - Delivery: Physical
    /// - Price: Per troy ounce
    /// - Calendar: COMEX
    /// - Contract size: 100 troy ounces
    #[must_use]
    pub fn comex_gold() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Physical,
            price_quotation: PriceQuotation::PerTroyOunce,
            pricing_calendar: CalendarId::NewYork,
            settlement_days: 2,
            delivery_location: Some("COMEX Warehouse".to_string()),
            contract_size: 100.0,
        }
    }

    /// Returns the LME Copper convention.
    ///
    /// - Delivery: Physical
    /// - Price: Per metric tonne
    /// - Calendar: LME
    /// - Contract size: 25 tonnes
    #[must_use]
    pub fn lme_copper() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Physical,
            price_quotation: PriceQuotation::PerMetricTonne,
            pricing_calendar: CalendarId::London,
            settlement_days: 2,
            delivery_location: Some("LME Warehouse".to_string()),
            contract_size: 25.0,
        }
    }

    /// Returns the CBOT Corn convention.
    ///
    /// - Delivery: Physical
    /// - Price: Per bushel
    /// - Calendar: CBOT
    /// - Contract size: 5,000 bushels
    #[must_use]
    pub fn cbot_corn() -> Self {
        Self {
            delivery_convention: DeliveryConvention::Physical,
            price_quotation: PriceQuotation::PerBushel,
            pricing_calendar: CalendarId::NewYork,
            settlement_days: 2,
            delivery_location: Some("Chicago".to_string()),
            contract_size: 5000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commodity_convention_new() {
        let conv = CommodityConvention::new(
            DeliveryConvention::Financial,
            PriceQuotation::PerMWh,
            CalendarId::London,
            1,
            Some("UK Grid".to_string()),
            50.0,
        );

        assert_eq!(conv.delivery_convention, DeliveryConvention::Financial);
        assert_eq!(conv.price_quotation, PriceQuotation::PerMWh);
        assert_eq!(conv.settlement_days, 1);
        assert_eq!(conv.delivery_location, Some("UK Grid".to_string()));
        assert!((conv.contract_size - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_wti_crude_convention() {
        let conv = CommodityConvention::wti_crude();

        assert_eq!(conv.delivery_convention, DeliveryConvention::Physical);
        assert_eq!(conv.price_quotation, PriceQuotation::PerBarrel);
        assert_eq!(conv.delivery_location, Some("Cushing, OK".to_string()));
        assert!((conv.contract_size - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_brent_crude_convention() {
        let conv = CommodityConvention::brent_crude();

        assert_eq!(conv.delivery_convention, DeliveryConvention::Cash);
        assert_eq!(conv.price_quotation, PriceQuotation::PerBarrel);
        assert!(conv.delivery_location.is_none());
    }

    #[test]
    fn test_henry_hub_gas_convention() {
        let conv = CommodityConvention::henry_hub_gas();

        assert_eq!(conv.price_quotation, PriceQuotation::PerMMBtu);
        assert!((conv.contract_size - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_comex_gold_convention() {
        let conv = CommodityConvention::comex_gold();

        assert_eq!(conv.price_quotation, PriceQuotation::PerTroyOunce);
        assert!((conv.contract_size - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_lme_copper_convention() {
        let conv = CommodityConvention::lme_copper();

        assert_eq!(conv.price_quotation, PriceQuotation::PerMetricTonne);
        assert_eq!(conv.pricing_calendar, CalendarId::London);
    }

    #[test]
    fn test_cbot_corn_convention() {
        let conv = CommodityConvention::cbot_corn();

        assert_eq!(conv.price_quotation, PriceQuotation::PerBushel);
        assert!((conv.contract_size - 5000.0).abs() < 1e-10);
    }

    #[test]
    fn test_delivery_convention_equality() {
        assert_eq!(DeliveryConvention::Physical, DeliveryConvention::Physical);
        assert_ne!(DeliveryConvention::Physical, DeliveryConvention::Cash);
    }

    #[test]
    fn test_price_quotation_equality() {
        assert_eq!(PriceQuotation::PerBarrel, PriceQuotation::PerBarrel);
        assert_ne!(PriceQuotation::PerBarrel, PriceQuotation::PerTroyOunce);
    }

    #[test]
    fn test_commodity_convention_clone() {
        let conv = CommodityConvention::wti_crude();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}

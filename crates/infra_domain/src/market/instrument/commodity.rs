//! Commodity instrument definitions.
//!
//! This module provides definitions for commodity derivatives including
//! forwards, swaps, vanilla options, Asian options, and spread options.

use super::{common::ExerciseStyle, error::InstrumentError};
use crate::{
    trade::{OptionType, SettlementType},
    Currency, Date, Frequency,
};

/// Energy commodity subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EnergyType {
    /// Crude oil (WTI, Brent, etc.).
    CrudeOil,
    /// Natural gas.
    NaturalGas,
    /// Heating oil.
    HeatingOil,
    /// Gasoline (RBOB).
    Gasoline,
    /// Electricity.
    Electricity,
    /// Coal.
    Coal,
}

/// Metal commodity subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MetalType {
    /// Gold.
    Gold,
    /// Silver.
    Silver,
    /// Platinum.
    Platinum,
    /// Palladium.
    Palladium,
    /// Copper.
    Copper,
    /// Aluminium.
    Aluminium,
    /// Zinc.
    Zinc,
    /// Nickel.
    Nickel,
}

/// Agricultural commodity subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AgricultureType {
    /// Wheat.
    Wheat,
    /// Corn.
    Corn,
    /// Soybeans.
    Soybeans,
    /// Coffee.
    Coffee,
    /// Sugar.
    Sugar,
    /// Cotton.
    Cotton,
    /// Cocoa.
    Cocoa,
}

/// Commodity type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommodityType {
    /// Energy commodities.
    Energy(EnergyType),
    /// Metal commodities.
    Metals(MetalType),
    /// Agricultural commodities.
    Agriculture(AgricultureType),
}

impl CommodityType {
    /// Returns the broad category name.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            CommodityType::Energy(_) => "Energy",
            CommodityType::Metals(_) => "Metals",
            CommodityType::Agriculture(_) => "Agriculture",
        }
    }
}

impl std::fmt::Display for CommodityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommodityType::Energy(e) => write!(f, "Energy:{:?}", e),
            CommodityType::Metals(m) => write!(f, "Metals:{:?}", m),
            CommodityType::Agriculture(a) => write!(f, "Agriculture:{:?}", a),
        }
    }
}

/// Quantity unit for commodity transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuantityUnit {
    /// Barrels (for oil).
    Barrels,
    /// MMBtu (for natural gas).
    MMBtu,
    /// Metric tonnes.
    MetricTonnes,
    /// Troy ounces (for precious metals).
    TroyOunces,
    /// Pounds.
    Pounds,
    /// Bushels (for grains).
    Bushels,
    /// MWh (for electricity).
    MWh,
}

/// Commodity forward contract.
///
/// An agreement to buy/sell a commodity at a predetermined price
/// on a future delivery date.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommodityForward {
    /// Commodity type.
    pub commodity: CommodityType,
    /// Delivery location (e.g., "Cushing, OK" for WTI).
    pub delivery_location: String,
    /// Delivery date.
    pub delivery_date: Date,
    /// Quantity.
    pub quantity: f64,
    /// Quantity unit.
    pub unit: QuantityUnit,
    /// Forward price per unit.
    pub forward_price: f64,
    /// Currency.
    pub currency: Currency,
}

impl CommodityForward {
    /// Validates the commodity forward parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.quantity <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Quantity must be positive",
            ));
        }
        if self.forward_price <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Forward price must be positive",
            ));
        }
        if self.delivery_location.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Delivery location must be specified",
            ));
        }
        Ok(())
    }

    /// Returns the total notional value.
    #[must_use]
    pub fn notional_value(&self) -> f64 { self.quantity * self.forward_price }
}

/// Commodity swap.
///
/// A swap exchanging a fixed price for a floating commodity price index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommoditySwap {
    /// Commodity type.
    pub commodity: CommodityType,
    /// Fixed price per unit.
    pub fixed_price: f64,
    /// Floating price index reference (e.g., "WTI Cushing Spot").
    pub floating_index: String,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Quantity per period.
    pub quantity_per_period: f64,
    /// Quantity unit.
    pub unit: QuantityUnit,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Currency.
    pub currency: Currency,
}

impl CommoditySwap {
    /// Validates the commodity swap parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.quantity_per_period <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Quantity per period must be positive",
            ));
        }
        if self.fixed_price <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Fixed price must be positive",
            ));
        }
        if self.floating_index.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Floating index must be specified",
            ));
        }
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        Ok(())
    }
}

/// Commodity vanilla option.
///
/// A standard option on a commodity.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommodityVanillaOption {
    /// Commodity type.
    pub commodity: CommodityType,
    /// Strike price per unit.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style.
    pub exercise_style: ExerciseStyle,
    /// Quantity.
    pub quantity: f64,
    /// Quantity unit.
    pub unit: QuantityUnit,
    /// Settlement type (Cash or Physical).
    pub settlement_type: SettlementType,
    /// Currency.
    pub currency: Currency,
}

impl CommodityVanillaOption {
    /// Validates the commodity vanilla option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.quantity <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Quantity must be positive",
            ));
        }
        if self.strike <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be positive",
            ));
        }
        Ok(())
    }
}

/// Commodity Asian option.
///
/// An option whose payoff depends on the average commodity price.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommodityAsianOption {
    /// Commodity type.
    pub commodity: CommodityType,
    /// Strike price per unit.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Averaging start date.
    pub averaging_start: Date,
    /// Averaging end date.
    pub averaging_end: Date,
    /// Observation frequency.
    pub observation_frequency: Frequency,
    /// Quantity.
    pub quantity: f64,
    /// Quantity unit.
    pub unit: QuantityUnit,
    /// Currency.
    pub currency: Currency,
}

impl CommodityAsianOption {
    /// Validates the commodity Asian option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.quantity <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Quantity must be positive",
            ));
        }
        if self.strike <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be positive",
            ));
        }
        if self.averaging_end <= self.averaging_start {
            return Err(InstrumentError::invalid_date(
                "Averaging end must be after averaging start",
            ));
        }
        if self.expiry < self.averaging_end {
            return Err(InstrumentError::invalid_date(
                "Expiry must be on or after averaging end",
            ));
        }
        Ok(())
    }
}

/// Spread option (on two commodities).
///
/// An option on the price differential between two commodities.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpreadOption {
    /// First commodity.
    pub commodity_1: CommodityType,
    /// Second commodity.
    pub commodity_2: CommodityType,
    /// Spread strike (commodity_1 price - commodity_2 price).
    pub spread_strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put on the spread).
    pub option_type: OptionType,
    /// Quantity.
    pub quantity: f64,
    /// Quantity unit (assumed same for both commodities).
    pub unit: QuantityUnit,
    /// Currency.
    pub currency: Currency,
}

impl SpreadOption {
    /// Validates the spread option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.quantity <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Quantity must be positive",
            ));
        }
        // Note: spread strike can be negative (e.g., crack spread)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_type_equality() {
        assert_eq!(EnergyType::CrudeOil, EnergyType::CrudeOil);
        assert_ne!(EnergyType::CrudeOil, EnergyType::NaturalGas);
    }

    #[test]
    fn test_metal_type_equality() {
        assert_eq!(MetalType::Gold, MetalType::Gold);
        assert_ne!(MetalType::Gold, MetalType::Silver);
    }

    #[test]
    fn test_agriculture_type_equality() {
        assert_eq!(AgricultureType::Wheat, AgricultureType::Wheat);
        assert_ne!(AgricultureType::Wheat, AgricultureType::Corn);
    }

    #[test]
    fn test_commodity_type_category() {
        let oil = CommodityType::Energy(EnergyType::CrudeOil);
        assert_eq!(oil.category(), "Energy");

        let gold = CommodityType::Metals(MetalType::Gold);
        assert_eq!(gold.category(), "Metals");

        let wheat = CommodityType::Agriculture(AgricultureType::Wheat);
        assert_eq!(wheat.category(), "Agriculture");
    }

    #[test]
    fn test_commodity_type_display() {
        let oil = CommodityType::Energy(EnergyType::CrudeOil);
        assert!(oil.to_string().contains("Energy"));
        assert!(oil.to_string().contains("CrudeOil"));
    }

    fn make_test_forward() -> CommodityForward {
        CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.50,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_commodity_forward_validate_success() {
        let fwd = make_test_forward();
        assert!(fwd.validate().is_ok());
    }

    #[test]
    fn test_commodity_forward_validate_negative_quantity() {
        let mut fwd = make_test_forward();
        fwd.quantity = -100.0;
        assert!(fwd.validate().is_err());
    }

    #[test]
    fn test_commodity_forward_validate_empty_location() {
        let mut fwd = make_test_forward();
        fwd.delivery_location = "".to_string();
        assert!(fwd.validate().is_err());
    }

    #[test]
    fn test_commodity_forward_notional_value() {
        let fwd = make_test_forward();
        assert!((fwd.notional_value() - 75500.0).abs() < 0.01);
    }

    fn make_test_swap() -> CommoditySwap {
        CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            fixed_price: 3.50,
            floating_index: "Henry Hub".to_string(),
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2026, 1, 1).unwrap(),
            quantity_per_period: 10000.0,
            unit: QuantityUnit::MMBtu,
            payment_frequency: Frequency::Monthly,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_commodity_swap_validate_success() {
        let swap = make_test_swap();
        assert!(swap.validate().is_ok());
    }

    #[test]
    fn test_commodity_swap_validate_invalid_dates() {
        let mut swap = make_test_swap();
        swap.maturity = Date::from_ymd(2024, 1, 1).unwrap();
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_commodity_swap_validate_empty_index() {
        let mut swap = make_test_swap();
        swap.floating_index = "".to_string();
        assert!(swap.validate().is_err());
    }

    fn make_test_vanilla_option() -> CommodityVanillaOption {
        CommodityVanillaOption {
            commodity: CommodityType::Metals(MetalType::Gold),
            strike: 2000.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 100.0,
            unit: QuantityUnit::TroyOunces,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_commodity_vanilla_option_validate_success() {
        let option = make_test_vanilla_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_commodity_vanilla_option_validate_negative_quantity() {
        let mut option = make_test_vanilla_option();
        option.quantity = -10.0;
        assert!(option.validate().is_err());
    }

    fn make_test_asian_option() -> CommodityAsianOption {
        CommodityAsianOption {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            strike: 70.0,
            expiry: Date::from_ymd(2025, 12, 31).unwrap(),
            option_type: OptionType::Call,
            averaging_start: Date::from_ymd(2025, 10, 1).unwrap(),
            averaging_end: Date::from_ymd(2025, 12, 31).unwrap(),
            observation_frequency: Frequency::Daily,
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_commodity_asian_option_validate_success() {
        let option = make_test_asian_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_commodity_asian_option_validate_invalid_averaging_dates() {
        let mut option = make_test_asian_option();
        option.averaging_end = Date::from_ymd(2025, 9, 1).unwrap(); // Before start
        assert!(option.validate().is_err());
    }

    #[test]
    fn test_commodity_asian_option_validate_expiry_before_averaging() {
        let mut option = make_test_asian_option();
        option.expiry = Date::from_ymd(2025, 11, 1).unwrap(); // Before averaging end
        assert!(option.validate().is_err());
    }

    fn make_test_spread_option() -> SpreadOption {
        SpreadOption {
            commodity_1: CommodityType::Energy(EnergyType::CrudeOil),
            commodity_2: CommodityType::Energy(EnergyType::HeatingOil),
            spread_strike: 10.0, // Crack spread
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_spread_option_validate_success() {
        let option = make_test_spread_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_spread_option_validate_negative_quantity() {
        let mut option = make_test_spread_option();
        option.quantity = -100.0;
        assert!(option.validate().is_err());
    }

    #[test]
    fn test_spread_option_negative_strike_allowed() {
        let mut option = make_test_spread_option();
        option.spread_strike = -5.0; // Negative spread is valid
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_quantity_unit_equality() {
        assert_eq!(QuantityUnit::Barrels, QuantityUnit::Barrels);
        assert_ne!(QuantityUnit::Barrels, QuantityUnit::MMBtu);
    }
}

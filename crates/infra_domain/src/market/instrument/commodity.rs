//! Commodity instrument definitions.

use super::{common::ExerciseStyle, error::InstrumentError};
use crate::{
    market::Currency,
    time::{Date, Frequency},
    trade::{OptionType, SettlementType},
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commodity_types() {
        assert_eq!(EnergyType::CrudeOil, EnergyType::CrudeOil);
        assert_ne!(EnergyType::CrudeOil, EnergyType::NaturalGas);
        assert_eq!(MetalType::Gold, MetalType::Gold);
        assert_ne!(MetalType::Gold, MetalType::Silver);
        assert_eq!(AgricultureType::Wheat, AgricultureType::Wheat);
        assert_ne!(AgricultureType::Wheat, AgricultureType::Corn);
        assert_eq!(QuantityUnit::Barrels, QuantityUnit::Barrels);
        assert_ne!(QuantityUnit::Barrels, QuantityUnit::MMBtu);

        let oil = CommodityType::Energy(EnergyType::CrudeOil);
        assert_eq!(oil.category(), "Energy");
        assert!(oil.to_string().contains("Energy"));
        assert!(oil.to_string().contains("CrudeOil"));
        assert_eq!(CommodityType::Metals(MetalType::Gold).category(), "Metals");
        assert_eq!(
            CommodityType::Agriculture(AgricultureType::Wheat).category(),
            "Agriculture"
        );
    }

    #[test]
    fn test_commodity_instruments_validation() {
        let fwd = CommodityForward {
            commodity: CommodityType::Energy(EnergyType::CrudeOil),
            delivery_location: "Cushing, OK".to_string(),
            delivery_date: Date::from_ymd(2025, 6, 15).unwrap(),
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            forward_price: 75.50,
            currency: Currency::USD,
        };
        assert!(fwd.validate().is_ok());
        assert!((fwd.notional_value() - 75500.0).abs() < 0.01);
        let mut bad = fwd.clone();
        bad.quantity = -100.0;
        assert!(bad.validate().is_err());
        let mut bad = fwd.clone();
        bad.delivery_location = "".to_string();
        assert!(bad.validate().is_err());

        let swap = CommoditySwap {
            commodity: CommodityType::Energy(EnergyType::NaturalGas),
            fixed_price: 3.50,
            floating_index: "Henry Hub".to_string(),
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2026, 1, 1).unwrap(),
            quantity_per_period: 10000.0,
            unit: QuantityUnit::MMBtu,
            payment_frequency: Frequency::Monthly,
            currency: Currency::USD,
        };
        assert!(swap.validate().is_ok());
        let mut bad = swap.clone();
        bad.maturity = Date::from_ymd(2024, 1, 1).unwrap();
        assert!(bad.validate().is_err());
        let mut bad = swap.clone();
        bad.floating_index = "".to_string();
        assert!(bad.validate().is_err());

        let opt = CommodityVanillaOption {
            commodity: CommodityType::Metals(MetalType::Gold),
            strike: 2000.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            quantity: 100.0,
            unit: QuantityUnit::TroyOunces,
            settlement_type: SettlementType::Cash,
            currency: Currency::USD,
        };
        assert!(opt.validate().is_ok());
        let mut bad = opt.clone();
        bad.quantity = -10.0;
        assert!(bad.validate().is_err());

        let asian = CommodityAsianOption {
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
        };
        assert!(asian.validate().is_ok());
        let mut bad = asian.clone();
        bad.averaging_end = Date::from_ymd(2025, 9, 1).unwrap();
        assert!(bad.validate().is_err());
        let mut bad = asian.clone();
        bad.expiry = Date::from_ymd(2025, 11, 1).unwrap();
        assert!(bad.validate().is_err());

        let spread = SpreadOption {
            commodity_1: CommodityType::Energy(EnergyType::CrudeOil),
            commodity_2: CommodityType::Energy(EnergyType::HeatingOil),
            spread_strike: 10.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            quantity: 1000.0,
            unit: QuantityUnit::Barrels,
            currency: Currency::USD,
        };
        assert!(spread.validate().is_ok());
        let mut bad = spread.clone();
        bad.quantity = -100.0;
        assert!(bad.validate().is_err());
        let mut ok = spread.clone();
        ok.spread_strike = -5.0;
        assert!(ok.validate().is_ok());
    }
}

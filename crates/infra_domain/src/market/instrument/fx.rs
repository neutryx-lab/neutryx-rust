//! Foreign exchange instrument definitions.
//!
//! This module provides definitions for FX derivatives including
//! spots, forwards, vanilla options, barrier options, and FX swaps.

use super::{
    common::{BarrierDirection, BarrierType, ExerciseStyle},
    error::InstrumentError,
};
use crate::{trade::OptionType, Currency, Date};

// Re-export CurrencyPair from market module
pub use crate::market::CurrencyPair;

/// FX spot transaction.
///
/// An immediate exchange of currencies at the current spot rate,
/// typically settling T+2.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSpot {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Spot rate (quote currency per unit of base currency).
    pub spot_rate: f64,
    /// Settlement date.
    pub settlement_date: Date,
    /// Notional amount in the notional currency.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxSpot {
    /// Validates the FX spot parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.spot_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Spot rate must be positive",
            ));
        }
        Ok(())
    }
}

/// FX forward transaction.
///
/// An agreement to exchange currencies at a predetermined rate
/// on a future date.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxForward {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Forward rate (quote currency per unit of base currency).
    pub forward_rate: f64,
    /// Settlement date.
    pub settlement_date: Date,
    /// Notional amount in the notional currency.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxForward {
    /// Validates the FX forward parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.forward_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Forward rate must be positive",
            ));
        }
        Ok(())
    }
}

/// FX vanilla option.
///
/// A standard European or American option to exchange currencies
/// at a predetermined strike rate.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxVanillaOption {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Strike rate.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Delivery date (typically spot after expiry).
    pub delivery_date: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style (European, American, Bermudan).
    pub exercise_style: ExerciseStyle,
    /// Notional amount.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxVanillaOption {
    /// Validates the FX vanilla option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.strike <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be positive",
            ));
        }
        if self.delivery_date < self.expiry {
            return Err(InstrumentError::invalid_date(
                "Delivery date must be on or after expiry",
            ));
        }
        Ok(())
    }
}

/// FX barrier option.
///
/// An option with a barrier that, if breached, either activates
/// (knock-in) or deactivates (knock-out) the option.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxBarrierOption {
    /// Underlying vanilla option.
    pub vanilla: FxVanillaOption,
    /// Barrier level.
    pub barrier_level: f64,
    /// Barrier type (KnockIn or KnockOut).
    pub barrier_type: BarrierType,
    /// Barrier direction (Up or Down).
    pub barrier_direction: BarrierDirection,
    /// Rebate amount (paid if option is knocked out).
    pub rebate: Option<f64>,
}

impl FxBarrierOption {
    /// Validates the FX barrier option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        self.vanilla.validate()?;

        if self.barrier_level <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Barrier level must be positive",
            ));
        }

        if let Some(rebate) = self.rebate {
            if rebate < 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Rebate must be non-negative",
                ));
            }
        }

        // Validate barrier vs strike consistency
        match (self.barrier_direction, self.vanilla.option_type) {
            (BarrierDirection::Up, OptionType::Call) => {
                if self.barrier_level <= self.vanilla.strike {
                    return Err(InstrumentError::invalid_parameter(
                        "Up-and-in/out call barrier must be above strike",
                    ));
                }
            }
            (BarrierDirection::Down, OptionType::Put) => {
                if self.barrier_level >= self.vanilla.strike {
                    return Err(InstrumentError::invalid_parameter(
                        "Down-and-in/out put barrier must be below strike",
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// FX swap (short-term swap).
///
/// A combination of a spot and forward transaction,
/// exchanging currencies on the near leg and reversing on the far leg.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwap {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Near leg date.
    pub near_leg_date: Date,
    /// Far leg date.
    pub far_leg_date: Date,
    /// Near leg rate.
    pub near_rate: f64,
    /// Far leg rate.
    pub far_rate: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxSwap {
    /// Validates the FX swap parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.near_rate <= 0.0 || self.far_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter("Rates must be positive"));
        }
        if self.far_leg_date <= self.near_leg_date {
            return Err(InstrumentError::invalid_date(
                "Far leg date must be after near leg date",
            ));
        }
        Ok(())
    }

    /// Returns the swap points (far rate - near rate).
    #[must_use]
    pub fn swap_points(&self) -> f64 { self.far_rate - self.near_rate }
}

// ============================================================================
// FX Swap Calibration Instruments
// ============================================================================

/// Swap points with scaling factor for forward rate calculation.
///
/// Swap points represent the difference between forward and spot rates,
/// typically quoted in "pips" with a scaling factor that varies by currency
/// pair.
///
/// # Scaling Factors by Currency Pair
///
/// - **EURUSD, GBPUSD, etc.**: 10000 (4 decimal places)
/// - **USDJPY, EURJPY**: 100 (2 decimal places)
///
/// # Example
///
/// ```rust
/// use infra_domain::trade::instrument_def::SwapPoints;
///
/// // EURUSD swap points: 50 pips = 0.0050
/// let sp = SwapPoints::for_eurusd(50.0);
/// let forward = sp.to_forward_rate(1.1000);
/// assert!((forward - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapPoints {
    /// Raw swap points value (e.g., 50 for 50 pips).
    value: f64,
    /// Scaling factor (e.g., 10000 for EURUSD, 100 for USDJPY).
    scaling_factor: f64,
}

impl SwapPoints {
    /// Creates new swap points with explicit scaling factor.
    #[must_use]
    pub fn new(value: f64, scaling_factor: f64) -> Self {
        Self {
            value,
            scaling_factor,
        }
    }

    /// Creates swap points for EURUSD-like pairs (scaling factor = 10000).
    #[must_use]
    pub fn for_eurusd(value: f64) -> Self { Self::new(value, 10000.0) }

    /// Creates swap points for USDJPY-like pairs (scaling factor = 100).
    #[must_use]
    pub fn for_usdjpy(value: f64) -> Self { Self::new(value, 100.0) }

    /// Creates swap points from rate difference.
    ///
    /// Calculates the swap points value given spot, forward, and scaling
    /// factor.
    #[must_use]
    pub fn from_rate_difference(spot: f64, forward: f64, scaling_factor: f64) -> Self {
        let value = (forward - spot) * scaling_factor;
        Self::new(value, scaling_factor)
    }

    /// Returns the raw swap points value.
    #[inline]
    #[must_use]
    pub fn value(&self) -> f64 { self.value }

    /// Returns the scaling factor.
    #[inline]
    #[must_use]
    pub fn scaling_factor(&self) -> f64 { self.scaling_factor }

    /// Converts to forward rate given spot rate.
    ///
    /// Formula: F = S + swap_points / scaling_factor
    #[inline]
    #[must_use]
    pub fn to_forward_rate(&self, spot: f64) -> f64 { spot + self.value / self.scaling_factor }

    /// Returns the swap points as a decimal rate adjustment.
    #[inline]
    #[must_use]
    pub fn as_decimal(&self) -> f64 { self.value / self.scaling_factor }
}

impl std::fmt::Display for SwapPoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} pips", self.value)
    }
}

/// Standard FX swap tenors for short-term forward curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxSwapTenor {
    /// Overnight (T+0 to T+1).
    ON,
    /// Tomorrow-Next (T+1 to T+2).
    TN,
    /// Spot-Next (T+2 to T+3).
    SN,
    /// 1 Week.
    W1,
    /// 2 Weeks.
    W2,
    /// 1 Month.
    M1,
    /// 2 Months.
    M2,
    /// 3 Months.
    M3,
    /// 6 Months.
    M6,
    /// 9 Months.
    M9,
    /// 1 Year.
    Y1,
}

impl FxSwapTenor {
    /// Returns the tenor name as a string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::ON => "ON",
            Self::TN => "TN",
            Self::SN => "SN",
            Self::W1 => "1W",
            Self::W2 => "2W",
            Self::M1 => "1M",
            Self::M2 => "2M",
            Self::M3 => "3M",
            Self::M6 => "6M",
            Self::M9 => "9M",
            Self::Y1 => "1Y",
        }
    }
}

impl std::fmt::Display for FxSwapTenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// FX swap convention for business day and settlement rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwapConvention {
    /// Number of business days from trade date to spot settlement.
    pub spot_lag: u32,
}

impl Default for FxSwapConvention {
    fn default() -> Self { Self { spot_lag: 2 } }
}

impl FxSwapConvention {
    /// Creates a convention with spot lag = 2 (standard T+2).
    #[must_use]
    pub fn standard() -> Self { Self::default() }

    /// Creates a convention with spot lag = 1 (for CAD, TRY, etc.).
    #[must_use]
    pub fn t_plus_1() -> Self { Self { spot_lag: 1 } }
}

/// FX Swap Instrument for forward point bootstrapping.
///
/// This structure is designed for calibration purposes, storing swap points
/// rather than outright rates, with associated conventions.
///
/// # Example
///
/// ```rust
/// use infra_domain::trade::instrument_def::{
///     FxSwapInstrument, SwapPoints, FxSwapConvention, CurrencyPair,
/// };
/// use infra_domain::{Currency, Date};
///
/// let inst = FxSwapInstrument {
///     currency_pair: CurrencyPair::new(Currency::EUR, Currency::USD),
///     near_date: Date::from_ymd(2025, 1, 3).unwrap(),
///     far_date: Date::from_ymd(2025, 4, 3).unwrap(),
///     spot_rate: 1.1000,
///     swap_points: SwapPoints::for_eurusd(50.0),
///     convention: FxSwapConvention::default(),
/// };
///
/// // Get implied forward rate
/// let forward = inst.implied_forward_rate();
/// assert!((forward - 1.1050).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwapInstrument {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Near leg date (typically spot date).
    pub near_date: Date,
    /// Far leg date (forward settlement date).
    pub far_date: Date,
    /// Spot rate for the near leg.
    pub spot_rate: f64,
    /// Swap points for forward calculation.
    pub swap_points: SwapPoints,
    /// Market convention.
    pub convention: FxSwapConvention,
}

impl FxSwapInstrument {
    /// Returns the implied forward rate.
    ///
    /// Formula: F = S + swap_points / scaling_factor
    #[must_use]
    pub fn implied_forward_rate(&self) -> f64 { self.swap_points.to_forward_rate(self.spot_rate) }

    /// Validates the FX swap instrument parameters.
    pub fn validate(&self) -> Result<(), FxSwapError> {
        if self.spot_rate <= 0.0 {
            return Err(FxSwapError::InvalidSpotRate(self.spot_rate));
        }
        if self.far_date <= self.near_date {
            return Err(FxSwapError::InvalidDates {
                near: self.near_date,
                far: self.far_date,
            });
        }
        Ok(())
    }
}

/// Errors specific to FX swap operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FxSwapError {
    /// Invalid swap dates (near >= far).
    #[error("Invalid swap dates: near {near} >= far {far}")]
    InvalidDates {
        /// Near leg date.
        near: Date,
        /// Far leg date.
        far: Date,
    },

    /// Invalid spot rate.
    #[error("Invalid spot rate: {0} (must be positive)")]
    InvalidSpotRate(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_currency_pair() -> CurrencyPair {
        CurrencyPair::new(Currency::EUR, Currency::USD)
    }

    // CurrencyPair tests are now in market/currency_pair.rs

    #[test]
    fn test_fx_spot_validate_success() {
        let spot = FxSpot {
            currency_pair: make_test_currency_pair(),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(spot.validate().is_ok());
    }

    #[test]
    fn test_fx_spot_validate_negative_notional() {
        let spot = FxSpot {
            currency_pair: make_test_currency_pair(),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: -1_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(spot.validate().is_err());
    }

    #[test]
    fn test_fx_forward_validate_success() {
        let fwd = FxForward {
            currency_pair: make_test_currency_pair(),
            forward_rate: 1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(fwd.validate().is_ok());
    }

    #[test]
    fn test_fx_forward_validate_negative_rate() {
        let fwd = FxForward {
            currency_pair: make_test_currency_pair(),
            forward_rate: -1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(fwd.validate().is_err());
    }

    fn make_test_vanilla_option() -> FxVanillaOption {
        FxVanillaOption {
            currency_pair: make_test_currency_pair(),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
    }

    #[test]
    fn test_fx_vanilla_option_validate_success() {
        let option = make_test_vanilla_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_fx_vanilla_option_validate_invalid_dates() {
        let mut option = make_test_vanilla_option();
        option.delivery_date = Date::from_ymd(2025, 6, 14).unwrap(); // before expiry
        assert!(option.validate().is_err());
    }

    #[test]
    fn test_fx_barrier_option_validate_success() {
        let barrier = FxBarrierOption {
            vanilla: make_test_vanilla_option(),
            barrier_level: 1.1500, // above strike for up-and-out call
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(0.001),
        };
        assert!(barrier.validate().is_ok());
    }

    #[test]
    fn test_fx_barrier_option_validate_invalid_barrier_level() {
        let barrier = FxBarrierOption {
            vanilla: make_test_vanilla_option(),
            barrier_level: 1.0500, // below strike (invalid for up-and-out call)
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: None,
        };
        assert!(barrier.validate().is_err());
    }

    #[test]
    fn test_fx_swap_validate_success() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(swap.validate().is_ok());
    }

    #[test]
    fn test_fx_swap_validate_invalid_dates() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // before near leg
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_fx_swap_points() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!((swap.swap_points() - 0.0020).abs() < 1e-10);
    }

    // === SwapPoints Tests ===

    #[test]
    fn test_swap_points_new() {
        let sp = SwapPoints::new(50.0, 10000.0);
        assert!((sp.value() - 50.0).abs() < 1e-10);
        assert!((sp.scaling_factor() - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_to_forward_rate_eurusd() {
        // EURUSD: scaling factor = 10000
        let sp = SwapPoints::new(50.0, 10000.0);
        let spot = 1.1000;
        let forward = sp.to_forward_rate(spot);
        // F = S + swap_points / scaling_factor = 1.1000 + 50/10000 = 1.1050
        assert!((forward - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_to_forward_rate_usdjpy() {
        // USDJPY: scaling factor = 100
        let sp = SwapPoints::new(-25.0, 100.0);
        let spot = 150.00;
        let forward = sp.to_forward_rate(spot);
        // F = S + swap_points / scaling_factor = 150.00 + (-25)/100 = 149.75
        assert!((forward - 149.75).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_for_eurusd() {
        let sp = SwapPoints::for_eurusd(50.0);
        assert!((sp.scaling_factor() - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_for_usdjpy() {
        let sp = SwapPoints::for_usdjpy(-25.0);
        assert!((sp.scaling_factor() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_from_rate_difference() {
        // Given spot and forward, calculate swap points
        let spot = 1.1000;
        let forward = 1.1050;
        let scaling = 10000.0;
        let sp = SwapPoints::from_rate_difference(spot, forward, scaling);
        assert!((sp.value() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_points_display() {
        let sp = SwapPoints::new(50.0, 10000.0);
        assert_eq!(sp.to_string(), "50.00 pips");
    }

    // === FxSwapTenor Tests ===

    #[test]
    fn test_fx_swap_tenor_names() {
        assert_eq!(FxSwapTenor::ON.name(), "ON");
        assert_eq!(FxSwapTenor::TN.name(), "TN");
        assert_eq!(FxSwapTenor::SN.name(), "SN");
        assert_eq!(FxSwapTenor::W1.name(), "1W");
        assert_eq!(FxSwapTenor::M3.name(), "3M");
        assert_eq!(FxSwapTenor::Y1.name(), "1Y");
    }

    // === FxSwapConvention Tests ===

    #[test]
    fn test_fx_swap_convention_default() {
        let conv = FxSwapConvention::default();
        assert_eq!(conv.spot_lag, 2);
    }

    // === FxSwapInstrument Tests ===

    #[test]
    fn test_fx_swap_instrument_implied_forward() {
        let inst = FxSwapInstrument {
            currency_pair: make_test_currency_pair(),
            near_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_date: Date::from_ymd(2025, 4, 3).unwrap(),
            spot_rate: 1.1000,
            swap_points: SwapPoints::for_eurusd(50.0),
            convention: FxSwapConvention::default(),
        };
        assert!((inst.implied_forward_rate() - 1.1050).abs() < 1e-10);
    }

    #[test]
    fn test_fx_swap_instrument_validate_success() {
        let inst = FxSwapInstrument {
            currency_pair: make_test_currency_pair(),
            near_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_date: Date::from_ymd(2025, 4, 3).unwrap(),
            spot_rate: 1.1000,
            swap_points: SwapPoints::for_eurusd(50.0),
            convention: FxSwapConvention::default(),
        };
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_fx_swap_instrument_validate_invalid_dates() {
        let inst = FxSwapInstrument {
            currency_pair: make_test_currency_pair(),
            near_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_date: Date::from_ymd(2025, 1, 3).unwrap(), // far before near
            spot_rate: 1.1000,
            swap_points: SwapPoints::for_eurusd(50.0),
            convention: FxSwapConvention::default(),
        };
        let result = inst.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_fx_swap_instrument_validate_invalid_spot() {
        let inst = FxSwapInstrument {
            currency_pair: make_test_currency_pair(),
            near_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_date: Date::from_ymd(2025, 4, 3).unwrap(),
            spot_rate: -1.0, // invalid
            swap_points: SwapPoints::for_eurusd(50.0),
            convention: FxSwapConvention::default(),
        };
        let result = inst.validate();
        assert!(result.is_err());
    }
}

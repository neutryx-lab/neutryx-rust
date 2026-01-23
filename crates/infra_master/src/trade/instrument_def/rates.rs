//! Interest rate instrument definitions.
//!
//! This module provides definitions for interest rate derivatives including
//! swaptions, caps/floors, FRNs, CMS swaps, and inflation swaps.

use super::{
    common::{NotionalSchedule, PayerReceiver},
    error::InstrumentError,
};
use crate::{
    trade::{ExerciseType, SettlementType},
    Currency, Date, Frequency, RateIndex, Tenor,
};

/// Swaption (option on an interest rate swap).
///
/// Represents the right (but not obligation) to enter into an underlying
/// interest rate swap at a future date.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Swaption {
    /// Tenor of the underlying swap.
    pub underlying_swap_tenor: Tenor,
    /// Expiry date of the option.
    pub expiry: Date,
    /// Exercise type (European, Bermudan, American).
    pub exercise_type: ExerciseType,
    /// Settlement type (Cash or Physical delivery).
    pub settlement_type: SettlementType,
    /// Strike rate (as decimal, e.g., 0.03 for 3%).
    pub strike: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency of the swaption.
    pub currency: Currency,
    /// Payer or Receiver swaption.
    pub payer_receiver: PayerReceiver,
}

impl Swaption {
    /// Validates the swaption parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.strike < 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be non-negative",
            ));
        }
        Ok(())
    }
}

/// Cap or Floor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CapFloorType {
    /// Cap (call option on interest rates).
    Cap,
    /// Floor (put option on interest rates).
    Floor,
    /// Collar (combination of cap and floor).
    Collar,
}

/// Interest rate cap or floor.
///
/// A cap is a series of call options (caplets) on an interest rate index.
/// A floor is a series of put options (floorlets) on an interest rate index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapFloor {
    /// Type of cap/floor (Cap, Floor, or Collar).
    pub cap_floor_type: CapFloorType,
    /// Strike rates (single strike for cap/floor, two for collar).
    pub strikes: Vec<f64>,
    /// Underlying rate index.
    pub index: RateIndex,
    /// Start date.
    pub start_date: Date,
    /// Tenor of the cap/floor.
    pub tenor: Tenor,
    /// Notional schedule (can be amortising).
    pub notional_schedule: NotionalSchedule,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Currency.
    pub currency: Currency,
}

impl CapFloor {
    /// Validates the cap/floor parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.strikes.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "At least one strike required",
            ));
        }

        match self.cap_floor_type {
            CapFloorType::Cap | CapFloorType::Floor => {
                if self.strikes.len() != 1 {
                    return Err(InstrumentError::invalid_parameter(
                        "Cap/Floor requires exactly one strike",
                    ));
                }
            }
            CapFloorType::Collar => {
                if self.strikes.len() != 2 {
                    return Err(InstrumentError::invalid_parameter(
                        "Collar requires exactly two strikes",
                    ));
                }
                if self.strikes[0] >= self.strikes[1] {
                    return Err(InstrumentError::invalid_parameter(
                        "Collar floor strike must be less than cap strike",
                    ));
                }
            }
        }

        for strike in &self.strikes {
            if *strike < 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Strike must be non-negative",
                ));
            }
        }

        Ok(())
    }
}

/// Floating rate note (FRN).
///
/// A bond with coupon payments linked to a floating rate index.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frn {
    /// Coupon rate index.
    pub coupon_index: RateIndex,
    /// Spread over the index (as decimal).
    pub spread: f64,
    /// Reset frequency for the coupon.
    pub reset_frequency: Frequency,
    /// Principal repayment schedule (amortising/bullet).
    pub principal_schedule: NotionalSchedule,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Currency.
    pub currency: Currency,
}

impl Frn {
    /// Validates the FRN parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        Ok(())
    }
}

/// Constant Maturity Swap (CMS) swap.
///
/// A swap where one leg pays a rate linked to a constant maturity swap rate.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CmsSwap {
    /// CMS reference tenor (e.g., 10Y for 10-year CMS rate).
    pub cms_tenor: Tenor,
    /// Convexity adjustment parameter.
    pub convexity_adjustment: Option<f64>,
    /// Start date.
    pub start_date: Date,
    /// Swap tenor.
    pub tenor: Tenor,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Spread over the CMS rate (as decimal).
    pub spread: f64,
}

impl CmsSwap {
    /// Validates the CMS swap parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        Ok(())
    }
}

/// Inflation swap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwapType {
    /// Zero-coupon inflation swap (single payment at maturity).
    ZeroCoupon,
    /// Year-on-year inflation swap (annual payments).
    YearOnYear,
}

/// Inflation swap.
///
/// A swap where one leg pays based on an inflation index (e.g., CPI).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationSwap {
    /// Inflation index identifier (e.g., "CPI", "HICP").
    pub inflation_index: String,
    /// Lag period in months (typically 2-3 months).
    pub lag_months: u32,
    /// Swap type (zero-coupon or year-on-year).
    pub swap_type: SwapType,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Fixed rate (for the fixed leg).
    pub fixed_rate: f64,
}

impl InflationSwap {
    /// Validates the inflation swap parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.inflation_index.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Inflation index must be specified",
            ));
        }
        Ok(())
    }
}

/// Overnight Index Swap (OIS).
///
/// An interest rate swap where the floating leg pays an overnight rate
/// compounded over the accrual period. Common overnight indices include
/// SOFR (USD), ESTR (EUR), SONIA (GBP), and TONA (JPY).
///
/// # Daily Compounding
///
/// For an OIS floating leg, the interest is calculated using daily compounding:
///
/// ```text
/// Compounded Rate = ∏(1 + ri × di) - 1
/// ```
///
/// where:
/// - `ri` is the overnight rate for day `i`
/// - `di` is the day count fraction for day `i`
///
/// # Example
///
/// ```rust,ignore
/// use infra_master::trade::instrument_def::{Ois, PayerReceiver};
/// use infra_master::{Currency, Date, RateIndex, Frequency};
///
/// let ois = Ois {
///     rate_index: RateIndex::Sofr,
///     fixed_rate: 0.04,
///     start_date: Date::from_ymd(2025, 1, 15).unwrap(),
///     end_date: Date::from_ymd(2030, 1, 15).unwrap(),
///     notional: 10_000_000.0,
///     currency: Currency::USD,
///     payer_receiver: PayerReceiver::Payer,
///     payment_frequency: Frequency::Annual,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ois {
    /// Overnight rate index (SOFR, ESTR, SONIA, TONA, etc.).
    pub rate_index: RateIndex,
    /// Fixed rate (as decimal, e.g., 0.04 for 4%).
    pub fixed_rate: f64,
    /// Start date of the swap.
    pub start_date: Date,
    /// End date (maturity) of the swap.
    pub end_date: Date,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Payer or Receiver of the fixed leg.
    pub payer_receiver: PayerReceiver,
    /// Payment frequency for both legs.
    pub payment_frequency: Frequency,
}

impl Ois {
    /// Validates the OIS parameters.
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.end_date <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "End date must be after start date",
            ));
        }
        Ok(())
    }

    /// Returns true if this is a payer OIS (pay fixed, receive floating).
    #[must_use]
    pub fn is_payer(&self) -> bool { self.payer_receiver == PayerReceiver::Payer }

    /// Returns the swap tenor in years (approximate).
    #[must_use]
    pub fn tenor_years(&self) -> f64 { (self.end_date - self.start_date) as f64 / 365.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_swaption() -> Swaption {
        Swaption {
            underlying_swap_tenor: Tenor::TenYears,
            expiry: Date::from_ymd(2026, 1, 15).unwrap(),
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: 0.03,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        }
    }

    #[test]
    fn test_swaption_validate_success() {
        let swaption = make_test_swaption();
        assert!(swaption.validate().is_ok());
    }

    #[test]
    fn test_swaption_validate_negative_notional() {
        let mut swaption = make_test_swaption();
        swaption.notional = -100.0;
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_swaption_validate_negative_strike() {
        let mut swaption = make_test_swaption();
        swaption.strike = -0.01;
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_swaption_clone() {
        let swaption = make_test_swaption();
        let cloned = swaption.clone();
        assert_eq!(swaption, cloned);
    }

    fn make_test_cap() -> CapFloor {
        CapFloor {
            cap_floor_type: CapFloorType::Cap,
            strikes: vec![0.03],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_cap_validate_success() {
        let cap = make_test_cap();
        assert!(cap.validate().is_ok());
    }

    #[test]
    fn test_cap_validate_empty_strikes() {
        let mut cap = make_test_cap();
        cap.strikes = vec![];
        assert!(cap.validate().is_err());
    }

    #[test]
    fn test_cap_validate_multiple_strikes() {
        let mut cap = make_test_cap();
        cap.strikes = vec![0.02, 0.04];
        assert!(cap.validate().is_err());
    }

    #[test]
    fn test_collar_validate_success() {
        let collar = CapFloor {
            cap_floor_type: CapFloorType::Collar,
            strikes: vec![0.02, 0.04], // floor strike < cap strike
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        };
        assert!(collar.validate().is_ok());
    }

    #[test]
    fn test_collar_validate_invalid_strikes() {
        let collar = CapFloor {
            cap_floor_type: CapFloorType::Collar,
            strikes: vec![0.04, 0.02], // floor strike > cap strike (invalid)
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        };
        assert!(collar.validate().is_err());
    }

    #[test]
    fn test_frn_validate_success() {
        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.001,
            reset_frequency: Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(1_000_000.0),
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            currency: Currency::USD,
        };
        assert!(frn.validate().is_ok());
    }

    #[test]
    fn test_frn_validate_invalid_dates() {
        let frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.001,
            reset_frequency: Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(1_000_000.0),
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(), // maturity before start
            currency: Currency::USD,
        };
        assert!(frn.validate().is_err());
    }

    #[test]
    fn test_cms_swap_validate_success() {
        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: Some(0.001),
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::EUR,
            spread: 0.0,
        };
        assert!(cms.validate().is_ok());
    }

    #[test]
    fn test_cms_swap_validate_negative_notional() {
        let cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional: -100.0,
            currency: Currency::EUR,
            spread: 0.0,
        };
        assert!(cms.validate().is_err());
    }

    #[test]
    fn test_inflation_swap_validate_success() {
        let swap = InflationSwap {
            inflation_index: "CPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(swap.validate().is_ok());
    }

    #[test]
    fn test_inflation_swap_validate_invalid_dates() {
        let swap = InflationSwap {
            inflation_index: "CPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::YearOnYear,
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_inflation_swap_validate_empty_index() {
        let swap = InflationSwap {
            inflation_index: "".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_cap_floor_type_equality() {
        assert_eq!(CapFloorType::Cap, CapFloorType::Cap);
        assert_ne!(CapFloorType::Cap, CapFloorType::Floor);
    }

    #[test]
    fn test_swap_type_equality() {
        assert_eq!(SwapType::ZeroCoupon, SwapType::ZeroCoupon);
        assert_ne!(SwapType::ZeroCoupon, SwapType::YearOnYear);
    }
}

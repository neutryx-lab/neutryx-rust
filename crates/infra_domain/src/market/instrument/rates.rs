//! Interest rate instrument definitions.
//!
//! This module provides definitions for interest rate derivatives including
//! FRNs, CMS swaps, inflation swaps, OIS, deposits, FRAs, futures, and swaps.
//!
//! Note: Swaptions and Caps/Floors are defined in the `ir_vol` module.

use super::{
    common::{NotionalSchedule, PayerReceiver},
    error::InstrumentError,
};
use crate::{market::{Currency, RateIndex}, time::{Date, EndOfMonthRule, Frequency, Tenor}};

// ============================================================================
// Floating Rate Note (FRN)
// ============================================================================

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

// ============================================================================
// CMS Swap
// ============================================================================

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

// ============================================================================
// Inflation Swap
// ============================================================================

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

// ============================================================================
// Overnight Index Swap (OIS)
// ============================================================================

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
/// use infra_domain::trade::instrument_def::{Ois, PayerReceiver};
/// use infra_domain::{market::{Currency, RateIndex}, time::{Date, Frequency}};
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

// ============================================================================
// Simple Money Market Instruments
// ============================================================================

/// Deposit (money market deposit).
///
/// A simple fixed-rate deposit instrument with a single payment at maturity.
///
/// # Example
///
/// ```rust,ignore
/// use infra_domain::trade::instrument_def::Deposit;
/// use infra_domain::{market::Currency, time::{Date, Tenor}};
///
/// let deposit = Deposit {
///     start_date: Date::from_ymd(2025, 1, 15).unwrap(),
///     tenor: Tenor::ThreeMonths,
///     rate: 0.045,
///     notional: 10_000_000.0,
///     currency: Currency::USD,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Deposit {
    /// Start date of the deposit.
    pub start_date: Date,
    /// Tenor of the deposit.
    pub tenor: Tenor,
    /// Fixed rate (as decimal, e.g., 0.045 for 4.5%).
    pub rate: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl Deposit {
    /// Validates the deposit parameters.
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
        if self.rate < -0.1 || self.rate > 0.5 {
            return Err(InstrumentError::invalid_parameter(
                "Rate must be between -10% and 50%",
            ));
        }
        Ok(())
    }

    /// Returns the end date (maturity) of the deposit.
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust)
    }

    /// Returns the year fraction for the deposit period.
    #[must_use]
    pub fn year_fraction(&self) -> f64 { (self.end_date() - self.start_date) as f64 / 360.0 }
}

/// Forward Rate Agreement (FRA).
///
/// A contract to exchange a fixed rate for a floating rate over a future
/// period.
///
/// # Example
///
/// ```rust,ignore
/// use infra_domain::trade::instrument_def::Fra;
/// use infra_domain::{market::{Currency, RateIndex}, time::{Date, Tenor}};
///
/// let fra = Fra {
///     fixing_date: Date::from_ymd(2025, 3, 15).unwrap(),
///     start_date: Date::from_ymd(2025, 3, 17).unwrap(),
///     tenor: Tenor::ThreeMonths,
///     strike: 0.04,
///     notional: 10_000_000.0,
///     currency: Currency::USD,
///     rate_index: RateIndex::Sofr,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fra {
    /// Fixing date for the floating rate.
    pub fixing_date: Date,
    /// Start date of the FRA period.
    pub start_date: Date,
    /// Tenor of the FRA period.
    pub tenor: Tenor,
    /// Strike rate (fixed rate).
    pub strike: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Rate index for the floating leg.
    pub rate_index: RateIndex,
}

impl Fra {
    /// Validates the FRA parameters.
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
        if self.strike < -0.1 || self.strike > 0.5 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be between -10% and 50%",
            ));
        }
        if self.start_date < self.fixing_date {
            return Err(InstrumentError::invalid_date(
                "Start date must be on or after fixing date",
            ));
        }
        Ok(())
    }

    /// Returns the end date of the FRA period.
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust)
    }

    /// Returns the year fraction for the FRA period.
    #[must_use]
    pub fn year_fraction(&self) -> f64 { (self.end_date() - self.start_date) as f64 / 360.0 }
}

/// Interest Rate Futures contract.
///
/// A standardised exchange-traded contract on short-term interest rates.
///
/// # Example
///
/// ```rust,ignore
/// use infra_domain::trade::instrument_def::Futures;
/// use infra_domain::{market::{Currency, RateIndex}, time::{Date, Tenor}};
///
/// let futures = Futures {
///     expiry_date: Date::from_ymd(2025, 6, 18).unwrap(),
///     underlying_tenor: Tenor::ThreeMonths,
///     price: 95.50,
///     notional: 1_000_000.0,
///     currency: Currency::USD,
///     rate_index: RateIndex::Sofr,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Futures {
    /// Expiry/delivery date of the futures contract.
    pub expiry_date: Date,
    /// Tenor of the underlying rate.
    pub underlying_tenor: Tenor,
    /// Futures price (100 - implied rate).
    pub price: f64,
    /// Contract notional.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Underlying rate index.
    pub rate_index: RateIndex,
}

impl Futures {
    /// Validates the futures parameters.
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
        if self.price < 80.0 || self.price > 110.0 {
            return Err(InstrumentError::invalid_parameter(
                "Price must be between 80 and 110 (implied rate -10% to 20%)",
            ));
        }
        Ok(())
    }

    /// Returns the implied rate from the futures price.
    #[must_use]
    pub fn implied_rate(&self) -> f64 { (100.0 - self.price) / 100.0 }

    /// Returns the end date of the underlying period.
    #[must_use]
    pub fn underlying_end_date(&self) -> Date {
        self.underlying_tenor
            .add_to_date(self.expiry_date, EndOfMonthRule::Adjust)
    }

    /// Returns the year fraction for the underlying period.
    #[must_use]
    pub fn year_fraction(&self) -> f64 {
        (self.underlying_end_date() - self.expiry_date) as f64 / 360.0
    }
}

// ============================================================================
// Interest Rate Swaps
// ============================================================================

/// Interest Rate Swap (IRS).
///
/// A standard fixed-for-floating interest rate swap where one party pays
/// a fixed rate and receives a floating rate (or vice versa).
///
/// # Example
///
/// ```rust,ignore
/// use infra_domain::trade::instrument_def::{InterestRateSwap, PayerReceiver};
/// use infra_domain::{market::{Currency, RateIndex}, time::{Date, Frequency, Tenor}};
///
/// let irs = InterestRateSwap {
///     start_date: Date::from_ymd(2025, 1, 15).unwrap(),
///     tenor: Tenor::FiveYears,
///     fixed_rate: 0.04,
///     spread: 0.0,
///     notional: 10_000_000.0,
///     currency: Currency::USD,
///     payer_receiver: PayerReceiver::Payer,
///     fixed_frequency: Frequency::SemiAnnual,
///     float_frequency: Frequency::Quarterly,
///     rate_index: RateIndex::Sofr,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InterestRateSwap {
    /// Start date of the swap.
    pub start_date: Date,
    /// Tenor of the swap.
    pub tenor: Tenor,
    /// Fixed rate (as decimal, e.g., 0.04 for 4%).
    pub fixed_rate: f64,
    /// Spread over the floating rate index.
    pub spread: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Payer or Receiver of the fixed leg.
    pub payer_receiver: PayerReceiver,
    /// Payment frequency for the fixed leg.
    pub fixed_frequency: Frequency,
    /// Payment frequency for the floating leg.
    pub float_frequency: Frequency,
    /// Rate index for the floating leg.
    pub rate_index: RateIndex,
}

impl InterestRateSwap {
    /// Validates the IRS parameters.
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
        if self.fixed_rate < -0.1 || self.fixed_rate > 0.5 {
            return Err(InstrumentError::invalid_parameter(
                "Fixed rate must be between -10% and 50%",
            ));
        }
        if self.tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Swap tenor must be at least 1 month",
            ));
        }
        Ok(())
    }

    /// Returns the end date of the swap.
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust)
    }

    /// Returns true if this is a payer swap (pay fixed, receive floating).
    #[must_use]
    pub fn is_payer(&self) -> bool { self.payer_receiver == PayerReceiver::Payer }

    /// Returns the swap tenor in years (approximate).
    #[must_use]
    pub fn tenor_years(&self) -> f64 { (self.end_date() - self.start_date) as f64 / 365.0 }
}

/// Basis Swap.
///
/// A swap where both legs are floating, typically referencing different
/// rate indices or tenors (e.g., 3M SOFR vs 6M SOFR, or SOFR vs Fed Funds).
///
/// # Example
///
/// ```rust,ignore
/// use infra_domain::trade::instrument_def::{BasisSwap, PayerReceiver};
/// use infra_domain::{market::{Currency, RateIndex}, time::{Date, Frequency, Tenor}};
///
/// let basis = BasisSwap {
///     start_date: Date::from_ymd(2025, 1, 15).unwrap(),
///     tenor: Tenor::FiveYears,
///     notional: 10_000_000.0,
///     currency: Currency::USD,
///     payer_receiver: PayerReceiver::Payer,
///     leg1_index: RateIndex::Sofr,
///     leg1_spread: 0.0,
///     leg1_frequency: Frequency::Quarterly,
///     leg2_index: RateIndex::Estr,
///     leg2_spread: 0.001,
///     leg2_frequency: Frequency::Quarterly,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasisSwap {
    /// Start date of the swap.
    pub start_date: Date,
    /// Tenor of the swap.
    pub tenor: Tenor,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Direction (Payer pays leg1, receives leg2).
    pub payer_receiver: PayerReceiver,
    /// Rate index for leg 1.
    pub leg1_index: RateIndex,
    /// Spread for leg 1.
    pub leg1_spread: f64,
    /// Payment frequency for leg 1.
    pub leg1_frequency: Frequency,
    /// Rate index for leg 2.
    pub leg2_index: RateIndex,
    /// Spread for leg 2.
    pub leg2_spread: f64,
    /// Payment frequency for leg 2.
    pub leg2_frequency: Frequency,
}

impl BasisSwap {
    /// Validates the basis swap parameters.
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
        if self.tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Swap tenor must be at least 1 month",
            ));
        }
        Ok(())
    }

    /// Returns the end date of the swap.
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust)
    }

    /// Returns the swap tenor in years (approximate).
    #[must_use]
    pub fn tenor_years(&self) -> f64 { (self.end_date() - self.start_date) as f64 / 365.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_swap_type_equality() {
        assert_eq!(SwapType::ZeroCoupon, SwapType::ZeroCoupon);
        assert_ne!(SwapType::ZeroCoupon, SwapType::YearOnYear);
    }

    #[test]
    fn test_ois_validate_success() {
        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            end_date: Date::from_ymd(2030, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: Frequency::Annual,
        };
        assert!(ois.validate().is_ok());
        assert!(ois.is_payer());
    }

    #[test]
    fn test_ois_validate_invalid_dates() {
        let ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2030, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: Frequency::Annual,
        };
        assert!(ois.validate().is_err());
    }

    #[test]
    fn test_deposit_validate_success() {
        let deposit = Deposit {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::ThreeMonths,
            rate: 0.045,
            notional: 10_000_000.0,
            currency: Currency::USD,
        };
        assert!(deposit.validate().is_ok());
    }

    #[test]
    fn test_deposit_end_date() {
        let deposit = Deposit {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::ThreeMonths,
            rate: 0.045,
            notional: 10_000_000.0,
            currency: Currency::USD,
        };
        // 3M from 2025-01-15 = 2025-04-15
        assert_eq!(deposit.end_date(), Date::from_ymd(2025, 4, 15).unwrap());
    }

    #[test]
    fn test_fra_validate_success() {
        let fra = Fra {
            fixing_date: Date::from_ymd(2025, 3, 15).unwrap(),
            start_date: Date::from_ymd(2025, 3, 17).unwrap(),
            tenor: Tenor::ThreeMonths,
            strike: 0.04,
            notional: 10_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(fra.validate().is_ok());
    }

    #[test]
    fn test_fra_validate_invalid_dates() {
        let fra = Fra {
            fixing_date: Date::from_ymd(2025, 3, 20).unwrap(),
            start_date: Date::from_ymd(2025, 3, 15).unwrap(), // start before fixing
            tenor: Tenor::ThreeMonths,
            strike: 0.04,
            notional: 10_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(fra.validate().is_err());
    }

    #[test]
    fn test_futures_validate_success() {
        let futures = Futures {
            expiry_date: Date::from_ymd(2025, 6, 18).unwrap(),
            underlying_tenor: Tenor::ThreeMonths,
            price: 95.50,
            notional: 1_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(futures.validate().is_ok());
        assert!((futures.implied_rate() - 0.045).abs() < 1e-10);
    }

    #[test]
    fn test_futures_validate_invalid_price() {
        let futures = Futures {
            expiry_date: Date::from_ymd(2025, 6, 18).unwrap(),
            underlying_tenor: Tenor::ThreeMonths,
            price: 120.0, // Invalid: too high
            notional: 1_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(futures.validate().is_err());
    }

    #[test]
    fn test_irs_validate_success() {
        let irs = InterestRateSwap {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            fixed_rate: 0.04,
            spread: 0.0,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            fixed_frequency: Frequency::SemiAnnual,
            float_frequency: Frequency::Quarterly,
            rate_index: RateIndex::Sofr,
        };
        assert!(irs.validate().is_ok());
        assert!(irs.is_payer());
    }

    #[test]
    fn test_irs_end_date() {
        let irs = InterestRateSwap {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            fixed_rate: 0.04,
            spread: 0.0,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            fixed_frequency: Frequency::SemiAnnual,
            float_frequency: Frequency::Quarterly,
            rate_index: RateIndex::Sofr,
        };
        // 5Y from 2025-01-15 = 2030-01-15
        assert_eq!(irs.end_date(), Date::from_ymd(2030, 1, 15).unwrap());
    }

    #[test]
    fn test_basis_swap_validate_success() {
        let basis = BasisSwap {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::FiveYears,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            leg1_index: RateIndex::Sofr,
            leg1_spread: 0.0,
            leg1_frequency: Frequency::Quarterly,
            leg2_index: RateIndex::Estr,
            leg2_spread: 0.001,
            leg2_frequency: Frequency::Quarterly,
        };
        assert!(basis.validate().is_ok());
    }

    #[test]
    fn test_basis_swap_validate_invalid_tenor() {
        let basis = BasisSwap {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::Overnight,
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            leg1_index: RateIndex::Sofr,
            leg1_spread: 0.0,
            leg1_frequency: Frequency::Quarterly,
            leg2_index: RateIndex::Estr,
            leg2_spread: 0.001,
            leg2_frequency: Frequency::Quarterly,
        };
        assert!(basis.validate().is_err());
    }
}

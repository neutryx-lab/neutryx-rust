//! Interest rate instrument definitions.

use super::{
    common::{NotionalSchedule, PayerReceiver},
    error::InstrumentError,
};
use crate::{
    market::{Currency, RateIndex},
    time::{Date, EndOfMonthRule, Frequency, Tenor},
};

/// Floating rate note (FRN).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_date_order(
            self.start_date,
            self.maturity,
            "Maturity must be after start date",
        )?;
        Ok(())
    }
}

/// Constant Maturity Swap (CMS) swap.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        Ok(())
    }
}

/// Inflation swap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SwapType {
    /// Zero-coupon inflation swap (single payment at maturity).
    ZeroCoupon,
    /// Year-on-year inflation swap (annual payments).
    YearOnYear,
}

/// Inflation swap.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_date_order(
            self.start_date,
            self.maturity,
            "Maturity must be after start date",
        )?;
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_not_empty(&self.inflation_index, "Inflation Index")?;
        Ok(())
    }
}

/// Overnight Index Swap (OIS).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_date_order(
            self.start_date,
            self.end_date,
            "End date must be after start date",
        )?;
        Ok(())
    }

    /// Returns true if this is a payer OIS (pay fixed, receive floating).
    #[must_use]
    pub fn is_payer(&self) -> bool { self.payer_receiver == PayerReceiver::Payer }

    /// Returns the swap tenor in years (approximate).
    #[must_use]
    pub fn tenor_years(&self) -> f64 { (self.end_date - self.start_date) as f64 / 365.0 }
}

/// Deposit (money market deposit).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_range(self.rate, -0.1, 0.5, "Rate")?;
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_range(self.strike, -0.1, 0.5, "Strike")?;
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_range(self.price, 80.0, 110.0, "Price")?;
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

/// Interest Rate Swap (IRS).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_range(self.fixed_rate, -0.1, 0.5, "Fixed Rate")?;
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

/// Bond type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BondType {
    /// Government bond (e.g., US Treasury, Bund, Gilt, JGB).
    Government,
    /// Corporate bond.
    Corporate,
    /// Agency bond (e.g., FNMA, FHLB).
    Agency,
}

/// Fixed-coupon bond instrument.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bond {
    /// Issuer name (e.g., "US Treasury", "Apple Inc").
    pub issuer: String,
    /// Coupon rate (as decimal, e.g., 0.04375 for 4.375%).
    pub coupon_rate: f64,
    /// Coupon payment frequency.
    pub coupon_frequency: Frequency,
    /// Issue / settlement date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Notional (face value).
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Bond type (government, corporate, agency).
    pub bond_type: BondType,
    /// Credit rating (e.g., "AA+", "A").
    pub rating: Option<String>,
}

impl Bond {
    /// Validates the bond parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_not_empty(&self.issuer, "Issuer")?;
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_range(self.coupon_rate, -0.05, 0.5, "Coupon rate")?;
        InstrumentError::check_date_order(
            self.start_date,
            self.maturity,
            "Maturity must be after start date",
        )?;
        Ok(())
    }

    /// Returns true if this is a government bond.
    #[must_use]
    pub fn is_government(&self) -> bool { self.bond_type == BondType::Government }

    /// Returns true if this is a corporate bond.
    #[must_use]
    pub fn is_corporate(&self) -> bool { self.bond_type == BondType::Corporate }
}

/// Basis Swap.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn validate(&self) -> Result<(), InstrumentError> {
        InstrumentError::check_positive(self.notional, "Notional")?;
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
    fn test_rates_vanilla_instruments() {
        let deposit = Deposit {
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            tenor: Tenor::ThreeMonths,
            rate: 0.045,
            notional: 10_000_000.0,
            currency: Currency::USD,
        };
        assert!(deposit.validate().is_ok());
        assert_eq!(deposit.end_date(), Date::from_ymd(2025, 4, 15).unwrap());

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

        let bad_fra = Fra {
            fixing_date: Date::from_ymd(2025, 3, 20).unwrap(),
            start_date: Date::from_ymd(2025, 3, 15).unwrap(),
            tenor: Tenor::ThreeMonths,
            strike: 0.04,
            notional: 10_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(bad_fra.validate().is_err());

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

        let bad_futures = Futures {
            expiry_date: Date::from_ymd(2025, 6, 18).unwrap(),
            underlying_tenor: Tenor::ThreeMonths,
            price: 120.0,
            notional: 1_000_000.0,
            currency: Currency::USD,
            rate_index: RateIndex::Sofr,
        };
        assert!(bad_futures.validate().is_err());
    }

    #[test]
    fn test_rates_swap_instruments() {
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
        assert_eq!(irs.end_date(), Date::from_ymd(2030, 1, 15).unwrap());

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

        let bad_ois = Ois {
            rate_index: RateIndex::Sofr,
            fixed_rate: 0.04,
            start_date: Date::from_ymd(2030, 1, 15).unwrap(),
            end_date: Date::from_ymd(2025, 1, 15).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
            payment_frequency: Frequency::Annual,
        };
        assert!(bad_ois.validate().is_err());

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

        let bad_basis = BasisSwap {
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
        assert!(bad_basis.validate().is_err());
    }

    #[test]
    fn test_rates_exotic_instruments() {
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

        let bad_frn = Frn {
            coupon_index: RateIndex::Sofr,
            spread: 0.001,
            reset_frequency: Frequency::Quarterly,
            principal_schedule: NotionalSchedule::constant(1_000_000.0),
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(),
            currency: Currency::USD,
        };
        assert!(bad_frn.validate().is_err());

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

        let bad_cms = CmsSwap {
            cms_tenor: Tenor::TenYears,
            convexity_adjustment: None,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional: -100.0,
            currency: Currency::EUR,
            spread: 0.0,
        };
        assert!(bad_cms.validate().is_err());

        let infl = InflationSwap {
            inflation_index: "CPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(infl.validate().is_ok());

        let bad_infl = InflationSwap {
            inflation_index: "CPI".to_string(),
            lag_months: 3,
            swap_type: SwapType::YearOnYear,
            start_date: Date::from_ymd(2030, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(bad_infl.validate().is_err());

        let empty_idx = InflationSwap {
            inflation_index: "".to_string(),
            lag_months: 3,
            swap_type: SwapType::ZeroCoupon,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2030, 1, 1).unwrap(),
            notional: 5_000_000.0,
            currency: Currency::USD,
            fixed_rate: 0.025,
        };
        assert!(empty_idx.validate().is_err());

        assert_eq!(SwapType::ZeroCoupon, SwapType::ZeroCoupon);
        assert_ne!(SwapType::ZeroCoupon, SwapType::YearOnYear);
    }

    #[test]
    fn test_bond_instrument() {
        let bond = Bond {
            issuer: "US Treasury".to_string(),
            coupon_rate: 0.04375,
            coupon_frequency: Frequency::SemiAnnual,
            start_date: Date::from_ymd(2024, 5, 15).unwrap(),
            maturity: Date::from_ymd(2034, 5, 15).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
            bond_type: BondType::Government,
            rating: Some("AA+".to_string()),
        };
        assert!(bond.validate().is_ok());
        assert!(bond.is_government());
        assert!(!bond.is_corporate());

        let corp = Bond {
            issuer: "Apple Inc".to_string(),
            coupon_rate: 0.0395,
            coupon_frequency: Frequency::SemiAnnual,
            start_date: Date::from_ymd(2024, 2, 8).unwrap(),
            maturity: Date::from_ymd(2029, 2, 8).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
            bond_type: BondType::Corporate,
            rating: Some("AA+".to_string()),
        };
        assert!(corp.validate().is_ok());
        assert!(corp.is_corporate());

        let mut bad = bond.clone();
        bad.issuer = "".to_string();
        assert!(bad.validate().is_err());

        let mut bad = bond.clone();
        bad.notional = -100.0;
        assert!(bad.validate().is_err());

        let mut bad = bond.clone();
        bad.maturity = Date::from_ymd(2020, 1, 1).unwrap();
        assert!(bad.validate().is_err());

        assert_eq!(BondType::Government, BondType::Government);
        assert_ne!(BondType::Government, BondType::Corporate);
    }
}

//! Interest rate volatility instrument definitions.

use super::{
    common::{NotionalSchedule, PayerReceiver, PaymentSchedule},
    error::InstrumentError,
};
use crate::{
    market::{Currency, RateIndex},
    time::{Date, EndOfMonthRule, Frequency, Tenor},
    trade::{ExerciseType, SettlementType},
};

/// Errors specific to IR Vol instrument operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IrVolInstrumentError {
    /// Invalid expiry date.
    #[error("Invalid expiry: {0} (must be future date)")]
    InvalidExpiry(Date),

    /// Invalid strike rate.
    #[error("Invalid strike: {0} (must be between -10% and 50%)")]
    InvalidStrike(f64),

    /// Invalid tenor.
    #[error("Invalid tenor: {0}")]
    InvalidTenor(String),

    /// Invalid volatility value.
    #[error("Invalid volatility: {0} (must be positive)")]
    InvalidVolatility(f64),
}

/// Swaption (option on an interest rate swap).
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
        if self.strike > 0.5 {
            return Err(InstrumentError::invalid_parameter(
                "Strike rate exceeds reasonable bounds (>50%)",
            ));
        }
        if self.underlying_swap_tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Underlying swap tenor must be at least 1 month",
            ));
        }
        Ok(())
    }

    /// Generates the underlying swap schedule.
    pub fn generate_underlying_schedule(
        &self,
        payment_frequency: Frequency,
        payment_lag: u32,
    ) -> Result<PaymentSchedule, InstrumentError> {
        self.validate()?;

        let swap_start = self.expiry;

        let swap_end = self
            .underlying_swap_tenor
            .add_to_date(swap_start, EndOfMonthRule::Adjust);

        if swap_end <= swap_start {
            return Err(InstrumentError::invalid_date(
                "Underlying swap end date must be after start date",
            ));
        }

        Ok(PaymentSchedule::generate(
            swap_start,
            swap_end,
            payment_frequency,
            payment_lag,
        ))
    }

    /// Returns the start date of the underlying swap.
    #[must_use]
    pub fn underlying_swap_start(&self) -> Date { self.expiry }

    /// Returns the end date of the underlying swap.
    #[must_use]
    pub fn underlying_swap_end(&self) -> Date {
        self.underlying_swap_tenor
            .add_to_date(self.expiry, EndOfMonthRule::Adjust)
    }

    /// Returns the expiry time in years from a given valuation date.
    #[must_use]
    pub fn expiry_years(&self, valuation_date: Date) -> f64 {
        (self.expiry - valuation_date) as f64 / 365.0
    }

    /// Returns true if this is a payer swaption.
    #[must_use]
    pub fn is_payer(&self) -> bool { self.payer_receiver == PayerReceiver::Payer }

    /// Returns the expiry tenor code (e.g., "1Y" for 1-year expiry).
    #[must_use]
    pub fn expiry_tenor_code(&self, valuation_date: Date) -> String {
        let years = self.expiry_years(valuation_date);
        if years < 1.0 {
            let months = (years * 12.0).round() as u32;
            format!("{}M", months)
        } else {
            format!("{}Y", years.round() as u32)
        }
    }
}

impl std::fmt::Display for Swaption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let direction = match self.payer_receiver {
            PayerReceiver::Payer => "Payer",
            PayerReceiver::Receiver => "Receiver",
        };
        write!(
            f,
            "{} Swaption {} into {} @ {:.2}%",
            direction,
            self.expiry,
            self.underlying_swap_tenor,
            self.strike * 100.0
        )
    }
}

/// Cap or Floor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum CapFloorType {
    /// Cap (call option on interest rates).
    Cap,
    /// Floor (put option on interest rates).
    Floor,
    /// Collar (combination of cap and floor).
    Collar,
}

impl CapFloorType {
    /// Returns the display name for this cap/floor type.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cap => "Cap",
            Self::Floor => "Floor",
            Self::Collar => "Collar",
        }
    }
}

impl std::fmt::Display for CapFloorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Interest rate cap or floor.
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
            if *strike > 0.5 {
                return Err(InstrumentError::invalid_parameter(
                    "Strike rate exceeds reasonable bounds (>50%)",
                ));
            }
        }

        if self.tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Cap/Floor tenor must be at least 1 month",
            ));
        }

        Ok(())
    }

    /// Generates the underlying cap/floor payment schedule.
    pub fn generate_underlying_schedule(
        &self,
        payment_lag: u32,
    ) -> Result<PaymentSchedule, InstrumentError> {
        self.validate()?;

        let end_date = self
            .tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust);

        if end_date <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Cap/Floor end date must be after start date",
            ));
        }

        Ok(PaymentSchedule::generate(
            self.start_date,
            end_date,
            self.payment_frequency,
            payment_lag,
        ))
    }

    /// Returns the end date of the cap/floor.
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust)
    }

    /// Returns the number of caplets/floorlets based on frequency and tenor.
    #[must_use]
    pub fn num_caplets(&self) -> u32 {
        let tenor_months = self.tenor.to_months();
        let freq_months = self.payment_frequency.months_per_period();
        tenor_months.checked_div(freq_months).unwrap_or(1)
    }

    /// Returns the primary strike (cap strike for cap, floor strike for floor).
    #[must_use]
    pub fn primary_strike(&self) -> f64 {
        match self.cap_floor_type {
            CapFloorType::Cap | CapFloorType::Floor => self.strikes.first().copied().unwrap_or(0.0),
            CapFloorType::Collar => self.strikes.get(1).copied().unwrap_or(0.0),
        }
    }

    /// Returns the floor strike for a collar (None for cap/floor).
    #[must_use]
    pub fn floor_strike(&self) -> Option<f64> {
        match self.cap_floor_type {
            CapFloorType::Collar => self.strikes.first().copied(),
            _ => None,
        }
    }

    /// Returns the cap strike for a collar (None for floor).
    #[must_use]
    pub fn cap_strike(&self) -> Option<f64> {
        match self.cap_floor_type {
            CapFloorType::Cap => self.strikes.first().copied(),
            CapFloorType::Collar => self.strikes.get(1).copied(),
            CapFloorType::Floor => None,
        }
    }
}

impl std::fmt::Display for CapFloor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cap_floor_type {
            CapFloorType::Cap => write!(
                f,
                "{} {} Cap @ {:.2}%",
                self.tenor,
                self.index,
                self.strikes[0] * 100.0
            ),
            CapFloorType::Floor => write!(
                f,
                "{} {} Floor @ {:.2}%",
                self.tenor,
                self.index,
                self.strikes[0] * 100.0
            ),
            CapFloorType::Collar => write!(
                f,
                "{} {} Collar {:.2}%-{:.2}%",
                self.tenor,
                self.index,
                self.strikes[0] * 100.0,
                self.strikes[1] * 100.0
            ),
        }
    }
}

/// IR Volatility Instrument variants.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IrVolInstrument {
    /// Swaption instrument.
    Swaption(Swaption),
    /// Cap/Floor instrument.
    CapFloor(CapFloor),
}

impl IrVolInstrument {
    /// Returns the currency for this instrument.
    #[must_use]
    pub fn currency(&self) -> Currency {
        match self {
            Self::Swaption(s) => s.currency,
            Self::CapFloor(c) => c.currency,
        }
    }

    /// Returns the expiry date for this instrument.
    #[must_use]
    pub fn expiry(&self) -> Date {
        match self {
            Self::Swaption(s) => s.expiry,
            Self::CapFloor(c) => c.start_date,
        }
    }

    /// Returns the primary strike for this instrument.
    #[must_use]
    pub fn strike(&self) -> f64 {
        match self {
            Self::Swaption(s) => s.strike,
            Self::CapFloor(c) => c.primary_strike(),
        }
    }

    /// Validates the instrument.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        match self {
            Self::Swaption(s) => s.validate(),
            Self::CapFloor(c) => c.validate(),
        }
    }
}

impl std::fmt::Display for IrVolInstrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Swaption(s) => write!(f, "{}", s),
            Self::CapFloor(c) => write!(f, "{}", c),
        }
    }
}

/// Builder for constructing Swaption instances with fluent API.
#[derive(Debug, Clone)]
pub struct SwaptionBuilder {
    expiry: Date,
    underlying_swap_tenor: Tenor,
    exercise_type: ExerciseType,
    settlement_type: SettlementType,
    strike: Option<f64>,
    notional: Option<f64>,
    currency: Currency,
    payer_receiver: PayerReceiver,
}

impl SwaptionBuilder {
    /// Creates a new builder with required expiry and underlying tenor.
    #[must_use]
    pub fn new(expiry: Date, underlying_swap_tenor: Tenor) -> Self {
        Self {
            expiry,
            underlying_swap_tenor,
            exercise_type: ExerciseType::European,
            settlement_type: SettlementType::Cash,
            strike: None,
            notional: None,
            currency: Currency::USD,
            payer_receiver: PayerReceiver::Payer,
        }
    }

    /// Sets the exercise type.
    #[must_use]
    pub fn exercise_type(mut self, exercise_type: ExerciseType) -> Self {
        self.exercise_type = exercise_type;
        self
    }

    /// Sets the settlement type.
    #[must_use]
    pub fn settlement_type(mut self, settlement_type: SettlementType) -> Self {
        self.settlement_type = settlement_type;
        self
    }

    /// Sets the strike rate.
    #[must_use]
    pub fn strike(mut self, strike: f64) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Sets the notional amount.
    #[must_use]
    pub fn notional(mut self, notional: f64) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Sets the currency.
    #[must_use]
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Sets this as a payer swaption.
    #[must_use]
    pub fn payer(mut self) -> Self {
        self.payer_receiver = PayerReceiver::Payer;
        self
    }

    /// Sets this as a receiver swaption.
    #[must_use]
    pub fn receiver(mut self) -> Self {
        self.payer_receiver = PayerReceiver::Receiver;
        self
    }

    /// Builds the Swaption, validating all parameters.
    pub fn build(self) -> Result<Swaption, InstrumentError> {
        let strike = self
            .strike
            .ok_or_else(|| InstrumentError::invalid_parameter("Strike must be specified"))?;
        let notional = self
            .notional
            .ok_or_else(|| InstrumentError::invalid_parameter("Notional must be specified"))?;

        let swaption = Swaption {
            underlying_swap_tenor: self.underlying_swap_tenor,
            expiry: self.expiry,
            exercise_type: self.exercise_type,
            settlement_type: self.settlement_type,
            strike,
            notional,
            currency: self.currency,
            payer_receiver: self.payer_receiver,
        };

        swaption.validate()?;
        Ok(swaption)
    }
}

/// Builder for constructing CapFloor instances with fluent API.
#[derive(Debug, Clone)]
pub struct CapFloorBuilder {
    start_date: Date,
    tenor: Tenor,
    cap_floor_type: Option<CapFloorType>,
    strikes: Vec<f64>,
    index: RateIndex,
    notional: Option<f64>,
    payment_frequency: Frequency,
    currency: Currency,
}

impl CapFloorBuilder {
    /// Creates a new builder with required start date and tenor.
    #[must_use]
    pub fn new(start_date: Date, tenor: Tenor) -> Self {
        Self {
            start_date,
            tenor,
            cap_floor_type: None,
            strikes: Vec::new(),
            index: RateIndex::Sofr,
            notional: None,
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        }
    }

    /// Configures this as a cap with the given strike.
    #[must_use]
    pub fn cap(mut self, strike: f64) -> Self {
        self.cap_floor_type = Some(CapFloorType::Cap);
        self.strikes = vec![strike];
        self
    }

    /// Configures this as a floor with the given strike.
    #[must_use]
    pub fn floor(mut self, strike: f64) -> Self {
        self.cap_floor_type = Some(CapFloorType::Floor);
        self.strikes = vec![strike];
        self
    }

    /// Configures this as a collar with floor and cap strikes.
    #[must_use]
    pub fn collar(mut self, floor_strike: f64, cap_strike: f64) -> Self {
        self.cap_floor_type = Some(CapFloorType::Collar);
        self.strikes = vec![floor_strike, cap_strike];
        self
    }

    /// Sets the rate index.
    #[must_use]
    pub fn index(mut self, index: RateIndex) -> Self {
        self.index = index;
        self
    }

    /// Sets the notional amount.
    #[must_use]
    pub fn notional(mut self, notional: f64) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Sets the payment frequency.
    #[must_use]
    pub fn frequency(mut self, frequency: Frequency) -> Self {
        self.payment_frequency = frequency;
        self
    }

    /// Sets the currency.
    #[must_use]
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Builds the CapFloor, validating all parameters.
    pub fn build(self) -> Result<CapFloor, InstrumentError> {
        let cap_floor_type = self.cap_floor_type.ok_or_else(|| {
            InstrumentError::invalid_parameter(
                "Cap/floor type must be specified. Call cap(), floor(), or collar() before build()",
            )
        })?;
        let notional = self
            .notional
            .ok_or_else(|| InstrumentError::invalid_parameter("Notional must be specified"))?;

        let cap_floor = CapFloor {
            cap_floor_type,
            strikes: self.strikes,
            index: self.index,
            start_date: self.start_date,
            tenor: self.tenor,
            notional_schedule: NotionalSchedule::constant(notional),
            payment_frequency: self.payment_frequency,
            currency: self.currency,
        };

        cap_floor.validate()?;
        Ok(cap_floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swaption() -> Swaption {
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
    fn cap() -> CapFloor {
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
    fn test_swaption_validate_and_features() {
        let s = swaption();
        assert!(s.validate().is_ok());
        assert_eq!(s, s.clone());
        assert!(s.is_payer());

        assert!(Swaption {
            notional: -100.0,
            ..s.clone()
        }
        .validate()
        .is_err());
        assert!(Swaption {
            strike: -0.01,
            ..s.clone()
        }
        .validate()
        .is_err());
        assert!(Swaption {
            strike: 0.6,
            ..s.clone()
        }
        .validate()
        .is_err());

        let schedule = s
            .generate_underlying_schedule(Frequency::Annual, 2)
            .unwrap();
        assert_eq!(schedule.num_periods(), 10);
        assert_eq!(schedule.start_date(), Some(s.expiry));
        assert_eq!(
            s.generate_underlying_schedule(Frequency::SemiAnnual, 0)
                .unwrap()
                .num_periods(),
            20
        );

        assert_eq!(s.underlying_swap_start(), s.expiry);
        assert_eq!(
            s.underlying_swap_end(),
            Date::from_ymd(2036, 1, 15).unwrap()
        );
        assert!((s.expiry_years(Date::from_ymd(2025, 1, 15).unwrap()) - 1.0).abs() < 0.01);

        let d = format!("{}", s);
        assert!(d.contains("Payer") && d.contains("Swaption") && d.contains("3.00%"));
    }

    #[test]
    fn test_capfloor_validate_and_features() {
        let c = cap();
        assert!(c.validate().is_ok());
        assert_eq!(CapFloorType::Cap, CapFloorType::Cap);
        assert_ne!(CapFloorType::Cap, CapFloorType::Floor);

        assert!(CapFloor {
            strikes: vec![],
            ..c.clone()
        }
        .validate()
        .is_err());
        assert!(CapFloor {
            strikes: vec![0.02, 0.04],
            ..c.clone()
        }
        .validate()
        .is_err());
        assert!(CapFloor {
            strikes: vec![0.6],
            ..c.clone()
        }
        .validate()
        .is_err());
        assert!(CapFloor {
            tenor: Tenor::Overnight,
            ..c.clone()
        }
        .validate()
        .is_err());

        assert_eq!(c.generate_underlying_schedule(2).unwrap().num_periods(), 20);
        assert_eq!(c.end_date(), Date::from_ymd(2030, 1, 1).unwrap());
        assert_eq!(c.num_caplets(), 20);
        assert!((c.primary_strike() - 0.03).abs() < 1e-10);

        let collar = CapFloor {
            cap_floor_type: CapFloorType::Collar,
            strikes: vec![0.02, 0.04],
            ..c.clone()
        };
        assert!(collar.validate().is_ok());
        assert_eq!(collar.floor_strike(), Some(0.02));
        assert_eq!(collar.cap_strike(), Some(0.04));
        let bad_collar = CapFloor {
            cap_floor_type: CapFloorType::Collar,
            strikes: vec![0.04, 0.02],
            ..c.clone()
        };
        assert!(bad_collar.validate().is_err());

        assert!(format!("{}", c).contains("Cap") && format!("{}", c).contains("3.00%"));
    }

    #[test]
    fn test_ir_vol_instrument() {
        let s = swaption();
        let inst_s = IrVolInstrument::Swaption(s.clone());
        assert_eq!(inst_s.currency(), Currency::USD);
        assert_eq!(inst_s.expiry(), s.expiry);
        assert!((inst_s.strike() - 0.03).abs() < 1e-10);
        assert!(inst_s.validate().is_ok());

        let c = cap();
        let inst_c = IrVolInstrument::CapFloor(c.clone());
        assert_eq!(inst_c.currency(), Currency::USD);
        assert_eq!(inst_c.expiry(), c.start_date);
        assert!(inst_c.validate().is_ok());
    }

    #[test]
    fn test_swaption_builder() {
        let s = SwaptionBuilder::new(Date::from_ymd(2026, 1, 15).unwrap(), Tenor::TenYears)
            .strike(0.03)
            .notional(10_000_000.0)
            .currency(Currency::USD)
            .payer()
            .build()
            .unwrap();
        assert_eq!(s.strike, 0.03);
        assert!(s.is_payer());

        let r = SwaptionBuilder::new(Date::from_ymd(2026, 1, 15).unwrap(), Tenor::TenYears)
            .strike(0.03)
            .notional(10_000_000.0)
            .receiver()
            .build()
            .unwrap();
        assert!(!r.is_payer());

        assert!(
            SwaptionBuilder::new(Date::from_ymd(2026, 1, 15).unwrap(), Tenor::TenYears)
                .notional(10_000_000.0)
                .build()
                .is_err()
        );
        assert!(
            SwaptionBuilder::new(Date::from_ymd(2026, 1, 15).unwrap(), Tenor::TenYears)
                .strike(0.03)
                .build()
                .is_err()
        );
    }

    #[test]
    fn test_capfloor_builder() {
        let d = Date::from_ymd(2025, 1, 1).unwrap();
        let c = CapFloorBuilder::new(d, Tenor::FiveYears)
            .cap(0.03)
            .index(RateIndex::Sofr)
            .notional(10_000_000.0)
            .build()
            .unwrap();
        assert_eq!(c.cap_floor_type, CapFloorType::Cap);

        let f = CapFloorBuilder::new(d, Tenor::FiveYears)
            .floor(0.01)
            .index(RateIndex::Sofr)
            .notional(10_000_000.0)
            .build()
            .unwrap();
        assert_eq!(f.cap_floor_type, CapFloorType::Floor);

        let co = CapFloorBuilder::new(d, Tenor::FiveYears)
            .collar(0.01, 0.05)
            .index(RateIndex::Sofr)
            .notional(10_000_000.0)
            .build()
            .unwrap();
        assert_eq!(co.strikes, vec![0.01, 0.05]);

        let freq = CapFloorBuilder::new(d, Tenor::FiveYears)
            .cap(0.03)
            .notional(10_000_000.0)
            .frequency(Frequency::Monthly)
            .build()
            .unwrap();
        assert_eq!(freq.payment_frequency, Frequency::Monthly);

        assert!(CapFloorBuilder::new(d, Tenor::FiveYears)
            .notional(10_000_000.0)
            .build()
            .is_err());
        assert!(CapFloorBuilder::new(d, Tenor::FiveYears)
            .cap(0.03)
            .build()
            .is_err());
    }
}

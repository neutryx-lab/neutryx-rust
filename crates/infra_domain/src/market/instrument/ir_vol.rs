//! Interest rate volatility instrument definitions.
//!
//! This module provides definitions for IR volatility instruments including
//! swaptions and caps/floors. These instruments are used for calibrating
//! IR volatility surfaces (swaption cubes, cap/floor volatility surfaces).

use super::{
    common::{NotionalSchedule, PayerReceiver, PaymentSchedule},
    error::InstrumentError,
};
use crate::{
    time::EndOfMonthRule,
    trade::{ExerciseType, SettlementType},
    Currency, Date, Frequency, RateIndex, Tenor,
};

// ============================================================================
// Error Types
// ============================================================================

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

// ============================================================================
// Swaption
// ============================================================================

/// Swaption (option on an interest rate swap).
///
/// Represents the right (but not obligation) to enter into an underlying
/// interest rate swap at a future date.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::instrument::{Swaption, PayerReceiver};
/// use infra_domain::trade::{ExerciseType, SettlementType};
/// use infra_domain::{Currency, Date, Tenor};
///
/// let swaption = Swaption {
///     underlying_swap_tenor: Tenor::TenYears,
///     expiry: Date::from_ymd(2026, 1, 15).unwrap(),
///     exercise_type: ExerciseType::European,
///     settlement_type: SettlementType::Cash,
///     strike: 0.03,
///     notional: 10_000_000.0,
///     currency: Currency::USD,
///     payer_receiver: PayerReceiver::Payer,
/// };
/// ```
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
        // Validate strike is reasonable (not more than 50%)
        if self.strike > 0.5 {
            return Err(InstrumentError::invalid_parameter(
                "Strike rate exceeds reasonable bounds (>50%)",
            ));
        }
        // Validate underlying tenor is reasonable for swaption
        if self.underlying_swap_tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Underlying swap tenor must be at least 1 month",
            ));
        }
        Ok(())
    }

    /// Generates the underlying swap schedule.
    ///
    /// The schedule represents the payment dates of the underlying swap
    /// that would begin at the swaption expiry date.
    ///
    /// # Arguments
    /// * `payment_frequency` - Payment frequency for the swap legs (default:
    ///   Annual)
    /// * `payment_lag` - Business days between accrual end and payment
    ///
    /// # Returns
    /// A `PaymentSchedule` containing the accrual periods of the underlying
    /// swap.
    ///
    /// # Errors
    /// Returns `InstrumentError` if the schedule cannot be generated.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use infra_domain::trade::instrument_def::Swaption;
    /// use infra_domain::time::Frequency;
    ///
    /// let swaption = /* ... */;
    /// let schedule = swaption.generate_underlying_schedule(Frequency::Annual, 2)?;
    /// ```
    pub fn generate_underlying_schedule(
        &self,
        payment_frequency: Frequency,
        payment_lag: u32,
    ) -> Result<PaymentSchedule, InstrumentError> {
        // Validate before generating schedule
        self.validate()?;

        // The underlying swap starts at swaption expiry
        let swap_start = self.expiry;

        // Calculate swap end date from tenor
        let swap_end = self
            .underlying_swap_tenor
            .add_to_date(swap_start, EndOfMonthRule::Adjust);

        // Validate the resulting schedule
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
    ///
    /// This is useful for swaption cube lookups.
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

// ============================================================================
// Cap/Floor Types
// ============================================================================

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

// ============================================================================
// CapFloor
// ============================================================================

/// Interest rate cap or floor.
///
/// A cap is a series of call options (caplets) on an interest rate index.
/// A floor is a series of put options (floorlets) on an interest rate index.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::instrument::{CapFloor, CapFloorType, NotionalSchedule};
/// use infra_domain::{Currency, Date, Frequency, RateIndex, Tenor};
///
/// let cap = CapFloor {
///     cap_floor_type: CapFloorType::Cap,
///     strikes: vec![0.03],
///     index: RateIndex::Sofr,
///     start_date: Date::from_ymd(2025, 1, 1).unwrap(),
///     tenor: Tenor::FiveYears,
///     notional_schedule: NotionalSchedule::constant(10_000_000.0),
///     payment_frequency: Frequency::Quarterly,
///     currency: Currency::USD,
/// };
/// ```
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
            // Validate strike is reasonable (not more than 50%)
            if *strike > 0.5 {
                return Err(InstrumentError::invalid_parameter(
                    "Strike rate exceeds reasonable bounds (>50%)",
                ));
            }
        }

        // Validate tenor is reasonable
        if self.tenor.to_months() == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Cap/Floor tenor must be at least 1 month",
            ));
        }

        Ok(())
    }

    /// Generates the underlying cap/floor payment schedule.
    ///
    /// The schedule represents the caplet/floorlet periods from start date
    /// to the end of the tenor.
    ///
    /// # Arguments
    /// * `payment_lag` - Business days between accrual end and payment
    ///
    /// # Returns
    /// A `PaymentSchedule` containing the accrual periods (caplets/floorlets).
    ///
    /// # Errors
    /// Returns `InstrumentError` if the schedule cannot be generated.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use infra_domain::trade::instrument_def::CapFloor;
    ///
    /// let cap = /* ... */;
    /// let schedule = cap.generate_underlying_schedule(2)?;
    /// ```
    pub fn generate_underlying_schedule(
        &self,
        payment_lag: u32,
    ) -> Result<PaymentSchedule, InstrumentError> {
        // Validate before generating schedule
        self.validate()?;

        // Calculate end date from tenor
        let end_date = self
            .tenor
            .add_to_date(self.start_date, EndOfMonthRule::Adjust);

        // Validate the resulting schedule
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
            CapFloorType::Collar => self.strikes.get(1).copied().unwrap_or(0.0), // Cap strike
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

// ============================================================================
// IR Vol Instrument Enum
// ============================================================================

/// IR Volatility Instrument variants.
///
/// These instruments are used for calibrating IR volatility surfaces.
/// The standard market convention quotes swaption volatilities in a cube
/// (expiry x underlying tenor x strike) and cap/floor volatilities in a surface.
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

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for constructing Swaption instances with fluent API.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::instrument::{SwaptionBuilder, PayerReceiver};
/// use infra_domain::trade::{ExerciseType, SettlementType};
/// use infra_domain::{Currency, Date, Tenor};
///
/// let swaption = SwaptionBuilder::new(
///         Date::from_ymd(2026, 1, 15).unwrap(),
///         Tenor::TenYears,
///     )
///     .strike(0.03)
///     .notional(10_000_000.0)
///     .currency(Currency::USD)
///     .payer()
///     .build()
///     .unwrap();
/// ```
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
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if:
    /// - Strike is not specified
    /// - Notional is not specified
    /// - Validation fails
    pub fn build(self) -> Result<Swaption, InstrumentError> {
        let strike = self.strike.ok_or_else(|| {
            InstrumentError::invalid_parameter("Strike must be specified")
        })?;
        let notional = self.notional.ok_or_else(|| {
            InstrumentError::invalid_parameter("Notional must be specified")
        })?;

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
///
/// # Example
///
/// ```rust
/// use infra_domain::market::instrument::{CapFloorBuilder, CapFloorType};
/// use infra_domain::{Currency, Date, Frequency, RateIndex, Tenor};
///
/// let cap = CapFloorBuilder::new(
///         Date::from_ymd(2025, 1, 1).unwrap(),
///         Tenor::FiveYears,
///     )
///     .cap(0.03)
///     .index(RateIndex::Sofr)
///     .notional(10_000_000.0)
///     .build()
///     .unwrap();
/// ```
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
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if:
    /// - Cap/floor type is not specified
    /// - Notional is not specified
    /// - Validation fails
    pub fn build(self) -> Result<CapFloor, InstrumentError> {
        let cap_floor_type = self.cap_floor_type.ok_or_else(|| {
            InstrumentError::invalid_parameter(
                "Cap/floor type must be specified. Call cap(), floor(), or collar() before build()",
            )
        })?;
        let notional = self.notional.ok_or_else(|| {
            InstrumentError::invalid_parameter("Notional must be specified")
        })?;

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Test Helpers ===

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

    // === Swaption Tests ===

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
    fn test_swaption_validate_invalid_strike() {
        let mut swaption = make_test_swaption();
        swaption.strike = 0.6; // 60% - too high
        assert!(swaption.validate().is_err());
    }

    #[test]
    fn test_swaption_clone() {
        let swaption = make_test_swaption();
        let cloned = swaption.clone();
        assert_eq!(swaption, cloned);
    }

    #[test]
    fn test_swaption_generate_underlying_schedule() {
        let swaption = make_test_swaption();
        let schedule = swaption
            .generate_underlying_schedule(Frequency::Annual, 2)
            .unwrap();

        // 10Y swap with annual frequency = 10 periods
        assert_eq!(schedule.num_periods(), 10);
        assert_eq!(schedule.start_date(), Some(swaption.expiry));
    }

    #[test]
    fn test_swaption_generate_underlying_schedule_semiannual() {
        let swaption = make_test_swaption();
        let schedule = swaption
            .generate_underlying_schedule(Frequency::SemiAnnual, 0)
            .unwrap();

        // 10Y swap with semi-annual frequency = 20 periods
        assert_eq!(schedule.num_periods(), 20);
    }

    #[test]
    fn test_swaption_underlying_swap_dates() {
        let swaption = make_test_swaption();
        let start = swaption.underlying_swap_start();
        let end = swaption.underlying_swap_end();

        assert_eq!(start, swaption.expiry);
        // 10Y from 2026-01-15 = 2036-01-15
        assert_eq!(end, Date::from_ymd(2036, 1, 15).unwrap());
    }

    #[test]
    fn test_swaption_expiry_years() {
        let swaption = make_test_swaption();
        let valuation = Date::from_ymd(2025, 1, 15).unwrap();
        let years = swaption.expiry_years(valuation);

        // Approximately 1 year
        assert!((years - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_swaption_is_payer() {
        let mut swaption = make_test_swaption();
        assert!(swaption.is_payer());

        swaption.payer_receiver = PayerReceiver::Receiver;
        assert!(!swaption.is_payer());
    }

    #[test]
    fn test_swaption_display() {
        let swaption = make_test_swaption();
        let display = format!("{}", swaption);
        assert!(display.contains("Payer"));
        assert!(display.contains("Swaption"));
        assert!(display.contains("3.00%"));
    }

    // === CapFloor Tests ===

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
    fn test_cap_validate_invalid_strike() {
        let mut cap = make_test_cap();
        cap.strikes = vec![0.6]; // 60% - too high
        assert!(cap.validate().is_err());
    }

    #[test]
    fn test_cap_validate_invalid_tenor() {
        let mut cap = make_test_cap();
        cap.tenor = Tenor::Overnight; // Too short for cap/floor
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
    fn test_capfloor_generate_underlying_schedule() {
        let cap = make_test_cap();
        let schedule = cap.generate_underlying_schedule(2).unwrap();

        // 5Y cap with quarterly frequency = 20 caplets
        assert_eq!(schedule.num_periods(), 20);
        assert_eq!(schedule.start_date(), Some(cap.start_date));
    }

    #[test]
    fn test_capfloor_end_date() {
        let cap = make_test_cap();
        let end = cap.end_date();

        // 5Y from 2025-01-01 = 2030-01-01
        assert_eq!(end, Date::from_ymd(2030, 1, 1).unwrap());
    }

    #[test]
    fn test_capfloor_num_caplets() {
        let cap = make_test_cap();
        let num = cap.num_caplets();

        // 5Y (60 months) / quarterly (3 months) = 20 caplets
        assert_eq!(num, 20);
    }

    #[test]
    fn test_capfloor_primary_strike() {
        let cap = make_test_cap();
        assert!((cap.primary_strike() - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_collar_strikes() {
        let collar = CapFloor {
            cap_floor_type: CapFloorType::Collar,
            strikes: vec![0.02, 0.04],
            index: RateIndex::Sofr,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            tenor: Tenor::FiveYears,
            notional_schedule: NotionalSchedule::constant(10_000_000.0),
            payment_frequency: Frequency::Quarterly,
            currency: Currency::USD,
        };

        assert_eq!(collar.floor_strike(), Some(0.02));
        assert_eq!(collar.cap_strike(), Some(0.04));
        assert!((collar.primary_strike() - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_capfloor_display() {
        let cap = make_test_cap();
        let display = format!("{}", cap);
        assert!(display.contains("Cap"));
        assert!(display.contains("3.00%"));
    }

    #[test]
    fn test_cap_floor_type_equality() {
        assert_eq!(CapFloorType::Cap, CapFloorType::Cap);
        assert_ne!(CapFloorType::Cap, CapFloorType::Floor);
    }

    // === IrVolInstrument Tests ===

    #[test]
    fn test_ir_vol_instrument_swaption() {
        let swaption = make_test_swaption();
        let inst = IrVolInstrument::Swaption(swaption.clone());

        assert_eq!(inst.currency(), Currency::USD);
        assert_eq!(inst.expiry(), swaption.expiry);
        assert!((inst.strike() - 0.03).abs() < 1e-10);
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_ir_vol_instrument_capfloor() {
        let cap = make_test_cap();
        let inst = IrVolInstrument::CapFloor(cap.clone());

        assert_eq!(inst.currency(), Currency::USD);
        assert_eq!(inst.expiry(), cap.start_date);
        assert!((inst.strike() - 0.03).abs() < 1e-10);
        assert!(inst.validate().is_ok());
    }

    // === Builder Tests ===

    #[test]
    fn test_swaption_builder() {
        let swaption = SwaptionBuilder::new(
            Date::from_ymd(2026, 1, 15).unwrap(),
            Tenor::TenYears,
        )
        .strike(0.03)
        .notional(10_000_000.0)
        .currency(Currency::USD)
        .payer()
        .build()
        .unwrap();

        assert_eq!(swaption.strike, 0.03);
        assert_eq!(swaption.notional, 10_000_000.0);
        assert!(swaption.is_payer());
    }

    #[test]
    fn test_swaption_builder_receiver() {
        let swaption = SwaptionBuilder::new(
            Date::from_ymd(2026, 1, 15).unwrap(),
            Tenor::TenYears,
        )
        .strike(0.03)
        .notional(10_000_000.0)
        .receiver()
        .build()
        .unwrap();

        assert!(!swaption.is_payer());
    }

    #[test]
    fn test_swaption_builder_missing_strike() {
        let result = SwaptionBuilder::new(
            Date::from_ymd(2026, 1, 15).unwrap(),
            Tenor::TenYears,
        )
        .notional(10_000_000.0)
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_swaption_builder_missing_notional() {
        let result = SwaptionBuilder::new(
            Date::from_ymd(2026, 1, 15).unwrap(),
            Tenor::TenYears,
        )
        .strike(0.03)
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_capfloor_builder_cap() {
        let cap = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .cap(0.03)
        .index(RateIndex::Sofr)
        .notional(10_000_000.0)
        .build()
        .unwrap();

        assert_eq!(cap.cap_floor_type, CapFloorType::Cap);
        assert_eq!(cap.strikes, vec![0.03]);
    }

    #[test]
    fn test_capfloor_builder_floor() {
        let floor = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .floor(0.01)
        .index(RateIndex::Sofr)
        .notional(10_000_000.0)
        .build()
        .unwrap();

        assert_eq!(floor.cap_floor_type, CapFloorType::Floor);
        assert_eq!(floor.strikes, vec![0.01]);
    }

    #[test]
    fn test_capfloor_builder_collar() {
        let collar = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .collar(0.01, 0.05)
        .index(RateIndex::Sofr)
        .notional(10_000_000.0)
        .build()
        .unwrap();

        assert_eq!(collar.cap_floor_type, CapFloorType::Collar);
        assert_eq!(collar.strikes, vec![0.01, 0.05]);
    }

    #[test]
    fn test_capfloor_builder_missing_type() {
        let result = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .notional(10_000_000.0)
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_capfloor_builder_missing_notional() {
        let result = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .cap(0.03)
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_capfloor_builder_with_frequency() {
        let cap = CapFloorBuilder::new(
            Date::from_ymd(2025, 1, 1).unwrap(),
            Tenor::FiveYears,
        )
        .cap(0.03)
        .notional(10_000_000.0)
        .frequency(Frequency::Monthly)
        .build()
        .unwrap();

        assert_eq!(cap.payment_frequency, Frequency::Monthly);
    }
}
//! Instrument definition for curve calibration.
//!
//! This module provides [`InstrumentDefinition`] which defines calibration
//! instruments as master data. These definitions are used by
//! [`CurveDefinition`] to specify which instruments make up a curve.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::definition::{InstrumentDefinition, InstrumentConventions};
//! use infra_domain::market::{Currency, RateType};
//!
//! // Deposit instrument
//! let depo = InstrumentDefinition::new(
//!     "USD-Depo-ON",
//!     Currency::USD,
//!     RateType::Deposit,
//!     "O/N",
//! );
//!
//! // OIS instrument with rate index
//! let ois = InstrumentDefinition::new(
//!     "USD-OIS-5Y",
//!     Currency::USD,
//!     RateType::Ois,
//!     "5Y",
//! ).with_rate_index("USD-SOFR");
//! ```

use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::market::{Currency, QuoteId, RateType};
use crate::time::{parse_fra_tenor, parse_tenor_to_years, CalendarId, DayCounter, Frequency, Tenor};

/// Instrument definition for curve calibration.
///
/// Defines a calibration instrument as master data, specifying its type,
/// currency, tenor, and optional market conventions.
///
/// # Convention-Based Definition
///
/// Instruments can be defined using a `convention` field that references
/// a market convention ID (e.g., "USD-SOFR-OIS", "EUR-DEPO"). The `RateType`
/// is then derived from the convention name.
///
/// # Examples
///
/// ```
/// use infra_domain::market::definition::InstrumentDefinition;
/// use infra_domain::market::{Currency, RateType};
///
/// // Convention-based definition (preferred)
/// let ois = InstrumentDefinition::from_convention(
///     "USD-OIS-5Y",
///     Currency::USD,
///     "USD-SOFR-OIS",
///     "5Y",
/// );
/// assert_eq!(ois.rate_type(), RateType::Ois);
///
/// // Legacy definition with explicit rate_type
/// let depo = InstrumentDefinition::new(
///     "USD-Depo-ON",
///     Currency::USD,
///     RateType::Deposit,
///     "O/N",
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InstrumentDefinition {
    /// Unique identifier (e.g., "USD-Depo-ON", "USD-OIS-5Y")
    pub id: String,

    /// Currency of the instrument
    pub currency: Currency,

    /// Convention ID (e.g., "USD-SOFR-OIS", "EUR-DEPO")
    /// If provided, `rate_type` is derived from this.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub convention: Option<String>,

    /// Instrument type - derived from convention if not explicitly set.
    /// When deserialising, if both `convention` and `rate_type` are present,
    /// `convention` takes precedence for deriving the actual rate type.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "rateType"
        )
    )]
    rate_type_override: Option<RateType>,

    /// Tenor specification (e.g., "O/N", "3M", "5Y", or FRA format "3x6")
    pub tenor: String,

    /// Related rate index ID (e.g., "USD-SOFR") - optional
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub rate_index: Option<String>,

    /// Market conventions override - optional
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub conventions: Option<InstrumentConventions>,

    /// Event date for event instruments (ISO format: YYYY-MM-DD).
    /// Only used when rate_type is Event.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub event_date: Option<String>,
}


/// Market conventions for an instrument.
///
/// If not specified, defaults are derived from the instrument type and currency.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InstrumentConventions {
    /// Day count convention
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub day_count: Option<DayCounter>,

    /// Spot lag in business days
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub spot_lag: Option<u8>,

    /// Holiday calendar
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub calendar: Option<CalendarId>,

    /// Payment frequency
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub payment_frequency: Option<Frequency>,
}

/// Error type for instrument definition parsing and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentDefError {
    /// Invalid tenor format
    InvalidTenor(String),
    /// Missing required field
    MissingField(&'static str),
    /// Invalid FRA tenor
    InvalidFraTenor(String),
    /// Missing event date for Event instrument
    MissingEventDate,
    /// Invalid event date format
    InvalidEventDate(String),
}

impl std::fmt::Display for InstrumentDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTenor(s) => write!(f, "Invalid tenor: {}", s),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidFraTenor(s) => write!(f, "Invalid FRA tenor: {}", s),
            Self::MissingEventDate => write!(f, "Event instrument requires eventDate"),
            Self::InvalidEventDate(s) => write!(f, "Invalid event date: {}", s),
        }
    }
}

impl std::error::Error for InstrumentDefError {}

impl InstrumentDefinition {
    /// Creates a new instrument definition with explicit rate type.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `currency` - Currency of the instrument
    /// * `rate_type` - Type of the instrument
    /// * `tenor` - Tenor string (e.g., "O/N", "3M", "5Y", "3x6")
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        currency: Currency,
        rate_type: RateType,
        tenor: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            convention: None,
            rate_type_override: Some(rate_type),
            tenor: tenor.into(),
            rate_index: None,
            conventions: None,
            event_date: None,
        }
    }

    /// Creates a new instrument definition from a convention ID.
    ///
    /// The `RateType` is derived from the convention name.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `currency` - Currency of the instrument
    /// * `convention` - Convention ID (e.g., "USD-SOFR-OIS", "EUR-DEPO")
    /// * `tenor` - Tenor string (e.g., "O/N", "3M", "5Y")
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::definition::InstrumentDefinition;
    /// use infra_domain::market::{Currency, RateType};
    ///
    /// let ois = InstrumentDefinition::from_convention(
    ///     "USD-OIS-5Y",
    ///     Currency::USD,
    ///     "USD-SOFR-OIS",
    ///     "5Y",
    /// );
    /// assert_eq!(ois.rate_type(), RateType::Ois);
    /// assert_eq!(ois.convention_id(), Some("USD-SOFR-OIS"));
    /// ```
    #[must_use]
    pub fn from_convention(
        id: impl Into<String>,
        currency: Currency,
        convention: impl Into<String>,
        tenor: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            convention: Some(convention.into()),
            rate_type_override: None,
            tenor: tenor.into(),
            rate_index: None,
            conventions: None,
            event_date: None,
        }
    }

    /// Creates a new event instrument definition.
    ///
    /// Event instruments represent rate jumps at specific dates (e.g., central bank meetings).
    /// The `event_date` is in ISO format (YYYY-MM-DD).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier (e.g., "USD-FOMC-2024-03")
    /// * `currency` - Currency of the instrument
    /// * `event_date` - Event date in ISO format (e.g., "2024-03-20")
    /// * `rate_index` - Rate index ID (e.g., "USD-SOFR")
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::definition::InstrumentDefinition;
    /// use infra_domain::market::{Currency, RateType};
    ///
    /// let event = InstrumentDefinition::from_event(
    ///     "USD-FOMC-2024-03",
    ///     Currency::USD,
    ///     "2024-03-20",
    ///     "USD-SOFR",
    /// );
    /// assert_eq!(event.rate_type(), RateType::Event);
    /// assert_eq!(event.event_date, Some("2024-03-20".to_string()));
    /// ```
    #[must_use]
    pub fn from_event(
        id: impl Into<String>,
        currency: Currency,
        event_date: impl Into<String>,
        rate_index: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            convention: None,
            rate_type_override: Some(RateType::Event),
            tenor: "EVENT".into(), // Placeholder tenor for events
            rate_index: Some(rate_index.into()),
            conventions: None,
            event_date: Some(event_date.into()),
        }
    }

    /// Creates an event instrument definition from an `EventInstrument`.
    ///
    /// This provides a conversion path from event data loaded via
    /// `MarketEvent` → `EventInstrument::from_historical()` to an
    /// `InstrumentDefinition` that can be registered in a `DefinitionRegistry`.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the instrument
    /// * `event` - The `EventInstrument` to convert
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::definition::InstrumentDefinition;
    /// use infra_domain::market::{EventInstrument, RateIndex, Currency, RateType};
    /// use infra_domain::market::events::EventType;
    /// use infra_domain::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     25.0,
    ///     0.85,
    ///     RateIndex::Sofr,
    /// );
    ///
    /// let def = InstrumentDefinition::from_event_instrument("USD-FOMC-2024-03", &event);
    /// assert_eq!(def.rate_type(), RateType::Event);
    /// assert_eq!(def.event_date, Some("2024-03-20".to_string()));
    /// ```
    #[must_use]
    pub fn from_event_instrument(
        id: impl Into<String>,
        event: &crate::market::EventInstrument,
    ) -> Self {
        let event_date = event.event_date();
        let date_str = format!(
            "{:04}-{:02}-{:02}",
            event_date.year(),
            event_date.month(),
            event_date.day()
        );

        Self {
            id: id.into(),
            currency: event.rate_index().currency(),
            convention: None,
            rate_type_override: Some(RateType::Event),
            tenor: "EVENT".into(),
            rate_index: Some(event.rate_index().code().to_string()),
            conventions: None,
            event_date: Some(date_str),
        }
    }

    /// Returns the rate type for this instrument.
    ///
    /// If a convention is set, derives the rate type from the convention name.
    /// Otherwise, returns the explicitly set rate type.
    ///
    /// # Panics
    ///
    /// Panics if neither convention nor rate_type is set. This should not happen
    /// for properly constructed instruments.
    #[must_use]
    pub fn rate_type(&self) -> RateType {
        // If convention is set, derive from it
        if let Some(ref conv) = self.convention {
            return Self::derive_rate_type_from_convention(conv);
        }

        // Fall back to explicit rate type
        self.rate_type_override
            .expect("InstrumentDefinition must have either convention or rate_type set")
    }

    /// Returns the convention ID if set.
    #[must_use]
    pub fn convention_id(&self) -> Option<&str> {
        self.convention.as_deref()
    }

    /// Derives the `RateType` from a convention ID string.
    ///
    /// Convention naming patterns:
    /// - `*-OIS` → OIS
    /// - `*-SWAP` → Swap
    /// - `*-DEPO` → Deposit
    /// - `*-FRA` → Fra
    /// - `*-FUTURES` → Futures
    /// - `*-BASIS` / `XCCY-*` → BasisSwap
    /// - `FX-*` / `*-FXOPTION` → FxSpot
    /// - `*-EVENT` → Event
    #[must_use]
    pub fn derive_rate_type_from_convention(convention: &str) -> RateType {
        let upper = convention.to_uppercase();

        // Check suffix patterns first (most specific)
        if upper.ends_with("-EVENT") {
            return RateType::Event;
        }
        if upper.ends_with("-OIS") {
            return RateType::Ois;
        }
        if upper.ends_with("-SWAP") {
            return RateType::Swap;
        }
        if upper.ends_with("-DEPO") {
            return RateType::Deposit;
        }
        if upper.ends_with("-FRA") {
            return RateType::Fra;
        }
        if upper.ends_with("-FUTURES") {
            return RateType::Futures;
        }
        if upper.ends_with("-SWAPTION") || upper.ends_with("-CAPFLOOR") {
            // Volatility products
            return RateType::Vol;
        }
        if upper.ends_with("-FXOPTION") || upper.starts_with("FX-") {
            return RateType::FxSpot;
        }
        if upper.starts_with("XCCY-") || upper.contains("-BASIS") {
            return RateType::BasisSwap;
        }

        // Check contains patterns (less specific)
        if upper.contains("OIS") {
            return RateType::Ois;
        }
        if upper.contains("SWAP") && !upper.contains("SWAPTION") {
            return RateType::Swap;
        }
        if upper.contains("DEPO") || upper.contains("DEPOSIT") {
            return RateType::Deposit;
        }
        if upper.contains("FRA") {
            return RateType::Fra;
        }
        if upper.contains("FUTURES") || upper.contains("FUT") {
            return RateType::Futures;
        }

        // Default to Deposit for unknown patterns
        RateType::Deposit
    }

    /// Sets the rate index for this instrument.
    #[must_use]
    pub fn with_rate_index(mut self, rate_index: impl Into<String>) -> Self {
        self.rate_index = Some(rate_index.into());
        self
    }

    /// Sets the convention for this instrument.
    #[must_use]
    pub fn with_convention(mut self, convention: impl Into<String>) -> Self {
        self.convention = Some(convention.into());
        self
    }

    /// Sets the conventions for this instrument.
    #[must_use]
    pub fn with_conventions(mut self, conventions: InstrumentConventions) -> Self {
        self.conventions = Some(conventions);
        self
    }

    /// Converts the tenor to years.
    ///
    /// For FRA instruments, returns the end tenor in years.
    ///
    /// # Errors
    ///
    /// Returns error if the tenor cannot be parsed.
    pub fn tenor_years(&self) -> Result<f64, InstrumentDefError> {
        // Check for FRA tenor first
        if let Some((_, end)) = parse_fra_tenor(&self.tenor) {
            return Ok(end);
        }

        parse_tenor_to_years(&self.tenor).map_err(|_| InstrumentDefError::InvalidTenor(self.tenor.clone()))
    }

    /// Returns the FRA start and end tenors in years, if this is a FRA instrument.
    ///
    /// Returns `None` if this is not a FRA or the tenor is not in FRA format.
    #[must_use]
    pub fn fra_tenors(&self) -> Option<(f64, f64)> {
        parse_fra_tenor(&self.tenor)
    }

    /// Checks if this instrument has a FRA tenor format.
    #[must_use]
    pub fn is_fra_tenor(&self) -> bool {
        parse_fra_tenor(&self.tenor).is_some()
    }

    /// Converts to a [`QuoteId`] for lookup in [`MarketQuoteSet`].
    ///
    /// # Errors
    ///
    /// Returns error if the tenor cannot be parsed.
    pub fn to_quote_id(&self) -> Result<QuoteId, InstrumentDefError> {
        let tenor = self.parse_tenor()?;
        Ok(QuoteId::new(self.currency, tenor, self.rate_type()))
    }

    /// Parses the tenor string to a [`Tenor`] enum.
    ///
    /// For FRA tenors, returns the end tenor.
    fn parse_tenor(&self) -> Result<Tenor, InstrumentDefError> {
        // For FRA, extract the end tenor
        if let Some((_, end_years)) = parse_fra_tenor(&self.tenor) {
            return Self::years_to_tenor(end_years)
                .ok_or_else(|| InstrumentDefError::InvalidFraTenor(self.tenor.clone()));
        }

        // Try parsing as standard Tenor
        Tenor::from_str(&self.tenor).map_err(|_| InstrumentDefError::InvalidTenor(self.tenor.clone()))
    }

    /// Converts a year fraction to the closest standard Tenor.
    fn years_to_tenor(years: f64) -> Option<Tenor> {
        let months = (years * 12.0).round() as u32;

        match months {
            0 => Some(Tenor::Overnight),
            1 => Some(Tenor::OneMonth),
            2 => Some(Tenor::TwoMonths),
            3 => Some(Tenor::ThreeMonths),
            6 => Some(Tenor::SixMonths),
            9 => Some(Tenor::NineMonths),
            12 => Some(Tenor::OneYear),
            24 => Some(Tenor::TwoYears),
            36 => Some(Tenor::ThreeYears),
            60 => Some(Tenor::FiveYears),
            84 => Some(Tenor::SevenYears),
            120 => Some(Tenor::TenYears),
            180 => Some(Tenor::FifteenYears),
            240 => Some(Tenor::TwentyYears),
            360 => Some(Tenor::ThirtyYears),
            _ => None,
        }
    }

    /// Validates the instrument definition.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentDefError> {
        if self.id.is_empty() {
            return Err(InstrumentDefError::MissingField("id"));
        }

        // Verify we have either convention or rate_type
        if self.convention.is_none() && self.rate_type_override.is_none() {
            return Err(InstrumentDefError::MissingField("convention or rateType"));
        }

        // Event-specific validation
        if self.rate_type() == RateType::Event {
            if self.event_date.is_none() {
                return Err(InstrumentDefError::MissingEventDate);
            }
            // Basic date format validation (YYYY-MM-DD)
            if let Some(ref date) = self.event_date {
                if date.len() != 10 || date.chars().filter(|c| *c == '-').count() != 2 {
                    return Err(InstrumentDefError::InvalidEventDate(date.clone()));
                }
            }
            // Event instruments don't need tenor validation
            return Ok(());
        }

        // Verify tenor can be parsed (non-Event instruments)
        let _ = self.tenor_years()?;

        // FRA-specific validation
        if self.rate_type() == RateType::Fra && !self.is_fra_tenor() {
            // FRA should have FRA tenor format, but we allow standard tenor too
        }

        Ok(())
    }

    /// Returns true if this is an event instrument.
    #[must_use]
    pub fn is_event(&self) -> bool {
        self.rate_type() == RateType::Event
    }

    /// Sets the event date for this instrument.
    #[must_use]
    pub fn with_event_date(mut self, event_date: impl Into<String>) -> Self {
        self.event_date = Some(event_date.into());
        self
    }
}

impl InstrumentConventions {
    /// Creates a new conventions builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the day count convention.
    #[must_use]
    pub fn with_day_count(mut self, day_count: DayCounter) -> Self {
        self.day_count = Some(day_count);
        self
    }

    /// Sets the spot lag.
    #[must_use]
    pub fn with_spot_lag(mut self, spot_lag: u8) -> Self {
        self.spot_lag = Some(spot_lag);
        self
    }

    /// Sets the calendar.
    #[must_use]
    pub fn with_calendar(mut self, calendar: CalendarId) -> Self {
        self.calendar = Some(calendar);
        self
    }

    /// Sets the payment frequency.
    #[must_use]
    pub fn with_payment_frequency(mut self, frequency: Frequency) -> Self {
        self.payment_frequency = Some(frequency);
        self
    }
}

/// Template for generating multiple instrument definitions.
///
/// A template defines common properties and a list of tenors,
/// generating one [`InstrumentDefinition`] per tenor.
///
/// # ID Pattern
///
/// The `id_pattern` field supports placeholders:
/// - `{currency}` - Currency code (e.g., "USD")
/// - `{tenor}` - Tenor string (e.g., "5Y")
/// - `{type}` - Short type name derived from convention (e.g., "OIS", "Depo")
///
/// # Examples
///
/// ```
/// use infra_domain::market::definition::InstrumentTemplate;
/// use infra_domain::market::Currency;
///
/// let template = InstrumentTemplate {
///     id_pattern: "{currency}-OIS-{tenor}".to_string(),
///     currency: Currency::USD,
///     convention: "USD-SOFR-OIS".to_string(),
///     rate_index: Some("USD-SOFR".to_string()),
///     tenors: vec!["1M".into(), "3M".into(), "6M".into(), "1Y".into(), "5Y".into()],
///     conventions: None,
/// };
///
/// let instruments = template.expand();
/// assert_eq!(instruments.len(), 5);
/// assert_eq!(instruments[0].id, "USD-OIS-1M");
/// assert_eq!(instruments[4].id, "USD-OIS-5Y");
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InstrumentTemplate {
    /// Pattern for generating IDs. Supports {currency}, {tenor}, {type} placeholders.
    pub id_pattern: String,

    /// Currency for all generated instruments
    pub currency: Currency,

    /// Convention ID (e.g., "USD-SOFR-OIS", "EUR-DEPO")
    pub convention: String,

    /// Rate index ID (e.g., "USD-SOFR") - optional
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub rate_index: Option<String>,

    /// List of tenors to generate instruments for
    pub tenors: Vec<String>,

    /// Market conventions override - optional, applied to all generated instruments
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub conventions: Option<InstrumentConventions>,
}

impl InstrumentTemplate {
    /// Creates a new instrument template.
    ///
    /// # Arguments
    ///
    /// * `id_pattern` - Pattern with placeholders for ID generation
    /// * `currency` - Currency for all instruments
    /// * `convention` - Convention ID
    /// * `tenors` - List of tenors
    #[must_use]
    pub fn new(
        id_pattern: impl Into<String>,
        currency: Currency,
        convention: impl Into<String>,
        tenors: Vec<String>,
    ) -> Self {
        Self {
            id_pattern: id_pattern.into(),
            currency,
            convention: convention.into(),
            rate_index: None,
            tenors,
            conventions: None,
        }
    }

    /// Sets the rate index for all generated instruments.
    #[must_use]
    pub fn with_rate_index(mut self, rate_index: impl Into<String>) -> Self {
        self.rate_index = Some(rate_index.into());
        self
    }

    /// Sets the conventions for all generated instruments.
    #[must_use]
    pub fn with_conventions(mut self, conventions: InstrumentConventions) -> Self {
        self.conventions = Some(conventions);
        self
    }

    /// Expands the template into a vector of [`InstrumentDefinition`].
    ///
    /// Generates one definition per tenor in the `tenors` list.
    #[must_use]
    pub fn expand(&self) -> Vec<InstrumentDefinition> {
        self.tenors
            .iter()
            .map(|tenor| self.expand_single(tenor))
            .collect()
    }

    /// Expands a single tenor into an [`InstrumentDefinition`].
    fn expand_single(&self, tenor: &str) -> InstrumentDefinition {
        let type_short = self.derive_type_short();
        let id = self
            .id_pattern
            .replace("{currency}", self.currency.code())
            .replace("{tenor}", tenor)
            .replace("{type}", &type_short);

        let mut def = InstrumentDefinition::from_convention(
            id,
            self.currency,
            &self.convention,
            tenor,
        );

        if let Some(ref rate_index) = self.rate_index {
            def = def.with_rate_index(rate_index);
        }

        if let Some(ref conventions) = self.conventions {
            def = def.with_conventions(conventions.clone());
        }

        def
    }

    /// Derives a short type name from the convention for ID generation.
    ///
    /// Examples:
    /// - "USD-SOFR-OIS" → "OIS"
    /// - "EUR-DEPO" → "Depo"
    /// - "USD-FRA" → "FRA"
    fn derive_type_short(&self) -> String {
        let upper = self.convention.to_uppercase();

        if upper.ends_with("-OIS") || upper.contains("OIS") {
            return "OIS".to_string();
        }
        if upper.ends_with("-SWAP") {
            return "Swap".to_string();
        }
        if upper.ends_with("-DEPO") || upper.contains("DEPO") {
            return "Depo".to_string();
        }
        if upper.ends_with("-FRA") || upper.contains("FRA") {
            return "FRA".to_string();
        }
        if upper.ends_with("-FUTURES") || upper.contains("FUT") {
            return "Futures".to_string();
        }

        // Default: extract last segment
        self.convention
            .rsplit('-')
            .next()
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
                    None => s.to_string(),
                }
            })
            .unwrap_or_default()
    }

    /// Returns the number of instruments this template will generate.
    #[must_use]
    pub fn count(&self) -> usize {
        self.tenors.len()
    }

    /// Validates the template.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentDefError> {
        if self.id_pattern.is_empty() {
            return Err(InstrumentDefError::MissingField("idPattern"));
        }
        if self.convention.is_empty() {
            return Err(InstrumentDefError::MissingField("convention"));
        }
        if self.tenors.is_empty() {
            return Err(InstrumentDefError::MissingField("tenors"));
        }

        // Validate each tenor can be parsed
        for tenor in &self.tenors {
            // FRA tenors are valid
            if parse_fra_tenor(tenor).is_some() {
                continue;
            }
            // Standard tenors
            if parse_tenor_to_years(tenor).is_err() {
                return Err(InstrumentDefError::InvalidTenor(tenor.clone()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_template_expand() {
        let template = InstrumentTemplate::new(
            "{currency}-OIS-{tenor}",
            Currency::USD,
            "USD-SOFR-OIS",
            vec!["1M".into(), "3M".into(), "6M".into(), "1Y".into(), "5Y".into()],
        )
        .with_rate_index("USD-SOFR");

        let instruments = template.expand();
        assert_eq!(instruments.len(), 5);
        assert_eq!(instruments[0].id, "USD-OIS-1M");
        assert_eq!(instruments[0].currency, Currency::USD);
        assert_eq!(instruments[0].rate_type(), RateType::Ois);
        assert_eq!(instruments[0].rate_index, Some("USD-SOFR".to_string()));
        assert_eq!(instruments[4].id, "USD-OIS-5Y");
    }

    #[test]
    fn test_instrument_template_with_type_placeholder() {
        let template = InstrumentTemplate::new(
            "{currency}-{type}-{tenor}",
            Currency::EUR,
            "EUR-DEPO",
            vec!["O/N".into(), "1W".into()],
        );

        let instruments = template.expand();
        assert_eq!(instruments[0].id, "EUR-Depo-O/N");
        assert_eq!(instruments[1].id, "EUR-Depo-1W");
    }

    #[test]
    fn test_instrument_template_validate() {
        let valid = InstrumentTemplate::new(
            "{currency}-OIS-{tenor}",
            Currency::USD,
            "USD-SOFR-OIS",
            vec!["1M".into(), "5Y".into()],
        );
        assert!(valid.validate().is_ok());

        let invalid_tenor = InstrumentTemplate::new(
            "{currency}-OIS-{tenor}",
            Currency::USD,
            "USD-SOFR-OIS",
            vec!["INVALID".into()],
        );
        assert!(invalid_tenor.validate().is_err());
    }

    #[test]
    fn test_instrument_template_fra_tenors() {
        let template = InstrumentTemplate::new(
            "{currency}-FRA-{tenor}",
            Currency::USD,
            "USD-FRA",
            vec!["1x4".into(), "3x6".into(), "6x9".into()],
        );

        assert!(template.validate().is_ok());
        let instruments = template.expand();
        assert_eq!(instruments.len(), 3);
        assert_eq!(instruments[0].id, "USD-FRA-1x4");
        assert_eq!(instruments[0].rate_type(), RateType::Fra);
    }

    #[test]
    fn test_instrument_definition_new() {
        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N");

        assert_eq!(def.id, "USD-Depo-ON");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type(), RateType::Deposit);
        assert_eq!(def.tenor, "O/N");
        assert!(def.rate_index.is_none());
        assert!(def.conventions.is_none());
    }

    #[test]
    fn test_instrument_definition_from_convention() {
        let def = InstrumentDefinition::from_convention(
            "USD-OIS-5Y",
            Currency::USD,
            "USD-SOFR-OIS",
            "5Y",
        );

        assert_eq!(def.id, "USD-OIS-5Y");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type(), RateType::Ois);
        assert_eq!(def.convention_id(), Some("USD-SOFR-OIS"));
        assert_eq!(def.tenor, "5Y");
    }

    #[test]
    fn test_derive_rate_type_from_convention() {
        // OIS conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-SOFR-OIS"),
            RateType::Ois
        );
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("EUR-ESTR-OIS"),
            RateType::Ois
        );

        // Swap conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-SOFR-SWAP"),
            RateType::Swap
        );
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("EUR-EURIBOR-SWAP"),
            RateType::Swap
        );

        // Deposit conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-DEPO"),
            RateType::Deposit
        );
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("EUR-DEPO"),
            RateType::Deposit
        );

        // FRA conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-FRA"),
            RateType::Fra
        );
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("EUR-FRA"),
            RateType::Fra
        );

        // Futures conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-FUTURES"),
            RateType::Futures
        );

        // XCCY Basis conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("XCCY-EURUSD"),
            RateType::BasisSwap
        );

        // Vol conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-SWAPTION"),
            RateType::Vol
        );

        // Event conventions
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("USD-SOFR-EVENT"),
            RateType::Event
        );
        assert_eq!(
            InstrumentDefinition::derive_rate_type_from_convention("EUR-ESTR-EVENT"),
            RateType::Event
        );
    }

    #[test]
    fn test_instrument_definition_with_rate_index() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y")
            .with_rate_index("USD-SOFR");

        assert_eq!(def.rate_index, Some("USD-SOFR".to_string()));
    }

    #[test]
    fn test_instrument_definition_with_conventions() {
        let conventions = InstrumentConventions::new()
            .with_day_count(DayCounter::Actual360)
            .with_spot_lag(2);

        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M")
            .with_conventions(conventions.clone());

        assert_eq!(def.conventions, Some(conventions));
    }

    #[test]
    fn test_tenor_years_standard() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!((def.tenor_years().unwrap() - 5.0).abs() < 1e-10);

        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M");
        assert!((def.tenor_years().unwrap() - 0.25).abs() < 1e-10);

        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N");
        assert!((def.tenor_years().unwrap() - 1.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_years_fra() {
        let def = InstrumentDefinition::new("USD-FRA-3x6", Currency::USD, RateType::Fra, "3x6");
        // FRA 3x6 ends at 6 months = 0.5 years
        assert!((def.tenor_years().unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fra_tenors() {
        let def = InstrumentDefinition::new("USD-FRA-3x6", Currency::USD, RateType::Fra, "3x6");
        let (start, end) = def.fra_tenors().unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fra_tenors_none_for_standard() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!(def.fra_tenors().is_none());
    }

    #[test]
    fn test_to_quote_id() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        let quote_id = def.to_quote_id().unwrap();

        assert_eq!(quote_id.currency, Currency::USD);
        assert_eq!(quote_id.rate_type, RateType::Ois);
        assert_eq!(quote_id.tenor, Tenor::FiveYears);
    }

    #[test]
    fn test_to_quote_id_from_convention() {
        let def = InstrumentDefinition::from_convention(
            "USD-OIS-5Y",
            Currency::USD,
            "USD-SOFR-OIS",
            "5Y",
        );
        let quote_id = def.to_quote_id().unwrap();

        assert_eq!(quote_id.currency, Currency::USD);
        assert_eq!(quote_id.rate_type, RateType::Ois);
        assert_eq!(quote_id.tenor, Tenor::FiveYears);
    }

    #[test]
    fn test_validate_success() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_validate_convention_based() {
        let def = InstrumentDefinition::from_convention(
            "USD-OIS-5Y",
            Currency::USD,
            "USD-SOFR-OIS",
            "5Y",
        );
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_id() {
        let def = InstrumentDefinition::new("", Currency::USD, RateType::Ois, "5Y");
        assert!(matches!(
            def.validate(),
            Err(InstrumentDefError::MissingField("id"))
        ));
    }

    #[test]
    fn test_validate_invalid_tenor() {
        let def = InstrumentDefinition::new("USD-OIS-INVALID", Currency::USD, RateType::Ois, "INVALID");
        assert!(matches!(
            def.validate(),
            Err(InstrumentDefError::InvalidTenor(_))
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y")
            .with_rate_index("USD-SOFR");

        let json = serde_json::to_string(&def).unwrap();
        let parsed: InstrumentDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(def.rate_type(), parsed.rate_type());
        assert_eq!(def.id, parsed.id);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json_legacy() {
        // Test backwards compatibility with rateType field
        let json = r#"{
            "id": "USD-Depo-ON",
            "currency": "USD",
            "rateType": "Deposit",
            "tenor": "O/N"
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "USD-Depo-ON");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type(), RateType::Deposit);
        assert_eq!(def.tenor, "O/N");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json_convention() {
        // Test convention-based deserialization
        let json = r#"{
            "id": "USD-OIS-5Y",
            "currency": "USD",
            "convention": "USD-SOFR-OIS",
            "tenor": "5Y",
            "rateIndex": "USD-SOFR"
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "USD-OIS-5Y");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type(), RateType::Ois);
        assert_eq!(def.convention_id(), Some("USD-SOFR-OIS"));
        assert_eq!(def.tenor, "5Y");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_with_conventions() {
        let json = r#"{
            "id": "USD-OIS-5Y",
            "currency": "USD",
            "convention": "USD-SOFR-OIS",
            "tenor": "5Y",
            "rateIndex": "USD-SOFR",
            "conventions": {
                "dayCount": "Actual360",
                "spotLag": 2
            }
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.rate_index, Some("USD-SOFR".to_string()));
        assert!(def.conventions.is_some());
        let conv = def.conventions.unwrap();
        assert_eq!(conv.day_count, Some(DayCounter::Actual360));
        assert_eq!(conv.spot_lag, Some(2));
    }

    #[test]
    fn test_event_instrument_from_event() {
        let event = InstrumentDefinition::from_event(
            "USD-FOMC-2024-03",
            Currency::USD,
            "2024-03-20",
            "USD-SOFR",
        );

        assert_eq!(event.id, "USD-FOMC-2024-03");
        assert_eq!(event.currency, Currency::USD);
        assert_eq!(event.rate_type(), RateType::Event);
        assert_eq!(event.event_date, Some("2024-03-20".to_string()));
        assert_eq!(event.rate_index, Some("USD-SOFR".to_string()));
        assert!(event.is_event());
    }

    #[test]
    fn test_event_instrument_validate_success() {
        let event = InstrumentDefinition::from_event(
            "USD-FOMC-2024-03",
            Currency::USD,
            "2024-03-20",
            "USD-SOFR",
        );
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_event_instrument_validate_missing_date() {
        let event = InstrumentDefinition::new(
            "USD-FOMC-2024-03",
            Currency::USD,
            RateType::Event,
            "EVENT",
        );
        assert!(matches!(
            event.validate(),
            Err(InstrumentDefError::MissingEventDate)
        ));
    }

    #[test]
    fn test_event_instrument_validate_invalid_date() {
        let mut event = InstrumentDefinition::from_event(
            "USD-FOMC-2024-03",
            Currency::USD,
            "invalid-date",
            "USD-SOFR",
        );
        event.event_date = Some("invalid".to_string());
        assert!(matches!(
            event.validate(),
            Err(InstrumentDefError::InvalidEventDate(_))
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_event_instrument() {
        let json = r#"{
            "id": "USD-FOMC-2024-03",
            "currency": "USD",
            "convention": "USD-SOFR-EVENT",
            "tenor": "EVENT",
            "rateIndex": "USD-SOFR",
            "eventDate": "2024-03-20"
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "USD-FOMC-2024-03");
        assert_eq!(def.rate_type(), RateType::Event);
        assert_eq!(def.event_date, Some("2024-03-20".to_string()));
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_from_event_instrument() {
        use crate::market::events::EventType;
        use crate::market::{EventInstrument, RateIndex};
        use crate::time::Date;

        let event = EventInstrument::new(
            Date::from_ymd(2024, 3, 20).unwrap(),
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        let def = InstrumentDefinition::from_event_instrument("USD-FOMC-2024-03", &event);

        assert_eq!(def.id, "USD-FOMC-2024-03");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type(), RateType::Event);
        assert_eq!(def.event_date, Some("2024-03-20".to_string()));
        assert_eq!(def.rate_index, Some("SOFR".to_string()));
        assert!(def.is_event());
        assert!(def.validate().is_ok());
    }
}

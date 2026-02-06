//! Event instrument for curve impact analysis.
//!
//! This module provides the [`EventInstrument`] type for representing market events
//! that may impact yield curves, such as central bank meetings.
//!
//! # Examples
//!
//! ```
//! use infra_domain::market::{EventInstrument, RateIndex};
//! use infra_domain::market::events::EventType;
//! use infra_domain::time::Date;
//!
//! let event = EventInstrument::new(
//!     Date::from_ymd(2024, 3, 20).unwrap(),
//!     EventType::CentralBankMeeting,
//!     25.0,  // Expected 25bp hike
//!     0.85,  // 85% confidence
//!     RateIndex::Sofr,
//! );
//!
//! assert_eq!(event.impact_on_curve(), 25.0);
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::events::{EventType, MarketEvent};
use super::RateIndex;
use crate::time::Date;

/// An event instrument representing a market event's impact on curves.
///
/// Event instruments capture expected rate moves from scheduled market events
/// like central bank meetings. They can be used in curve construction to
/// model expected jumps at specific dates.
///
/// # Examples
///
/// ```
/// use infra_domain::market::{EventInstrument, RateIndex};
/// use infra_domain::market::events::EventType;
/// use infra_domain::time::Date;
///
/// // Create an event for expected FOMC rate hike
/// let fomc = EventInstrument::new(
///     Date::from_ymd(2024, 3, 20).unwrap(),
///     EventType::CentralBankMeeting,
///     25.0,
///     0.90,
///     RateIndex::Sofr,
/// );
///
/// assert!(fomc.event_type() == EventType::CentralBankMeeting);
/// assert_eq!(fomc.impact_on_curve(), 25.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EventInstrument {
    /// Date of the event.
    event_date: Date,
    /// Type of market event.
    event_type: EventType,
    /// Expected spread/jump in basis points.
    ///
    /// Positive values indicate rate increases, negative values indicate cuts.
    expected_spread: f64,
    /// Confidence level (0.0 to 1.0).
    ///
    /// Represents the probability that the expected spread will materialise.
    confidence: f64,
    /// Rate index affected by this event.
    rate_index: RateIndex,
}

impl EventInstrument {
    /// Creates a new event instrument.
    ///
    /// # Arguments
    ///
    /// * `event_date` - Date when the event occurs
    /// * `event_type` - Type of market event
    /// * `expected_spread` - Expected rate change in basis points
    /// * `confidence` - Probability of the expected outcome (0.0 to 1.0)
    /// * `rate_index` - The rate index affected by this event
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{EventInstrument, RateIndex};
    /// use infra_domain::market::events::EventType;
    /// use infra_domain::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 6, 12).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     -25.0,  // Expected 25bp cut
    ///     0.75,
    ///     RateIndex::Estr,
    /// );
    /// ```
    #[must_use]
    pub fn new(
        event_date: Date,
        event_type: EventType,
        expected_spread: f64,
        confidence: f64,
        rate_index: RateIndex,
    ) -> Self {
        Self {
            event_date,
            event_type,
            expected_spread,
            confidence: confidence.clamp(0.0, 1.0),
            rate_index,
        }
    }

    /// Creates an event instrument from a historical market event.
    ///
    /// Converts a [`MarketEvent`] (typically a central bank meeting) into an
    /// [`EventInstrument`] that can be used for curve analysis.
    ///
    /// # Arguments
    ///
    /// * `event` - The market event to convert
    /// * `rate_index` - The rate index affected by this event
    /// * `default_confidence` - Default confidence if not derivable from event
    ///
    /// # Returns
    ///
    /// Returns `None` if:
    /// - The event date cannot be parsed
    /// - The event has no expected jump defined
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{EventInstrument, RateIndex};
    /// use infra_domain::market::events::{MarketEvent, EventType, EventImportance};
    ///
    /// let fomc = MarketEvent::new(
    ///     "FOMC-2024-03",
    ///     EventType::CentralBankMeeting,
    ///     "FOMC Meeting",
    ///     "2024-03-20",
    ///     EventImportance::Critical,
    ///     "Bloomberg",
    /// ).with_expected_jump_bps(25.0);
    ///
    /// let event_instrument = EventInstrument::from_historical(
    ///     &fomc,
    ///     RateIndex::Sofr,
    ///     0.80,
    /// );
    ///
    /// assert!(event_instrument.is_some());
    /// let ei = event_instrument.unwrap();
    /// assert_eq!(ei.expected_spread(), 25.0);
    /// ```
    #[must_use]
    pub fn from_historical(
        event: &MarketEvent,
        rate_index: RateIndex,
        default_confidence: f64,
    ) -> Option<Self> {
        // Parse the event date (expected format: YYYY-MM-DD)
        let event_date = parse_date_string(&event.date)?;

        // Get the expected jump; return None if not available
        let expected_spread = event.expected_jump_bps()?;

        Some(Self::new(
            event_date,
            event.event_type,
            expected_spread,
            default_confidence,
            rate_index,
        ))
    }

    /// Returns the event date.
    #[must_use]
    pub fn event_date(&self) -> Date {
        self.event_date
    }

    /// Returns the event type.
    #[must_use]
    pub fn event_type(&self) -> EventType {
        self.event_type
    }

    /// Returns the expected spread in basis points.
    #[must_use]
    pub fn expected_spread(&self) -> f64 {
        self.expected_spread
    }

    /// Returns the confidence level.
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Returns the rate index affected by this event.
    #[must_use]
    pub fn rate_index(&self) -> RateIndex {
        self.rate_index
    }

    /// Calculates the impact on the curve at the event date.
    ///
    /// Currently returns the expected spread directly. This is a placeholder
    /// for future enhancements that may incorporate:
    /// - Confidence-weighted impacts
    /// - Multiple scenario analysis
    /// - Historical pattern matching
    ///
    /// # Returns
    ///
    /// The expected spread in basis points.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{EventInstrument, RateIndex};
    /// use infra_domain::market::events::EventType;
    /// use infra_domain::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 9, 18).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     50.0,
    ///     0.60,
    ///     RateIndex::Sofr,
    /// );
    ///
    /// // Currently returns expected_spread directly
    /// assert_eq!(event.impact_on_curve(), 50.0);
    /// ```
    #[must_use]
    pub fn impact_on_curve(&self) -> f64 {
        // Placeholder: returns expected_spread for now
        // Future: may incorporate confidence weighting, scenarios, etc.
        self.expected_spread
    }

    /// Calculates the confidence-weighted impact on the curve.
    ///
    /// Multiplies the expected spread by the confidence level to get
    /// a probability-weighted expected impact.
    ///
    /// # Returns
    ///
    /// The confidence-weighted spread in basis points.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_domain::market::{EventInstrument, RateIndex};
    /// use infra_domain::market::events::EventType;
    /// use infra_domain::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 9, 18).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     50.0,
    ///     0.60,
    ///     RateIndex::Sofr,
    /// );
    ///
    /// // 50bp * 60% confidence = 30bp weighted impact
    /// assert!((event.weighted_impact() - 30.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn weighted_impact(&self) -> f64 {
        self.expected_spread * self.confidence
    }

    /// Returns true if this is a central bank meeting event.
    #[must_use]
    pub fn is_central_bank_meeting(&self) -> bool {
        self.event_type == EventType::CentralBankMeeting
    }

    /// Returns true if the expected spread is positive (rate hike).
    #[must_use]
    pub fn is_rate_hike(&self) -> bool {
        self.expected_spread > 0.0
    }

    /// Returns true if the expected spread is negative (rate cut).
    #[must_use]
    pub fn is_rate_cut(&self) -> bool {
        self.expected_spread < 0.0
    }
}

impl std::fmt::Display for EventInstrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let direction = if self.expected_spread > 0.0 {
            "+"
        } else {
            ""
        };
        write!(
            f,
            "{} {} {}{}bp @ {:.0}% confidence ({})",
            self.event_date,
            self.event_type.display_name(),
            direction,
            self.expected_spread,
            self.confidence * 100.0,
            self.rate_index.code()
        )
    }
}

/// Parses a date string in YYYY-MM-DD format.
fn parse_date_string(s: &str) -> Option<Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    Date::from_ymd(year, month, day).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::events::EventImportance;

    #[test]
    fn test_new_event_instrument() {
        let date = Date::from_ymd(2024, 3, 20).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        assert_eq!(event.event_date(), date);
        assert_eq!(event.event_type(), EventType::CentralBankMeeting);
        assert_eq!(event.expected_spread(), 25.0);
        assert_eq!(event.confidence(), 0.85);
        assert_eq!(event.rate_index(), RateIndex::Sofr);
    }

    #[test]
    fn test_confidence_clamping() {
        let date = Date::from_ymd(2024, 3, 20).unwrap();

        // Test confidence > 1.0 is clamped
        let event_high = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            1.5,
            RateIndex::Sofr,
        );
        assert_eq!(event_high.confidence(), 1.0);

        // Test confidence < 0.0 is clamped
        let event_low = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            -0.2,
            RateIndex::Sofr,
        );
        assert_eq!(event_low.confidence(), 0.0);
    }

    #[test]
    fn test_impact_on_curve() {
        let date = Date::from_ymd(2024, 6, 12).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            50.0,
            0.75,
            RateIndex::Estr,
        );

        // impact_on_curve returns expected_spread directly for now
        assert_eq!(event.impact_on_curve(), 50.0);
    }

    #[test]
    fn test_weighted_impact() {
        let date = Date::from_ymd(2024, 9, 18).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            50.0,
            0.60,
            RateIndex::Sofr,
        );

        // 50bp * 0.6 = 30bp
        assert!((event.weighted_impact() - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_from_historical() {
        let market_event = MarketEvent::new(
            "FOMC-2024-03",
            EventType::CentralBankMeeting,
            "FOMC Meeting",
            "2024-03-20",
            EventImportance::Critical,
            "Bloomberg",
        )
        .with_expected_jump_bps(25.0);

        let event = EventInstrument::from_historical(&market_event, RateIndex::Sofr, 0.80);
        assert!(event.is_some());

        let event = event.unwrap();
        assert_eq!(event.expected_spread(), 25.0);
        assert_eq!(event.confidence(), 0.80);
        assert_eq!(event.rate_index(), RateIndex::Sofr);
        assert_eq!(event.event_date(), Date::from_ymd(2024, 3, 20).unwrap());
    }

    #[test]
    fn test_from_historical_no_jump() {
        let market_event = MarketEvent::new(
            "FOMC-2024-03",
            EventType::CentralBankMeeting,
            "FOMC Meeting",
            "2024-03-20",
            EventImportance::Critical,
            "Bloomberg",
        );
        // No expected_jump_bps set

        let event = EventInstrument::from_historical(&market_event, RateIndex::Sofr, 0.80);
        assert!(event.is_none());
    }

    #[test]
    fn test_from_historical_invalid_date() {
        let mut market_event = MarketEvent::new(
            "FOMC-2024-03",
            EventType::CentralBankMeeting,
            "FOMC Meeting",
            "invalid-date",
            EventImportance::Critical,
            "Bloomberg",
        )
        .with_expected_jump_bps(25.0);

        market_event.date = "not-a-date".to_string();

        let event = EventInstrument::from_historical(&market_event, RateIndex::Sofr, 0.80);
        assert!(event.is_none());
    }

    #[test]
    fn test_is_rate_hike_cut() {
        let date = Date::from_ymd(2024, 3, 20).unwrap();

        // Rate hike
        let hike = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );
        assert!(hike.is_rate_hike());
        assert!(!hike.is_rate_cut());

        // Rate cut
        let cut = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            -25.0,
            0.85,
            RateIndex::Sofr,
        );
        assert!(!cut.is_rate_hike());
        assert!(cut.is_rate_cut());

        // No change
        let hold = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            0.0,
            0.95,
            RateIndex::Sofr,
        );
        assert!(!hold.is_rate_hike());
        assert!(!hold.is_rate_cut());
    }

    #[test]
    fn test_is_central_bank_meeting() {
        let date = Date::from_ymd(2024, 3, 20).unwrap();

        let cb_event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );
        assert!(cb_event.is_central_bank_meeting());

        let other_event = EventInstrument::new(
            date,
            EventType::EconomicRelease,
            5.0,
            0.50,
            RateIndex::Sofr,
        );
        assert!(!other_event.is_central_bank_meeting());
    }

    #[test]
    fn test_display() {
        let date = Date::from_ymd(2024, 3, 20).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        let display = format!("{}", event);
        assert!(display.contains("Central Bank Meeting"));
        assert!(display.contains("+25bp"));
        assert!(display.contains("85%"));
        assert!(display.contains("SOFR"));
    }

    #[test]
    fn test_display_negative() {
        let date = Date::from_ymd(2024, 6, 12).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            -50.0,
            0.70,
            RateIndex::Estr,
        );

        let display = format!("{}", event);
        assert!(display.contains("-50bp"));
        assert!(display.contains("70%"));
    }

    #[test]
    fn test_parse_date_string() {
        assert_eq!(
            parse_date_string("2024-03-20"),
            Date::from_ymd(2024, 3, 20).ok()
        );
        assert_eq!(
            parse_date_string("2025-12-01"),
            Date::from_ymd(2025, 12, 1).ok()
        );
        assert!(parse_date_string("invalid").is_none());
        assert!(parse_date_string("2024-13-01").is_none()); // Invalid month
        assert!(parse_date_string("2024-02-30").is_none()); // Invalid day
    }

    #[test]
    fn test_eur_ecb_event() {
        let date = Date::from_ymd(2024, 4, 11).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            -25.0,
            0.65,
            RateIndex::Estr,
        );

        assert_eq!(event.rate_index(), RateIndex::Estr);
        assert!(event.is_rate_cut());
        assert_eq!(event.weighted_impact(), -25.0 * 0.65);
    }

    #[test]
    fn test_gbp_boe_event() {
        let date = Date::from_ymd(2024, 5, 9).unwrap();
        let event = EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            0.0,
            0.90,
            RateIndex::Sonia,
        );

        assert_eq!(event.rate_index(), RateIndex::Sonia);
        assert!(!event.is_rate_hike());
        assert!(!event.is_rate_cut());
        assert_eq!(event.impact_on_curve(), 0.0);
    }
}

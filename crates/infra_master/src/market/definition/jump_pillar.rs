//! Jump pillar definition for yield curve discontinuities.
//!
//! This module provides the [`JumpPillar`] type for representing rate jumps
//! at specific dates, typically corresponding to central bank meeting events.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::definition::JumpPillar;
//! use infra_master::time::Date;
//!
//! let jump = JumpPillar::new(
//!     Date::from_ymd(2024, 3, 20).unwrap(),
//!     25.0,  // Expected 25bp jump
//!     0.85,  // 85% confidence
//! );
//!
//! assert_eq!(jump.expected_jump_bps(), 25.0);
//! assert!((jump.weighted_jump_bps() - 21.25).abs() < 1e-10);
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::market::EventInstrument;
use crate::time::Date;

/// A jump pillar representing a rate discontinuity at a specific date.
///
/// Jump pillars capture expected rate changes at known event dates (e.g., central
/// bank meetings). They are used in curve construction to model jumps that would
/// otherwise cause interpolation artifacts.
///
/// # Fields
///
/// - `jump_date` - The date when the jump occurs
/// - `expected_jump_bps` - Expected jump magnitude in basis points
/// - `event_reference` - Optional reference to the source EventInstrument
/// - `confidence` - Probability that the jump will occur (0.0 to 1.0)
///
/// # Examples
///
/// ```
/// use infra_master::market::definition::JumpPillar;
/// use infra_master::time::Date;
///
/// // Create a jump pillar for an expected FOMC rate hike
/// let jump = JumpPillar::new(
///     Date::from_ymd(2024, 3, 20).unwrap(),
///     25.0,
///     0.90,
/// )
/// .with_event_reference("FOMC-2024-03-20");
///
/// assert_eq!(jump.jump_date(), Date::from_ymd(2024, 3, 20).unwrap());
/// assert_eq!(jump.expected_jump_bps(), 25.0);
/// assert_eq!(jump.confidence(), 0.90);
/// assert_eq!(jump.event_reference(), Some("FOMC-2024-03-20"));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct JumpPillar {
    /// Date when the jump occurs.
    jump_date: Date,

    /// Expected jump magnitude in basis points.
    ///
    /// Positive values indicate rate increases, negative values indicate cuts.
    expected_jump_bps: f64,

    /// Optional reference to the source event (e.g., EventInstrument ID).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    event_reference: Option<String>,

    /// Confidence level that the jump will occur (0.0 to 1.0).
    confidence: f64,
}

impl JumpPillar {
    /// Creates a new jump pillar.
    ///
    /// # Arguments
    ///
    /// * `jump_date` - The date when the jump occurs
    /// * `expected_jump_bps` - Expected jump magnitude in basis points
    /// * `confidence` - Probability of the jump occurring (clamped to [0.0, 1.0])
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::time::Date;
    ///
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 6, 12).unwrap(),
    ///     -25.0,  // Expected 25bp cut
    ///     0.75,
    /// );
    /// ```
    #[must_use]
    pub fn new(jump_date: Date, expected_jump_bps: f64, confidence: f64) -> Self {
        Self {
            jump_date,
            expected_jump_bps,
            event_reference: None,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Sets the event reference ID.
    ///
    /// # Arguments
    ///
    /// * `ref_id` - Reference ID string (e.g., EventInstrument identifier)
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::time::Date;
    ///
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     25.0,
    ///     0.85,
    /// )
    /// .with_event_reference("FOMC-2024-03");
    ///
    /// assert_eq!(jump.event_reference(), Some("FOMC-2024-03"));
    /// ```
    #[must_use]
    pub fn with_event_reference(mut self, ref_id: impl Into<String>) -> Self {
        self.event_reference = Some(ref_id.into());
        self
    }

    /// Creates a jump pillar from an EventInstrument.
    ///
    /// Converts the event's expected spread and confidence into a JumpPillar.
    ///
    /// # Arguments
    ///
    /// * `event` - The EventInstrument to convert from
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::market::{EventInstrument, RateIndex};
    /// use infra_master::market::events::EventType;
    /// use infra_master::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     25.0,
    ///     0.85,
    ///     RateIndex::Sofr,
    /// );
    ///
    /// let jump = JumpPillar::from_event_instrument(&event);
    /// assert_eq!(jump.jump_date(), event.event_date());
    /// assert_eq!(jump.expected_jump_bps(), event.expected_spread());
    /// assert_eq!(jump.confidence(), event.confidence());
    /// ```
    #[must_use]
    pub fn from_event_instrument(event: &EventInstrument) -> Self {
        Self {
            jump_date: event.event_date(),
            expected_jump_bps: event.expected_spread(),
            event_reference: None,
            confidence: event.confidence(),
        }
    }

    /// Creates a jump pillar from an EventInstrument with a reference ID.
    ///
    /// # Arguments
    ///
    /// * `event` - The EventInstrument to convert from
    /// * `ref_id` - Reference ID to associate with this jump
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::market::{EventInstrument, RateIndex};
    /// use infra_master::market::events::EventType;
    /// use infra_master::time::Date;
    ///
    /// let event = EventInstrument::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     EventType::CentralBankMeeting,
    ///     25.0,
    ///     0.85,
    ///     RateIndex::Sofr,
    /// );
    ///
    /// let jump = JumpPillar::from_event_instrument_with_ref(&event, "FOMC-2024-03");
    /// assert_eq!(jump.event_reference(), Some("FOMC-2024-03"));
    /// ```
    #[must_use]
    pub fn from_event_instrument_with_ref(event: &EventInstrument, ref_id: impl Into<String>) -> Self {
        Self::from_event_instrument(event).with_event_reference(ref_id)
    }

    /// Returns the jump date.
    #[must_use]
    pub fn jump_date(&self) -> Date {
        self.jump_date
    }

    /// Returns the expected jump in basis points.
    #[must_use]
    pub fn expected_jump_bps(&self) -> f64 {
        self.expected_jump_bps
    }

    /// Returns the confidence level.
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Returns the event reference ID if set.
    #[must_use]
    pub fn event_reference(&self) -> Option<&str> {
        self.event_reference.as_deref()
    }

    /// Returns the confidence-weighted jump in basis points.
    ///
    /// This is the expected jump multiplied by the confidence level,
    /// representing the probability-weighted expected impact.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::time::Date;
    ///
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     50.0,
    ///     0.60,
    /// );
    ///
    /// // 50bp * 60% = 30bp weighted jump
    /// assert!((jump.weighted_jump_bps() - 30.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn weighted_jump_bps(&self) -> f64 {
        self.expected_jump_bps * self.confidence
    }

    /// Converts the expected jump from basis points to decimal rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::time::Date;
    ///
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     25.0,  // 25bp
    ///     1.0,
    /// );
    ///
    /// // 25bp = 0.0025
    /// assert!((jump.expected_jump_rate() - 0.0025).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn expected_jump_rate(&self) -> f64 {
        self.expected_jump_bps / 10_000.0
    }

    /// Converts the weighted jump from basis points to decimal rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use infra_master::market::definition::JumpPillar;
    /// use infra_master::time::Date;
    ///
    /// let jump = JumpPillar::new(
    ///     Date::from_ymd(2024, 3, 20).unwrap(),
    ///     50.0,  // 50bp
    ///     0.60,  // 60% confidence
    /// );
    ///
    /// // 50bp * 60% = 30bp = 0.003
    /// assert!((jump.weighted_jump_rate() - 0.003).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn weighted_jump_rate(&self) -> f64 {
        self.weighted_jump_bps() / 10_000.0
    }

    /// Returns true if this represents a rate increase.
    #[must_use]
    pub fn is_rate_hike(&self) -> bool {
        self.expected_jump_bps > 0.0
    }

    /// Returns true if this represents a rate decrease.
    #[must_use]
    pub fn is_rate_cut(&self) -> bool {
        self.expected_jump_bps < 0.0
    }

    /// Returns true if this has high confidence (>= 0.8).
    #[must_use]
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }
}

impl std::fmt::Display for JumpPillar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let direction = if self.expected_jump_bps > 0.0 {
            "+"
        } else {
            ""
        };
        write!(
            f,
            "JumpPillar({}: {}{}bp @ {:.0}%)",
            self.jump_date,
            direction,
            self.expected_jump_bps,
            self.confidence * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::events::EventType;
    use crate::market::RateIndex;

    fn test_date() -> Date {
        Date::from_ymd(2024, 3, 20).unwrap()
    }

    #[test]
    fn test_new_jump_pillar() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);

        assert_eq!(jump.jump_date(), test_date());
        assert_eq!(jump.expected_jump_bps(), 25.0);
        assert_eq!(jump.confidence(), 0.85);
        assert!(jump.event_reference().is_none());
    }

    #[test]
    fn test_with_event_reference() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85).with_event_reference("FOMC-2024-03");

        assert_eq!(jump.event_reference(), Some("FOMC-2024-03"));
    }

    #[test]
    fn test_confidence_clamping() {
        // Confidence > 1.0 clamped to 1.0
        let high = JumpPillar::new(test_date(), 25.0, 1.5);
        assert_eq!(high.confidence(), 1.0);

        // Confidence < 0.0 clamped to 0.0
        let low = JumpPillar::new(test_date(), 25.0, -0.5);
        assert_eq!(low.confidence(), 0.0);

        // Valid confidence unchanged
        let valid = JumpPillar::new(test_date(), 25.0, 0.75);
        assert_eq!(valid.confidence(), 0.75);
    }

    #[test]
    fn test_from_event_instrument() {
        let event = EventInstrument::new(
            test_date(),
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        let jump = JumpPillar::from_event_instrument(&event);

        assert_eq!(jump.jump_date(), event.event_date());
        assert_eq!(jump.expected_jump_bps(), event.expected_spread());
        assert_eq!(jump.confidence(), event.confidence());
        assert!(jump.event_reference().is_none());
    }

    #[test]
    fn test_from_event_instrument_with_ref() {
        let event = EventInstrument::new(
            test_date(),
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        let jump = JumpPillar::from_event_instrument_with_ref(&event, "FOMC-2024-03");

        assert_eq!(jump.event_reference(), Some("FOMC-2024-03"));
    }

    #[test]
    fn test_weighted_jump_bps() {
        let jump = JumpPillar::new(test_date(), 50.0, 0.60);
        // 50 * 0.6 = 30
        assert!((jump.weighted_jump_bps() - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_expected_jump_rate() {
        let jump = JumpPillar::new(test_date(), 25.0, 1.0);
        // 25bp = 0.0025
        assert!((jump.expected_jump_rate() - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn test_weighted_jump_rate() {
        let jump = JumpPillar::new(test_date(), 50.0, 0.60);
        // 50bp * 60% = 30bp = 0.003
        assert!((jump.weighted_jump_rate() - 0.003).abs() < 1e-10);
    }

    #[test]
    fn test_is_rate_hike() {
        let hike = JumpPillar::new(test_date(), 25.0, 0.85);
        assert!(hike.is_rate_hike());
        assert!(!hike.is_rate_cut());

        let cut = JumpPillar::new(test_date(), -25.0, 0.85);
        assert!(!cut.is_rate_hike());
        assert!(cut.is_rate_cut());

        let hold = JumpPillar::new(test_date(), 0.0, 0.95);
        assert!(!hold.is_rate_hike());
        assert!(!hold.is_rate_cut());
    }

    #[test]
    fn test_is_high_confidence() {
        let high = JumpPillar::new(test_date(), 25.0, 0.90);
        assert!(high.is_high_confidence());

        let medium = JumpPillar::new(test_date(), 25.0, 0.79);
        assert!(!medium.is_high_confidence());

        let boundary = JumpPillar::new(test_date(), 25.0, 0.80);
        assert!(boundary.is_high_confidence());
    }

    #[test]
    fn test_display_positive() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);
        let display = format!("{}", jump);

        assert!(display.contains("+25bp"));
        assert!(display.contains("85%"));
    }

    #[test]
    fn test_display_negative() {
        let jump = JumpPillar::new(test_date(), -50.0, 0.70);
        let display = format!("{}", jump);

        assert!(display.contains("-50bp"));
        assert!(display.contains("70%"));
    }

    #[test]
    fn test_clone_and_partial_eq() {
        let jump1 = JumpPillar::new(test_date(), 25.0, 0.85);
        let jump2 = jump1.clone();

        assert_eq!(jump1, jump2);
    }

    #[test]
    fn test_debug() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);
        let debug = format!("{:?}", jump);

        assert!(debug.contains("JumpPillar"));
        assert!(debug.contains("25"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85).with_event_reference("FOMC-2024-03");

        let json = serde_json::to_string(&jump).unwrap();
        let parsed: JumpPillar = serde_json::from_str(&json).unwrap();

        assert_eq!(jump, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_without_event_reference() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);

        let json = serde_json::to_string(&jump).unwrap();

        // event_reference should be skipped when None
        assert!(!json.contains("eventReference"));

        let parsed: JumpPillar = serde_json::from_str(&json).unwrap();
        assert_eq!(jump, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_camel_case() {
        let json = r#"{
            "jumpDate": "2024-03-20",
            "expectedJumpBps": 25.0,
            "confidence": 0.85,
            "eventReference": "FOMC-2024-03"
        }"#;

        let jump: JumpPillar = serde_json::from_str(json).unwrap();
        assert_eq!(jump.expected_jump_bps(), 25.0);
        assert_eq!(jump.confidence(), 0.85);
        assert_eq!(jump.event_reference(), Some("FOMC-2024-03"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_defaults() {
        // eventReference should default to None
        let json = r#"{
            "jumpDate": "2024-03-20",
            "expectedJumpBps": 25.0,
            "confidence": 0.85
        }"#;

        let jump: JumpPillar = serde_json::from_str(json).unwrap();
        assert!(jump.event_reference().is_none());
    }
}

//! Event instrument for curve impact analysis.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{
    events::{EventType, MarketEvent},
    RateIndex,
};
use crate::time::Date;

/// An event instrument representing a market event's impact on curves.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EventInstrument {
    /// Date of the event.
    event_date: Date,
    /// Type of market event.
    event_type: EventType,
    /// Expected spread/jump in basis points.
    expected_spread: f64,
    /// Confidence level (0.0 to 1.0).
    confidence: f64,
    /// Rate index affected by this event.
    rate_index: RateIndex,
    /// End date for turn events (when the spike reverts).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    end_date: Option<Date>,
}

impl EventInstrument {
    /// Creates a new event instrument.
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
            end_date: None,
        }
    }

    /// Creates an event instrument from a historical market event.
    #[must_use]
    pub fn from_historical(
        event: &MarketEvent,
        rate_index: RateIndex,
        default_confidence: f64,
    ) -> Option<Self> {
        let event_date = parse_date_string(&event.date)?;

        let expected_spread = event.expected_jump_bps()?;

        Some(Self::new(
            event_date,
            event.event_type,
            expected_spread,
            default_confidence,
            rate_index,
        ))
    }

    /// Creates a new turn event instrument with an end date.
    #[must_use]
    pub fn new_turn(
        event_date: Date,
        end_date: Date,
        event_type: EventType,
        expected_spread: f64,
        confidence: f64,
        rate_index: RateIndex,
    ) -> Self {
        assert!(
            end_date > event_date,
            "Turn end_date must be after event_date"
        );
        Self {
            event_date,
            event_type,
            expected_spread,
            confidence: confidence.clamp(0.0, 1.0),
            rate_index,
            end_date: Some(end_date),
        }
    }

    /// Sets the end date for turn events.
    #[must_use]
    pub fn with_end_date(mut self, end_date: Date) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Returns the event date.
    #[must_use]
    pub fn event_date(&self) -> Date { self.event_date }

    /// Returns the event type.
    #[must_use]
    pub fn event_type(&self) -> EventType { self.event_type }

    /// Returns the expected spread in basis points.
    #[must_use]
    pub fn expected_spread(&self) -> f64 { self.expected_spread }

    /// Returns the confidence level.
    #[must_use]
    pub fn confidence(&self) -> f64 { self.confidence }

    /// Returns the rate index affected by this event.
    #[must_use]
    pub fn rate_index(&self) -> RateIndex { self.rate_index }

    /// Returns the end date for turn events, or `None` for permanent jumps.
    #[must_use]
    pub fn end_date(&self) -> Option<Date> { self.end_date }

    /// Returns true if this is a turn event (temporary spike).
    #[must_use]
    pub fn is_turn(&self) -> bool { self.event_type.is_turn() }

    /// Returns true if this is a permanent jump (no end date).
    #[must_use]
    pub fn is_permanent_jump(&self) -> bool { self.end_date.is_none() }

    /// Calculates the impact on the curve at the event date.
    #[must_use]
    pub fn impact_on_curve(&self) -> f64 { self.expected_spread }

    /// Calculates the confidence-weighted impact on the curve.
    #[must_use]
    pub fn weighted_impact(&self) -> f64 { self.expected_spread * self.confidence }

    /// Returns true if this is a central bank meeting event.
    #[must_use]
    pub fn is_central_bank_meeting(&self) -> bool {
        self.event_type == EventType::CentralBankMeeting
    }

    /// Returns true if the expected spread is positive (rate hike).
    #[must_use]
    pub fn is_rate_hike(&self) -> bool { self.expected_spread > 0.0 }

    /// Returns true if the expected spread is negative (rate cut).
    #[must_use]
    pub fn is_rate_cut(&self) -> bool { self.expected_spread < 0.0 }
}

impl std::fmt::Display for EventInstrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let direction = if self.expected_spread > 0.0 { "+" } else { "" };
        write!(
            f,
            "{} {} {}{}bp @ {:.0}% confidence ({})",
            self.event_date,
            self.event_type.display_name(),
            direction,
            self.expected_spread,
            self.confidence * 100.0,
            self.rate_index.code()
        )?;
        if let Some(end_date) = self.end_date {
            write!(f, " [reverts {}]", end_date)?;
        }
        Ok(())
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

    fn cb(date: Date, spread: f64, conf: f64, idx: RateIndex) -> EventInstrument {
        EventInstrument::new(date, EventType::CentralBankMeeting, spread, conf, idx)
    }

    #[test]
    fn test_event_instrument_core() {
        let d = Date::from_ymd(2024, 3, 20).unwrap();
        let e = cb(d, 25.0, 0.85, RateIndex::Sofr);
        assert_eq!(e.event_date(), d);
        assert_eq!(e.event_type(), EventType::CentralBankMeeting);
        assert_eq!(e.expected_spread(), 25.0);
        assert_eq!(e.confidence(), 0.85);
        assert_eq!(e.rate_index(), RateIndex::Sofr);

        assert_eq!(cb(d, 25.0, 1.5, RateIndex::Sofr).confidence(), 1.0);
        assert_eq!(cb(d, 25.0, -0.2, RateIndex::Sofr).confidence(), 0.0);

        let e50 = cb(
            Date::from_ymd(2024, 6, 12).unwrap(),
            50.0,
            0.75,
            RateIndex::Estr,
        );
        assert_eq!(e50.impact_on_curve(), 50.0);
        let e60 = cb(
            Date::from_ymd(2024, 9, 18).unwrap(),
            50.0,
            0.60,
            RateIndex::Sofr,
        );
        assert!((e60.weighted_impact() - 30.0).abs() < 1e-10);

        assert!(cb(d, 25.0, 0.85, RateIndex::Sofr).is_rate_hike());
        assert!(!cb(d, 25.0, 0.85, RateIndex::Sofr).is_rate_cut());
        assert!(cb(d, -25.0, 0.85, RateIndex::Sofr).is_rate_cut());
        assert!(!cb(d, 0.0, 0.95, RateIndex::Sofr).is_rate_hike());
        assert!(!cb(d, 0.0, 0.95, RateIndex::Sofr).is_rate_cut());

        assert!(cb(d, 25.0, 0.85, RateIndex::Sofr).is_central_bank_meeting());
        assert!(
            !EventInstrument::new(d, EventType::EconomicRelease, 5.0, 0.5, RateIndex::Sofr)
                .is_central_bank_meeting()
        );

        let disp = format!("{}", cb(d, 25.0, 0.85, RateIndex::Sofr));
        assert!(
            disp.contains("Central Bank Meeting")
                && disp.contains("+25bp")
                && disp.contains("85%")
                && disp.contains("SOFR")
        );
        let disp_neg = format!(
            "{}",
            cb(
                Date::from_ymd(2024, 6, 12).unwrap(),
                -50.0,
                0.70,
                RateIndex::Estr
            )
        );
        assert!(disp_neg.contains("-50bp") && disp_neg.contains("70%"));

        let ecb = cb(
            Date::from_ymd(2024, 4, 11).unwrap(),
            -25.0,
            0.65,
            RateIndex::Estr,
        );
        assert_eq!(ecb.rate_index(), RateIndex::Estr);
        assert!(ecb.is_rate_cut());
        assert_eq!(ecb.weighted_impact(), -25.0 * 0.65);
        let boe = cb(
            Date::from_ymd(2024, 5, 9).unwrap(),
            0.0,
            0.90,
            RateIndex::Sonia,
        );
        assert_eq!(boe.impact_on_curve(), 0.0);
    }

    #[test]
    fn test_event_instrument_conversion() {
        let me = MarketEvent::new(
            "FOMC-2024-03",
            EventType::CentralBankMeeting,
            "FOMC Meeting",
            "2024-03-20",
            EventImportance::Critical,
            "Bloomberg",
        )
        .with_expected_jump_bps(25.0);
        let e = EventInstrument::from_historical(&me, RateIndex::Sofr, 0.80).unwrap();
        assert_eq!(e.expected_spread(), 25.0);
        assert_eq!(e.confidence(), 0.80);
        assert_eq!(e.event_date(), Date::from_ymd(2024, 3, 20).unwrap());

        let no_jump = MarketEvent::new(
            "FOMC",
            EventType::CentralBankMeeting,
            "FOMC",
            "2024-03-20",
            EventImportance::Critical,
            "Bloomberg",
        );
        assert!(EventInstrument::from_historical(&no_jump, RateIndex::Sofr, 0.80).is_none());

        let mut bad = me.clone();
        bad.date = "not-a-date".to_string();
        assert!(EventInstrument::from_historical(&bad, RateIndex::Sofr, 0.80).is_none());

        assert_eq!(
            parse_date_string("2024-03-20"),
            Date::from_ymd(2024, 3, 20).ok()
        );
        assert_eq!(
            parse_date_string("2025-12-01"),
            Date::from_ymd(2025, 12, 1).ok()
        );
        assert!(parse_date_string("invalid").is_none());
        assert!(parse_date_string("2024-13-01").is_none());
        assert!(parse_date_string("2024-02-30").is_none());
    }

    #[test]
    fn test_event_instrument_turns() {
        let ed = Date::from_ymd(2024, 12, 31).unwrap();
        let end = Date::from_ymd(2025, 1, 2).unwrap();
        let turn =
            EventInstrument::new_turn(ed, end, EventType::TurnOfYear, 12.5, 1.0, RateIndex::Sofr);
        assert_eq!(turn.event_date(), ed);
        assert_eq!(turn.end_date(), Some(end));
        assert_eq!(turn.event_type(), EventType::TurnOfYear);
        assert!(turn.is_turn() && !turn.is_permanent_jump());

        let disp = format!("{}", turn);
        assert!(
            disp.contains("Turn of Year") && disp.contains("+12.5bp") && disp.contains("[reverts")
        );

        let perm = cb(
            Date::from_ymd(2024, 3, 20).unwrap(),
            25.0,
            0.85,
            RateIndex::Sofr,
        );
        assert!(perm.end_date().is_none() && perm.is_permanent_jump() && !perm.is_turn());

        let with_end =
            EventInstrument::new(ed, EventType::Turn, 5.0, 1.0, RateIndex::Sofr).with_end_date(end);
        assert_eq!(with_end.end_date(), Some(end));

        assert!(
            EventInstrument::new(ed, EventType::TurnOfYear, 12.5, 1.0, RateIndex::Sofr).is_turn()
        );
        assert!(
            EventInstrument::new(ed, EventType::TurnOfQuarter, 5.0, 1.0, RateIndex::Sofr).is_turn()
        );
        assert!(
            EventInstrument::new(ed, EventType::TurnOfMonth, 2.0, 1.0, RateIndex::Sofr).is_turn()
        );
        assert!(!cb(ed, 25.0, 0.85, RateIndex::Sofr).is_turn());
    }
}

//! Curve definition for yield curve construction.

use serde::{Deserialize, Serialize};

use crate::{
    market::{EventInstrument, RateIndex},
    time::Date,
};

/// A jump pillar representing a rate discontinuity at a specific date.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JumpPillar {
    /// Date when the jump occurs.
    jump_date: Date,

    /// Expected jump magnitude in basis points.
    expected_jump_bps: f64,

    /// Optional reference to the source event (e.g., EventInstrument ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event_reference: Option<String>,

    /// Confidence level that the jump will occur (0.0 to 1.0).
    confidence: f64,

    /// End date for turn events (when the temporary spike reverts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_date: Option<Date>,
}

impl JumpPillar {
    /// Creates a new jump pillar.
    #[must_use]
    pub fn new(jump_date: Date, expected_jump_bps: f64, confidence: f64) -> Self {
        Self {
            jump_date,
            expected_jump_bps,
            event_reference: None,
            confidence: confidence.clamp(0.0, 1.0),
            end_date: None,
        }
    }

    /// Sets the event reference ID.
    #[must_use]
    pub fn with_event_reference(mut self, ref_id: impl Into<String>) -> Self {
        self.event_reference = Some(ref_id.into());
        self
    }

    /// Creates a jump pillar from an EventInstrument.
    #[must_use]
    pub fn from_event_instrument(event: &EventInstrument) -> Self {
        Self {
            jump_date: event.event_date(),
            expected_jump_bps: event.expected_spread(),
            event_reference: None,
            confidence: event.confidence(),
            end_date: event.end_date(),
        }
    }

    /// Creates a jump pillar from an EventInstrument with a reference ID.
    #[must_use]
    pub fn from_event_instrument_with_ref(
        event: &EventInstrument,
        ref_id: impl Into<String>,
    ) -> Self {
        Self::from_event_instrument(event).with_event_reference(ref_id)
    }

    /// Returns the jump date.
    #[must_use]
    pub fn jump_date(&self) -> Date { self.jump_date }

    /// Returns the expected jump in basis points.
    #[must_use]
    pub fn expected_jump_bps(&self) -> f64 { self.expected_jump_bps }

    /// Returns the confidence level.
    #[must_use]
    pub fn confidence(&self) -> f64 { self.confidence }

    /// Returns the event reference ID if set.
    #[must_use]
    pub fn event_reference(&self) -> Option<&str> { self.event_reference.as_deref() }

    /// Returns the confidence-weighted jump in basis points.
    #[must_use]
    pub fn weighted_jump_bps(&self) -> f64 { self.expected_jump_bps * self.confidence }

    /// Converts the expected jump from basis points to decimal rate.
    #[must_use]
    pub fn expected_jump_rate(&self) -> f64 { self.expected_jump_bps / 10_000.0 }

    /// Converts the weighted jump from basis points to decimal rate.
    #[must_use]
    pub fn weighted_jump_rate(&self) -> f64 { self.weighted_jump_bps() / 10_000.0 }

    /// Returns true if this represents a rate increase.
    #[must_use]
    pub fn is_rate_hike(&self) -> bool { self.expected_jump_bps > 0.0 }

    /// Returns true if this represents a rate decrease.
    #[must_use]
    pub fn is_rate_cut(&self) -> bool { self.expected_jump_bps < 0.0 }

    /// Returns true if this has high confidence (>= 0.8).
    #[must_use]
    pub fn is_high_confidence(&self) -> bool { self.confidence >= 0.8 }

    /// Returns the end date for turn events, or `None` for permanent jumps.
    #[must_use]
    pub fn end_date(&self) -> Option<Date> { self.end_date }

    /// Sets the end date for turn events.
    #[must_use]
    pub fn with_end_date(mut self, end_date: Date) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Returns true if this is a turn (temporary spike with an end date).
    #[must_use]
    pub fn is_turn(&self) -> bool { self.end_date.is_some() }
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

/// Builder for creating JumpPillars from EventInstruments.
#[derive(Debug, Clone)]
pub struct JumpPillarBuilder {
    events: Vec<EventInstrument>,
    rate_index_filter: Option<RateIndex>,
    date_range: Option<(Date, Date)>,
    min_confidence: f64,
}

impl JumpPillarBuilder {
    /// Creates a new builder with the given events.
    #[must_use]
    pub fn new(events: Vec<EventInstrument>) -> Self {
        Self {
            events,
            rate_index_filter: None,
            date_range: None,
            min_confidence: 0.0,
        }
    }

    /// Filters events to only include those matching the specified rate index.
    #[must_use]
    pub fn with_rate_index(mut self, index: RateIndex) -> Self {
        self.rate_index_filter = Some(index);
        self
    }

    /// Filters events to only include those within the specified date range.
    #[must_use]
    pub fn with_date_range(mut self, start: Date, end: Date) -> Self {
        self.date_range = Some((start, end));
        self
    }

    /// Filters events to only include those with confidence at or above the.
    #[must_use]
    pub fn with_min_confidence(mut self, threshold: f64) -> Self {
        self.min_confidence = threshold.clamp(0.0, 1.0);
        self
    }

    /// Builds the filtered and sorted list of JumpPillars.
    #[must_use]
    pub fn build(self) -> Vec<JumpPillar> {
        let mut pillars: Vec<JumpPillar> = self
            .events
            .iter()
            .filter(|event| {
                if let Some(index) = self.rate_index_filter {
                    if event.rate_index() != index {
                        return false;
                    }
                }

                if let Some((start, end)) = self.date_range {
                    let date = event.event_date();
                    if date < start || date > end {
                        return false;
                    }
                }

                if event.confidence() < self.min_confidence {
                    return false;
                }

                true
            })
            .map(JumpPillar::from_event_instrument)
            .collect();

        pillars.sort_by_key(|a| a.jump_date());

        pillars
    }
}

/// Curve definition - the recipe for building a yield curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveDefinition {
    /// Curve name (e.g., "USD-SOFR-Discount").
    pub name: String,

    /// Reference to RateIndexDefinition.id (e.g., "USD-SOFR").
    pub rate_index: String,

    /// List of InstrumentDefinition.id references.
    pub instruments: Vec<String>,

    /// Jump pillars for rate discontinuities (e.g., central bank meetings).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jump_pillars: Vec<JumpPillar>,

    /// Calibration method.
    #[serde(default)]
    pub calibration_method: CalibrationMethod,

    /// Interpolation method for the resulting curve.
    #[serde(default)]
    pub interpolation: InterpolationMethod,

    /// Whether to allow extrapolation beyond the last pillar.
    #[serde(default = "default_true")]
    pub allow_extrapolation: bool,
}

fn default_true() -> bool { true }

/// Calibration method for curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalibrationMethod {
    /// Sequential bootstrapping (pillar-by-pillar).
    #[default]
    Sequential,
    /// Global calibration (all pillars simultaneously).
    Global,
    /// Levenberg-Marquardt non-linear least squares.
    #[serde(rename = "levenberg_marquardt")]
    LevenbergMarquardt,
    /// Penalised (regularised) global calibration with forward smoothness penalty.
    Penalised,
    /// Best fit via QR least squares (for overdetermined systems).
    #[serde(rename = "best_fit")]
    BestFit,
}

/// Interpolation method for the resulting yield curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterpolationMethod {
    /// Linear interpolation on discount factors.
    Linear,
    /// Log-linear interpolation (default, preserves no-arbitrage).
    #[default]
    LogLinear,
    /// Flat forward interpolation.
    FlatForward,
    /// Natural cubic spline on instantaneous forward rates.
    CubicSplineFwd,
    /// Monotone convex interpolation (Hagan & West, 2006).
    MonotoneConvex,
    /// Natural cubic spline on log(DF).
    LogCubicDF,
    /// Tension spline on instantaneous forward rates.
    TensionSpline,
}

/// Error type for curve definition validation.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveDefError {
    /// Missing required field.
    MissingField(&'static str),
    /// No instruments specified.
    NoInstruments,
    /// Unknown rate index reference.
    UnknownRateIndex(String),
    /// Unknown instrument reference.
    UnknownInstrument(String),
    /// Duplicate instrument in the list.
    DuplicateInstrument(String),
    /// Duplicate jump pillar date.
    DuplicateJumpDate(Date),
    /// Invalid confidence value for jump pillar.
    InvalidConfidence {
        /// The date of the problematic jump pillar.
        date: Date,
        /// The invalid confidence value.
        value: f64,
    },
    /// Jump would cause negative discount factor.
    JumpWouldCauseNegativeDF {
        /// The date of the problematic jump pillar.
        date: Date,
        /// The jump size in basis points.
        jump_bps: f64,
    },
    /// Invalid end date for turn pillar (end_date must be after jump_date).
    InvalidEndDate {
        /// The jump date.
        jump_date: Date,
        /// The invalid end date.
        end_date: Date,
    },
}

impl std::fmt::Display for CurveDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::NoInstruments => write!(f, "No instruments specified"),
            Self::UnknownRateIndex(id) => write!(f, "Unknown rate index: {}", id),
            Self::UnknownInstrument(id) => write!(f, "Unknown instrument: {}", id),
            Self::DuplicateInstrument(id) => write!(f, "Duplicate instrument: {}", id),
            Self::DuplicateJumpDate(date) => write!(f, "Duplicate jump pillar date: {}", date),
            Self::InvalidConfidence { date, value } => {
                write!(
                    f,
                    "Invalid confidence {} for jump pillar at {}: must be in [0.0, 1.0]",
                    value, date
                )
            }
            Self::JumpWouldCauseNegativeDF { date, jump_bps } => {
                write!(
                    f,
                    "Jump of {}bp at {} would cause negative discount factor",
                    jump_bps, date
                )
            }
            Self::InvalidEndDate {
                jump_date,
                end_date,
            } => {
                write!(
                    f,
                    "Turn pillar end_date {} must be after jump_date {}",
                    end_date, jump_date
                )
            }
        }
    }
}

impl std::error::Error for CurveDefError {}

impl CurveDefinition {
    /// Creates a new curve definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        rate_index: impl Into<String>,
        instruments: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            rate_index: rate_index.into(),
            instruments,
            jump_pillars: Vec::new(),
            calibration_method: CalibrationMethod::default(),
            interpolation: InterpolationMethod::default(),
            allow_extrapolation: true,
        }
    }

    /// Sets the calibration method.
    #[must_use]
    pub fn with_calibration_method(mut self, method: CalibrationMethod) -> Self {
        self.calibration_method = method;
        self
    }

    /// Sets the interpolation method.
    #[must_use]
    pub fn with_interpolation(mut self, interpolation: InterpolationMethod) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets whether extrapolation is allowed.
    #[must_use]
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Adds an instrument to the definition.
    #[must_use]
    pub fn with_instrument(mut self, instrument_id: impl Into<String>) -> Self {
        self.instruments.push(instrument_id.into());
        self
    }

    /// Sets the jump pillars for rate discontinuities.
    #[must_use]
    pub fn with_jump_pillars(mut self, pillars: Vec<JumpPillar>) -> Self {
        self.jump_pillars = pillars;
        self
    }

    /// Adds a single jump pillar to the definition.
    #[must_use]
    pub fn with_jump_pillar(mut self, pillar: JumpPillar) -> Self {
        self.jump_pillars.push(pillar);
        self
    }

    /// Returns the number of jump pillars in the definition.
    #[must_use]
    pub fn jump_pillar_count(&self) -> usize { self.jump_pillars.len() }

    /// Returns true if the definition has any jump pillars.
    #[must_use]
    pub fn has_jumps(&self) -> bool { !self.jump_pillars.is_empty() }

    /// Validates the curve definition (basic validation only).
    pub fn validate(&self) -> Result<(), CurveDefError> {
        if self.name.is_empty() {
            return Err(CurveDefError::MissingField("name"));
        }

        if self.rate_index.is_empty() {
            return Err(CurveDefError::MissingField("rate_index"));
        }

        if self.instruments.is_empty() {
            return Err(CurveDefError::NoInstruments);
        }

        let mut seen = std::collections::HashSet::new();
        for inst_id in &self.instruments {
            if !seen.insert(inst_id) {
                return Err(CurveDefError::DuplicateInstrument(inst_id.clone()));
            }
        }

        self.validate_jump_pillars()?;

        Ok(())
    }

    /// Validates the jump pillars.
    fn validate_jump_pillars(&self) -> Result<(), CurveDefError> {
        if self.jump_pillars.is_empty() {
            return Ok(());
        }

        let mut seen_dates = std::collections::HashSet::new();

        for pillar in &self.jump_pillars {
            if !seen_dates.insert(pillar.jump_date()) {
                return Err(CurveDefError::DuplicateJumpDate(pillar.jump_date()));
            }

            let confidence = pillar.confidence();
            if !(0.0..=1.0).contains(&confidence) {
                return Err(CurveDefError::InvalidConfidence {
                    date: pillar.jump_date(),
                    value: confidence,
                });
            }

            let jump_bps = pillar.expected_jump_bps();
            if jump_bps.abs() > 10000.0 {
                return Err(CurveDefError::JumpWouldCauseNegativeDF {
                    date: pillar.jump_date(),
                    jump_bps,
                });
            }

            if let Some(end_date) = pillar.end_date() {
                if end_date <= pillar.jump_date() {
                    return Err(CurveDefError::InvalidEndDate {
                        jump_date: pillar.jump_date(),
                        end_date,
                    });
                }
            }
        }

        Ok(())
    }

    /// Returns the number of instruments in the definition.
    #[must_use]
    pub fn instrument_count(&self) -> usize { self.instruments.len() }
}

impl CalibrationMethod {
    /// Returns the display name of the calibration method.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential Bootstrapping",
            Self::Global => "Global Calibration",
            Self::LevenbergMarquardt => "Levenberg-Marquardt",
            Self::Penalised => "Penalised",
            Self::BestFit => "Best Fit",
        }
    }
}

impl InterpolationMethod {
    /// Returns the display name of the interpolation method.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::LogLinear => "Log-Linear",
            Self::FlatForward => "Flat Forward",
            Self::CubicSplineFwd => "Cubic Spline (Fwd)",
            Self::MonotoneConvex => "Monotone Convex",
            Self::LogCubicDF => "Log-Cubic DF",
            Self::TensionSpline => "Tension Spline",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::events::EventType;

    fn test_date() -> Date { Date::from_ymd(2024, 3, 20).unwrap() }

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
        let high = JumpPillar::new(test_date(), 25.0, 1.5);
        assert_eq!(high.confidence(), 1.0);

        let low = JumpPillar::new(test_date(), 25.0, -0.5);
        assert_eq!(low.confidence(), 0.0);

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
        assert!((jump.weighted_jump_bps() - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_expected_jump_rate() {
        let jump = JumpPillar::new(test_date(), 25.0, 1.0);
        assert!((jump.expected_jump_rate() - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn test_weighted_jump_rate() {
        let jump = JumpPillar::new(test_date(), 50.0, 0.60);
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
    fn test_jump_pillar_clone_and_partial_eq() {
        let jump1 = JumpPillar::new(test_date(), 25.0, 0.85);
        let jump2 = jump1.clone();

        assert_eq!(jump1, jump2);
    }

    #[test]
    fn test_jump_pillar_debug() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);
        let debug = format!("{:?}", jump);

        assert!(debug.contains("JumpPillar"));
        assert!(debug.contains("25"));
    }

    #[test]
    fn test_jump_pillar_with_end_date() {
        let start = Date::from_ymd(2024, 12, 31).unwrap();
        let end = Date::from_ymd(2025, 1, 2).unwrap();
        let jump = JumpPillar::new(start, 12.5, 1.0).with_end_date(end);

        assert_eq!(jump.end_date(), Some(end));
        assert!(jump.is_turn());
    }

    #[test]
    fn test_jump_pillar_permanent_has_no_end_date() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);
        assert!(jump.end_date().is_none());
        assert!(!jump.is_turn());
    }

    #[test]
    fn test_from_event_instrument_with_turn() {
        let event_date = Date::from_ymd(2024, 12, 31).unwrap();
        let end_date = Date::from_ymd(2025, 1, 2).unwrap();
        let event = EventInstrument::new_turn(
            event_date,
            end_date,
            EventType::TurnOfYear,
            12.5,
            1.0,
            RateIndex::Sofr,
        );

        let jump = JumpPillar::from_event_instrument(&event);
        assert_eq!(jump.jump_date(), event_date);
        assert_eq!(jump.end_date(), Some(end_date));
        assert!(jump.is_turn());
        assert_eq!(jump.expected_jump_bps(), 12.5);
    }

    #[test]
    fn test_from_event_instrument_permanent_jump() {
        let event = EventInstrument::new(
            test_date(),
            EventType::CentralBankMeeting,
            25.0,
            0.85,
            RateIndex::Sofr,
        );

        let jump = JumpPillar::from_event_instrument(&event);
        assert!(jump.end_date().is_none());
        assert!(!jump.is_turn());
    }

    #[test]
    fn test_jump_pillar_serde_roundtrip() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85).with_event_reference("FOMC-2024-03");

        let json = serde_json::to_string(&jump).unwrap();
        let parsed: JumpPillar = serde_json::from_str(&json).unwrap();

        assert_eq!(jump, parsed);
    }

    #[test]
    fn test_jump_pillar_serde_without_event_reference() {
        let jump = JumpPillar::new(test_date(), 25.0, 0.85);

        let json = serde_json::to_string(&jump).unwrap();

        assert!(!json.contains("eventReference"));

        let parsed: JumpPillar = serde_json::from_str(&json).unwrap();
        assert_eq!(jump, parsed);
    }

    #[test]
    fn test_jump_pillar_serde_camel_case() {
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

    #[test]
    fn test_jump_pillar_serde_defaults() {
        let json = r#"{
            "jumpDate": "2024-03-20",
            "expectedJumpBps": 25.0,
            "confidence": 0.85
        }"#;

        let jump: JumpPillar = serde_json::from_str(json).unwrap();
        assert!(jump.event_reference().is_none());
    }

    fn make_event(date: Date, spread: f64, confidence: f64, index: RateIndex) -> EventInstrument {
        EventInstrument::new(
            date,
            EventType::CentralBankMeeting,
            spread,
            confidence,
            index,
        )
    }

    #[test]
    fn test_builder_empty_input() {
        let pillars = JumpPillarBuilder::new(vec![]).build();
        assert!(pillars.is_empty());
    }

    #[test]
    fn test_builder_no_filters() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.70,
                RateIndex::Estr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events).build();

        assert_eq!(pillars.len(), 2);
    }

    #[test]
    fn test_builder_rate_index_filter() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.70,
                RateIndex::Estr,
            ),
            make_event(
                Date::from_ymd(2024, 9, 18).unwrap(),
                25.0,
                0.80,
                RateIndex::Sofr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events)
            .with_rate_index(RateIndex::Sofr)
            .build();

        assert_eq!(pillars.len(), 2);
        assert_eq!(pillars[0].jump_date(), Date::from_ymd(2024, 3, 20).unwrap());
        assert_eq!(pillars[1].jump_date(), Date::from_ymd(2024, 9, 18).unwrap());
    }

    #[test]
    fn test_builder_date_range_filter() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 1, 15).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.70,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 9, 18).unwrap(),
                25.0,
                0.80,
                RateIndex::Sofr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events)
            .with_date_range(
                Date::from_ymd(2024, 3, 1).unwrap(),
                Date::from_ymd(2024, 6, 30).unwrap(),
            )
            .build();

        assert_eq!(pillars.len(), 2);
        assert_eq!(pillars[0].jump_date(), Date::from_ymd(2024, 3, 20).unwrap());
        assert_eq!(pillars[1].jump_date(), Date::from_ymd(2024, 6, 12).unwrap());
    }

    #[test]
    fn test_builder_min_confidence_filter() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.90,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.50,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 9, 18).unwrap(),
                25.0,
                0.75,
                RateIndex::Sofr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events)
            .with_min_confidence(0.75)
            .build();

        assert_eq!(pillars.len(), 2);
        assert_eq!(pillars[0].jump_date(), Date::from_ymd(2024, 3, 20).unwrap());
        assert_eq!(pillars[1].jump_date(), Date::from_ymd(2024, 9, 18).unwrap());
    }

    #[test]
    fn test_builder_combined_filters() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 1, 15).unwrap(),
                25.0,
                0.90,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 3, 25).unwrap(),
                25.0,
                0.50,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.70,
                RateIndex::Estr,
            ),
            make_event(
                Date::from_ymd(2024, 9, 18).unwrap(),
                25.0,
                0.80,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 12, 1).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events)
            .with_rate_index(RateIndex::Sofr)
            .with_date_range(
                Date::from_ymd(2024, 3, 1).unwrap(),
                Date::from_ymd(2024, 10, 31).unwrap(),
            )
            .with_min_confidence(0.70)
            .build();

        assert_eq!(pillars.len(), 2);
        assert_eq!(pillars[0].jump_date(), Date::from_ymd(2024, 3, 20).unwrap());
        assert_eq!(pillars[1].jump_date(), Date::from_ymd(2024, 9, 18).unwrap());
    }

    #[test]
    fn test_builder_sorted_by_date() {
        let events = vec![
            make_event(
                Date::from_ymd(2024, 9, 18).unwrap(),
                25.0,
                0.80,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
                RateIndex::Sofr,
            ),
            make_event(
                Date::from_ymd(2024, 6, 12).unwrap(),
                25.0,
                0.70,
                RateIndex::Sofr,
            ),
        ];

        let pillars = JumpPillarBuilder::new(events).build();

        assert_eq!(pillars.len(), 3);
        assert!(pillars[0].jump_date() < pillars[1].jump_date());
        assert!(pillars[1].jump_date() < pillars[2].jump_date());
    }

    #[test]
    fn test_builder_preserves_event_data() {
        let events = vec![make_event(
            Date::from_ymd(2024, 3, 20).unwrap(),
            50.0,
            0.90,
            RateIndex::Sofr,
        )];

        let pillars = JumpPillarBuilder::new(events).build();

        assert_eq!(pillars.len(), 1);
        assert_eq!(pillars[0].expected_jump_bps(), 50.0);
        assert_eq!(pillars[0].confidence(), 0.90);
    }

    #[test]
    fn test_builder_min_confidence_clamping() {
        let events = vec![make_event(
            Date::from_ymd(2024, 3, 20).unwrap(),
            25.0,
            0.50,
            RateIndex::Sofr,
        )];

        let pillars = JumpPillarBuilder::new(events.clone())
            .with_min_confidence(-0.5)
            .build();
        assert_eq!(pillars.len(), 1);

        let pillars = JumpPillarBuilder::new(events)
            .with_min_confidence(1.5)
            .build();
        assert_eq!(pillars.len(), 0);
    }

    #[test]
    fn test_curve_definition_new() {
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-OIS-5Y".to_string()],
        );

        assert_eq!(curve.name, "USD-SOFR-Discount");
        assert_eq!(curve.rate_index, "USD-SOFR");
        assert_eq!(curve.instruments.len(), 2);
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
    }

    #[test]
    fn test_curve_definition_builder() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec![])
            .with_instrument("USD-Depo-ON")
            .with_instrument("USD-OIS-5Y")
            .with_calibration_method(CalibrationMethod::Global)
            .with_interpolation(InterpolationMethod::FlatForward)
            .with_extrapolation(false);

        assert_eq!(curve.instruments.len(), 2);
        assert_eq!(curve.calibration_method, CalibrationMethod::Global);
        assert_eq!(curve.interpolation, InterpolationMethod::FlatForward);
        assert!(!curve.allow_extrapolation);
    }

    #[test]
    fn test_validate_success() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-Depo-ON".to_string()]);
        assert!(curve.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let curve = CurveDefinition::new("", "USD-SOFR", vec!["USD-Depo-ON".to_string()]);
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::MissingField("name"))
        ));
    }

    #[test]
    fn test_validate_empty_rate_index() {
        let curve = CurveDefinition::new("USD-SOFR", "", vec!["USD-Depo-ON".to_string()]);
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::MissingField("rate_index"))
        ));
    }

    #[test]
    fn test_validate_no_instruments() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec![]);
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::NoInstruments)
        ));
    }

    #[test]
    fn test_validate_duplicate_instrument() {
        let curve = CurveDefinition::new(
            "USD-SOFR",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-Depo-ON".to_string()],
        );
        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::DuplicateInstrument(_))
        ));
    }

    #[test]
    fn test_instrument_count() {
        let curve = CurveDefinition::new(
            "USD-SOFR",
            "USD-SOFR",
            vec![
                "USD-Depo-ON".to_string(),
                "USD-OIS-1Y".to_string(),
                "USD-OIS-5Y".to_string(),
            ],
        );
        assert_eq!(curve.instrument_count(), 3);
    }

    #[test]
    fn test_calibration_method_defaults() {
        assert_eq!(CalibrationMethod::default(), CalibrationMethod::Sequential);
    }

    #[test]
    fn test_interpolation_method_defaults() {
        assert_eq!(
            InterpolationMethod::default(),
            InterpolationMethod::LogLinear
        );
    }

    #[test]
    fn test_serde_roundtrip() {
        let curve = CurveDefinition::new(
            "USD-SOFR-Discount",
            "USD-SOFR",
            vec!["USD-Depo-ON".to_string(), "USD-OIS-5Y".to_string()],
        )
        .with_calibration_method(CalibrationMethod::Global);

        let json = serde_json::to_string(&curve).unwrap();
        let parsed: CurveDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(curve, parsed);
    }

    #[test]
    fn test_serde_from_json() {
        let json = r#"{
            "name": "USD-SOFR-Discount",
            "rateIndex": "USD-SOFR",
            "instruments": ["USD-Depo-ON", "USD-OIS-1Y", "USD-OIS-5Y"],
            "calibrationMethod": "sequential",
            "interpolation": "loglinear",
            "allowExtrapolation": true
        }"#;

        let curve: CurveDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(curve.name, "USD-SOFR-Discount");
        assert_eq!(curve.rate_index, "USD-SOFR");
        assert_eq!(curve.instruments.len(), 3);
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
    }

    #[test]
    fn test_serde_defaults_applied() {
        let json = r#"{
            "name": "EUR-ESTR",
            "rateIndex": "EUR-ESTR",
            "instruments": ["EUR-Depo-ON"]
        }"#;

        let curve: CurveDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(curve.calibration_method, CalibrationMethod::Sequential);
        assert_eq!(curve.interpolation, InterpolationMethod::LogLinear);
        assert!(curve.allow_extrapolation);
        assert!(curve.jump_pillars.is_empty());
    }

    #[test]
    fn test_with_jump_pillars() {
        let pillars = vec![
            JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.85),
            JumpPillar::new(Date::from_ymd(2024, 6, 12).unwrap(), -25.0, 0.70),
        ];

        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillars(pillars);

        assert_eq!(curve.jump_pillar_count(), 2);
        assert!(curve.has_jumps());
    }

    #[test]
    fn test_with_jump_pillar() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
            ))
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 6, 12).unwrap(),
                -25.0,
                0.70,
            ));

        assert_eq!(curve.jump_pillar_count(), 2);
    }

    #[test]
    fn test_has_jumps_empty() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()]);
        assert!(!curve.has_jumps());
    }

    #[test]
    fn test_validate_with_valid_jump_pillars() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
            ));

        assert!(curve.validate().is_ok());
    }

    #[test]
    fn test_validate_duplicate_jump_date() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
            ))
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                -25.0,
                0.70,
            ));

        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::DuplicateJumpDate(_))
        ));
    }

    #[test]
    fn test_validate_invalid_end_date() {
        let jump_date = Date::from_ymd(2024, 12, 31).unwrap();
        let end_date = Date::from_ymd(2024, 12, 30).unwrap();
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(jump_date, 12.5, 1.0).with_end_date(end_date));

        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::InvalidEndDate { .. })
        ));
    }

    #[test]
    fn test_validate_valid_turn_pillar() {
        let jump_date = Date::from_ymd(2024, 12, 31).unwrap();
        let end_date = Date::from_ymd(2025, 1, 2).unwrap();
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(jump_date, 12.5, 1.0).with_end_date(end_date));

        assert!(curve.validate().is_ok());
    }

    #[test]
    fn test_validate_extreme_jump() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                20000.0,
                0.85,
            ));

        assert!(matches!(
            curve.validate(),
            Err(CurveDefError::JumpWouldCauseNegativeDF { .. })
        ));
    }

    #[test]
    fn test_serde_with_jump_pillars() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()])
            .with_jump_pillar(JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.85,
            ));

        let json = serde_json::to_string(&curve).unwrap();
        let parsed: CurveDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(curve, parsed);
        assert_eq!(parsed.jump_pillar_count(), 1);
    }

    #[test]
    fn test_serde_empty_jump_pillars_omitted() {
        let curve = CurveDefinition::new("USD-SOFR", "USD-SOFR", vec!["USD-OIS-1Y".to_string()]);

        let json = serde_json::to_string(&curve).unwrap();
        assert!(!json.contains("jumpPillars"));
    }

    #[test]
    fn test_serde_backward_compatibility() {
        let json = r#"{
            "name": "USD-SOFR",
            "rateIndex": "USD-SOFR",
            "instruments": ["USD-OIS-1Y"]
        }"#;

        let curve: CurveDefinition = serde_json::from_str(json).unwrap();
        assert!(!curve.has_jumps());
        assert!(curve.validate().is_ok());
    }
}

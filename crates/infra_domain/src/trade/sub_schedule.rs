//! Sub-schedule types for compound/averaged coupon periods.

use super::payoff::Payoff;
use crate::time::Date;

/// A sub-period within a coupon period, used for averaging or compounding.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubSchedule {
    /// Fixing date for the sub-period rate observation.
    pub fixing: Date,
    /// Start date of the sub-period.
    pub start: Date,
    /// End date of the sub-period.
    pub end: Date,
    /// Day count fraction for this sub-period.
    pub day_fraction: f64,
    /// Payoff formula for the sub-period rate.
    pub compound_payoff: Payoff,
    /// Fixed spread value, if applicable.
    pub spread: Option<f64>,
}

impl SubSchedule {
    /// Creates a new sub-schedule entry.
    #[must_use]
    pub fn new(
        fixing: Date,
        start: Date,
        end: Date,
        day_fraction: f64,
        compound_payoff: Payoff,
    ) -> Self {
        Self {
            fixing,
            start,
            end,
            day_fraction,
            compound_payoff,
            spread: None,
        }
    }

    /// Sets the spread value.
    #[must_use]
    pub fn with_spread(mut self, spread: f64) -> Self {
        self.spread = Some(spread);
        self
    }

    /// Returns the spread value if set.
    #[must_use]
    pub fn spread_value(&self) -> f64 { self.spread.unwrap_or(0.0) }
}

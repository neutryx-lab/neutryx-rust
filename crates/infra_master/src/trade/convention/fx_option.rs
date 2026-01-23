//! FX option convention definitions.
//!
//! This module provides types for representing FX option market conventions.

use crate::{CalendarId, Currency};

/// Premium currency specification for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PremiumCurrency {
    /// Premium paid in base currency (first in pair).
    Base,
    /// Premium paid in quote currency (second in pair).
    Quote,
    /// Premium paid in a custom currency.
    Custom(Currency),
}

/// Delta convention for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeltaConvention {
    /// Spot delta (delta with respect to spot FX rate).
    SpotDelta,
    /// Forward delta (delta with respect to forward FX rate).
    ForwardDelta,
}

/// Cut-off time specification for FX option expiry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CutOffTime {
    /// Hour (0-23) in the specified timezone.
    pub hour: u8,
    /// Minute (0-59).
    pub minute: u8,
    /// Timezone identifier (e.g., "NY", "LON", "TOK").
    pub timezone: String,
}

impl CutOffTime {
    /// Creates a new cut-off time.
    #[must_use]
    pub fn new(hour: u8, minute: u8, timezone: impl Into<String>) -> Self {
        Self {
            hour,
            minute,
            timezone: timezone.into(),
        }
    }

    /// Returns the standard New York cut-off time (10:00 NY).
    #[must_use]
    pub fn ny_cut() -> Self {
        Self::new(10, 0, "NY")
    }

    /// Returns the standard Tokyo cut-off time (15:00 TOK).
    #[must_use]
    pub fn tokyo_cut() -> Self {
        Self::new(15, 0, "TOK")
    }

    /// Returns the standard London cut-off time (10:00 LON).
    #[must_use]
    pub fn london_cut() -> Self {
        Self::new(10, 0, "LON")
    }
}

impl std::fmt::Display for CutOffTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02} {}", self.hour, self.minute, self.timezone)
    }
}

/// Convention for FX options.
///
/// Represents the market conventions for pricing and settling FX options.
///
/// # Example
///
/// ```rust
/// use infra_master::trade::convention::{
///     FxOptionConvention, PremiumCurrency, DeltaConvention, CutOffTime,
/// };
/// use infra_master::CalendarId;
///
/// let conv = FxOptionConvention::g10_standard();
/// assert_eq!(conv.delta_convention, DeltaConvention::SpotDelta);
/// assert_eq!(conv.settlement_days, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxOptionConvention {
    /// Premium currency specification.
    pub premium_currency: PremiumCurrency,
    /// Delta convention for quoting.
    pub delta_convention: DeltaConvention,
    /// Cut-off time for expiry.
    pub cut_off_time: CutOffTime,
    /// Number of business days to settlement after expiry.
    pub settlement_days: u32,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Whether to use premium-adjusted delta.
    pub premium_adjusted_delta: bool,
}

impl FxOptionConvention {
    /// Creates a new FX option convention.
    #[must_use]
    pub fn new(
        premium_currency: PremiumCurrency,
        delta_convention: DeltaConvention,
        cut_off_time: CutOffTime,
        settlement_days: u32,
        calendar: CalendarId,
        premium_adjusted_delta: bool,
    ) -> Self {
        Self {
            premium_currency,
            delta_convention,
            cut_off_time,
            settlement_days,
            calendar,
            premium_adjusted_delta,
        }
    }

    /// Returns the standard G10 FX option convention.
    ///
    /// - Premium currency: Quote (USD for most pairs)
    /// - Delta: Spot delta
    /// - Cut-off: 10:00 NY
    /// - Settlement: T+2
    /// - Calendar: New York
    #[must_use]
    pub fn g10_standard() -> Self {
        Self {
            premium_currency: PremiumCurrency::Quote,
            delta_convention: DeltaConvention::SpotDelta,
            cut_off_time: CutOffTime::ny_cut(),
            settlement_days: 2,
            calendar: CalendarId::NewYork,
            premium_adjusted_delta: false,
        }
    }

    /// Returns the EUR/USD FX option convention.
    ///
    /// - Premium currency: USD (Quote)
    /// - Delta: Spot delta
    /// - Cut-off: 10:00 NY
    /// - Settlement: T+2
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            premium_currency: PremiumCurrency::Quote,
            delta_convention: DeltaConvention::SpotDelta,
            cut_off_time: CutOffTime::ny_cut(),
            settlement_days: 2,
            calendar: CalendarId::NewYork,
            premium_adjusted_delta: false,
        }
    }

    /// Returns the USD/JPY FX option convention.
    ///
    /// - Premium currency: USD (Base)
    /// - Delta: Spot delta, premium-adjusted
    /// - Cut-off: 15:00 TOK
    /// - Settlement: T+2
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            premium_currency: PremiumCurrency::Base,
            delta_convention: DeltaConvention::SpotDelta,
            cut_off_time: CutOffTime::tokyo_cut(),
            settlement_days: 2,
            calendar: CalendarId::Tokyo,
            premium_adjusted_delta: true,
        }
    }

    /// Returns the GBP/USD FX option convention.
    ///
    /// - Premium currency: USD (Quote)
    /// - Delta: Spot delta
    /// - Cut-off: 10:00 NY
    /// - Settlement: T+2
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            premium_currency: PremiumCurrency::Quote,
            delta_convention: DeltaConvention::SpotDelta,
            cut_off_time: CutOffTime::ny_cut(),
            settlement_days: 2,
            calendar: CalendarId::NewYork,
            premium_adjusted_delta: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cut_off_time_new() {
        let cut_off = CutOffTime::new(10, 30, "NY");
        assert_eq!(cut_off.hour, 10);
        assert_eq!(cut_off.minute, 30);
        assert_eq!(cut_off.timezone, "NY");
    }

    #[test]
    fn test_cut_off_time_ny_cut() {
        let cut_off = CutOffTime::ny_cut();
        assert_eq!(cut_off.hour, 10);
        assert_eq!(cut_off.minute, 0);
        assert_eq!(cut_off.timezone, "NY");
    }

    #[test]
    fn test_cut_off_time_tokyo_cut() {
        let cut_off = CutOffTime::tokyo_cut();
        assert_eq!(cut_off.hour, 15);
        assert_eq!(cut_off.minute, 0);
        assert_eq!(cut_off.timezone, "TOK");
    }

    #[test]
    fn test_cut_off_time_display() {
        let cut_off = CutOffTime::ny_cut();
        assert_eq!(cut_off.to_string(), "10:00 NY");
    }

    #[test]
    fn test_fx_option_convention_new() {
        let conv = FxOptionConvention::new(
            PremiumCurrency::Quote,
            DeltaConvention::ForwardDelta,
            CutOffTime::london_cut(),
            2,
            CalendarId::London,
            true,
        );

        assert_eq!(conv.premium_currency, PremiumCurrency::Quote);
        assert_eq!(conv.delta_convention, DeltaConvention::ForwardDelta);
        assert_eq!(conv.settlement_days, 2);
        assert!(conv.premium_adjusted_delta);
    }

    #[test]
    fn test_g10_standard_convention() {
        let conv = FxOptionConvention::g10_standard();

        assert_eq!(conv.premium_currency, PremiumCurrency::Quote);
        assert_eq!(conv.delta_convention, DeltaConvention::SpotDelta);
        assert_eq!(conv.settlement_days, 2);
        assert!(!conv.premium_adjusted_delta);
    }

    #[test]
    fn test_eur_usd_convention() {
        let conv = FxOptionConvention::eur_usd();

        assert_eq!(conv.premium_currency, PremiumCurrency::Quote);
        assert_eq!(conv.cut_off_time.timezone, "NY");
    }

    #[test]
    fn test_usd_jpy_convention() {
        let conv = FxOptionConvention::usd_jpy();

        assert_eq!(conv.premium_currency, PremiumCurrency::Base);
        assert_eq!(conv.cut_off_time.timezone, "TOK");
        assert!(conv.premium_adjusted_delta);
    }

    #[test]
    fn test_gbp_usd_convention() {
        let conv = FxOptionConvention::gbp_usd();

        assert_eq!(conv.premium_currency, PremiumCurrency::Quote);
        assert_eq!(conv.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_premium_currency_equality() {
        assert_eq!(PremiumCurrency::Base, PremiumCurrency::Base);
        assert_ne!(PremiumCurrency::Base, PremiumCurrency::Quote);
        assert_eq!(
            PremiumCurrency::Custom(Currency::USD),
            PremiumCurrency::Custom(Currency::USD)
        );
    }

    #[test]
    fn test_delta_convention_equality() {
        assert_eq!(DeltaConvention::SpotDelta, DeltaConvention::SpotDelta);
        assert_ne!(DeltaConvention::SpotDelta, DeltaConvention::ForwardDelta);
    }

    #[test]
    fn test_fx_option_convention_clone() {
        let conv = FxOptionConvention::g10_standard();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}

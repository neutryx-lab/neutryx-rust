//! Foreign exchange convention definitions.
//!
//! This module provides types for all FX-related conventions:
//!
//! - [`FxConvention`]: Spot FX conventions
//! - [`FxOptionConvention`], [`PremiumCurrency`], [`DeltaConvention`],
//!   [`CutOffTime`]: FX option conventions
//! - [`FxSwapConvention`], [`FxSettlementType`], [`NearLegType`]: FX swap
//!   conventions

use crate::{
    market::Currency,
    time::{BusinessDayConvention, CalendarId},
};

// ============================================================================
// FX Spot Conventions
// ============================================================================

/// Convention for foreign exchange transactions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxConvention {
    /// Number of spot days.
    pub spot_days: u32,
    /// Calendar for settlement.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
}

impl FxConvention {
    /// Creates a new FX convention.
    #[must_use]
    pub fn new(
        spot_days: u32,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
    ) -> Self {
        Self {
            spot_days,
            calendar,
            business_day_convention,
        }
    }
}

super::define_convention_factories! {
    for FxConvention;
    /// Returns the USD/JPY FX convention.
    usd_jpy => {
        spot_days: 2, calendar: CalendarId::Tokyo,
        business_day_convention: BusinessDayConvention::ModifiedFollowing,
    };
    /// Returns the EUR/USD FX convention.
    eur_usd => {
        spot_days: 2, calendar: CalendarId::Target,
        business_day_convention: BusinessDayConvention::ModifiedFollowing,
    };
    /// Returns the GBP/USD FX convention.
    gbp_usd => {
        spot_days: 2, calendar: CalendarId::London,
        business_day_convention: BusinessDayConvention::ModifiedFollowing,
    };
    /// Returns the default USD FX convention.
    usd_default => {
        spot_days: 2, calendar: CalendarId::NewYork,
        business_day_convention: BusinessDayConvention::ModifiedFollowing,
    };
}

impl FxConvention {
    /// Returns the default EUR FX convention.
    #[must_use]
    pub fn eur_default() -> Self { Self::eur_usd() }

    /// Returns the default GBP FX convention.
    #[must_use]
    pub fn gbp_default() -> Self { Self::gbp_usd() }

    /// Returns the default JPY FX convention.
    #[must_use]
    pub fn jpy_default() -> Self { Self::usd_jpy() }
}

// ============================================================================
// FX Option Conventions
// ============================================================================

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
    pub fn ny_cut() -> Self { Self::new(10, 0, "NY") }

    /// Returns the standard Tokyo cut-off time (15:00 TOK).
    #[must_use]
    pub fn tokyo_cut() -> Self { Self::new(15, 0, "TOK") }

    /// Returns the standard London cut-off time (10:00 LON).
    #[must_use]
    pub fn london_cut() -> Self { Self::new(10, 0, "LON") }
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
/// use infra_domain::market::convention::{
///     FxOptionConvention, PremiumCurrency, DeltaConvention, CutOffTime,
/// };
/// use infra_domain::time::CalendarId;
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

// ============================================================================
// FX Swap Conventions
// ============================================================================

/// Settlement type for FX transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxSettlementType {
    /// Deliverable (physical exchange of currencies).
    Deliverable,
    /// Non-deliverable (cash settlement in reference currency).
    NonDeliverable,
}

/// Near leg type for FX swaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NearLegType {
    /// Today (T+0).
    Today,
    /// Tomorrow (T+1).
    Tomorrow,
    /// Spot (T+spot_days, typically T+2).
    Spot,
}

/// Convention for an FX swap.
///
/// Represents the market conventions for pricing and settling FX swaps
/// where two FX transactions (near and far legs) are executed simultaneously.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::{FxSwapConvention, NearLegType};
///
/// let conv = FxSwapConvention::usd_jpy();
/// assert_eq!(conv.near_leg_type, NearLegType::Spot);
/// assert_eq!(conv.spot_days, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwapConvention {
    /// Base currency (first in pair).
    pub base_currency: Currency,
    /// Quote currency (second in pair).
    pub quote_currency: Currency,
    /// Type of near leg (Today, Tomorrow, or Spot).
    pub near_leg_type: NearLegType,
    /// Number of spot days.
    pub spot_days: u32,
    /// Calendar for base currency.
    pub base_calendar: CalendarId,
    /// Calendar for quote currency.
    pub quote_calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Settlement type.
    pub settlement_type: FxSettlementType,
}

impl FxSwapConvention {
    /// Creates a new FX swap convention.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        near_leg_type: NearLegType,
        spot_days: u32,
        base_calendar: CalendarId,
        quote_calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        settlement_type: FxSettlementType,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            near_leg_type,
            spot_days,
            base_calendar,
            quote_calendar,
            business_day_convention,
            settlement_type,
        }
    }

    /// Returns the USD/JPY FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: New York
    /// - Quote calendar: Tokyo
    /// - Settlement: Deliverable
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            base_currency: Currency::USD,
            quote_currency: Currency::JPY,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::NewYork,
            quote_calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the EUR/USD FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: TARGET
    /// - Quote calendar: New York
    /// - Settlement: Deliverable
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            base_currency: Currency::EUR,
            quote_currency: Currency::USD,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::Target,
            quote_calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the GBP/USD FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: London
    /// - Quote calendar: New York
    /// - Settlement: Deliverable
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            base_currency: Currency::GBP,
            quote_currency: Currency::USD,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::London,
            quote_calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Returns the EUR/JPY FX swap convention.
    ///
    /// - Near leg: Spot
    /// - Spot days: 2
    /// - Base calendar: TARGET
    /// - Quote calendar: Tokyo
    /// - Settlement: Deliverable
    #[must_use]
    pub fn eur_jpy() -> Self {
        Self {
            base_currency: Currency::EUR,
            quote_currency: Currency::JPY,
            near_leg_type: NearLegType::Spot,
            spot_days: 2,
            base_calendar: CalendarId::Target,
            quote_calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            settlement_type: FxSettlementType::Deliverable,
        }
    }

    /// Creates a tom/next FX swap convention based on an existing convention.
    ///
    /// Tom/next swaps have near leg on T+1 (tomorrow) and far leg on T+2
    /// (spot).
    #[must_use]
    pub fn as_tom_next(&self) -> Self {
        Self {
            near_leg_type: NearLegType::Tomorrow,
            ..self.clone()
        }
    }

    /// Creates an overnight FX swap convention based on an existing convention.
    ///
    /// Overnight swaps have near leg on T+0 (today) and far leg on T+1
    /// (tomorrow).
    #[must_use]
    pub fn as_overnight(&self) -> Self {
        Self {
            near_leg_type: NearLegType::Today,
            ..self.clone()
        }
    }

    /// Returns whether this is a deliverable FX swap.
    #[must_use]
    pub fn is_deliverable(&self) -> bool { self.settlement_type == FxSettlementType::Deliverable }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_spot_conventions() {
        let eur = FxConvention::eur_usd();
        assert_eq!(eur.spot_days, 2);
        assert_eq!(eur.calendar, CalendarId::Target);

        let jpy = FxConvention::usd_jpy();
        assert_eq!(jpy.calendar, CalendarId::Tokyo);
    }

    #[test]
    fn test_fx_option_conventions() {
        let g10 = FxOptionConvention::g10_standard();
        assert_eq!(g10.premium_currency, PremiumCurrency::Quote);
        assert_eq!(g10.delta_convention, DeltaConvention::SpotDelta);
        assert_eq!(g10.settlement_days, 2);

        let jpy = FxOptionConvention::usd_jpy();
        assert_eq!(jpy.premium_currency, PremiumCurrency::Base);
        assert!(jpy.premium_adjusted_delta);

        assert_eq!(CutOffTime::ny_cut().to_string(), "10:00 NY");
    }

    #[test]
    fn test_fx_swap_conventions() {
        let conv = FxSwapConvention::usd_jpy();
        assert_eq!(conv.base_currency, Currency::USD);
        assert_eq!(conv.quote_currency, Currency::JPY);
        assert_eq!(conv.near_leg_type, NearLegType::Spot);
        assert!(conv.is_deliverable());

        let tom = conv.as_tom_next();
        assert_eq!(tom.near_leg_type, NearLegType::Tomorrow);

        let ovn = FxSwapConvention::eur_usd().as_overnight();
        assert_eq!(ovn.near_leg_type, NearLegType::Today);
    }
}

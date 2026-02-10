//! Interest rates convention definitions.
//!
//! This module provides types for all interest rate-related conventions:
//!
//! - [`DepositConvention`]: Money market deposit conventions
//! - [`FraConvention`]: Forward Rate Agreement conventions
//! - [`FuturesConvention`]: Interest rate futures conventions
//! - [`SwapConvention`], [`SwapLegConvention`]: Interest rate swap conventions
//! - [`BondConvention`]: Government and corporate bond conventions
//! - [`CapFloorConvention`]: Interest rate cap/floor conventions
//! - [`SwaptionConvention`], [`SettlementConvention`]: Swaption conventions
//! - [`InflationSwapConvention`], [`InflationIndex`],
//!   [`InflationInterpolation`]: Inflation swap conventions
//! - [`XCcyBasisConvention`], [`XCcyLegConvention`], [`BasisSpreadLeg`]:
//!   Cross-currency basis swap conventions

use crate::{
    market::{Currency, RateIndex},
    time::{BusinessDayConvention, CalendarId, DayCounter, Frequency},
};

// ============================================================================
// Deposit Conventions
// ============================================================================

/// Convention for a deposit (money market) instrument.
///
/// Represents the market conventions for pricing and settling deposit
/// instruments.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::DepositConvention;
///
/// let conv = DepositConvention::usd();
/// assert_eq!(conv.spot_lag, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositConvention {
    /// Day count convention for accrual calculation.
    pub day_count: DayCounter,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention for date adjustments.
    pub business_day_convention: BusinessDayConvention,
    /// Number of business days from trade date to settlement (spot lag).
    pub spot_lag: u32,
}

impl DepositConvention {
    /// Creates a new deposit convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        spot_lag: u32,
    ) -> Self {
        Self {
            day_count,
            calendar,
            business_day_convention,
            spot_lag,
        }
    }

    /// Returns the USD deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: New York
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn usd() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the EUR deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: TARGET
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn eur() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the GBP deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: London
    /// - Business day convention: Modified Following
    /// - Spot lag: T+0 (same day settlement)
    #[must_use]
    pub fn gbp() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 0,
        }
    }

    /// Returns the JPY deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: Tokyo
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn jpy() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the CHF deposit convention.
    ///
    /// - Day count: ACT/360
    /// - Calendar: TARGET (commonly used for CHF)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn chf() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the AUD deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: WeekendOnly (placeholder for Sydney)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+2
    #[must_use]
    pub fn aud() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::WeekendOnly,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the CAD deposit convention.
    ///
    /// - Day count: ACT/365 Fixed
    /// - Calendar: WeekendOnly (placeholder for Toronto)
    /// - Business day convention: Modified Following
    /// - Spot lag: T+1
    #[must_use]
    pub fn cad() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            calendar: CalendarId::WeekendOnly,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 1,
        }
    }
}

// ============================================================================
// FRA Conventions
// ============================================================================

/// Convention for a Forward Rate Agreement.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FraConvention {
    /// Day count convention.
    pub day_count: DayCounter,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
}

impl FraConvention {
    /// Creates a new FRA convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
    ) -> Self {
        Self {
            day_count,
            calendar,
            business_day_convention,
            index,
        }
    }

    /// Returns the USD SOFR FRA convention.
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Sofr,
        }
    }

    /// Returns the EUR EURIBOR 3M FRA convention.
    #[must_use]
    pub fn eur_euribor_3m() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Euribor3M,
        }
    }
}

// ============================================================================
// Futures Conventions
// ============================================================================

/// Convention for an interest rate future.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FuturesConvention {
    /// Contract size (notional per contract).
    pub contract_size: f64,
    /// Tick size (minimum price movement).
    pub tick_size: f64,
    /// Day count convention.
    pub day_count: DayCounter,
    /// Calendar for settlement.
    pub calendar: CalendarId,
}

impl FuturesConvention {
    /// Creates a new futures convention.
    #[must_use]
    pub fn new(
        contract_size: f64,
        tick_size: f64,
        day_count: DayCounter,
        calendar: CalendarId,
    ) -> Self {
        Self {
            contract_size,
            tick_size,
            day_count,
            calendar,
        }
    }

    /// Returns the CME Eurodollar futures convention.
    #[must_use]
    pub fn cme_eurodollar() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.0025, // 0.25 basis points
            day_count: DayCounter::Actual360,
            calendar: CalendarId::NewYork,
        }
    }

    /// Returns the CME SOFR futures convention.
    #[must_use]
    pub fn cme_sofr() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.0025,
            day_count: DayCounter::Actual360,
            calendar: CalendarId::NewYork,
        }
    }

    /// Returns the Eurex EURIBOR futures convention.
    #[must_use]
    pub fn eurex_euribor() -> Self {
        Self {
            contract_size: 1_000_000.0,
            tick_size: 0.005, // 0.5 basis points
            day_count: DayCounter::Actual360,
            calendar: CalendarId::Target,
        }
    }
}

// ============================================================================
// Swap Conventions
// ============================================================================

/// Convention for a single leg of a swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapLegConvention {
    /// Day count convention for this leg.
    pub day_count: DayCounter,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Number of days between end of accrual and payment.
    pub payment_lag: u32,
}

impl SwapLegConvention {
    /// Creates a new swap leg convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        payment_lag: u32,
    ) -> Self {
        Self {
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            payment_lag,
        }
    }
}

/// Convention for an interest rate swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapConvention {
    /// Convention for the fixed leg.
    pub fixed_leg: SwapLegConvention,
    /// Convention for the floating leg.
    pub float_leg: SwapLegConvention,
    /// Rate index for the floating leg.
    pub float_index: RateIndex,
    /// Number of spot days from trade date to start date.
    pub spot_lag: u32,
}

impl SwapConvention {
    /// Creates a new swap convention.
    #[must_use]
    pub fn new(
        fixed_leg: SwapLegConvention,
        float_leg: SwapLegConvention,
        float_index: RateIndex,
        spot_lag: u32,
    ) -> Self {
        Self {
            fixed_leg,
            float_leg,
            float_index,
            spot_lag,
        }
    }

    /// Returns the USD SOFR swap convention.
    ///
    /// - Fixed leg: Annual, ACT/360, NY calendar, Modified Following
    /// - Float leg: Annual, ACT/360, NY calendar, Modified Following (SOFR
    ///   compounded)
    /// - Spot lag: 2 days
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Sofr,
            spot_lag: 2,
        }
    }

    /// Returns the EUR EURIBOR 6M swap convention.
    ///
    /// - Fixed leg: Annual, 30/360, TARGET calendar, Modified Following
    /// - Float leg: Semi-Annual, ACT/360, TARGET calendar, Modified Following
    /// - Spot lag: 2 days
    #[must_use]
    pub fn eur_euribor_6m() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCounter::Thirty360Bond,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::SemiAnnual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Euribor6M,
            spot_lag: 2,
        }
    }

    /// Returns the JPY TONAR swap convention.
    ///
    /// - Fixed leg: Annual, ACT/365, Tokyo calendar, Modified Following
    /// - Float leg: Annual, ACT/365, Tokyo calendar, Modified Following
    /// - Spot lag: 2 days
    #[must_use]
    pub fn jpy_tonar() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Tonar,
            spot_lag: 2,
        }
    }

    /// Returns the GBP SONIA swap convention.
    ///
    /// - Fixed leg: Annual, ACT/365, London calendar, Modified Following
    /// - Float leg: Annual, ACT/365, London calendar, Modified Following
    /// - Spot lag: 0 days (same day)
    #[must_use]
    pub fn gbp_sonia() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_leg: SwapLegConvention {
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 0,
            },
            float_index: RateIndex::Sonia,
            spot_lag: 0,
        }
    }

    /// Returns the EUR ESTR swap convention.
    ///
    /// - Fixed leg: Annual, ACT/360, TARGET calendar, Modified Following
    /// - Float leg: Annual, ACT/360, TARGET calendar, Modified Following (ESTR
    ///   compounded)
    /// - Spot lag: 2 days
    #[must_use]
    pub fn eur_estr() -> Self {
        Self {
            fixed_leg: SwapLegConvention {
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_leg: SwapLegConvention {
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Annual,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                payment_lag: 2,
            },
            float_index: RateIndex::Estr,
            spot_lag: 2,
        }
    }
}

// ============================================================================
// Bond Conventions
// ============================================================================

/// Convention for a bond.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BondConvention {
    /// Day count convention.
    pub day_count: DayCounter,
    /// Coupon payment frequency.
    pub coupon_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Number of settlement days.
    pub settlement_days: u32,
}

impl BondConvention {
    /// Creates a new bond convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        coupon_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        settlement_days: u32,
    ) -> Self {
        Self {
            day_count,
            coupon_frequency,
            calendar,
            business_day_convention,
            settlement_days,
        }
    }

    /// Returns the US Treasury bond convention.
    #[must_use]
    pub fn us_treasury() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 1,
        }
    }

    /// Returns the UK Gilt convention.
    #[must_use]
    pub fn uk_gilt() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 1,
        }
    }

    /// Returns the German Bund convention.
    #[must_use]
    pub fn german_bund() -> Self {
        Self {
            day_count: DayCounter::ActualActualIsda,
            coupon_frequency: Frequency::Annual,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 2,
        }
    }

    /// Returns the JGB (Japanese Government Bond) convention.
    #[must_use]
    pub fn jgb() -> Self {
        Self {
            day_count: DayCounter::Actual365Fixed,
            coupon_frequency: Frequency::SemiAnnual,
            calendar: CalendarId::Tokyo,
            business_day_convention: BusinessDayConvention::Following,
            settlement_days: 3,
        }
    }
}

// ============================================================================
// Cap/Floor Conventions
// ============================================================================

/// Convention for an interest rate cap or floor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapFloorConvention {
    /// Day count convention.
    pub day_count: DayCounter,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
}

impl CapFloorConvention {
    /// Creates a new cap/floor convention.
    #[must_use]
    pub fn new(
        day_count: DayCounter,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
    ) -> Self {
        Self {
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            index,
        }
    }

    /// Returns the USD SOFR cap/floor convention.
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Sofr,
        }
    }

    /// Returns the EUR EURIBOR 3M cap/floor convention.
    #[must_use]
    pub fn eur_euribor_3m() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Euribor3M,
        }
    }

    /// Returns the EUR ESTR cap/floor convention.
    ///
    /// - Day count: ACT/360
    /// - Payment frequency: Quarterly
    /// - Calendar: TARGET
    /// - Business day convention: Modified Following
    /// - Index: ESTR
    #[must_use]
    pub fn eur_estr() -> Self {
        Self {
            day_count: DayCounter::Actual360,
            payment_frequency: Frequency::Quarterly,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            index: RateIndex::Estr,
        }
    }
}

// ============================================================================
// Swaption Conventions
// ============================================================================

/// Settlement convention for swaption premium and exercise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SettlementConvention {
    /// Physical delivery (enter into underlying swap).
    Physical,
    /// Cash settlement (receive cash based on swap NPV).
    Cash,
}

/// Convention for a swaption.
///
/// Represents the market conventions for pricing and settling swaptions.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::{SwaptionConvention, SettlementConvention};
/// use infra_domain::market::Currency;
///
/// let conv = SwaptionConvention::usd_sofr();
/// assert_eq!(conv.premium_currency, Currency::USD);
/// assert_eq!(conv.exercise_settlement, SettlementConvention::Cash);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwaptionConvention {
    /// Convention for the underlying swap.
    pub underlying_swap: SwapConvention,
    /// Settlement convention for the premium payment.
    pub premium_settlement: SettlementConvention,
    /// Settlement convention for exercise.
    pub exercise_settlement: SettlementConvention,
    /// Currency for the premium.
    pub premium_currency: Currency,
    /// Number of business days from trade date to premium payment.
    pub premium_lag: u32,
    /// Number of business days from exercise to swap start.
    pub exercise_lag: u32,
}

impl SwaptionConvention {
    /// Creates a new swaption convention.
    #[must_use]
    pub fn new(
        underlying_swap: SwapConvention,
        premium_settlement: SettlementConvention,
        exercise_settlement: SettlementConvention,
        premium_currency: Currency,
        premium_lag: u32,
        exercise_lag: u32,
    ) -> Self {
        Self {
            underlying_swap,
            premium_settlement,
            exercise_settlement,
            premium_currency,
            premium_lag,
            exercise_lag,
        }
    }

    /// Returns the USD SOFR swaption convention.
    ///
    /// - Underlying: USD SOFR swap
    /// - Exercise settlement: Cash
    /// - Premium currency: USD
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn usd_sofr() -> Self {
        Self {
            underlying_swap: SwapConvention::usd_sofr(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::USD,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }

    /// Returns the EUR EURIBOR swaption convention.
    ///
    /// - Underlying: EUR EURIBOR 6M swap
    /// - Exercise settlement: Cash
    /// - Premium currency: EUR
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn eur_euribor() -> Self {
        Self {
            underlying_swap: SwapConvention::eur_euribor_6m(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::EUR,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }

    /// Returns the GBP SONIA swaption convention.
    ///
    /// - Underlying: GBP SONIA swap
    /// - Exercise settlement: Cash
    /// - Premium currency: GBP
    /// - Premium lag: 0 days
    /// - Exercise lag: 0 days
    #[must_use]
    pub fn gbp_sonia() -> Self {
        Self {
            underlying_swap: SwapConvention::gbp_sonia(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::GBP,
            premium_lag: 0,
            exercise_lag: 0,
        }
    }

    /// Returns the JPY TONAR swaption convention.
    ///
    /// - Underlying: JPY TONAR swap
    /// - Exercise settlement: Cash
    /// - Premium currency: JPY
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn jpy_tonar() -> Self {
        Self {
            underlying_swap: SwapConvention::jpy_tonar(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::JPY,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }

    /// Returns the EUR ESTR swaption convention.
    ///
    /// - Underlying: EUR ESTR swap
    /// - Exercise settlement: Cash
    /// - Premium currency: EUR
    /// - Premium lag: 2 days
    /// - Exercise lag: 2 days
    #[must_use]
    pub fn eur_estr() -> Self {
        Self {
            underlying_swap: SwapConvention::eur_estr(),
            premium_settlement: SettlementConvention::Cash,
            exercise_settlement: SettlementConvention::Cash,
            premium_currency: Currency::EUR,
            premium_lag: 2,
            exercise_lag: 2,
        }
    }
}

// ============================================================================
// Inflation Swap Conventions
// ============================================================================

/// Interpolation method for inflation index fixings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InflationInterpolation {
    /// Use the index value from the reference month (no interpolation).
    Flat,
    /// Linear interpolation between monthly values.
    Linear,
}

/// Inflation index type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InflationIndex {
    /// US Consumer Price Index (All Urban Consumers, not seasonally adjusted).
    UsCpi,
    /// UK Retail Price Index.
    UkRpi,
    /// Eurozone Harmonised Index of Consumer Prices (ex-Tobacco).
    EuHicp,
    /// French Consumer Price Index (ex-Tobacco).
    FrCpi,
    /// Custom inflation index.
    Custom(String),
}

impl InflationIndex {
    /// Returns the standard publication lag in months.
    #[must_use]
    pub fn publication_lag(&self) -> u32 {
        match self {
            InflationIndex::UsCpi => 2,
            InflationIndex::UkRpi => 1,
            InflationIndex::EuHicp => 2,
            InflationIndex::FrCpi => 2,
            InflationIndex::Custom(_) => 2,
        }
    }

    /// Returns the index code.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            InflationIndex::UsCpi => "CPURNSA",
            InflationIndex::UkRpi => "UKRPI",
            InflationIndex::EuHicp => "CPTFEMU",
            InflationIndex::FrCpi => "FRCPXTOB",
            InflationIndex::Custom(code) => code,
        }
    }
}

impl std::fmt::Display for InflationIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Convention for inflation swaps.
///
/// Represents the market conventions for pricing and settling inflation swaps.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::{
///     InflationSwapConvention, InflationIndex, InflationInterpolation,
/// };
///
/// let conv = InflationSwapConvention::us_cpi_zc();
/// assert_eq!(conv.inflation_index, InflationIndex::UsCpi);
/// assert_eq!(conv.lag_months, 3);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationSwapConvention {
    /// Inflation index used for the swap.
    pub inflation_index: InflationIndex,
    /// Observation lag in months.
    pub lag_months: u32,
    /// Interpolation method for index fixings.
    pub interpolation: InflationInterpolation,
    /// Day count convention for fixed leg.
    pub fixed_day_count: DayCounter,
    /// Payment frequency for fixed leg.
    pub fixed_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Number of spot days.
    pub spot_lag: u32,
}

impl InflationSwapConvention {
    /// Creates a new inflation swap convention.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inflation_index: InflationIndex,
        lag_months: u32,
        interpolation: InflationInterpolation,
        fixed_day_count: DayCounter,
        fixed_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        spot_lag: u32,
    ) -> Self {
        Self {
            inflation_index,
            lag_months,
            interpolation,
            fixed_day_count,
            fixed_frequency,
            calendar,
            business_day_convention,
            spot_lag,
        }
    }

    /// Returns the US CPI zero-coupon inflation swap convention.
    ///
    /// - Index: US CPI (NSA)
    /// - Lag: 3 months
    /// - Interpolation: Linear
    /// - Fixed leg: ACT/ACT, Annual
    #[must_use]
    pub fn us_cpi_zc() -> Self {
        Self {
            inflation_index: InflationIndex::UsCpi,
            lag_months: 3,
            interpolation: InflationInterpolation::Linear,
            fixed_day_count: DayCounter::ActualActualIsda,
            fixed_frequency: Frequency::Annual,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the US CPI year-on-year inflation swap convention.
    ///
    /// - Index: US CPI (NSA)
    /// - Lag: 3 months
    /// - Interpolation: Linear
    /// - Fixed leg: ACT/ACT, Annual
    #[must_use]
    pub fn us_cpi_yoy() -> Self {
        Self {
            inflation_index: InflationIndex::UsCpi,
            lag_months: 3,
            interpolation: InflationInterpolation::Linear,
            fixed_day_count: DayCounter::ActualActualIsda,
            fixed_frequency: Frequency::Annual,
            calendar: CalendarId::NewYork,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the UK RPI zero-coupon inflation swap convention.
    ///
    /// - Index: UK RPI
    /// - Lag: 2 months
    /// - Interpolation: Linear
    /// - Fixed leg: ACT/ACT, Annual
    #[must_use]
    pub fn uk_rpi_zc() -> Self {
        Self {
            inflation_index: InflationIndex::UkRpi,
            lag_months: 2,
            interpolation: InflationInterpolation::Linear,
            fixed_day_count: DayCounter::ActualActualIsda,
            fixed_frequency: Frequency::Annual,
            calendar: CalendarId::London,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 0,
        }
    }

    /// Returns the EUR HICP zero-coupon inflation swap convention.
    ///
    /// - Index: EUR HICP (ex-Tobacco)
    /// - Lag: 3 months
    /// - Interpolation: Flat
    /// - Fixed leg: ACT/ACT, Annual
    #[must_use]
    pub fn eur_hicp_zc() -> Self {
        Self {
            inflation_index: InflationIndex::EuHicp,
            lag_months: 3,
            interpolation: InflationInterpolation::Flat,
            fixed_day_count: DayCounter::ActualActualIsda,
            fixed_frequency: Frequency::Annual,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }

    /// Returns the French CPI zero-coupon inflation swap convention.
    ///
    /// - Index: French CPI (ex-Tobacco)
    /// - Lag: 3 months
    /// - Interpolation: Linear
    /// - Fixed leg: ACT/ACT, Annual
    #[must_use]
    pub fn fr_cpi_zc() -> Self {
        Self {
            inflation_index: InflationIndex::FrCpi,
            lag_months: 3,
            interpolation: InflationInterpolation::Linear,
            fixed_day_count: DayCounter::ActualActualIsda,
            fixed_frequency: Frequency::Annual,
            calendar: CalendarId::Target,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            spot_lag: 2,
        }
    }
}

// ============================================================================
// Cross-Currency Basis Swap Conventions
// ============================================================================

/// Convention for a leg of a cross-currency basis swap.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XCcyLegConvention {
    /// Currency of this leg.
    pub currency: Currency,
    /// Day count convention.
    pub day_count: DayCounter,
    /// Payment frequency.
    pub payment_frequency: Frequency,
    /// Calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Business day convention.
    pub business_day_convention: BusinessDayConvention,
    /// Reference rate index.
    pub index: RateIndex,
    /// Number of days between end of accrual and payment.
    pub payment_lag: u32,
}

impl XCcyLegConvention {
    /// Creates a new cross-currency leg convention.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        currency: Currency,
        day_count: DayCounter,
        payment_frequency: Frequency,
        calendar: CalendarId,
        business_day_convention: BusinessDayConvention,
        index: RateIndex,
        payment_lag: u32,
    ) -> Self {
        Self {
            currency,
            day_count,
            payment_frequency,
            calendar,
            business_day_convention,
            index,
            payment_lag,
        }
    }
}

/// Specifies which leg receives the basis spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BasisSpreadLeg {
    /// Basis spread is on the base currency leg.
    Base,
    /// Basis spread is on the quote currency leg.
    Quote,
}

/// Convention for a cross-currency basis swap.
///
/// Represents the market conventions for pricing and settling cross-currency
/// basis swaps where two floating rate legs in different currencies are
/// exchanged.
///
/// # Example
///
/// ```rust
/// use infra_domain::market::convention::XCcyBasisConvention;
///
/// let conv = XCcyBasisConvention::usd_jpy();
/// assert_eq!(conv.spot_lag, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XCcyBasisConvention {
    /// Convention for the base currency leg (first in pair).
    pub base_leg: XCcyLegConvention,
    /// Convention for the quote currency leg (second in pair).
    pub quote_leg: XCcyLegConvention,
    /// Which leg receives the basis spread.
    pub spread_on: BasisSpreadLeg,
    /// Number of spot days from trade date to start date.
    pub spot_lag: u32,
    /// Whether notionals are exchanged at inception and maturity.
    pub exchange_notional: bool,
}

impl XCcyBasisConvention {
    /// Creates a new cross-currency basis swap convention.
    #[must_use]
    pub fn new(
        base_leg: XCcyLegConvention,
        quote_leg: XCcyLegConvention,
        spread_on: BasisSpreadLeg,
        spot_lag: u32,
        exchange_notional: bool,
    ) -> Self {
        Self {
            base_leg,
            quote_leg,
            spread_on,
            spot_lag,
            exchange_notional,
        }
    }

    /// Returns the USD/JPY cross-currency basis swap convention.
    ///
    /// - Base leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Quote leg (JPY): TONAR, Quarterly, ACT/365, Tokyo calendar
    /// - Spread on: JPY leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn usd_jpy() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::JPY,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Tonar,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Quote,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the EUR/USD cross-currency basis swap convention.
    ///
    /// - Base leg (EUR): ESTR, Quarterly, ACT/360, TARGET calendar
    /// - Quote leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Spread on: EUR leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn eur_usd() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::EUR,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Estr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the GBP/USD cross-currency basis swap convention.
    ///
    /// - Base leg (GBP): SONIA, Quarterly, ACT/365, London calendar
    /// - Quote leg (USD): SOFR, Quarterly, ACT/360, NY calendar
    /// - Spread on: GBP leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn gbp_usd() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::GBP,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::London,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sonia,
                payment_lag: 0,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::USD,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::NewYork,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Sofr,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the EUR/JPY cross-currency basis swap convention.
    ///
    /// - Base leg (EUR): ESTR, Quarterly, ACT/360, TARGET calendar
    /// - Quote leg (JPY): TONAR, Quarterly, ACT/365, Tokyo calendar
    /// - Spread on: EUR leg
    /// - Spot lag: 2 days
    /// - Notional exchange: Yes
    #[must_use]
    pub fn eur_jpy() -> Self {
        Self {
            base_leg: XCcyLegConvention {
                currency: Currency::EUR,
                day_count: DayCounter::Actual360,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Target,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Estr,
                payment_lag: 2,
            },
            quote_leg: XCcyLegConvention {
                currency: Currency::JPY,
                day_count: DayCounter::Actual365Fixed,
                payment_frequency: Frequency::Quarterly,
                calendar: CalendarId::Tokyo,
                business_day_convention: BusinessDayConvention::ModifiedFollowing,
                index: RateIndex::Tonar,
                payment_lag: 2,
            },
            spread_on: BasisSpreadLeg::Base,
            spot_lag: 2,
            exchange_notional: true,
        }
    }

    /// Returns the base currency of this swap.
    #[must_use]
    pub fn base_currency(&self) -> Currency { self.base_leg.currency }

    /// Returns the quote currency of this swap.
    #[must_use]
    pub fn quote_currency(&self) -> Currency { self.quote_leg.currency }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_conventions() {
        let usd = DepositConvention::usd();
        assert_eq!(usd.day_count, DayCounter::Actual360);
        assert_eq!(usd.calendar, CalendarId::NewYork);
        assert_eq!(usd.spot_lag, 2);

        let gbp = DepositConvention::gbp();
        assert_eq!(gbp.day_count, DayCounter::Actual365Fixed);
        assert_eq!(gbp.spot_lag, 0);
    }

    #[test]
    fn test_swap_conventions() {
        let usd = SwapConvention::usd_sofr();
        assert_eq!(usd.float_index, RateIndex::Sofr);
        assert_eq!(usd.spot_lag, 2);
        assert_eq!(usd.fixed_leg.day_count, DayCounter::Actual360);

        let gbp = SwapConvention::gbp_sonia();
        assert_eq!(gbp.float_index, RateIndex::Sonia);
        assert_eq!(gbp.spot_lag, 0);
    }

    #[test]
    fn test_fra_futures_bond_capfloor_swaption() {
        let fra = FraConvention::usd_sofr();
        assert_eq!(fra.index, RateIndex::Sofr);

        let fut = FuturesConvention::cme_sofr();
        assert_eq!(fut.contract_size, 1_000_000.0);

        let bond = BondConvention::us_treasury();
        assert_eq!(bond.coupon_frequency, Frequency::SemiAnnual);

        let cap = CapFloorConvention::usd_sofr();
        assert_eq!(cap.index, RateIndex::Sofr);

        let swn = SwaptionConvention::usd_sofr();
        assert_eq!(swn.premium_currency, Currency::USD);
        assert_eq!(swn.exercise_settlement, SettlementConvention::Cash);
    }

    #[test]
    fn test_inflation_and_xccy() {
        let infl = InflationSwapConvention::us_cpi_zc();
        assert_eq!(infl.inflation_index, InflationIndex::UsCpi);
        assert_eq!(infl.lag_months, 3);
        assert_eq!(InflationIndex::UsCpi.code(), "CPURNSA");
        assert_eq!(InflationIndex::UkRpi.code(), "UKRPI");

        let xccy = XCcyBasisConvention::usd_jpy();
        assert_eq!(xccy.base_currency(), Currency::USD);
        assert_eq!(xccy.quote_currency(), Currency::JPY);
        assert_eq!(xccy.spread_on, BasisSpreadLeg::Quote);
        assert!(xccy.exchange_notional);

        let eur_usd = XCcyBasisConvention::eur_usd();
        assert_eq!(eur_usd.spread_on, BasisSpreadLeg::Base);
    }
}

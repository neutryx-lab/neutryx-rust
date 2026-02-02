//! Inflation swap convention definitions.
//!
//! This module provides types for representing inflation swap market
//! conventions.

use crate::time::{BusinessDayConvention, CalendarId, DayCounter, Frequency};

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
/// use infra_master::market::convention::{
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inflation_index_publication_lag() {
        assert_eq!(InflationIndex::UsCpi.publication_lag(), 2);
        assert_eq!(InflationIndex::UkRpi.publication_lag(), 1);
        assert_eq!(InflationIndex::EuHicp.publication_lag(), 2);
    }

    #[test]
    fn test_inflation_index_code() {
        assert_eq!(InflationIndex::UsCpi.code(), "CPURNSA");
        assert_eq!(InflationIndex::UkRpi.code(), "UKRPI");
        assert_eq!(
            InflationIndex::Custom("CUSTOM".to_string()).code(),
            "CUSTOM"
        );
    }

    #[test]
    fn test_inflation_index_display() {
        assert_eq!(InflationIndex::UsCpi.to_string(), "CPURNSA");
    }

    #[test]
    fn test_inflation_swap_convention_new() {
        let conv = InflationSwapConvention::new(
            InflationIndex::UsCpi,
            3,
            InflationInterpolation::Linear,
            DayCounter::ActualActualIsda,
            Frequency::Annual,
            CalendarId::NewYork,
            BusinessDayConvention::ModifiedFollowing,
            2,
        );

        assert_eq!(conv.inflation_index, InflationIndex::UsCpi);
        assert_eq!(conv.lag_months, 3);
        assert_eq!(conv.interpolation, InflationInterpolation::Linear);
    }

    #[test]
    fn test_us_cpi_zc_convention() {
        let conv = InflationSwapConvention::us_cpi_zc();

        assert_eq!(conv.inflation_index, InflationIndex::UsCpi);
        assert_eq!(conv.lag_months, 3);
        assert_eq!(conv.interpolation, InflationInterpolation::Linear);
        assert_eq!(conv.calendar, CalendarId::NewYork);
    }

    #[test]
    fn test_us_cpi_yoy_convention() {
        let conv = InflationSwapConvention::us_cpi_yoy();

        assert_eq!(conv.inflation_index, InflationIndex::UsCpi);
        assert_eq!(conv.fixed_frequency, Frequency::Annual);
    }

    #[test]
    fn test_uk_rpi_zc_convention() {
        let conv = InflationSwapConvention::uk_rpi_zc();

        assert_eq!(conv.inflation_index, InflationIndex::UkRpi);
        assert_eq!(conv.lag_months, 2);
        assert_eq!(conv.calendar, CalendarId::London);
        assert_eq!(conv.spot_lag, 0);
    }

    #[test]
    fn test_eur_hicp_zc_convention() {
        let conv = InflationSwapConvention::eur_hicp_zc();

        assert_eq!(conv.inflation_index, InflationIndex::EuHicp);
        assert_eq!(conv.interpolation, InflationInterpolation::Flat);
        assert_eq!(conv.calendar, CalendarId::Target);
    }

    #[test]
    fn test_fr_cpi_zc_convention() {
        let conv = InflationSwapConvention::fr_cpi_zc();

        assert_eq!(conv.inflation_index, InflationIndex::FrCpi);
        assert_eq!(conv.interpolation, InflationInterpolation::Linear);
    }

    #[test]
    fn test_inflation_interpolation_equality() {
        assert_eq!(InflationInterpolation::Flat, InflationInterpolation::Flat);
        assert_ne!(InflationInterpolation::Flat, InflationInterpolation::Linear);
    }

    #[test]
    fn test_inflation_index_equality() {
        assert_eq!(InflationIndex::UsCpi, InflationIndex::UsCpi);
        assert_ne!(InflationIndex::UsCpi, InflationIndex::UkRpi);
    }

    #[test]
    fn test_inflation_swap_convention_clone() {
        let conv = InflationSwapConvention::us_cpi_zc();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}

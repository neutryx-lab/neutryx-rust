//! Cross-currency basis swap instrument definitions.
//!
//! This module provides definitions for cross-currency basis swaps (XCCY),
//! used for constructing medium to long-term FX forward curves.

use crate::{Currency, Date, Frequency, RateIndex};

// ============================================================================
// BasisSpread Newtype
// ============================================================================

/// Basis spread in basis points for XCCY swaps.
///
/// The basis spread is quoted in basis points (bps) and applied to one leg
/// of the cross-currency swap (typically the foreign leg).
///
/// # Example
///
/// ```rust
/// use infra_master::trade::instrument_def::BasisSpread;
///
/// // -15 bps basis spread
/// let spread = BasisSpread::from_bps(-15.0);
/// assert!((spread.as_decimal() - (-0.0015)).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasisSpread(f64);

impl BasisSpread {
    /// Creates a basis spread from basis points value.
    #[must_use]
    pub fn from_bps(bps: f64) -> Self { Self(bps) }

    /// Creates a zero basis spread.
    #[must_use]
    pub fn zero() -> Self { Self(0.0) }

    /// Returns the basis spread in basis points.
    #[inline]
    #[must_use]
    pub fn bps(&self) -> f64 { self.0 }

    /// Returns the basis spread as a decimal (bps / 10000).
    #[inline]
    #[must_use]
    pub fn as_decimal(&self) -> f64 { self.0 / 10000.0 }
}

impl Default for BasisSpread {
    fn default() -> Self { Self::zero() }
}

impl std::fmt::Display for BasisSpread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} bps", self.0)
    }
}

// ============================================================================
// XCCY Swap Components
// ============================================================================

/// Notional exchange type for XCCY swaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NotionalExchange {
    /// Exchange notionals at trade inception only.
    Initial,
    /// Exchange notionals at maturity only.
    Final,
    /// Exchange notionals at both inception and maturity.
    Both,
    /// No notional exchange.
    None,
}

impl Default for NotionalExchange {
    fn default() -> Self { Self::Both }
}

/// Indicates which leg receives the basis spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpreadLeg {
    /// Spread applied to domestic leg.
    Domestic,
    /// Spread applied to foreign leg (standard).
    Foreign,
}

impl Default for SpreadLeg {
    fn default() -> Self { Self::Foreign }
}

/// Cross-currency swap leg details.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XccyLeg {
    /// Currency for this leg.
    pub currency: Currency,
    /// Floating rate index (e.g., SOFR, EURIBOR).
    pub rate_index: RateIndex,
    /// Payment frequency.
    pub payment_frequency: Frequency,
}

impl XccyLeg {
    /// Creates a new XCCY leg.
    #[must_use]
    pub fn new(currency: Currency, rate_index: RateIndex, payment_frequency: Frequency) -> Self {
        Self {
            currency,
            rate_index,
            payment_frequency,
        }
    }
}

/// Cross-currency basis swap convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XccyBasisConvention {
    /// Notional exchange type.
    pub notional_exchange: NotionalExchange,
    /// Mark-to-market (resettable) flag.
    pub mark_to_market: bool,
    /// Which leg receives the basis spread.
    pub spread_leg: SpreadLeg,
}

impl Default for XccyBasisConvention {
    fn default() -> Self {
        Self {
            notional_exchange: NotionalExchange::Both,
            mark_to_market: false,
            spread_leg: SpreadLeg::Foreign,
        }
    }
}

impl XccyBasisConvention {
    /// Standard non-MTM convention with basis on foreign leg.
    #[must_use]
    pub fn standard() -> Self { Self::default() }

    /// Resettable (mark-to-market) convention.
    #[must_use]
    pub fn resettable() -> Self {
        Self {
            notional_exchange: NotionalExchange::Both,
            mark_to_market: true,
            spread_leg: SpreadLeg::Foreign,
        }
    }
}

/// Standard XCCY swap tenors for long-term curve construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum XccyTenor {
    /// 2 Years.
    Y2,
    /// 3 Years.
    Y3,
    /// 4 Years.
    Y4,
    /// 5 Years.
    Y5,
    /// 7 Years.
    Y7,
    /// 10 Years.
    Y10,
    /// 15 Years.
    Y15,
    /// 20 Years.
    Y20,
    /// 25 Years.
    Y25,
    /// 30 Years.
    Y30,
}

impl XccyTenor {
    /// Returns the tenor in years.
    #[must_use]
    pub fn years(&self) -> u32 {
        match self {
            Self::Y2 => 2,
            Self::Y3 => 3,
            Self::Y4 => 4,
            Self::Y5 => 5,
            Self::Y7 => 7,
            Self::Y10 => 10,
            Self::Y15 => 15,
            Self::Y20 => 20,
            Self::Y25 => 25,
            Self::Y30 => 30,
        }
    }

    /// Returns the tenor name as a string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Y2 => "2Y",
            Self::Y3 => "3Y",
            Self::Y4 => "4Y",
            Self::Y5 => "5Y",
            Self::Y7 => "7Y",
            Self::Y10 => "10Y",
            Self::Y15 => "15Y",
            Self::Y20 => "20Y",
            Self::Y25 => "25Y",
            Self::Y30 => "30Y",
        }
    }
}

impl std::fmt::Display for XccyTenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// CrossCurrencyBasisSwap
// ============================================================================

/// Cross-currency basis swap instrument.
///
/// A XCCY basis swap exchanges floating rate payments in two different
/// currencies, with the basis spread applied to one leg. Used for
/// constructing long-term FX forward curves (2Y-30Y).
///
/// # Example
///
/// ```rust
/// use infra_master::trade::instrument_def::{
///     CrossCurrencyBasisSwap, BasisSpread, XccyLeg, XccyBasisConvention,
/// };
/// use infra_master::{Currency, Date, Frequency, RateIndex};
///
/// let xccy = CrossCurrencyBasisSwap {
///     domestic_currency: Currency::USD,
///     foreign_currency: Currency::EUR,
///     notional: 10_000_000.0,
///     start_date: Date::from_ymd(2025, 1, 15).unwrap(),
///     maturity: Date::from_ymd(2030, 1, 15).unwrap(),
///     domestic_leg: XccyLeg::new(Currency::USD, RateIndex::Sofr, Frequency::Quarterly),
///     foreign_leg: XccyLeg::new(Currency::EUR, RateIndex::Euribor3M, Frequency::Quarterly),
///     basis_spread: BasisSpread::from_bps(-15.0),
///     convention: XccyBasisConvention::default(),
/// };
///
/// assert!(xccy.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrossCurrencyBasisSwap {
    /// Domestic currency (typically USD).
    pub domestic_currency: Currency,
    /// Foreign currency.
    pub foreign_currency: Currency,
    /// Notional amount (in domestic currency).
    pub notional: f64,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Domestic leg details.
    pub domestic_leg: XccyLeg,
    /// Foreign leg details.
    pub foreign_leg: XccyLeg,
    /// Basis spread.
    pub basis_spread: BasisSpread,
    /// Market convention.
    pub convention: XccyBasisConvention,
}

impl CrossCurrencyBasisSwap {
    /// Validates the XCCY swap parameters.
    pub fn validate(&self) -> Result<(), XccySwapError> {
        // Validate notional
        if self.notional <= 0.0 {
            return Err(XccySwapError::InvalidNotional(self.notional));
        }

        // Validate dates
        if self.maturity <= self.start_date {
            return Err(XccySwapError::InvalidDates {
                start: self.start_date,
                maturity: self.maturity,
            });
        }

        // Validate leg currencies match
        if self.domestic_leg.currency != self.domestic_currency {
            return Err(XccySwapError::CurrencyMismatch {
                leg: "domestic".to_string(),
                expected: self.domestic_currency,
                actual: self.domestic_leg.currency,
            });
        }

        if self.foreign_leg.currency != self.foreign_currency {
            return Err(XccySwapError::CurrencyMismatch {
                leg: "foreign".to_string(),
                expected: self.foreign_currency,
                actual: self.foreign_leg.currency,
            });
        }

        // Validate currencies are different
        if self.domestic_currency == self.foreign_currency {
            return Err(XccySwapError::SameCurrency(self.domestic_currency));
        }

        Ok(())
    }

    /// Returns the tenor in years (approximate).
    #[must_use]
    pub fn tenor_years(&self) -> f64 {
        let days = self.maturity - self.start_date;
        days as f64 / 365.25
    }
}

impl std::fmt::Display for CrossCurrencyBasisSwap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} XCCY {:.0}Y @ {}",
            self.domestic_currency.code(),
            self.foreign_currency.code(),
            self.tenor_years(),
            self.basis_spread
        )
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors specific to XCCY swap operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum XccySwapError {
    /// Invalid notional amount.
    #[error("Invalid notional: {0} (must be positive)")]
    InvalidNotional(f64),

    /// Invalid dates (maturity <= start).
    #[error("Invalid dates: maturity {maturity} must be after start {start}")]
    InvalidDates {
        /// Start date.
        start: Date,
        /// Maturity date.
        maturity: Date,
    },

    /// Currency mismatch between leg and swap definition.
    #[error("Currency mismatch on {leg} leg: expected {expected}, got {actual}")]
    CurrencyMismatch {
        /// Leg identifier.
        leg: String,
        /// Expected currency.
        expected: Currency,
        /// Actual currency.
        actual: Currency,
    },

    /// Same currency on both legs.
    #[error("Both legs have the same currency: {0}")]
    SameCurrency(Currency),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === BasisSpread Tests ===

    #[test]
    fn test_basis_spread_from_bps() {
        let spread = BasisSpread::from_bps(-15.0);
        assert!((spread.bps() - (-15.0)).abs() < 1e-10);
    }

    #[test]
    fn test_basis_spread_as_decimal() {
        let spread = BasisSpread::from_bps(-15.0);
        // -15 bps = -0.0015
        assert!((spread.as_decimal() - (-0.0015)).abs() < 1e-10);
    }

    #[test]
    fn test_basis_spread_zero() {
        let spread = BasisSpread::zero();
        assert!((spread.bps()).abs() < 1e-10);
    }

    #[test]
    fn test_basis_spread_display() {
        let spread = BasisSpread::from_bps(-15.0);
        assert_eq!(spread.to_string(), "-15.0 bps");
    }

    #[test]
    fn test_basis_spread_default() {
        let spread = BasisSpread::default();
        assert!((spread.bps()).abs() < 1e-10);
    }

    // === XccyTenor Tests ===

    #[test]
    fn test_xccy_tenor_years() {
        assert_eq!(XccyTenor::Y2.years(), 2);
        assert_eq!(XccyTenor::Y5.years(), 5);
        assert_eq!(XccyTenor::Y10.years(), 10);
        assert_eq!(XccyTenor::Y30.years(), 30);
    }

    #[test]
    fn test_xccy_tenor_name() {
        assert_eq!(XccyTenor::Y2.name(), "2Y");
        assert_eq!(XccyTenor::Y10.name(), "10Y");
    }

    #[test]
    fn test_xccy_tenor_display() {
        assert_eq!(XccyTenor::Y5.to_string(), "5Y");
    }

    // === XccyBasisConvention Tests ===

    #[test]
    fn test_xccy_convention_default() {
        let conv = XccyBasisConvention::default();
        assert_eq!(conv.notional_exchange, NotionalExchange::Both);
        assert!(!conv.mark_to_market);
        assert_eq!(conv.spread_leg, SpreadLeg::Foreign);
    }

    #[test]
    fn test_xccy_convention_resettable() {
        let conv = XccyBasisConvention::resettable();
        assert!(conv.mark_to_market);
    }

    // === CrossCurrencyBasisSwap Tests ===

    fn make_test_xccy() -> CrossCurrencyBasisSwap {
        CrossCurrencyBasisSwap {
            domestic_currency: Currency::USD,
            foreign_currency: Currency::EUR,
            notional: 10_000_000.0,
            start_date: Date::from_ymd(2025, 1, 15).unwrap(),
            maturity: Date::from_ymd(2030, 1, 15).unwrap(),
            domestic_leg: XccyLeg::new(Currency::USD, RateIndex::Sofr, Frequency::Quarterly),
            foreign_leg: XccyLeg::new(Currency::EUR, RateIndex::Euribor3M, Frequency::Quarterly),
            basis_spread: BasisSpread::from_bps(-15.0),
            convention: XccyBasisConvention::default(),
        }
    }

    #[test]
    fn test_xccy_validate_success() {
        let xccy = make_test_xccy();
        assert!(xccy.validate().is_ok());
    }

    #[test]
    fn test_xccy_validate_invalid_notional() {
        let mut xccy = make_test_xccy();
        xccy.notional = -1_000_000.0;
        let result = xccy.validate();
        assert!(matches!(result, Err(XccySwapError::InvalidNotional(_))));
    }

    #[test]
    fn test_xccy_validate_invalid_dates() {
        let mut xccy = make_test_xccy();
        xccy.maturity = Date::from_ymd(2024, 1, 15).unwrap(); // before start
        let result = xccy.validate();
        assert!(matches!(result, Err(XccySwapError::InvalidDates { .. })));
    }

    #[test]
    fn test_xccy_validate_currency_mismatch_domestic() {
        let mut xccy = make_test_xccy();
        xccy.domestic_leg.currency = Currency::GBP; // mismatch
        let result = xccy.validate();
        assert!(matches!(
            result,
            Err(XccySwapError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn test_xccy_validate_currency_mismatch_foreign() {
        let mut xccy = make_test_xccy();
        xccy.foreign_leg.currency = Currency::GBP; // mismatch
        let result = xccy.validate();
        assert!(matches!(
            result,
            Err(XccySwapError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn test_xccy_validate_same_currency() {
        let mut xccy = make_test_xccy();
        xccy.foreign_currency = Currency::USD;
        xccy.foreign_leg.currency = Currency::USD;
        let result = xccy.validate();
        assert!(matches!(result, Err(XccySwapError::SameCurrency(_))));
    }

    #[test]
    fn test_xccy_swap_tenor_years() {
        let xccy = make_test_xccy();
        let tenor = xccy.tenor_years();
        // Approximately 5 years
        assert!((tenor - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_xccy_display() {
        let xccy = make_test_xccy();
        let display = xccy.to_string();
        assert!(display.contains("USD/EUR"));
        assert!(display.contains("XCCY"));
        assert!(display.contains("-15.0 bps"));
    }
}

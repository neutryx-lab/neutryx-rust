//! Curve definition types for declarative curve construction.
//!
//! This module provides types for defining curves in a declarative manner,
//! mapping indices to instrument specifications for bootstrapping.
//!
//! # Architecture
//!
//! - `CurveInstrumentType`: Enum of instrument types used in curve construction
//! - `InstrumentTenor`: Standard tenors for curve instruments (1M to 50Y)
//! - `InstrumentSpec`: Specification for a single instrument in curve construction
//! - `CurveDefinition`: Index-to-instrument mapping for curve construction

use std::fmt;
#[cfg(feature = "serde")]
use std::path::Path;
use std::str::FromStr;

use infra_master::market::RateIndex;
use infra_master::trade::convention::SwapConvention;

use super::engine_error::CurveEngineError;

/// Instrument types used in curve construction.
///
/// Each type represents a different instrument category that can be
/// used as input for yield curve bootstrapping.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::CurveInstrumentType;
///
/// let inst_type = CurveInstrumentType::Ois;
/// assert_eq!(inst_type.code(), "OIS");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CurveInstrumentType {
    /// Overnight Index Swap - primary instrument for discount curve construction.
    #[default]
    Ois,
    /// Interest Rate Swap - used for tenor curve construction.
    Irs,
    /// Forward Rate Agreement - single-period forward contract.
    Fra,
    /// Interest Rate Future - exchange-traded rate futures.
    Future,
    /// Deposit - money market deposit rate.
    Deposit,
}

impl CurveInstrumentType {
    /// Returns the standard code for this instrument type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveInstrumentType;
    ///
    /// assert_eq!(CurveInstrumentType::Ois.code(), "OIS");
    /// assert_eq!(CurveInstrumentType::Irs.code(), "IRS");
    /// ```
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            CurveInstrumentType::Ois => "OIS",
            CurveInstrumentType::Irs => "IRS",
            CurveInstrumentType::Fra => "FRA",
            CurveInstrumentType::Future => "FUT",
            CurveInstrumentType::Deposit => "DEP",
        }
    }

    /// Returns a description of this instrument type.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            CurveInstrumentType::Ois => "Overnight Index Swap",
            CurveInstrumentType::Irs => "Interest Rate Swap",
            CurveInstrumentType::Fra => "Forward Rate Agreement",
            CurveInstrumentType::Future => "Interest Rate Future",
            CurveInstrumentType::Deposit => "Money Market Deposit",
        }
    }

    /// Returns all available instrument types.
    #[must_use]
    pub fn all() -> &'static [CurveInstrumentType] {
        &[
            CurveInstrumentType::Ois,
            CurveInstrumentType::Irs,
            CurveInstrumentType::Fra,
            CurveInstrumentType::Future,
            CurveInstrumentType::Deposit,
        ]
    }
}

impl fmt::Display for CurveInstrumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

impl FromStr for CurveInstrumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OIS" => Ok(CurveInstrumentType::Ois),
            "IRS" | "SWAP" => Ok(CurveInstrumentType::Irs),
            "FRA" => Ok(CurveInstrumentType::Fra),
            "FUT" | "FUTURE" | "FUTURES" => Ok(CurveInstrumentType::Future),
            "DEP" | "DEPOSIT" | "MM" => Ok(CurveInstrumentType::Deposit),
            _ => Err(format!("Unknown instrument type: {}", s)),
        }
    }
}

/// Standard tenors for curve construction instruments.
///
/// Covers the full range of tenors typically used in yield curve
/// bootstrapping, from overnight to 50 years.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::InstrumentTenor;
///
/// let tenor = InstrumentTenor::FiveYears;
/// assert_eq!(tenor.code(), "5Y");
/// assert!((tenor.to_years() - 5.0).abs() < 1e-10);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InstrumentTenor {
    /// Overnight (O/N)
    Overnight,
    /// One week (1W)
    OneWeek,
    /// Two weeks (2W)
    TwoWeeks,
    /// One month (1M)
    OneMonth,
    /// Two months (2M)
    TwoMonths,
    /// Three months (3M)
    ThreeMonths,
    /// Six months (6M)
    SixMonths,
    /// Nine months (9M)
    NineMonths,
    /// One year (1Y)
    #[default]
    OneYear,
    /// Eighteen months (18M)
    EighteenMonths,
    /// Two years (2Y)
    TwoYears,
    /// Three years (3Y)
    ThreeYears,
    /// Four years (4Y)
    FourYears,
    /// Five years (5Y)
    FiveYears,
    /// Six years (6Y)
    SixYears,
    /// Seven years (7Y)
    SevenYears,
    /// Eight years (8Y)
    EightYears,
    /// Nine years (9Y)
    NineYears,
    /// Ten years (10Y)
    TenYears,
    /// Twelve years (12Y)
    TwelveYears,
    /// Fifteen years (15Y)
    FifteenYears,
    /// Twenty years (20Y)
    TwentyYears,
    /// Twenty-five years (25Y)
    TwentyFiveYears,
    /// Thirty years (30Y)
    ThirtyYears,
    /// Forty years (40Y)
    FortyYears,
    /// Fifty years (50Y)
    FiftyYears,
}

impl InstrumentTenor {
    /// Returns the standard code for this tenor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::InstrumentTenor;
    ///
    /// assert_eq!(InstrumentTenor::Overnight.code(), "ON");
    /// assert_eq!(InstrumentTenor::ThreeMonths.code(), "3M");
    /// assert_eq!(InstrumentTenor::TenYears.code(), "10Y");
    /// ```
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            InstrumentTenor::Overnight => "ON",
            InstrumentTenor::OneWeek => "1W",
            InstrumentTenor::TwoWeeks => "2W",
            InstrumentTenor::OneMonth => "1M",
            InstrumentTenor::TwoMonths => "2M",
            InstrumentTenor::ThreeMonths => "3M",
            InstrumentTenor::SixMonths => "6M",
            InstrumentTenor::NineMonths => "9M",
            InstrumentTenor::OneYear => "1Y",
            InstrumentTenor::EighteenMonths => "18M",
            InstrumentTenor::TwoYears => "2Y",
            InstrumentTenor::ThreeYears => "3Y",
            InstrumentTenor::FourYears => "4Y",
            InstrumentTenor::FiveYears => "5Y",
            InstrumentTenor::SixYears => "6Y",
            InstrumentTenor::SevenYears => "7Y",
            InstrumentTenor::EightYears => "8Y",
            InstrumentTenor::NineYears => "9Y",
            InstrumentTenor::TenYears => "10Y",
            InstrumentTenor::TwelveYears => "12Y",
            InstrumentTenor::FifteenYears => "15Y",
            InstrumentTenor::TwentyYears => "20Y",
            InstrumentTenor::TwentyFiveYears => "25Y",
            InstrumentTenor::ThirtyYears => "30Y",
            InstrumentTenor::FortyYears => "40Y",
            InstrumentTenor::FiftyYears => "50Y",
        }
    }

    /// Returns the tenor in years as f64.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::InstrumentTenor;
    ///
    /// assert!((InstrumentTenor::Overnight.to_years() - 1.0/365.0).abs() < 1e-10);
    /// assert!((InstrumentTenor::ThreeMonths.to_years() - 0.25).abs() < 1e-10);
    /// assert!((InstrumentTenor::OneYear.to_years() - 1.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn to_years(&self) -> f64 {
        match self {
            InstrumentTenor::Overnight => 1.0 / 365.0,
            InstrumentTenor::OneWeek => 7.0 / 365.0,
            InstrumentTenor::TwoWeeks => 14.0 / 365.0,
            InstrumentTenor::OneMonth => 1.0 / 12.0,
            InstrumentTenor::TwoMonths => 2.0 / 12.0,
            InstrumentTenor::ThreeMonths => 0.25,
            InstrumentTenor::SixMonths => 0.5,
            InstrumentTenor::NineMonths => 0.75,
            InstrumentTenor::OneYear => 1.0,
            InstrumentTenor::EighteenMonths => 1.5,
            InstrumentTenor::TwoYears => 2.0,
            InstrumentTenor::ThreeYears => 3.0,
            InstrumentTenor::FourYears => 4.0,
            InstrumentTenor::FiveYears => 5.0,
            InstrumentTenor::SixYears => 6.0,
            InstrumentTenor::SevenYears => 7.0,
            InstrumentTenor::EightYears => 8.0,
            InstrumentTenor::NineYears => 9.0,
            InstrumentTenor::TenYears => 10.0,
            InstrumentTenor::TwelveYears => 12.0,
            InstrumentTenor::FifteenYears => 15.0,
            InstrumentTenor::TwentyYears => 20.0,
            InstrumentTenor::TwentyFiveYears => 25.0,
            InstrumentTenor::ThirtyYears => 30.0,
            InstrumentTenor::FortyYears => 40.0,
            InstrumentTenor::FiftyYears => 50.0,
        }
    }

    /// Returns the number of months for this tenor.
    ///
    /// For tenors shorter than a month, returns 0.
    #[must_use]
    pub fn to_months(&self) -> u32 {
        match self {
            InstrumentTenor::Overnight | InstrumentTenor::OneWeek | InstrumentTenor::TwoWeeks => 0,
            InstrumentTenor::OneMonth => 1,
            InstrumentTenor::TwoMonths => 2,
            InstrumentTenor::ThreeMonths => 3,
            InstrumentTenor::SixMonths => 6,
            InstrumentTenor::NineMonths => 9,
            InstrumentTenor::OneYear => 12,
            InstrumentTenor::EighteenMonths => 18,
            InstrumentTenor::TwoYears => 24,
            InstrumentTenor::ThreeYears => 36,
            InstrumentTenor::FourYears => 48,
            InstrumentTenor::FiveYears => 60,
            InstrumentTenor::SixYears => 72,
            InstrumentTenor::SevenYears => 84,
            InstrumentTenor::EightYears => 96,
            InstrumentTenor::NineYears => 108,
            InstrumentTenor::TenYears => 120,
            InstrumentTenor::TwelveYears => 144,
            InstrumentTenor::FifteenYears => 180,
            InstrumentTenor::TwentyYears => 240,
            InstrumentTenor::TwentyFiveYears => 300,
            InstrumentTenor::ThirtyYears => 360,
            InstrumentTenor::FortyYears => 480,
            InstrumentTenor::FiftyYears => 600,
        }
    }

    /// Returns all standard tenors in ascending order.
    #[must_use]
    pub fn all() -> &'static [InstrumentTenor] {
        &[
            InstrumentTenor::Overnight,
            InstrumentTenor::OneWeek,
            InstrumentTenor::TwoWeeks,
            InstrumentTenor::OneMonth,
            InstrumentTenor::TwoMonths,
            InstrumentTenor::ThreeMonths,
            InstrumentTenor::SixMonths,
            InstrumentTenor::NineMonths,
            InstrumentTenor::OneYear,
            InstrumentTenor::EighteenMonths,
            InstrumentTenor::TwoYears,
            InstrumentTenor::ThreeYears,
            InstrumentTenor::FourYears,
            InstrumentTenor::FiveYears,
            InstrumentTenor::SixYears,
            InstrumentTenor::SevenYears,
            InstrumentTenor::EightYears,
            InstrumentTenor::NineYears,
            InstrumentTenor::TenYears,
            InstrumentTenor::TwelveYears,
            InstrumentTenor::FifteenYears,
            InstrumentTenor::TwentyYears,
            InstrumentTenor::TwentyFiveYears,
            InstrumentTenor::ThirtyYears,
            InstrumentTenor::FortyYears,
            InstrumentTenor::FiftyYears,
        ]
    }

    /// Returns standard OIS tenors for curve construction.
    #[must_use]
    pub fn standard_ois_tenors() -> &'static [InstrumentTenor] {
        &[
            InstrumentTenor::OneMonth,
            InstrumentTenor::ThreeMonths,
            InstrumentTenor::SixMonths,
            InstrumentTenor::OneYear,
            InstrumentTenor::TwoYears,
            InstrumentTenor::ThreeYears,
            InstrumentTenor::FiveYears,
            InstrumentTenor::SevenYears,
            InstrumentTenor::TenYears,
            InstrumentTenor::FifteenYears,
            InstrumentTenor::TwentyYears,
            InstrumentTenor::ThirtyYears,
            InstrumentTenor::FiftyYears,
        ]
    }
}

impl fmt::Display for InstrumentTenor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.code()) }
}

impl FromStr for InstrumentTenor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ON" | "O/N" | "OVERNIGHT" => Ok(InstrumentTenor::Overnight),
            "1W" => Ok(InstrumentTenor::OneWeek),
            "2W" => Ok(InstrumentTenor::TwoWeeks),
            "1M" => Ok(InstrumentTenor::OneMonth),
            "2M" => Ok(InstrumentTenor::TwoMonths),
            "3M" => Ok(InstrumentTenor::ThreeMonths),
            "6M" => Ok(InstrumentTenor::SixMonths),
            "9M" => Ok(InstrumentTenor::NineMonths),
            "1Y" | "12M" => Ok(InstrumentTenor::OneYear),
            "18M" => Ok(InstrumentTenor::EighteenMonths),
            "2Y" | "24M" => Ok(InstrumentTenor::TwoYears),
            "3Y" | "36M" => Ok(InstrumentTenor::ThreeYears),
            "4Y" | "48M" => Ok(InstrumentTenor::FourYears),
            "5Y" | "60M" => Ok(InstrumentTenor::FiveYears),
            "6Y" | "72M" => Ok(InstrumentTenor::SixYears),
            "7Y" | "84M" => Ok(InstrumentTenor::SevenYears),
            "8Y" | "96M" => Ok(InstrumentTenor::EightYears),
            "9Y" | "108M" => Ok(InstrumentTenor::NineYears),
            "10Y" | "120M" => Ok(InstrumentTenor::TenYears),
            "12Y" | "144M" => Ok(InstrumentTenor::TwelveYears),
            "15Y" | "180M" => Ok(InstrumentTenor::FifteenYears),
            "20Y" | "240M" => Ok(InstrumentTenor::TwentyYears),
            "25Y" | "300M" => Ok(InstrumentTenor::TwentyFiveYears),
            "30Y" | "360M" => Ok(InstrumentTenor::ThirtyYears),
            "40Y" | "480M" => Ok(InstrumentTenor::FortyYears),
            "50Y" | "600M" => Ok(InstrumentTenor::FiftyYears),
            _ => Err(format!("Unknown tenor: {}", s)),
        }
    }
}

/// Specification for a single instrument used in curve construction.
///
/// Defines the instrument type, tenor, and optional parameters needed
/// to construct a bootstrap instrument from market data.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::{
///     InstrumentSpec, CurveInstrumentType, InstrumentTenor,
/// };
///
/// // OIS at 5Y tenor
/// let spec = InstrumentSpec::new(CurveInstrumentType::Ois, InstrumentTenor::FiveYears);
/// assert_eq!(spec.instrument_type(), CurveInstrumentType::Ois);
/// assert_eq!(spec.tenor(), InstrumentTenor::FiveYears);
///
/// // Future with convexity adjustment
/// let future_spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001);
/// assert!(future_spec.convexity_adjustment().is_some());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstrumentSpec {
    /// Instrument type (OIS, IRS, FRA, Future, Deposit).
    instrument_type: CurveInstrumentType,
    /// Tenor (maturity) of the instrument.
    tenor: InstrumentTenor,
    /// Optional convexity adjustment for futures.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    convexity_adjustment: Option<f64>,
}

impl InstrumentSpec {
    /// Creates a new instrument specification.
    ///
    /// # Arguments
    ///
    /// * `instrument_type` - The type of instrument
    /// * `tenor` - The tenor (maturity) of the instrument
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::{
    ///     InstrumentSpec, CurveInstrumentType, InstrumentTenor,
    /// };
    ///
    /// let spec = InstrumentSpec::new(CurveInstrumentType::Ois, InstrumentTenor::FiveYears);
    /// assert_eq!(spec.instrument_type(), CurveInstrumentType::Ois);
    /// ```
    #[must_use]
    pub fn new(instrument_type: CurveInstrumentType, tenor: InstrumentTenor) -> Self {
        Self {
            instrument_type,
            tenor,
            convexity_adjustment: None,
        }
    }

    /// Creates an OIS instrument specification.
    #[must_use]
    pub fn ois(tenor: InstrumentTenor) -> Self { Self::new(CurveInstrumentType::Ois, tenor) }

    /// Creates an IRS instrument specification.
    #[must_use]
    pub fn irs(tenor: InstrumentTenor) -> Self { Self::new(CurveInstrumentType::Irs, tenor) }

    /// Creates a FRA instrument specification.
    #[must_use]
    pub fn fra(tenor: InstrumentTenor) -> Self { Self::new(CurveInstrumentType::Fra, tenor) }

    /// Creates a deposit instrument specification.
    #[must_use]
    pub fn deposit(tenor: InstrumentTenor) -> Self {
        Self::new(CurveInstrumentType::Deposit, tenor)
    }

    /// Creates a future instrument specification with convexity adjustment.
    ///
    /// # Arguments
    ///
    /// * `tenor` - The tenor (maturity) of the future
    /// * `convexity_adjustment` - The convexity adjustment to apply
    #[must_use]
    pub fn future(tenor: InstrumentTenor, convexity_adjustment: f64) -> Self {
        Self {
            instrument_type: CurveInstrumentType::Future,
            tenor,
            convexity_adjustment: Some(convexity_adjustment),
        }
    }

    /// Creates an instrument specification with convexity adjustment.
    #[must_use]
    pub fn with_convexity_adjustment(mut self, adjustment: f64) -> Self {
        self.convexity_adjustment = Some(adjustment);
        self
    }

    /// Returns the instrument type.
    #[must_use]
    pub fn instrument_type(&self) -> CurveInstrumentType { self.instrument_type }

    /// Returns the tenor.
    #[must_use]
    pub fn tenor(&self) -> InstrumentTenor { self.tenor }

    /// Returns the convexity adjustment if set.
    #[must_use]
    pub fn convexity_adjustment(&self) -> Option<f64> { self.convexity_adjustment }

    /// Returns the tenor in years as f64.
    #[must_use]
    pub fn maturity_years(&self) -> f64 { self.tenor.to_years() }

    /// Validates the instrument specification.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or an error message describing the issue.
    pub fn validate(&self) -> Result<(), String> {
        // Convexity adjustment should only be set for futures
        if self.convexity_adjustment.is_some()
            && self.instrument_type != CurveInstrumentType::Future
        {
            return Err(format!(
                "Convexity adjustment is only valid for futures, not {:?}",
                self.instrument_type
            ));
        }

        // Futures typically have short tenors
        if self.instrument_type == CurveInstrumentType::Future
            && self.tenor.to_years() > 3.0
        {
            return Err(format!(
                "Future tenor {} is unusually long (typically <= 3Y)",
                self.tenor
            ));
        }

        // Deposits typically have very short tenors
        if self.instrument_type == CurveInstrumentType::Deposit
            && self.tenor.to_years() > 1.0
        {
            return Err(format!(
                "Deposit tenor {} is unusually long (typically <= 1Y)",
                self.tenor
            ));
        }

        Ok(())
    }
}

impl fmt::Display for InstrumentSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.instrument_type, self.tenor)?;
        if let Some(adj) = self.convexity_adjustment {
            write!(f, " (conv_adj: {:.6})", adj)?;
        }
        Ok(())
    }
}

/// Definition for constructing a yield curve from a set of instruments.
///
/// Maps a rate index to a collection of instrument specifications that
/// will be used to bootstrap the yield curve.
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::{
///     CurveDefinition, InstrumentSpec, InstrumentTenor,
/// };
/// use infra_master::market::RateIndex;
/// use infra_master::trade::convention::SwapConvention;
///
/// let definition = CurveDefinition::new(
///     "USD-SOFR",
///     RateIndex::Sofr,
///     SwapConvention::usd_sofr(),
/// )
/// .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
/// .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
/// .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
/// .with_instrument(InstrumentSpec::ois(InstrumentTenor::TenYears));
///
/// assert_eq!(definition.index_key(), "USD-SOFR");
/// assert_eq!(definition.rate_index(), RateIndex::Sofr);
/// assert_eq!(definition.instruments().len(), 4);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CurveDefinition {
    /// Curve identifier (e.g., "USD-SOFR", "EUR-ESTR").
    index_key: String,
    /// The rate index this curve is associated with.
    rate_index: RateIndex,
    /// List of instrument specifications for bootstrapping.
    instruments: Vec<InstrumentSpec>,
    /// Swap convention for instrument construction.
    convention: SwapConvention,
}

impl CurveDefinition {
    /// Creates a new curve definition.
    ///
    /// # Arguments
    ///
    /// * `index_key` - Unique identifier for the curve (e.g., "USD-SOFR")
    /// * `rate_index` - The rate index this curve represents
    /// * `convention` - Swap convention for instrument construction
    #[must_use]
    pub fn new(
        index_key: impl Into<String>,
        rate_index: RateIndex,
        convention: SwapConvention,
    ) -> Self {
        Self {
            index_key: index_key.into(),
            rate_index,
            instruments: Vec::new(),
            convention,
        }
    }

    /// Creates a curve definition with instruments.
    #[must_use]
    pub fn with_instruments(
        index_key: impl Into<String>,
        rate_index: RateIndex,
        convention: SwapConvention,
        instruments: Vec<InstrumentSpec>,
    ) -> Self {
        Self {
            index_key: index_key.into(),
            rate_index,
            instruments,
            convention,
        }
    }

    /// Adds an instrument specification to the curve definition.
    #[must_use]
    pub fn with_instrument(mut self, spec: InstrumentSpec) -> Self {
        self.instruments.push(spec);
        self
    }

    /// Adds multiple instrument specifications to the curve definition.
    #[must_use]
    pub fn with_instruments_iter(
        mut self,
        specs: impl IntoIterator<Item = InstrumentSpec>,
    ) -> Self {
        self.instruments.extend(specs);
        self
    }

    /// Returns the curve identifier.
    #[must_use]
    pub fn index_key(&self) -> &str { &self.index_key }

    /// Returns the rate index.
    #[must_use]
    pub fn rate_index(&self) -> RateIndex { self.rate_index }

    /// Returns the instrument specifications.
    #[must_use]
    pub fn instruments(&self) -> &[InstrumentSpec] { &self.instruments }

    /// Returns the swap convention.
    #[must_use]
    pub fn convention(&self) -> &SwapConvention { &self.convention }

    /// Returns instrument specifications sorted by tenor (ascending maturity).
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::{
    ///     CurveDefinition, InstrumentSpec, InstrumentTenor,
    /// };
    /// use infra_master::market::RateIndex;
    /// use infra_master::trade::convention::SwapConvention;
    ///
    /// let definition = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
    ///     .with_instrument(InstrumentSpec::ois(InstrumentTenor::TenYears))
    ///     .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
    ///     .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));
    ///
    /// let sorted = definition.sorted_instruments();
    /// assert_eq!(sorted[0].tenor(), InstrumentTenor::OneYear);
    /// assert_eq!(sorted[1].tenor(), InstrumentTenor::FiveYears);
    /// assert_eq!(sorted[2].tenor(), InstrumentTenor::TenYears);
    /// ```
    #[must_use]
    pub fn sorted_instruments(&self) -> Vec<&InstrumentSpec> {
        let mut sorted: Vec<_> = self.instruments.iter().collect();
        sorted.sort_by_key(|s| s.tenor);
        sorted
    }

    /// Returns instrument specifications sorted by tenor as owned values.
    #[must_use]
    pub fn sorted_instruments_owned(&self) -> Vec<InstrumentSpec> {
        let mut sorted = self.instruments.clone();
        sorted.sort_by_key(|s| s.tenor);
        sorted
    }

    /// Returns the number of instruments in this definition.
    #[must_use]
    pub fn len(&self) -> usize { self.instruments.len() }

    /// Returns true if there are no instruments.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.instruments.is_empty() }

    /// Validates the curve definition.
    ///
    /// # Validation Rules
    ///
    /// - Must have at least one instrument
    /// - All instruments must pass individual validation
    /// - Convention's float_index should match rate_index (warning only)
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or an error message describing the issue.
    pub fn validate(&self) -> Result<(), String> {
        // Must have at least one instrument
        if self.instruments.is_empty() {
            return Err("Curve definition must have at least one instrument".to_string());
        }

        // Validate each instrument
        for (i, spec) in self.instruments.iter().enumerate() {
            if let Err(e) = spec.validate() {
                return Err(format!("Instrument {} validation failed: {}", i, e));
            }
        }

        // Check convention consistency (warning-level, not error)
        if self.convention.float_index != self.rate_index {
            // This is allowed but unusual - convention might be intentionally different
            // for basis swaps or cross-currency curves
        }

        Ok(())
    }

    /// Creates a default SOFR OIS curve definition with standard tenors.
    ///
    /// Includes tenors: 1M, 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
    #[must_use]
    pub fn default_usd_sofr() -> Self {
        Self::with_instruments(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
            vec![
                InstrumentSpec::ois(InstrumentTenor::OneMonth),
                InstrumentSpec::ois(InstrumentTenor::ThreeMonths),
                InstrumentSpec::ois(InstrumentTenor::SixMonths),
                InstrumentSpec::ois(InstrumentTenor::OneYear),
                InstrumentSpec::ois(InstrumentTenor::TwoYears),
                InstrumentSpec::ois(InstrumentTenor::ThreeYears),
                InstrumentSpec::ois(InstrumentTenor::FiveYears),
                InstrumentSpec::ois(InstrumentTenor::SevenYears),
                InstrumentSpec::ois(InstrumentTenor::TenYears),
                InstrumentSpec::ois(InstrumentTenor::FifteenYears),
                InstrumentSpec::ois(InstrumentTenor::TwentyYears),
                InstrumentSpec::ois(InstrumentTenor::ThirtyYears),
            ],
        )
    }

    /// Gets a default curve definition for a known rate index.
    ///
    /// Returns `None` if no default definition exists for the given index.
    #[must_use]
    pub fn default_for_index(index: RateIndex) -> Option<Self> {
        match index {
            RateIndex::Sofr => Some(Self::default_usd_sofr()),
            // Add more defaults as needed
            _ => None,
        }
    }

    /// Loads a curve definition from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::Io` if the file cannot be read.
    /// Returns `CurveEngineError::ConfigurationParse` if the JSON is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pricer_models::market::calibration::bootstrapping::CurveDefinition;
    /// use std::path::Path;
    ///
    /// let definition = CurveDefinition::load_from_json(Path::new("config/usd-sofr.json"))?;
    /// ```
    #[cfg(feature = "serde")]
    pub fn load_from_json(path: &Path) -> Result<Self, CurveEngineError> {
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    /// Loads a curve definition from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string containing the curve definition
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::ConfigurationParse` if the JSON is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::CurveDefinition;
    ///
    /// let json = r#"{
    ///     "index_key": "USD-SOFR",
    ///     "rate_index": "Sofr",
    ///     "instruments": [
    ///         {"instrument_type": "Ois", "tenor": "OneYear"},
    ///         {"instrument_type": "Ois", "tenor": "FiveYears"}
    ///     ],
    ///     "convention": {
    ///         "fixed_leg": {
    ///             "day_count": "Actual360",
    ///             "payment_frequency": "Annual",
    ///             "calendar": "NewYork",
    ///             "business_day_convention": "ModifiedFollowing",
    ///             "payment_lag": 2
    ///         },
    ///         "float_leg": {
    ///             "day_count": "Actual360",
    ///             "payment_frequency": "Annual",
    ///             "calendar": "NewYork",
    ///             "business_day_convention": "ModifiedFollowing",
    ///             "payment_lag": 2
    ///         },
    ///         "float_index": "Sofr",
    ///         "spot_lag": 2
    ///     }
    /// }"#;
    ///
    /// let definition = CurveDefinition::load_from_str(json).unwrap();
    /// assert_eq!(definition.index_key(), "USD-SOFR");
    /// ```
    #[cfg(feature = "serde")]
    pub fn load_from_str(json: &str) -> Result<Self, CurveEngineError> {
        let definition: Self =
            serde_json::from_str(json).map_err(|e| CurveEngineError::parse(e.to_string()))?;

        // Validate the loaded definition
        definition
            .validate()
            .map_err(|e| CurveEngineError::configuration("definition", e))?;

        Ok(definition)
    }

    /// Gets the default curve definition for a rate index, or returns an error
    /// if no default exists.
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::UnknownIndex` if no default definition exists.
    pub fn require_default_for_index(index: RateIndex) -> Result<Self, CurveEngineError> {
        Self::default_for_index(index).ok_or_else(|| {
            CurveEngineError::unknown_index(format!("{:?}", index))
        })
    }

    /// Saves the curve definition to a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the JSON file
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::Io` if the file cannot be written.
    #[cfg(feature = "serde")]
    pub fn save_to_json(&self, path: &Path) -> Result<(), CurveEngineError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CurveEngineError::parse(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Serializes the curve definition to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::ConfigurationParse` if serialization fails.
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> Result<String, CurveEngineError> {
        serde_json::to_string_pretty(self).map_err(|e| CurveEngineError::parse(e.to_string()))
    }
}

impl fmt::Display for CurveDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CurveDefinition({}, {} instruments)",
            self.index_key,
            self.instruments.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // CurveInstrumentType Tests
    // ========================================

    #[test]
    fn test_instrument_type_code() {
        assert_eq!(CurveInstrumentType::Ois.code(), "OIS");
        assert_eq!(CurveInstrumentType::Irs.code(), "IRS");
        assert_eq!(CurveInstrumentType::Fra.code(), "FRA");
        assert_eq!(CurveInstrumentType::Future.code(), "FUT");
        assert_eq!(CurveInstrumentType::Deposit.code(), "DEP");
    }

    #[test]
    fn test_instrument_type_description() {
        assert_eq!(
            CurveInstrumentType::Ois.description(),
            "Overnight Index Swap"
        );
        assert_eq!(CurveInstrumentType::Irs.description(), "Interest Rate Swap");
    }

    #[test]
    fn test_instrument_type_default() {
        let default: CurveInstrumentType = Default::default();
        assert_eq!(default, CurveInstrumentType::Ois);
    }

    #[test]
    fn test_instrument_type_display() {
        assert_eq!(format!("{}", CurveInstrumentType::Ois), "OIS");
        assert_eq!(format!("{}", CurveInstrumentType::Future), "FUT");
    }

    #[test]
    fn test_instrument_type_from_str() {
        assert_eq!(
            "OIS".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Ois
        );
        assert_eq!(
            "irs".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Irs
        );
        assert_eq!(
            "SWAP".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Irs
        );
        assert_eq!(
            "FUT".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Future
        );
        assert_eq!(
            "FUTURE".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Future
        );
        assert_eq!(
            "DEP".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Deposit
        );
        assert_eq!(
            "MM".parse::<CurveInstrumentType>().unwrap(),
            CurveInstrumentType::Deposit
        );
    }

    #[test]
    fn test_instrument_type_from_str_invalid() {
        assert!("INVALID".parse::<CurveInstrumentType>().is_err());
    }

    #[test]
    fn test_instrument_type_all() {
        let all = CurveInstrumentType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&CurveInstrumentType::Ois));
        assert!(all.contains(&CurveInstrumentType::Deposit));
    }

    #[test]
    fn test_instrument_type_clone_copy() {
        let t1 = CurveInstrumentType::Ois;
        let t2 = t1; // Copy
        let t3 = t1.clone();
        assert_eq!(t1, t2);
        assert_eq!(t1, t3);
    }

    #[test]
    fn test_instrument_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CurveInstrumentType::Ois);
        set.insert(CurveInstrumentType::Irs);
        set.insert(CurveInstrumentType::Ois); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ========================================
    // InstrumentTenor Tests
    // ========================================

    #[test]
    fn test_tenor_code() {
        assert_eq!(InstrumentTenor::Overnight.code(), "ON");
        assert_eq!(InstrumentTenor::OneWeek.code(), "1W");
        assert_eq!(InstrumentTenor::ThreeMonths.code(), "3M");
        assert_eq!(InstrumentTenor::OneYear.code(), "1Y");
        assert_eq!(InstrumentTenor::TenYears.code(), "10Y");
        assert_eq!(InstrumentTenor::FiftyYears.code(), "50Y");
    }

    #[test]
    fn test_tenor_to_years() {
        assert!((InstrumentTenor::Overnight.to_years() - 1.0 / 365.0).abs() < 1e-10);
        assert!((InstrumentTenor::OneWeek.to_years() - 7.0 / 365.0).abs() < 1e-10);
        assert!((InstrumentTenor::OneMonth.to_years() - 1.0 / 12.0).abs() < 1e-10);
        assert!((InstrumentTenor::ThreeMonths.to_years() - 0.25).abs() < 1e-10);
        assert!((InstrumentTenor::SixMonths.to_years() - 0.5).abs() < 1e-10);
        assert!((InstrumentTenor::OneYear.to_years() - 1.0).abs() < 1e-10);
        assert!((InstrumentTenor::TenYears.to_years() - 10.0).abs() < 1e-10);
        assert!((InstrumentTenor::FiftyYears.to_years() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_to_months() {
        assert_eq!(InstrumentTenor::Overnight.to_months(), 0);
        assert_eq!(InstrumentTenor::OneMonth.to_months(), 1);
        assert_eq!(InstrumentTenor::ThreeMonths.to_months(), 3);
        assert_eq!(InstrumentTenor::OneYear.to_months(), 12);
        assert_eq!(InstrumentTenor::FiveYears.to_months(), 60);
        assert_eq!(InstrumentTenor::FiftyYears.to_months(), 600);
    }

    #[test]
    fn test_tenor_default() {
        let default: InstrumentTenor = Default::default();
        assert_eq!(default, InstrumentTenor::OneYear);
    }

    #[test]
    fn test_tenor_display() {
        assert_eq!(format!("{}", InstrumentTenor::ThreeMonths), "3M");
        assert_eq!(format!("{}", InstrumentTenor::TenYears), "10Y");
    }

    #[test]
    fn test_tenor_from_str() {
        assert_eq!(
            "ON".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::Overnight
        );
        assert_eq!(
            "3M".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::ThreeMonths
        );
        assert_eq!(
            "1Y".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::OneYear
        );
        assert_eq!(
            "12M".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::OneYear
        );
        assert_eq!(
            "10Y".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::TenYears
        );
        assert_eq!(
            "50Y".parse::<InstrumentTenor>().unwrap(),
            InstrumentTenor::FiftyYears
        );
    }

    #[test]
    fn test_tenor_from_str_invalid() {
        assert!("100Y".parse::<InstrumentTenor>().is_err());
        assert!("INVALID".parse::<InstrumentTenor>().is_err());
    }

    #[test]
    fn test_tenor_ordering() {
        assert!(InstrumentTenor::Overnight < InstrumentTenor::OneMonth);
        assert!(InstrumentTenor::OneMonth < InstrumentTenor::OneYear);
        assert!(InstrumentTenor::OneYear < InstrumentTenor::TenYears);
        assert!(InstrumentTenor::TenYears < InstrumentTenor::FiftyYears);
    }

    #[test]
    fn test_tenor_all() {
        let all = InstrumentTenor::all();
        assert_eq!(all.len(), 26);
        assert_eq!(all[0], InstrumentTenor::Overnight);
        assert_eq!(all[all.len() - 1], InstrumentTenor::FiftyYears);

        // Verify ordering
        for i in 1..all.len() {
            assert!(all[i - 1] < all[i], "Tenors should be in ascending order");
        }
    }

    #[test]
    fn test_standard_ois_tenors() {
        let tenors = InstrumentTenor::standard_ois_tenors();
        assert!(!tenors.is_empty());
        assert!(tenors.contains(&InstrumentTenor::OneYear));
        assert!(tenors.contains(&InstrumentTenor::TenYears));
        assert!(tenors.contains(&InstrumentTenor::ThirtyYears));
    }

    #[test]
    fn test_tenor_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InstrumentTenor::OneYear);
        set.insert(InstrumentTenor::FiveYears);
        set.insert(InstrumentTenor::OneYear); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ========================================
    // InstrumentSpec Tests
    // ========================================

    #[test]
    fn test_spec_new() {
        let spec = InstrumentSpec::new(CurveInstrumentType::Ois, InstrumentTenor::FiveYears);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Ois);
        assert_eq!(spec.tenor(), InstrumentTenor::FiveYears);
        assert!(spec.convexity_adjustment().is_none());
    }

    #[test]
    fn test_spec_ois() {
        let spec = InstrumentSpec::ois(InstrumentTenor::TenYears);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Ois);
        assert_eq!(spec.tenor(), InstrumentTenor::TenYears);
    }

    #[test]
    fn test_spec_irs() {
        let spec = InstrumentSpec::irs(InstrumentTenor::FiveYears);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Irs);
    }

    #[test]
    fn test_spec_fra() {
        let spec = InstrumentSpec::fra(InstrumentTenor::ThreeMonths);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Fra);
    }

    #[test]
    fn test_spec_deposit() {
        let spec = InstrumentSpec::deposit(InstrumentTenor::OneMonth);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Deposit);
    }

    #[test]
    fn test_spec_future() {
        let spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001);
        assert_eq!(spec.instrument_type(), CurveInstrumentType::Future);
        assert_eq!(spec.convexity_adjustment(), Some(0.0001));
    }

    #[test]
    fn test_spec_with_convexity_adjustment() {
        let spec = InstrumentSpec::new(CurveInstrumentType::Future, InstrumentTenor::SixMonths)
            .with_convexity_adjustment(0.0002);
        assert_eq!(spec.convexity_adjustment(), Some(0.0002));
    }

    #[test]
    fn test_spec_maturity_years() {
        let spec = InstrumentSpec::ois(InstrumentTenor::FiveYears);
        assert!((spec.maturity_years() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_spec_validate_valid_ois() {
        let spec = InstrumentSpec::ois(InstrumentTenor::TenYears);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_spec_validate_valid_future() {
        let spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_spec_validate_convexity_on_non_future() {
        let spec = InstrumentSpec::ois(InstrumentTenor::OneYear)
            .with_convexity_adjustment(0.0001);
        let result = spec.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Convexity adjustment"));
    }

    #[test]
    fn test_spec_validate_future_long_tenor() {
        let spec = InstrumentSpec::future(InstrumentTenor::FiveYears, 0.0001);
        let result = spec.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unusually long"));
    }

    #[test]
    fn test_spec_validate_deposit_long_tenor() {
        let spec = InstrumentSpec::deposit(InstrumentTenor::TwoYears);
        let result = spec.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unusually long"));
    }

    #[test]
    fn test_spec_display() {
        let spec = InstrumentSpec::ois(InstrumentTenor::FiveYears);
        assert_eq!(format!("{}", spec), "OIS 5Y");

        let future_spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.000123);
        let display = format!("{}", future_spec);
        assert!(display.contains("FUT 3M"));
        assert!(display.contains("conv_adj"));
    }

    #[test]
    fn test_spec_clone() {
        let spec1 = InstrumentSpec::ois(InstrumentTenor::TenYears);
        let spec2 = spec1.clone();
        assert_eq!(spec1, spec2);
    }

    #[test]
    fn test_spec_equality() {
        let spec1 = InstrumentSpec::ois(InstrumentTenor::FiveYears);
        let spec2 = InstrumentSpec::ois(InstrumentTenor::FiveYears);
        let spec3 = InstrumentSpec::ois(InstrumentTenor::TenYears);

        assert_eq!(spec1, spec2);
        assert_ne!(spec1, spec3);
    }

    #[test]
    fn test_spec_debug() {
        let spec = InstrumentSpec::ois(InstrumentTenor::FiveYears);
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("InstrumentSpec"));
        assert!(debug_str.contains("Ois"));
        assert!(debug_str.contains("FiveYears"));
    }

    // ========================================
    // Serde Tests (when feature enabled)
    // ========================================

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;

        #[test]
        fn test_instrument_type_serde() {
            let inst_type = CurveInstrumentType::Ois;
            let json = serde_json::to_string(&inst_type).unwrap();
            let deserialized: CurveInstrumentType = serde_json::from_str(&json).unwrap();
            assert_eq!(inst_type, deserialized);
        }

        #[test]
        fn test_tenor_serde() {
            let tenor = InstrumentTenor::FiveYears;
            let json = serde_json::to_string(&tenor).unwrap();
            let deserialized: InstrumentTenor = serde_json::from_str(&json).unwrap();
            assert_eq!(tenor, deserialized);
        }

        #[test]
        fn test_spec_serde() {
            let spec = InstrumentSpec::ois(InstrumentTenor::TenYears);
            let json = serde_json::to_string(&spec).unwrap();
            let deserialized: InstrumentSpec = serde_json::from_str(&json).unwrap();
            assert_eq!(spec, deserialized);
        }

        #[test]
        fn test_spec_serde_with_convexity() {
            let spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001);
            let json = serde_json::to_string(&spec).unwrap();
            assert!(json.contains("convexity_adjustment"));
            let deserialized: InstrumentSpec = serde_json::from_str(&json).unwrap();
            assert_eq!(spec, deserialized);
        }

        #[test]
        fn test_spec_serde_without_convexity_skipped() {
            let spec = InstrumentSpec::ois(InstrumentTenor::FiveYears);
            let json = serde_json::to_string(&spec).unwrap();
            // convexity_adjustment should be skipped when None
            assert!(!json.contains("convexity_adjustment"));
        }

        #[test]
        fn test_curve_definition_serde() {
            let def = CurveDefinition::default_usd_sofr();
            let json = serde_json::to_string(&def).unwrap();
            let deserialized: CurveDefinition = serde_json::from_str(&json).unwrap();
            assert_eq!(def.index_key(), deserialized.index_key());
            assert_eq!(def.rate_index(), deserialized.rate_index());
            assert_eq!(def.instruments().len(), deserialized.instruments().len());
        }

        #[test]
        fn test_curve_definition_load_from_str() {
            let json = r#"{
                "index_key": "USD-SOFR",
                "rate_index": "Sofr",
                "instruments": [
                    {"instrument_type": "Ois", "tenor": "OneYear"},
                    {"instrument_type": "Ois", "tenor": "FiveYears"}
                ],
                "convention": {
                    "fixed_leg": {
                        "day_count": "Actual360",
                        "payment_frequency": "Annual",
                        "calendar": "NewYork",
                        "business_day_convention": "ModifiedFollowing",
                        "payment_lag": 2
                    },
                    "float_leg": {
                        "day_count": "Actual360",
                        "payment_frequency": "Annual",
                        "calendar": "NewYork",
                        "business_day_convention": "ModifiedFollowing",
                        "payment_lag": 2
                    },
                    "float_index": "Sofr",
                    "spot_lag": 2
                }
            }"#;

            let def = CurveDefinition::load_from_str(json).unwrap();
            assert_eq!(def.index_key(), "USD-SOFR");
            assert_eq!(def.rate_index(), RateIndex::Sofr);
            assert_eq!(def.len(), 2);
            assert_eq!(def.instruments()[0].tenor(), InstrumentTenor::OneYear);
            assert_eq!(def.instruments()[1].tenor(), InstrumentTenor::FiveYears);
        }

        #[test]
        fn test_curve_definition_load_from_str_invalid_json() {
            let json = "{ invalid json }";
            let result = CurveDefinition::load_from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn test_curve_definition_load_from_str_empty_instruments() {
            let json = r#"{
                "index_key": "USD-SOFR",
                "rate_index": "Sofr",
                "instruments": [],
                "convention": {
                    "fixed_leg": {
                        "day_count": "Actual360",
                        "payment_frequency": "Annual",
                        "calendar": "NewYork",
                        "business_day_convention": "ModifiedFollowing",
                        "payment_lag": 2
                    },
                    "float_leg": {
                        "day_count": "Actual360",
                        "payment_frequency": "Annual",
                        "calendar": "NewYork",
                        "business_day_convention": "ModifiedFollowing",
                        "payment_lag": 2
                    },
                    "float_index": "Sofr",
                    "spot_lag": 2
                }
            }"#;

            let result = CurveDefinition::load_from_str(json);
            assert!(result.is_err());
            // Should fail validation (empty instruments)
        }

        #[test]
        fn test_curve_definition_to_json_string() {
            let def = CurveDefinition::default_usd_sofr();
            let json = def.to_json_string().unwrap();
            assert!(json.contains("USD-SOFR"));
            assert!(json.contains("Sofr"));
            assert!(json.contains("instruments"));
        }

        #[test]
        fn test_curve_definition_roundtrip() {
            let original = CurveDefinition::default_usd_sofr();
            let json = original.to_json_string().unwrap();
            let loaded = CurveDefinition::load_from_str(&json).unwrap();

            assert_eq!(original.index_key(), loaded.index_key());
            assert_eq!(original.rate_index(), loaded.rate_index());
            assert_eq!(original.len(), loaded.len());

            let orig_sorted = original.sorted_instruments();
            let loaded_sorted = loaded.sorted_instruments();
            for i in 0..orig_sorted.len() {
                assert_eq!(orig_sorted[i].tenor(), loaded_sorted[i].tenor());
                assert_eq!(orig_sorted[i].instrument_type(), loaded_sorted[i].instrument_type());
            }
        }
    }

    // ========================================
    // CurveDefinition Tests
    // ========================================

    #[test]
    fn test_curve_definition_require_default_for_index_known() {
        let def = CurveDefinition::require_default_for_index(RateIndex::Sofr);
        assert!(def.is_ok());
        assert_eq!(def.unwrap().rate_index(), RateIndex::Sofr);
    }

    #[test]
    fn test_curve_definition_require_default_for_index_unknown() {
        let result = CurveDefinition::require_default_for_index(RateIndex::Sonia);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_definition_new() {
        let def = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        );
        assert_eq!(def.index_key(), "USD-SOFR");
        assert_eq!(def.rate_index(), RateIndex::Sofr);
        assert!(def.instruments().is_empty());
        assert!(def.is_empty());
        assert_eq!(def.len(), 0);
    }

    #[test]
    fn test_curve_definition_with_instrument() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));

        assert_eq!(def.len(), 2);
        assert!(!def.is_empty());
        assert_eq!(def.instruments()[0].tenor(), InstrumentTenor::OneYear);
        assert_eq!(def.instruments()[1].tenor(), InstrumentTenor::FiveYears);
    }

    #[test]
    fn test_curve_definition_with_instruments() {
        let instruments = vec![
            InstrumentSpec::ois(InstrumentTenor::OneYear),
            InstrumentSpec::ois(InstrumentTenor::TwoYears),
            InstrumentSpec::ois(InstrumentTenor::ThreeYears),
        ];
        let def = CurveDefinition::with_instruments(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
            instruments,
        );
        assert_eq!(def.len(), 3);
    }

    #[test]
    fn test_curve_definition_with_instruments_iter() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instruments_iter([
                InstrumentSpec::ois(InstrumentTenor::OneYear),
                InstrumentSpec::ois(InstrumentTenor::FiveYears),
            ]);
        assert_eq!(def.len(), 2);
    }

    #[test]
    fn test_curve_definition_sorted_instruments() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::TenYears))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears));

        let sorted = def.sorted_instruments();
        assert_eq!(sorted[0].tenor(), InstrumentTenor::OneYear);
        assert_eq!(sorted[1].tenor(), InstrumentTenor::TwoYears);
        assert_eq!(sorted[2].tenor(), InstrumentTenor::FiveYears);
        assert_eq!(sorted[3].tenor(), InstrumentTenor::TenYears);
    }

    #[test]
    fn test_curve_definition_sorted_instruments_owned() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::ThirtyYears))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::ThreeMonths));

        let sorted = def.sorted_instruments_owned();
        assert_eq!(sorted[0].tenor(), InstrumentTenor::ThreeMonths);
        assert_eq!(sorted[1].tenor(), InstrumentTenor::ThirtyYears);
    }

    #[test]
    fn test_curve_definition_validate_empty() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr());
        let result = def.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one instrument"));
    }

    #[test]
    fn test_curve_definition_validate_valid() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear));
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_curve_definition_validate_invalid_instrument() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear).with_convexity_adjustment(0.1));
        let result = def.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation failed"));
    }

    #[test]
    fn test_curve_definition_default_usd_sofr() {
        let def = CurveDefinition::default_usd_sofr();
        assert_eq!(def.index_key(), "USD-SOFR");
        assert_eq!(def.rate_index(), RateIndex::Sofr);
        assert_eq!(def.len(), 12);
        assert!(def.validate().is_ok());

        // Check sorted order
        let sorted = def.sorted_instruments();
        for i in 1..sorted.len() {
            assert!(
                sorted[i - 1].tenor() < sorted[i].tenor(),
                "Default instruments should be in sorted order"
            );
        }
    }

    #[test]
    fn test_curve_definition_default_for_index_sofr() {
        let def = CurveDefinition::default_for_index(RateIndex::Sofr);
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.rate_index(), RateIndex::Sofr);
    }

    #[test]
    fn test_curve_definition_default_for_index_unknown() {
        // Sonia currently has no default definition
        let def = CurveDefinition::default_for_index(RateIndex::Sonia);
        assert!(def.is_none());
    }

    #[test]
    fn test_curve_definition_display() {
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
            .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));
        let display = format!("{}", def);
        assert!(display.contains("USD-SOFR"));
        assert!(display.contains("2 instruments"));
    }

    #[test]
    fn test_curve_definition_clone() {
        let def1 = CurveDefinition::default_usd_sofr();
        let def2 = def1.clone();
        assert_eq!(def1.index_key(), def2.index_key());
        assert_eq!(def1.rate_index(), def2.rate_index());
        assert_eq!(def1.len(), def2.len());
    }

    #[test]
    fn test_curve_definition_convention() {
        let convention = SwapConvention::usd_sofr();
        let def = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, convention.clone());
        assert_eq!(def.convention().float_index, RateIndex::Sofr);
        assert_eq!(def.convention().spot_lag, 2);
    }

    #[test]
    fn test_curve_definition_debug() {
        let def = CurveDefinition::default_usd_sofr();
        let debug_str = format!("{:?}", def);
        assert!(debug_str.contains("CurveDefinition"));
        assert!(debug_str.contains("USD-SOFR"));
    }
}

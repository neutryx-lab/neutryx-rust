//! Instrument pre-compilation module for efficient calibration.
//!
//! This module provides the infrastructure for compiling `MarketInstrument`
//! to static `CompiledInstrument` objects, eliminating calendar and convention
//! lookups during calibration iterations.
//!
//! # Requirements Coverage
//!
//! - **Requirement 1**: Instrument Compiler Infrastructure
//! - **Requirement 8**: Error Handling and Validation

use num_traits::Float;
use thiserror::Error;

use crate::builder::CalibrationInstrument;
use crate::market::{curves::YieldCurve, MarketDataError};

// =============================================================================
// InstrumentType Enumeration (Requirement 1.4)
// =============================================================================

/// Enumeration of supported instrument types for compilation.
///
/// # Requirement 1.4
///
/// The Compiler shall support Deposit, Swap, OIS, FRA, Futures instrument types.
/// XCcyBasis, FxForward, FxSwap are explicitly unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentType {
    /// Simple deposit/money market instrument.
    Deposit,
    /// Interest rate swap (IRS).
    Swap,
    /// Overnight index swap.
    Ois,
    /// Forward rate agreement.
    Fra,
    /// Interest rate futures (with convexity adjustment).
    Futures,
}

impl InstrumentType {
    /// Returns the string representation of the instrument type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::builder::compile::InstrumentType;
    ///
    /// assert_eq!(InstrumentType::Deposit.as_str(), "Deposit");
    /// assert_eq!(InstrumentType::Swap.as_str(), "Swap");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Swap => "Swap",
            Self::Ois => "OIS",
            Self::Fra => "FRA",
            Self::Futures => "Futures",
        }
    }
}

impl std::fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// CompileError Type (Requirements 8.1-8.5, 1.3, 1.5)
// =============================================================================

/// Errors that can occur during instrument compilation.
///
/// # Requirement 8.4
///
/// The CompileError shall use `thiserror` to provide structured errors.
///
/// # Requirement 8.5
///
/// When an error occurs, the system shall include the problematic
/// instrument's index and rate ID.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Invalid maturity - maturity date is before the valuation date.
    ///
    /// # Requirement 8.1
    ///
    /// When the maturity date is before the valuation date, the Compiler
    /// shall return this error.
    #[error("Invalid maturity for instrument {index}: {rate_id}")]
    InvalidMaturity {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Invalid year fraction - cashflow has negative year fraction.
    ///
    /// # Requirement 8.2
    ///
    /// When a cashflow date has a negative year fraction, the Compiler
    /// shall return this error.
    #[error("Invalid year fraction at index {index} for instrument {rate_id}")]
    InvalidYearFraction {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Convention mismatch - convention and instrument type are inconsistent.
    ///
    /// # Requirement 8.3
    ///
    /// When the convention and instrument type are inconsistent, the Compiler
    /// shall return this error.
    #[error("Convention mismatch for instrument {index}: {rate_id}")]
    ConventionMismatch {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Invalid convention detected during compilation.
    ///
    /// # Requirement 1.3
    ///
    /// When an invalid convention is detected at compile time, the Compiler
    /// shall return this error.
    #[error("Invalid convention for instrument {index}: {rate_id}")]
    InvalidConvention {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Rate identifier of the problematic instrument.
        rate_id: String,
    },

    /// Unsupported instrument type.
    ///
    /// # Requirement 1.5
    ///
    /// When the MarketConvention is XCcyBasis, FxForward, or FxSwap,
    /// the Compiler shall return this error.
    #[error("Unsupported instrument type at index {index}: {instrument_type}")]
    UnsupportedInstrument {
        /// Index of the problematic instrument (0-based).
        index: usize,
        /// Name of the unsupported instrument type.
        instrument_type: String,
    },
}

impl CompileError {
    /// Creates an invalid maturity error.
    pub fn invalid_maturity(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidMaturity {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an invalid year fraction error.
    pub fn invalid_year_fraction(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidYearFraction {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates a convention mismatch error.
    pub fn convention_mismatch(index: usize, rate_id: impl Into<String>) -> Self {
        Self::ConventionMismatch {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an invalid convention error.
    pub fn invalid_convention(index: usize, rate_id: impl Into<String>) -> Self {
        Self::InvalidConvention {
            index,
            rate_id: rate_id.into(),
        }
    }

    /// Creates an unsupported instrument error.
    pub fn unsupported_instrument(index: usize, instrument_type: impl Into<String>) -> Self {
        Self::UnsupportedInstrument {
            index,
            instrument_type: instrument_type.into(),
        }
    }

    /// Returns the instrument index where the error occurred.
    pub fn instrument_index(&self) -> usize {
        match self {
            Self::InvalidMaturity { index, .. }
            | Self::InvalidYearFraction { index, .. }
            | Self::ConventionMismatch { index, .. }
            | Self::InvalidConvention { index, .. }
            | Self::UnsupportedInstrument { index, .. } => *index,
        }
    }
}

// =============================================================================
// CompiledInstrument Structure (Requirements 1.2, 2.5, 3.5)
// =============================================================================

/// Pre-compiled calibration instrument with static cashflow data.
///
/// # Requirement 1.2
///
/// The CompiledInstrument shall hold pre-computed fields:
/// - cashflow_dates (Vec<T> as times)
/// - year_fractions (Vec<T>)
/// - notionals (Vec<T>)
/// - discount_factor_indices (Vec<usize>)
///
/// # Requirement 2.5
///
/// The CompiledInstrument shall implement Clone and Debug traits.
///
/// # Requirement 3.5
///
/// During calibration iteration, the System shall not cause memory allocations.
/// This is achieved by pre-allocating all vectors at compile time.
#[derive(Debug, Clone)]
pub struct CompiledInstrument<T: Float> {
    /// Instrument type identifier.
    instrument_type: InstrumentType,
    /// Market-quoted rate.
    market_rate: T,
    /// Maturity in year fraction.
    maturity: T,
    /// Cashflow times (year fractions from valuation date).
    cashflow_times: Vec<T>,
    /// Year fractions for each cashflow period.
    year_fractions: Vec<T>,
    /// Notional amounts for each cashflow.
    notionals: Vec<T>,
    /// Discount factor indices for InterpolationMatrix.
    df_indices: Vec<usize>,
    /// Fixed rate (for swaps, if applicable).
    fixed_rate: Option<T>,
}

impl<T: Float> CompiledInstrument<T> {
    /// Creates a new CompiledInstrument with validation.
    ///
    /// # Arguments
    ///
    /// * `instrument_type` - Type of the instrument
    /// * `market_rate` - Market-quoted rate
    /// * `maturity` - Maturity in year fraction
    /// * `cashflow_times` - Cashflow times (must be ascending and positive)
    /// * `year_fractions` - Year fractions (must be positive)
    /// * `notionals` - Notional amounts
    /// * `fixed_rate` - Optional fixed rate for swaps
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Arrays have different lengths
    /// - cashflow_times is not ascending
    /// - year_fractions contains non-positive values
    pub fn new(
        instrument_type: InstrumentType,
        market_rate: T,
        maturity: T,
        cashflow_times: Vec<T>,
        year_fractions: Vec<T>,
        notionals: Vec<T>,
        fixed_rate: Option<T>,
    ) -> Result<Self, String> {
        // Validate array lengths match
        let n = cashflow_times.len();
        if year_fractions.len() != n || notionals.len() != n {
            return Err("Array lengths must match".to_string());
        }

        // Validate ascending order of cashflow_times
        for i in 1..n {
            if cashflow_times[i] <= cashflow_times[i - 1] {
                return Err("cashflow_times must be strictly ascending".to_string());
            }
        }

        // Validate positive year_fractions
        for (i, &yf) in year_fractions.iter().enumerate() {
            if yf <= T::zero() {
                return Err(format!("year_fraction at index {} must be positive", i));
            }
        }

        // Generate df_indices (0-indexed for now, will be mapped by InterpolationMatrix)
        let df_indices = (0..n).collect();

        Ok(Self {
            instrument_type,
            market_rate,
            maturity,
            cashflow_times,
            year_fractions,
            notionals,
            df_indices,
            fixed_rate,
        })
    }

    /// Creates a simple deposit instrument.
    ///
    /// # Arguments
    ///
    /// * `market_rate` - Market-quoted rate
    /// * `maturity` - Maturity in year fraction
    pub fn deposit(market_rate: T, maturity: T) -> Result<Self, String> {
        if maturity <= T::zero() {
            return Err("maturity must be positive".to_string());
        }
        Self::new(
            InstrumentType::Deposit,
            market_rate,
            maturity,
            vec![maturity],
            vec![maturity],
            vec![T::one()],
            None,
        )
    }

    /// Creates a FRA instrument.
    ///
    /// # Arguments
    ///
    /// * `market_rate` - Market-quoted rate
    /// * `start` - Start time in year fraction
    /// * `end` - End time in year fraction
    pub fn fra(market_rate: T, start: T, end: T) -> Result<Self, String> {
        if end <= start {
            return Err("end must be greater than start".to_string());
        }
        if start < T::zero() {
            return Err("start must be non-negative".to_string());
        }
        let tau = end - start;
        Self::new(
            InstrumentType::Fra,
            market_rate,
            end,
            vec![start, end],
            vec![tau, tau],
            vec![T::one(), T::one()],
            None,
        )
    }

    /// Returns the instrument type.
    pub fn get_instrument_type(&self) -> InstrumentType {
        self.instrument_type
    }

    /// Returns the cashflow times.
    pub fn cashflow_times(&self) -> &[T] {
        &self.cashflow_times
    }

    /// Returns the year fractions.
    pub fn year_fractions(&self) -> &[T] {
        &self.year_fractions
    }

    /// Returns the notional amounts.
    pub fn notionals(&self) -> &[T] {
        &self.notionals
    }

    /// Returns the discount factor indices.
    pub fn df_indices(&self) -> &[usize] {
        &self.df_indices
    }

    /// Returns the fixed rate if set.
    pub fn fixed_rate(&self) -> Option<T> {
        self.fixed_rate
    }

    /// Returns the number of cashflows.
    pub fn num_cashflows(&self) -> usize {
        self.cashflow_times.len()
    }
}

// =============================================================================
// CalibrationInstrument Implementation (Requirements 3.1-3.4)
// =============================================================================

impl<T: Float> CalibrationInstrument<T> for CompiledInstrument<T> {
    /// Returns the market-quoted rate.
    ///
    /// # Requirement 3.1
    ///
    /// market_rate() returns the stored value directly.
    fn market_rate(&self) -> T {
        self.market_rate
    }

    /// Computes the theoretical rate from the yield curve.
    ///
    /// # Requirement 3.1
    ///
    /// theoretical_rate() computes using DF retrieval and vector products only,
    /// without calendar operations.
    ///
    /// # Requirement 3.2
    ///
    /// The computation maintains O(n) time complexity where n = number of cashflows.
    fn theoretical_rate<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError> {
        match self.instrument_type {
            InstrumentType::Deposit => {
                // Deposit: rate = (1/DF(T) - 1) / T
                let df = curve.discount_factor(self.maturity)?;
                if df <= T::zero() {
                    return Err(MarketDataError::InvalidInput {
                        message: "discount factor must be positive".to_string(),
                    });
                }
                Ok((T::one() / df - T::one()) / self.maturity)
            }
            InstrumentType::Fra => {
                // FRA: rate = (DF(start) / DF(end) - 1) / tau
                if self.cashflow_times.len() < 2 {
                    return Err(MarketDataError::InvalidInput {
                        message: "FRA requires at least 2 cashflow times".to_string(),
                    });
                }
                let start = self.cashflow_times[0];
                let end = self.cashflow_times[1];
                let tau = self.year_fractions[0];

                let df_start = if start <= T::zero() {
                    T::one()
                } else {
                    curve.discount_factor(start)?
                };
                let df_end = curve.discount_factor(end)?;

                if df_end <= T::zero() {
                    return Err(MarketDataError::InvalidInput {
                        message: "discount factor must be positive".to_string(),
                    });
                }

                Ok((df_start / df_end - T::one()) / tau)
            }
            InstrumentType::Ois | InstrumentType::Swap => {
                // Swap/OIS: par rate = (1 - DF(T)) / annuity
                // annuity = sum(DF(t_i) * tau_i)
                let mut annuity = T::zero();
                for (i, &t) in self.cashflow_times.iter().enumerate() {
                    let df = curve.discount_factor(t)?;
                    let tau = self.year_fractions[i];
                    annuity = annuity + df * tau;
                }

                if annuity <= T::zero() {
                    return Err(MarketDataError::InterpolationFailed {
                        reason: "annuity is zero or negative".to_string(),
                    });
                }

                let df_maturity = curve.discount_factor(self.maturity)?;
                Ok((T::one() - df_maturity) / annuity)
            }
            InstrumentType::Futures => {
                // Futures: similar to FRA but with potential convexity adjustment
                // For now, treat as FRA
                if self.cashflow_times.len() < 2 {
                    // Single cashflow: treat as deposit-like
                    let df = curve.discount_factor(self.maturity)?;
                    if df <= T::zero() {
                        return Err(MarketDataError::InvalidInput {
                            message: "discount factor must be positive".to_string(),
                        });
                    }
                    return Ok((T::one() / df - T::one()) / self.maturity);
                }

                let start = self.cashflow_times[0];
                let end = self.cashflow_times[1];
                let tau = self.year_fractions[0];

                let df_start = if start <= T::zero() {
                    T::one()
                } else {
                    curve.discount_factor(start)?
                };
                let df_end = curve.discount_factor(end)?;

                if df_end <= T::zero() {
                    return Err(MarketDataError::InvalidInput {
                        message: "discount factor must be positive".to_string(),
                    });
                }

                Ok((df_start / df_end - T::one()) / tau)
            }
        }
    }

    /// Returns the instrument maturity.
    fn maturity(&self) -> T {
        self.maturity
    }

    /// Returns the instrument type as a string.
    fn instrument_type(&self) -> &'static str {
        self.instrument_type.as_str()
    }
}

// =============================================================================
// InstrumentCompiler (Requirements 1.1, 5.1, 5.2)
// =============================================================================

use std::marker::PhantomData;

use infra_master::market::convention::MarketConvention;
use infra_master::market::MarketInstrument as InfraMasterInstrument;
use infra_master::time::Date;
use pricer_core::math::numeric::from_f64;

/// Compiler for converting `infra_master::market::MarketInstrument` to
/// `CompiledInstrument<T>`.
///
/// # Requirement 5.1
///
/// The InstrumentCompiler shall be placed in `pricer_models::builder` module.
///
/// # Requirement 5.2
///
/// The Compiler shall receive `infra_master::market::MarketInstrument` as input
/// and output `pricer_models` specific types.
#[derive(Debug, Clone)]
pub struct InstrumentCompiler<T: Float> {
    /// Valuation date for year fraction calculations.
    valuation_date: Date,
    /// Phantom marker for generic type.
    _marker: PhantomData<T>,
}

impl<T: Float> InstrumentCompiler<T> {
    /// Creates a new InstrumentCompiler.
    ///
    /// # Arguments
    ///
    /// * `valuation_date` - The valuation date for year fraction calculations
    pub fn new(valuation_date: Date) -> Self {
        Self {
            valuation_date,
            _marker: PhantomData,
        }
    }

    /// Returns the valuation date.
    pub fn valuation_date(&self) -> Date {
        self.valuation_date
    }

    /// Compiles a single MarketInstrument to a CompiledInstrument.
    ///
    /// # Requirement 1.1
    ///
    /// When `compile()` is called, the Compiler shall convert `MarketInstrument`
    /// to `CompiledInstrument`, pre-computing all cashflow dates, year fractions,
    /// and notionals.
    ///
    /// # Arguments
    ///
    /// * `instrument` - The MarketInstrument to compile
    /// * `index` - The instrument index for error reporting
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if:
    /// - Maturity is before valuation date (InvalidMaturity)
    /// - Year fraction is negative (InvalidYearFraction)
    /// - Convention is unsupported (UnsupportedInstrument)
    pub fn compile(
        &self,
        instrument: &InfraMasterInstrument,
        index: usize,
    ) -> Result<CompiledInstrument<T>, CompileError> {
        let rate_id = instrument.rate_id.to_string();

        // Check for unsupported instrument types (Requirement 1.5)
        let instrument_type = self.get_instrument_type(&instrument.convention, index)?;

        // Validate maturity (Requirement 8.1)
        if instrument.maturity_date <= self.valuation_date {
            return Err(CompileError::invalid_maturity(index, &rate_id));
        }

        // Calculate year fractions and cashflow times
        let maturity_years = self.date_to_year_fraction(instrument.maturity_date);
        if maturity_years <= T::zero() {
            return Err(CompileError::invalid_maturity(index, &rate_id));
        }

        // Build cashflow structure based on instrument type
        match instrument_type {
            InstrumentType::Deposit => {
                self.compile_deposit(instrument, index, maturity_years, &rate_id)
            }
            InstrumentType::Fra => self.compile_fra(instrument, index, maturity_years, &rate_id),
            InstrumentType::Swap | InstrumentType::Ois => {
                self.compile_swap(instrument, index, maturity_years, &rate_id, instrument_type)
            }
            InstrumentType::Futures => {
                self.compile_futures(instrument, index, maturity_years, &rate_id)
            }
        }
    }

    /// Batch compiles multiple MarketInstruments.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Iterator of MarketInstruments to compile
    ///
    /// # Errors
    ///
    /// Returns the first compilation error encountered.
    pub fn compile_batch<'a, I>(
        &self,
        instruments: I,
    ) -> Result<Vec<CompiledInstrument<T>>, CompileError>
    where
        I: IntoIterator<Item = &'a InfraMasterInstrument>,
    {
        instruments
            .into_iter()
            .enumerate()
            .map(|(i, inst)| self.compile(inst, i))
            .collect()
    }

    /// Determines the InstrumentType from the MarketConvention.
    fn get_instrument_type(
        &self,
        convention: &MarketConvention,
        index: usize,
    ) -> Result<InstrumentType, CompileError> {
        match convention {
            MarketConvention::Deposit(_) => Ok(InstrumentType::Deposit),
            MarketConvention::Swap(_) => Ok(InstrumentType::Swap),
            MarketConvention::Ois(_) => Ok(InstrumentType::Ois),
            MarketConvention::Fra(_) => Ok(InstrumentType::Fra),
            MarketConvention::Futures(_) => Ok(InstrumentType::Futures),
            MarketConvention::XCcyBasis(_) => {
                Err(CompileError::unsupported_instrument(index, "XCcyBasis"))
            }
            MarketConvention::FxForward(_) => {
                Err(CompileError::unsupported_instrument(index, "FxForward"))
            }
            MarketConvention::FxSwap(_) => {
                Err(CompileError::unsupported_instrument(index, "FxSwap"))
            }
        }
    }

    /// Converts a Date to year fraction from valuation date.
    fn date_to_year_fraction(&self, date: Date) -> T {
        let days = date - self.valuation_date;
        from_f64::<T>(days as f64 / 365.0)
    }

    /// Compiles a Deposit instrument.
    fn compile_deposit(
        &self,
        instrument: &InfraMasterInstrument,
        index: usize,
        maturity_years: T,
        rate_id: &str,
    ) -> Result<CompiledInstrument<T>, CompileError> {
        let market_rate = from_f64::<T>(instrument.rate_value);
        let year_fraction = from_f64::<T>(instrument.year_fraction());

        if year_fraction <= T::zero() {
            return Err(CompileError::invalid_year_fraction(index, rate_id));
        }

        CompiledInstrument::new(
            InstrumentType::Deposit,
            market_rate,
            maturity_years,
            vec![maturity_years],
            vec![year_fraction],
            vec![T::one()],
            None,
        )
        .map_err(|_| CompileError::invalid_year_fraction(index, rate_id))
    }

    /// Compiles a FRA instrument.
    fn compile_fra(
        &self,
        instrument: &InfraMasterInstrument,
        index: usize,
        maturity_years: T,
        rate_id: &str,
    ) -> Result<CompiledInstrument<T>, CompileError> {
        let market_rate = from_f64::<T>(instrument.rate_value);
        let start_years = self.date_to_year_fraction(instrument.effective_date);
        let year_fraction = from_f64::<T>(instrument.year_fraction());

        if year_fraction <= T::zero() {
            return Err(CompileError::invalid_year_fraction(index, rate_id));
        }

        // FRA has two cashflow times: start and end
        let cashflow_times = if start_years <= T::zero() {
            vec![maturity_years]
        } else {
            vec![start_years, maturity_years]
        };

        let n = cashflow_times.len();
        CompiledInstrument::new(
            InstrumentType::Fra,
            market_rate,
            maturity_years,
            cashflow_times,
            vec![year_fraction; n],
            vec![T::one(); n],
            None,
        )
        .map_err(|_| CompileError::invalid_year_fraction(index, rate_id))
    }

    /// Compiles a Swap or OIS instrument.
    fn compile_swap(
        &self,
        instrument: &InfraMasterInstrument,
        index: usize,
        maturity_years: T,
        rate_id: &str,
        instrument_type: InstrumentType,
    ) -> Result<CompiledInstrument<T>, CompileError> {
        let market_rate = from_f64::<T>(instrument.rate_value);

        // Generate annual payment schedule
        let mut cashflow_times = Vec::new();
        let mut year_fractions = Vec::new();
        let mut notionals = Vec::new();

        let period = T::one(); // Annual for simplicity
        let mut t = period;
        while t < maturity_years {
            cashflow_times.push(t);
            year_fractions.push(period);
            notionals.push(T::one());
            t = t + period;
        }

        // Add final cashflow at maturity
        if cashflow_times.is_empty() || cashflow_times.last().map(|&x| x) != Some(maturity_years) {
            let last_yf = if cashflow_times.is_empty() {
                maturity_years
            } else {
                maturity_years - *cashflow_times.last().unwrap()
            };

            if last_yf <= T::zero() {
                return Err(CompileError::invalid_year_fraction(index, rate_id));
            }

            cashflow_times.push(maturity_years);
            year_fractions.push(last_yf);
            notionals.push(T::one());
        }

        CompiledInstrument::new(
            instrument_type,
            market_rate,
            maturity_years,
            cashflow_times,
            year_fractions,
            notionals,
            Some(market_rate),
        )
        .map_err(|_| CompileError::invalid_year_fraction(index, rate_id))
    }

    /// Compiles a Futures instrument.
    fn compile_futures(
        &self,
        instrument: &InfraMasterInstrument,
        index: usize,
        maturity_years: T,
        rate_id: &str,
    ) -> Result<CompiledInstrument<T>, CompileError> {
        let market_rate = from_f64::<T>(instrument.rate_value);
        let start_years = self.date_to_year_fraction(instrument.effective_date);
        let year_fraction = from_f64::<T>(instrument.year_fraction());

        if year_fraction <= T::zero() {
            return Err(CompileError::invalid_year_fraction(index, rate_id));
        }

        // Futures similar to FRA
        let cashflow_times = if start_years <= T::zero() {
            vec![maturity_years]
        } else {
            vec![start_years, maturity_years]
        };

        let n = cashflow_times.len();
        CompiledInstrument::new(
            InstrumentType::Futures,
            market_rate,
            maturity_years,
            cashflow_times,
            vec![year_fraction; n],
            vec![T::one(); n],
            None,
        )
        .map_err(|_| CompileError::invalid_year_fraction(index, rate_id))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstrumentType Tests (Requirement 1.4)
    // =========================================================================

    #[test]
    fn test_instrument_type_as_str() {
        assert_eq!(InstrumentType::Deposit.as_str(), "Deposit");
        assert_eq!(InstrumentType::Swap.as_str(), "Swap");
        assert_eq!(InstrumentType::Ois.as_str(), "OIS");
        assert_eq!(InstrumentType::Fra.as_str(), "FRA");
        assert_eq!(InstrumentType::Futures.as_str(), "Futures");
    }

    #[test]
    fn test_instrument_type_display() {
        assert_eq!(format!("{}", InstrumentType::Deposit), "Deposit");
        assert_eq!(format!("{}", InstrumentType::Swap), "Swap");
        assert_eq!(format!("{}", InstrumentType::Ois), "OIS");
        assert_eq!(format!("{}", InstrumentType::Fra), "FRA");
        assert_eq!(format!("{}", InstrumentType::Futures), "Futures");
    }

    #[test]
    fn test_instrument_type_clone_copy() {
        let original = InstrumentType::Swap;
        let copied = original;
        let cloned = original.clone();
        assert_eq!(original, copied);
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_instrument_type_debug() {
        let swap = InstrumentType::Swap;
        let debug_str = format!("{:?}", swap);
        assert!(debug_str.contains("Swap"));
    }

    #[test]
    fn test_instrument_type_eq() {
        assert_eq!(InstrumentType::Deposit, InstrumentType::Deposit);
        assert_ne!(InstrumentType::Deposit, InstrumentType::Swap);
    }

    #[test]
    fn test_instrument_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InstrumentType::Deposit);
        set.insert(InstrumentType::Swap);
        set.insert(InstrumentType::Deposit); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // =========================================================================
    // CompileError Tests (Requirements 8.1-8.5)
    // =========================================================================

    #[test]
    fn test_invalid_maturity_error() {
        let err = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid maturity"));
        assert!(msg.contains("0"));
        assert!(msg.contains("USD-SOFR-1Y"));
        assert_eq!(err.instrument_index(), 0);
    }

    #[test]
    fn test_invalid_year_fraction_error() {
        let err = CompileError::invalid_year_fraction(3, "EUR-ESTR-5Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid year fraction"));
        assert!(msg.contains("3"));
        assert!(msg.contains("EUR-ESTR-5Y"));
        assert_eq!(err.instrument_index(), 3);
    }

    #[test]
    fn test_convention_mismatch_error() {
        let err = CompileError::convention_mismatch(5, "JPY-TONA-10Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Convention mismatch"));
        assert!(msg.contains("5"));
        assert!(msg.contains("JPY-TONA-10Y"));
        assert_eq!(err.instrument_index(), 5);
    }

    #[test]
    fn test_invalid_convention_error() {
        let err = CompileError::invalid_convention(2, "GBP-SONIA-2Y");
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid convention"));
        assert!(msg.contains("2"));
        assert!(msg.contains("GBP-SONIA-2Y"));
        assert_eq!(err.instrument_index(), 2);
    }

    #[test]
    fn test_unsupported_instrument_error() {
        let err = CompileError::unsupported_instrument(1, "XCcyBasis");
        let msg = format!("{}", err);
        assert!(msg.contains("Unsupported instrument type"));
        assert!(msg.contains("1"));
        assert!(msg.contains("XCcyBasis"));
        assert_eq!(err.instrument_index(), 1);
    }

    #[test]
    fn test_compile_error_equality() {
        let err1 = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let err2 = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let err3 = CompileError::invalid_maturity(1, "USD-SOFR-1Y");
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_compile_error_debug() {
        let err = CompileError::invalid_maturity(0, "USD-SOFR-1Y");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidMaturity"));
        assert!(debug_str.contains("USD-SOFR-1Y"));
    }

    #[test]
    fn test_compile_error_clone() {
        let original = CompileError::unsupported_instrument(5, "FxForward");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // =========================================================================
    // CompiledInstrument Tests (Requirements 1.2, 2.5, 3.5)
    // =========================================================================

    #[test]
    fn test_compiled_instrument_deposit_creation() {
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        assert_eq!(inst.get_instrument_type(), InstrumentType::Deposit);
        assert!((inst.market_rate - 0.03).abs() < 1e-10);
        assert!((inst.maturity - 1.0).abs() < 1e-10);
        assert_eq!(inst.num_cashflows(), 1);
    }

    #[test]
    fn test_compiled_instrument_fra_creation() {
        let inst: CompiledInstrument<f64> = CompiledInstrument::fra(0.025, 0.5, 1.0).unwrap();
        assert_eq!(inst.get_instrument_type(), InstrumentType::Fra);
        assert!((inst.market_rate - 0.025).abs() < 1e-10);
        assert!((inst.maturity - 1.0).abs() < 1e-10);
        assert_eq!(inst.num_cashflows(), 2);
    }

    #[test]
    fn test_compiled_instrument_accessors() {
        let inst: CompiledInstrument<f64> =
            CompiledInstrument::new(
                InstrumentType::Swap,
                0.035,
                5.0,
                vec![1.0, 2.0, 3.0, 4.0, 5.0],
                vec![1.0, 1.0, 1.0, 1.0, 1.0],
                vec![1.0, 1.0, 1.0, 1.0, 1.0],
                Some(0.035),
            )
            .unwrap();

        assert_eq!(inst.cashflow_times().len(), 5);
        assert_eq!(inst.year_fractions().len(), 5);
        assert_eq!(inst.notionals().len(), 5);
        assert_eq!(inst.df_indices().len(), 5);
        assert!(inst.fixed_rate().is_some());
        assert!((inst.fixed_rate().unwrap() - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_compiled_instrument_clone_debug() {
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        let cloned = inst.clone();
        assert!((cloned.market_rate - inst.market_rate).abs() < 1e-10);

        let debug_str = format!("{:?}", inst);
        assert!(debug_str.contains("Deposit"));
    }

    #[test]
    fn test_compiled_instrument_array_length_mismatch() {
        let result: Result<CompiledInstrument<f64>, _> = CompiledInstrument::new(
            InstrumentType::Swap,
            0.03,
            1.0,
            vec![1.0, 2.0],
            vec![1.0], // Length mismatch
            vec![1.0, 1.0],
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lengths"));
    }

    #[test]
    fn test_compiled_instrument_ascending_order_validation() {
        let result: Result<CompiledInstrument<f64>, _> = CompiledInstrument::new(
            InstrumentType::Swap,
            0.03,
            2.0,
            vec![2.0, 1.0], // Not ascending
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ascending"));
    }

    #[test]
    fn test_compiled_instrument_positive_year_fraction_validation() {
        let result: Result<CompiledInstrument<f64>, _> = CompiledInstrument::new(
            InstrumentType::Swap,
            0.03,
            1.0,
            vec![1.0],
            vec![-0.5], // Negative year fraction
            vec![1.0],
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_compiled_instrument_deposit_invalid_maturity() {
        let result: Result<CompiledInstrument<f64>, _> = CompiledInstrument::deposit(0.03, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_compiled_instrument_fra_invalid_dates() {
        // End before start
        let result: Result<CompiledInstrument<f64>, _> = CompiledInstrument::fra(0.03, 1.0, 0.5);
        assert!(result.is_err());

        // Negative start
        let result2: Result<CompiledInstrument<f64>, _> = CompiledInstrument::fra(0.03, -0.5, 0.5);
        assert!(result2.is_err());
    }

    // =========================================================================
    // CalibrationInstrument Implementation Tests (Requirements 3.1-3.4)
    // =========================================================================

    use crate::market::curves::{BootstrapInterpolation, BootstrappedCurve, FlatCurve};
    use crate::builder::CalibrationInstrument as CalibInstrument;
    use approx::assert_relative_eq;

    fn create_test_curve() -> BootstrappedCurve<f64> {
        let pillars = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
        let discount_factors: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
        BootstrappedCurve::new(
            pillars,
            discount_factors,
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap()
    }

    #[test]
    fn test_calibration_instrument_market_rate() {
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        assert_relative_eq!(CalibInstrument::market_rate(&inst), 0.03, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_maturity() {
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 2.5).unwrap();
        assert_relative_eq!(CalibInstrument::maturity(&inst), 2.5, epsilon = 1e-10);
    }

    #[test]
    fn test_calibration_instrument_type() {
        let deposit: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        assert_eq!(CalibInstrument::instrument_type(&deposit), "Deposit");

        let fra: CompiledInstrument<f64> = CompiledInstrument::fra(0.025, 0.5, 1.0).unwrap();
        assert_eq!(CalibInstrument::instrument_type(&fra), "FRA");
    }

    #[test]
    fn test_theoretical_rate_deposit() {
        let curve = create_test_curve();
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        let theoretical = inst.theoretical_rate(&curve).unwrap();

        // For a flat 3% curve: theoretical rate should be close to 3%
        assert_relative_eq!(theoretical, 0.03, epsilon = 5e-3);
    }

    #[test]
    fn test_theoretical_rate_fra() {
        let curve = create_test_curve();
        let inst: CompiledInstrument<f64> = CompiledInstrument::fra(0.03, 0.5, 1.0).unwrap();
        let theoretical = inst.theoretical_rate(&curve).unwrap();

        // FRA rate should be close to the flat curve rate
        assert_relative_eq!(theoretical, 0.03, epsilon = 5e-3);
    }

    #[test]
    fn test_pricing_error_computation() {
        let curve = create_test_curve();
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.03, 1.0).unwrap();
        let error = inst.pricing_error(&curve).unwrap();

        // Error should be small for a well-calibrated instrument
        assert!(error.abs() < 0.01, "expected small error, got {}", error);
    }

    #[test]
    fn test_theoretical_rate_flat_curve() {
        let curve = FlatCurve::new(0.05_f64);
        let inst: CompiledInstrument<f64> = CompiledInstrument::deposit(0.05, 1.0).unwrap();
        let theoretical = inst.theoretical_rate(&curve).unwrap();

        // For deposit: rate = (1/DF - 1) / T = (exp(r*T) - 1) / T
        // For small T and r: approx = r + r^2*T/2 + ...
        // At r=0.05, T=1: (exp(0.05) - 1) / 1 ≈ 0.05127
        assert_relative_eq!(theoretical, (0.05_f64.exp() - 1.0) / 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_swap_theoretical_rate() {
        let curve = FlatCurve::new(0.04_f64);

        // Create a 5-year swap with annual payments
        let inst: CompiledInstrument<f64> = CompiledInstrument::new(
            InstrumentType::Swap,
            0.04,
            5.0,
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
            Some(0.04),
        )
        .unwrap();

        let theoretical = inst.theoretical_rate(&curve).unwrap();

        // For a flat curve, par swap rate should be close to the zero rate
        assert_relative_eq!(theoretical, 0.04, epsilon = 5e-3);
    }

    // =========================================================================
    // InstrumentCompiler Tests (Requirements 3.1-3.5)
    // =========================================================================

    use infra_master::market::convention::{DepositConvention, FraConvention, SwapConvention};
    use infra_master::market::{Currency, RateId, RateType};
    use infra_master::time::Tenor;

    fn create_test_valuation_date() -> Date {
        Date::from_ymd(2024, 1, 15).unwrap()
    }

    #[test]
    fn test_instrument_compiler_new() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);
        assert_eq!(compiler.valuation_date(), valuation_date);
    }

    #[test]
    fn test_compile_deposit() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let convention = MarketConvention::Deposit(DepositConvention::usd());
        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.05,
            convention,
            valuation_date,
            1_000_000.0,
        )
        .unwrap();

        let compiled = compiler.compile(&instrument, 0).unwrap();
        assert_eq!(compiled.get_instrument_type(), InstrumentType::Deposit);
        assert!((CalibInstrument::market_rate(&compiled) - 0.05).abs() < 1e-10);
        assert!(compiled.cashflow_times().len() >= 1);
    }

    #[test]
    fn test_compile_swap() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let rate_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
        let convention = MarketConvention::Swap(SwapConvention::usd_sofr());
        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.045,
            convention,
            valuation_date,
            10_000_000.0,
        )
        .unwrap();

        let compiled = compiler.compile(&instrument, 0).unwrap();
        assert_eq!(compiled.get_instrument_type(), InstrumentType::Swap);
        assert!((CalibInstrument::market_rate(&compiled) - 0.045).abs() < 1e-10);
        assert!(compiled.cashflow_times().len() >= 1);
    }

    #[test]
    fn test_compile_ois() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let rate_id = RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois);
        let convention = MarketConvention::Ois(SwapConvention::usd_sofr());
        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.052,
            convention,
            valuation_date,
            5_000_000.0,
        )
        .unwrap();

        let compiled = compiler.compile(&instrument, 0).unwrap();
        assert_eq!(compiled.get_instrument_type(), InstrumentType::Ois);
    }

    #[test]
    fn test_compile_fra() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Fra);
        let convention = MarketConvention::Fra(FraConvention::usd_sofr());
        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.051,
            convention,
            valuation_date,
            2_000_000.0,
        )
        .unwrap();

        let compiled = compiler.compile(&instrument, 0).unwrap();
        assert_eq!(compiled.get_instrument_type(), InstrumentType::Fra);
    }

    #[test]
    fn test_compile_unsupported_xccy() {
        use infra_master::market::convention::XCcyBasisConvention;

        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let rate_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::BasisSwap);
        let convention = MarketConvention::XCcyBasis(XCcyBasisConvention::usd_jpy());
        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.0025,
            convention,
            valuation_date,
            100_000_000.0,
        )
        .unwrap();

        let result = compiler.compile(&instrument, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompileError::UnsupportedInstrument { instrument_type, .. } => {
                assert_eq!(instrument_type, "XCcyBasis");
            }
            _ => panic!("Expected UnsupportedInstrument error"),
        }
    }

    #[test]
    fn test_compile_batch() {
        let valuation_date = create_test_valuation_date();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(valuation_date);

        let instruments = vec![
            InfraMasterInstrument::new(
                RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit),
                0.05,
                MarketConvention::Deposit(DepositConvention::usd()),
                valuation_date,
                1_000_000.0,
            )
            .unwrap(),
            InfraMasterInstrument::new(
                RateId::new(Currency::USD, Tenor::OneYear, RateType::Ois),
                0.052,
                MarketConvention::Ois(SwapConvention::usd_sofr()),
                valuation_date,
                5_000_000.0,
            )
            .unwrap(),
        ];

        let compiled = compiler.compile_batch(&instruments).unwrap();
        assert_eq!(compiled.len(), 2);
        assert_eq!(compiled[0].get_instrument_type(), InstrumentType::Deposit);
        assert_eq!(compiled[1].get_instrument_type(), InstrumentType::Ois);
    }

    #[test]
    fn test_compile_invalid_maturity() {
        // Use a future valuation date to make the instrument's maturity in the past
        let future_valuation = Date::from_ymd(2025, 6, 15).unwrap();
        let compiler: InstrumentCompiler<f64> = InstrumentCompiler::new(future_valuation);

        // Create instrument with normal dates (maturity in 2024)
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let convention = MarketConvention::Deposit(DepositConvention::usd());
        let instrument_date = Date::from_ymd(2024, 1, 15).unwrap();

        let instrument = InfraMasterInstrument::new(
            rate_id,
            0.05,
            convention,
            instrument_date,
            1_000_000.0,
        )
        .unwrap();

        // The instrument's maturity (around 2024-04) is before the compiler's valuation date (2025-06)
        let result = compiler.compile(&instrument, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::InvalidMaturity { .. }));
    }
}

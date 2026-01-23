//! Instrument conversion adapter for curve bootstrapping.
//!
//! This module provides the `InstrumentAdapter` for converting `CurveDefinition`
//! and market rates into `BootstrapInstrument` vectors suitable for curve construction.
//!
//! # Architecture
//!
//! The adapter bridges between:
//! - `infra_master::trade::convention::SwapConvention` (source conventions)
//! - `BootstrapInstrument<T>` (target bootstrap instruments)
//!
//! # Examples
//!
//! ```
//! use pricer_models::market::calibration::bootstrapping::{
//!     InstrumentAdapter, CurveDefinition, InstrumentSpec, InstrumentTenor,
//!     BootstrapInstrument,
//! };
//! use infra_master::market::RateIndex;
//! use infra_master::trade::convention::SwapConvention;
//!
//! // Create a curve definition
//! let definition = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
//!     .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
//!     .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
//!     .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));
//!
//! // Market rates for each tenor (as decimals, e.g., 0.03 = 3%)
//! let rates = [
//!     (InstrumentTenor::OneYear, 0.035),
//!     (InstrumentTenor::TwoYears, 0.037),
//!     (InstrumentTenor::FiveYears, 0.040),
//! ];
//!
//! // Convert to bootstrap instruments
//! let instruments = InstrumentAdapter::convert(&definition, &rates).unwrap();
//! assert_eq!(instruments.len(), 3);
//! ```

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::definition::{CurveDefinition, CurveInstrumentType, InstrumentSpec, InstrumentTenor};
use super::engine_error::CurveEngineError;
use super::instrument::{BootstrapInstrument, Frequency};

use infra_master::time::Frequency as InfraMasterFrequency;
use infra_master::trade::convention::SwapConvention;

/// Adapter for converting curve definitions and rates to bootstrap instruments.
///
/// Provides static methods for converting `InstrumentSpec` specifications
/// combined with market rates into `BootstrapInstrument` instances.
///
/// # Type Parameters
///
/// All conversion methods are generic over `T: Float` to support both
/// `f64` for production use and `Dual` for automatic differentiation.
pub struct InstrumentAdapter;

impl InstrumentAdapter {
    /// Converts a curve definition and rates into bootstrap instruments.
    ///
    /// The rates array must have the same length as the instruments in the definition,
    /// with each rate corresponding to an instrument at the same index.
    ///
    /// # Arguments
    ///
    /// * `definition` - The curve definition specifying instrument types and tenors
    /// * `rates` - Array of (tenor, rate) pairs for each instrument
    ///
    /// # Returns
    ///
    /// A vector of `BootstrapInstrument<T>` sorted by maturity.
    ///
    /// # Errors
    ///
    /// Returns `CurveEngineError::Instrument` if rates don't match instruments,
    /// or `CurveEngineError::IncompleteInstrumentDefinition` for invalid specs.
    ///
    /// # Examples
    ///
    /// ```
    /// use pricer_models::market::calibration::bootstrapping::{
    ///     InstrumentAdapter, CurveDefinition, InstrumentSpec, InstrumentTenor,
    /// };
    /// use infra_master::market::RateIndex;
    /// use infra_master::trade::convention::SwapConvention;
    ///
    /// let definition = CurveDefinition::new("USD-SOFR", RateIndex::Sofr, SwapConvention::usd_sofr())
    ///     .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
    ///     .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));
    ///
    /// let rates = [
    ///     (InstrumentTenor::OneYear, 0.03),
    ///     (InstrumentTenor::FiveYears, 0.04),
    /// ];
    ///
    /// let instruments: Vec<_> = InstrumentAdapter::convert(&definition, &rates).unwrap();
    /// assert_eq!(instruments.len(), 2);
    /// ```
    pub fn convert<T: Float>(
        definition: &CurveDefinition,
        rates: &[(InstrumentTenor, T)],
    ) -> Result<Vec<BootstrapInstrument<T>>, CurveEngineError> {
        // Validate rates match instruments
        if rates.len() != definition.instruments().len() {
            return Err(CurveEngineError::instrument(
                "rates",
                format!(
                    "Expected {} rates for {} instruments, got {}",
                    definition.instruments().len(),
                    definition.index_key(),
                    rates.len()
                ),
            ));
        }

        // Get sorted instruments and their rates
        let sorted_specs = definition.sorted_instruments_owned();
        let mut sorted_rates: Vec<_> = rates.to_vec();
        sorted_rates.sort_by_key(|(tenor, _)| *tenor);

        // Verify tenors match
        for (spec, (rate_tenor, _)) in sorted_specs.iter().zip(sorted_rates.iter()) {
            if spec.tenor() != *rate_tenor {
                return Err(CurveEngineError::instrument(
                    spec.tenor().code(),
                    format!(
                        "Rate tenor {} doesn't match instrument tenor {}",
                        rate_tenor, spec.tenor()
                    ),
                ));
            }
        }

        // Convert each instrument
        let mut instruments = Vec::with_capacity(sorted_specs.len());
        for (spec, (_, rate)) in sorted_specs.iter().zip(sorted_rates.iter()) {
            let instrument = Self::convert_single(spec, *rate, definition.convention())?;
            instruments.push(instrument);
        }

        Ok(instruments)
    }

    /// Converts a single instrument specification to a bootstrap instrument.
    ///
    /// # Arguments
    ///
    /// * `spec` - The instrument specification
    /// * `rate` - The market rate for this instrument
    /// * `convention` - The swap convention to apply
    ///
    /// # Returns
    ///
    /// A `BootstrapInstrument<T>` for the given specification.
    fn convert_single<T: Float>(
        spec: &InstrumentSpec,
        rate: T,
        convention: &SwapConvention,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        let maturity: T = from_f64(spec.maturity_years());

        match spec.instrument_type() {
            CurveInstrumentType::Ois => {
                Self::create_ois(maturity, rate, convention)
            }
            CurveInstrumentType::Irs => {
                Self::create_irs(maturity, rate, convention)
            }
            CurveInstrumentType::Fra => {
                Self::create_fra(spec, rate)
            }
            CurveInstrumentType::Future => {
                Self::create_future(spec, rate)
            }
            CurveInstrumentType::Deposit => {
                Ok(Self::create_deposit(maturity, rate))
            }
        }
    }

    /// Creates an OIS bootstrap instrument.
    ///
    /// Applies the fixed leg convention from the swap convention to determine
    /// the payment frequency.
    ///
    /// # Arguments
    ///
    /// * `maturity` - Maturity in years
    /// * `rate` - OIS rate as decimal
    /// * `convention` - Swap convention for frequency
    ///
    /// # Requirements Trace
    ///
    /// - Requirement 3.1: Generate BootstrapInstrument from SwapConvention
    /// - Requirement 3.2: Apply OIS convention from SwapConvention
    pub fn create_ois<T: Float>(
        maturity: T,
        rate: T,
        convention: &SwapConvention,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        let frequency = Self::convert_frequency(convention.fixed_leg.payment_frequency)?;
        Ok(BootstrapInstrument::ois_with_frequency(maturity, rate, frequency))
    }

    /// Creates an IRS bootstrap instrument.
    ///
    /// Applies both fixed and float leg conventions from the swap convention.
    ///
    /// # Arguments
    ///
    /// * `maturity` - Maturity in years
    /// * `rate` - Fixed rate as decimal
    /// * `convention` - Swap convention for frequencies
    ///
    /// # Requirements Trace
    ///
    /// - Requirement 3.3: Apply Fixed Leg and Float Leg cashflow schedules
    pub fn create_irs<T: Float>(
        maturity: T,
        rate: T,
        convention: &SwapConvention,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        let fixed_frequency = Self::convert_frequency(convention.fixed_leg.payment_frequency)?;
        let float_frequency = Self::convert_frequency(convention.float_leg.payment_frequency)?;
        Ok(BootstrapInstrument::irs_with_frequencies(
            maturity,
            rate,
            fixed_frequency,
            float_frequency,
        ))
    }

    /// Creates a FRA bootstrap instrument.
    ///
    /// The FRA period is derived from the tenor specification. For standard tenors:
    /// - 3M → 0.0 to 0.25 years
    /// - 6M → 0.0 to 0.5 years
    /// - etc.
    ///
    /// # Arguments
    ///
    /// * `spec` - Instrument specification with tenor
    /// * `rate` - Forward rate as decimal
    ///
    /// # Requirements Trace
    ///
    /// - Requirement 3.4: Extract period (start, end) and rate from FRA definition
    pub fn create_fra<T: Float>(
        spec: &InstrumentSpec,
        rate: T,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        // For standard FRAs, we assume start at 0 and end at the tenor
        // More complex FRAs (e.g., 3x6 FRA) would need additional specification
        let end: T = from_f64(spec.maturity_years());
        let start: T = T::zero();

        Ok(BootstrapInstrument::fra(start, end, rate))
    }

    /// Creates a FRA bootstrap instrument with explicit start and end.
    ///
    /// Use this for non-standard FRAs like 3x6 (3 months to 6 months).
    ///
    /// # Arguments
    ///
    /// * `start_years` - Start time in years
    /// * `end_years` - End time in years
    /// * `rate` - Forward rate as decimal
    pub fn create_fra_explicit<T: Float>(
        start_years: T,
        end_years: T,
        rate: T,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        if start_years >= end_years {
            return Err(CurveEngineError::instrument(
                "FRA",
                format!(
                    "Start ({:?}) must be before end ({:?})",
                    start_years.to_f64(),
                    end_years.to_f64()
                ),
            ));
        }
        Ok(BootstrapInstrument::fra(start_years, end_years, rate))
    }

    /// Creates a Future bootstrap instrument.
    ///
    /// Applies convexity adjustment from the specification. If not provided,
    /// uses zero convexity adjustment.
    ///
    /// The rate is interpreted as the futures price (100 - implied rate).
    /// For a future at 97.50, the implied rate is 2.50%.
    ///
    /// # Arguments
    ///
    /// * `spec` - Instrument specification with tenor and convexity adjustment
    /// * `price` - Futures price (e.g., 97.50)
    ///
    /// # Requirements Trace
    ///
    /// - Requirement 3.5: Extract price and maturity, apply convexity adjustment
    pub fn create_future<T: Float>(
        spec: &InstrumentSpec,
        price: T,
    ) -> Result<BootstrapInstrument<T>, CurveEngineError> {
        let maturity: T = from_f64(spec.maturity_years());
        let convexity_adjustment: T = from_f64(spec.convexity_adjustment().unwrap_or(0.0));

        Ok(BootstrapInstrument::future(maturity, price, convexity_adjustment))
    }

    /// Creates a deposit bootstrap instrument.
    ///
    /// Deposits are treated as single-period OIS for bootstrapping purposes.
    fn create_deposit<T: Float>(maturity: T, rate: T) -> BootstrapInstrument<T> {
        // Deposits are modeled as single-period OIS
        BootstrapInstrument::ois(maturity, rate)
    }

    /// Converts infra_master Frequency to bootstrap Frequency.
    fn convert_frequency(
        freq: InfraMasterFrequency,
    ) -> Result<Frequency, CurveEngineError> {
        match freq {
            InfraMasterFrequency::Annual => Ok(Frequency::Annual),
            InfraMasterFrequency::SemiAnnual => Ok(Frequency::SemiAnnual),
            InfraMasterFrequency::Quarterly => Ok(Frequency::Quarterly),
            InfraMasterFrequency::Monthly => Ok(Frequency::Monthly),
            InfraMasterFrequency::Daily => Ok(Frequency::Daily),
            InfraMasterFrequency::Weekly => {
                // Weekly is not directly supported, treat as daily for approximation
                // This is rare in swap conventions
                Err(CurveEngineError::configuration(
                    "frequency",
                    "Weekly frequency is not supported for bootstrap instruments",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infra_master::market::RateIndex;

    // ========================================
    // OIS Conversion Tests
    // ========================================

    #[test]
    fn test_create_ois_basic() {
        let convention = SwapConvention::usd_sofr();
        let result = InstrumentAdapter::create_ois(5.0_f64, 0.03, &convention);

        assert!(result.is_ok());
        let instrument = result.unwrap();
        assert!(instrument.is_ois());
        assert!((instrument.maturity() - 5.0).abs() < 1e-10);
        assert!((instrument.rate() - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_create_ois_applies_convention_frequency() {
        let convention = SwapConvention::usd_sofr();
        let instrument: BootstrapInstrument<f64> =
            InstrumentAdapter::create_ois(2.0, 0.025, &convention).unwrap();

        // USD SOFR convention uses annual payments
        assert!(instrument.is_ois());
        if let BootstrapInstrument::Ois { payment_frequency, .. } = instrument {
            assert_eq!(payment_frequency, Frequency::Annual);
        } else {
            panic!("Expected OIS variant");
        }
    }

    #[test]
    fn test_create_ois_semiannual_frequency() {
        // EUR EURIBOR uses semi-annual float
        let convention = SwapConvention::eur_euribor_6m();
        let instrument: BootstrapInstrument<f64> =
            InstrumentAdapter::create_ois(5.0, 0.02, &convention).unwrap();

        // Fixed leg of EUR EURIBOR is annual
        if let BootstrapInstrument::Ois { payment_frequency, .. } = instrument {
            assert_eq!(payment_frequency, Frequency::Annual);
        }
    }

    // ========================================
    // IRS Conversion Tests
    // ========================================

    #[test]
    fn test_create_irs_basic() {
        let convention = SwapConvention::usd_sofr();
        let result = InstrumentAdapter::create_irs(10.0_f64, 0.04, &convention);

        assert!(result.is_ok());
        let instrument = result.unwrap();
        assert!(instrument.is_irs());
        assert!((instrument.maturity() - 10.0).abs() < 1e-10);
        assert!((instrument.rate() - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_create_irs_applies_convention_frequencies() {
        let convention = SwapConvention::eur_euribor_6m();
        let instrument: BootstrapInstrument<f64> =
            InstrumentAdapter::create_irs(5.0, 0.02, &convention).unwrap();

        if let BootstrapInstrument::Irs {
            fixed_frequency,
            float_frequency,
            ..
        } = instrument {
            assert_eq!(fixed_frequency, Frequency::Annual);
            assert_eq!(float_frequency, Frequency::SemiAnnual);
        } else {
            panic!("Expected IRS variant");
        }
    }

    // ========================================
    // FRA Conversion Tests
    // ========================================

    #[test]
    fn test_create_fra_basic() {
        let spec = InstrumentSpec::fra(InstrumentTenor::ThreeMonths);
        let result = InstrumentAdapter::create_fra::<f64>(&spec, 0.025);

        assert!(result.is_ok());
        let instrument = result.unwrap();
        assert!(instrument.is_fra());
        assert!((instrument.maturity() - 0.25).abs() < 1e-10);
        assert!((instrument.rate() - 0.025).abs() < 1e-10);
    }

    #[test]
    fn test_create_fra_explicit() {
        let result = InstrumentAdapter::create_fra_explicit::<f64>(0.25, 0.5, 0.03);

        assert!(result.is_ok());
        let instrument = result.unwrap();
        assert!(instrument.is_fra());
        assert!((instrument.start() - 0.25).abs() < 1e-10);
        assert!((instrument.maturity() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_create_fra_explicit_invalid_dates() {
        let result = InstrumentAdapter::create_fra_explicit::<f64>(0.5, 0.25, 0.03);
        assert!(result.is_err());

        if let Err(CurveEngineError::Instrument { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected Instrument error");
        }
    }

    // ========================================
    // Future Conversion Tests
    // ========================================

    #[test]
    fn test_create_future_basic() {
        let spec = InstrumentSpec::future(InstrumentTenor::ThreeMonths, 0.0001);
        let result = InstrumentAdapter::create_future::<f64>(&spec, 97.5);

        assert!(result.is_ok());
        let instrument = result.unwrap();
        assert!(instrument.is_future());
        assert!((instrument.maturity() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_create_future_with_convexity_adjustment() {
        let spec = InstrumentSpec::future(InstrumentTenor::SixMonths, 0.0005);
        let instrument: BootstrapInstrument<f64> =
            InstrumentAdapter::create_future(&spec, 97.0).unwrap();

        if let BootstrapInstrument::Future { convexity_adjustment, .. } = instrument {
            assert!((convexity_adjustment - 0.0005).abs() < 1e-10);
        } else {
            panic!("Expected Future variant");
        }
    }

    #[test]
    fn test_create_future_no_convexity() {
        let spec = InstrumentSpec::new(CurveInstrumentType::Future, InstrumentTenor::ThreeMonths);
        let instrument: BootstrapInstrument<f64> =
            InstrumentAdapter::create_future(&spec, 97.5).unwrap();

        if let BootstrapInstrument::Future { convexity_adjustment, .. } = instrument {
            assert!(convexity_adjustment.abs() < 1e-10);
        }
    }

    // ========================================
    // Full Conversion Tests
    // ========================================

    #[test]
    fn test_convert_basic() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears));

        let rates = [
            (InstrumentTenor::OneYear, 0.035_f64),
            (InstrumentTenor::TwoYears, 0.037),
            (InstrumentTenor::FiveYears, 0.040),
        ];

        let result = InstrumentAdapter::convert(&definition, &rates);
        assert!(result.is_ok());

        let instruments = result.unwrap();
        assert_eq!(instruments.len(), 3);

        // Should be sorted by maturity
        assert!((instruments[0].maturity() - 1.0).abs() < 1e-10);
        assert!((instruments[1].maturity() - 2.0).abs() < 1e-10);
        assert!((instruments[2].maturity() - 5.0).abs() < 1e-10);

        // Check rates
        assert!((instruments[0].rate() - 0.035).abs() < 1e-10);
        assert!((instruments[1].rate() - 0.037).abs() < 1e-10);
        assert!((instruments[2].rate() - 0.040).abs() < 1e-10);
    }

    #[test]
    fn test_convert_unsorted_input() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::FiveYears))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears));

        // Rates in same order as definition (unsorted)
        let rates = [
            (InstrumentTenor::FiveYears, 0.040_f64),
            (InstrumentTenor::OneYear, 0.035),
            (InstrumentTenor::TwoYears, 0.037),
        ];

        let instruments = InstrumentAdapter::convert(&definition, &rates).unwrap();

        // Should be sorted by maturity
        assert!((instruments[0].maturity() - 1.0).abs() < 1e-10);
        assert!((instruments[1].maturity() - 2.0).abs() < 1e-10);
        assert!((instruments[2].maturity() - 5.0).abs() < 1e-10);

        // Rates should follow sorted order
        assert!((instruments[0].rate() - 0.035).abs() < 1e-10);
        assert!((instruments[1].rate() - 0.037).abs() < 1e-10);
        assert!((instruments[2].rate() - 0.040).abs() < 1e-10);
    }

    #[test]
    fn test_convert_mismatched_rates_count() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears));

        let rates = [(InstrumentTenor::OneYear, 0.035_f64)]; // Only 1 rate for 2 instruments

        let result = InstrumentAdapter::convert(&definition, &rates);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_mismatched_tenors() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::TwoYears));

        let rates = [
            (InstrumentTenor::OneYear, 0.035_f64),
            (InstrumentTenor::ThreeYears, 0.038), // Wrong tenor!
        ];

        let result = InstrumentAdapter::convert(&definition, &rates);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_mixed_instruments() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::deposit(InstrumentTenor::OneMonth))
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear))
        .with_instrument(InstrumentSpec::irs(InstrumentTenor::FiveYears));

        let rates = [
            (InstrumentTenor::OneMonth, 0.030_f64),
            (InstrumentTenor::OneYear, 0.035),
            (InstrumentTenor::FiveYears, 0.040),
        ];

        let instruments = InstrumentAdapter::convert(&definition, &rates).unwrap();

        assert_eq!(instruments.len(), 3);
        assert!(instruments[0].is_ois()); // Deposit treated as OIS
        assert!(instruments[1].is_ois());
        assert!(instruments[2].is_irs());
    }

    // ========================================
    // Frequency Conversion Tests
    // ========================================

    #[test]
    fn test_convert_frequency_annual() {
        let freq = InstrumentAdapter::convert_frequency(InfraMasterFrequency::Annual).unwrap();
        assert_eq!(freq, Frequency::Annual);
    }

    #[test]
    fn test_convert_frequency_semiannual() {
        let freq = InstrumentAdapter::convert_frequency(InfraMasterFrequency::SemiAnnual).unwrap();
        assert_eq!(freq, Frequency::SemiAnnual);
    }

    #[test]
    fn test_convert_frequency_quarterly() {
        let freq = InstrumentAdapter::convert_frequency(InfraMasterFrequency::Quarterly).unwrap();
        assert_eq!(freq, Frequency::Quarterly);
    }

    #[test]
    fn test_convert_frequency_monthly() {
        let freq = InstrumentAdapter::convert_frequency(InfraMasterFrequency::Monthly).unwrap();
        assert_eq!(freq, Frequency::Monthly);
    }

    #[test]
    fn test_convert_frequency_daily() {
        let freq = InstrumentAdapter::convert_frequency(InfraMasterFrequency::Daily).unwrap();
        assert_eq!(freq, Frequency::Daily);
    }

    #[test]
    fn test_convert_frequency_weekly_unsupported() {
        let result = InstrumentAdapter::convert_frequency(InfraMasterFrequency::Weekly);
        assert!(result.is_err());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_convert_with_f32() {
        let definition = CurveDefinition::new(
            "USD-SOFR",
            RateIndex::Sofr,
            SwapConvention::usd_sofr(),
        )
        .with_instrument(InstrumentSpec::ois(InstrumentTenor::OneYear));

        let rates = [(InstrumentTenor::OneYear, 0.035_f32)];

        let result = InstrumentAdapter::convert::<f32>(&definition, &rates);
        assert!(result.is_ok());

        let instruments = result.unwrap();
        assert_eq!(instruments.len(), 1);
        assert!((instruments[0].rate() - 0.035_f32).abs() < 1e-5);
    }

    // ========================================
    // Default SOFR Definition Tests
    // ========================================

    #[test]
    fn test_convert_default_sofr_definition() {
        let definition = CurveDefinition::default_usd_sofr();

        // Create rates for all 12 standard tenors
        let rates: Vec<(InstrumentTenor, f64)> = definition
            .instruments()
            .iter()
            .enumerate()
            .map(|(i, spec)| (spec.tenor(), 0.03 + 0.002 * i as f64))
            .collect();

        let instruments = InstrumentAdapter::convert(&definition, &rates).unwrap();

        assert_eq!(instruments.len(), 12);

        // First instrument should be 1M OIS
        assert!(instruments[0].is_ois());
        assert!(instruments[0].maturity() < 0.1);

        // Last instrument should be 30Y OIS
        assert!(instruments[11].is_ois());
        assert!((instruments[11].maturity() - 30.0).abs() < 0.1);
    }
}

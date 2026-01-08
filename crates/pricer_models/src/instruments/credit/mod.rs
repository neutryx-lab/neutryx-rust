//! Credit derivative instruments.
//!
//! This module provides credit derivative instruments including:
//! - [`CreditDefaultSwap`]: Single-name CDS with protection and premium legs
//!
//! # Feature Flag
//!
//! This module is available when the `credit` feature is enabled.
//!
//! # Architecture
//!
//! Credit instruments follow the same enum dispatch pattern as other
//! asset classes for Enzyme AD compatibility. The [`CreditInstrument`]
//! enum wraps all credit derivative types.
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::credit::{CreditDefaultSwap, CreditInstrument, CdsDirection};
//! use pricer_models::instruments::InstrumentTrait;
//! use pricer_models::schedules::{ScheduleBuilder, Frequency};
//! use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
//!
//! let start = Date::from_ymd(2024, 3, 20).unwrap();
//! let end = Date::from_ymd(2029, 3, 20).unwrap();
//!
//! let schedule = ScheduleBuilder::new()
//!     .start(start)
//!     .end(end)
//!     .frequency(Frequency::Quarterly)
//!     .day_count(DayCountConvention::ActualActual360)
//!     .build()
//!     .unwrap();
//!
//! let cds = CreditDefaultSwap::new(
//!     "ACME Corp".to_string(),
//!     10_000_000.0,
//!     0.01,
//!     0.4,
//!     schedule,
//!     Currency::USD,
//!     CdsDirection::BuyProtection,
//! );
//!
//! // Wrap in CreditInstrument enum
//! let credit_instrument = CreditInstrument::Cds(cds);
//!
//! assert_eq!(credit_instrument.type_name(), "CreditDefaultSwap");
//! ```

mod cds;
mod pricing;
pub mod simulation;

pub use cds::{CdsDirection, CreditDefaultSwap};
pub use pricing::{CdsPricer, CdsPriceResult};
pub use simulation::{
    CreditMonteCarloSimulator, CreditPathResult, DefaultStatus, DefaultTimeSimulator,
};

use num_traits::Float;
use pricer_core::types::Currency;

use crate::instruments::traits::InstrumentTrait;

/// Credit derivative instrument enum for static dispatch.
///
/// Wraps all credit derivative types for Enzyme-compatible static dispatch
/// without trait objects or dynamic allocation.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Cds`: Credit Default Swap
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::credit::{CreditDefaultSwap, CreditInstrument, CdsDirection};
/// use pricer_models::schedules::{ScheduleBuilder, Frequency};
/// use pricer_core::types::{Currency, time::{Date, DayCountConvention}};
///
/// let start = Date::from_ymd(2024, 3, 20).unwrap();
/// let end = Date::from_ymd(2029, 3, 20).unwrap();
///
/// let schedule = ScheduleBuilder::new()
///     .start(start)
///     .end(end)
///     .frequency(Frequency::Quarterly)
///     .day_count(DayCountConvention::ActualActual360)
///     .build()
///     .unwrap();
///
/// let cds = CreditDefaultSwap::new(
///     "ACME Corp".to_string(),
///     10_000_000.0,
///     0.01,
///     0.4,
///     schedule,
///     Currency::USD,
///     CdsDirection::BuyProtection,
/// );
///
/// let instrument = CreditInstrument::Cds(cds);
/// assert!(instrument.is_cds());
/// ```
#[derive(Debug, Clone)]
pub enum CreditInstrument<T: Float> {
    /// Credit Default Swap.
    Cds(CreditDefaultSwap<T>),
}

impl<T: Float> CreditInstrument<T> {
    /// Compute the payoff at given spot price.
    ///
    /// Note: CDS payoff is not spot-based; returns zero.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            CreditInstrument::Cds(cds) => cds.payoff(spot),
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            CreditInstrument::Cds(cds) => cds.expiry(),
        }
    }

    /// Return the settlement currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            CreditInstrument::Cds(cds) => cds.currency(),
        }
    }

    /// Return whether this is a CDS.
    #[inline]
    pub fn is_cds(&self) -> bool {
        matches!(self, CreditInstrument::Cds(_))
    }

    /// Return a reference to the CDS if this is a Cds variant.
    pub fn as_cds(&self) -> Option<&CreditDefaultSwap<T>> {
        match self {
            CreditInstrument::Cds(cds) => Some(cds),
        }
    }
}

impl<T: Float> InstrumentTrait<T> for CreditInstrument<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.expiry()
    }

    #[inline]
    fn currency(&self) -> Currency {
        self.currency()
    }

    fn type_name(&self) -> &'static str {
        match self {
            CreditInstrument::Cds(_) => "CreditDefaultSwap",
        }
    }

    fn notional(&self) -> T {
        match self {
            CreditInstrument::Cds(cds) => cds.notional(),
        }
    }
}

impl<T: Float> From<CreditDefaultSwap<T>> for CreditInstrument<T> {
    fn from(cds: CreditDefaultSwap<T>) -> Self {
        CreditInstrument::Cds(cds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedules::{Frequency, ScheduleBuilder};
    use pricer_core::types::time::Date;

    fn create_test_cds() -> CreditDefaultSwap<f64> {
        let start = Date::from_ymd(2024, 3, 20).unwrap();
        let end = Date::from_ymd(2029, 3, 20).unwrap();

        let schedule = ScheduleBuilder::new()
            .start(start)
            .end(end)
            .frequency(Frequency::Quarterly)
            .day_count(pricer_core::types::time::DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        CreditDefaultSwap::new(
            "ACME Corp".to_string(),
            10_000_000.0,
            0.01,
            0.4,
            schedule,
            Currency::USD,
            CdsDirection::BuyProtection,
        )
    }

    #[test]
    fn test_credit_instrument_cds() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        assert!(instrument.is_cds());
        assert!(instrument.as_cds().is_some());
    }

    #[test]
    fn test_credit_instrument_payoff() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        // CDS payoff is not spot-based
        assert_eq!(instrument.payoff(100.0), 0.0);
    }

    #[test]
    fn test_credit_instrument_expiry() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        let expiry = instrument.expiry();
        assert!(expiry > 4.0 && expiry < 6.0);
    }

    #[test]
    fn test_credit_instrument_currency() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        assert_eq!(instrument.currency(), Currency::USD);
    }

    #[test]
    fn test_credit_instrument_type_name() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        assert_eq!(instrument.type_name(), "CreditDefaultSwap");
    }

    #[test]
    fn test_credit_instrument_notional() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);

        let notional = <CreditInstrument<f64> as InstrumentTrait<f64>>::notional(&instrument);
        assert!((notional - 10_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_credit_instrument_from_cds() {
        let cds = create_test_cds();
        let instrument: CreditInstrument<f64> = cds.into();

        assert!(instrument.is_cds());
    }

    #[test]
    fn test_credit_instrument_clone() {
        let cds = create_test_cds();
        let inst1 = CreditInstrument::Cds(cds);
        let inst2 = inst1.clone();

        assert!((inst1.expiry() - inst2.expiry()).abs() < 1e-10);
    }

    #[test]
    fn test_credit_instrument_debug() {
        let cds = create_test_cds();
        let instrument = CreditInstrument::Cds(cds);
        let debug_str = format!("{:?}", instrument);

        assert!(debug_str.contains("Cds"));
    }
}

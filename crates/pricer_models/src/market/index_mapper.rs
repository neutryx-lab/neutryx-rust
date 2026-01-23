//! Index to curve mapping utilities.
//!
//! This module provides traits and implementations for mapping
//! rate indices (e.g., SOFR, EURIBOR) to their corresponding
//! yield curve names used in pricing.

use infra_master::RateIndex;

use super::curves::CurveName;
use super::error::MarketDataError;

/// Trait for mapping rate indices to curve names.
///
/// This trait provides the interface for converting a `RateIndex`
/// to the corresponding `CurveName` used to retrieve yield curves
/// from a `CurveSet`.
///
/// # Implementors
///
/// The default implementation [`DefaultIndexCurveMapper`] provides
/// standard mappings for all supported rate indices.
///
/// # Example
///
/// ```
/// use pricer_models::market::{IndexCurveMapper, DefaultIndexCurveMapper};
/// use pricer_models::market::curves::CurveName;
/// use infra_master::RateIndex;
///
/// let mapper = DefaultIndexCurveMapper;
/// let curve_name = mapper.map_to_curve(RateIndex::Sofr).unwrap();
/// assert_eq!(curve_name, CurveName::Sofr);
/// ```
pub trait IndexCurveMapper {
    /// Maps a rate index to its corresponding curve name.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to map
    ///
    /// # Returns
    ///
    /// * `Ok(CurveName)` - The corresponding curve name
    /// * `Err(MarketDataError::UnsupportedIndex)` - If the index is not supported
    fn map_to_curve(&self, index: RateIndex) -> Result<CurveName, MarketDataError>;
}

/// Default implementation of `IndexCurveMapper`.
///
/// Provides standard mappings from rate indices to curve names:
///
/// | Rate Index | Curve Name |
/// |------------|------------|
/// | SOFR | Sofr |
/// | TONAR | Tonar |
/// | ESTR | Estr |
/// | SONIA | Sonia |
/// | SARON | Saron |
/// | EURIBOR 3M | Euribor |
/// | EURIBOR 6M | Euribor |
///
/// # Example
///
/// ```
/// use pricer_models::market::{IndexCurveMapper, DefaultIndexCurveMapper};
/// use pricer_models::market::curves::CurveName;
/// use infra_master::RateIndex;
///
/// let mapper = DefaultIndexCurveMapper;
///
/// // All EURIBOR tenors map to the same curve
/// assert_eq!(
///     mapper.map_to_curve(RateIndex::Euribor3M).unwrap(),
///     CurveName::Euribor
/// );
/// assert_eq!(
///     mapper.map_to_curve(RateIndex::Euribor6M).unwrap(),
///     CurveName::Euribor
/// );
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultIndexCurveMapper;

impl IndexCurveMapper for DefaultIndexCurveMapper {
    fn map_to_curve(&self, index: RateIndex) -> Result<CurveName, MarketDataError> {
        match index {
            RateIndex::Sofr => Ok(CurveName::Sofr),
            RateIndex::Tonar => Ok(CurveName::Tonar),
            RateIndex::Estr => Ok(CurveName::Estr),
            RateIndex::Sonia => Ok(CurveName::Sonia),
            RateIndex::Saron => Ok(CurveName::Saron),
            RateIndex::Euribor3M | RateIndex::Euribor6M => Ok(CurveName::Euribor),
            // Handle any future variants as unsupported
            _ => Err(MarketDataError::UnsupportedIndex {
                index: format!("{:?}", index),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sofr_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Sofr).unwrap(),
            CurveName::Sofr
        );
    }

    #[test]
    fn test_tonar_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Tonar).unwrap(),
            CurveName::Tonar
        );
    }

    #[test]
    fn test_sonia_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Sonia).unwrap(),
            CurveName::Sonia
        );
    }

    #[test]
    fn test_saron_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Saron).unwrap(),
            CurveName::Saron
        );
    }

    #[test]
    fn test_estr_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Estr).unwrap(),
            CurveName::Estr
        );
    }

    #[test]
    fn test_euribor_3m_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Euribor3M).unwrap(),
            CurveName::Euribor
        );
    }

    #[test]
    fn test_euribor_6m_mapping() {
        let mapper = DefaultIndexCurveMapper;
        assert_eq!(
            mapper.map_to_curve(RateIndex::Euribor6M).unwrap(),
            CurveName::Euribor
        );
    }

    #[test]
    fn test_all_rate_indices_covered() {
        let mapper = DefaultIndexCurveMapper;

        // Test all variants are mapped (none should fail)
        let indices = [
            RateIndex::Sofr,
            RateIndex::Tonar,
            RateIndex::Estr,
            RateIndex::Sonia,
            RateIndex::Saron,
            RateIndex::Euribor3M,
            RateIndex::Euribor6M,
        ];

        for index in indices {
            assert!(
                mapper.map_to_curve(index).is_ok(),
                "Index {:?} should be mapped",
                index
            );
        }
    }
}

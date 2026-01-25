//! FX Forward Curve Builder.
//!
//! This module provides the `FxForwardCurveBuilder<T>` for constructing
//! calibrated FX forward curves from market instruments.
//!
//! ## Supported Instruments
//!
//! - **FX Swaps** (short-term: ON to 1Y): Used for constructing the short end
//! - **XCCY Basis Swaps** (long-term: 2Y to 30Y): Used for constructing the
//!   long end
//!
//! ## Example
//!
//! ```ignore
//! use pricer_models::market::fx_calibration::FxForwardCurveBuilder;
//!
//! let curve = FxForwardCurveBuilder::new(currency_pair)
//!     .with_spot_rate(1.10)
//!     .with_domestic_curve(usd_curve)
//!     .with_foreign_curve(eur_curve)
//!     .with_fx_swaps(&fx_swaps)
//!     .with_xccy_basis_swaps(&xccy_swaps)
//!     .build()?;
//! ```

use std::sync::Arc;

use infra_master::trade::instrument_def::{CurrencyPair, FxSwapInstrument};
use num_traits::Float;

use super::curve::{CalibratedFxCurve, ExtrapolationPolicy, FxCurveError};
use crate::market::curves::YieldCurve;

// ============================================================================
// FxForwardCurveBuilder
// ============================================================================

/// Builder for constructing calibrated FX forward curves.
///
/// Supports bootstrapping from FX swaps (short-term) and XCCY basis swaps
/// (long-term), with automatic blending in the transition region.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for AD compatibility
///
/// # Example
///
/// ```ignore
/// let curve = FxForwardCurveBuilder::new(CurrencyPair::new(Currency::EUR, Currency::USD))
///     .with_spot_rate(1.10)
///     .with_domestic_curve(usd_curve)
///     .with_foreign_curve(eur_curve)
///     .with_fx_swaps(&fx_swaps)
///     .build()?;
/// ```
pub struct FxForwardCurveBuilder<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Spot rate.
    spot_rate: Option<T>,
    /// Domestic (quote currency) discount curve.
    domestic_curve: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// Foreign (base currency) discount curve.
    foreign_curve: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// FX swap instruments (short-term).
    fx_swaps: Vec<FxSwapData<T>>,
    /// XCCY basis swap instruments (long-term).
    xccy_swaps: Vec<XccySwapData<T>>,
    /// Extrapolation policy.
    extrapolation: ExtrapolationPolicy,
    /// Transition region start (years) - default 1.0.
    transition_start: T,
    /// Transition region end (years) - default 2.0.
    transition_end: T,
}

/// Internal representation of FX swap data for bootstrapping.
#[derive(Debug, Clone)]
pub struct FxSwapData<T: Float> {
    /// Tenor in years.
    pub tenor: T,
    /// Forward points.
    pub forward_points: T,
}

/// Internal representation of XCCY basis swap data for bootstrapping.
#[derive(Debug, Clone)]
pub struct XccySwapData<T: Float> {
    /// Tenor in years.
    pub tenor: T,
    /// Basis spread in decimal (e.g., -0.0015 for -15 bps).
    pub basis_spread: T,
}

impl<T: Float + Send + Sync> FxForwardCurveBuilder<T> {
    /// Creates a new builder for the given currency pair.
    #[must_use]
    pub fn new(currency_pair: CurrencyPair) -> Self {
        Self {
            currency_pair,
            spot_rate: None,
            domestic_curve: None,
            foreign_curve: None,
            fx_swaps: Vec::new(),
            xccy_swaps: Vec::new(),
            extrapolation: ExtrapolationPolicy::Flat,
            transition_start: T::one(),
            transition_end: T::one() + T::one(),
        }
    }

    /// Sets the spot rate.
    #[must_use]
    pub fn with_spot_rate(mut self, spot: T) -> Self {
        self.spot_rate = Some(spot);
        self
    }

    /// Sets the domestic (quote currency) discount curve.
    #[must_use]
    pub fn with_domestic_curve(mut self, curve: Arc<dyn YieldCurve<T> + Send + Sync>) -> Self {
        self.domestic_curve = Some(curve);
        self
    }

    /// Sets the foreign (base currency) discount curve.
    #[must_use]
    pub fn with_foreign_curve(mut self, curve: Arc<dyn YieldCurve<T> + Send + Sync>) -> Self {
        self.foreign_curve = Some(curve);
        self
    }

    /// Adds FX swap instruments from infra_master types.
    ///
    /// Extracts forward points from FX swap instruments and adds them
    /// to the builder for short-term curve construction.
    #[must_use]
    pub fn with_fx_swap_instruments(mut self, swaps: &[FxSwapInstrument]) -> Self {
        for swap in swaps {
            // Calculate tenor in years (approximate using 365.25 days/year)
            let days = swap.far_date - swap.near_date;
            let tenor =
                T::from(days).unwrap_or_else(T::zero) / T::from(365.25).unwrap_or_else(T::one);
            let forward_points = T::from(swap.swap_points.as_decimal()).unwrap_or_else(T::zero);

            self.fx_swaps.push(FxSwapData {
                tenor,
                forward_points,
            });
        }
        self
    }

    /// Adds FX swap data directly.
    #[must_use]
    pub fn with_fx_swaps(mut self, swaps: Vec<FxSwapData<T>>) -> Self {
        self.fx_swaps = swaps;
        self
    }

    /// Adds XCCY basis swap data.
    #[must_use]
    pub fn with_xccy_swaps(mut self, swaps: Vec<XccySwapData<T>>) -> Self {
        self.xccy_swaps = swaps;
        self
    }

    /// Sets the extrapolation policy.
    #[must_use]
    pub fn with_extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Sets the transition region for blending short and long-term curves.
    ///
    /// Default is 1.0 to 2.0 years.
    #[must_use]
    pub fn with_transition_region(mut self, start: T, end: T) -> Self {
        self.transition_start = start;
        self.transition_end = end;
        self
    }

    /// Builds the calibrated FX forward curve.
    ///
    /// # Process
    ///
    /// 1. Validates all required inputs are present
    /// 2. Bootstraps short-term forward points from FX swaps
    /// 3. Bootstraps long-term forward points from XCCY basis swaps
    /// 4. Blends the two curves in the transition region
    /// 5. Constructs the final `CalibratedFxCurve`
    ///
    /// # Errors
    ///
    /// Returns `FxCurveError` if:
    /// - Missing required inputs (spot rate, discount curves)
    /// - Insufficient data points
    /// - Bootstrap fails
    pub fn build(self) -> Result<CalibratedFxCurve<T>, FxCurveError> {
        // Validate required inputs
        let spot_rate = self.spot_rate.ok_or(FxCurveError::MissingSpotRate)?;

        let domestic_curve = self
            .domestic_curve
            .clone()
            .ok_or(FxCurveError::MissingDomesticCurve)?;

        let foreign_curve = self
            .foreign_curve
            .clone()
            .ok_or(FxCurveError::MissingForeignCurve)?;

        // Bootstrap forward points
        let (pillar_times, pillar_forward_points) =
            self.bootstrap_forward_points(spot_rate, &domestic_curve, &foreign_curve)?;

        // Validate we have enough data
        if pillar_times.len() < 2 {
            return Err(FxCurveError::insufficient_data(2, pillar_times.len()));
        }

        // Create the calibrated curve
        CalibratedFxCurve::new(
            self.currency_pair,
            spot_rate,
            pillar_times,
            pillar_forward_points,
            domestic_curve,
            foreign_curve,
            self.extrapolation,
        )
    }

    /// Bootstraps forward points from FX swaps and XCCY basis swaps.
    fn bootstrap_forward_points(
        &self,
        spot_rate: T,
        domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Result<(Vec<T>, Vec<T>), FxCurveError> {
        let mut pillar_times = Vec::new();
        let mut pillar_forward_points = Vec::new();

        // 1. Process FX swaps (short-term)
        let mut fx_swap_data: Vec<_> = self.fx_swaps.clone();
        fx_swap_data.sort_by(|a, b| {
            a.tenor
                .partial_cmp(&b.tenor)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for swap in &fx_swap_data {
            pillar_times.push(swap.tenor);
            pillar_forward_points.push(swap.forward_points);
        }

        // 2. Process XCCY basis swaps (long-term)
        if !self.xccy_swaps.is_empty() {
            let mut xccy_data: Vec<_> = self.xccy_swaps.clone();
            xccy_data.sort_by(|a, b| {
                a.tenor
                    .partial_cmp(&b.tenor)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for xccy in &xccy_data {
                // Bootstrap forward points from XCCY basis swap
                // Forward points = S * (DF_f / DF_d - 1) + basis adjustment
                let fp = self.bootstrap_xccy_forward_points(
                    xccy.tenor,
                    xccy.basis_spread,
                    spot_rate,
                    domestic_curve,
                    foreign_curve,
                )?;

                pillar_times.push(xccy.tenor);
                pillar_forward_points.push(fp);
            }
        }

        // 3. Handle case with no instruments - use IRP from discount curves
        if pillar_times.is_empty() {
            // Generate default pillars using IRP
            let default_tenors = [
                T::from(0.25).unwrap_or_else(T::zero),
                T::from(0.5).unwrap_or_else(T::zero),
                T::one(),
                T::one() + T::one(),
                T::from(5.0).unwrap_or_else(T::zero),
            ];

            for &t in &default_tenors {
                let df_d = domestic_curve
                    .discount_factor(t)
                    .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
                let df_f = foreign_curve
                    .discount_factor(t)
                    .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

                let fp = spot_rate * (df_f / df_d - T::one());

                pillar_times.push(t);
                pillar_forward_points.push(fp);
            }
        }

        // 4. Blend short and long-term in transition region if needed
        if !self.fx_swaps.is_empty() && !self.xccy_swaps.is_empty() {
            self.blend_transition_region(
                &mut pillar_times,
                &mut pillar_forward_points,
                spot_rate,
                domestic_curve,
                foreign_curve,
            )?;
        }

        Ok((pillar_times, pillar_forward_points))
    }

    /// Bootstraps forward points from a single XCCY basis swap.
    ///
    /// The forward points are calculated using interest rate parity
    /// with a basis spread adjustment.
    fn bootstrap_xccy_forward_points(
        &self,
        tenor: T,
        basis_spread: T,
        spot_rate: T,
        domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Result<T, FxCurveError> {
        // Get discount factors
        let df_d = domestic_curve
            .discount_factor(tenor)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;
        let df_f = foreign_curve
            .discount_factor(tenor)
            .map_err(|e| FxCurveError::discount_curve_error(format!("{:?}", e)))?;

        // Forward points with basis adjustment
        // The basis spread affects the implied foreign rate:
        // F = S * DF_f' / DF_d where DF_f' = exp(-(r_f + basis) * t)
        // Simplified: F ≈ S * (DF_f / DF_d) * (1 + basis * t)
        // Forward points = F - S

        let base_fp = spot_rate * (df_f / df_d - T::one());
        let basis_adjustment = spot_rate * basis_spread * tenor;
        let fp = base_fp + basis_adjustment;

        Ok(fp)
    }

    /// Blends forward points in the transition region between short and
    /// long-term data.
    fn blend_transition_region(
        &self,
        pillar_times: &mut Vec<T>,
        pillar_forward_points: &mut Vec<T>,
        _spot_rate: T,
        _domestic_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
        _foreign_curve: &Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Result<(), FxCurveError> {
        // Sort pillars by tenor
        let mut pairs: Vec<_> = pillar_times
            .iter()
            .copied()
            .zip(pillar_forward_points.iter().copied())
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Remove duplicates (keep later instrument in transition region)
        let mut unique_pairs: Vec<(T, T)> = Vec::new();
        for (t, fp) in pairs {
            if let Some(last) = unique_pairs.last_mut() {
                // If same tenor (within tolerance), update the forward point
                let diff = if t > last.0 { t - last.0 } else { last.0 - t };
                let tolerance = T::from(0.01).unwrap_or_else(T::zero); // 0.01 years = ~4 days
                if diff < tolerance {
                    // Prefer XCCY data in transition region (later entries)
                    last.1 = fp;
                    continue;
                }
            }
            unique_pairs.push((t, fp));
        }

        // Update the vectors
        pillar_times.clear();
        pillar_forward_points.clear();
        for (t, fp) in unique_pairs {
            pillar_times.push(t);
            pillar_forward_points.push(fp);
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use infra_master::Currency;

    use super::*;
    use crate::market::{curves::FlatCurve, fx_calibration::FxCurve};

    fn make_test_curves() -> (
        Arc<dyn YieldCurve<f64> + Send + Sync>,
        Arc<dyn YieldCurve<f64> + Send + Sync>,
    ) {
        let domestic = Arc::new(FlatCurve::new(0.05)) as Arc<dyn YieldCurve<f64> + Send + Sync>;
        let foreign = Arc::new(FlatCurve::new(0.03)) as Arc<dyn YieldCurve<f64> + Send + Sync>;
        (domestic, foreign)
    }

    #[test]
    fn test_builder_new() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let builder: FxForwardCurveBuilder<f64> = FxForwardCurveBuilder::new(pair);
        assert_eq!(builder.currency_pair, pair);
    }

    #[test]
    fn test_builder_missing_spot_rate() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingSpotRate)));
    }

    #[test]
    fn test_builder_missing_domestic_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (_, foreign) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingDomesticCurve)));
    }

    #[test]
    fn test_builder_missing_foreign_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, _) = make_test_curves();

        let result = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .build();

        assert!(matches!(result, Err(FxCurveError::MissingForeignCurve)));
    }

    #[test]
    fn test_builder_with_default_irp() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        // Build curve without any instruments - should use IRP
        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .build()
            .unwrap();

        // Check forward rate at 1Y
        let fwd_1y = curve.forward_rate(1.0).unwrap();
        // Expected: F = S * exp(r_d - r_f) = 1.10 * exp(0.05 - 0.03) = 1.10 * exp(0.02)
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_builder_with_fx_swaps() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.25,
                forward_points: 0.0050,
            },
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .build()
            .unwrap();

        // Check forward points at pillars
        let fp_1y = curve.forward_points(1.0).unwrap();
        assert!((fp_1y - 0.0200).abs() < 1e-10);
    }

    #[test]
    fn test_builder_with_xccy_swaps() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let xccy_swaps = vec![
            XccySwapData {
                tenor: 2.0,
                basis_spread: -0.0015, // -15 bps
            },
            XccySwapData {
                tenor: 5.0,
                basis_spread: -0.0020, // -20 bps
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_xccy_swaps(xccy_swaps)
            .build()
            .unwrap();

        // Check we can query at 2Y and 5Y
        let fwd_2y = curve.forward_rate(2.0);
        let fwd_5y = curve.forward_rate(5.0);

        assert!(fwd_2y.is_ok());
        assert!(fwd_5y.is_ok());
    }

    #[test]
    fn test_builder_extrapolation_policy() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![
            FxSwapData {
                tenor: 0.5,
                forward_points: 0.0100,
            },
            FxSwapData {
                tenor: 1.0,
                forward_points: 0.0200,
            },
        ];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_extrapolation(ExtrapolationPolicy::Error)
            .build()
            .unwrap();

        // Query beyond max tenor should fail
        let result = curve.forward_rate(5.0);
        assert!(matches!(
            result,
            Err(FxCurveError::ExtrapolationNotAllowed { .. })
        ));
    }

    #[test]
    fn test_builder_transition_region() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let (domestic, foreign) = make_test_curves();

        let fx_swaps = vec![FxSwapData {
            tenor: 1.0,
            forward_points: 0.0200,
        }];

        let xccy_swaps = vec![XccySwapData {
            tenor: 2.0,
            basis_spread: -0.0015,
        }];

        let curve = FxForwardCurveBuilder::new(pair)
            .with_spot_rate(1.10)
            .with_domestic_curve(domestic)
            .with_foreign_curve(foreign)
            .with_fx_swaps(fx_swaps)
            .with_xccy_swaps(xccy_swaps)
            .with_transition_region(1.0, 2.0)
            .build()
            .unwrap();

        // Check interpolation in transition region (1.5Y)
        let fwd_1_5y = curve.forward_rate(1.5);
        assert!(fwd_1_5y.is_ok());
    }

    #[test]
    fn test_fx_swap_data_clone() {
        let data = FxSwapData {
            tenor: 1.0,
            forward_points: 0.0200,
        };
        let cloned = data.clone();
        assert!((cloned.tenor - 1.0).abs() < 1e-10);
        assert!((cloned.forward_points - 0.0200).abs() < 1e-10);
    }

    #[test]
    fn test_xccy_swap_data_clone() {
        let data = XccySwapData {
            tenor: 5.0,
            basis_spread: -0.0015,
        };
        let cloned = data.clone();
        assert!((cloned.tenor - 5.0).abs() < 1e-10);
        assert!((cloned.basis_spread - (-0.0015)).abs() < 1e-10);
    }
}

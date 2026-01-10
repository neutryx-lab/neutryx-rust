//! Demo module for lazy-arc-pricing-kernel demonstration.
//!
//! This module provides simplified enum definitions for demonstrating
//! the 3-stage rocket pattern, lazy evaluation, and Arc caching architecture.
//!
//! # Components
//!
//! - `ModelEnum`: Stochastic models (BlackScholes, HullWhite) with evolve method
//! - `InstrumentEnum`: Instruments (VanillaSwap, CmsSwap) with requires_vol() method
//! - `CurveEnum`: Yield curves with get_df() method
//! - `VolSurfaceEnum`: Volatility surfaces for convexity adjustments
//!
//! # Design Notes
//!
//! This is a minimal implementation for architecture demonstration purposes.
//! Production code should use the full `models` and `instruments` modules.

// =============================================================================
// Models (Stage 1: State Evolution)
// =============================================================================

/// BlackScholes model with constant volatility parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackScholes {
    /// Volatility parameter (annualised).
    pub vol: f64,
}

/// Hull-White one-factor model with mean reversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullWhite {
    /// Mean reversion speed.
    pub mean_rev: f64,
    /// Volatility parameter.
    pub vol: f64,
}

/// Enum wrapping stochastic models for static dispatch.
///
/// This enum enables the 3-stage rocket pattern where models are defined
/// as pure data structures with no market data dependencies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelEnum {
    /// BlackScholes model.
    BlackScholes(BlackScholes),
    /// Hull-White interest rate model.
    HullWhite(HullWhite),
}

impl ModelEnum {
    /// Applies state evolution to the given state (simplified).
    ///
    /// For demonstration purposes, this applies a simple transformation.
    /// Production models would use proper stochastic calculus.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the current state value.
    pub fn evolve(&self, state: &mut f64) {
        match self {
            ModelEnum::BlackScholes(bs) => {
                // Simplified: multiply state by (1 + vol)
                *state *= 1.0 + bs.vol;
            }
            ModelEnum::HullWhite(hw) => {
                // Simplified: apply mean reversion towards 0.05
                let target = 0.05;
                *state += hw.mean_rev * (target - *state) + hw.vol;
            }
        }
    }
}

// =============================================================================
// Instruments (Stage 1: Payoffs)
// =============================================================================

/// Vanilla interest rate swap (Vol-independent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VanillaSwap {
    /// Fixed rate of the swap.
    pub fixed_rate: f64,
}

/// CMS (Constant Maturity Swap) requiring volatility for convexity adjustment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmsSwap {
    /// Fixed rate of the swap.
    pub fixed_rate: f64,
}

/// Enum wrapping instruments for static dispatch.
///
/// This enum enables compile-time determination of volatility dependencies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstrumentEnum {
    /// Vanilla swap (no volatility required).
    VanillaSwap(VanillaSwap),
    /// CMS swap (volatility required for convexity adjustment).
    CmsSwap(CmsSwap),
}

impl InstrumentEnum {
    /// Determines whether this instrument requires volatility for pricing.
    ///
    /// This method enables lazy evaluation - volatility surfaces are only
    /// constructed when actually needed.
    ///
    /// # Returns
    ///
    /// - `false` for `VanillaSwap`
    /// - `true` for `CmsSwap`
    pub fn requires_vol(&self) -> bool {
        match self {
            InstrumentEnum::VanillaSwap(_) => false,
            InstrumentEnum::CmsSwap(_) => true,
        }
    }

    /// Returns the fixed rate of the instrument.
    pub fn fixed_rate(&self) -> f64 {
        match self {
            InstrumentEnum::VanillaSwap(swap) => swap.fixed_rate,
            InstrumentEnum::CmsSwap(swap) => swap.fixed_rate,
        }
    }
}

// =============================================================================
// Market Objects (Stage 2: Market Data)
// =============================================================================

/// Flat yield curve with constant rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatCurve {
    /// Constant interest rate.
    pub rate: f64,
}

/// Enum wrapping yield curves for static dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveEnum {
    /// Flat curve with constant rate.
    Flat(FlatCurve),
}

impl CurveEnum {
    /// Computes the discount factor for time t.
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years (must be >= 0).
    ///
    /// # Returns
    ///
    /// Discount factor in range (0, 1].
    ///
    /// # Postconditions
    ///
    /// - Returns 1.0 when t = 0
    /// - Returns exp(-rate * t) for t > 0
    pub fn get_df(&self, t: f64) -> f64 {
        match self {
            CurveEnum::Flat(curve) => (-curve.rate * t).exp(),
        }
    }
}

/// SABR volatility surface (simplified).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrVolSurface {
    /// Alpha parameter (initial volatility).
    pub alpha: f64,
}

/// Enum wrapping volatility surfaces for static dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolSurfaceEnum {
    /// SABR volatility surface.
    Sabr(SabrVolSurface),
}

impl VolSurfaceEnum {
    /// Returns the convexity adjustment factor.
    ///
    /// This is a simplified implementation for demonstration purposes.
    /// Production code would compute proper SABR-based convexity adjustments.
    pub fn convexity_adjustment(&self) -> f64 {
        match self {
            VolSurfaceEnum::Sabr(sabr) => sabr.alpha * 0.01,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Task 1.1: ModelEnum Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_black_scholes_creation() {
        let bs = BlackScholes { vol: 0.2 };
        assert!((bs.vol - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_hull_white_creation() {
        let hw = HullWhite {
            mean_rev: 0.1,
            vol: 0.01,
        };
        assert!((hw.mean_rev - 0.1).abs() < 1e-10);
        assert!((hw.vol - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_model_enum_black_scholes() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        match model {
            ModelEnum::BlackScholes(bs) => assert!((bs.vol - 0.2).abs() < 1e-10),
            _ => panic!("Expected BlackScholes variant"),
        }
    }

    #[test]
    fn test_model_enum_hull_white() {
        let model = ModelEnum::HullWhite(HullWhite {
            mean_rev: 0.1,
            vol: 0.01,
        });
        match model {
            ModelEnum::HullWhite(hw) => {
                assert!((hw.mean_rev - 0.1).abs() < 1e-10);
                assert!((hw.vol - 0.01).abs() < 1e-10);
            }
            _ => panic!("Expected HullWhite variant"),
        }
    }

    #[test]
    fn test_model_evolve_black_scholes() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        let mut state = 100.0;
        model.evolve(&mut state);
        // Expected: 100 * (1 + 0.2) = 120
        assert!((state - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_model_evolve_hull_white() {
        let model = ModelEnum::HullWhite(HullWhite {
            mean_rev: 0.1,
            vol: 0.01,
        });
        let mut state = 0.03;
        model.evolve(&mut state);
        // Expected: 0.03 + 0.1 * (0.05 - 0.03) + 0.01 = 0.03 + 0.002 + 0.01 = 0.042
        assert!((state - 0.042).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // Task 1.2: InstrumentEnum Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_vanilla_swap_creation() {
        let swap = VanillaSwap { fixed_rate: 0.02 };
        assert!((swap.fixed_rate - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_cms_swap_creation() {
        let swap = CmsSwap { fixed_rate: 0.025 };
        assert!((swap.fixed_rate - 0.025).abs() < 1e-10);
    }

    #[test]
    fn test_instrument_enum_vanilla_swap() {
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        match instrument {
            InstrumentEnum::VanillaSwap(swap) => {
                assert!((swap.fixed_rate - 0.02).abs() < 1e-10)
            }
            _ => panic!("Expected VanillaSwap variant"),
        }
    }

    #[test]
    fn test_instrument_enum_cms_swap() {
        let instrument = InstrumentEnum::CmsSwap(CmsSwap { fixed_rate: 0.025 });
        match instrument {
            InstrumentEnum::CmsSwap(swap) => {
                assert!((swap.fixed_rate - 0.025).abs() < 1e-10)
            }
            _ => panic!("Expected CmsSwap variant"),
        }
    }

    #[test]
    fn test_vanilla_swap_requires_vol_false() {
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        assert!(!instrument.requires_vol());
    }

    #[test]
    fn test_cms_swap_requires_vol_true() {
        let instrument = InstrumentEnum::CmsSwap(CmsSwap { fixed_rate: 0.025 });
        assert!(instrument.requires_vol());
    }

    #[test]
    fn test_instrument_fixed_rate() {
        let vanilla = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        let cms = InstrumentEnum::CmsSwap(CmsSwap { fixed_rate: 0.025 });

        assert!((vanilla.fixed_rate() - 0.02).abs() < 1e-10);
        assert!((cms.fixed_rate() - 0.025).abs() < 1e-10);
    }

    // -------------------------------------------------------------------------
    // Task 1.3: Market Object Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_flat_curve_creation() {
        let curve = FlatCurve { rate: 0.05 };
        assert!((curve.rate - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_curve_enum_flat() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        match curve {
            CurveEnum::Flat(c) => assert!((c.rate - 0.05).abs() < 1e-10),
        }
    }

    #[test]
    fn test_curve_get_df_at_zero() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let df = curve.get_df(0.0);
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_curve_get_df_at_one_year() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let df = curve.get_df(1.0);
        // Expected: exp(-0.05 * 1) = exp(-0.05) ≈ 0.9512
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_curve_get_df_at_five_years() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let df = curve.get_df(5.0);
        // Expected: exp(-0.05 * 5) = exp(-0.25) ≈ 0.7788
        let expected = (-0.25_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_curve_get_df_in_valid_range() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        for t in [0.0, 0.5, 1.0, 2.0, 5.0, 10.0] {
            let df = curve.get_df(t);
            assert!(df > 0.0, "Discount factor must be positive");
            assert!(df <= 1.0, "Discount factor must be <= 1");
        }
    }

    #[test]
    fn test_sabr_vol_surface_creation() {
        let sabr = SabrVolSurface { alpha: 0.3 };
        assert!((sabr.alpha - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_vol_surface_enum_sabr() {
        let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });
        match vol {
            VolSurfaceEnum::Sabr(s) => assert!((s.alpha - 0.3).abs() < 1e-10),
        }
    }

    #[test]
    fn test_vol_surface_convexity_adjustment() {
        let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });
        let adj = vol.convexity_adjustment();
        // Expected: 0.3 * 0.01 = 0.003
        assert!((adj - 0.003).abs() < 1e-10);
    }
}

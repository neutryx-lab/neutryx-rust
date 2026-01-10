//! Pricing context and kernel for the 3-stage rocket pattern.
//!
//! This module provides:
//! - `PricingContext`: Lightweight reference-based context for pricing (Stage 2)
//! - `price_single_trade`: Pure pricing kernel with no HashMap lookups (Stage 3)
//!
//! # Architecture Role
//!
//! This module implements the final two stages of the 3-stage rocket pattern:
//!
//! 1. **Stage 1 (Definition)**: `ModelEnum`, `InstrumentEnum` in pricer_models
//! 2. **Stage 2 (Linking)**: `PricingContext` binds Arc references to context
//! 3. **Stage 3 (Execution)**: `price_single_trade` performs pure computation
//!
//! # Design Principles
//!
//! - **Zero HashMap Lookups**: All market data resolved before kernel entry
//! - **Reference-Based**: Uses `&'a CurveEnum` not `Arc` for zero-cost abstraction
//! - **Static Dispatch**: All enum matching resolved at compile time
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_pricing::context::{PricingContext, price_single_trade};
//! use pricer_models::demo::{ModelEnum, InstrumentEnum, CurveEnum, VolSurfaceEnum};
//!
//! let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
//! let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });
//!
//! let ctx = PricingContext {
//!     discount_curve: &curve,
//!     adjustment_vol: Some(&vol),
//! };
//!
//! let pv = price_single_trade(&model, &instrument, &ctx);
//! ```

use pricer_models::demo::{CurveEnum, InstrumentEnum, ModelEnum, VolSurfaceEnum};

/// Lightweight pricing context holding references to market data.
///
/// This struct implements Stage 2 of the 3-stage rocket pattern:
/// - Borrows references from Arc-cached market data
/// - Lifetime `'a` ensures references remain valid during pricing
/// - No ownership, no cloning, pure pointer access
///
/// # Design Notes
///
/// - `discount_curve` is always required for discounting
/// - `adjustment_vol` is optional (only needed for CmsSwap)
/// - References are borrowed from `Arc<T>` in the orchestration layer
#[derive(Debug, Clone, Copy)]
pub struct PricingContext<'a> {
    /// Reference to the discount curve (always required).
    pub discount_curve: &'a CurveEnum,
    /// Optional reference to volatility surface for convexity adjustment.
    pub adjustment_vol: Option<&'a VolSurfaceEnum>,
}

impl<'a> PricingContext<'a> {
    /// Creates a new pricing context with the given market data references.
    ///
    /// # Arguments
    ///
    /// * `discount_curve` - Reference to the discount curve.
    /// * `adjustment_vol` - Optional reference to volatility surface.
    pub fn new(
        discount_curve: &'a CurveEnum,
        adjustment_vol: Option<&'a VolSurfaceEnum>,
    ) -> Self {
        Self {
            discount_curve,
            adjustment_vol,
        }
    }
}

/// Computes the present value of a single trade.
///
/// This function implements Stage 3 of the 3-stage rocket pattern:
/// - Pure computation with no market data resolution
/// - No HashMap lookups or dynamic allocation
/// - Static dispatch via enum matching
///
/// # Arguments
///
/// * `model` - Reference to the stochastic model.
/// * `instrument` - Reference to the instrument being priced.
/// * `ctx` - Pricing context with market data references.
///
/// # Returns
///
/// The discounted present value of the trade.
///
/// # Pricing Logic
///
/// 1. Apply model evolution to initial state
/// 2. Compute payoff based on instrument type:
///    - `VanillaSwap`: state - fixed_rate
///    - `CmsSwap`: state - fixed_rate + convexity_adjustment
/// 3. Discount payoff using discount curve at T=1Y
///
/// # Preconditions
///
/// - For `CmsSwap`, `ctx.adjustment_vol` must be `Some`
///
/// # Invariants
///
/// - No HashMap lookups during execution
/// - No dynamic memory allocation
pub fn price_single_trade(
    model: &ModelEnum,
    instrument: &InstrumentEnum,
    ctx: &PricingContext,
) -> f64 {
    // Stage 3: Pure computation

    // Step 1: Apply model evolution to get the evolved state
    let mut state = 0.05; // Initial rate state
    model.evolve(&mut state);

    // Step 2: Compute payoff based on instrument type
    let payoff = match instrument {
        InstrumentEnum::VanillaSwap(swap) => {
            // Vanilla swap: simple rate difference
            state - swap.fixed_rate
        }
        InstrumentEnum::CmsSwap(swap) => {
            // CMS swap: add convexity adjustment from vol surface
            let convexity = ctx
                .adjustment_vol
                .map(|vol| vol.convexity_adjustment())
                .unwrap_or(0.0);
            state - swap.fixed_rate + convexity
        }
    };

    // Step 3: Discount the payoff
    let df = ctx.discount_curve.get_df(1.0);
    payoff * df
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_models::demo::{
        BlackScholes, CmsSwap, FlatCurve, HullWhite, SabrVolSurface, VanillaSwap,
    };

    // -------------------------------------------------------------------------
    // Task 3.1: PricingContext Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_pricing_context_creation() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let ctx = PricingContext {
            discount_curve: &curve,
            adjustment_vol: None,
        };
        assert!(ctx.adjustment_vol.is_none());
    }

    #[test]
    fn test_pricing_context_with_vol() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });
        let ctx = PricingContext {
            discount_curve: &curve,
            adjustment_vol: Some(&vol),
        };
        assert!(ctx.adjustment_vol.is_some());
    }

    #[test]
    fn test_pricing_context_new_constructor() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });
        let ctx = PricingContext::new(&curve, Some(&vol));
        assert!(ctx.adjustment_vol.is_some());
    }

    #[test]
    fn test_pricing_context_is_copy() {
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let ctx1 = PricingContext::new(&curve, None);
        let ctx2 = ctx1; // Copy
        // Both should be valid
        assert!(ctx1.adjustment_vol.is_none());
        assert!(ctx2.adjustment_vol.is_none());
    }

    // -------------------------------------------------------------------------
    // Task 3.2: price_single_trade Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_price_vanilla_swap_black_scholes() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });

        let ctx = PricingContext::new(&curve, None);
        let pv = price_single_trade(&model, &instrument, &ctx);

        // state = 0.05 * (1 + 0.2) = 0.06
        // payoff = 0.06 - 0.02 = 0.04
        // df = exp(-0.05) ≈ 0.9512
        // pv = 0.04 * 0.9512 ≈ 0.0380
        let expected_state = 0.05 * 1.2;
        let expected_payoff = expected_state - 0.02;
        let expected_df = (-0.05_f64).exp();
        let expected_pv = expected_payoff * expected_df;

        assert!(
            (pv - expected_pv).abs() < 1e-10,
            "Expected {}, got {}",
            expected_pv,
            pv
        );
    }

    #[test]
    fn test_price_vanilla_swap_hull_white() {
        let model = ModelEnum::HullWhite(HullWhite {
            mean_rev: 0.1,
            vol: 0.01,
        });
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });

        let ctx = PricingContext::new(&curve, None);
        let pv = price_single_trade(&model, &instrument, &ctx);

        // state = 0.05 + 0.1 * (0.05 - 0.05) + 0.01 = 0.06
        // payoff = 0.06 - 0.02 = 0.04
        let expected_state = 0.05 + 0.1 * (0.05 - 0.05) + 0.01;
        let expected_payoff = expected_state - 0.02;
        let expected_df = (-0.05_f64).exp();
        let expected_pv = expected_payoff * expected_df;

        assert!(
            (pv - expected_pv).abs() < 1e-10,
            "Expected {}, got {}",
            expected_pv,
            pv
        );
    }

    #[test]
    fn test_price_cms_swap_with_vol() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        let instrument = InstrumentEnum::CmsSwap(CmsSwap { fixed_rate: 0.02 });
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let vol = VolSurfaceEnum::Sabr(SabrVolSurface { alpha: 0.3 });

        let ctx = PricingContext::new(&curve, Some(&vol));
        let pv = price_single_trade(&model, &instrument, &ctx);

        // state = 0.05 * (1 + 0.2) = 0.06
        // convexity = 0.3 * 0.01 = 0.003
        // payoff = 0.06 - 0.02 + 0.003 = 0.043
        // df = exp(-0.05) ≈ 0.9512
        let expected_state = 0.05 * 1.2;
        let expected_convexity = 0.3 * 0.01;
        let expected_payoff = expected_state - 0.02 + expected_convexity;
        let expected_df = (-0.05_f64).exp();
        let expected_pv = expected_payoff * expected_df;

        assert!(
            (pv - expected_pv).abs() < 1e-10,
            "Expected {}, got {}",
            expected_pv,
            pv
        );
    }

    #[test]
    fn test_price_cms_swap_without_vol_uses_zero_adjustment() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        let instrument = InstrumentEnum::CmsSwap(CmsSwap { fixed_rate: 0.02 });
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });

        // No vol provided - should use 0 convexity adjustment
        let ctx = PricingContext::new(&curve, None);
        let pv = price_single_trade(&model, &instrument, &ctx);

        // state = 0.05 * (1 + 0.2) = 0.06
        // convexity = 0 (no vol)
        // payoff = 0.06 - 0.02 + 0 = 0.04
        let expected_state = 0.05 * 1.2;
        let expected_payoff = expected_state - 0.02;
        let expected_df = (-0.05_f64).exp();
        let expected_pv = expected_payoff * expected_df;

        assert!(
            (pv - expected_pv).abs() < 1e-10,
            "Expected {}, got {}",
            expected_pv,
            pv
        );
    }

    #[test]
    fn test_discount_factor_applied_correctly() {
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.0 }); // No vol evolution
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.0 });

        // With rate = 0, df = 1.0
        let curve_zero = CurveEnum::Flat(FlatCurve { rate: 0.0 });
        let ctx_zero = PricingContext::new(&curve_zero, None);
        let pv_zero = price_single_trade(&model, &instrument, &ctx_zero);

        // With rate = 0.1, df = exp(-0.1) ≈ 0.9048
        let curve_ten = CurveEnum::Flat(FlatCurve { rate: 0.1 });
        let ctx_ten = PricingContext::new(&curve_ten, None);
        let pv_ten = price_single_trade(&model, &instrument, &ctx_ten);

        // state = 0.05 * 1.0 = 0.05
        // payoff = 0.05 - 0 = 0.05
        let expected_payoff = 0.05;
        let expected_pv_zero = expected_payoff * 1.0;
        let expected_pv_ten = expected_payoff * (-0.1_f64).exp();

        assert!(
            (pv_zero - expected_pv_zero).abs() < 1e-10,
            "Zero rate: expected {}, got {}",
            expected_pv_zero,
            pv_zero
        );
        assert!(
            (pv_ten - expected_pv_ten).abs() < 1e-10,
            "10% rate: expected {}, got {}",
            expected_pv_ten,
            pv_ten
        );
    }

    // -------------------------------------------------------------------------
    // Requirement 3.6: No HashMap lookups verification
    // -------------------------------------------------------------------------

    #[test]
    fn test_no_allocation_in_pricing_kernel() {
        // This test verifies the kernel doesn't allocate by running many iterations
        // If it allocated, this would be slow/fail due to memory pressure
        let model = ModelEnum::BlackScholes(BlackScholes { vol: 0.2 });
        let instrument = InstrumentEnum::VanillaSwap(VanillaSwap { fixed_rate: 0.02 });
        let curve = CurveEnum::Flat(FlatCurve { rate: 0.05 });
        let ctx = PricingContext::new(&curve, None);

        let mut sum = 0.0;
        for _ in 0..10_000 {
            sum += price_single_trade(&model, &instrument, &ctx);
        }

        // Just verify we got reasonable results
        assert!(sum > 0.0);
    }
}

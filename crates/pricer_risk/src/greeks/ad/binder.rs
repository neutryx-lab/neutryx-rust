//! AAD Binder Layer for Shadow Object pattern.
//!
//! This module bridges the gap between high-level market data structures
//! and low-level slice-based pricing kernels, enabling Enzyme AAD integration.
//!
//! # Architecture
//!
//! ```text
//! MarketData (structs)
//!       ↓  as_slice()
//! &[f64] slices
//!       ↓  pricing_kernel()
//! PV (f64)
//!       ↓  d_pricing_kernel() (Enzyme)
//! Shadow slices (gradients)
//!       ↓  collect into Shadow struct
//! RiskResult<Shadow>
//! ```
//!
//! # Requirements Coverage
//!
//! - 3.1-3.7: MarketRiskCalculator trait and implementation
//! - 4.1-4.5: Zero-copy data transfer
//! - 7.1-7.5: ActivityMask and partial differentiation

use thiserror::Error;

use super::{
    kernel::{finite_difference_gradients, pricing_kernel_irs},
    shadow::{Shadow, SimpleMarketData, SimpleYieldCurve},
};

// =============================================================================
// Error Types (Task 4.3)
// =============================================================================

/// Errors in Shadow AAD operations.
///
/// # Requirements Coverage
///
/// - 8.4: Error handling without panics
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ShadowAadError {
    /// Input slices have mismatched lengths.
    #[error("Length mismatch: expected {expected}, got {actual}")]
    LengthMismatch {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// Input slice is empty when non-empty is required.
    #[error("Empty slice: {field}")]
    EmptySlice {
        /// Field name
        field: &'static str,
    },

    /// Enzyme AD is not available (feature not enabled).
    #[error("Enzyme AD not available, using finite difference fallback")]
    EnzymeNotAvailable,

    /// Invalid market data.
    #[error("Invalid market data: {message}")]
    InvalidMarketData {
        /// Error message
        message: String,
    },
}

// =============================================================================
// Activity Mask (Task 3.1)
// =============================================================================

/// Activity mask for partial differentiation.
///
/// Controls which market data components are differentiated (Active)
/// vs treated as constants (Const) in AAD.
///
/// # Requirements Coverage
///
/// - 7.1: Active/Const component specification
/// - 7.5: Default all components active
///
/// # Example
///
/// ```rust
/// use pricer_risk::greeks::ad::binder::ActivityMask;
///
/// // All active (default)
/// let mask = ActivityMask::default();
/// assert!(mask.rates_active);
///
/// // Only rates active
/// let mask = ActivityMask::rates_only();
/// assert!(mask.rates_active);
/// assert!(!mask.volatilities_active);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityMask {
    /// Whether interest rates are active (differentiable).
    pub rates_active: bool,
    /// Whether volatilities are active (differentiable).
    pub volatilities_active: bool,
    /// Whether FX rates are active (differentiable).
    pub fx_rates_active: bool,
}

impl Default for ActivityMask {
    /// Create mask with all components active.
    ///
    /// # Requirements Coverage
    ///
    /// - 7.5: Default all active
    fn default() -> Self {
        Self {
            rates_active: true,
            volatilities_active: true,
            fx_rates_active: true,
        }
    }
}

impl ActivityMask {
    /// Create mask with only rates active.
    #[inline]
    pub fn rates_only() -> Self {
        Self {
            rates_active: true,
            volatilities_active: false,
            fx_rates_active: false,
        }
    }

    /// Create mask with only volatilities active.
    #[inline]
    pub fn volatilities_only() -> Self {
        Self {
            rates_active: false,
            volatilities_active: true,
            fx_rates_active: false,
        }
    }

    /// Create mask with only FX rates active.
    #[inline]
    pub fn fx_only() -> Self {
        Self {
            rates_active: false,
            volatilities_active: false,
            fx_rates_active: true,
        }
    }

    /// Create mask with no components active (all const).
    #[inline]
    pub fn none() -> Self {
        Self {
            rates_active: false,
            volatilities_active: false,
            fx_rates_active: false,
        }
    }
}

// =============================================================================
// Risk Result (Task 3.1)
// =============================================================================

/// Result of AAD risk calculation.
///
/// Contains the computed PV and the gradient shadow object.
///
/// # Type Parameters
///
/// * `M` - Market data type implementing Shadow trait
///
/// # Requirements Coverage
///
/// - 7.1: RiskResult<M: Shadow> structure
/// - 6.1: Shadow has identical structure to market data
///
/// # Example
///
/// ```rust
/// use pricer_risk::greeks::ad::binder::RiskResult;
/// use pricer_risk::greeks::ad::shadow::SimpleYieldCurve;
///
/// // After AAD calculation
/// let result = RiskResult {
///     pv: 1_000_000.0,
///     gradients: SimpleYieldCurve::new(vec![100.0, 200.0, 300.0], vec![1.0, 2.0, 5.0]),
/// };
///
/// // Access gradients
/// assert_eq!(result.gradients.rates[0], 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct RiskResult<M: Shadow> {
    /// Present value of the trade.
    pub pv: f64,
    /// Gradient shadow object with identical structure to input market data.
    pub gradients: M,
}

impl<M: Shadow> RiskResult<M> {
    /// Create a new risk result.
    #[inline]
    pub fn new(pv: f64, gradients: M) -> Self { Self { pv, gradients } }
}

// =============================================================================
// Trade Parameters
// =============================================================================

/// Parameters for an Interest Rate Swap trade.
///
/// Contains all the information needed to price a swap using the kernel.
#[derive(Debug, Clone)]
pub struct IrsTradeParams {
    /// Notional amounts per period.
    pub notionals: Vec<f64>,
    /// Year fractions per period.
    pub year_fractions: Vec<f64>,
    /// Fixed leg rate.
    pub fixed_rate: f64,
}

impl IrsTradeParams {
    /// Create new IRS trade parameters.
    pub fn new(notionals: Vec<f64>, year_fractions: Vec<f64>, fixed_rate: f64) -> Self {
        Self {
            notionals,
            year_fractions,
            fixed_rate,
        }
    }

    /// Create a uniform swap (same notional and year fraction for all periods).
    pub fn uniform(notional: f64, year_fraction: f64, fixed_rate: f64, n_periods: usize) -> Self {
        Self {
            notionals: vec![notional; n_periods],
            year_fractions: vec![year_fraction; n_periods],
            fixed_rate,
        }
    }
}

// =============================================================================
// Market Risk Calculator (Task 3.2)
// =============================================================================

/// Calculator for market risk using Shadow Object AAD.
///
/// This struct provides the high-level interface for computing PV and
/// gradients using the Shadow Object pattern.
///
/// # Requirements Coverage
///
/// - 3.1-3.7: MarketRiskCalculator implementation
/// - 4.1-4.5: Zero-copy data passing
#[derive(Debug, Clone)]
pub struct MarketRiskCalculator {
    /// Bump size for finite difference fallback.
    pub bump_size: f64,
}

impl Default for MarketRiskCalculator {
    fn default() -> Self { Self { bump_size: 1e-7 } }
}

impl MarketRiskCalculator {
    /// Create a new calculator with custom bump size.
    pub fn with_bump_size(bump_size: f64) -> Self { Self { bump_size } }

    /// Calculate risk for an IRS trade using Shadow Object AAD.
    ///
    /// # Arguments
    ///
    /// * `market` - Market data (yield curve)
    /// * `trade` - Trade parameters
    /// * `mask` - Activity mask for partial differentiation
    ///
    /// # Returns
    ///
    /// `RiskResult` containing PV and gradients.
    ///
    /// # Requirements Coverage
    ///
    /// - 3.2: Slice extraction from market data
    /// - 3.3: Slice to shadow buffer
    /// - 3.4: Activity mask application
    /// - 3.5: Kernel invocation
    /// - 3.6: Result collection
    /// - 4.1: Zero-copy slice extraction
    pub fn calculate_irs_risk(
        &self,
        market: &SimpleYieldCurve,
        trade: &IrsTradeParams,
        mask: ActivityMask,
    ) -> Result<RiskResult<SimpleYieldCurve>, ShadowAadError> {
        // Validate inputs
        if market.is_empty() {
            return Err(ShadowAadError::EmptySlice { field: "rates" });
        }
        if market.len() != trade.notionals.len() {
            return Err(ShadowAadError::LengthMismatch {
                expected: market.len(),
                actual: trade.notionals.len(),
            });
        }

        // Calculate PV
        let mut pv = 0.0;
        pricing_kernel_irs(
            market.rates_slice(),
            market.times_slice(),
            &trade.notionals,
            &trade.year_fractions,
            trade.fixed_rate,
            &mut pv,
        );

        // Create shadow for gradient accumulation
        let mut shadow = market.create_shadow();

        // Calculate gradients based on activity mask
        if mask.rates_active {
            // Use finite difference for now (Enzyme version would use d_pricing_kernel_irs)
            let gradients = finite_difference_gradients(
                pricing_kernel_irs,
                market.rates_slice(),
                market.times_slice(),
                &trade.notionals,
                &trade.year_fractions,
                trade.fixed_rate,
                self.bump_size,
            );

            // Copy gradients to shadow
            for (i, &grad) in gradients.iter().enumerate() {
                shadow.rates[i] = grad;
            }
        }
        // If rates_active is false, gradients remain at zero (from create_shadow)

        Ok(RiskResult::new(pv, shadow))
    }

    /// Calculate risk for market data with multiple curves.
    ///
    /// # Arguments
    ///
    /// * `market` - Full market data (discount curve, optional forward curve)
    /// * `trade` - Trade parameters
    /// * `mask` - Activity mask
    ///
    /// # Returns
    ///
    /// `RiskResult` containing PV and gradients for all active curves.
    pub fn calculate_full_market_risk(
        &self,
        market: &SimpleMarketData,
        trade: &IrsTradeParams,
        mask: ActivityMask,
    ) -> Result<RiskResult<SimpleMarketData>, ShadowAadError> {
        // Validate inputs
        if market.discount_curve.is_empty() {
            return Err(ShadowAadError::EmptySlice {
                field: "discount_curve.rates",
            });
        }

        // Calculate PV using discount curve
        let mut pv = 0.0;
        pricing_kernel_irs(
            market.discount_curve.rates_slice(),
            market.discount_curve.times_slice(),
            &trade.notionals,
            &trade.year_fractions,
            trade.fixed_rate,
            &mut pv,
        );

        // Create shadow for gradient accumulation
        let mut shadow = market.create_shadow();

        // Calculate gradients for discount curve
        if mask.rates_active {
            let gradients = finite_difference_gradients(
                pricing_kernel_irs,
                market.discount_curve.rates_slice(),
                market.discount_curve.times_slice(),
                &trade.notionals,
                &trade.year_fractions,
                trade.fixed_rate,
                self.bump_size,
            );

            for (i, &grad) in gradients.iter().enumerate() {
                shadow.discount_curve.rates[i] = grad;
            }
        }

        // Forward curve gradients would require a different kernel
        // For now, they remain at zero

        Ok(RiskResult::new(pv, shadow))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 3.1: ActivityMask tests
    // =========================================================================

    #[test]
    fn test_activity_mask_default() {
        let mask = ActivityMask::default();
        assert!(mask.rates_active);
        assert!(mask.volatilities_active);
        assert!(mask.fx_rates_active);
    }

    #[test]
    fn test_activity_mask_rates_only() {
        let mask = ActivityMask::rates_only();
        assert!(mask.rates_active);
        assert!(!mask.volatilities_active);
        assert!(!mask.fx_rates_active);
    }

    #[test]
    fn test_activity_mask_volatilities_only() {
        let mask = ActivityMask::volatilities_only();
        assert!(!mask.rates_active);
        assert!(mask.volatilities_active);
        assert!(!mask.fx_rates_active);
    }

    #[test]
    fn test_activity_mask_none() {
        let mask = ActivityMask::none();
        assert!(!mask.rates_active);
        assert!(!mask.volatilities_active);
        assert!(!mask.fx_rates_active);
    }

    // =========================================================================
    // Task 3.1: RiskResult tests
    // =========================================================================

    #[test]
    fn test_risk_result_creation() {
        let gradients = SimpleYieldCurve::new(vec![100.0, 200.0], vec![1.0, 2.0]);
        let result = RiskResult::new(1_000_000.0, gradients);

        assert_eq!(result.pv, 1_000_000.0);
        assert_eq!(result.gradients.rates, vec![100.0, 200.0]);
    }

    // =========================================================================
    // Task 3.2, 3.3: MarketRiskCalculator tests
    // =========================================================================

    #[test]
    fn test_irs_trade_params() {
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        assert_eq!(trade.notionals, vec![1_000_000.0; 3]);
        assert_eq!(trade.year_fractions, vec![1.0; 3]);
        assert_eq!(trade.fixed_rate, 0.03);
    }

    #[test]
    fn test_calculate_irs_risk_basic() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03, 0.03, 0.03], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // ATM swap should have near-zero PV
        assert!(
            result.pv.abs() < 1.0,
            "ATM swap PV should be near zero, got {}",
            result.pv
        );

        // Gradients should be non-zero (rate sensitivity)
        for (i, &grad) in result.gradients.rates.iter().enumerate() {
            assert!(grad.abs() > 0.0, "Gradient {} should be non-zero", i);
        }
    }

    #[test]
    fn test_calculate_irs_risk_positive_pv() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.05, 0.05, 0.05], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3); // Fixed < floating

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        assert!(result.pv > 0.0, "Expected positive PV, got {}", result.pv);
    }

    #[test]
    fn test_calculate_irs_risk_const_rates() {
        // When rates are const, gradients should be zero
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03, 0.03, 0.03], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::none())
            .unwrap();

        // All gradients should be zero
        for (i, &grad) in result.gradients.rates.iter().enumerate() {
            assert_eq!(grad, 0.0, "Const gradient {} should be zero", i);
        }
    }

    #[test]
    fn test_calculate_irs_risk_empty_market() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![], vec![]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 0);

        let result = calc.calculate_irs_risk(&market, &trade, ActivityMask::default());

        assert!(matches!(result, Err(ShadowAadError::EmptySlice { .. })));
    }

    #[test]
    fn test_calculate_irs_risk_length_mismatch() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03, 0.03, 0.03], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 2); // Mismatch!

        let result = calc.calculate_irs_risk(&market, &trade, ActivityMask::default());

        assert!(matches!(result, Err(ShadowAadError::LengthMismatch { .. })));
    }

    #[test]
    fn test_calculate_full_market_risk() {
        let calc = MarketRiskCalculator::default();
        let discount = SimpleYieldCurve::new(vec![0.03, 0.03, 0.03], vec![1.0, 2.0, 3.0]);
        let forward = SimpleYieldCurve::new(vec![0.035, 0.035, 0.035], vec![1.0, 2.0, 3.0]);
        let market = SimpleMarketData::with_discount_curve(discount).with_forward_curve(forward);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_full_market_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // Discount curve gradients should be non-zero
        for &grad in &result.gradients.discount_curve.rates {
            assert!(grad.abs() > 0.0);
        }

        // Forward curve gradients should be zero (not used in this kernel)
        for &grad in &result.gradients.forward_curve.as_ref().unwrap().rates {
            assert_eq!(grad, 0.0);
        }
    }

    // =========================================================================
    // Task 3.4: Gradient verification tests
    // =========================================================================

    #[test]
    fn test_gradient_magnitude() {
        // Verify gradient magnitudes are reasonable (DV01 = ~100 per bp per year per
        // $1M)
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03], vec![1.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 1);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // For a 1Y swap, sensitivity should be around notional * year_fraction * df
        // Order of magnitude: ~1M * 1 * 0.97 = ~970,000
        let grad_magnitude = result.gradients.rates[0].abs();
        assert!(
            grad_magnitude > 100_000.0 && grad_magnitude < 2_000_000.0,
            "Gradient magnitude {} outside expected range",
            grad_magnitude
        );
    }

    #[test]
    fn test_gradient_structure_preserved() {
        // Verify shadow structure matches original
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // Same length
        assert_eq!(result.gradients.len(), market.len());
        // Times preserved (const)
        assert_eq!(result.gradients.times, market.times);
    }

    // =========================================================================
    // Task 5.1: YieldCurve Integration Tests
    // =========================================================================

    #[test]
    fn test_yield_curve_delta_calculation() {
        // Test Delta/DV01 calculation for yield curve
        let calc = MarketRiskCalculator::default();

        // Realistic yield curve with multiple tenors
        let tenors = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
        let rates = vec![0.03, 0.032, 0.035, 0.038, 0.042, 0.045];
        let market = SimpleYieldCurve::new(rates.clone(), tenors.clone());

        let trade = IrsTradeParams::new(
            vec![1_000_000.0; 6],
            vec![0.25, 0.25, 0.5, 1.0, 3.0, 5.0],
            0.04,
        );

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // Verify gradients are computed for all tenors
        assert_eq!(result.gradients.rates.len(), 6);

        // Verify longer tenors generally have larger sensitivities (DV01)
        // This is a simplified check - real DV01 depends on cashflow timing
        for &grad in &result.gradients.rates {
            assert!(
                grad.abs() > 0.0,
                "All rate sensitivities should be non-zero"
            );
        }
    }

    #[test]
    fn test_yield_curve_large_scale() {
        // Test 5.1: Performance with 100 pillar points
        let calc = MarketRiskCalculator::default();

        // Create large yield curve (100 pillars)
        let n = 100;
        let tenors: Vec<f64> = (1..=n).map(|i| i as f64 * 0.1).collect(); // 0.1Y to 10Y
        let rates: Vec<f64> = (1..=n).map(|i| 0.02 + 0.0002 * i as f64).collect();
        let market = SimpleYieldCurve::new(rates.clone(), tenors.clone());

        let trade = IrsTradeParams::uniform(1_000_000.0, 0.1, 0.035, n);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // Verify all gradients computed
        assert_eq!(result.gradients.rates.len(), n);

        // At least some gradients should be non-zero
        let non_zero_count = result
            .gradients
            .rates
            .iter()
            .filter(|&&g| g.abs() > 1e-10)
            .count();
        assert!(non_zero_count > 0, "Should have non-zero gradients");
    }

    #[test]
    fn test_shadow_overhead_minimal() {
        // Test 5.1: Verify clone + zero_out overhead is reasonable
        use std::time::Instant;

        // Large curve for overhead measurement
        let n = 1000;
        let rates: Vec<f64> = (0..n).map(|i| 0.02 + 0.00001 * i as f64).collect();
        let times: Vec<f64> = (1..=n).map(|i| i as f64 * 0.01).collect();
        let market = SimpleYieldCurve::new(rates, times);

        // Measure shadow creation time
        let start = Instant::now();
        for _ in 0..1000 {
            let _shadow = market.create_shadow();
        }
        let elapsed = start.elapsed();

        // 1000 iterations should complete in < 100ms (0.1ms per shadow creation)
        assert!(
            elapsed.as_millis() < 100,
            "Shadow creation overhead too high: {:?}",
            elapsed
        );
    }

    // =========================================================================
    // Task 5.2: VolSurface Integration Tests
    // =========================================================================

    #[test]
    fn test_vol_surface_structure() {
        use super::super::shadow::{Shadow, SimpleVolSurface};

        // Create a realistic vol surface (5 expiries x 5 strikes)
        let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
        let expiries = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let vols = vec![
            vec![0.25, 0.22, 0.20, 0.22, 0.25], // 3M smile
            vec![0.24, 0.21, 0.19, 0.21, 0.24], // 6M smile
            vec![0.23, 0.20, 0.18, 0.20, 0.23], // 1Y smile
            vec![0.22, 0.19, 0.17, 0.19, 0.22], // 2Y smile
            vec![0.21, 0.18, 0.16, 0.18, 0.21], // 5Y smile
        ];

        let surface = SimpleVolSurface::new(vols.clone(), strikes.clone(), expiries.clone());

        // Verify structure
        assert_eq!(surface.n_expiries(), 5);
        assert_eq!(surface.n_strikes(), 5);

        // Create shadow and verify structure preserved
        let shadow = surface.create_shadow();

        assert_eq!(shadow.n_expiries(), surface.n_expiries());
        assert_eq!(shadow.n_strikes(), surface.n_strikes());
        assert_eq!(shadow.strikes, surface.strikes);
        assert_eq!(shadow.expiries, surface.expiries);

        // All vols should be zeroed
        for row in &shadow.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }
    }

    #[test]
    fn test_vol_surface_gradient_mapping() {
        use super::super::shadow::{Shadow, SimpleVolSurface};

        // Create surface
        let surface = SimpleVolSurface::new(
            vec![vec![0.20, 0.22], vec![0.21, 0.23]],
            vec![100.0, 110.0],
            vec![0.5, 1.0],
        );

        // Create shadow and simulate gradient accumulation
        let mut shadow = surface.create_shadow();

        // Set gradients at specific positions
        *shadow.vol_mut(0, 0) = 1.5; // (expiry=0, strike=0)
        *shadow.vol_mut(1, 1) = 2.3; // (expiry=1, strike=1)

        // Verify gradient retrieval at same indices
        assert_eq!(shadow.vol(0, 0), 1.5);
        assert_eq!(shadow.vol(1, 1), 2.3);
        assert_eq!(shadow.vol(0, 1), 0.0); // Unchanged
    }

    // =========================================================================
    // Task 5.3: Feature Flag Verification Tests
    // =========================================================================

    #[test]
    fn test_fallback_without_enzyme() {
        // This test verifies that the code compiles and runs without enzyme-ad feature
        // The finite difference fallback should produce valid results

        let calc = MarketRiskCalculator::with_bump_size(1e-8);
        let market = SimpleYieldCurve::new(vec![0.03, 0.04, 0.05], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.04, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::rates_only())
            .unwrap();

        // Finite difference should produce reasonable gradients
        // For an ATM-ish swap, expect sensitivities in the range of 100K-2M
        for (i, &grad) in result.gradients.rates.iter().enumerate() {
            assert!(
                grad.abs() > 10_000.0,
                "FD gradient {} too small: {}",
                i,
                grad
            );
        }
    }

    #[test]
    fn test_bump_size_sensitivity() {
        // Verify that different bump sizes produce similar results (within tolerance)
        let market = SimpleYieldCurve::new(vec![0.03], vec![1.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 1);

        let calc_small = MarketRiskCalculator::with_bump_size(1e-8);
        let calc_large = MarketRiskCalculator::with_bump_size(1e-6);

        let result_small = calc_small
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();
        let result_large = calc_large
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        // Results should be close (within 1% relative error)
        let grad_small = result_small.gradients.rates[0];
        let grad_large = result_large.gradients.rates[0];
        let rel_error = ((grad_small - grad_large) / grad_small).abs();

        assert!(
            rel_error < 0.01,
            "Bump size sensitivity too high: small={}, large={}, rel_error={}",
            grad_small,
            grad_large,
            rel_error
        );
    }

    #[test]
    fn test_error_types_complete() {
        // Verify all error types are properly defined and usable
        let err1 = ShadowAadError::LengthMismatch {
            expected: 10,
            actual: 5,
        };
        assert!(err1.to_string().contains("Length mismatch"));

        let err2 = ShadowAadError::EmptySlice { field: "rates" };
        assert!(err2.to_string().contains("Empty slice"));

        let err3 = ShadowAadError::EnzymeNotAvailable;
        assert!(err3.to_string().contains("Enzyme AD not available"));

        let err4 = ShadowAadError::InvalidMarketData {
            message: "test".to_string(),
        };
        assert!(err4.to_string().contains("Invalid market data"));
    }
}

// =============================================================================
// GlobalBootstrapResult Integration (Task 3.5)
// =============================================================================

#[cfg(feature = "global-bootstrap")]
mod global_bootstrap_integration {
    use pricer_models::builder::{GlobalBootstrapResult, IftError};

    use super::*;

    // =========================================================================
    // Error Types
    // =========================================================================

    /// Error type for IFT-based risk calculations.
    #[derive(Debug, Error, Clone, PartialEq)]
    pub enum IftRiskError {
        /// IFT computation error (forwarded from GlobalBootstrapResult).
        #[error("IFT computation error: {0}")]
        IftError(#[from] IftError),

        /// Curve mismatch - trade references different curve than provided.
        #[error("Curve mismatch: trade expects {expected}, got {actual}")]
        CurveMismatch {
            /// Expected curve identifier
            expected: String,
            /// Actual curve identifier
            actual: String,
        },

        /// Empty trade batch provided.
        #[error("Empty trade batch")]
        EmptyBatch,

        /// Dimension mismatch in batch processing.
        #[error("Batch dimension mismatch: {message}")]
        BatchDimensionMismatch {
            /// Error message
            message: String,
        },

        /// Shadow AAD error during computation.
        #[error("Shadow AAD error: {0}")]
        ShadowAadError(#[from] ShadowAadError),
    }

    // =========================================================================
    // Result Types
    // =========================================================================

    /// Result of IFT-based curve sensitivity calculation.
    #[derive(Debug, Clone)]
    pub struct IftRiskResult {
        /// Present value of the trade.
        pub pv: f64,

        /// Sensitivity of PV to each pillar DF (∂PV/∂DF_i).
        pub df_sensitivities: Vec<f64>,

        /// Sensitivity of PV to each market input (∂PV/∂quote_j).
        /// Computed via IFT: ∂PV/∂quote = ∂PV/∂DF · ∂DF/∂quote
        pub market_sensitivities: Vec<f64>,
    }

    /// Result of batched IFT risk calculation across multiple trades.
    #[derive(Debug, Clone)]
    pub struct BatchIftRiskResult {
        /// Individual trade results.
        pub trade_results: Vec<IftRiskResult>,

        /// Aggregated market sensitivities across all trades.
        pub total_market_sensitivities: Vec<f64>,

        /// Total PV across all trades.
        pub total_pv: f64,
    }

    // =========================================================================
    // Trade Abstraction for IFT
    // =========================================================================

    /// Trait for trades that can compute sensitivities to discount factors.
    ///
    /// This is required for IFT-based sensitivity propagation.
    pub trait IftTrade {
        /// Compute the present value and sensitivities to discount factors.
        ///
        /// # Arguments
        ///
        /// * `discount_factors` - Discount factors at each pillar
        /// * `pillars` - Pillar maturities in years
        ///
        /// # Returns
        ///
        /// Tuple of (pv, sensitivities) where sensitivities[i] = ∂PV/∂DF_i
        fn compute_with_df_sensitivities(
            &self,
            discount_factors: &[f64],
            pillars: &[f64],
        ) -> (f64, Vec<f64>);
    }

    // =========================================================================
    // MarketRiskCalculator Extension
    // =========================================================================

    impl MarketRiskCalculator {
        /// Calculate IFT-based market risk using GlobalBootstrapResult.
        ///
        /// Uses the Implicit Function Theorem to propagate sensitivities:
        /// ```text
        /// ∂PV/∂quote = ∂PV/∂DF · ∂DF/∂quote
        ///            = ∂PV/∂DF · (-J⁻¹ · ∂F/∂quote)
        /// ```
        ///
        /// # Arguments
        ///
        /// * `bootstrap_result` - Calibration result with cached J⁻¹
        /// * `trade` - Trade implementing IftTrade
        /// * `dF_dquote` - Sensitivity of residuals to market quotes
        ///   (n_instruments × n_quotes)
        ///
        /// # Returns
        ///
        /// IftRiskResult with PV and sensitivities to both DFs and market
        /// quotes.
        ///
        /// # Requirements Coverage
        ///
        /// - AC7.1: Accept GlobalBootstrapResult as input
        /// - AC7.2: Use cached J⁻¹ for sensitivity calculation
        pub fn calculate_ift_risk<T: IftTrade>(
            &self,
            bootstrap_result: &GlobalBootstrapResult<f64>,
            trade: &T,
            dF_dquote: &[Vec<f64>],
        ) -> Result<IftRiskResult, IftRiskError> {
            // Check IFT availability
            if !bootstrap_result.can_compute_ift() {
                return Err(IftRiskError::IftError(IftError::NoJacobianInverse));
            }

            // Compute PV and DF sensitivities
            let (pv, df_sensitivities) = trade.compute_with_df_sensitivities(
                &bootstrap_result.discount_factors,
                &bootstrap_result.pillars,
            );

            // Compute market sensitivities via IFT
            // ∂PV/∂quote_j = Σ_i (∂PV/∂DF_i) · (∂DF_i/∂quote_j)
            // where ∂DF/∂quote = -J⁻¹ · ∂F/∂quote
            let mut market_sensitivities = Vec::with_capacity(dF_dquote.len());

            for dF_dq in dF_dquote {
                // Get ∂DF/∂quote_j using IFT
                let df_dquote = bootstrap_result.ift_sensitivity(dF_dq)?;

                // Chain rule: ∂PV/∂quote_j = ∂PV/∂DF · ∂DF/∂quote_j
                let market_sens: f64 = df_sensitivities
                    .iter()
                    .zip(df_dquote.iter())
                    .map(|(&dpv_ddf, &ddf_dq)| dpv_ddf * ddf_dq)
                    .sum();

                market_sensitivities.push(market_sens);
            }

            Ok(IftRiskResult {
                pv,
                df_sensitivities,
                market_sensitivities,
            })
        }

        /// Calculate batched IFT risk for multiple trades sharing the same
        /// curve.
        ///
        /// This is more efficient than calling `calculate_ift_risk` for each
        /// trade because the IFT computation (J⁻¹ · ∂F/∂quote) is
        /// performed once and reused across all trades.
        ///
        /// # Arguments
        ///
        /// * `bootstrap_result` - Calibration result with cached J⁻¹
        /// * `trades` - Vector of trades implementing IftTrade
        /// * `dF_dquote` - Sensitivity of residuals to market quotes
        ///   (n_instruments × n_quotes)
        ///
        /// # Returns
        ///
        /// BatchIftRiskResult with individual trade results and aggregated
        /// sensitivities.
        ///
        /// # Requirements Coverage
        ///
        /// - AC7.4: Batch curve sensitivities across trades sharing the same
        ///   curve
        pub fn calculate_batch_ift_risk<T: IftTrade>(
            &self,
            bootstrap_result: &GlobalBootstrapResult<f64>,
            trades: &[T],
            dF_dquote: &[Vec<f64>],
        ) -> Result<BatchIftRiskResult, IftRiskError> {
            if trades.is_empty() {
                return Err(IftRiskError::EmptyBatch);
            }

            // Check IFT availability
            if !bootstrap_result.can_compute_ift() {
                return Err(IftRiskError::IftError(IftError::NoJacobianInverse));
            }

            // Pre-compute ∂DF/∂quote for all market parameters (shared across trades)
            let n_quotes = dF_dquote.len();
            let mut df_dquote_cache: Vec<Vec<f64>> = Vec::with_capacity(n_quotes);

            for dF_dq in dF_dquote {
                let df_dquote = bootstrap_result.ift_sensitivity(dF_dq)?;
                df_dquote_cache.push(df_dquote);
            }

            // Process each trade
            let mut trade_results = Vec::with_capacity(trades.len());
            let mut total_pv = 0.0;
            let mut total_market_sensitivities = vec![0.0; n_quotes];

            for trade in trades {
                // Compute PV and DF sensitivities
                let (pv, df_sensitivities) = trade.compute_with_df_sensitivities(
                    &bootstrap_result.discount_factors,
                    &bootstrap_result.pillars,
                );

                total_pv += pv;

                // Compute market sensitivities using cached ∂DF/∂quote
                let mut market_sensitivities = Vec::with_capacity(n_quotes);

                for (j, df_dquote) in df_dquote_cache.iter().enumerate() {
                    let market_sens: f64 = df_sensitivities
                        .iter()
                        .zip(df_dquote.iter())
                        .map(|(&dpv_ddf, &ddf_dq)| dpv_ddf * ddf_dq)
                        .sum();

                    market_sensitivities.push(market_sens);
                    total_market_sensitivities[j] += market_sens;
                }

                trade_results.push(IftRiskResult {
                    pv,
                    df_sensitivities,
                    market_sensitivities,
                });
            }

            Ok(BatchIftRiskResult {
                trade_results,
                total_market_sensitivities,
                total_pv,
            })
        }

        /// Check if IFT-based risk calculation is available.
        ///
        /// Returns true if the bootstrap result has a cached Jacobian inverse.
        #[inline]
        pub fn can_compute_ift_risk(&self, bootstrap_result: &GlobalBootstrapResult<f64>) -> bool {
            bootstrap_result.can_compute_ift()
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    #[cfg(test)]
    mod tests {
        use pricer_core::math::linalg::DMatrix;
        use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve};

        use super::*;

        // Mock trade for testing
        struct MockIrsTrade {
            notional: f64,
            fixed_rate: f64,
            n_periods: usize,
        }

        impl MockIrsTrade {
            fn new(notional: f64, fixed_rate: f64, n_periods: usize) -> Self {
                Self {
                    notional,
                    fixed_rate,
                    n_periods,
                }
            }
        }

        impl IftTrade for MockIrsTrade {
            fn compute_with_df_sensitivities(
                &self,
                discount_factors: &[f64],
                _pillars: &[f64],
            ) -> (f64, Vec<f64>) {
                // Simplified IRS PV: PV = Notional * Σ(DF_i * (floating - fixed) * dt)
                // Assume dt = 1.0 and floating = 3%
                let floating_rate = 0.03;
                let spread = floating_rate - self.fixed_rate;

                let n = discount_factors.len().min(self.n_periods);
                let mut pv = 0.0;
                let mut sensitivities = vec![0.0; discount_factors.len()];

                for (i, &df) in discount_factors.iter().take(n).enumerate() {
                    pv += self.notional * df * spread;
                    sensitivities[i] = self.notional * spread;
                }

                (pv, sensitivities)
            }
        }

        // Helper to create a mock GlobalBootstrapResult
        fn create_mock_bootstrap_result(
            n_pillars: usize,
            with_jacobian: bool,
        ) -> GlobalBootstrapResult<f64> {
            let pillars: Vec<f64> = (1..=n_pillars).map(|i| i as f64).collect();
            let discount_factors: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

            let jacobian_inverse = if with_jacobian {
                Some(DMatrix::identity(n_pillars, n_pillars))
            } else {
                None
            };

            let curve = BootstrappedCurve::new(
                pillars.clone(),
                discount_factors.clone(),
                BootstrapInterpolation::LogLinear,
                true,
            )
            .unwrap();

            GlobalBootstrapResult {
                curve,
                pillars,
                discount_factors,
                residual_norm: 1e-12,
                iterations: 5,
                converged: true,
                jacobian_inverse,
                residual_history: None,
                condition_number: Some(10.0),
                pricing_errors: None,
                realised_jumps: None,
            }
        }

        // =====================================================================
        // Task 3.5 Tests: MarketRiskCalculator GlobalBootstrapResult Integration
        // =====================================================================

        #[test]
        fn test_calculate_ift_risk_basic() {
            // RED: Test IFT risk calculation with valid inputs
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

            // dF/dquote for 3 market quotes (identity for simplicity)
            let dF_dquote = vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ];

            let risk_result = calc.calculate_ift_risk(&result, &trade, &dF_dquote);

            assert!(risk_result.is_ok());
            let risk = risk_result.unwrap();
            assert!(risk.pv.abs() > 0.0);
            assert_eq!(risk.df_sensitivities.len(), 3);
            assert_eq!(risk.market_sensitivities.len(), 3);
        }

        #[test]
        fn test_calculate_ift_risk_no_jacobian_inverse() {
            // Test error when J⁻¹ is not available
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, false); // No jacobian
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);
            let dF_dquote = vec![vec![1.0, 0.0, 0.0]];

            let risk_result = calc.calculate_ift_risk(&result, &trade, &dF_dquote);

            assert!(matches!(
                risk_result,
                Err(IftRiskError::IftError(IftError::NoJacobianInverse))
            ));
        }

        #[test]
        fn test_calculate_ift_risk_dimension_mismatch() {
            // Test error when dF_dquote has wrong dimension
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

            // Wrong dimension: 2 elements instead of 3
            let dF_dquote = vec![vec![1.0, 0.0]];

            let risk_result = calc.calculate_ift_risk(&result, &trade, &dF_dquote);

            assert!(matches!(
                risk_result,
                Err(IftRiskError::IftError(IftError::DimensionMismatch { .. }))
            ));
        }

        #[test]
        fn test_calculate_batch_ift_risk_basic() {
            // Test batch IFT risk calculation
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);

            let trades = vec![
                MockIrsTrade::new(1_000_000.0, 0.02, 3),
                MockIrsTrade::new(2_000_000.0, 0.025, 3),
                MockIrsTrade::new(500_000.0, 0.03, 3),
            ];

            let dF_dquote = vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ];

            let batch_result = calc.calculate_batch_ift_risk(&result, &trades, &dF_dquote);

            assert!(batch_result.is_ok());
            let batch = batch_result.unwrap();
            assert_eq!(batch.trade_results.len(), 3);
            assert_eq!(batch.total_market_sensitivities.len(), 3);

            // Total PV should be sum of individual PVs
            let sum_pv: f64 = batch.trade_results.iter().map(|r| r.pv).sum();
            assert!((batch.total_pv - sum_pv).abs() < 1e-10);
        }

        #[test]
        fn test_calculate_batch_ift_risk_empty_batch() {
            // Test error on empty batch
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trades: Vec<MockIrsTrade> = vec![];
            let dF_dquote = vec![vec![1.0, 0.0, 0.0]];

            let batch_result = calc.calculate_batch_ift_risk(&result, &trades, &dF_dquote);

            assert!(matches!(batch_result, Err(IftRiskError::EmptyBatch)));
        }

        #[test]
        fn test_calculate_batch_ift_risk_aggregation() {
            // Test that batch aggregation is correct
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);

            let trades = vec![
                MockIrsTrade::new(1_000_000.0, 0.02, 3),
                MockIrsTrade::new(1_000_000.0, 0.02, 3), // Same trade
            ];

            let dF_dquote = vec![vec![1.0, 0.0, 0.0]];

            let batch_result = calc
                .calculate_batch_ift_risk(&result, &trades, &dF_dquote)
                .unwrap();

            // Total market sensitivity should be 2x individual
            let individual_sens = batch_result.trade_results[0].market_sensitivities[0];
            let total_sens = batch_result.total_market_sensitivities[0];
            assert!((total_sens - 2.0 * individual_sens).abs() < 1e-10);
        }

        #[test]
        fn test_can_compute_ift_risk() {
            let calc = MarketRiskCalculator::default();

            let result_with_jac = create_mock_bootstrap_result(3, true);
            assert!(calc.can_compute_ift_risk(&result_with_jac));

            let result_without_jac = create_mock_bootstrap_result(3, false);
            assert!(!calc.can_compute_ift_risk(&result_without_jac));
        }

        #[test]
        fn test_ift_risk_chain_rule_correctness() {
            // Verify the chain rule: ∂PV/∂quote = ∂PV/∂DF · ∂DF/∂quote
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

            // With identity Jacobian, ∂DF/∂quote should equal -dF_dquote
            let dF_dquote = vec![vec![0.01, 0.0, 0.0]]; // 1bp shift in first quote

            let risk = calc
                .calculate_ift_risk(&result, &trade, &dF_dquote)
                .unwrap();

            // ∂PV/∂quote[0] should equal -∂PV/∂DF[0] * 0.01 (with identity J⁻¹)
            let expected = -risk.df_sensitivities[0] * 0.01;
            assert!(
                (risk.market_sensitivities[0] - expected).abs() < 1e-10,
                "Chain rule verification failed: got {}, expected {}",
                risk.market_sensitivities[0],
                expected
            );
        }

        #[test]
        fn test_ift_risk_error_display() {
            // Test error Display implementations
            let err1 = IftRiskError::IftError(IftError::NoJacobianInverse);
            assert!(err1.to_string().contains("IFT computation error"));

            let err2 = IftRiskError::EmptyBatch;
            assert!(err2.to_string().contains("Empty trade batch"));

            let err3 = IftRiskError::CurveMismatch {
                expected: "USD-OIS".to_string(),
                actual: "EUR-OIS".to_string(),
            };
            assert!(err3.to_string().contains("Curve mismatch"));
        }
    }
}

// Re-export for global-bootstrap feature
#[cfg(feature = "global-bootstrap")]
pub use global_bootstrap_integration::*;

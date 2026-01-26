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

use super::kernel::{finite_difference_gradients, pricing_kernel_irs};
use super::shadow::{Shadow, SimpleMarketData, SimpleYieldCurve};
use thiserror::Error;

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
/// use pricer_risk::enzyme::binder::ActivityMask;
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
/// use pricer_risk::enzyme::binder::RiskResult;
/// use pricer_risk::enzyme::shadow::SimpleYieldCurve;
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
    pub fn new(pv: f64, gradients: M) -> Self {
        Self { pv, gradients }
    }
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
    fn default() -> Self {
        Self { bump_size: 1e-7 }
    }
}

impl MarketRiskCalculator {
    /// Create a new calculator with custom bump size.
    pub fn with_bump_size(bump_size: f64) -> Self {
        Self { bump_size }
    }

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
            assert!(
                grad.abs() > 0.0,
                "Gradient {} should be non-zero",
                i
            );
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
        // Verify gradient magnitudes are reasonable (DV01 = ~100 per bp per year per $1M)
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
}

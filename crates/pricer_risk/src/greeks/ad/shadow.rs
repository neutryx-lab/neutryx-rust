//! Shadow Object pattern for Enzyme AAD.
//!
//! This module provides the `Shadow` trait for creating gradient-accumulating
//! shadow objects that mirror the structure of market data for AAD computation.
//!
//! # Shadow Object Pattern
//!
//! The Shadow Object pattern enables Enzyme AAD without modifying existing data
//! structures:
//!
//! 1. **Data structures remain unchanged**: No generic `T` parameter needed
//! 2. **Shadow via clone**: Gradient objects created by cloning + zeroing
//! 3. **Kernels use slices**: Pricing functions take `&[f64]` arguments
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_risk::greeks::ad::shadow::Shadow;
//!
//! // Clone market data and zero all f64 values
//! let market = YieldCurve { rates: vec![0.02, 0.03, 0.04], times: vec![1.0, 2.0, 5.0] };
//! let d_market = market.create_shadow();
//!
//! // d_market.rates is now [0.0, 0.0, 0.0]
//! // After AAD: d_market.rates contains gradients
//! ```
//!
//! # Requirements Coverage
//!
//! - 1.1: Clone bound on Shadow trait
//! - 1.2: zero_out() sets all f64/Vec<f64> to 0.0
//! - 1.3: create_shadow() = clone() + zero_out()
//! - 1.4: Shadow has identical memory layout to original
//! - 6.1-6.5: Gradient mapping matches original structure

/// Shadow trait for gradient object generation.
///
/// Implementors of this trait can create "shadow" copies where all numeric
/// values are zeroed. These shadow objects serve as gradient accumulators
/// in reverse-mode AAD.
///
/// # Requirements Coverage
///
/// - 1.1: Requires `Clone` bound
/// - 1.2: `zero_out()` resets all numeric fields to 0.0
/// - 1.3: `create_shadow()` provides default impl (clone + zero_out)
/// - 1.4: Shadow type is identical to `Self` (same memory layout)
/// - 1.5: Supports nested structures via recursive `zero_out()` calls
pub trait Shadow: Clone {
    /// Reset all numeric fields to zero.
    ///
    /// This method sets all `f64` fields and `Vec<f64>` elements to `0.0`.
    /// For nested structures, implementations should recursively call
    /// `zero_out()` on child Shadow implementors.
    ///
    /// # Postconditions
    ///
    /// After calling `zero_out()`, all numeric values in `self` are `0.0`.
    fn zero_out(&mut self);

    /// Create a shadow object for gradient accumulation.
    ///
    /// This method clones `self` and zeroes all numeric values, creating
    /// a gradient buffer with identical structure to the original.
    ///
    /// # Default Implementation
    ///
    /// The default implementation clones and calls `zero_out()`:
    ///
    /// ```rust,ignore
    /// let mut shadow = self.clone();
    /// shadow.zero_out();
    /// shadow
    /// ```
    ///
    /// # Returns
    ///
    /// A new instance with all numeric values set to `0.0`.
    ///
    /// # Guarantees
    ///
    /// - Shadow has identical type and memory layout as original
    /// - Original object is not modified
    /// - All `f64` values in shadow are `0.0`
    #[inline]
    fn create_shadow(&self) -> Self {
        let mut shadow = self.clone();
        shadow.zero_out();
        shadow
    }
}

// =============================================================================
// Primitive Type Implementations
// =============================================================================

impl Shadow for f64 {
    #[inline]
    fn zero_out(&mut self) { *self = 0.0; }
}

impl Shadow for f32 {
    #[inline]
    fn zero_out(&mut self) { *self = 0.0; }
}

impl Shadow for Vec<f64> {
    #[inline]
    fn zero_out(&mut self) { self.fill(0.0); }
}

impl Shadow for Vec<f32> {
    #[inline]
    fn zero_out(&mut self) { self.fill(0.0); }
}

impl Shadow for Vec<Vec<f64>> {
    #[inline]
    fn zero_out(&mut self) {
        for row in self.iter_mut() {
            row.fill(0.0);
        }
    }
}

impl Shadow for Vec<Vec<f32>> {
    #[inline]
    fn zero_out(&mut self) {
        for row in self.iter_mut() {
            row.fill(0.0);
        }
    }
}

// =============================================================================
// Market Data Structures for Shadow Object AAD
// =============================================================================

/// Simple yield curve for Shadow Object AAD.
///
/// This structure holds yield curve data in a format suitable for Enzyme AAD.
/// Unlike `InterpolatedCurve<T>`, this uses concrete `f64` without generics,
/// enabling direct Shadow trait implementation.
///
/// # Structure
///
/// - `rates`: Zero rates at pillar tenors (Active: differentiable)
/// - `times`: Tenor points in years (Const: non-differentiable)
///
/// # Requirements Coverage
///
/// - 1.5: Nested Shadow structure support
/// - 6.1: Shadow has identical field structure
/// - 6.2: Gradient for `rates[i]` is at `d_curve.rates[i]`
/// - 6.4: Named field access for gradients
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleYieldCurve {
    /// Zero rates at pillar tenors (Active input for AAD)
    pub rates: Vec<f64>,
    /// Tenor points in years (Const input for AAD)
    pub times: Vec<f64>,
}

impl SimpleYieldCurve {
    /// Create a new yield curve from rates and times.
    ///
    /// # Arguments
    ///
    /// * `rates` - Zero rates at pillar tenors
    /// * `times` - Tenor points in years
    ///
    /// # Panics
    ///
    /// Panics if rates and times have different lengths.
    #[inline]
    pub fn new(rates: Vec<f64>, times: Vec<f64>) -> Self {
        assert_eq!(
            rates.len(),
            times.len(),
            "rates and times must have same length"
        );
        Self { rates, times }
    }

    /// Return the number of pillar points.
    #[inline]
    pub fn len(&self) -> usize { self.rates.len() }

    /// Return whether the curve is empty.
    #[inline]
    pub fn is_empty(&self) -> bool { self.rates.is_empty() }

    /// Get rates as a slice (for kernel functions).
    #[inline]
    pub fn rates_slice(&self) -> &[f64] { &self.rates }

    /// Get times as a slice (for kernel functions).
    #[inline]
    pub fn times_slice(&self) -> &[f64] { &self.times }

    /// Get mutable rates slice (for gradient accumulation).
    #[inline]
    pub fn rates_slice_mut(&mut self) -> &mut [f64] { &mut self.rates }
}

impl Shadow for SimpleYieldCurve {
    /// Zero out all rate values.
    ///
    /// Times are NOT zeroed because they are const (non-differentiable).
    /// Only the rates (active inputs) are set to zero.
    #[inline]
    fn zero_out(&mut self) {
        self.rates.zero_out();
        // times are const, do not zero
    }
}

/// Simple volatility surface for Shadow Object AAD.
///
/// This structure holds volatility surface data in a format suitable for
/// Enzyme AAD. Uses concrete `f64` without generics.
///
/// # Structure
///
/// - `vols`: 2D volatility grid `vols[expiry_idx][strike_idx]` (Active)
/// - `strikes`: Strike prices (Const)
/// - `expiries`: Expiry times in years (Const)
///
/// # Requirements Coverage
///
/// - 1.5: Nested Shadow structure support
/// - 6.1: Shadow has identical field structure
/// - 6.4: Named field access (`d_surface.vols`)
/// - 6.5: Multiple curves/surfaces preserve identity
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleVolSurface {
    /// Volatility grid: `vols[expiry_idx][strike_idx]` (Active input for AAD)
    pub vols: Vec<Vec<f64>>,
    /// Strike prices (Const input for AAD)
    pub strikes: Vec<f64>,
    /// Expiry times in years (Const input for AAD)
    pub expiries: Vec<f64>,
}

impl SimpleVolSurface {
    /// Create a new volatility surface.
    ///
    /// # Arguments
    ///
    /// * `vols` - 2D volatility grid `vols[expiry_idx][strike_idx]`
    /// * `strikes` - Strike prices
    /// * `expiries` - Expiry times in years
    ///
    /// # Panics
    ///
    /// Panics if dimensions are inconsistent.
    #[inline]
    pub fn new(vols: Vec<Vec<f64>>, strikes: Vec<f64>, expiries: Vec<f64>) -> Self {
        assert_eq!(vols.len(), expiries.len(), "vols rows must match expiries");
        for (i, row) in vols.iter().enumerate() {
            assert_eq!(
                row.len(),
                strikes.len(),
                "vols row {} must match strikes",
                i
            );
        }
        Self {
            vols,
            strikes,
            expiries,
        }
    }

    /// Return the number of expiries.
    #[inline]
    pub fn n_expiries(&self) -> usize { self.expiries.len() }

    /// Return the number of strikes.
    #[inline]
    pub fn n_strikes(&self) -> usize { self.strikes.len() }

    /// Get volatility at (expiry_idx, strike_idx).
    #[inline]
    pub fn vol(&self, expiry_idx: usize, strike_idx: usize) -> f64 {
        self.vols[expiry_idx][strike_idx]
    }

    /// Get mutable reference to volatility at (expiry_idx, strike_idx).
    #[inline]
    pub fn vol_mut(&mut self, expiry_idx: usize, strike_idx: usize) -> &mut f64 {
        &mut self.vols[expiry_idx][strike_idx]
    }

    /// Flatten vols to a single slice for kernel functions.
    ///
    /// Returns row-major order: `[row0, row1, ...]`
    pub fn vols_flat(&self) -> Vec<f64> { self.vols.iter().flatten().copied().collect() }
}

impl Shadow for SimpleVolSurface {
    /// Zero out all volatility values.
    ///
    /// Strikes and expiries are NOT zeroed because they are const.
    /// Only the vols (active inputs) are set to zero.
    #[inline]
    fn zero_out(&mut self) {
        self.vols.zero_out();
        // strikes and expiries are const, do not zero
    }
}

/// Combined market data for Shadow Object AAD.
///
/// Holds multiple curves and surfaces together, demonstrating
/// nested Shadow support.
///
/// # Requirements Coverage
///
/// - 1.5: Nested structures support
/// - 6.5: Multiple curves preserve identity
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleMarketData {
    /// Discount curve
    pub discount_curve: SimpleYieldCurve,
    /// Forward curve (optional)
    pub forward_curve: Option<SimpleYieldCurve>,
    /// Volatility surface (optional)
    pub vol_surface: Option<SimpleVolSurface>,
}

impl SimpleMarketData {
    /// Create market data with only a discount curve.
    #[inline]
    pub fn with_discount_curve(discount_curve: SimpleYieldCurve) -> Self {
        Self {
            discount_curve,
            forward_curve: None,
            vol_surface: None,
        }
    }

    /// Add a forward curve.
    #[inline]
    pub fn with_forward_curve(mut self, forward_curve: SimpleYieldCurve) -> Self {
        self.forward_curve = Some(forward_curve);
        self
    }

    /// Add a volatility surface.
    #[inline]
    pub fn with_vol_surface(mut self, vol_surface: SimpleVolSurface) -> Self {
        self.vol_surface = Some(vol_surface);
        self
    }
}

impl Shadow for SimpleMarketData {
    /// Zero out all active inputs in nested structures.
    #[inline]
    fn zero_out(&mut self) {
        self.discount_curve.zero_out();
        if let Some(ref mut fwd) = self.forward_curve {
            fwd.zero_out();
        }
        if let Some(ref mut vol) = self.vol_surface {
            vol.zero_out();
        }
    }
}

// =============================================================================
// GlobalBootstrapResult Shadow Implementation (Requirement 7.3)
// =============================================================================

#[cfg(feature = "global-bootstrap")]
mod global_bootstrap_shadow {
    use super::Shadow;
    use pricer_models::builder::GlobalBootstrapResult;

    /// Shadow implementation for GlobalBootstrapResult<f64>.
    ///
    /// This enables IFT-based curve sensitivity computation within the AAD
    /// framework. The implementation distinguishes between:
    ///
    /// - **Active inputs** (zeroed): discount_factors, residual_norm, pricing_errors
    /// - **Const inputs** (preserved): curve, jacobian_inverse, pillars, iterations, etc.
    ///
    /// The `jacobian_inverse` is treated as const because it represents the
    /// fixed system response at calibration time and is used for IFT computation.
    ///
    /// # Requirement: 7.3
    impl Shadow for GlobalBootstrapResult<f64> {
        fn zero_out(&mut self) {
            // Active inputs: discount_factors (differentiable calibration outputs)
            self.discount_factors.zero_out();

            // Active: residual_norm (scalar output)
            self.residual_norm = 0.0;

            // Active: pricing_errors (per-instrument residuals)
            if let Some(ref mut errors) = self.pricing_errors {
                errors.zero_out();
            }

            // The following are CONST (not zeroed):
            // - curve: Reconstructed from discount_factors, not independent
            // - jacobian_inverse: Fixed at calibration time, used for IFT
            // - pillars: Time points (non-differentiable)
            // - iterations: Solver metadata (non-differentiable)
            // - converged: Boolean flag (non-differentiable)
            // - residual_history: Diagnostic data (non-differentiable)
            // - condition_number: Diagnostic data (non-differentiable)
            // - realised_jumps: Jump calibration metadata (non-differentiable)
        }
    }
}

#[cfg(feature = "global-bootstrap")]
pub use global_bootstrap_shadow::*;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 1.1: Shadow trait basic tests (Requirements 1.1, 1.2, 1.3, 1.4)
    // =========================================================================

    #[test]
    fn test_f64_zero_out() {
        // Requirement 1.2: zero_out() sets f64 to 0.0
        let mut val = 42.0_f64;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_f64_create_shadow() {
        // Requirement 1.3: create_shadow() clones and zeros
        let val = 42.0_f64;
        let shadow = val.create_shadow();

        // Shadow is zeroed
        assert_eq!(shadow, 0.0);
        // Original unchanged
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_f32_zero_out() {
        let mut val = 42.0_f32;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_vec_f64_zero_out() {
        // Requirement 1.2: zero_out() sets all Vec<f64> elements to 0.0
        let mut vec = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        vec.zero_out();

        assert_eq!(vec.len(), 5);
        for &v in &vec {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn test_vec_f64_create_shadow() {
        // Requirement 1.3, 1.4: create_shadow preserves structure
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let shadow = original.create_shadow();

        // Same length (structure preserved)
        assert_eq!(shadow.len(), original.len());

        // All values zeroed
        for &v in &shadow {
            assert_eq!(v, 0.0);
        }

        // Original unchanged
        assert_eq!(original, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_vec_f64_empty() {
        // Edge case: empty vector
        let mut vec: Vec<f64> = vec![];
        vec.zero_out();
        assert!(vec.is_empty());

        let shadow = vec.create_shadow();
        assert!(shadow.is_empty());
    }

    #[test]
    fn test_vec_vec_f64_zero_out() {
        // Requirement 1.5: Nested structure support
        let mut matrix = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];

        matrix.zero_out();

        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0].len(), 3);
        assert_eq!(matrix[1].len(), 3);

        for row in &matrix {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }
    }

    #[test]
    fn test_vec_vec_f64_create_shadow() {
        let original = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];

        let shadow = original.create_shadow();

        // Structure preserved
        assert_eq!(shadow.len(), 2);
        assert_eq!(shadow[0].len(), 3);
        assert_eq!(shadow[1].len(), 3);

        // All values zeroed
        for row in &shadow {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        // Original unchanged
        assert_eq!(original[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(original[1], vec![4.0, 5.0, 6.0]);
    }

    // =========================================================================
    // Additional edge case tests
    // =========================================================================

    #[test]
    fn test_f64_zero_remains_zero() {
        let mut val = 0.0_f64;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_f64_negative_value() {
        let mut val = -123.456_f64;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_f64_infinity() {
        let mut val = f64::INFINITY;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_f64_nan() {
        let mut val = f64::NAN;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_large_vec() {
        // Performance edge case: large vector
        let mut vec: Vec<f64> = (0..10000).map(|i| i as f64).collect();
        vec.zero_out();

        for &v in &vec {
            assert_eq!(v, 0.0);
        }
    }

    // =========================================================================
    // Task 1.2: Market data structure Shadow tests (Requirements 1.5, 6.1-6.5)
    // =========================================================================

    #[test]
    fn test_simple_yield_curve_new() {
        let curve = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);

        assert_eq!(curve.len(), 3);
        assert!(!curve.is_empty());
        assert_eq!(curve.rates_slice(), &[0.02, 0.03, 0.04]);
        assert_eq!(curve.times_slice(), &[1.0, 2.0, 5.0]);
    }

    #[test]
    fn test_simple_yield_curve_zero_out() {
        // Requirement 6.2: Gradient mapping
        let mut curve = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);

        curve.zero_out();

        // Rates are zeroed (active)
        assert_eq!(curve.rates, vec![0.0, 0.0, 0.0]);
        // Times are NOT zeroed (const)
        assert_eq!(curve.times, vec![1.0, 2.0, 5.0]);
    }

    #[test]
    fn test_simple_yield_curve_create_shadow() {
        // Requirement 6.1: Identical field structure
        let original = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);

        let shadow = original.create_shadow();

        // Structure preserved
        assert_eq!(shadow.len(), original.len());
        assert_eq!(shadow.times, original.times);

        // Rates zeroed
        for &r in &shadow.rates {
            assert_eq!(r, 0.0);
        }

        // Original unchanged
        assert_eq!(original.rates, vec![0.02, 0.03, 0.04]);
    }

    #[test]
    fn test_simple_yield_curve_gradient_mapping() {
        // Requirement 6.2: d_market.rates[i] corresponds to market.rates[i]
        let market = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);
        let mut d_market = market.create_shadow();

        // Simulate gradient accumulation
        d_market.rates[0] = 1.5;
        d_market.rates[1] = 2.3;
        d_market.rates[2] = 0.7;

        // Gradients accessible at same indices
        assert_eq!(d_market.rates[0], 1.5);
        assert_eq!(d_market.rates[1], 2.3);
        assert_eq!(d_market.rates[2], 0.7);
    }

    #[test]
    fn test_simple_vol_surface_new() {
        let surface = SimpleVolSurface::new(
            vec![vec![0.20, 0.22, 0.25], vec![0.21, 0.23, 0.26]],
            vec![90.0, 100.0, 110.0],
            vec![0.5, 1.0],
        );

        assert_eq!(surface.n_expiries(), 2);
        assert_eq!(surface.n_strikes(), 3);
        assert_eq!(surface.vol(0, 1), 0.22);
        assert_eq!(surface.vol(1, 2), 0.26);
    }

    #[test]
    fn test_simple_vol_surface_zero_out() {
        let mut surface = SimpleVolSurface::new(
            vec![vec![0.20, 0.22, 0.25], vec![0.21, 0.23, 0.26]],
            vec![90.0, 100.0, 110.0],
            vec![0.5, 1.0],
        );

        surface.zero_out();

        // Vols are zeroed (active)
        for row in &surface.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        // Strikes and expiries are NOT zeroed (const)
        assert_eq!(surface.strikes, vec![90.0, 100.0, 110.0]);
        assert_eq!(surface.expiries, vec![0.5, 1.0]);
    }

    #[test]
    fn test_simple_vol_surface_create_shadow() {
        // Requirement 6.1: Identical structure
        let original = SimpleVolSurface::new(
            vec![vec![0.20, 0.22, 0.25], vec![0.21, 0.23, 0.26]],
            vec![90.0, 100.0, 110.0],
            vec![0.5, 1.0],
        );

        let shadow = original.create_shadow();

        // Structure preserved
        assert_eq!(shadow.n_expiries(), original.n_expiries());
        assert_eq!(shadow.n_strikes(), original.n_strikes());
        assert_eq!(shadow.strikes, original.strikes);
        assert_eq!(shadow.expiries, original.expiries);

        // Vols zeroed
        for row in &shadow.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        // Original unchanged
        assert_eq!(original.vol(0, 0), 0.20);
    }

    #[test]
    fn test_simple_vol_surface_vols_flat() {
        let surface = SimpleVolSurface::new(
            vec![vec![0.20, 0.22], vec![0.21, 0.23]],
            vec![100.0, 110.0],
            vec![0.5, 1.0],
        );

        let flat = surface.vols_flat();
        assert_eq!(flat, vec![0.20, 0.22, 0.21, 0.23]);
    }

    #[test]
    fn test_simple_market_data_discount_only() {
        let curve = SimpleYieldCurve::new(vec![0.02, 0.03], vec![1.0, 2.0]);
        let market = SimpleMarketData::with_discount_curve(curve);

        assert!(market.forward_curve.is_none());
        assert!(market.vol_surface.is_none());
    }

    #[test]
    fn test_simple_market_data_full() {
        let discount = SimpleYieldCurve::new(vec![0.02, 0.03], vec![1.0, 2.0]);
        let forward = SimpleYieldCurve::new(vec![0.025, 0.035], vec![1.0, 2.0]);
        let vol = SimpleVolSurface::new(
            vec![vec![0.20, 0.22], vec![0.21, 0.23]],
            vec![100.0, 110.0],
            vec![0.5, 1.0],
        );

        let market = SimpleMarketData::with_discount_curve(discount)
            .with_forward_curve(forward)
            .with_vol_surface(vol);

        assert!(market.forward_curve.is_some());
        assert!(market.vol_surface.is_some());
    }

    #[test]
    fn test_simple_market_data_zero_out() {
        // Requirement 1.5: Nested structure support
        let discount = SimpleYieldCurve::new(vec![0.02, 0.03], vec![1.0, 2.0]);
        let forward = SimpleYieldCurve::new(vec![0.025, 0.035], vec![1.0, 2.0]);
        let vol = SimpleVolSurface::new(
            vec![vec![0.20, 0.22], vec![0.21, 0.23]],
            vec![100.0, 110.0],
            vec![0.5, 1.0],
        );

        let mut market = SimpleMarketData::with_discount_curve(discount)
            .with_forward_curve(forward)
            .with_vol_surface(vol);

        market.zero_out();

        // All active inputs are zeroed
        assert_eq!(market.discount_curve.rates, vec![0.0, 0.0]);
        assert_eq!(market.forward_curve.as_ref().unwrap().rates, vec![0.0, 0.0]);
        for row in &market.vol_surface.as_ref().unwrap().vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        // Const inputs are preserved
        assert_eq!(market.discount_curve.times, vec![1.0, 2.0]);
        assert_eq!(
            market.vol_surface.as_ref().unwrap().strikes,
            vec![100.0, 110.0]
        );
    }

    #[test]
    fn test_simple_market_data_create_shadow() {
        // Requirement 6.5: Multiple curves preserve identity
        let discount = SimpleYieldCurve::new(vec![0.02, 0.03], vec![1.0, 2.0]);
        let forward = SimpleYieldCurve::new(vec![0.025, 0.035], vec![1.0, 2.0]);

        let market = SimpleMarketData::with_discount_curve(discount).with_forward_curve(forward);

        let shadow = market.create_shadow();

        // Curve identity preserved
        assert_eq!(shadow.discount_curve.len(), market.discount_curve.len());
        assert_eq!(
            shadow.forward_curve.as_ref().unwrap().len(),
            market.forward_curve.as_ref().unwrap().len()
        );

        // Active inputs zeroed
        assert_eq!(shadow.discount_curve.rates, vec![0.0, 0.0]);
        assert_eq!(shadow.forward_curve.as_ref().unwrap().rates, vec![0.0, 0.0]);

        // Original unchanged
        assert_eq!(market.discount_curve.rates, vec![0.02, 0.03]);
    }

    #[test]
    fn test_gradient_named_field_access() {
        // Requirement 6.4: Named field access for gradients
        let market = SimpleMarketData::with_discount_curve(SimpleYieldCurve::new(
            vec![0.02, 0.03],
            vec![1.0, 2.0],
        ));

        let mut d_market = market.create_shadow();

        // Access gradients via named fields
        d_market.discount_curve.rates[0] = 1.5;

        assert_eq!(d_market.discount_curve.rates[0], 1.5);
    }

    // =========================================================================
    // GlobalBootstrapResult Shadow tests (Requirement 7.3)
    // =========================================================================

    #[cfg(feature = "global-bootstrap")]
    mod global_bootstrap_tests {
        use super::*;
        use nalgebra::DMatrix;
        use pricer_models::builder::GlobalBootstrapResult;
        use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve};

        fn create_test_result() -> GlobalBootstrapResult<f64> {
            let pillars = vec![1.0, 2.0, 5.0];
            let discount_factors = vec![0.97, 0.94, 0.85];
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
                residual_norm: 1e-10,
                iterations: 5,
                converged: true,
                jacobian_inverse: Some(DMatrix::identity(3, 3)),
                residual_history: Some(vec![1e-4, 1e-6, 1e-8, 1e-10]),
                condition_number: Some(100.0),
                pricing_errors: Some(vec![1e-10, 2e-10, 3e-10]),
                realised_jumps: None,
            }
        }

        #[test]
        fn test_global_bootstrap_result_zero_out() {
            // Requirement 7.3: Shadow trait for GlobalBootstrapResult
            let mut result = create_test_result();

            // Store original const values
            let original_pillars = result.pillars.clone();
            let original_iterations = result.iterations;
            let original_jacobian = result.jacobian_inverse.clone();

            result.zero_out();

            // Active inputs are zeroed
            assert_eq!(result.discount_factors, vec![0.0, 0.0, 0.0]);
            assert_eq!(result.residual_norm, 0.0);
            assert_eq!(result.pricing_errors, Some(vec![0.0, 0.0, 0.0]));

            // Const inputs are preserved
            assert_eq!(result.pillars, original_pillars);
            assert_eq!(result.iterations, original_iterations);
            assert_eq!(result.jacobian_inverse, original_jacobian);
            assert!(result.converged);
        }

        #[test]
        fn test_global_bootstrap_result_create_shadow() {
            let original = create_test_result();
            let shadow = original.create_shadow();

            // Active inputs zeroed in shadow
            assert_eq!(shadow.discount_factors, vec![0.0, 0.0, 0.0]);
            assert_eq!(shadow.residual_norm, 0.0);
            assert_eq!(shadow.pricing_errors, Some(vec![0.0, 0.0, 0.0]));

            // Original unchanged
            assert_eq!(original.discount_factors, vec![0.97, 0.94, 0.85]);
            assert!((original.residual_norm - 1e-10).abs() < 1e-15);
            assert_eq!(original.pricing_errors, Some(vec![1e-10, 2e-10, 3e-10]));

            // Structure preserved
            assert_eq!(shadow.pillars.len(), original.pillars.len());
            assert_eq!(shadow.iterations, original.iterations);
        }

        #[test]
        fn test_global_bootstrap_result_jacobian_inverse_preserved() {
            // Jacobian inverse must be const for IFT computation
            let mut result = create_test_result();
            let original_j_inv = result.jacobian_inverse.clone();

            result.zero_out();

            // J⁻¹ is preserved (const)
            assert_eq!(result.jacobian_inverse, original_j_inv);
        }

        #[test]
        fn test_global_bootstrap_result_no_pricing_errors() {
            let pillars = vec![1.0, 2.0];
            let discount_factors = vec![0.97, 0.94];
            let curve = BootstrappedCurve::new(
                pillars.clone(),
                discount_factors.clone(),
                BootstrapInterpolation::LogLinear,
                true,
            )
            .unwrap();

            let mut result = GlobalBootstrapResult {
                curve,
                pillars,
                discount_factors,
                residual_norm: 1e-10,
                iterations: 3,
                converged: true,
                jacobian_inverse: None,
                residual_history: None,
                condition_number: None,
                pricing_errors: None, // No pricing errors
                realised_jumps: None,
            };

            // Should not panic when pricing_errors is None
            result.zero_out();

            assert_eq!(result.discount_factors, vec![0.0, 0.0]);
            assert_eq!(result.residual_norm, 0.0);
            assert!(result.pricing_errors.is_none());
        }

        #[test]
        fn test_global_bootstrap_result_gradient_accumulation() {
            // Simulate gradient accumulation workflow
            let original = create_test_result();
            let mut d_result = original.create_shadow();

            // Simulate reverse-mode AD gradient accumulation
            d_result.discount_factors[0] = 0.5; // ∂L/∂DF_0
            d_result.discount_factors[1] = 0.3; // ∂L/∂DF_1
            d_result.discount_factors[2] = 0.2; // ∂L/∂DF_2

            // Gradients are accessible at same indices
            assert_eq!(d_result.discount_factors[0], 0.5);
            assert_eq!(d_result.discount_factors[1], 0.3);
            assert_eq!(d_result.discount_factors[2], 0.2);
        }
    }
}

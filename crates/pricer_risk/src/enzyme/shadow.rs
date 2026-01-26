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
//! use pricer_risk::enzyme::shadow::Shadow;
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
    pub fn len(&self) -> usize {
        self.rates.len()
    }

    /// Return whether the curve is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rates.is_empty()
    }

    /// Get rates as a slice (for kernel functions).
    #[inline]
    pub fn rates_slice(&self) -> &[f64] {
        &self.rates
    }

    /// Get times as a slice (for kernel functions).
    #[inline]
    pub fn times_slice(&self) -> &[f64] {
        &self.times
    }

    /// Get mutable rates slice (for gradient accumulation).
    #[inline]
    pub fn rates_slice_mut(&mut self) -> &mut [f64] {
        &mut self.rates
    }
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
        assert_eq!(
            vols.len(),
            expiries.len(),
            "vols rows must match expiries"
        );
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
    pub fn n_expiries(&self) -> usize {
        self.expiries.len()
    }

    /// Return the number of strikes.
    #[inline]
    pub fn n_strikes(&self) -> usize {
        self.strikes.len()
    }

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
    pub fn vols_flat(&self) -> Vec<f64> {
        self.vols.iter().flatten().copied().collect()
    }
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
}

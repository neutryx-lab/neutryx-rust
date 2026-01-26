//! PricingKernel: SoA (Structure of Arrays) intermediate representation.
//!
//! This module provides the core `PricingKernel` structure that represents
//! flattened cashflows in a SIMD-friendly format for high-performance
//! pricing.
//!
//! # Design Principles
//!
//! - **SoA Layout**: Each field is stored as a contiguous array for
//!   cache-efficient sequential access
//! - **64-byte Alignment**: All numerical arrays use `AlignedBuffer` for
//!   AVX-512 compatibility
//! - **Enzyme AD Compatible**: Only primitive types, no trait objects
//! - **Immutable After Compilation**: Once created, kernels are not modified
//!
//! # Branchless Unified Formula
//!
//! The kernel supports a unified pricing formula that works for both
//! fixed and floating cashflows without branching:
//!
//! ```text
//! Payoff = (L_idx × α + β) × N × τ × FX_idx
//! ```
//!
//! Where:
//! - `L_idx`: Forward rate (0.0 for fixed flows via dummy index)
//! - `α`: Gearing (0.0 for fixed, 1.0 for floating)
//! - `β`: Spread (fixed rate for fixed, spread for floating)
//! - `N`: Notional
//! - `τ`: Year fraction
//! - `FX_idx`: FX rate (1.0 for single currency via dummy index)

use super::{aligned_buffer::AlignedBuffer, error::CompileError};

/// SoA (Structure of Arrays) intermediate representation for cashflows.
///
/// `PricingKernel` stores all cashflow data in contiguous arrays optimised
/// for SIMD operations and Enzyme automatic differentiation.
///
/// # Invariants
///
/// - All arrays have equal length (`len`)
/// - `fwd_index_ids[i] == 0` indicates a fixed flow (dummy forward)
/// - `fx_index_ids[i] == 0` indicates no FX conversion (dummy FX)
/// - Payment dates are sorted in ascending order
///
/// # Examples
///
/// ```
/// use pricer_core::ir::PricingKernel;
///
/// // Create a kernel from vectors (validates length invariant)
/// let kernel = PricingKernel::new(
///     vec![19000, 19180],           // payment_dates
///     vec![18900, 19080],           // fixing_dates
///     vec![0.5, 0.5],               // year_fractions
///     vec![1_000_000.0, 1_000_000.0], // notionals
///     vec![0.05, 0.05],             // spreads (fixed rate)
///     vec![0.0, 0.0],               // gearings (0.0 = fixed)
///     vec![0, 0],                   // currency_ids
///     vec![0, 0],                   // discount_curve_ids
///     vec![0, 0],                   // fwd_index_ids (0 = fixed)
///     vec![0, 0],                   // fx_index_ids (0 = no FX)
/// ).expect("Valid kernel");
///
/// assert_eq!(kernel.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct PricingKernel {
    // === Date Management (i32: Days from Unix Epoch) ===
    /// Payment dates (sorted ascending).
    pub payment_dates: AlignedBuffer<i32>,

    /// Fixing/observation dates for floating coupons.
    pub fixing_dates: AlignedBuffer<i32>,

    // === Static Calculation Coefficients (f64) ===
    /// Year fractions (pre-computed from DayCountConvention).
    pub year_fractions: AlignedBuffer<f64>,

    /// Notional amounts.
    pub notionals: AlignedBuffer<f64>,

    /// Spreads (fixed rate for fixed legs, spread for floating legs).
    pub spreads: AlignedBuffer<f64>,

    /// Gearing coefficients (0.0 for fixed, 1.0+ for floating).
    pub gearings: AlignedBuffer<f64>,

    // === Index Pointers (ID references) ===
    /// Currency IDs (0 = base currency).
    pub currency_ids: Vec<u8>,

    /// Discount curve IDs for present value calculation.
    pub discount_curve_ids: Vec<u8>,

    /// Forward index IDs (0 = dummy returning 0.0 for fixed flows).
    pub fwd_index_ids: Vec<u16>,

    /// FX index IDs (0 = dummy returning 1.0 for single currency).
    pub fx_index_ids: Vec<u16>,

    // === Metadata ===
    /// Number of cashflows.
    len: usize,

    /// Number of original trades (for batched compilation).
    pub trade_count: usize,
}

impl PricingKernel {
    /// Creates a new `PricingKernel` from vectors.
    ///
    /// Validates that all arrays have equal length.
    ///
    /// # Arguments
    ///
    /// * `payment_dates` - Payment dates as days from Unix epoch
    /// * `fixing_dates` - Fixing dates for floating coupons
    /// * `year_fractions` - Pre-computed year fractions
    /// * `notionals` - Notional amounts
    /// * `spreads` - Spread or fixed rate values
    /// * `gearings` - Gearing coefficients
    /// * `currency_ids` - Currency ID references
    /// * `discount_curve_ids` - Discount curve ID references
    /// * `fwd_index_ids` - Forward index ID references
    /// * `fx_index_ids` - FX index ID references
    ///
    /// # Errors
    ///
    /// Returns `CompileError::LengthMismatch` if arrays have different lengths.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        payment_dates: Vec<i32>,
        fixing_dates: Vec<i32>,
        year_fractions: Vec<f64>,
        notionals: Vec<f64>,
        spreads: Vec<f64>,
        gearings: Vec<f64>,
        currency_ids: Vec<u8>,
        discount_curve_ids: Vec<u8>,
        fwd_index_ids: Vec<u16>,
        fx_index_ids: Vec<u16>,
    ) -> Result<Self, CompileError> {
        let len = payment_dates.len();

        // Validate all arrays have the same length
        if fixing_dates.len() != len {
            return Err(CompileError::length_mismatch(len, fixing_dates.len()));
        }
        if year_fractions.len() != len {
            return Err(CompileError::length_mismatch(len, year_fractions.len()));
        }
        if notionals.len() != len {
            return Err(CompileError::length_mismatch(len, notionals.len()));
        }
        if spreads.len() != len {
            return Err(CompileError::length_mismatch(len, spreads.len()));
        }
        if gearings.len() != len {
            return Err(CompileError::length_mismatch(len, gearings.len()));
        }
        if currency_ids.len() != len {
            return Err(CompileError::length_mismatch(len, currency_ids.len()));
        }
        if discount_curve_ids.len() != len {
            return Err(CompileError::length_mismatch(len, discount_curve_ids.len()));
        }
        if fwd_index_ids.len() != len {
            return Err(CompileError::length_mismatch(len, fwd_index_ids.len()));
        }
        if fx_index_ids.len() != len {
            return Err(CompileError::length_mismatch(len, fx_index_ids.len()));
        }

        Ok(Self {
            payment_dates: AlignedBuffer::from_vec(payment_dates),
            fixing_dates: AlignedBuffer::from_vec(fixing_dates),
            year_fractions: AlignedBuffer::from_vec(year_fractions),
            notionals: AlignedBuffer::from_vec(notionals),
            spreads: AlignedBuffer::from_vec(spreads),
            gearings: AlignedBuffer::from_vec(gearings),
            currency_ids,
            discount_curve_ids,
            fwd_index_ids,
            fx_index_ids,
            len,
            trade_count: 1,
        })
    }

    /// Creates an empty `PricingKernel`.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            payment_dates: AlignedBuffer::with_capacity(0),
            fixing_dates: AlignedBuffer::with_capacity(0),
            year_fractions: AlignedBuffer::with_capacity(0),
            notionals: AlignedBuffer::with_capacity(0),
            spreads: AlignedBuffer::with_capacity(0),
            gearings: AlignedBuffer::with_capacity(0),
            currency_ids: Vec::new(),
            discount_curve_ids: Vec::new(),
            fwd_index_ids: Vec::new(),
            fx_index_ids: Vec::new(),
            len: 0,
            trade_count: 0,
        }
    }

    /// Returns the number of cashflows in the kernel.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize { self.len }

    /// Returns `true` if the kernel contains no cashflows.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Returns the number of original trades.
    #[inline]
    #[must_use]
    pub fn trade_count(&self) -> usize { self.trade_count }

    /// Sets the trade count for batched compilation.
    pub fn set_trade_count(&mut self, count: usize) { self.trade_count = count; }

    /// Validates that all internal arrays have consistent lengths.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::LengthMismatch` if arrays are inconsistent.
    pub fn validate(&self) -> Result<(), CompileError> {
        let len = self.len;

        if self.payment_dates.len() != len {
            return Err(CompileError::length_mismatch(len, self.payment_dates.len()));
        }
        if self.fixing_dates.len() != len {
            return Err(CompileError::length_mismatch(len, self.fixing_dates.len()));
        }
        if self.year_fractions.len() != len {
            return Err(CompileError::length_mismatch(
                len,
                self.year_fractions.len(),
            ));
        }
        if self.notionals.len() != len {
            return Err(CompileError::length_mismatch(len, self.notionals.len()));
        }
        if self.spreads.len() != len {
            return Err(CompileError::length_mismatch(len, self.spreads.len()));
        }
        if self.gearings.len() != len {
            return Err(CompileError::length_mismatch(len, self.gearings.len()));
        }
        if self.currency_ids.len() != len {
            return Err(CompileError::length_mismatch(len, self.currency_ids.len()));
        }
        if self.discount_curve_ids.len() != len {
            return Err(CompileError::length_mismatch(
                len,
                self.discount_curve_ids.len(),
            ));
        }
        if self.fwd_index_ids.len() != len {
            return Err(CompileError::length_mismatch(len, self.fwd_index_ids.len()));
        }
        if self.fx_index_ids.len() != len {
            return Err(CompileError::length_mismatch(len, self.fx_index_ids.len()));
        }

        Ok(())
    }

    /// Returns the total memory usage in bytes.
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        self.payment_dates.memory_usage()
            + self.fixing_dates.memory_usage()
            + self.year_fractions.memory_usage()
            + self.notionals.memory_usage()
            + self.spreads.memory_usage()
            + self.gearings.memory_usage()
            + self.currency_ids.len() * std::mem::size_of::<u8>()
            + self.discount_curve_ids.len() * std::mem::size_of::<u8>()
            + self.fwd_index_ids.len() * std::mem::size_of::<u16>()
            + self.fx_index_ids.len() * std::mem::size_of::<u16>()
    }

    /// Returns `true` if all aligned buffers are properly aligned.
    #[must_use]
    pub fn is_aligned(&self) -> bool {
        self.payment_dates.is_aligned()
            && self.fixing_dates.is_aligned()
            && self.year_fractions.is_aligned()
            && self.notionals.is_aligned()
            && self.spreads.is_aligned()
            && self.gearings.is_aligned()
    }
}

/// Builder for constructing `PricingKernel` incrementally.
///
/// Provides a more ergonomic API for building kernels when cashflows
/// are generated one at a time.
#[derive(Debug, Default)]
pub struct PricingKernelBuilder {
    payment_dates: Vec<i32>,
    fixing_dates: Vec<i32>,
    year_fractions: Vec<f64>,
    notionals: Vec<f64>,
    spreads: Vec<f64>,
    gearings: Vec<f64>,
    currency_ids: Vec<u8>,
    discount_curve_ids: Vec<u8>,
    fwd_index_ids: Vec<u16>,
    fx_index_ids: Vec<u16>,
    trade_count: usize,
}

impl PricingKernelBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Creates a builder with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            payment_dates: Vec::with_capacity(capacity),
            fixing_dates: Vec::with_capacity(capacity),
            year_fractions: Vec::with_capacity(capacity),
            notionals: Vec::with_capacity(capacity),
            spreads: Vec::with_capacity(capacity),
            gearings: Vec::with_capacity(capacity),
            currency_ids: Vec::with_capacity(capacity),
            discount_curve_ids: Vec::with_capacity(capacity),
            fwd_index_ids: Vec::with_capacity(capacity),
            fx_index_ids: Vec::with_capacity(capacity),
            trade_count: 0,
        }
    }

    /// Adds a cashflow to the builder.
    #[allow(clippy::too_many_arguments)]
    pub fn add_cashflow(
        &mut self,
        payment_date: i32,
        fixing_date: i32,
        year_fraction: f64,
        notional: f64,
        spread: f64,
        gearing: f64,
        currency_id: u8,
        discount_curve_id: u8,
        fwd_index_id: u16,
        fx_index_id: u16,
    ) -> &mut Self {
        self.payment_dates.push(payment_date);
        self.fixing_dates.push(fixing_date);
        self.year_fractions.push(year_fraction);
        self.notionals.push(notional);
        self.spreads.push(spread);
        self.gearings.push(gearing);
        self.currency_ids.push(currency_id);
        self.discount_curve_ids.push(discount_curve_id);
        self.fwd_index_ids.push(fwd_index_id);
        self.fx_index_ids.push(fx_index_id);
        self
    }

    /// Adds a fixed cashflow (gearing = 0, fwd_index_id = 0).
    #[allow(clippy::too_many_arguments)]
    pub fn add_fixed_cashflow(
        &mut self,
        payment_date: i32,
        fixing_date: i32,
        year_fraction: f64,
        notional: f64,
        fixed_rate: f64,
        currency_id: u8,
        discount_curve_id: u8,
    ) -> &mut Self {
        self.add_cashflow(
            payment_date,
            fixing_date,
            year_fraction,
            notional,
            fixed_rate, // spread = fixed rate
            0.0,        // gearing = 0 for fixed
            currency_id,
            discount_curve_id,
            0, // fwd_index_id = 0 (dummy)
            0, // fx_index_id = 0 (no FX)
        )
    }

    /// Adds a floating cashflow.
    #[allow(clippy::too_many_arguments)]
    pub fn add_floating_cashflow(
        &mut self,
        payment_date: i32,
        fixing_date: i32,
        year_fraction: f64,
        notional: f64,
        spread: f64,
        gearing: f64,
        currency_id: u8,
        discount_curve_id: u8,
        fwd_index_id: u16,
    ) -> &mut Self {
        self.add_cashflow(
            payment_date,
            fixing_date,
            year_fraction,
            notional,
            spread,
            gearing,
            currency_id,
            discount_curve_id,
            fwd_index_id,
            0, // fx_index_id = 0 (no FX)
        )
    }

    /// Sets the trade count.
    pub fn set_trade_count(&mut self, count: usize) -> &mut Self {
        self.trade_count = count;
        self
    }

    /// Increments the trade count.
    pub fn increment_trade_count(&mut self) -> &mut Self {
        self.trade_count += 1;
        self
    }

    /// Sorts cashflows by payment date (ascending).
    pub fn sort_by_payment_date(&mut self) -> &mut Self {
        if self.payment_dates.is_empty() {
            return self;
        }

        // Create indices and sort by payment date
        let mut indices: Vec<usize> = (0..self.payment_dates.len()).collect();
        indices.sort_by_key(|&i| self.payment_dates[i]);

        // Reorder all arrays
        self.payment_dates = indices.iter().map(|&i| self.payment_dates[i]).collect();
        self.fixing_dates = indices.iter().map(|&i| self.fixing_dates[i]).collect();
        self.year_fractions = indices.iter().map(|&i| self.year_fractions[i]).collect();
        self.notionals = indices.iter().map(|&i| self.notionals[i]).collect();
        self.spreads = indices.iter().map(|&i| self.spreads[i]).collect();
        self.gearings = indices.iter().map(|&i| self.gearings[i]).collect();
        self.currency_ids = indices.iter().map(|&i| self.currency_ids[i]).collect();
        self.discount_curve_ids = indices
            .iter()
            .map(|&i| self.discount_curve_ids[i])
            .collect();
        self.fwd_index_ids = indices.iter().map(|&i| self.fwd_index_ids[i]).collect();
        self.fx_index_ids = indices.iter().map(|&i| self.fx_index_ids[i]).collect();

        self
    }

    /// Returns the current number of cashflows in the builder.
    #[must_use]
    pub fn len(&self) -> usize { self.payment_dates.len() }

    /// Returns `true` if the builder is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.payment_dates.is_empty() }

    /// Builds the `PricingKernel`.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if validation fails.
    pub fn build(self) -> Result<PricingKernel, CompileError> {
        let mut kernel = PricingKernel::new(
            self.payment_dates,
            self.fixing_dates,
            self.year_fractions,
            self.notionals,
            self.spreads,
            self.gearings,
            self.currency_ids,
            self.discount_curve_ids,
            self.fwd_index_ids,
            self.fx_index_ids,
        )?;

        kernel.set_trade_count(self.trade_count);
        Ok(kernel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_kernel_new() {
        let kernel = PricingKernel::new(
            vec![19000, 19180],
            vec![18900, 19080],
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        )
        .expect("Valid kernel");

        assert_eq!(kernel.len(), 2);
        assert!(!kernel.is_empty());
    }

    #[test]
    fn test_pricing_kernel_empty() {
        let kernel = PricingKernel::empty();
        assert_eq!(kernel.len(), 0);
        assert!(kernel.is_empty());
    }

    #[test]
    fn test_pricing_kernel_length_mismatch() {
        let result = PricingKernel::new(
            vec![19000, 19180],
            vec![18900], // Wrong length
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompileError::LengthMismatch { .. }
        ));
    }

    #[test]
    fn test_pricing_kernel_validate() {
        let kernel = PricingKernel::new(
            vec![19000],
            vec![18900],
            vec![0.5],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        assert!(kernel.validate().is_ok());
    }

    #[test]
    fn test_pricing_kernel_alignment() {
        let kernel = PricingKernel::new(
            vec![19000, 19180, 19365],
            vec![18900, 19080, 19265],
            vec![0.5, 0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05, 0.05],
            vec![0.0, 0.0, 0.0],
            vec![0, 0, 0],
            vec![0, 0, 0],
            vec![0, 0, 0],
            vec![0, 0, 0],
        )
        .unwrap();

        assert!(kernel.is_aligned());
    }

    #[test]
    fn test_pricing_kernel_memory_usage() {
        let kernel = PricingKernel::new(
            vec![19000],
            vec![18900],
            vec![0.5],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        assert!(kernel.memory_usage() > 0);
    }

    #[test]
    fn test_pricing_kernel_trade_count() {
        let mut kernel = PricingKernel::new(
            vec![19000],
            vec![18900],
            vec![0.5],
            vec![1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .unwrap();

        assert_eq!(kernel.trade_count(), 1);

        kernel.set_trade_count(5);
        assert_eq!(kernel.trade_count(), 5);
    }

    #[test]
    fn test_pricing_kernel_clone() {
        let kernel = PricingKernel::new(
            vec![19000, 19180],
            vec![18900, 19080],
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        )
        .unwrap();

        let cloned = kernel.clone();
        assert_eq!(cloned.len(), kernel.len());
        assert_eq!(cloned.payment_dates[0], kernel.payment_dates[0]);
    }

    #[test]
    fn test_builder_new() {
        let builder = PricingKernelBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_builder_add_cashflow() {
        let mut builder = PricingKernelBuilder::new();
        builder.add_cashflow(19000, 18900, 0.5, 1_000_000.0, 0.05, 0.0, 0, 0, 0, 0);

        assert_eq!(builder.len(), 1);
    }

    #[test]
    fn test_builder_add_fixed_cashflow() {
        let mut builder = PricingKernelBuilder::new();
        builder.add_fixed_cashflow(19000, 18900, 0.5, 1_000_000.0, 0.05, 0, 0);

        let kernel = builder.build().unwrap();
        assert!((kernel.gearings[0] - 0.0).abs() < 1e-10);
        assert_eq!(kernel.fwd_index_ids[0], 0);
    }

    #[test]
    fn test_builder_add_floating_cashflow() {
        let mut builder = PricingKernelBuilder::new();
        builder.add_floating_cashflow(19000, 18900, 0.5, 1_000_000.0, 0.001, 1.0, 0, 0, 1);

        let kernel = builder.build().unwrap();
        assert!((kernel.gearings[0] - 1.0).abs() < 1e-10);
        assert_eq!(kernel.fwd_index_ids[0], 1);
    }

    #[test]
    fn test_builder_sort_by_payment_date() {
        let mut builder = PricingKernelBuilder::new();
        builder.add_fixed_cashflow(19180, 19080, 0.5, 1_000_000.0, 0.05, 0, 0);
        builder.add_fixed_cashflow(19000, 18900, 0.5, 1_000_000.0, 0.05, 0, 0);
        builder.add_fixed_cashflow(19365, 19265, 0.5, 1_000_000.0, 0.05, 0, 0);

        builder.sort_by_payment_date();

        let kernel = builder.build().unwrap();
        assert_eq!(kernel.payment_dates[0], 19000);
        assert_eq!(kernel.payment_dates[1], 19180);
        assert_eq!(kernel.payment_dates[2], 19365);
    }

    #[test]
    fn test_builder_trade_count() {
        let mut builder = PricingKernelBuilder::new();
        builder.add_fixed_cashflow(19000, 18900, 0.5, 1_000_000.0, 0.05, 0, 0);
        builder.increment_trade_count();
        builder.add_fixed_cashflow(19180, 19080, 0.5, 1_000_000.0, 0.05, 0, 0);
        builder.increment_trade_count();

        let kernel = builder.build().unwrap();
        assert_eq!(kernel.trade_count(), 2);
    }

    #[test]
    fn test_builder_with_capacity() {
        let builder = PricingKernelBuilder::with_capacity(100);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_builder_build_empty() {
        let builder = PricingKernelBuilder::new();
        let kernel = builder.build().unwrap();
        assert!(kernel.is_empty());
    }
}

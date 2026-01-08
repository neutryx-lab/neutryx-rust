//! Multi-curve management for derivatives pricing.
//!
//! This module provides:
//! - [`CurveSet`]: Container for managing multiple named yield curves

use super::{CurveEnum, CurveName};
use crate::market_data::error::MarketDataError;
use num_traits::Float;
use std::collections::HashMap;

/// Container for managing multiple named yield curves.
///
/// `CurveSet` provides a unified interface for storing and retrieving
/// yield curves used in derivatives pricing. It supports the multi-curve
/// framework where discount curves and forward curves are managed separately.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Multi-Curve Framework
///
/// In modern derivatives pricing, different curves are used for different purposes:
/// - **Discount curves**: Used to compute present values (typically OIS curves)
/// - **Forward curves**: Used to project future cash flows (e.g., SOFR, EURIBOR)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum, FlatCurve, YieldCurve};
///
/// // Create a curve set
/// let mut curves = CurveSet::new();
///
/// // Add discount curve (OIS)
/// curves.insert(CurveName::Ois, CurveEnum::flat(0.03));
///
/// // Add forward curve (SOFR)
/// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035));
///
/// // Set default discount curve
/// curves.set_discount_curve(CurveName::Ois);
///
/// // Get curves
/// let discount = curves.discount_curve().unwrap();
/// let df = discount.discount_factor(1.0).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct CurveSet<T: Float> {
    /// Named curves stored in a HashMap
    curves: HashMap<CurveName, CurveEnum<T>>,
    /// Default discount curve name
    discount_curve_name: Option<CurveName>,
}

impl<T: Float> Default for CurveSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> CurveSet<T> {
    /// Create an empty curve set.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::CurveSet;
    ///
    /// let curves: CurveSet<f64> = CurveSet::new();
    /// assert!(curves.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            curves: HashMap::new(),
            discount_curve_name: None,
        }
    }

    /// Create a curve set with a single flat discount curve.
    ///
    /// This is a convenience constructor for simple scenarios
    /// where only a single discount rate is needed.
    ///
    /// # Arguments
    ///
    /// * `rate` - The flat discount rate
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, YieldCurve};
    ///
    /// let curves = CurveSet::with_flat_discount(0.05_f64);
    /// let discount = curves.discount_curve().unwrap();
    /// let df = discount.discount_factor(1.0).unwrap();
    /// assert!((df - 0.951229).abs() < 1e-5);
    /// ```
    pub fn with_flat_discount(rate: T) -> Self {
        let mut set = Self::new();
        set.insert(CurveName::Discount, CurveEnum::flat(rate));
        set.set_discount_curve(CurveName::Discount);
        set
    }

    /// Insert a curve with the given name.
    ///
    /// If a curve with the same name already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name
    /// * `curve` - The yield curve to insert
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    /// assert!(curves.contains(&CurveName::Sofr));
    /// ```
    #[inline]
    pub fn insert(&mut self, name: CurveName, curve: CurveEnum<T>) {
        self.curves.insert(name, curve);
    }

    /// Get a curve by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name to look up
    ///
    /// # Returns
    ///
    /// * `Some(&CurveEnum)` - Reference to the curve if found
    /// * `None` - If no curve with that name exists
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum, YieldCurve};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    ///
    /// let sofr = curves.get(&CurveName::Sofr).unwrap();
    /// let rate = sofr.zero_rate(1.0).unwrap();
    /// assert!((rate - 0.035).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn get(&self, name: &CurveName) -> Option<&CurveEnum<T>> {
        self.curves.get(name)
    }

    /// Get a curve by name, returning an error if not found.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name to look up
    ///
    /// # Returns
    ///
    /// * `Ok(&CurveEnum)` - Reference to the curve
    /// * `Err(MarketDataError::CurveNotFound)` - If no curve with that name exists
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    ///
    /// let result = curves.get_or_err(&CurveName::Tonar);
    /// assert!(result.is_err());
    /// ```
    pub fn get_or_err(&self, name: &CurveName) -> Result<&CurveEnum<T>, MarketDataError> {
        self.curves
            .get(name)
            .ok_or(MarketDataError::CurveNotFound { name: *name })
    }

    /// Check if a curve with the given name exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name to check
    ///
    /// # Returns
    ///
    /// `true` if the curve exists, `false` otherwise
    #[inline]
    pub fn contains(&self, name: &CurveName) -> bool {
        self.curves.contains_key(name)
    }

    /// Remove a curve by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name to remove
    ///
    /// # Returns
    ///
    /// * `Some(CurveEnum)` - The removed curve if it existed
    /// * `None` - If no curve with that name existed
    #[inline]
    pub fn remove(&mut self, name: &CurveName) -> Option<CurveEnum<T>> {
        // Clear discount curve reference if removing the discount curve
        if Some(*name) == self.discount_curve_name {
            self.discount_curve_name = None;
        }
        self.curves.remove(name)
    }

    /// Return the number of curves in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.curves.len()
    }

    /// Check if the curve set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
    }

    /// Set the default discount curve name.
    ///
    /// This designates which curve should be used for discounting
    /// when calling `discount_curve()`.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name to use as the default discount curve
    ///
    /// # Note
    ///
    /// This does not verify that the curve exists. The curve must be
    /// inserted before or after calling this method.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
    /// curves.set_discount_curve(CurveName::Ois);
    /// ```
    #[inline]
    pub fn set_discount_curve(&mut self, name: CurveName) {
        self.discount_curve_name = Some(name);
    }

    /// Get the default discount curve.
    ///
    /// Returns the curve designated as the discount curve via
    /// `set_discount_curve()`, or looks for a curve named `Discount`.
    ///
    /// # Returns
    ///
    /// * `Some(&CurveEnum)` - The discount curve if found
    /// * `None` - If no discount curve is configured
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum, YieldCurve};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
    /// curves.set_discount_curve(CurveName::Ois);
    ///
    /// let discount = curves.discount_curve().unwrap();
    /// ```
    pub fn discount_curve(&self) -> Option<&CurveEnum<T>> {
        // First try the explicitly set discount curve
        if let Some(name) = self.discount_curve_name {
            if let Some(curve) = self.curves.get(&name) {
                return Some(curve);
            }
        }
        // Fall back to looking for a curve named Discount
        self.curves.get(&CurveName::Discount)
    }

    /// Get the default discount curve, returning an error if not found.
    ///
    /// # Returns
    ///
    /// * `Ok(&CurveEnum)` - The discount curve
    /// * `Err(MarketDataError::CurveNotFound)` - If no discount curve is configured
    pub fn discount_curve_or_err(&self) -> Result<&CurveEnum<T>, MarketDataError> {
        self.discount_curve().ok_or(MarketDataError::CurveNotFound {
            name: CurveName::Discount,
        })
    }

    /// Get a forward curve by name.
    ///
    /// This is a semantic alias for `get()` that makes the intent clearer
    /// when retrieving curves used for forward rate projections.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name (e.g., `CurveName::Sofr`, `CurveName::Euribor`)
    ///
    /// # Returns
    ///
    /// * `Some(&CurveEnum)` - The forward curve if found
    /// * `None` - If no curve with that name exists
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum, YieldCurve};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    ///
    /// let forward = curves.forward_curve(&CurveName::Sofr).unwrap();
    /// let fwd_rate = forward.forward_rate(1.0, 2.0).unwrap();
    /// ```
    #[inline]
    pub fn forward_curve(&self, name: &CurveName) -> Option<&CurveEnum<T>> {
        self.curves.get(name)
    }

    /// Get a forward curve by name, returning an error if not found.
    ///
    /// # Arguments
    ///
    /// * `name` - The curve name
    ///
    /// # Returns
    ///
    /// * `Ok(&CurveEnum)` - The forward curve
    /// * `Err(MarketDataError::CurveNotFound)` - If no curve with that name exists
    pub fn forward_curve_or_err(&self, name: &CurveName) -> Result<&CurveEnum<T>, MarketDataError> {
        self.curves
            .get(name)
            .ok_or(MarketDataError::CurveNotFound { name: *name })
    }

    /// Iterate over all curves in the set.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    ///
    /// for (name, curve) in curves.iter() {
    ///     println!("{}: {:?}", name, curve);
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&CurveName, &CurveEnum<T>)> {
        self.curves.iter()
    }

    /// Get all curve names in the set.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{CurveSet, CurveName, CurveEnum};
    ///
    /// let mut curves = CurveSet::new();
    /// curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
    /// curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
    ///
    /// let names: Vec<_> = curves.curve_names().collect();
    /// assert_eq!(names.len(), 2);
    /// ```
    #[inline]
    pub fn curve_names(&self) -> impl Iterator<Item = &CurveName> {
        self.curves.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::curves::{CurveInterpolation, InterpolatedCurve, YieldCurve};

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_new_empty() {
        let curves: CurveSet<f64> = CurveSet::new();
        assert!(curves.is_empty());
        assert_eq!(curves.len(), 0);
    }

    #[test]
    fn test_default() {
        let curves: CurveSet<f64> = CurveSet::default();
        assert!(curves.is_empty());
    }

    #[test]
    fn test_with_flat_discount() {
        let curves = CurveSet::with_flat_discount(0.05_f64);
        assert!(!curves.is_empty());
        assert!(curves.contains(&CurveName::Discount));

        let discount = curves.discount_curve().unwrap();
        let df = discount.discount_factor(1.0).unwrap();
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    // ========================================
    // Insert/Get Tests
    // ========================================

    #[test]
    fn test_insert_and_get() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        assert!(curves.contains(&CurveName::Sofr));
        assert!(!curves.contains(&CurveName::Tonar));
        assert_eq!(curves.len(), 1);

        let sofr = curves.get(&CurveName::Sofr).unwrap();
        let rate = sofr.zero_rate(1.0).unwrap();
        assert!((rate - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_insert_replaces_existing() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.03_f64));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.04_f64));

        assert_eq!(curves.len(), 1);

        let sofr = curves.get(&CurveName::Sofr).unwrap();
        let rate = sofr.zero_rate(1.0).unwrap();
        assert!((rate - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_get_nonexistent() {
        let curves: CurveSet<f64> = CurveSet::new();
        assert!(curves.get(&CurveName::Sofr).is_none());
    }

    #[test]
    fn test_get_or_err_existing() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        let result = curves.get_or_err(&CurveName::Sofr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_or_err_nonexistent() {
        let curves: CurveSet<f64> = CurveSet::new();
        let result = curves.get_or_err(&CurveName::Sofr);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::CurveNotFound { name } => {
                assert_eq!(name, CurveName::Sofr);
            }
            _ => panic!("Expected CurveNotFound error"),
        }
    }

    // ========================================
    // Remove Tests
    // ========================================

    #[test]
    fn test_remove_existing() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        let removed = curves.remove(&CurveName::Sofr);
        assert!(removed.is_some());
        assert!(!curves.contains(&CurveName::Sofr));
        assert!(curves.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut curves: CurveSet<f64> = CurveSet::new();
        let removed = curves.remove(&CurveName::Sofr);
        assert!(removed.is_none());
    }

    #[test]
    fn test_remove_clears_discount_reference() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
        curves.set_discount_curve(CurveName::Ois);

        // Verify discount curve works
        assert!(curves.discount_curve().is_some());

        // Remove the discount curve
        curves.remove(&CurveName::Ois);

        // Discount curve should now return None
        assert!(curves.discount_curve().is_none());
    }

    // ========================================
    // Discount Curve Tests
    // ========================================

    #[test]
    fn test_discount_curve_explicit() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.set_discount_curve(CurveName::Ois);

        let discount = curves.discount_curve().unwrap();
        let rate = discount.zero_rate(1.0).unwrap();
        assert!((rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_discount_curve_fallback() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.025_f64));

        // Without explicitly setting discount curve, should find Discount
        let discount = curves.discount_curve().unwrap();
        let rate = discount.zero_rate(1.0).unwrap();
        assert!((rate - 0.025).abs() < 1e-10);
    }

    #[test]
    fn test_discount_curve_none() {
        let curves: CurveSet<f64> = CurveSet::new();
        assert!(curves.discount_curve().is_none());
    }

    #[test]
    fn test_discount_curve_or_err() {
        let curves: CurveSet<f64> = CurveSet::new();
        let result = curves.discount_curve_or_err();
        assert!(result.is_err());
    }

    // ========================================
    // Forward Curve Tests
    // ========================================

    #[test]
    fn test_forward_curve() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.insert(CurveName::Euribor, CurveEnum::flat(0.04_f64));

        let sofr = curves.forward_curve(&CurveName::Sofr).unwrap();
        let rate = sofr.zero_rate(1.0).unwrap();
        assert!((rate - 0.035).abs() < 1e-10);

        let euribor = curves.forward_curve(&CurveName::Euribor).unwrap();
        let rate = euribor.zero_rate(1.0).unwrap();
        assert!((rate - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_forward_curve_or_err() {
        let curves: CurveSet<f64> = CurveSet::new();
        let result = curves.forward_curve_or_err(&CurveName::Sofr);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::CurveNotFound { name } => {
                assert_eq!(name, CurveName::Sofr);
            }
            _ => panic!("Expected CurveNotFound error"),
        }
    }

    // ========================================
    // Iteration Tests
    // ========================================

    #[test]
    fn test_iter() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        let count = curves.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_curve_names() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        let names: Vec<_> = curves.curve_names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&CurveName::Ois));
        assert!(names.contains(&&CurveName::Sofr));
    }

    // ========================================
    // With Interpolated Curves
    // ========================================

    #[test]
    fn test_with_interpolated_curve() {
        let tenors = [0.5_f64, 1.0, 2.0, 5.0];
        let rates = [0.02, 0.025, 0.03, 0.035];
        let interp =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::Interpolated(interp));

        let sofr = curves.get(&CurveName::Sofr).unwrap();
        let rate = sofr.zero_rate(1.0).unwrap();
        assert!((rate - 0.025).abs() < 1e-10);
    }

    // ========================================
    // Custom Curve Names
    // ========================================

    #[test]
    fn test_custom_curve_name() {
        let mut curves = CurveSet::new();
        curves.insert(
            CurveName::Custom("MY_SPECIAL_CURVE"),
            CurveEnum::flat(0.05_f64),
        );

        assert!(curves.contains(&CurveName::Custom("MY_SPECIAL_CURVE")));

        let custom = curves.get(&CurveName::Custom("MY_SPECIAL_CURVE")).unwrap();
        let rate = custom.zero_rate(1.0).unwrap();
        assert!((rate - 0.05).abs() < 1e-10);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));
        curves.set_discount_curve(CurveName::Sofr);

        let cloned = curves.clone();
        assert_eq!(cloned.len(), curves.len());

        let sofr = cloned.get(&CurveName::Sofr).unwrap();
        let rate = sofr.zero_rate(1.0).unwrap();
        assert!((rate - 0.035).abs() < 1e-10);
    }

    // ========================================
    // Multi-Curve Usage Pattern
    // ========================================

    #[test]
    fn test_multi_curve_pattern() {
        // This test demonstrates a typical multi-curve setup
        let mut curves = CurveSet::new();

        // OIS curve for discounting (typically lower rates)
        curves.insert(CurveName::Ois, CurveEnum::flat(0.03_f64));

        // SOFR curve for forward projections (typically higher spread)
        curves.insert(CurveName::Sofr, CurveEnum::flat(0.035_f64));

        // Set OIS as the discount curve
        curves.set_discount_curve(CurveName::Ois);

        // Calculate discount factor using OIS
        let discount = curves.discount_curve().unwrap();
        let df = discount.discount_factor(1.0).unwrap();

        // Calculate forward rate using SOFR
        let forward = curves.forward_curve(&CurveName::Sofr).unwrap();
        let fwd_rate = forward.forward_rate(0.5, 1.0).unwrap();

        // Verify rates
        assert!((df - (-0.03_f64).exp()).abs() < 1e-10);
        assert!((fwd_rate - 0.035).abs() < 1e-10);
    }
}

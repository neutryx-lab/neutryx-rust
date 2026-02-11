//! Shadow Object pattern for Enzyme AAD gradient accumulation.

/// Shadow trait for gradient object generation via clone + zero_out.
pub trait Shadow: Clone {
    /// Reset all numeric fields to zero.
    fn zero_out(&mut self);

    /// Create a shadow object for gradient accumulation.
    #[inline]
    fn create_shadow(&self) -> Self {
        let mut shadow = self.clone();
        shadow.zero_out();
        shadow
    }
}

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

/// Simple yield curve for Shadow Object AAD.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleYieldCurve {
    /// Zero rates at pillar tenors (Active input for AAD)
    pub rates: Vec<f64>,
    /// Tenor points in years (Const input for AAD)
    pub times: Vec<f64>,
}

impl SimpleYieldCurve {
    /// Create a new yield curve from rates and times.
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
    #[inline]
    fn zero_out(&mut self) {
        self.rates.zero_out();
        // times are const, do not zero
    }
}

/// Simple volatility surface for Shadow Object AAD.
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
    pub fn vols_flat(&self) -> Vec<f64> { self.vols.iter().flatten().copied().collect() }
}

impl Shadow for SimpleVolSurface {
    #[inline]
    fn zero_out(&mut self) {
        self.vols.zero_out();
        // strikes and expiries are const, do not zero
    }
}

/// Combined market data for Shadow Object AAD.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleMarketData {
    /// Shadow discount curve.
    pub discount_curve: SimpleYieldCurve,
    /// Shadow forward curve.
    pub forward_curve: Option<SimpleYieldCurve>,
    /// Shadow volatility surface.
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

mod global_bootstrap_shadow {
    use pricer_models::builder::GlobalBootstrapResult;

    use super::Shadow;

    /// Shadow implementation for GlobalBootstrapResult<f64> enabling IFT-based
    /// curve sensitivity computation.
    impl Shadow for GlobalBootstrapResult<f64> {
        fn zero_out(&mut self) {
            self.discount_factors.zero_out();
            self.residual_norm = 0.0;

            if let Some(ref mut errors) = self.pricing_errors {
                errors.zero_out();
            }

            // The following are CONST (not zeroed):
            // - curve, jacobian_inverse, pillars, iterations,
            // - converged, residual_history, condition_number, realised_jumps
        }
    }
}

#[allow(unused_imports)]
pub use global_bootstrap_shadow::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_zero_out() {
        let mut val = 42.0_f64;
        val.zero_out();
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_f64_create_shadow() {
        let val = 42.0_f64;
        let shadow = val.create_shadow();

        assert_eq!(shadow, 0.0);
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
        let mut vec = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        vec.zero_out();

        assert_eq!(vec.len(), 5);
        for &v in &vec {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn test_vec_f64_create_shadow() {
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let shadow = original.create_shadow();

        assert_eq!(shadow.len(), original.len());

        for &v in &shadow {
            assert_eq!(v, 0.0);
        }

        assert_eq!(original, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_vec_f64_empty() {
        let mut vec: Vec<f64> = vec![];
        vec.zero_out();
        assert!(vec.is_empty());

        let shadow = vec.create_shadow();
        assert!(shadow.is_empty());
    }

    #[test]
    fn test_vec_vec_f64_zero_out() {
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

        assert_eq!(shadow.len(), 2);
        assert_eq!(shadow[0].len(), 3);
        assert_eq!(shadow[1].len(), 3);

        for row in &shadow {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        assert_eq!(original[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(original[1], vec![4.0, 5.0, 6.0]);
    }

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
        let mut vec: Vec<f64> = (0..10000).map(|i| i as f64).collect();
        vec.zero_out();

        for &v in &vec {
            assert_eq!(v, 0.0);
        }
    }

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
        let mut curve = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);

        curve.zero_out();

        assert_eq!(curve.rates, vec![0.0, 0.0, 0.0]);
        assert_eq!(curve.times, vec![1.0, 2.0, 5.0]);
    }

    #[test]
    fn test_simple_yield_curve_create_shadow() {
        let original = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);

        let shadow = original.create_shadow();

        assert_eq!(shadow.len(), original.len());
        assert_eq!(shadow.times, original.times);

        for &r in &shadow.rates {
            assert_eq!(r, 0.0);
        }

        assert_eq!(original.rates, vec![0.02, 0.03, 0.04]);
    }

    #[test]
    fn test_simple_yield_curve_gradient_mapping() {
        let market = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);
        let mut d_market = market.create_shadow();

        d_market.rates[0] = 1.5;
        d_market.rates[1] = 2.3;
        d_market.rates[2] = 0.7;

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

        for row in &surface.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        assert_eq!(surface.strikes, vec![90.0, 100.0, 110.0]);
        assert_eq!(surface.expiries, vec![0.5, 1.0]);
    }

    #[test]
    fn test_simple_vol_surface_create_shadow() {
        let original = SimpleVolSurface::new(
            vec![vec![0.20, 0.22, 0.25], vec![0.21, 0.23, 0.26]],
            vec![90.0, 100.0, 110.0],
            vec![0.5, 1.0],
        );

        let shadow = original.create_shadow();

        assert_eq!(shadow.n_expiries(), original.n_expiries());
        assert_eq!(shadow.n_strikes(), original.n_strikes());
        assert_eq!(shadow.strikes, original.strikes);
        assert_eq!(shadow.expiries, original.expiries);

        for row in &shadow.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

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

        assert_eq!(market.discount_curve.rates, vec![0.0, 0.0]);
        assert_eq!(market.forward_curve.as_ref().unwrap().rates, vec![0.0, 0.0]);
        for row in &market.vol_surface.as_ref().unwrap().vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }

        assert_eq!(market.discount_curve.times, vec![1.0, 2.0]);
        assert_eq!(
            market.vol_surface.as_ref().unwrap().strikes,
            vec![100.0, 110.0]
        );
    }

    #[test]
    fn test_simple_market_data_create_shadow() {
        let discount = SimpleYieldCurve::new(vec![0.02, 0.03], vec![1.0, 2.0]);
        let forward = SimpleYieldCurve::new(vec![0.025, 0.035], vec![1.0, 2.0]);

        let market = SimpleMarketData::with_discount_curve(discount).with_forward_curve(forward);

        let shadow = market.create_shadow();

        assert_eq!(shadow.discount_curve.len(), market.discount_curve.len());
        assert_eq!(
            shadow.forward_curve.as_ref().unwrap().len(),
            market.forward_curve.as_ref().unwrap().len()
        );

        assert_eq!(shadow.discount_curve.rates, vec![0.0, 0.0]);
        assert_eq!(shadow.forward_curve.as_ref().unwrap().rates, vec![0.0, 0.0]);

        assert_eq!(market.discount_curve.rates, vec![0.02, 0.03]);
    }

    #[test]
    fn test_gradient_named_field_access() {
        let market = SimpleMarketData::with_discount_curve(SimpleYieldCurve::new(
            vec![0.02, 0.03],
            vec![1.0, 2.0],
        ));

        let mut d_market = market.create_shadow();

        d_market.discount_curve.rates[0] = 1.5;

        assert_eq!(d_market.discount_curve.rates[0], 1.5);
    }

    mod global_bootstrap_tests {
        use nalgebra::DMatrix;
        use pricer_models::{
            builder::GlobalBootstrapResult,
            market::curves::{BootstrapInterpolation, BootstrappedCurve},
        };

        use super::*;

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
            let mut result = create_test_result();

            let original_pillars = result.pillars.clone();
            let original_iterations = result.iterations;
            let original_jacobian = result.jacobian_inverse.clone();

            result.zero_out();

            assert_eq!(result.discount_factors, vec![0.0, 0.0, 0.0]);
            assert_eq!(result.residual_norm, 0.0);
            assert_eq!(result.pricing_errors, Some(vec![0.0, 0.0, 0.0]));

            assert_eq!(result.pillars, original_pillars);
            assert_eq!(result.iterations, original_iterations);
            assert_eq!(result.jacobian_inverse, original_jacobian);
            assert!(result.converged);
        }

        #[test]
        fn test_global_bootstrap_result_create_shadow() {
            let original = create_test_result();
            let shadow = original.create_shadow();

            assert_eq!(shadow.discount_factors, vec![0.0, 0.0, 0.0]);
            assert_eq!(shadow.residual_norm, 0.0);
            assert_eq!(shadow.pricing_errors, Some(vec![0.0, 0.0, 0.0]));

            assert_eq!(original.discount_factors, vec![0.97, 0.94, 0.85]);
            assert!((original.residual_norm - 1e-10).abs() < 1e-15);
            assert_eq!(original.pricing_errors, Some(vec![1e-10, 2e-10, 3e-10]));

            assert_eq!(shadow.pillars.len(), original.pillars.len());
            assert_eq!(shadow.iterations, original.iterations);
        }

        #[test]
        fn test_global_bootstrap_result_jacobian_inverse_preserved() {
            let mut result = create_test_result();
            let original_j_inv = result.jacobian_inverse.clone();

            result.zero_out();

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
                pricing_errors: None,
                realised_jumps: None,
            };

            result.zero_out();

            assert_eq!(result.discount_factors, vec![0.0, 0.0]);
            assert_eq!(result.residual_norm, 0.0);
            assert!(result.pricing_errors.is_none());
        }

        #[test]
        fn test_global_bootstrap_result_gradient_accumulation() {
            let original = create_test_result();
            let mut d_result = original.create_shadow();

            d_result.discount_factors[0] = 0.5;
            d_result.discount_factors[1] = 0.3;
            d_result.discount_factors[2] = 0.2;

            assert_eq!(d_result.discount_factors[0], 0.5);
            assert_eq!(d_result.discount_factors[1], 0.3);
            assert_eq!(d_result.discount_factors[2], 0.2);
        }
    }
}

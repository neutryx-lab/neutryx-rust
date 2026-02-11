//! AAD Binder Layer for Shadow Object pattern.

use thiserror::Error;

use super::{
    kernel::{finite_difference_gradients, pricing_kernel_irs},
    shadow::{Shadow, SimpleMarketData, SimpleYieldCurve},
};

/// Errors in Shadow AAD operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ShadowAadError {
    /// Input slices have mismatched lengths.
    #[error("Length mismatch: expected {expected}, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    /// Input slice is empty when non-empty is required.
    #[error("Empty slice: {field}")]
    EmptySlice { field: &'static str },

    /// Enzyme AD is not available (feature not enabled).
    #[error("Enzyme AD not available, using finite difference fallback")]
    EnzymeNotAvailable,

    /// Invalid market data.
    #[error("Invalid market data: {message}")]
    InvalidMarketData { message: String },
}

/// Activity mask for partial differentiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityMask {
    pub rates_active: bool,
    pub volatilities_active: bool,
    pub fx_rates_active: bool,
}

impl Default for ActivityMask {
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

/// Result of AAD risk calculation containing PV and gradient shadow object.
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

/// Parameters for an Interest Rate Swap trade.
#[derive(Debug, Clone)]
pub struct IrsTradeParams {
    pub notionals: Vec<f64>,
    pub year_fractions: Vec<f64>,
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

/// Calculator for market risk using Shadow Object AAD.
#[derive(Debug, Clone)]
pub struct MarketRiskCalculator {
    pub bump_size: f64,
}

impl Default for MarketRiskCalculator {
    fn default() -> Self { Self { bump_size: 1e-7 } }
}

impl MarketRiskCalculator {
    /// Create a new calculator with custom bump size.
    pub fn with_bump_size(bump_size: f64) -> Self { Self { bump_size } }

    /// Calculate risk for an IRS trade using Shadow Object AAD.
    pub fn calculate_irs_risk(
        &self,
        market: &SimpleYieldCurve,
        trade: &IrsTradeParams,
        mask: ActivityMask,
    ) -> Result<RiskResult<SimpleYieldCurve>, ShadowAadError> {
        if market.is_empty() {
            return Err(ShadowAadError::EmptySlice { field: "rates" });
        }
        if market.len() != trade.notionals.len() {
            return Err(ShadowAadError::LengthMismatch {
                expected: market.len(),
                actual: trade.notionals.len(),
            });
        }

        let mut pv = 0.0;
        pricing_kernel_irs(
            market.rates_slice(),
            market.times_slice(),
            &trade.notionals,
            &trade.year_fractions,
            trade.fixed_rate,
            &mut pv,
        );

        let mut shadow = market.create_shadow();

        if mask.rates_active {
            let gradients = finite_difference_gradients(
                pricing_kernel_irs,
                market.rates_slice(),
                market.times_slice(),
                &trade.notionals,
                &trade.year_fractions,
                trade.fixed_rate,
                self.bump_size,
            );

            for (i, &grad) in gradients.iter().enumerate() {
                shadow.rates[i] = grad;
            }
        }

        Ok(RiskResult::new(pv, shadow))
    }

    /// Calculate risk for market data with multiple curves.
    pub fn calculate_full_market_risk(
        &self,
        market: &SimpleMarketData,
        trade: &IrsTradeParams,
        mask: ActivityMask,
    ) -> Result<RiskResult<SimpleMarketData>, ShadowAadError> {
        if market.discount_curve.is_empty() {
            return Err(ShadowAadError::EmptySlice {
                field: "discount_curve.rates",
            });
        }

        let mut pv = 0.0;
        pricing_kernel_irs(
            market.discount_curve.rates_slice(),
            market.discount_curve.times_slice(),
            &trade.notionals,
            &trade.year_fractions,
            trade.fixed_rate,
            &mut pv,
        );

        let mut shadow = market.create_shadow();

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

        Ok(RiskResult::new(pv, shadow))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_risk_result_creation() {
        let gradients = SimpleYieldCurve::new(vec![100.0, 200.0], vec![1.0, 2.0]);
        let result = RiskResult::new(1_000_000.0, gradients);

        assert_eq!(result.pv, 1_000_000.0);
        assert_eq!(result.gradients.rates, vec![100.0, 200.0]);
    }

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

        assert!(
            result.pv.abs() < 1.0,
            "ATM swap PV should be near zero, got {}",
            result.pv
        );

        for (i, &grad) in result.gradients.rates.iter().enumerate() {
            assert!(grad.abs() > 0.0, "Gradient {} should be non-zero", i);
        }
    }

    #[test]
    fn test_calculate_irs_risk_positive_pv() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.05, 0.05, 0.05], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        assert!(result.pv > 0.0, "Expected positive PV, got {}", result.pv);
    }

    #[test]
    fn test_calculate_irs_risk_const_rates() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03, 0.03, 0.03], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::none())
            .unwrap();

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
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 2);

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

        for &grad in &result.gradients.discount_curve.rates {
            assert!(grad.abs() > 0.0);
        }

        for &grad in &result.gradients.forward_curve.as_ref().unwrap().rates {
            assert_eq!(grad, 0.0);
        }
    }

    #[test]
    fn test_gradient_magnitude() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.03], vec![1.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 1);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        let grad_magnitude = result.gradients.rates[0].abs();
        assert!(
            grad_magnitude > 100_000.0 && grad_magnitude < 2_000_000.0,
            "Gradient magnitude {} outside expected range",
            grad_magnitude
        );
    }

    #[test]
    fn test_gradient_structure_preserved() {
        let calc = MarketRiskCalculator::default();
        let market = SimpleYieldCurve::new(vec![0.02, 0.03, 0.04], vec![1.0, 2.0, 5.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.03, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        assert_eq!(result.gradients.len(), market.len());
        assert_eq!(result.gradients.times, market.times);
    }

    #[test]
    fn test_yield_curve_delta_calculation() {
        let calc = MarketRiskCalculator::default();

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

        assert_eq!(result.gradients.rates.len(), 6);

        for &grad in &result.gradients.rates {
            assert!(
                grad.abs() > 0.0,
                "All rate sensitivities should be non-zero"
            );
        }
    }

    #[test]
    fn test_yield_curve_large_scale() {
        let calc = MarketRiskCalculator::default();

        let n = 100;
        let tenors: Vec<f64> = (1..=n).map(|i| i as f64 * 0.1).collect();
        let rates: Vec<f64> = (1..=n).map(|i| 0.02 + 0.0002 * i as f64).collect();
        let market = SimpleYieldCurve::new(rates.clone(), tenors.clone());

        let trade = IrsTradeParams::uniform(1_000_000.0, 0.1, 0.035, n);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::default())
            .unwrap();

        assert_eq!(result.gradients.rates.len(), n);

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
        use std::time::Instant;

        let n = 1000;
        let rates: Vec<f64> = (0..n).map(|i| 0.02 + 0.00001 * i as f64).collect();
        let times: Vec<f64> = (1..=n).map(|i| i as f64 * 0.01).collect();
        let market = SimpleYieldCurve::new(rates, times);

        let start = Instant::now();
        for _ in 0..1000 {
            let _shadow = market.create_shadow();
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "Shadow creation overhead too high: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_vol_surface_structure() {
        use super::super::shadow::{Shadow, SimpleVolSurface};

        let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
        let expiries = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let vols = vec![
            vec![0.25, 0.22, 0.20, 0.22, 0.25],
            vec![0.24, 0.21, 0.19, 0.21, 0.24],
            vec![0.23, 0.20, 0.18, 0.20, 0.23],
            vec![0.22, 0.19, 0.17, 0.19, 0.22],
            vec![0.21, 0.18, 0.16, 0.18, 0.21],
        ];

        let surface = SimpleVolSurface::new(vols.clone(), strikes.clone(), expiries.clone());

        assert_eq!(surface.n_expiries(), 5);
        assert_eq!(surface.n_strikes(), 5);

        let shadow = surface.create_shadow();

        assert_eq!(shadow.n_expiries(), surface.n_expiries());
        assert_eq!(shadow.n_strikes(), surface.n_strikes());
        assert_eq!(shadow.strikes, surface.strikes);
        assert_eq!(shadow.expiries, surface.expiries);

        for row in &shadow.vols {
            for &v in row {
                assert_eq!(v, 0.0);
            }
        }
    }

    #[test]
    fn test_vol_surface_gradient_mapping() {
        use super::super::shadow::{Shadow, SimpleVolSurface};

        let surface = SimpleVolSurface::new(
            vec![vec![0.20, 0.22], vec![0.21, 0.23]],
            vec![100.0, 110.0],
            vec![0.5, 1.0],
        );

        let mut shadow = surface.create_shadow();

        *shadow.vol_mut(0, 0) = 1.5;
        *shadow.vol_mut(1, 1) = 2.3;

        assert_eq!(shadow.vol(0, 0), 1.5);
        assert_eq!(shadow.vol(1, 1), 2.3);
        assert_eq!(shadow.vol(0, 1), 0.0);
    }

    #[test]
    fn test_fallback_without_enzyme() {
        let calc = MarketRiskCalculator::with_bump_size(1e-8);
        let market = SimpleYieldCurve::new(vec![0.03, 0.04, 0.05], vec![1.0, 2.0, 3.0]);
        let trade = IrsTradeParams::uniform(1_000_000.0, 1.0, 0.04, 3);

        let result = calc
            .calculate_irs_risk(&market, &trade, ActivityMask::rates_only())
            .unwrap();

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

mod global_bootstrap_integration {
    use pricer_models::builder::{GlobalBootstrapResult, IftError};

    use super::*;

    /// Error type for IFT-based risk calculations.
    #[derive(Debug, Error, Clone, PartialEq)]
    pub enum IftRiskError {
        /// IFT computation error (forwarded from GlobalBootstrapResult).
        #[error("IFT computation error: {0}")]
        IftError(#[from] IftError),

        /// Curve mismatch - trade references different curve than provided.
        #[error("Curve mismatch: trade expects {expected}, got {actual}")]
        CurveMismatch { expected: String, actual: String },

        /// Empty trade batch provided.
        #[error("Empty trade batch")]
        EmptyBatch,

        /// Dimension mismatch in batch processing.
        #[error("Batch dimension mismatch: {message}")]
        BatchDimensionMismatch { message: String },

        /// Shadow AAD error during computation.
        #[error("Shadow AAD error: {0}")]
        ShadowAadError(#[from] ShadowAadError),
    }

    /// Result of IFT-based curve sensitivity calculation.
    #[derive(Debug, Clone)]
    pub struct IftRiskResult {
        pub pv: f64,
        pub df_sensitivities: Vec<f64>,
        pub market_sensitivities: Vec<f64>,
    }

    /// Result of batched IFT risk calculation across multiple trades.
    #[derive(Debug, Clone)]
    pub struct BatchIftRiskResult {
        pub trade_results: Vec<IftRiskResult>,
        pub total_market_sensitivities: Vec<f64>,
        pub total_pv: f64,
    }

    /// Trait for trades that can compute sensitivities to discount factors.
    pub trait IftTrade {
        /// Compute the present value and sensitivities to discount factors.
        fn compute_with_df_sensitivities(
            &self,
            discount_factors: &[f64],
            pillars: &[f64],
        ) -> (f64, Vec<f64>);
    }

    impl MarketRiskCalculator {
        /// Calculate IFT-based market risk using GlobalBootstrapResult.
        pub fn calculate_ift_risk<T: IftTrade>(
            &self,
            bootstrap_result: &GlobalBootstrapResult<f64>,
            trade: &T,
            dF_dquote: &[Vec<f64>],
        ) -> Result<IftRiskResult, IftRiskError> {
            if !bootstrap_result.can_compute_ift() {
                return Err(IftRiskError::IftError(IftError::NoJacobianInverse));
            }

            let (pv, df_sensitivities) = trade.compute_with_df_sensitivities(
                &bootstrap_result.discount_factors,
                &bootstrap_result.pillars,
            );

            let mut market_sensitivities = Vec::with_capacity(dF_dquote.len());

            for dF_dq in dF_dquote {
                let df_dquote = bootstrap_result.ift_sensitivity(dF_dq)?;

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
        pub fn calculate_batch_ift_risk<T: IftTrade>(
            &self,
            bootstrap_result: &GlobalBootstrapResult<f64>,
            trades: &[T],
            dF_dquote: &[Vec<f64>],
        ) -> Result<BatchIftRiskResult, IftRiskError> {
            if trades.is_empty() {
                return Err(IftRiskError::EmptyBatch);
            }

            if !bootstrap_result.can_compute_ift() {
                return Err(IftRiskError::IftError(IftError::NoJacobianInverse));
            }

            let n_quotes = dF_dquote.len();
            let mut df_dquote_cache: Vec<Vec<f64>> = Vec::with_capacity(n_quotes);

            for dF_dq in dF_dquote {
                let df_dquote = bootstrap_result.ift_sensitivity(dF_dq)?;
                df_dquote_cache.push(df_dquote);
            }

            let mut trade_results = Vec::with_capacity(trades.len());
            let mut total_pv = 0.0;
            let mut total_market_sensitivities = vec![0.0; n_quotes];

            for trade in trades {
                let (pv, df_sensitivities) = trade.compute_with_df_sensitivities(
                    &bootstrap_result.discount_factors,
                    &bootstrap_result.pillars,
                );

                total_pv += pv;

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
        #[inline]
        pub fn can_compute_ift_risk(&self, bootstrap_result: &GlobalBootstrapResult<f64>) -> bool {
            bootstrap_result.can_compute_ift()
        }
    }

    #[cfg(test)]
    mod tests {
        use pricer_core::math::linalg::DMatrix;
        use pricer_models::market::curves::{BootstrapInterpolation, BootstrappedCurve};

        use super::*;

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

        #[test]
        fn test_calculate_ift_risk_basic() {
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

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
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, false);
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
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

            let dF_dquote = vec![vec![1.0, 0.0]];

            let risk_result = calc.calculate_ift_risk(&result, &trade, &dF_dquote);

            assert!(matches!(
                risk_result,
                Err(IftRiskError::IftError(IftError::DimensionMismatch { .. }))
            ));
        }

        #[test]
        fn test_calculate_batch_ift_risk_basic() {
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

            let sum_pv: f64 = batch.trade_results.iter().map(|r| r.pv).sum();
            assert!((batch.total_pv - sum_pv).abs() < 1e-10);
        }

        #[test]
        fn test_calculate_batch_ift_risk_empty_batch() {
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trades: Vec<MockIrsTrade> = vec![];
            let dF_dquote = vec![vec![1.0, 0.0, 0.0]];

            let batch_result = calc.calculate_batch_ift_risk(&result, &trades, &dF_dquote);

            assert!(matches!(batch_result, Err(IftRiskError::EmptyBatch)));
        }

        #[test]
        fn test_calculate_batch_ift_risk_aggregation() {
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);

            let trades = vec![
                MockIrsTrade::new(1_000_000.0, 0.02, 3),
                MockIrsTrade::new(1_000_000.0, 0.02, 3),
            ];

            let dF_dquote = vec![vec![1.0, 0.0, 0.0]];

            let batch_result = calc
                .calculate_batch_ift_risk(&result, &trades, &dF_dquote)
                .unwrap();

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
            let calc = MarketRiskCalculator::default();
            let result = create_mock_bootstrap_result(3, true);
            let trade = MockIrsTrade::new(1_000_000.0, 0.02, 3);

            let dF_dquote = vec![vec![0.01, 0.0, 0.0]];

            let risk = calc
                .calculate_ift_risk(&result, &trade, &dF_dquote)
                .unwrap();

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

pub use global_bootstrap_integration::*;

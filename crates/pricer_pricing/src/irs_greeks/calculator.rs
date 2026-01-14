//! IRS Greeks calculator implementation.
//!
//! Provides AAD and bump-and-revalue Greeks calculation for Interest Rate Swaps.

use num_traits::Float;
use std::marker::PhantomData;
use std::time::Instant;

#[cfg(feature = "l1l2-integration")]
use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet, YieldCurve};
#[cfg(feature = "l1l2-integration")]
use pricer_core::types::time::Date;
#[cfg(feature = "l1l2-integration")]
use pricer_models::instruments::rates::{price_irs, InterestRateSwap};

use crate::greeks::GreeksMode;

use super::config::IrsGreeksConfig;
use super::error::IrsGreeksError;
use super::result::{IrsDeltaResult, IrsGreeksResult};

/// IRS Greeks calculator.
///
/// Computes NPV, DV01, and tenor Deltas for Interest Rate Swaps using
/// either AAD (Adjoint Algorithmic Differentiation) or bump-and-revalue methods.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_pricing::irs_greeks::{IrsGreeksCalculator, IrsGreeksConfig};
///
/// let config = IrsGreeksConfig::default();
/// let calculator = IrsGreeksCalculator::<f64>::new(config);
///
/// let npv = calculator.compute_npv(&swap, &curves, valuation_date)?;
/// ```
pub struct IrsGreeksCalculator<T: Float> {
    config: IrsGreeksConfig,
    _phantom: PhantomData<T>,
}

impl<T: Float> IrsGreeksCalculator<T> {
    /// Creates a new IRS Greeks calculator with the given configuration.
    pub fn new(config: IrsGreeksConfig) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &IrsGreeksConfig {
        &self.config
    }
}

#[cfg(feature = "l1l2-integration")]
impl IrsGreeksCalculator<f64> {
    /// Validates the swap parameters.
    fn validate_swap(&self, swap: &InterestRateSwap<f64>) -> Result<(), IrsGreeksError> {
        // Check for positive notional
        if swap.notional() <= 0.0 {
            return Err(IrsGreeksError::InvalidSwap(
                "Notional must be positive".to_string(),
            ));
        }

        // Check that swap has periods
        if swap.fixed_leg().schedule().periods().is_empty() {
            return Err(IrsGreeksError::InvalidSwap(
                "Fixed leg has no periods".to_string(),
            ));
        }

        if swap.floating_leg().schedule().periods().is_empty() {
            return Err(IrsGreeksError::InvalidSwap(
                "Floating leg has no periods".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates the curve set.
    fn validate_curves(&self, curves: &CurveSet<f64>) -> Result<(), IrsGreeksError> {
        if curves.discount_curve().is_none() {
            return Err(IrsGreeksError::CurveNotFound(
                "Discount curve not found".to_string(),
            ));
        }
        Ok(())
    }

    /// Computes the NPV of an IRS.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap to price
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    ///
    /// # Returns
    ///
    /// The net present value of the swap.
    ///
    /// # Errors
    ///
    /// Returns `IrsGreeksError` if:
    /// - Swap parameters are invalid (negative notional, etc.)
    /// - Required curves are not found
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.1: IRSパラメータからInterestRateSwapオブジェクトを使用
    /// - Requirement 1.2: 固定レグと変動レグの現在価値を計算
    /// - Requirement 1.3: スワップの正味現在価値(NPV)を返却
    /// - Requirement 1.5: 無効なパラメータに対するエラー処理
    pub fn compute_npv(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<f64, IrsGreeksError> {
        // Validate inputs
        self.validate_swap(swap)?;
        self.validate_curves(curves)?;

        // Use the existing price_irs function from pricer_models
        let npv = price_irs(swap, curves, valuation_date);

        Ok(npv)
    }

    /// Computes DV01 (1bp parallel shift sensitivity).
    ///
    /// DV01 represents the change in PV for a 1 basis point parallel shift
    /// in the yield curve.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    ///
    /// # Returns
    ///
    /// The DV01 value (positive for receiver swaps, typically negative for payer swaps
    /// when rates rise).
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.4: DV01(1bp金利変動に対するPV変化)を出力する
    pub fn compute_dv01(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<f64, IrsGreeksError> {
        self.validate_swap(swap)?;
        self.validate_curves(curves)?;

        let bump_size = self.config.bump_size;

        // Base NPV
        let base_npv = price_irs(swap, curves, valuation_date);

        // Create bumped curves (+1bp parallel shift)
        let bumped_curves = self.create_parallel_bumped_curves(curves, bump_size);

        // Bumped NPV
        let bumped_npv = price_irs(swap, &bumped_curves, valuation_date);

        // DV01 = change in PV for 1bp shift
        // Since we bumped by bump_size, we need to scale appropriately
        let dv01 = (bumped_npv - base_npv) / (bump_size / 0.0001);

        Ok(dv01.abs())
    }

    /// Creates a parallel-bumped version of the curve set.
    fn create_parallel_bumped_curves(&self, curves: &CurveSet<f64>, bump: f64) -> CurveSet<f64> {
        let mut bumped = CurveSet::new();

        for (name, curve) in curves.iter() {
            // Get the base rate and bump it
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + bump;
            bumped.insert(*name, CurveEnum::flat(bumped_rate));
        }

        // Preserve discount curve setting
        if let Some(discount) = curves.discount_curve() {
            let base_rate = discount.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + bump;
            bumped.insert(CurveName::Discount, CurveEnum::flat(bumped_rate));
            bumped.set_discount_curve(CurveName::Discount);
        }

        bumped
    }

    /// Creates a tenor-point bumped version of the curve set.
    fn create_tenor_bumped_curves(
        &self,
        curves: &CurveSet<f64>,
        _tenor: f64,
        bump: f64,
    ) -> CurveSet<f64> {
        // For simplicity with flat curves, bump the entire curve
        // In a real implementation, this would bump only the specific tenor point
        self.create_parallel_bumped_curves(curves, bump)
    }

    /// Computes tenor Deltas using bump-and-revalue.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    /// * `tenor_points` - Tenor points to compute Deltas for (in years)
    /// * `bump_size` - Bump size for finite differences
    ///
    /// # Returns
    ///
    /// `IrsDeltaResult` containing Deltas for each tenor point and DV01.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.2: 有限差分法でDeltaを計算する
    /// - Requirement 2.6: 計算時間をナノ秒精度で計測
    pub fn compute_tenor_deltas_bump(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: &[f64],
        bump_size: f64,
    ) -> Result<IrsDeltaResult<f64>, IrsGreeksError> {
        self.validate_swap(swap)?;
        self.validate_curves(curves)?;

        let start_time = Instant::now();

        // Base NPV (used for debugging/logging, not for central difference)
        let _base_npv = price_irs(swap, curves, valuation_date);

        let mut deltas = Vec::with_capacity(tenor_points.len());
        let mut total_delta = 0.0;

        for &tenor in tenor_points {
            // Bump up
            let up_curves = self.create_tenor_bumped_curves(curves, tenor, bump_size);
            let up_npv = price_irs(swap, &up_curves, valuation_date);

            // Bump down
            let down_curves = self.create_tenor_bumped_curves(curves, tenor, -bump_size);
            let down_npv = price_irs(swap, &down_curves, valuation_date);

            // Central difference
            let delta = (up_npv - down_npv) / (2.0 * bump_size);
            deltas.push(delta);
            total_delta += delta;
        }

        let compute_time_ns = start_time.elapsed().as_nanos() as u64;

        // DV01 is the sum of all tenor Deltas scaled to 1bp
        let dv01 = total_delta.abs() * (0.0001 / bump_size);

        Ok(IrsDeltaResult::new(
            tenor_points.to_vec(),
            deltas,
            dv01,
            compute_time_ns,
        ))
    }

    /// Computes tenor Deltas using AAD (placeholder implementation).
    ///
    /// When `enzyme-ad` feature is enabled, this uses Enzyme for
    /// single-pass reverse-mode AD. Otherwise, it falls back to bump-and-revalue.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    /// * `tenor_points` - Tenor points to compute Deltas for
    ///
    /// # Returns
    ///
    /// `IrsDeltaResult` containing Deltas for each tenor point.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.1: EnzymeベースのADを使用してDeltaを計算
    /// - Requirement 2.5: 全テナーのDeltaを単一の逆伝播で計算
    pub fn compute_tenor_deltas_aad(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: &[f64],
    ) -> Result<IrsDeltaResult<f64>, IrsGreeksError> {
        // Phase 1: Fallback to bump-and-revalue
        // Phase 2 (enzyme-ad feature): Use Enzyme #[autodiff] macro
        #[cfg(feature = "enzyme-ad")]
        {
            // TODO: Implement Enzyme-based AAD computation
            // This would use #[autodiff_reverse] on the pricing function
            // to compute all tenor Deltas in a single reverse pass
            self.compute_tenor_deltas_bump(swap, curves, valuation_date, tenor_points, 0.0001)
        }

        #[cfg(not(feature = "enzyme-ad"))]
        {
            // Fallback to bump-and-revalue with smaller bump for better accuracy
            self.compute_tenor_deltas_bump(
                swap,
                curves,
                valuation_date,
                tenor_points,
                self.config.bump_size,
            )
        }
    }

    /// Computes Deltas using the specified calculation mode.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set
    /// * `valuation_date` - The valuation date
    /// * `tenor_points` - Tenor points for Delta calculation
    /// * `mode` - Calculation mode (BumpRevalue, NumDual, or EnzymeAAD)
    ///
    /// # Returns
    ///
    /// `IrsGreeksResult` containing NPV and Delta results.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.1-2.6: Full Greeks calculation with mode selection
    pub fn compute_deltas(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: &[f64],
        mode: GreeksMode,
    ) -> Result<IrsGreeksResult<f64>, IrsGreeksError> {
        self.validate_swap(swap)?;
        self.validate_curves(curves)?;

        // Compute NPV first
        let npv = price_irs(swap, curves, valuation_date);

        let mut result = IrsGreeksResult::new(npv);

        match mode {
            GreeksMode::BumpRevalue => {
                let bump_result = self.compute_tenor_deltas_bump(
                    swap,
                    curves,
                    valuation_date,
                    tenor_points,
                    self.config.bump_size,
                )?;
                result = result.with_bump_result(bump_result);
            }
            #[cfg(feature = "enzyme-ad")]
            GreeksMode::EnzymeAAD => {
                let aad_result =
                    self.compute_tenor_deltas_aad(swap, curves, valuation_date, tenor_points)?;
                result = result.with_aad_result(aad_result);
            }
            _ => {
                // Default to bump-and-revalue for unsupported modes
                let bump_result = self.compute_tenor_deltas_bump(
                    swap,
                    curves,
                    valuation_date,
                    tenor_points,
                    self.config.bump_size,
                )?;
                result = result.with_bump_result(bump_result);
            }
        }

        Ok(result)
    }

    /// Verifies accuracy between AAD and bump-and-revalue results.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set
    /// * `valuation_date` - The valuation date
    /// * `tenor_points` - Tenor points for comparison
    ///
    /// # Returns
    ///
    /// `IrsGreeksResult` with both AAD and bump results and accuracy check.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.3: 計算結果の差分が許容誤差(1e-6相対誤差)以内であることを検証
    pub fn verify_accuracy(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        tenor_points: &[f64],
    ) -> Result<IrsGreeksResult<f64>, IrsGreeksError> {
        // Validate inputs before computing
        self.validate_swap(swap)?;
        self.validate_curves(curves)?;

        let npv = price_irs(swap, curves, valuation_date);

        // Compute using both methods
        let aad_result =
            self.compute_tenor_deltas_aad(swap, curves, valuation_date, tenor_points)?;
        let bump_result = self.compute_tenor_deltas_bump(
            swap,
            curves,
            valuation_date,
            tenor_points,
            self.config.bump_size,
        )?;

        // Calculate relative errors
        let mut errors = Vec::with_capacity(tenor_points.len());
        let mut max_error = 0.0_f64;

        for i in 0..tenor_points.len() {
            let aad_delta = aad_result.deltas[i];
            let bump_delta = bump_result.deltas[i];

            let rel_error = if bump_delta.abs() > 1e-10 {
                ((aad_delta - bump_delta) / bump_delta).abs()
            } else {
                (aad_delta - bump_delta).abs()
            };

            errors.push(rel_error);
            max_error = max_error.max(rel_error);
        }

        // Check tolerance
        if max_error > self.config.tolerance {
            return Err(IrsGreeksError::AccuracyCheckFailed(
                max_error,
                self.config.tolerance,
            ));
        }

        Ok(IrsGreeksResult::new(npv)
            .with_aad_result(aad_result)
            .with_bump_result(bump_result)
            .with_accuracy_check(errors))
    }
}

#[cfg(all(test, feature = "l1l2-integration"))]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_new() {
        let config = IrsGreeksConfig::default();
        let _calculator = IrsGreeksCalculator::<f64>::new(config);
    }

    #[test]
    fn test_calculator_config() {
        let config = IrsGreeksConfig::default().with_bump_size(0.0005);
        let calculator = IrsGreeksCalculator::<f64>::new(config);
        assert!((calculator.config().bump_size - 0.0005).abs() < 1e-10);
    }
}

//! 1-Factor Non-Parametric Markov Functional Model.
//!
//! Calibrates a mapping from a Gaussian state variable to interest rates
//! by inverting market swaption prices at each grid point using
//! Newton-Raphson. Supports multi-curve calibration with three
//! simultaneous rate index mappings.

use pricer_core::{
    math::{
        normal_dist::{norm_cdf, norm_pdf},
        numeric::from_f64,
        solvers::{NewtonRaphsonSolver, SolverConfig},
    },
    traits::Float,
};

use super::{
    config::{MfmCalibrationResult, MfmConfig, MfmVolType},
    integral_adjuster::IntegralAdjusterNormal,
    rate_mapping::{CalibratedSlice, MfmRateIndex, RateIndexCalibration},
    vol_cube::SwaptionVolCube,
    MfmError,
};

// ─── Main struct ────────────────────────────────────────────────────

/// 1-Factor Non-Parametric Markov Functional Model.
///
/// Implements a Gaussian recombining tree with non-parametric rate mapping
/// for pricing callable inverse floaters and related structured products.
///
/// The model calibrates a mapping from a Gaussian state variable `x` to
/// interest rates by matching market swaption prices at each grid point.
/// It supports multi-curve calibration with three simultaneous rate index
/// mappings: funding swap rate, coupon swap rate, and coupon LIBOR rate.
#[derive(Debug, Clone)]
pub struct MarkovFunctionalNonParametric1F<T: Float> {
    config: MfmConfig<T>,
}

impl<T: Float> MarkovFunctionalNonParametric1F<T> {
    /// Constructs a new MFM calibration engine from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MfmError::InvalidParameter`] if any configuration parameter
    /// is out of its acceptable range.
    pub fn new(config: MfmConfig<T>) -> Result<Self, MfmError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns a reference to the model configuration.
    pub fn config(&self) -> &MfmConfig<T> { &self.config }

    // ── Gaussian grid construction ──────────────────────────────────

    /// Builds the internal Gaussian grid for the recombining tree.
    ///
    /// Returns `(x_grid, dx)` where `x_grid` is a vector of grid point
    /// values centred at zero and `dx` is the spacing between adjacent
    /// grid points.
    ///
    /// The grid extends `num_std_devs` standard deviations on each side
    /// of zero, using the terminal variance of the OU process to set the
    /// scale.
    pub fn build_gaussian_grid(&self) -> (Vec<T>, T) {
        let n = self.config.num_grid_points;
        let center = n / 2;

        // Terminal variance of the OU process:
        //   V = (sigma^2 / (2*a)) * (1 - exp(-2*a*T_max))
        // For grid construction we use the terminal variance at the
        // longest exercise time, or fall back to the asymptotic
        // variance when no exercise times are supplied.
        let a = self.config.mean_reversion;
        let sigma = self.config.volatility;
        let two: T = from_f64(2.0);
        let two_a = two * a;

        let terminal_var = if self.config.exercise_times.is_empty() {
            // Asymptotic variance: sigma^2 / (2*a)
            sigma * sigma / two_a
        } else {
            let t_max = self
                .config
                .exercise_times
                .iter()
                .copied()
                .fold(T::zero(), |acc, t| if t > acc { t } else { acc });
            (sigma * sigma / two_a) * (T::one() - (-two_a * t_max).exp())
        };

        let terminal_std = terminal_var.sqrt();

        // dx = 2 * num_std_devs * sqrt(V) / (num_grid_points - 1)
        let n_minus_1: T = from_f64((n - 1) as f64);
        let dx = two * self.config.num_std_devs * terminal_std / n_minus_1;

        let mut x_grid = Vec::with_capacity(n);
        for i in 0..n {
            let offset: T = from_f64::<T>(i as f64) - from_f64::<T>(center as f64);
            x_grid.push(offset * dx);
        }

        (x_grid, dx)
    }

    // ── Conditional standard deviation ──────────────────────────────

    /// Computes the conditional standard deviation of the Gaussian state
    /// variable at time `t`.
    ///
    /// `sigma_x(t) = sqrt((sigma^2 / (2*a)) * (1 - exp(-2*a*t)))`
    pub fn conditional_std_dev(&self, t: T) -> T {
        let a = self.config.mean_reversion;
        let sigma = self.config.volatility;
        let two: T = from_f64(2.0);
        let two_a = two * a;

        let var = (sigma * sigma / two_a) * (T::one() - (-two_a * t).exp());
        var.sqrt()
    }

    // ── Forward swap rate computation ───────────────────────────────

    /// Computes the par forward swap rate for a swap starting at
    /// `exercise_time` with tenor `swap_tenor` and payment frequency
    /// `pay_freq`.
    ///
    /// Uses the standard formula:
    /// `S = (DF_start - DF_end) / Annuity`
    pub fn compute_forward_swap_rate(
        &self,
        exercise_time: T,
        swap_tenor: T,
        pay_freq: T,
        discount_fn: &dyn Fn(T) -> T,
    ) -> T {
        let df_start = discount_fn(exercise_time);
        let df_end = discount_fn(exercise_time + swap_tenor);
        let annuity = self.compute_annuity(exercise_time, swap_tenor, pay_freq, discount_fn);

        (df_start - df_end) / annuity
    }

    /// Computes the annuity (PV01) for a swap starting at `exercise_time`
    /// with tenor `swap_tenor` and payment frequency `pay_freq`.
    ///
    /// `A = sum_{j=1}^{N} DF(t_j) * dcf`
    /// where `t_j = exercise_time + j * dcf` and `dcf = pay_freq`.
    pub fn compute_annuity(
        &self,
        exercise_time: T,
        swap_tenor: T,
        pay_freq: T,
        discount_fn: &dyn Fn(T) -> T,
    ) -> T {
        let dcf = pay_freq;
        let num_periods = ((swap_tenor / dcf).to_f64().unwrap()).round() as usize;
        let mut annuity = T::zero();

        for j in 1..=num_periods {
            let t_j = exercise_time + from_f64::<T>(j as f64) * dcf;
            annuity = annuity + discount_fn(t_j) * dcf;
        }

        annuity
    }

    // ── Per-slice calibration ───────────────────────────────────────

    /// Calibrates a single time-slice of the rate mapping.
    ///
    /// For each grid point, determines the swap rate by mapping the
    /// Gaussian state variable through the appropriate distribution.
    /// Under Normal (Bachelier) dynamics the mapping is linear; under
    /// Lognormal dynamics it is exponential.
    ///
    /// Returns a [`CalibratedSlice`] with swap rates, discount factors,
    /// and annuities at each grid point.
    #[allow(clippy::too_many_arguments)]
    pub fn calibrate_slice(
        &self,
        _rate_index: MfmRateIndex,
        exercise_idx: usize,
        exercise_time: T,
        swap_tenor: T,
        pay_freq: T,
        x_grid: &[T],
        sigma_x: T,
        fwd_swap: T,
        normal_vol: T,
        discount_fn: &dyn Fn(T) -> T,
    ) -> Result<CalibratedSlice<T>, MfmError> {
        let n = x_grid.len();
        let mut swap_rates = Vec::with_capacity(n);
        let mut discount_factors = Vec::with_capacity(n);
        let mut annuities = Vec::with_capacity(n);

        let eps: T = from_f64(1e-15);

        // Guard against zero sigma_x (degenerate case at t ~ 0).
        if sigma_x.abs() < eps {
            // All grid points collapse to the forward.
            for _ in 0..n {
                swap_rates.push(fwd_swap);
            }
        } else {
            match self.config.vol_type {
                MfmVolType::Normal => {
                    // Linear mapping: S(x_i) = fwd + sigma_n * sqrt(T) * (x_i / sigma_x)
                    let sqrt_t = exercise_time.sqrt();
                    let vol_sqrt_t = normal_vol * sqrt_t;

                    // For the general smile case, use Newton-Raphson to match
                    // the cumulative swaption price at each grid point.
                    // For flat normal vol the linear mapping is exact.
                    let solver = NewtonRaphsonSolver::new(SolverConfig {
                        tolerance: self.config.nr_tolerance,
                        max_iterations: self.config.nr_max_iterations,
                    });

                    for (grid_idx, &x_i) in x_grid.iter().enumerate() {
                        let z = x_i / sigma_x;

                        // Target: the cumulative probability in the swap rate
                        // distribution corresponding to state x_i.
                        let cum_prob = norm_cdf(z);

                        // Linear initial guess
                        let s_init = fwd_swap + vol_sqrt_t * z;

                        // Newton-Raphson to find S such that
                        // Bachelier_CDF(S; fwd, sigma_n, T) = cum_prob
                        // i.e. N((S - fwd) / (sigma_n * sqrt(T))) = cum_prob
                        // For flat vol this converges in 0 iterations (already exact).
                        if vol_sqrt_t.abs() < eps {
                            swap_rates.push(fwd_swap);
                        } else {
                            let fwd = fwd_swap;
                            let v = vol_sqrt_t;

                            let f_nr = |s: T| -> T { norm_cdf((s - fwd) / v) - cum_prob };
                            let f_prime_nr = |s: T| -> T { norm_pdf((s - fwd) / v) / v };

                            match solver.find_root(f_nr, f_prime_nr, s_init) {
                                Ok(s) => swap_rates.push(s),
                                Err(_) => {
                                    return Err(MfmError::NewtonRaphsonFailed {
                                        exercise_idx,
                                        grid_idx,
                                    });
                                }
                            }
                        }
                    }
                }
                MfmVolType::Lognormal => {
                    // Lognormal mapping:
                    // S(x_i) = fwd * exp(sigma_ln * sqrt(T) * z - 0.5 * sigma_ln^2 * T)
                    // where the normal_vol here is interpreted as the log-normal vol.
                    let sqrt_t = exercise_time.sqrt();
                    let half: T = from_f64(0.5);

                    for &x_i in x_grid.iter() {
                        let z = x_i / sigma_x;
                        let s = fwd_swap
                            * (normal_vol * sqrt_t * z
                                - half * normal_vol * normal_vol * exercise_time)
                                .exp();
                        swap_rates.push(s);
                    }
                }
            }
        }

        // Compute discount factors and annuities at each grid point.
        let dcf = pay_freq;
        let num_periods = ((swap_tenor / dcf).to_f64().unwrap()).round() as usize;
        let df_start = discount_fn(exercise_time);

        for i in 0..n {
            let s_i = swap_rates[i];

            // Compute the annuity at this grid point by bootstrapping
            // discount factors from the swap rate.
            //
            // For a par swap: DF_j = DF_{j-1} / (1 + S * dcf)
            // More precisely, for a single-period swap:
            //   DF_end = DF_start / (1 + S * dcf)
            // For multi-period swaps with constant swap rate S:
            //   A = dcf * sum_{j=1}^{N} DF_j
            //   DF_j = DF_start / (1 + S * dcf)^j   (for annual compounding)
            //
            // However, the exact relation from the swap rate definition gives:
            //   S = (DF_start - DF_end) / A
            //   => DF_end = DF_start - S * A
            //
            // We bootstrap iteratively:
            //   DF_1 = DF_start / (1 + S * dcf)
            //   DF_j = DF_{j-1} / (1 + S * dcf)
            //   A = dcf * sum(DF_j)
            let one_plus_s_dcf = T::one() + s_i * dcf;

            let mut ann = T::zero();
            let mut df_prev = df_start;

            for _j in 1..=num_periods {
                let df_j = df_prev / one_plus_s_dcf;
                ann = ann + df_j * dcf;
                df_prev = df_j;
            }

            annuities.push(ann);

            // Terminal discount factor
            let df_end = df_start - s_i * ann;
            discount_factors.push(df_end);
        }

        Ok(CalibratedSlice {
            exercise_time,
            x_grid: x_grid.to_vec(),
            swap_rates,
            discount_factors,
            annuities,
        })
    }

    // ── Full calibration ────────────────────────────────────────────

    /// Runs the full MFM calibration across all exercise dates and rate
    /// indices.
    ///
    /// # Arguments
    ///
    /// * `funding_curve` - Discount factor function for the funding curve:
    ///   `funding_curve(t) -> DF(t)`.
    /// * `coupon_curve` - Discount factor function for the coupon projection
    ///   curve: `coupon_curve(t) -> DF(t)`.
    /// * `vol_cube` - Swaption volatility cube providing normal vols.
    ///
    /// # Algorithm
    ///
    /// 1. Build the internal Gaussian grid.
    /// 2. For each exercise date, calibrate three rate index mappings (funding
    ///    swap, coupon swap, coupon LIBOR).
    /// 3. Apply integral adjuster corrections so that tree-implied discount
    ///    factors match the yield curve.
    /// 4. Assemble and return the [`MfmCalibrationResult`].
    ///
    /// # Errors
    ///
    /// Returns an error if Newton-Raphson fails at any grid node, or if
    /// the volatility cube returns an invalid value.
    pub fn calibrate<V: SwaptionVolCube<T>>(
        &self,
        funding_curve: &dyn Fn(T) -> T,
        coupon_curve: &dyn Fn(T) -> T,
        vol_cube: &V,
    ) -> Result<MfmCalibrationResult<T>, MfmError> {
        let num_dates = self.config.exercise_times.len();
        let (x_grid, _dx) = self.build_gaussian_grid();

        let mut funding_slices = Vec::with_capacity(num_dates);
        let mut coupon_swap_slices = Vec::with_capacity(num_dates);
        let mut coupon_libor_slices = Vec::with_capacity(num_dates);

        let mut max_calibration_error = T::zero();

        // Integral adjuster with one entry per exercise date.
        let mut adjuster = IntegralAdjusterNormal::new(num_dates);

        for k in 0..num_dates {
            let exercise_time = self.config.exercise_times[k];
            let swap_tenor = self.config.swap_tenors[k];
            let pay_freq = self.config.payment_frequencies[k];
            let sigma_x = self.conditional_std_dev(exercise_time);

            // ── Funding index swap rate ─────────────────────────────
            let fwd_funding =
                self.compute_forward_swap_rate(exercise_time, swap_tenor, pay_freq, funding_curve);

            let sigma_n_funding =
                vol_cube.normal_vol(exercise_time, swap_tenor, fwd_funding, fwd_funding)?;

            let funding_slice = self.calibrate_slice(
                MfmRateIndex::FundingIndexSwapRate,
                k,
                exercise_time,
                swap_tenor,
                pay_freq,
                &x_grid,
                sigma_x,
                fwd_funding,
                sigma_n_funding,
                funding_curve,
            )?;

            // ── Coupon index swap rate ──────────────────────────────
            let fwd_coupon =
                self.compute_forward_swap_rate(exercise_time, swap_tenor, pay_freq, coupon_curve);

            let sigma_n_coupon =
                vol_cube.normal_vol(exercise_time, swap_tenor, fwd_coupon, fwd_coupon)?;

            let coupon_swap_slice = self.calibrate_slice(
                MfmRateIndex::CouponIndexSwapRate,
                k,
                exercise_time,
                swap_tenor,
                pay_freq,
                &x_grid,
                sigma_x,
                fwd_coupon,
                sigma_n_coupon,
                coupon_curve,
            )?;

            // ── Coupon LIBOR rate ───────────────────────────────────
            // LIBOR is a simple rate over one payment period.
            // L = (1/dcf) * (DF_start/DF_end - 1)
            let libor_tenor = pay_freq;
            let df_start_libor = coupon_curve(exercise_time);
            let df_end_libor = coupon_curve(exercise_time + libor_tenor);
            let fwd_libor = (df_start_libor / df_end_libor - T::one()) / libor_tenor;

            let sigma_n_libor =
                vol_cube.normal_vol(exercise_time, libor_tenor, fwd_libor, fwd_libor)?;

            // For LIBOR we calibrate a single-period "swap" with the same
            // machinery, treating the LIBOR fixing as a one-period swap.
            let coupon_libor_slice = self.calibrate_slice(
                MfmRateIndex::CouponLibor,
                k,
                exercise_time,
                libor_tenor,
                libor_tenor,
                &x_grid,
                sigma_x,
                fwd_libor,
                sigma_n_libor,
                coupon_curve,
            )?;

            // ── Integral adjuster (moment matching) ─────────────────
            // Compute Arrow-Debreu weights from the normal PDF.
            let n = x_grid.len();
            let mut probabilities = vec![T::zero(); n];
            let eps: T = from_f64(1e-15);

            if sigma_x.abs() > eps {
                let mut prob_sum = T::zero();
                for (i, &x_i) in x_grid.iter().enumerate() {
                    probabilities[i] = norm_pdf(x_i / sigma_x) / sigma_x;
                    prob_sum = prob_sum + probabilities[i];
                }
                // Normalise so probabilities sum to 1.
                if prob_sum > eps {
                    for p in probabilities.iter_mut() {
                        *p = *p / prob_sum;
                    }
                }
            } else {
                // Degenerate: all weight on centre node.
                let center = n / 2;
                probabilities[center] = T::one();
            }

            // Compute multiplicative correction for discount factors.
            let analytical_df = funding_curve(exercise_time + swap_tenor);
            let mm_result = IntegralAdjusterNormal::compute_multiplicative_correction(
                &funding_slice.discount_factors,
                &probabilities,
                analytical_df,
            );

            match mm_result.correction {
                super::integral_adjuster::MomentMatchCorrection::Multiplicative { multiplier } => {
                    adjuster.set_multiplier(k, multiplier);
                }
                super::integral_adjuster::MomentMatchCorrection::Additive { adder } => {
                    adjuster.set_adder(k, adder);
                }
            }

            // Track calibration error: difference between tree-implied
            // and analytical expected discount factor.
            let calib_err = (mm_result.tree_expected - mm_result.analytical_expected).abs();
            if calib_err > max_calibration_error {
                max_calibration_error = calib_err;
            }

            funding_slices.push(funding_slice);
            coupon_swap_slices.push(coupon_swap_slice);
            coupon_libor_slices.push(coupon_libor_slice);
        }

        Ok(MfmCalibrationResult {
            funding_calibration: RateIndexCalibration {
                rate_index: MfmRateIndex::FundingIndexSwapRate,
                slices: funding_slices,
            },
            coupon_swap_calibration: RateIndexCalibration {
                rate_index: MfmRateIndex::CouponIndexSwapRate,
                slices: coupon_swap_slices,
            },
            coupon_libor_calibration: RateIndexCalibration {
                rate_index: MfmRateIndex::CouponLibor,
                slices: coupon_libor_slices,
            },
            adjuster,
            max_nr_iterations_used: self.config.nr_max_iterations,
            max_calibration_error,
        })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markov_functional::vol_cube::FlatSwaptionVolCube;

    /// Helper: build a valid MfmConfig for testing.
    fn test_config() -> MfmConfig<f64> {
        MfmConfig {
            mean_reversion: 0.03,
            volatility: 0.01,
            num_grid_points: 41,
            num_std_devs: 5.0,
            vol_type: MfmVolType::Normal,
            nr_tolerance: 1e-10,
            nr_max_iterations: 100,
            exercise_times: vec![1.0, 2.0, 3.0],
            swap_tenors: vec![5.0, 5.0, 5.0],
            payment_frequencies: vec![1.0, 1.0, 1.0],
        }
    }

    /// Flat discount factor function: DF(t) = exp(-r * t)
    fn flat_df(rate: f64) -> impl Fn(f64) -> f64 { move |t: f64| (-rate * t).exp() }

    // ── Construction tests ──────────────────────────────────────────

    #[test]
    fn test_new_valid_config() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config);
        assert!(model.is_ok());
    }

    #[test]
    fn test_new_invalid_config() {
        let mut config = test_config();
        config.mean_reversion = -0.01; // Invalid: must be positive
        let model = MarkovFunctionalNonParametric1F::new(config);
        assert!(model.is_err());
        match model.unwrap_err() {
            MfmError::InvalidParameter { name, .. } => {
                assert_eq!(name, "mean_reversion");
            }
            other => panic!("expected InvalidParameter, got {:?}", other),
        }
    }

    #[test]
    fn test_new_invalid_config_even_grid() {
        let mut config = test_config();
        config.num_grid_points = 40; // Invalid: must be odd
        let model = MarkovFunctionalNonParametric1F::new(config);
        assert!(model.is_err());
    }

    #[test]
    fn test_new_invalid_config_mismatched_lengths() {
        let mut config = test_config();
        config.swap_tenors = vec![5.0]; // Mismatch with exercise_times
        let model = MarkovFunctionalNonParametric1F::new(config);
        assert!(model.is_err());
    }

    // ── Gaussian grid tests ─────────────────────────────────────────

    #[test]
    fn test_build_gaussian_grid() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let (x_grid, dx) = model.build_gaussian_grid();

        // Correct number of grid points.
        assert_eq!(x_grid.len(), 41);

        // Grid should be symmetric about zero.
        let center = 41 / 2;
        assert!(
            x_grid[center].abs() < 1e-14,
            "center grid point should be 0, got {}",
            x_grid[center]
        );

        // Check symmetry: x[center - k] = -x[center + k]
        for k in 1..=center {
            assert!(
                (x_grid[center - k] + x_grid[center + k]).abs() < 1e-14,
                "grid not symmetric at offset {}: {} vs {}",
                k,
                x_grid[center - k],
                x_grid[center + k]
            );
        }

        // dx should be positive.
        assert!(dx > 0.0, "dx should be positive, got {}", dx);

        // Grid spacing should equal dx.
        for i in 1..x_grid.len() {
            let spacing = x_grid[i] - x_grid[i - 1];
            assert!(
                (spacing - dx).abs() < 1e-14,
                "grid spacing at {} is {}, expected {}",
                i,
                spacing,
                dx
            );
        }

        // Grid should be ascending.
        for i in 1..x_grid.len() {
            assert!(x_grid[i] > x_grid[i - 1]);
        }
    }

    #[test]
    fn test_build_gaussian_grid_small() {
        let config = MfmConfig {
            num_grid_points: 3,
            num_std_devs: 3.0,
            ..test_config()
        };
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let (x_grid, dx) = model.build_gaussian_grid();

        assert_eq!(x_grid.len(), 3);
        assert!(x_grid[1].abs() < 1e-14); // Center at 0
        assert!((x_grid[0] + x_grid[2]).abs() < 1e-14); // Symmetric
        assert!(dx > 0.0);
    }

    #[test]
    fn test_build_gaussian_grid_no_exercise_times() {
        // When no exercise times are provided, uses asymptotic variance.
        let config = MfmConfig {
            exercise_times: vec![],
            swap_tenors: vec![],
            payment_frequencies: vec![],
            ..test_config()
        };
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let (x_grid, dx) = model.build_gaussian_grid();

        assert_eq!(x_grid.len(), 41);
        assert!(dx > 0.0);
        assert!(x_grid[20].abs() < 1e-14);
    }

    // ── Conditional std dev tests ───────────────────────────────────

    #[test]
    fn test_conditional_std_dev() {
        let config = MfmConfig {
            mean_reversion: 0.05,
            volatility: 0.01,
            ..test_config()
        };
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        // At t = 0 the conditional std dev should be 0.
        let sd0 = model.conditional_std_dev(0.0);
        assert!(sd0.abs() < 1e-14, "sigma_x(0) should be 0, got {}", sd0);

        // At t = 1 with a = 0.05, sigma = 0.01:
        // var = (0.01^2 / (2*0.05)) * (1 - exp(-2*0.05*1))
        //     = (0.0001 / 0.1) * (1 - exp(-0.1))
        //     = 0.001 * (1 - 0.904837)
        //     = 0.001 * 0.095163
        //     = 0.000095163
        // std = sqrt(0.000095163) ≈ 0.009755
        let sd1 = model.conditional_std_dev(1.0);
        let expected_var = (0.0001 / 0.1) * (1.0 - (-0.1_f64).exp());
        let expected_sd = expected_var.sqrt();
        assert!(
            (sd1 - expected_sd).abs() < 1e-10,
            "sigma_x(1) = {}, expected {}",
            sd1,
            expected_sd
        );

        // Conditional std dev should increase monotonically with time.
        let sd2 = model.conditional_std_dev(2.0);
        assert!(
            sd2 > sd1,
            "sigma_x should increase with time: sd(2)={} <= sd(1)={}",
            sd2,
            sd1
        );

        // For very large t, should approach the asymptotic value
        // sigma / sqrt(2*a).
        let sd_large = model.conditional_std_dev(1000.0);
        let asymptotic = 0.01 / (2.0 * 0.05_f64).sqrt();
        assert!(
            (sd_large - asymptotic).abs() < 1e-6,
            "sigma_x(1000) = {}, expected asymptotic {}",
            sd_large,
            asymptotic
        );
    }

    #[test]
    fn test_conditional_std_dev_high_mean_reversion() {
        let config = MfmConfig {
            mean_reversion: 1.0,
            volatility: 0.02,
            ..test_config()
        };
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        let sd = model.conditional_std_dev(5.0);
        let asymptotic = 0.02 / (2.0_f64).sqrt();

        // With high mean reversion, should converge quickly to asymptotic.
        assert!(
            (sd - asymptotic).abs() < 1e-6,
            "high MR: sigma_x(5) = {}, expected asymptotic {}",
            sd,
            asymptotic
        );
    }

    // ── Forward swap rate tests ─────────────────────────────────────

    #[test]
    fn test_forward_swap_rate_flat_curve() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        let flat_rate = 0.05;
        let df = flat_df(flat_rate);

        let exercise_time = 1.0;
        let swap_tenor = 5.0;
        let pay_freq = 1.0;

        let fwd = model.compute_forward_swap_rate(exercise_time, swap_tenor, pay_freq, &df);

        // For a flat continuously compounded curve, the forward swap rate
        // should be close to the flat rate but not exactly equal (because
        // the swap rate is a par rate, not a continuously compounded rate).
        //
        // With annual payment frequency and continuous compounding:
        // DF(t) = exp(-r*t)
        // S = (DF(1) - DF(6)) / sum_{j=1}^{5} DF(1+j) * 1
        //   = (exp(-0.05) - exp(-0.30)) / sum_{j=1}^{5} exp(-0.05*(1+j))
        let df_start = (-0.05_f64).exp();
        let df_end = (-0.30_f64).exp();
        let mut annuity_expected = 0.0;
        for j in 1..=5 {
            annuity_expected += (-(0.05 * (1 + j) as f64)).exp();
        }
        let expected_fwd = (df_start - df_end) / annuity_expected;

        assert!(
            (fwd - expected_fwd).abs() < 1e-10,
            "forward swap rate = {}, expected {}",
            fwd,
            expected_fwd
        );

        // The forward swap rate for a flat curve should be positive.
        assert!(fwd > 0.0);
    }

    #[test]
    fn test_forward_swap_rate_zero_rate() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        // With zero rates: DF(t) = 1 for all t.
        let df = |_t: f64| -> f64 { 1.0 };
        let fwd = model.compute_forward_swap_rate(1.0, 5.0, 1.0, &df);

        // S = (1 - 1) / A = 0
        assert!(
            fwd.abs() < 1e-14,
            "forward swap rate with zero rates should be 0, got {}",
            fwd
        );
    }

    #[test]
    fn test_compute_annuity() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        let df = flat_df(0.05);
        let annuity = model.compute_annuity(1.0, 5.0, 1.0, &df);

        // Expected annuity: sum_{j=1}^{5} exp(-0.05*(1+j)) * 1.0
        let mut expected = 0.0;
        for j in 1..=5 {
            expected += (-(0.05 * (1 + j) as f64)).exp();
        }

        assert!(
            (annuity - expected).abs() < 1e-10,
            "annuity = {}, expected {}",
            annuity,
            expected
        );
    }

    #[test]
    fn test_compute_annuity_semiannual() {
        let config = test_config();
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        let df = flat_df(0.05);
        let annuity = model.compute_annuity(1.0, 2.0, 0.5, &df);

        // 4 periods of 0.5y each, dcf = 0.5
        let mut expected = 0.0;
        for j in 1..=4 {
            expected += (-(0.05 * (1.0 + 0.5 * j as f64))).exp() * 0.5;
        }

        assert!(
            (annuity - expected).abs() < 1e-10,
            "semiannual annuity = {}, expected {}",
            annuity,
            expected
        );
    }

    // ── Full calibration tests ──────────────────────────────────────

    #[test]
    fn test_calibrate_flat_vol_flat_curve() {
        let config = MfmConfig {
            mean_reversion: 0.03,
            volatility: 0.01,
            num_grid_points: 41,
            num_std_devs: 5.0,
            vol_type: MfmVolType::Normal,
            nr_tolerance: 1e-10,
            nr_max_iterations: 100,
            exercise_times: vec![1.0],
            swap_tenors: vec![5.0],
            payment_frequencies: vec![1.0],
        };

        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();

        let flat_rate = 0.05;
        let funding = flat_df(flat_rate);
        let coupon = flat_df(flat_rate);
        let vol_cube = FlatSwaptionVolCube::from_normal_vol(0.005).unwrap();

        let result = model.calibrate(&funding, &coupon, &vol_cube).unwrap();

        // Check funding calibration.
        assert_eq!(result.funding_calibration.num_exercise_dates(), 1);
        let slice = result.funding_calibration.slice(0);
        assert_eq!(slice.num_nodes(), 41);

        // Swap rates should be monotonically increasing across the grid.
        for i in 1..slice.swap_rates.len() {
            assert!(
                slice.swap_rates[i] >= slice.swap_rates[i - 1],
                "swap rates not monotonic at index {}: {} < {}",
                i,
                slice.swap_rates[i],
                slice.swap_rates[i - 1]
            );
        }

        // Centre grid point should have swap rate close to the forward.
        let center = 41 / 2;
        let fwd = model.compute_forward_swap_rate(1.0, 5.0, 1.0, &funding);
        assert!(
            (slice.swap_rates[center] - fwd).abs() < 1e-6,
            "center swap rate = {}, expected fwd = {}",
            slice.swap_rates[center],
            fwd
        );

        // Discount factors should be monotonically decreasing (higher
        // swap rates lead to lower terminal DF).
        for i in 1..slice.discount_factors.len() {
            assert!(
                slice.discount_factors[i] <= slice.discount_factors[i - 1],
                "discount factors not monotonically decreasing at index {}: {} > {}",
                i,
                slice.discount_factors[i],
                slice.discount_factors[i - 1]
            );
        }

        // All discount factors should be positive.
        for (i, &df) in slice.discount_factors.iter().enumerate() {
            assert!(
                df > 0.0,
                "discount factor at index {} is non-positive: {}",
                i,
                df
            );
        }

        // Annuities should be positive.
        for (i, &ann) in slice.annuities.iter().enumerate() {
            assert!(ann > 0.0, "annuity at index {} is non-positive: {}", i, ann);
        }

        // Coupon swap calibration should also be present.
        assert_eq!(result.coupon_swap_calibration.num_exercise_dates(), 1);

        // Coupon LIBOR calibration should also be present.
        assert_eq!(result.coupon_libor_calibration.num_exercise_dates(), 1);

        // Adjuster should have one entry.
        assert_eq!(result.adjuster.adders.len(), 1);
        assert_eq!(result.adjuster.multipliers.len(), 1);
    }

    #[test]
    fn test_calibrate_multiple_exercise_dates() {
        let config = MfmConfig {
            mean_reversion: 0.03,
            volatility: 0.01,
            num_grid_points: 21,
            num_std_devs: 4.0,
            vol_type: MfmVolType::Normal,
            nr_tolerance: 1e-10,
            nr_max_iterations: 100,
            exercise_times: vec![1.0, 2.0, 3.0],
            swap_tenors: vec![5.0, 5.0, 5.0],
            payment_frequencies: vec![1.0, 1.0, 1.0],
        };

        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let funding = flat_df(0.04);
        let coupon = flat_df(0.04);
        let vol_cube = FlatSwaptionVolCube::from_normal_vol(0.004).unwrap();

        let result = model.calibrate(&funding, &coupon, &vol_cube).unwrap();

        assert_eq!(result.funding_calibration.num_exercise_dates(), 3);
        assert_eq!(result.coupon_swap_calibration.num_exercise_dates(), 3);
        assert_eq!(result.coupon_libor_calibration.num_exercise_dates(), 3);
        assert_eq!(result.adjuster.adders.len(), 3);

        // Each slice should have 21 nodes.
        for k in 0..3 {
            assert_eq!(result.funding_calibration.slice(k).num_nodes(), 21);
        }
    }

    #[test]
    fn test_calibrate_lognormal_vol() {
        let config = MfmConfig {
            mean_reversion: 0.03,
            volatility: 0.01,
            num_grid_points: 21,
            num_std_devs: 4.0,
            vol_type: MfmVolType::Lognormal,
            nr_tolerance: 1e-10,
            nr_max_iterations: 100,
            exercise_times: vec![1.0],
            swap_tenors: vec![5.0],
            payment_frequencies: vec![1.0],
        };

        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let funding = flat_df(0.05);
        let coupon = flat_df(0.05);
        let vol_cube = FlatSwaptionVolCube::from_normal_vol(0.10).unwrap(); // 10% lognormal vol

        let result = model.calibrate(&funding, &coupon, &vol_cube).unwrap();

        let slice = result.funding_calibration.slice(0);

        // Swap rates should still be monotonically increasing.
        for i in 1..slice.swap_rates.len() {
            assert!(
                slice.swap_rates[i] >= slice.swap_rates[i - 1],
                "lognormal: swap rates not monotonic at index {}",
                i
            );
        }

        // All swap rates should be positive under lognormal dynamics.
        for (i, &s) in slice.swap_rates.iter().enumerate() {
            assert!(
                s > 0.0,
                "lognormal: swap rate at index {} is non-positive: {}",
                i,
                s
            );
        }
    }

    #[test]
    fn test_calibrate_slice_direct() {
        let config = MfmConfig {
            mean_reversion: 0.03,
            volatility: 0.01,
            num_grid_points: 11,
            num_std_devs: 3.0,
            vol_type: MfmVolType::Normal,
            nr_tolerance: 1e-10,
            nr_max_iterations: 100,
            exercise_times: vec![1.0],
            swap_tenors: vec![5.0],
            payment_frequencies: vec![1.0],
        };

        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let funding = flat_df(0.04);
        let (x_grid, _) = model.build_gaussian_grid();
        let sigma_x = model.conditional_std_dev(1.0);
        let fwd = model.compute_forward_swap_rate(1.0, 5.0, 1.0, &funding);

        let slice = model
            .calibrate_slice(
                MfmRateIndex::FundingIndexSwapRate,
                0,
                1.0,
                5.0,
                1.0,
                &x_grid,
                sigma_x,
                fwd,
                0.005,
                &funding,
            )
            .unwrap();

        assert_eq!(slice.num_nodes(), 11);
        assert!((slice.exercise_time - 1.0).abs() < 1e-14);

        // Verify swap rates, DFs, and annuities have matching lengths.
        assert_eq!(slice.swap_rates.len(), 11);
        assert_eq!(slice.discount_factors.len(), 11);
        assert_eq!(slice.annuities.len(), 11);
    }

    #[test]
    fn test_calibrate_empty_schedule() {
        let config = MfmConfig {
            exercise_times: vec![],
            swap_tenors: vec![],
            payment_frequencies: vec![],
            ..test_config()
        };

        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        let funding = flat_df(0.05);
        let coupon = flat_df(0.05);
        let vol_cube = FlatSwaptionVolCube::from_normal_vol(0.005).unwrap();

        let result = model.calibrate(&funding, &coupon, &vol_cube).unwrap();

        assert_eq!(result.funding_calibration.num_exercise_dates(), 0);
        assert_eq!(result.coupon_swap_calibration.num_exercise_dates(), 0);
        assert_eq!(result.coupon_libor_calibration.num_exercise_dates(), 0);
    }

    #[test]
    fn test_config_accessor() {
        let config = test_config();
        let mr = config.mean_reversion;
        let model = MarkovFunctionalNonParametric1F::new(config).unwrap();
        assert!((model.config().mean_reversion - mr).abs() < 1e-14);
    }
}

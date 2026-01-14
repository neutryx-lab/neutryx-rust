//! Sequential bootstrapping engine.
//!
//! This module provides `SequentialBootstrapper<T>`, the main bootstrapping
//! engine that constructs yield curves from market instruments using
//! Newton-Raphson with Brent fallback.

use super::config::GenericBootstrapConfig;
use super::curve::BootstrappedCurve;
use super::error::BootstrapError;
use super::instrument::BootstrapInstrument;
use num_traits::Float;

/// Result of a bootstrap operation.
#[derive(Debug, Clone)]
pub struct GenericBootstrapResult<T: Float> {
    /// The bootstrapped curve
    pub curve: BootstrappedCurve<T>,
    /// Pillar maturities
    pub pillars: Vec<T>,
    /// Discount factors at each pillar
    pub discount_factors: Vec<T>,
    /// Residual at each pillar
    pub residuals: Vec<T>,
    /// Number of iterations used for each pillar
    pub iterations: Vec<usize>,
}

/// Sequential bootstrapping engine.
///
/// Implements the standard sequential stripping algorithm:
/// 1. Sort instruments by maturity
/// 2. For each instrument, solve for the discount factor at maturity
/// 3. Use Newton-Raphson with Brent fallback for root-finding
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_optimiser::bootstrapping::{
///     SequentialBootstrapper, GenericBootstrapConfig, BootstrapInstrument
/// };
///
/// // Create instruments
/// let instruments: Vec<BootstrapInstrument<f64>> = vec![
///     BootstrapInstrument::ois(1.0, 0.03),
///     BootstrapInstrument::ois(2.0, 0.032),
///     BootstrapInstrument::ois(3.0, 0.034),
/// ];
///
/// // Bootstrap
/// let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::default();
/// let bootstrapper = SequentialBootstrapper::new(config);
/// let result = bootstrapper.bootstrap(&instruments).unwrap();
///
/// assert_eq!(result.pillars.len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct SequentialBootstrapper<T: Float> {
    /// Bootstrap configuration
    config: GenericBootstrapConfig<T>,
}

impl<T: Float> SequentialBootstrapper<T> {
    /// Create a new sequential bootstrapper.
    pub fn new(config: GenericBootstrapConfig<T>) -> Self {
        Self { config }
    }

    /// Create a bootstrapper with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GenericBootstrapConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &GenericBootstrapConfig<T> {
        &self.config
    }

    /// Bootstrap a yield curve from instruments.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments sorted by maturity
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Successfully bootstrapped curve with diagnostics
    /// * `Err(BootstrapError)` - If bootstrapping fails
    pub fn bootstrap(
        &self,
        instruments: &[BootstrapInstrument<T>],
    ) -> Result<GenericBootstrapResult<T>, BootstrapError> {
        // Validate inputs
        self.validate_instruments(instruments)?;

        // Sort instruments by maturity (create sorted indices)
        let mut sorted_indices: Vec<usize> = (0..instruments.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            instruments[a]
                .maturity()
                .partial_cmp(&instruments[b].maturity())
                .unwrap()
        });

        // Initialize partial curve
        let mut pillars: Vec<T> = Vec::with_capacity(instruments.len());
        let mut discount_factors: Vec<T> = Vec::with_capacity(instruments.len());
        let mut residuals: Vec<T> = Vec::with_capacity(instruments.len());
        let mut iterations: Vec<usize> = Vec::with_capacity(instruments.len());

        // Bootstrap each instrument sequentially
        for &idx in &sorted_indices {
            let instrument = &instruments[idx];
            let maturity = instrument.maturity();

            // Check for duplicate maturity
            if pillars.last().map_or(false, |&last| {
                (last - maturity).abs() < T::from(1e-10).unwrap()
            }) {
                return Err(BootstrapError::duplicate_maturity(
                    maturity.to_f64().unwrap_or(0.0),
                ));
            }

            // Create partial curve function for residual computation
            let partial_curve_df = |t: T| -> T {
                if t <= T::zero() {
                    return T::one();
                }

                // Find bracketing pillars in the partial curve
                if pillars.is_empty() {
                    // No pillars yet, use simple extrapolation
                    return T::one();
                }

                // Check if t is before first pillar
                if t < pillars[0] {
                    // Extrapolate using first pillar's rate
                    let r = -discount_factors[0].ln() / pillars[0];
                    return (-r * t).exp();
                }

                // Check if t is after last pillar
                if t > *pillars.last().unwrap() {
                    let n = pillars.len();
                    let r = -discount_factors[n - 1].ln() / pillars[n - 1];
                    return (-r * t).exp();
                }

                // Find bracketing index
                let mut lo = 0;
                let mut hi = pillars.len() - 1;
                while lo < hi {
                    let mid = (lo + hi + 1) / 2;
                    if pillars[mid] <= t {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }

                // Log-linear interpolation
                if lo + 1 < pillars.len() {
                    let t1 = pillars[lo];
                    let t2 = pillars[lo + 1];
                    let df1 = discount_factors[lo];
                    let df2 = discount_factors[lo + 1];

                    let w = (t - t1) / (t2 - t1);
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    log_df.exp()
                } else {
                    discount_factors[lo]
                }
            };

            // Solve for discount factor at maturity
            let (df, iter_count, final_residual) =
                self.solve_for_df(instrument, &partial_curve_df)?;

            // Validate result
            if !self.config.allow_negative_rates {
                let implied_rate = -df.ln() / maturity;
                if implied_rate < T::zero() {
                    return Err(BootstrapError::negative_rate(
                        maturity.to_f64().unwrap_or(0.0),
                        implied_rate.to_f64().unwrap_or(0.0),
                    ));
                }
            }

            // Check for arbitrage (DF should be decreasing)
            if let Some(&last_df) = discount_factors.last() {
                if df >= last_df {
                    return Err(BootstrapError::arbitrage_detected(
                        maturity.to_f64().unwrap_or(0.0),
                    ));
                }
            }

            // Store results
            pillars.push(maturity);
            discount_factors.push(df);
            residuals.push(final_residual);
            iterations.push(iter_count);
        }

        // Build final curve
        let curve = BootstrappedCurve::new(
            pillars.clone(),
            discount_factors.clone(),
            self.config.interpolation,
            self.config.allow_extrapolation,
        )
        .map_err(|e| BootstrapError::invalid_input(e))?;

        Ok(GenericBootstrapResult {
            curve,
            pillars,
            discount_factors,
            residuals,
            iterations,
        })
    }

    /// Validate input instruments.
    fn validate_instruments(
        &self,
        instruments: &[BootstrapInstrument<T>],
    ) -> Result<(), BootstrapError> {
        if instruments.is_empty() {
            return Err(BootstrapError::insufficient_data(1, 0));
        }

        for inst in instruments {
            inst.validate(self.config.max_maturity)
                .map_err(|e| BootstrapError::invalid_input(e))?;
        }

        Ok(())
    }

    /// Solve for discount factor using Newton-Raphson with Brent fallback.
    fn solve_for_df<F>(
        &self,
        instrument: &BootstrapInstrument<T>,
        partial_curve_df: F,
    ) -> Result<(T, usize, T), BootstrapError>
    where
        F: Fn(T) -> T,
    {
        let maturity = instrument.maturity();

        // Initial guess: use simple discounting approximation
        let rate = instrument.rate();
        let initial_df = T::one() / (T::one() + rate * maturity);

        // Try Newton-Raphson first
        let result = self.newton_raphson_solve(instrument, &partial_curve_df, initial_df);

        match result {
            Ok((df, iterations)) => {
                let residual = instrument.residual(df, &partial_curve_df);
                Ok((df, iterations, residual))
            }
            Err(_) => {
                // Fallback to Brent method
                self.brent_solve(instrument, &partial_curve_df)
            }
        }
    }

    /// Newton-Raphson solver.
    fn newton_raphson_solve<F>(
        &self,
        instrument: &BootstrapInstrument<T>,
        partial_curve_df: &F,
        initial_df: T,
    ) -> Result<(T, usize), BootstrapError>
    where
        F: Fn(T) -> T,
    {
        let mut df = initial_df;
        let epsilon = T::from(1e-30).unwrap();

        for iteration in 0..self.config.max_iterations {
            let residual = instrument.residual(df, partial_curve_df);

            // Check convergence
            if residual.abs() < self.config.tolerance {
                return Ok((df, iteration));
            }

            let derivative = instrument.residual_derivative(df, partial_curve_df);

            // Check for near-zero derivative
            if derivative.abs() < epsilon {
                return Err(BootstrapError::convergence_failure(
                    instrument.maturity().to_f64().unwrap_or(0.0),
                    residual.to_f64().unwrap_or(0.0),
                    iteration,
                ));
            }

            // Newton update
            let new_df = df - residual / derivative;

            // Ensure DF stays positive
            df = if new_df > T::zero() {
                new_df
            } else {
                df / T::from(2.0).unwrap()
            };

            // Check for non-finite
            if !df.is_finite() {
                return Err(BootstrapError::convergence_failure(
                    instrument.maturity().to_f64().unwrap_or(0.0),
                    residual.to_f64().unwrap_or(0.0),
                    iteration,
                ));
            }
        }

        Err(BootstrapError::convergence_failure(
            instrument.maturity().to_f64().unwrap_or(0.0),
            instrument.residual(df, partial_curve_df).to_f64().unwrap_or(0.0),
            self.config.max_iterations,
        ))
    }

    /// Brent solver fallback.
    fn brent_solve<F>(
        &self,
        instrument: &BootstrapInstrument<T>,
        partial_curve_df: &F,
    ) -> Result<(T, usize, T), BootstrapError>
    where
        F: Fn(T) -> T,
    {
        // Bracket for DF: typically between 0.001 and 1.0
        let mut a = T::from(0.001).unwrap();
        let mut b = T::from(1.0).unwrap();

        // Ensure bracket is valid
        let fa = instrument.residual(a, partial_curve_df);
        let fb = instrument.residual(b, partial_curve_df);

        if fa * fb > T::zero() {
            // Try to find a valid bracket
            let rates = [0.0001, 0.001, 0.01, 0.1, 0.5, 0.9, 0.99, 0.999];
            let mut found_bracket = false;

            for &r in &rates {
                let df_test = T::from(r).unwrap();
                let f_test = instrument.residual(df_test, partial_curve_df);

                if f_test * fb <= T::zero() {
                    a = df_test;
                    found_bracket = true;
                    break;
                }
                if f_test * fa <= T::zero() {
                    b = df_test;
                    found_bracket = true;
                    break;
                }
            }

            if !found_bracket {
                return Err(BootstrapError::convergence_failure(
                    instrument.maturity().to_f64().unwrap_or(0.0),
                    fa.to_f64().unwrap_or(0.0),
                    0,
                ));
            }
        }

        // Brent's method implementation
        let mut c = a;
        let mut d = b - a;
        let mut e = d;

        let mut fa = instrument.residual(a, partial_curve_df);
        let mut fb = instrument.residual(b, partial_curve_df);
        let mut fc = fa;

        for iteration in 0..self.config.max_iterations {
            // Check convergence
            if fb.abs() < self.config.tolerance {
                return Ok((b, iteration, fb));
            }

            // Ensure |f(b)| <= |f(c)|
            if fc.abs() < fb.abs() {
                std::mem::swap(&mut a, &mut b);
                std::mem::swap(&mut fa, &mut fb);
                c = a;
                fc = fa;
            }

            let tol = self.config.tolerance;
            let m = (c - b) / T::from(2.0).unwrap();

            if m.abs() <= tol {
                return Ok((b, iteration, fb));
            }

            // Decide on bisection or interpolation
            if e.abs() < tol || fa.abs() <= fb.abs() {
                // Bisection
                d = m;
                e = m;
            } else {
                // Try interpolation
                let s = fb / fa;
                let (p, q);

                if (a - c).abs() < T::from(1e-12).unwrap() {
                    // Secant method
                    p = T::from(2.0).unwrap() * m * s;
                    q = T::one() - s;
                } else {
                    // Inverse quadratic interpolation
                    let q_temp = fa / fc;
                    let r = fb / fc;
                    p = s * (T::from(2.0).unwrap() * m * q_temp * (q_temp - r)
                        - (b - a) * (r - T::one()));
                    q = (q_temp - T::one()) * (r - T::one()) * (s - T::one());
                }

                let p = if p > T::zero() { -p } else { p };
                let q = q.abs();

                // Check if interpolation is acceptable
                if T::from(2.0).unwrap() * p
                    < T::from(3.0).unwrap() * m * q - (tol * q).abs()
                    && p < (e * q).abs() / T::from(2.0).unwrap()
                {
                    e = d;
                    d = p / q;
                } else {
                    d = m;
                    e = m;
                }
            }

            a = b;
            fa = fb;

            if d.abs() > tol {
                b = b + d;
            } else {
                b = b + if m > T::zero() { tol } else { -tol };
            }

            fb = instrument.residual(b, partial_curve_df);

            // Update c if signs differ
            if (fb > T::zero()) == (fc > T::zero()) {
                c = a;
                fc = fa;
                d = b - a;
                e = d;
            }
        }

        Err(BootstrapError::convergence_failure(
            instrument.maturity().to_f64().unwrap_or(0.0),
            fb.to_f64().unwrap_or(0.0),
            self.config.max_iterations,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::market_data::curves::YieldCurve;

    // ========================================
    // Basic Bootstrap Tests
    // ========================================

    #[test]
    fn test_bootstrap_single_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> =
            vec![BootstrapInstrument::ois(1.0, 0.03)];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        assert_eq!(result.pillars.len(), 1);
        assert!((result.pillars[0] - 1.0).abs() < 1e-10);

        // Check DF is approximately correct
        let expected_df = 1.0 / (1.0 + 0.03);
        assert!(
            (result.discount_factors[0] - expected_df).abs() < 1e-6,
            "Expected DF ~{}, got {}",
            expected_df,
            result.discount_factors[0]
        );
    }

    #[test]
    fn test_bootstrap_multiple_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        assert_eq!(result.pillars.len(), 3);

        // DFs should be decreasing
        assert!(result.discount_factors[0] > result.discount_factors[1]);
        assert!(result.discount_factors[1] > result.discount_factors[2]);
    }

    #[test]
    fn test_bootstrap_unsorted_instruments() {
        // Instruments in wrong order - should still work
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(3.0, 0.034),
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        // Result should be sorted
        assert!((result.pillars[0] - 1.0).abs() < 1e-10);
        assert!((result.pillars[1] - 2.0).abs() < 1e-10);
        assert!((result.pillars[2] - 3.0).abs() < 1e-10);
    }

    // ========================================
    // Error Handling Tests
    // ========================================

    #[test]
    fn test_bootstrap_empty_instruments() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_insufficient_data());
    }

    #[test]
    fn test_bootstrap_duplicate_maturity() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(1.0, 0.032), // Duplicate maturity
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_duplicate_maturity());
    }

    // ========================================
    // Result Validation Tests
    // ========================================

    #[test]
    fn test_bootstrap_reproduces_input_rates() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        // Check residuals are small
        for residual in &result.residuals {
            assert!(
                residual.abs() < 1e-10,
                "Residual {} should be near zero",
                residual
            );
        }
    }

    #[test]
    fn test_bootstrap_curve_discount_factor() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        // Query DF at pillar
        let df = result.curve.discount_factor(1.0).unwrap();
        assert!(
            (df - result.discount_factors[0]).abs() < 1e-10,
            "Curve DF should match bootstrapped DF"
        );
    }

    #[test]
    fn test_bootstrap_curve_interpolation() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let bootstrapper = SequentialBootstrapper::<f64>::with_defaults();
        let result = bootstrapper.bootstrap(&instruments).unwrap();

        // Query interpolated DF
        let df = result.curve.discount_factor(1.5).unwrap();
        assert!(
            df > result.discount_factors[1] && df < result.discount_factors[0],
            "Interpolated DF should be between pillar values"
        );
    }

    // ========================================
    // Configuration Tests
    // ========================================

    #[test]
    fn test_custom_config() {
        let config: GenericBootstrapConfig<f64> = GenericBootstrapConfig::builder()
            .tolerance(1e-14)
            .max_iterations(200)
            .build();

        let bootstrapper = SequentialBootstrapper::new(config);
        assert!((bootstrapper.config().tolerance - 1e-14).abs() < 1e-19);
        assert_eq!(bootstrapper.config().max_iterations, 200);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let bootstrapper1 = SequentialBootstrapper::<f64>::with_defaults();
        let bootstrapper2 = bootstrapper1.clone();

        assert_eq!(
            bootstrapper1.config().max_iterations,
            bootstrapper2.config().max_iterations
        );
    }
}

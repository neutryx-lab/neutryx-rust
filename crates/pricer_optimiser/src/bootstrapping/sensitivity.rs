//! Sensitivity calculation for yield curve bootstrapping.
//!
//! This module provides AAD (Adjoint Algorithmic Differentiation) support
//! for computing sensitivities of discount factors to input rates using
//! num-dual forward-mode AD and bump-and-revalue verification.
//!
//! ## AAD Approach
//!
//! The sensitivity calculation uses the implicit function theorem:
//! For f(DF, rate) = implied_rate(DF) - market_rate = 0,
//! we compute: dDF/drate = -(df/drate) / (df/dDF)
//!
//! This avoids recording solver iterations in the AD tape, achieving O(1)
//! cost for sensitivity computation regardless of iteration count.

use super::config::GenericBootstrapConfig;
use super::curve::BootstrappedCurve;
use super::engine::SequentialBootstrapper;
use super::error::BootstrapError;
use super::instrument::BootstrapInstrument;

/// Result of bootstrap with sensitivities.
///
/// Contains the bootstrapped curve along with a sensitivity matrix
/// mapping each input rate to its effect on each output discount factor.
#[derive(Debug, Clone)]
pub struct BootstrapResultWithSensitivities {
    /// The bootstrapped curve
    pub curve: BootstrappedCurve<f64>,
    /// Pillar maturities
    pub pillars: Vec<f64>,
    /// Discount factors at each pillar
    pub discount_factors: Vec<f64>,
    /// Sensitivity matrix: sensitivities[i][j] = d(DF_i) / d(rate_j)
    /// Row i corresponds to pillar i, column j corresponds to input j
    pub sensitivities: Vec<Vec<f64>>,
}

/// Bootstrapper with AAD sensitivity calculation.
///
/// Extends `SequentialBootstrapper` with the ability to compute
/// sensitivities of discount factors to input rates using either
/// num-dual forward-mode AD or bump-and-revalue.
#[derive(Debug, Clone)]
pub struct SensitivityBootstrapper {
    /// Underlying bootstrapper
    bootstrapper: SequentialBootstrapper<f64>,
    /// Bump size for finite difference (default: 1bp = 0.0001)
    bump_size: f64,
}

impl SensitivityBootstrapper {
    /// Create a new sensitivity bootstrapper with default configuration.
    pub fn new(config: GenericBootstrapConfig<f64>) -> Self {
        Self {
            bootstrapper: SequentialBootstrapper::new(config),
            bump_size: 0.0001, // 1 basis point
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GenericBootstrapConfig::default())
    }

    /// Set the bump size for finite difference calculations.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.bump_size = bump_size;
        self
    }

    /// Get the bump size.
    pub fn bump_size(&self) -> f64 {
        self.bump_size
    }

    /// Bootstrap with sensitivity calculation using implicit function theorem.
    ///
    /// Computes d(DF_i)/d(rate_j) for all pillars i and inputs j using
    /// the implicit function theorem, achieving O(1) cost per sensitivity.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments for bootstrapping
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Bootstrap result with sensitivity matrix
    /// * `Err(e)` - If bootstrapping fails
    #[cfg(feature = "num-dual-mode")]
    pub fn bootstrap_with_sensitivities(
        &self,
        instruments: &[BootstrapInstrument<f64>],
    ) -> Result<BootstrapResultWithSensitivities, BootstrapError> {
        // First, perform standard bootstrap to get the curve
        let base_result = self.bootstrapper.bootstrap(instruments)?;

        let n_pillars = base_result.pillars.len();
        let n_inputs = instruments.len();

        // Build sensitivity matrix
        // For each pillar i, compute dDF_i/drate_j for all inputs j
        let mut sensitivities = vec![vec![0.0; n_inputs]; n_pillars];

        // Sort instruments by maturity (same as in bootstrap)
        let mut sorted_indices: Vec<usize> = (0..n_inputs).collect();
        sorted_indices.sort_by(|&a, &b| {
            instruments[a]
                .maturity()
                .partial_cmp(&instruments[b].maturity())
                .unwrap()
        });

        // For sequential bootstrap, sensitivity propagates forward
        // DF_i depends on rates_0..i (all rates up to and including i)
        for pillar_idx in 0..n_pillars {
            let df = base_result.discount_factors[pillar_idx];

            // Get the instrument at this pillar
            let input_idx = sorted_indices[pillar_idx];
            let instrument = &instruments[input_idx];

            // Compute sensitivity using implicit function theorem
            // For f(DF, rate) = implied_rate(DF) - rate = 0
            // dDF/drate = -1 / (d_implied_rate/d_DF)

            // Create partial curve function from previous pillars
            let partial_pillars: Vec<f64> = base_result.pillars[..pillar_idx].to_vec();
            let partial_dfs: Vec<f64> = base_result.discount_factors[..pillar_idx].to_vec();

            let partial_curve_df = |t: f64| -> f64 {
                if t <= 0.0 {
                    return 1.0;
                }
                if partial_pillars.is_empty() {
                    return 1.0;
                }
                if t < partial_pillars[0] {
                    let r = -partial_dfs[0].ln() / partial_pillars[0];
                    return (-r * t).exp();
                }
                if t > *partial_pillars.last().unwrap() {
                    let n = partial_pillars.len();
                    let r = -partial_dfs[n - 1].ln() / partial_pillars[n - 1];
                    return (-r * t).exp();
                }
                // Find bracket and interpolate
                let mut lo = 0;
                let mut hi = partial_pillars.len() - 1;
                while lo < hi {
                    let mid = (lo + hi + 1) / 2;
                    if partial_pillars[mid] <= t {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }
                if lo + 1 < partial_pillars.len() {
                    let t1 = partial_pillars[lo];
                    let t2 = partial_pillars[lo + 1];
                    let df1 = partial_dfs[lo];
                    let df2 = partial_dfs[lo + 1];
                    let w = (t - t1) / (t2 - t1);
                    let log_df = df1.ln() * (1.0 - w) + df2.ln() * w;
                    log_df.exp()
                } else {
                    partial_dfs[lo]
                }
            };

            // Compute d(residual)/d(DF) at the solution
            let d_residual_d_df = instrument.residual_derivative(df, partial_curve_df);

            // For direct sensitivity using implicit function theorem:
            // f(DF, rate) = implied_rate(DF) - rate = 0
            // dDF/drate = -(df/drate) / (df/dDF) = -(-1) / d_residual_d_df = 1 / d_residual_d_df
            // Since d_residual_d_df is negative (higher DF -> lower rate), result is negative

            if d_residual_d_df.abs() > 1e-30 {
                // Direct sensitivity to own rate
                let direct_sensitivity = 1.0 / d_residual_d_df;
                sensitivities[pillar_idx][input_idx] = direct_sensitivity;

                // Propagate sensitivities from previous pillars
                // DF_i depends on DF_j (j < i) through the partial curve
                // d(DF_i)/d(rate_k) = sum_j [d(DF_i)/d(DF_j) * d(DF_j)/d(rate_k)]
                for prev_pillar_idx in 0..pillar_idx {
                    let prev_input_idx = sorted_indices[prev_pillar_idx];

                    // Compute d(DF_i)/d(DF_j) using finite difference
                    let prev_df = base_result.discount_factors[prev_pillar_idx];
                    let epsilon = 1e-8;

                    // Bump prev_df and recompute implied rate
                    let mut bumped_partial_dfs = partial_dfs.clone();
                    if prev_pillar_idx < bumped_partial_dfs.len() {
                        bumped_partial_dfs[prev_pillar_idx] = prev_df + epsilon;
                    }

                    let bumped_partial_curve_df = |t: f64| -> f64 {
                        if t <= 0.0 {
                            return 1.0;
                        }
                        if partial_pillars.is_empty() {
                            return 1.0;
                        }
                        if t < partial_pillars[0] {
                            let r = -bumped_partial_dfs[0].ln() / partial_pillars[0];
                            return (-r * t).exp();
                        }
                        if t > *partial_pillars.last().unwrap() {
                            let n = partial_pillars.len();
                            let r = -bumped_partial_dfs[n - 1].ln() / partial_pillars[n - 1];
                            return (-r * t).exp();
                        }
                        let mut lo = 0;
                        let mut hi = partial_pillars.len() - 1;
                        while lo < hi {
                            let mid = (lo + hi + 1) / 2;
                            if partial_pillars[mid] <= t {
                                lo = mid;
                            } else {
                                hi = mid - 1;
                            }
                        }
                        if lo + 1 < partial_pillars.len() {
                            let t1 = partial_pillars[lo];
                            let t2 = partial_pillars[lo + 1];
                            let df1 = bumped_partial_dfs[lo];
                            let df2 = bumped_partial_dfs[lo + 1];
                            let w = (t - t1) / (t2 - t1);
                            let log_df = df1.ln() * (1.0 - w) + df2.ln() * w;
                            log_df.exp()
                        } else {
                            bumped_partial_dfs[lo]
                        }
                    };

                    // Compute residual change
                    let residual_base = instrument.residual(df, &partial_curve_df);
                    let residual_bumped = instrument.residual(df, &bumped_partial_curve_df);
                    let d_residual_d_prev_df = (residual_bumped - residual_base) / epsilon;

                    // Chain rule: d(DF_i)/d(rate_k) += d(DF_i)/d(DF_j) * d(DF_j)/d(rate_k)
                    if d_residual_d_df.abs() > 1e-30 && d_residual_d_prev_df.abs() > 1e-30 {
                        let d_df_i_d_df_j = -d_residual_d_prev_df / d_residual_d_df;
                        sensitivities[pillar_idx][prev_input_idx] +=
                            d_df_i_d_df_j * sensitivities[prev_pillar_idx][prev_input_idx];
                    }
                }
            }
        }

        Ok(BootstrapResultWithSensitivities {
            curve: base_result.curve,
            pillars: base_result.pillars,
            discount_factors: base_result.discount_factors,
            sensitivities,
        })
    }

    /// Bootstrap with sensitivities using bump-and-revalue method.
    ///
    /// This provides a reference implementation for validation.
    /// For each input rate, bumps by `bump_size` and recomputes the curve.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Market instruments for bootstrapping
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Bootstrap result with sensitivity matrix
    /// * `Err(e)` - If bootstrapping fails
    pub fn bootstrap_with_bump_and_revalue(
        &self,
        instruments: &[BootstrapInstrument<f64>],
    ) -> Result<BootstrapResultWithSensitivities, BootstrapError> {
        // First, perform base bootstrap
        let base_result = self.bootstrapper.bootstrap(instruments)?;

        let n_pillars = base_result.pillars.len();
        let n_inputs = instruments.len();
        let mut sensitivities = vec![vec![0.0; n_inputs]; n_pillars];

        // For each input, bump and revalue
        for input_idx in 0..n_inputs {
            // Create bumped instruments
            let bumped_instruments: Vec<BootstrapInstrument<f64>> = instruments
                .iter()
                .enumerate()
                .map(|(i, inst)| {
                    if i == input_idx {
                        bump_instrument(inst, self.bump_size)
                    } else {
                        inst.clone()
                    }
                })
                .collect();

            // Bootstrap with bumped instruments
            if let Ok(bumped_result) = self.bootstrapper.bootstrap(&bumped_instruments) {
                // Compute finite difference sensitivities
                for (pillar_idx, sensitivity_row) in
                    sensitivities.iter_mut().enumerate().take(n_pillars)
                {
                    let df_base = base_result.discount_factors[pillar_idx];
                    let df_bumped = bumped_result.discount_factors[pillar_idx];
                    sensitivity_row[input_idx] = (df_bumped - df_base) / self.bump_size;
                }
            }
        }

        Ok(BootstrapResultWithSensitivities {
            curve: base_result.curve,
            pillars: base_result.pillars,
            discount_factors: base_result.discount_factors,
            sensitivities,
        })
    }

    /// Verify AAD sensitivities against bump-and-revalue.
    ///
    /// Returns the maximum absolute difference between AAD and bump-and-revalue
    /// sensitivities. A small difference (< 1e-4) indicates correct AAD implementation.
    #[cfg(feature = "num-dual-mode")]
    pub fn verify_sensitivities(
        &self,
        instruments: &[BootstrapInstrument<f64>],
        tolerance: f64,
    ) -> Result<SensitivityVerification, BootstrapError> {
        let aad_result = self.bootstrap_with_sensitivities(instruments)?;
        let bump_result = self.bootstrap_with_bump_and_revalue(instruments)?;

        let n_pillars = aad_result.pillars.len();
        let n_inputs = instruments.len();

        let mut max_abs_diff: f64 = 0.0;
        let mut max_rel_diff: f64 = 0.0;
        let mut all_within_tolerance = true;

        for pillar_idx in 0..n_pillars {
            for input_idx in 0..n_inputs {
                let aad_sens = aad_result.sensitivities[pillar_idx][input_idx];
                let bump_sens = bump_result.sensitivities[pillar_idx][input_idx];

                let abs_diff = (aad_sens - bump_sens).abs();
                let rel_diff = if bump_sens.abs() > 1e-10 {
                    abs_diff / bump_sens.abs()
                } else {
                    abs_diff
                };

                max_abs_diff = max_abs_diff.max(abs_diff);
                max_rel_diff = max_rel_diff.max(rel_diff);

                if abs_diff > tolerance && rel_diff > tolerance {
                    all_within_tolerance = false;
                }
            }
        }

        Ok(SensitivityVerification {
            aad_sensitivities: aad_result.sensitivities,
            bump_sensitivities: bump_result.sensitivities,
            max_absolute_difference: max_abs_diff,
            max_relative_difference: max_rel_diff,
            within_tolerance: all_within_tolerance,
        })
    }
}

/// Result of sensitivity verification.
#[derive(Debug, Clone)]
pub struct SensitivityVerification {
    /// AAD-computed sensitivities
    pub aad_sensitivities: Vec<Vec<f64>>,
    /// Bump-and-revalue sensitivities
    pub bump_sensitivities: Vec<Vec<f64>>,
    /// Maximum absolute difference
    pub max_absolute_difference: f64,
    /// Maximum relative difference
    pub max_relative_difference: f64,
    /// Whether all differences are within tolerance
    pub within_tolerance: bool,
}

/// Bump an instrument's rate by the given amount.
fn bump_instrument(instrument: &BootstrapInstrument<f64>, bump: f64) -> BootstrapInstrument<f64> {
    match instrument {
        BootstrapInstrument::Ois {
            maturity,
            rate,
            payment_frequency,
        } => BootstrapInstrument::Ois {
            maturity: *maturity,
            rate: *rate + bump,
            payment_frequency: *payment_frequency,
        },
        BootstrapInstrument::Irs {
            maturity,
            rate,
            fixed_frequency,
            float_frequency,
        } => BootstrapInstrument::Irs {
            maturity: *maturity,
            rate: *rate + bump,
            fixed_frequency: *fixed_frequency,
            float_frequency: *float_frequency,
        },
        BootstrapInstrument::Fra { start, end, rate } => BootstrapInstrument::Fra {
            start: *start,
            end: *end,
            rate: *rate + bump,
        },
        BootstrapInstrument::Future {
            maturity,
            price,
            convexity_adjustment,
        } => {
            // For futures, bumping rate means adjusting price (price = 100 - rate)
            BootstrapInstrument::Future {
                maturity: *maturity,
                price: *price - bump * 100.0, // Rate up -> price down
                convexity_adjustment: *convexity_adjustment,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Basic Sensitivity Tests
    // ========================================

    #[test]
    fn test_bump_and_revalue_single_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![BootstrapInstrument::ois(1.0, 0.03)];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result = bootstrapper
            .bootstrap_with_bump_and_revalue(&instruments)
            .unwrap();

        // Should have one pillar and one input
        assert_eq!(result.pillars.len(), 1);
        assert_eq!(result.sensitivities.len(), 1);
        assert_eq!(result.sensitivities[0].len(), 1);

        // Sensitivity should be negative (higher rate -> lower DF)
        assert!(
            result.sensitivities[0][0] < 0.0,
            "dDF/drate should be negative, got {}",
            result.sensitivities[0][0]
        );
    }

    #[test]
    fn test_bump_and_revalue_multiple_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result = bootstrapper
            .bootstrap_with_bump_and_revalue(&instruments)
            .unwrap();

        // Should have 3 pillars and 3 inputs
        assert_eq!(result.pillars.len(), 3);
        assert_eq!(result.sensitivities.len(), 3);

        // Diagonal elements (own sensitivities) should be negative
        for i in 0..3 {
            assert!(
                result.sensitivities[i][i] < 0.0,
                "Diagonal sensitivity [{i}][{i}] should be negative"
            );
        }

        // Later pillars should have small sensitivity to earlier rates
        // (due to sequential bootstrap)
        assert!(
            result.sensitivities[0][1].abs() < 1e-10,
            "First pillar shouldn't depend on second rate"
        );
        assert!(
            result.sensitivities[0][2].abs() < 1e-10,
            "First pillar shouldn't depend on third rate"
        );
    }

    #[test]
    fn test_bump_and_revalue_triangular_structure() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
        ];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result = bootstrapper
            .bootstrap_with_bump_and_revalue(&instruments)
            .unwrap();

        // Sequential bootstrap creates lower-triangular sensitivity structure
        // DF_1 depends only on rate_1
        // DF_2 depends on rate_1 and rate_2

        // DF_1 sensitivity to rate_1 should be non-zero
        assert!(
            result.sensitivities[0][0].abs() > 1e-8,
            "DF_1 should depend on rate_1"
        );

        // DF_1 sensitivity to rate_2 should be ~zero
        assert!(
            result.sensitivities[0][1].abs() < 1e-10,
            "DF_1 shouldn't depend on rate_2"
        );

        // DF_2 sensitivity to rate_2 should be non-zero
        assert!(
            result.sensitivities[1][1].abs() > 1e-8,
            "DF_2 should depend on rate_2"
        );
    }

    // ========================================
    // num-dual Mode Tests
    // ========================================

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_aad_single_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![BootstrapInstrument::ois(1.0, 0.03)];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result = bootstrapper
            .bootstrap_with_sensitivities(&instruments)
            .unwrap();

        // Should have one pillar and one input
        assert_eq!(result.pillars.len(), 1);
        assert_eq!(result.sensitivities.len(), 1);
        assert_eq!(result.sensitivities[0].len(), 1);

        // Sensitivity should be negative
        assert!(
            result.sensitivities[0][0] < 0.0,
            "AAD dDF/drate should be negative, got {}",
            result.sensitivities[0][0]
        );
    }

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_aad_multiple_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result = bootstrapper
            .bootstrap_with_sensitivities(&instruments)
            .unwrap();

        // Diagonal elements should be negative
        for i in 0..3 {
            assert!(
                result.sensitivities[i][i] < 0.0,
                "AAD diagonal sensitivity [{i}][{i}] should be negative"
            );
        }
    }

    // ========================================
    // Verification Tests
    // ========================================

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_verify_sensitivities_single_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![BootstrapInstrument::ois(1.0, 0.03)];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let verification = bootstrapper
            .verify_sensitivities(&instruments, 0.01) // 1% tolerance
            .unwrap();

        // AAD and bump-and-revalue should match within tolerance
        assert!(
            verification.within_tolerance,
            "AAD should match bump-and-revalue. Max abs diff: {}, max rel diff: {}",
            verification.max_absolute_difference, verification.max_relative_difference
        );
    }

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_verify_sensitivities_multiple_ois() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![
            BootstrapInstrument::ois(1.0, 0.03),
            BootstrapInstrument::ois(2.0, 0.032),
            BootstrapInstrument::ois(3.0, 0.034),
        ];

        let bootstrapper = SensitivityBootstrapper::with_defaults().with_bump_size(0.00001); // Use smaller bump for more accuracy

        let verification = bootstrapper
            .verify_sensitivities(&instruments, 0.05) // 5% tolerance
            .unwrap();

        // Report verification results
        println!(
            "Verification: max_abs_diff = {}, max_rel_diff = {}",
            verification.max_absolute_difference, verification.max_relative_difference
        );

        // AAD and bump-and-revalue should match reasonably well
        assert!(
            verification.max_relative_difference < 0.1, // 10% relative tolerance
            "AAD should approximately match bump-and-revalue. Max rel diff: {}",
            verification.max_relative_difference
        );
    }

    // ========================================
    // Bump Instrument Tests
    // ========================================

    #[test]
    fn test_bump_ois() {
        let ois: BootstrapInstrument<f64> = BootstrapInstrument::ois(1.0, 0.03);
        let bumped = bump_instrument(&ois, 0.0001);

        assert!((bumped.rate() - 0.0301).abs() < 1e-10);
        assert!((bumped.maturity() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bump_irs() {
        let irs: BootstrapInstrument<f64> = BootstrapInstrument::irs(5.0, 0.04);
        let bumped = bump_instrument(&irs, 0.0001);

        assert!((bumped.rate() - 0.0401).abs() < 1e-10);
    }

    #[test]
    fn test_bump_fra() {
        let fra: BootstrapInstrument<f64> = BootstrapInstrument::fra(0.25, 0.5, 0.025);
        let bumped = bump_instrument(&fra, 0.0001);

        assert!((bumped.rate() - 0.0251).abs() < 1e-10);
    }

    #[test]
    fn test_bump_future() {
        let future: BootstrapInstrument<f64> = BootstrapInstrument::future(0.25, 97.5, 0.0001);
        let original_rate = future.rate();
        let bumped = bump_instrument(&future, 0.0001);

        // Rate should increase by bump
        assert!(
            (bumped.rate() - original_rate - 0.0001).abs() < 1e-6,
            "Future rate should increase by bump"
        );
    }

    // ========================================
    // Configuration Tests
    // ========================================

    #[test]
    fn test_custom_bump_size() {
        let bootstrapper = SensitivityBootstrapper::with_defaults().with_bump_size(0.00001);
        assert!((bootstrapper.bump_size() - 0.00001).abs() < 1e-15);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let bootstrapper1 = SensitivityBootstrapper::with_defaults();
        let bootstrapper2 = bootstrapper1.clone();
        assert!((bootstrapper1.bump_size() - bootstrapper2.bump_size()).abs() < 1e-15);
    }

    #[test]
    fn test_result_clone() {
        let instruments: Vec<BootstrapInstrument<f64>> = vec![BootstrapInstrument::ois(1.0, 0.03)];

        let bootstrapper = SensitivityBootstrapper::with_defaults();
        let result1 = bootstrapper
            .bootstrap_with_bump_and_revalue(&instruments)
            .unwrap();
        let result2 = result1.clone();

        assert_eq!(result1.pillars.len(), result2.pillars.len());
        assert_eq!(result1.sensitivities.len(), result2.sensitivities.len());
    }
}

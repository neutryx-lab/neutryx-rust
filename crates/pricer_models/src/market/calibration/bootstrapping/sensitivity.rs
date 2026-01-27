//! Sensitivity calculation for yield curve bootstrapping.
//!
//! This module provides sensitivity calculations for discount factors
//! to input rates using bump-and-revalue method.
//!
//! ## Approach
//!
//! The sensitivity calculation uses finite differences (bump-and-revalue)
//! to compute dDF/drate for each input rate.

use super::{
    config::GenericBootstrapConfig, curve::BootstrappedCurve, engine::SequentialBootstrapper,
    error::BootstrapError, instrument::BootstrapInstrument,
};

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
    /// Sensitivity matrix: `sensitivities[i][j]` = d(DF_i) / d(rate_j)
    /// Row i corresponds to pillar i, column j corresponds to input j
    pub sensitivities: Vec<Vec<f64>>,
}

/// Bootstrapper with sensitivity calculation.
///
/// Extends `SequentialBootstrapper` with the ability to compute
/// sensitivities of discount factors to input rates using bump-and-revalue.
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
    pub fn with_defaults() -> Self { Self::new(GenericBootstrapConfig::default()) }

    /// Set the bump size for finite difference calculations.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.bump_size = bump_size;
        self
    }

    /// Get the bump size.
    pub fn bump_size(&self) -> f64 { self.bump_size }

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
                price: bump.mul_add(-100.0, *price), // Rate up -> price down
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

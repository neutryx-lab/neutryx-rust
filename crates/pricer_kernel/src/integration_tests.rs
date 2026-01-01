//! L1/L2 integration tests for pricer_kernel.
//!
//! These tests verify that pricer_kernel correctly integrates with
//! pricer_core (L1) and pricer_models (L2) when the `l1l2-integration` feature is enabled.

#[cfg(all(test, feature = "l1l2-integration"))]
mod tests {
    use pricer_core::math::smoothing::{smooth_indicator, smooth_max};
    use pricer_core::traits::Float;

    /// Test that smooth_max from pricer_core is accessible and works correctly.
    #[test]
    fn test_smooth_max_integration() {
        let a = 3.0_f64;
        let b = 5.0_f64;
        let epsilon = 1e-6;

        let result = smooth_max(a, b, epsilon);

        // smooth_max should approximate max(a, b) = 5.0
        assert!((result - 5.0).abs() < 1e-3);
    }

    /// Test that smooth_indicator from pricer_core is accessible and works correctly.
    #[test]
    fn test_smooth_indicator_integration() {
        let epsilon = 1e-6;

        // At x=0, smooth_indicator should be approximately 0.5
        let at_zero = smooth_indicator(0.0_f64, epsilon);
        assert!((at_zero - 0.5).abs() < 1e-3);

        // For large positive x, should be close to 1
        let positive = smooth_indicator(10.0_f64, epsilon);
        assert!(positive > 0.99);

        // For large negative x, should be close to 0
        let negative = smooth_indicator(-10.0_f64, epsilon);
        assert!(negative < 0.01);
    }

    /// Test that Float trait from pricer_core is accessible.
    #[test]
    fn test_float_trait_integration() {
        fn use_float<T: Float>(x: T) -> T {
            x + x
        }

        let result = use_float(2.0_f64);
        assert!((result - 4.0).abs() < 1e-10);
    }
}

#[cfg(all(test, feature = "l1l2-integration"))]
mod pricer_models_tests {
    use pricer_models::models::stochastic::StochasticModel;
    use pricer_models::models::StochasticModelEnum;

    /// Test that StochasticModel trait from pricer_models is accessible.
    #[test]
    fn test_stochastic_model_trait_integration() {
        // Verify the trait is importable and usable
        fn accepts_stochastic_model<M: StochasticModel<f64>>(_model: &M) {
            // Just verify the trait bound compiles
        }

        // This test just verifies the import works
        // Actual model usage will be tested in later phases
    }

    /// Test that StochasticModelEnum is accessible for static dispatch.
    #[test]
    fn test_stochastic_model_enum_accessible() {
        // Verify the enum can be imported
        // Full usage tests will be in later phases
        let _enum_type_check: Option<StochasticModelEnum<f64>> = None;
    }
}

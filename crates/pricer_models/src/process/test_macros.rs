/// Generates common stochastic model tests that are identical in structure
/// across all models: model creation, initial state, evolve step (no shock,
/// positive shock, negative shock), differentiability, f32 compatibility,
/// and multi-step stability.
///
/// Model-specific tests (Feller condition, martingale property, QE scheme,
/// theta function, analytical pricers, etc.) remain hand-written in each
/// model file.
macro_rules! generate_stochastic_model_tests {
    (
        model: $model:ty,
        model_f32: $model_f32:ty,
        default_f64_params: $f64_params:expr,
        default_f32_params: $f32_params:expr,
        model_name: $name:expr,
        brownian_dim: $bdim:expr,
        num_factors: $nf:expr,
        zero_shock: [$($zs:expr),*],
        positive_shock: [$($ps:expr),*],
        negative_shock: [$($ns:expr),*],
        price_increased: $inc:expr,
        price_decreased: $dec:expr,
        state_finite_check: $finite:expr $(,)?
    ) => {
        mod generated_tests {
            use super::*;

            #[test]
            fn test_model_basics() {
                assert_eq!(<$model>::model_name(), $name);
                assert_eq!(<$model>::brownian_dim(), $bdim);
                assert_eq!(<$model>::num_factors(), $nf);
            }

            #[test]
            fn test_initial_state() {
                let params = $f64_params;
                let _state = <$model>::initial_state(&params);
            }

            #[test]
            fn test_evolve_step_no_shock() {
                let params = $f64_params;
                let state = <$model>::initial_state(&params);
                let dw = [$($zs),*];
                let next = <$model>::evolve_step(state, 1.0 / 252.0, &dw, &params);
                let check: fn(&_) -> bool = $finite;
                assert!(check(&next), "State should be finite after zero-shock step");
            }

            #[test]
            fn test_evolve_step_positive_shock() {
                let params = $f64_params;
                let state = <$model>::initial_state(&params);
                let dw = [$($ps),*];
                let next = <$model>::evolve_step(state, 1.0 / 252.0, &dw, &params);
                let check: fn(&_, &_) -> bool = $inc;
                assert!(check(&next, &state), "Positive shock should increase price");
            }

            #[test]
            fn test_evolve_step_negative_shock() {
                let params = $f64_params;
                let state = <$model>::initial_state(&params);
                let dw = [$($ns),*];
                let next = <$model>::evolve_step(state, 1.0 / 252.0, &dw, &params);
                let check: fn(&_, &_) -> bool = $dec;
                assert!(check(&next, &state), "Negative shock should decrease price");
            }

            #[test]
            fn test_is_differentiable() {
                use pricer_core::traits::priceable::Differentiable;
                fn assert_diff<D: Differentiable>(_d: &D) {}
                let model = <$model>::new();
                assert_diff(&model);
            }

            #[test]
            fn test_f32_compatibility() {
                let params = $f32_params;
                let state = <$model_f32>::initial_state(&params);
                let dw: Vec<f32> = vec![0.0_f32; $bdim];
                let next = <$model_f32>::evolve_step(state, 1.0_f32 / 252.0, &dw, &params);
                let _ = next;
            }

            #[test]
            fn test_multi_step_stability() {
                let params = $f64_params;
                let mut state = <$model>::initial_state(&params);
                let dw = [$($zs),*];
                let check: fn(&_) -> bool = $finite;
                for i in 0..252 {
                    state = <$model>::evolve_step(state, 1.0 / 252.0, &dw, &params);
                    assert!(check(&state), "State not finite at step {i}");
                }
            }
        }
    };
}

pub(crate) use generate_stochastic_model_tests;

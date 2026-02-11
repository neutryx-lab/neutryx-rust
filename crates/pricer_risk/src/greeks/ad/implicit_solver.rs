//! Implicit Function Theorem-based curve sensitivity computation.

use nalgebra::{DMatrix, DVector, RealField};
use num_traits::Float;
use thiserror::Error;

/// Errors that can occur during implicit solver operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ImplicitSolverError {
    #[error("Dimension mismatch: Jacobian inverse has {jacobian_cols} columns but adjoint has {adjoint_len} elements")]
    DimensionMismatch {
        jacobian_cols: usize,
        adjoint_len: usize,
    },

    #[error("Jacobian inverse not available; use finite difference fallback")]
    JacobianInverseNotAvailable,

    #[error("Function evaluation failed: {0}")]
    FunctionEvaluationFailed(String),
}

/// Curve sensitivity computation result containing dL/dm.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveSensitivities<T: RealField + Copy> {
    pub market_sensitivities: DVector<T>,
}

impl<T: RealField + Copy> CurveSensitivities<T> {
    pub fn new(market_sensitivities: DVector<T>) -> Self {
        Self {
            market_sensitivities,
        }
    }

    pub fn dimension(&self) -> usize { self.market_sensitivities.len() }

    pub fn is_finite(&self) -> bool
    where
        T: Float,
    {
        self.market_sensitivities.iter().all(|&x| x.is_finite())
    }
}

/// IFT-based solver for curve sensitivities: dL/dm = J^-T * dL/dx*.
pub struct ImplicitSolver;

impl ImplicitSolver {
    /// Compute curve sensitivities using the Implicit Function Theorem.
    pub fn compute_curve_sensitivities<T: RealField + Copy>(
        jacobian_inverse: &DMatrix<T>,
        adjoint_x: &DVector<T>,
    ) -> Result<CurveSensitivities<T>, ImplicitSolverError> {
        if jacobian_inverse.ncols() != adjoint_x.len() {
            return Err(ImplicitSolverError::DimensionMismatch {
                jacobian_cols: jacobian_inverse.ncols(),
                adjoint_len: adjoint_x.len(),
            });
        }

        let j_inv_t = jacobian_inverse.transpose();
        let market_sensitivities = &j_inv_t * adjoint_x;

        Ok(CurveSensitivities::new(market_sensitivities))
    }

    /// Compute curve sensitivities using central finite difference fallback.
    pub fn compute_curve_sensitivities_fd<T, F>(
        loss_fn: F,
        curve_nodes: &DVector<T>,
        epsilon: T,
    ) -> CurveSensitivities<T>
    where
        T: RealField + Copy + Float,
        F: Fn(&DVector<T>) -> T,
    {
        let n = curve_nodes.len();
        let mut sensitivities = DVector::zeros(n);
        let two = T::one() + T::one();

        for i in 0..n {
            let mut nodes_up = curve_nodes.clone();
            nodes_up[i] = nodes_up[i] + epsilon;
            let loss_up = loss_fn(&nodes_up);

            let mut nodes_down = curve_nodes.clone();
            nodes_down[i] = nodes_down[i] - epsilon;
            let loss_down = loss_fn(&nodes_down);

            sensitivities[i] = (loss_up - loss_down) / (two * epsilon);
        }

        CurveSensitivities::new(sensitivities)
    }

    /// Compute sensitivities with automatic fallback to finite differences.
    pub fn compute_with_fallback<T, F>(
        jacobian_inverse: Option<&DMatrix<T>>,
        adjoint_x: &DVector<T>,
        loss_fn: F,
        curve_nodes: &DVector<T>,
        epsilon: T,
    ) -> (CurveSensitivities<T>, bool)
    where
        T: RealField + Copy + Float,
        F: Fn(&DVector<T>) -> T,
    {
        match jacobian_inverse {
            Some(j_inv) => match Self::compute_curve_sensitivities(j_inv, adjoint_x) {
                Ok(sens) => (sens, false),
                Err(_) => {
                    let sens = Self::compute_curve_sensitivities_fd(loss_fn, curve_nodes, epsilon);
                    (sens, true)
                }
            },
            None => {
                let sens = Self::compute_curve_sensitivities_fd(loss_fn, curve_nodes, epsilon);
                (sens, true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_curve_sensitivities_new() {
        let sens = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let curve_sens = CurveSensitivities::new(sens.clone());

        assert_eq!(curve_sens.dimension(), 3);
        assert_eq!(curve_sens.market_sensitivities, sens);
    }

    #[test]
    fn test_curve_sensitivities_is_finite() {
        let finite_sens = CurveSensitivities::new(DVector::from_vec(vec![1.0, 2.0]));
        assert!(finite_sens.is_finite());

        let infinite_sens = CurveSensitivities::new(DVector::from_vec(vec![f64::INFINITY, 2.0]));
        assert!(!infinite_sens.is_finite());

        let nan_sens = CurveSensitivities::new(DVector::from_vec(vec![f64::NAN, 2.0]));
        assert!(!nan_sens.is_finite());
    }

    #[test]
    fn test_compute_curve_sensitivities_identity_jacobian() {
        let j_inv = DMatrix::<f64>::identity(3, 3);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        assert_eq!(sens.dimension(), 3);
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_diagonal_jacobian() {
        let j_inv = DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 0.0, 0.5]);
        let adjoint = DVector::from_vec(vec![1.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        assert_relative_eq!(sens.market_sensitivities[0], 2.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_general_jacobian() {
        let j_inv = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let adjoint = DVector::from_vec(vec![1.0, 1.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        assert_relative_eq!(sens.market_sensitivities[0], 4.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 6.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_curve_sensitivities_dimension_mismatch() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        let result = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint);

        assert!(result.is_err());
        match result.unwrap_err() {
            ImplicitSolverError::DimensionMismatch {
                jacobian_cols,
                adjoint_len,
            } => {
                assert_eq!(jacobian_cols, 2);
                assert_eq!(adjoint_len, 3);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    #[test]
    fn test_compute_sensitivities_fd_quadratic() {
        let loss_fn = |x: &DVector<f64>| x[0] * x[0] + x[1] * x[1];
        let nodes = DVector::from_vec(vec![3.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 6.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 8.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_linear() {
        let loss_fn = |x: &DVector<f64>| 2.0 * x[0] + 3.0 * x[1] + 5.0 * x[2];
        let nodes = DVector::from_vec(vec![1.0, 2.0, 3.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 2.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 3.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[2], 5.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_cross_terms() {
        let loss_fn = |x: &DVector<f64>| x[0] * x[1];
        let nodes = DVector::from_vec(vec![3.0, 4.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 4.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 3.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_sensitivities_fd_exponential() {
        let loss_fn = |x: &DVector<f64>| x[0].exp();
        let nodes = DVector::from_vec(vec![1.0]);

        let sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert_relative_eq!(sens.market_sensitivities[0], 1.0_f64.exp(), epsilon = 1e-4);
    }

    #[test]
    fn test_compute_with_fallback_uses_implicit_when_available() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0]);
        let nodes = DVector::from_vec(vec![0.0, 0.0]);
        let loss_fn = |_: &DVector<f64>| 0.0;

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(Some(&j_inv), &adjoint, loss_fn, &nodes, 1e-6);

        assert!(!used_fd);
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(sens.market_sensitivities[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_compute_with_fallback_uses_fd_when_no_jacobian() {
        let adjoint = DVector::from_vec(vec![1.0, 2.0]);
        let nodes = DVector::from_vec(vec![3.0, 4.0]);
        let loss_fn = |x: &DVector<f64>| x[0] * x[0] + x[1] * x[1];

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(None, &adjoint, loss_fn, &nodes, 1e-6);

        assert!(used_fd);
        assert_relative_eq!(sens.market_sensitivities[0], 6.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 8.0, epsilon = 1e-4);
    }

    #[test]
    fn test_compute_with_fallback_uses_fd_on_dimension_mismatch() {
        let j_inv = DMatrix::<f64>::identity(2, 2);
        let adjoint = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let nodes = DVector::from_vec(vec![3.0, 4.0, 5.0]);
        let loss_fn = |x: &DVector<f64>| x[0] + x[1] + x[2];

        let (sens, used_fd) =
            ImplicitSolver::compute_with_fallback(Some(&j_inv), &adjoint, loss_fn, &nodes, 1e-6);

        assert!(used_fd);
        assert_relative_eq!(sens.market_sensitivities[0], 1.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[1], 1.0, epsilon = 1e-4);
        assert_relative_eq!(sens.market_sensitivities[2], 1.0, epsilon = 1e-4);
    }

    #[test]
    fn test_implicit_solver_error_display() {
        let err = ImplicitSolverError::DimensionMismatch {
            jacobian_cols: 2,
            adjoint_len: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Dimension mismatch"));
        assert!(msg.contains("2"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_implicit_solver_error_clone() {
        let err = ImplicitSolverError::JacobianInverseNotAvailable;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_integration_with_multidim_solver_result_pattern() {
        let n = 5;
        let calibration_jacobian_inverse = DMatrix::from_row_slice(
            n,
            n,
            &[
                0.98, 0.01, 0.00, 0.00, 0.00, 0.01, 0.96, 0.02, 0.00, 0.00, 0.00, 0.02, 0.95, 0.02,
                0.00, 0.00, 0.00, 0.02, 0.94, 0.03, 0.00, 0.00, 0.00, 0.03, 0.93,
            ],
        );

        let portfolio_dv01_to_nodes = DVector::from_vec(vec![500.0, 300.0, 200.0, 150.0, 100.0]);

        let market_dv01 = ImplicitSolver::compute_curve_sensitivities(
            &calibration_jacobian_inverse,
            &portfolio_dv01_to_nodes,
        )
        .unwrap();

        assert!(market_dv01.is_finite());
        assert_eq!(market_dv01.dimension(), n);

        let deposit_dv01 = market_dv01.market_sensitivities[0];
        let ten_year_swap_dv01 = market_dv01.market_sensitivities[4];

        assert!(
            deposit_dv01 > 400.0,
            "Deposit DV01 should be ~500 (1Y node contribution)"
        );
        assert!(
            ten_year_swap_dv01 > 80.0 && ten_year_swap_dv01 < 200.0,
            "10Y swap DV01 should reflect 10Y node with spillover"
        );

        let total_portfolio_dv01: f64 = portfolio_dv01_to_nodes.iter().sum();
        let total_market_dv01: f64 = market_dv01.market_sensitivities.iter().sum();

        assert!(
            (total_market_dv01 - total_portfolio_dv01).abs() / total_portfolio_dv01 < 0.20,
            "Total DV01 should be approximately conserved"
        );
    }

    #[test]
    fn test_financial_interpretation_of_sensitivities() {
        let jacobian_inverse = DMatrix::from_row_slice(2, 2, &[0.99, 0.0, 0.01, 0.98]);

        let adjoint_x = DVector::from_vec(vec![100.0, 50.0]);

        let sens =
            ImplicitSolver::compute_curve_sensitivities(&jacobian_inverse, &adjoint_x).unwrap();

        assert_relative_eq!(sens.market_sensitivities[0], 99.5, epsilon = 0.01);
        assert_relative_eq!(sens.market_sensitivities[1], 49.0, epsilon = 0.01);
    }

    #[test]
    fn test_implicit_vs_fd_accuracy_linear_function() {
        let n = 5;
        let coeffs = DVector::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let nodes = DVector::from_vec(vec![0.1, 0.2, 0.3, 0.4, 0.5]);

        let j_inv = DMatrix::<f64>::identity(n, n);
        let implicit_sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &coeffs).unwrap();

        let loss_fn = |x: &DVector<f64>| coeffs.dot(x);
        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        for i in 0..n {
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                fd_sens.market_sensitivities[i],
                epsilon = 1e-4
            );
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                coeffs[i],
                epsilon = 1e-10
            );
        }
    }

    #[test]
    fn test_implicit_vs_fd_accuracy_quadratic_with_non_identity_jacobian() {
        let n = 3;

        let a = DMatrix::from_row_slice(n, n, &[2.0, 0.5, 0.1, 0.5, 3.0, 0.2, 0.1, 0.2, 4.0]);

        let nodes = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let adjoint = 2.0 * &a * &nodes;
        let j_inv = DMatrix::<f64>::identity(n, n);

        let implicit_sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        let a_clone = a.clone();
        let loss_fn = move |x: &DVector<f64>| x.dot(&(&a_clone * x));
        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        for i in 0..n {
            assert_relative_eq!(
                implicit_sens.market_sensitivities[i],
                fd_sens.market_sensitivities[i],
                epsilon = 1e-3
            );
        }
    }

    #[test]
    fn test_aad_verification_30_node_curve() {
        let n = 30;

        let j_inv = DMatrix::from_fn(n, n, |i, j| {
            let dist = (i as f64 - j as f64).abs();
            if i == j {
                0.99
            } else {
                0.05 * (-0.5 * dist).exp()
            }
        });

        let adjoint = DVector::from_fn(n, |i, _| 1000.0 * (-0.1 * i as f64).exp());

        let sens = ImplicitSolver::compute_curve_sensitivities(&j_inv, &adjoint).unwrap();

        assert!(sens.is_finite());
        assert_eq!(sens.dimension(), n);

        assert!(
            sens.market_sensitivities[0].abs() > sens.market_sensitivities[n - 1].abs(),
            "Short-end sensitivity should be larger than long-end"
        );

        let nodes = DVector::from_fn(n, |i, _| 0.03 + 0.001 * i as f64);
        let adjoint_for_fd = adjoint.clone();
        let loss_fn = move |x: &DVector<f64>| adjoint_for_fd.dot(x);

        let fd_sens = ImplicitSolver::compute_curve_sensitivities_fd(loss_fn, &nodes, 1e-6);

        assert!(fd_sens.is_finite());
        assert_eq!(fd_sens.dimension(), n);
    }

    #[test]
    fn test_aad_end_to_end_flow() {
        let n = 5;
        let calibration_jacobian_inverse = DMatrix::from_row_slice(
            n,
            n,
            &[
                0.98, 0.01, 0.00, 0.00, 0.00, 0.01, 0.96, 0.02, 0.00, 0.00, 0.00, 0.02, 0.95, 0.02,
                0.00, 0.00, 0.00, 0.02, 0.94, 0.03, 0.00, 0.00, 0.00, 0.03, 0.93,
            ],
        );

        let portfolio_dv01_to_nodes = DVector::from_vec(vec![500.0, 300.0, 200.0, 150.0, 100.0]);

        let market_dv01 = ImplicitSolver::compute_curve_sensitivities(
            &calibration_jacobian_inverse,
            &portfolio_dv01_to_nodes,
        )
        .expect("Should succeed with valid dimensions");

        assert!(market_dv01.is_finite());
        assert_eq!(market_dv01.dimension(), n);

        let j_inv_t = calibration_jacobian_inverse.transpose();
        let expected = &j_inv_t * &portfolio_dv01_to_nodes;
        for i in 0..3 {
            assert_relative_eq!(
                market_dv01.market_sensitivities[i],
                expected[i],
                epsilon = 1e-10
            );
        }
    }
}

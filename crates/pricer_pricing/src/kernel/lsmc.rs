//! Longstaff-Schwartz Monte Carlo (LSMC) regression for callable products.
//!
//! This module provides regression functionality for optimal exercise
//! determination in American/Bermudan option pricing.
//!
//! # Algorithm
//!
//! At each exercise date, we regress the continuation value against
//! basis functions of the state variables:
//!
//! ```text
//! E[V_{t+1} | X_t] ≈ Σᵢ αᵢ φᵢ(X_t)
//! ```
//!
//! where:
//! - `V_{t+1}` is the discounted future option value
//! - `X_t` is the state variable (e.g., short rate)
//! - `φᵢ` are basis functions
//! - `αᵢ` are regression coefficients
//!
//! # Example
//!
//! ```ignore
//! use pricer_pricing::kernel::lsmc::{LSMCRegressor, BasisFunction};
//!
//! let regressor = LSMCRegressor::new(BasisFunction::Laguerre(3));
//! let result = regressor.fit(&exercise_state, &future_values);
//! ```

/// Basis function type for LSMC regression.
///
/// Determines which polynomial basis is used for regression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasisFunction {
    /// Laguerre polynomials (standard in LSMC).
    /// The parameter is the degree (number of terms).
    Laguerre(usize),

    /// Simple power basis (1, x, x², ...).
    /// The parameter is the maximum power.
    Powers(usize),
}

impl BasisFunction {
    /// Returns the number of basis functions.
    #[must_use]
    pub const fn num_terms(&self) -> usize {
        match self {
            Self::Laguerre(n) | Self::Powers(n) => *n,
        }
    }
}

impl Default for BasisFunction {
    fn default() -> Self { Self::Laguerre(3) }
}

/// Result of LSMC regression.
#[derive(Clone, Debug)]
pub struct RegressionResult {
    /// Regression coefficients.
    pub coefficients: Vec<f64>,

    /// R-squared goodness of fit.
    pub r_squared: f64,

    /// Number of paths used in regression (in-the-money paths).
    pub num_samples: usize,
}

impl RegressionResult {
    /// Creates a new regression result.
    #[must_use]
    pub fn new(coefficients: Vec<f64>, r_squared: f64, num_samples: usize) -> Self {
        Self {
            coefficients,
            r_squared,
            num_samples,
        }
    }

    /// Creates an empty result (no regression performed).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            coefficients: Vec::new(),
            r_squared: 0.0,
            num_samples: 0,
        }
    }

    /// Predicts continuation value for a given state variable.
    #[must_use]
    pub fn predict(&self, x: f64, basis: BasisFunction) -> f64 {
        if self.coefficients.is_empty() {
            return 0.0;
        }

        let basis_values = LSMCRegressor::evaluate_basis(x, basis);
        self.coefficients
            .iter()
            .zip(basis_values.iter())
            .map(|(c, b)| c * b)
            .sum()
    }
}

/// LSMC regressor for continuation value estimation.
///
/// Implements the Longstaff-Schwartz algorithm for determining
/// optimal exercise in American/Bermudan options.
#[derive(Clone, Debug)]
pub struct LSMCRegressor {
    /// Basis function type.
    basis: BasisFunction,
}

impl Default for LSMCRegressor {
    fn default() -> Self { Self::new(BasisFunction::default()) }
}

impl LSMCRegressor {
    /// Creates a new LSMC regressor.
    ///
    /// # Arguments
    ///
    /// * `basis` - Basis function type for regression
    #[must_use]
    pub const fn new(basis: BasisFunction) -> Self { Self { basis } }

    /// Returns the basis function type.
    #[must_use]
    pub const fn basis(&self) -> BasisFunction { self.basis }

    /// Fits regression to estimate continuation values.
    ///
    /// # Arguments
    ///
    /// * `state_variables` - State variable values (e.g., short rates) for each
    ///   path
    /// * `future_values` - Discounted future option values for each path
    /// * `in_the_money` - Optional filter for in-the-money paths only
    ///
    /// # Returns
    ///
    /// `RegressionResult` containing fitted coefficients.
    pub fn fit(
        &self,
        state_variables: &[f64],
        future_values: &[f64],
        in_the_money: Option<&[bool]>,
    ) -> RegressionResult {
        let num_paths = state_variables.len();
        assert_eq!(
            num_paths,
            future_values.len(),
            "State variables and future values must have same length"
        );

        // Filter for in-the-money paths if mask provided
        let (x_filtered, y_filtered): (Vec<f64>, Vec<f64>) = if let Some(itm) = in_the_money {
            assert_eq!(
                itm.len(),
                num_paths,
                "In-the-money mask must match path count"
            );
            state_variables
                .iter()
                .zip(future_values.iter())
                .zip(itm.iter())
                .filter(|(_, &is_itm)| is_itm)
                .map(|((x, y), _)| (*x, *y))
                .unzip()
        } else {
            (state_variables.to_vec(), future_values.to_vec())
        };

        let num_samples = x_filtered.len();

        // Need at least as many samples as basis functions
        if num_samples < self.basis.num_terms() + 1 {
            return RegressionResult::empty();
        }

        // Build design matrix X and target vector Y
        let num_terms = self.basis.num_terms();
        let x_matrix = self.build_design_matrix(&x_filtered);
        let y_vec = &y_filtered;

        // Solve normal equations: (X'X)β = X'Y
        let coefficients = self.solve_normal_equations(&x_matrix, y_vec, num_terms);

        // Calculate R-squared
        let r_squared = self.calculate_r_squared(&x_filtered, &y_filtered, &coefficients);

        RegressionResult::new(coefficients, r_squared, num_samples)
    }

    /// Builds the design matrix of basis function values.
    fn build_design_matrix(&self, x_values: &[f64]) -> Vec<Vec<f64>> {
        x_values
            .iter()
            .map(|&x| Self::evaluate_basis(x, self.basis))
            .collect()
    }

    /// Evaluates basis functions at a point.
    #[must_use]
    pub fn evaluate_basis(x: f64, basis: BasisFunction) -> Vec<f64> {
        match basis {
            BasisFunction::Laguerre(n) => Self::laguerre_basis(x, n),
            BasisFunction::Powers(n) => Self::power_basis(x, n),
        }
    }

    /// Evaluates Laguerre polynomial basis.
    ///
    /// L_0(x) = 1
    /// L_1(x) = 1 - x
    /// L_2(x) = 1 - 2x + x²/2
    /// L_n(x) = ((2n-1-x)L_{n-1} - (n-1)L_{n-2}) / n
    fn laguerre_basis(x: f64, n: usize) -> Vec<f64> {
        let mut result = Vec::with_capacity(n);

        if n == 0 {
            return result;
        }

        result.push(1.0); // L_0

        if n == 1 {
            return result;
        }

        result.push(1.0 - x); // L_1

        for i in 2..n {
            let l_prev = result[i - 1];
            let l_prev2 = result[i - 2];
            let l_n = ((2.0 * i as f64 - 1.0 - x) * l_prev - (i as f64 - 1.0) * l_prev2) / i as f64;
            result.push(l_n);
        }

        result
    }

    /// Evaluates power basis (1, x, x², ...).
    fn power_basis(x: f64, n: usize) -> Vec<f64> { (0..n).map(|i| x.powi(i as i32)).collect() }

    /// Solves normal equations using `nalgebra::Cholesky` decomposition.
    fn solve_normal_equations(
        &self,
        x_matrix: &[Vec<f64>],
        y_vec: &[f64],
        num_terms: usize,
    ) -> Vec<f64> {
        use nalgebra::{DMatrix, DVector};

        // Compute X'X (symmetric) as DMatrix
        let mut xtx = DMatrix::zeros(num_terms, num_terms);
        for row in x_matrix {
            for i in 0..num_terms {
                for j in 0..num_terms {
                    xtx[(i, j)] += row[i] * row[j];
                }
            }
        }

        // Compute X'Y as DVector
        let mut xty = DVector::zeros(num_terms);
        for (row, &y) in x_matrix.iter().zip(y_vec.iter()) {
            for i in 0..num_terms {
                xty[i] += row[i] * y;
            }
        }

        // Add regularisation (ridge) for numerical stability
        let ridge = 1e-10;
        for i in 0..num_terms {
            xtx[(i, i)] += ridge;
        }

        // Solve using nalgebra Cholesky decomposition
        match nalgebra::Cholesky::new(xtx) {
            Some(chol) => {
                let solution = chol.solve(&xty);
                solution.iter().copied().collect()
            }
            None => {
                // Matrix not positive definite, return zero coefficients
                vec![0.0; num_terms]
            }
        }
    }

    /// Calculates R-squared goodness of fit.
    fn calculate_r_squared(&self, x_values: &[f64], y_values: &[f64], coefficients: &[f64]) -> f64 {
        if x_values.is_empty() || coefficients.is_empty() {
            return 0.0;
        }

        // Calculate mean of Y
        let y_mean = y_values.iter().sum::<f64>() / y_values.len() as f64;

        // Calculate SS_tot and SS_res
        let mut ss_tot = 0.0;
        let mut ss_res = 0.0;

        for (x, y) in x_values.iter().zip(y_values.iter()) {
            let basis_values = Self::evaluate_basis(*x, self.basis);
            let y_pred: f64 = coefficients
                .iter()
                .zip(basis_values.iter())
                .map(|(c, b)| c * b)
                .sum();

            ss_tot += (y - y_mean).powi(2);
            ss_res += (y - y_pred).powi(2);
        }

        if ss_tot == 0.0 {
            return 1.0;
        }

        1.0 - ss_res / ss_tot
    }

    /// Determines optimal exercise decisions.
    ///
    /// Compares continuation value (from regression) with intrinsic value.
    ///
    /// # Arguments
    ///
    /// * `state_variables` - Current state for each path
    /// * `intrinsic_values` - Exercise value for each path
    /// * `regression_result` - Fitted regression coefficients
    ///
    /// # Returns
    ///
    /// Vector of booleans: true if exercise is optimal.
    #[must_use]
    pub fn determine_exercise(
        &self,
        state_variables: &[f64],
        intrinsic_values: &[f64],
        regression_result: &RegressionResult,
    ) -> Vec<bool> {
        state_variables
            .iter()
            .zip(intrinsic_values.iter())
            .map(|(&x, &intrinsic)| {
                let continuation = regression_result.predict(x, self.basis);
                intrinsic > continuation && intrinsic > 0.0
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // BasisFunction Tests
    // =========================================================================

    #[test]
    fn test_basis_function_num_terms() {
        assert_eq!(BasisFunction::Laguerre(3).num_terms(), 3);
        assert_eq!(BasisFunction::Powers(5).num_terms(), 5);
    }

    #[test]
    fn test_basis_function_default() {
        let default = BasisFunction::default();
        assert_eq!(default, BasisFunction::Laguerre(3));
    }

    // =========================================================================
    // RegressionResult Tests
    // =========================================================================

    #[test]
    fn test_regression_result_new() {
        let result = RegressionResult::new(vec![1.0, 2.0], 0.95, 100);
        assert_eq!(result.coefficients.len(), 2);
        assert!((result.r_squared - 0.95).abs() < 1e-10);
        assert_eq!(result.num_samples, 100);
    }

    #[test]
    fn test_regression_result_empty() {
        let result = RegressionResult::empty();
        assert!(result.coefficients.is_empty());
        assert!(result.r_squared.abs() < 1e-10);
        assert_eq!(result.num_samples, 0);
    }

    #[test]
    fn test_regression_result_predict() {
        // Linear: y = 1 + 2x
        let result = RegressionResult::new(vec![1.0, 2.0], 1.0, 10);
        let prediction = result.predict(0.5, BasisFunction::Powers(2));
        assert!((prediction - 2.0).abs() < 1e-10); // 1 + 2*0.5 = 2
    }

    // =========================================================================
    // LSMCRegressor Tests
    // =========================================================================

    #[test]
    fn test_lsmc_regressor_new() {
        let regressor = LSMCRegressor::new(BasisFunction::Laguerre(4));
        assert_eq!(regressor.basis(), BasisFunction::Laguerre(4));
    }

    #[test]
    fn test_lsmc_regressor_default() {
        let regressor = LSMCRegressor::default();
        assert_eq!(regressor.basis(), BasisFunction::Laguerre(3));
    }

    #[test]
    fn test_power_basis_evaluation() {
        let basis = LSMCRegressor::evaluate_basis(2.0, BasisFunction::Powers(4));
        assert_eq!(basis.len(), 4);
        assert!((basis[0] - 1.0).abs() < 1e-10); // x^0 = 1
        assert!((basis[1] - 2.0).abs() < 1e-10); // x^1 = 2
        assert!((basis[2] - 4.0).abs() < 1e-10); // x^2 = 4
        assert!((basis[3] - 8.0).abs() < 1e-10); // x^3 = 8
    }

    #[test]
    fn test_laguerre_basis_evaluation() {
        let x = 0.5;
        let basis = LSMCRegressor::evaluate_basis(x, BasisFunction::Laguerre(3));
        assert_eq!(basis.len(), 3);

        // L_0(x) = 1
        assert!((basis[0] - 1.0).abs() < 1e-10);

        // L_1(x) = 1 - x = 0.5
        assert!((basis[1] - 0.5).abs() < 1e-10);

        // L_2(x) = 1 - 2x + x²/2 = 1 - 1 + 0.125 = 0.125
        assert!((basis[2] - 0.125).abs() < 1e-10);
    }

    #[test]
    fn test_fit_linear_regression() {
        let regressor = LSMCRegressor::new(BasisFunction::Powers(2));

        // Data: y = 1 + 2x (exactly linear)
        let x: Vec<f64> = (0..100).map(|i| i as f64 / 100.0).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 2.0 * xi).collect();

        let result = regressor.fit(&x, &y, None);

        assert!(!result.coefficients.is_empty());
        // Should recover coefficients close to [1, 2]
        assert!((result.coefficients[0] - 1.0).abs() < 0.01);
        assert!((result.coefficients[1] - 2.0).abs() < 0.01);
        // R-squared should be very close to 1
        assert!(result.r_squared > 0.99);
    }

    #[test]
    fn test_fit_with_noise() {
        let regressor = LSMCRegressor::new(BasisFunction::Powers(2));

        // Data: y = 1 + 2x + noise
        let x: Vec<f64> = (0..100).map(|i| i as f64 / 100.0).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &xi)| 1.0 + 2.0 * xi + 0.01 * ((i % 5) as f64 - 2.0))
            .collect();

        let result = regressor.fit(&x, &y, None);

        assert!(!result.coefficients.is_empty());
        // Should approximately recover coefficients
        assert!((result.coefficients[0] - 1.0).abs() < 0.1);
        assert!((result.coefficients[1] - 2.0).abs() < 0.1);
        // R-squared should be high but not perfect
        assert!(result.r_squared > 0.9);
    }

    #[test]
    fn test_fit_with_itm_filter() {
        let regressor = LSMCRegressor::new(BasisFunction::Powers(2));

        let x: Vec<f64> = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let y: Vec<f64> = vec![1.2, 1.4, 1.6, 1.8, 2.0];
        let itm = vec![true, true, true, false, false]; // Only use first 3

        let result = regressor.fit(&x, &y, Some(&itm));

        assert_eq!(result.num_samples, 3);
    }

    #[test]
    fn test_fit_insufficient_samples() {
        let regressor = LSMCRegressor::new(BasisFunction::Powers(5));

        // Only 3 samples, but need 5+ for degree-5 polynomial
        let x = vec![0.1, 0.2, 0.3];
        let y = vec![1.0, 2.0, 3.0];

        let result = regressor.fit(&x, &y, None);

        // Should return empty result
        assert!(result.coefficients.is_empty());
    }

    #[test]
    fn test_determine_exercise() {
        let regressor = LSMCRegressor::new(BasisFunction::Powers(2));

        // Regression result: continuation value = 1 + x
        let regression = RegressionResult::new(vec![1.0, 1.0], 1.0, 100);

        let states = vec![0.0, 0.5, 1.0];
        let intrinsic = vec![0.5, 2.0, 1.5]; // > or < continuation

        // Continuation values: 1, 1.5, 2
        // Intrinsic: 0.5, 2.0, 1.5
        // Exercise if intrinsic > continuation AND intrinsic > 0

        let decisions = regressor.determine_exercise(&states, &intrinsic, &regression);

        assert!(!decisions[0]); // 0.5 < 1.0 → don't exercise
        assert!(decisions[1]); // 2.0 > 1.5 → exercise
        assert!(!decisions[2]); // 1.5 < 2.0 → don't exercise
    }

    #[test]
    fn test_laguerre_vs_powers() {
        // Both should work for simple polynomial data
        let x: Vec<f64> = (0..50).map(|i| i as f64 / 50.0).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + xi + 0.5 * xi * xi).collect();

        let laguerre_reg = LSMCRegressor::new(BasisFunction::Laguerre(3));
        let powers_reg = LSMCRegressor::new(BasisFunction::Powers(3));

        let lag_result = laguerre_reg.fit(&x, &y, None);
        let pow_result = powers_reg.fit(&x, &y, None);

        // Both should have good fit
        assert!(lag_result.r_squared > 0.9);
        assert!(pow_result.r_squared > 0.9);
    }
}

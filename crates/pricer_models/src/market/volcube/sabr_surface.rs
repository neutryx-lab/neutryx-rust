//! SABR パラメータ平面。
//!
//! # Requirements: 1.2, 1.5
//!
//! Expiry-Tenor格子上でSABRパラメータ（alpha, rho, nu）を
//! Bilinear補間で取得する機能を提供する。

use num_traits::Float;
use pricer_core::{math::interpolators::BilinearInterpolator, types::InterpolationError};

use super::types::SabrParams;

/// SABR パラメータ平面。
///
/// # Requirements: 1.2, 1.5, 4.1
///
/// Expiry-Tenor格子上でカリブレーション済みSABRパラメータを保持し、
/// 任意の(expiry, tenor)点でBilinear補間によりパラメータを取得する。
///
/// # 構造
///
/// 各パラメータ（alpha, rho, nu）を独立した2D補間器で管理。
/// betaは設定で固定されているため補間不要。
#[derive(Debug, Clone)]
pub struct SabrParameterSurface<T: Float> {
    /// Expiry軸の格子点（昇順）。
    expiries: Vec<T>,
    /// Tenor軸の格子点（昇順）。
    tenors: Vec<T>,
    /// 固定beta値。
    beta: T,
    /// Alpha補間器。
    alpha_interpolator: BilinearInterpolator<T>,
    /// Rho補間器。
    rho_interpolator: BilinearInterpolator<T>,
    /// Nu補間器。
    nu_interpolator: BilinearInterpolator<T>,
}

impl<T: Float> SabrParameterSurface<T> {
    /// 格子点とパラメータからSABR平面を構築。
    ///
    /// # Arguments
    ///
    /// * `expiries` - Expiry格子点（昇順、年単位）
    /// * `tenors` - Tenor格子点（昇順、年単位）
    /// * `params` - パラメータ行列 `params\[i\]\[j\]` = (expiry\[i\],
    ///   tenor\[j\])でのSABRパラメータ
    /// * `beta` - 固定beta値
    ///
    /// # Errors
    ///
    /// 格子点が不足している場合やデータに不整合がある場合にエラーを返す。
    pub fn new(
        expiries: Vec<T>,
        tenors: Vec<T>,
        params: &[Vec<SabrParams<T>>],
        beta: T,
    ) -> Result<Self, InterpolationError> {
        let n_expiries = expiries.len();
        let n_tenors = tenors.len();

        if n_expiries < 2 {
            return Err(InterpolationError::InsufficientData {
                got: n_expiries,
                need: 2,
            });
        }
        if n_tenors < 2 {
            return Err(InterpolationError::InsufficientData {
                got: n_tenors,
                need: 2,
            });
        }
        if params.len() != n_expiries {
            return Err(InterpolationError::InvalidInput(format!(
                "Expected {} expiry rows, got {}",
                n_expiries,
                params.len()
            )));
        }
        for (i, row) in params.iter().enumerate() {
            if row.len() != n_tenors {
                return Err(InterpolationError::InvalidInput(format!(
                    "Row {} has {} tenors, expected {}",
                    i,
                    row.len(),
                    n_tenors
                )));
            }
        }

        // パラメータ行列を個別の2D配列に分解
        let mut alpha_grid: Vec<Vec<T>> = Vec::with_capacity(n_expiries);
        let mut rho_grid: Vec<Vec<T>> = Vec::with_capacity(n_expiries);
        let mut nu_grid: Vec<Vec<T>> = Vec::with_capacity(n_expiries);

        for row in params {
            let alpha_row: Vec<T> = row.iter().map(|p| p.alpha).collect();
            let rho_row: Vec<T> = row.iter().map(|p| p.rho).collect();
            let nu_row: Vec<T> = row.iter().map(|p| p.nu).collect();

            alpha_grid.push(alpha_row);
            rho_grid.push(rho_row);
            nu_grid.push(nu_row);
        }

        // Vec<Vec<T>> を &[&[T]] に変換するためのスライス参照
        let alpha_refs: Vec<&[T]> = alpha_grid.iter().map(|v| v.as_slice()).collect();
        let rho_refs: Vec<&[T]> = rho_grid.iter().map(|v| v.as_slice()).collect();
        let nu_refs: Vec<&[T]> = nu_grid.iter().map(|v| v.as_slice()).collect();

        let alpha_interpolator = BilinearInterpolator::new(&expiries, &tenors, &alpha_refs)?;
        let rho_interpolator = BilinearInterpolator::new(&expiries, &tenors, &rho_refs)?;
        let nu_interpolator = BilinearInterpolator::new(&expiries, &tenors, &nu_refs)?;

        Ok(Self {
            expiries,
            tenors,
            beta,
            alpha_interpolator,
            rho_interpolator,
            nu_interpolator,
        })
    }

    /// 指定した(expiry, tenor)でのSABRパラメータを補間取得。
    ///
    /// # Arguments
    ///
    /// * `expiry` - クエリexpiry（年単位）
    /// * `tenor` - クエリtenor（年単位）
    ///
    /// # Returns
    ///
    /// 補間されたSABRパラメータ。ドメイン外の場合はエラー。
    pub fn interpolate(&self, expiry: T, tenor: T) -> Result<SabrParams<T>, InterpolationError> {
        let alpha = self.alpha_interpolator.interpolate(expiry, tenor)?;
        let rho = self.rho_interpolator.interpolate(expiry, tenor)?;
        let nu = self.nu_interpolator.interpolate(expiry, tenor)?;

        Ok(SabrParams {
            alpha,
            beta: self.beta,
            rho,
            nu,
        })
    }

    /// Expiry軸のドメインを取得。
    pub fn expiry_domain(&self) -> (T, T) {
        if self.expiries.is_empty() {
            return (T::zero(), T::zero());
        }
        (self.expiries[0], *self.expiries.last().unwrap())
    }

    /// Tenor軸のドメインを取得。
    pub fn tenor_domain(&self) -> (T, T) {
        if self.tenors.is_empty() {
            return (T::zero(), T::zero());
        }
        (self.tenors[0], *self.tenors.last().unwrap())
    }

    /// 固定beta値を取得。
    pub fn beta(&self) -> T { self.beta }

    /// Expiry格子点を取得。
    pub fn expiries(&self) -> &[T] { &self.expiries }

    /// Tenor格子点を取得。
    pub fn tenors(&self) -> &[T] { &self.tenors }

    /// 指定したExpiryでのAlpha値を補間取得（tenor固定）。
    pub fn alpha_at(&self, expiry: T, tenor: T) -> Result<T, InterpolationError> {
        self.alpha_interpolator.interpolate(expiry, tenor)
    }

    /// 指定したExpiryでのRho値を補間取得（tenor固定）。
    pub fn rho_at(&self, expiry: T, tenor: T) -> Result<T, InterpolationError> {
        self.rho_interpolator.interpolate(expiry, tenor)
    }

    /// 指定したExpiryでのNu値を補間取得（tenor固定）。
    pub fn nu_at(&self, expiry: T, tenor: T) -> Result<T, InterpolationError> {
        self.nu_interpolator.interpolate(expiry, tenor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_surface() -> SabrParameterSurface<f64> {
        // 2x2の最小グリッド
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0, 5.0];
        let beta = 0.5;

        // パラメータ行列
        let params = vec![
            // expiry = 0.5
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),   // tenor = 2
                SabrParams::new(0.05, beta, -0.25, 0.35), // tenor = 5
            ],
            // expiry = 1.0
            vec![
                SabrParams::new(0.045, beta, -0.35, 0.45), // tenor = 2
                SabrParams::new(0.055, beta, -0.2, 0.3),   // tenor = 5
            ],
        ];

        SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap()
    }

    #[test]
    fn test_sabr_surface_new() {
        let surface = create_test_surface();
        assert_eq!(surface.expiries().len(), 2);
        assert_eq!(surface.tenors().len(), 2);
        assert_eq!(surface.beta(), 0.5);
    }

    #[test]
    fn test_sabr_surface_interpolate_at_grid_point() {
        let surface = create_test_surface();

        // 格子点(0.5, 2.0)でのパラメータ
        let params = surface.interpolate(0.5, 2.0).unwrap();
        assert!((params.alpha - 0.04).abs() < 1e-10);
        assert!((params.rho - (-0.3)).abs() < 1e-10);
        assert!((params.nu - 0.4).abs() < 1e-10);
        assert_eq!(params.beta, 0.5);
    }

    #[test]
    fn test_sabr_surface_interpolate_midpoint() {
        let surface = create_test_surface();

        // グリッド中央点(0.75, 3.5)での補間
        let params = surface.interpolate(0.75, 3.5).unwrap();

        // alpha: (0.04+0.05+0.045+0.055)/4 = 0.0475
        // bilinear補間なので正確な中央値ではないが範囲内
        assert!(params.alpha > 0.04 && params.alpha < 0.055);
        assert!(params.rho > -0.35 && params.rho < -0.2);
        assert!(params.nu > 0.3 && params.nu < 0.45);
    }

    #[test]
    fn test_sabr_surface_domain() {
        let surface = create_test_surface();

        let (exp_min, exp_max) = surface.expiry_domain();
        assert_eq!(exp_min, 0.5);
        assert_eq!(exp_max, 1.0);

        let (ten_min, ten_max) = surface.tenor_domain();
        assert_eq!(ten_min, 2.0);
        assert_eq!(ten_max, 5.0);
    }

    #[test]
    fn test_sabr_surface_insufficient_expiries() {
        let expiries = vec![0.5]; // 1つだけ
        let tenors = vec![2.0, 5.0];
        let params = vec![vec![
            SabrParams::new(0.04, 0.5, -0.3, 0.4),
            SabrParams::new(0.05, 0.5, -0.25, 0.35),
        ]];

        let result = SabrParameterSurface::new(expiries, tenors, &params, 0.5);
        assert!(result.is_err());
        if let Err(InterpolationError::InsufficientData { got, need }) = result {
            assert_eq!(got, 1);
            assert_eq!(need, 2);
        }
    }

    #[test]
    fn test_sabr_surface_insufficient_tenors() {
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0]; // 1つだけ
        let params = vec![
            vec![SabrParams::new(0.04, 0.5, -0.3, 0.4)],
            vec![SabrParams::new(0.045, 0.5, -0.35, 0.45)],
        ];

        let result = SabrParameterSurface::new(expiries, tenors, &params, 0.5);
        assert!(result.is_err());
        if let Err(InterpolationError::InsufficientData { got, need }) = result {
            assert_eq!(got, 1);
            assert_eq!(need, 2);
        }
    }

    #[test]
    fn test_sabr_surface_row_mismatch() {
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0, 5.0];
        let params = vec![
            vec![SabrParams::new(0.04, 0.5, -0.3, 0.4)], // 1 tenor instead of 2
            vec![
                SabrParams::new(0.045, 0.5, -0.35, 0.45),
                SabrParams::new(0.055, 0.5, -0.2, 0.3),
            ],
        ];

        let result = SabrParameterSurface::new(expiries, tenors, &params, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_sabr_surface_individual_params() {
        let surface = create_test_surface();

        let alpha = surface.alpha_at(0.5, 2.0).unwrap();
        assert!((alpha - 0.04).abs() < 1e-10);

        let rho = surface.rho_at(0.5, 2.0).unwrap();
        assert!((rho - (-0.3)).abs() < 1e-10);

        let nu = surface.nu_at(0.5, 2.0).unwrap();
        assert!((nu - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_surface_3x3_grid() {
        // より大きなグリッドでのテスト
        let expiries = vec![0.5, 1.0, 2.0];
        let tenors = vec![2.0, 5.0, 10.0];
        let beta = 0.5;

        let params = vec![
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),
                SabrParams::new(0.05, beta, -0.25, 0.35),
                SabrParams::new(0.06, beta, -0.2, 0.3),
            ],
            vec![
                SabrParams::new(0.045, beta, -0.35, 0.45),
                SabrParams::new(0.055, beta, -0.3, 0.4),
                SabrParams::new(0.065, beta, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.05, beta, -0.4, 0.5),
                SabrParams::new(0.06, beta, -0.35, 0.45),
                SabrParams::new(0.07, beta, -0.3, 0.4),
            ],
        ];

        let surface = SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap();

        // 中央点で補間
        let mid_params = surface.interpolate(1.0, 5.0).unwrap();
        assert!((mid_params.alpha - 0.055).abs() < 1e-10); // 格子点
    }

    #[test]
    fn test_sabr_surface_clone() {
        let surface = create_test_surface();
        let cloned = surface.clone();

        assert_eq!(surface.expiries(), cloned.expiries());
        assert_eq!(surface.tenors(), cloned.tenors());
        assert_eq!(surface.beta(), cloned.beta());
    }
}

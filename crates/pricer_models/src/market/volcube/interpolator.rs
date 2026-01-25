//! VolCube補間器フレームワーク。
//!
//! # Requirements: 3.3, 3.6
//!
//! このモジュールはVolCube内での補間ストラテジーを提供する。
//! 各軸（Expiry, Tenor, Strike）での補間方式を定義する。

use num_traits::Float;

/// VolCube補間器トレイト。
///
/// # Requirements: 3.3, 3.6
///
/// 1次元補間を抽象化するトレイト。Expiry、Tenor、Strike軸で
/// それぞれ異なる補間方式を選択可能にする。
pub trait VolCubeInterpolator<T: Float>: Send + Sync {
    /// 1次元補間を実行。
    ///
    /// # Arguments
    ///
    /// * `x` - 補間対象の座標
    /// * `grid` - 格子点の座標リスト（昇順）
    /// * `values` - 格子点での値リスト
    ///
    /// # Returns
    ///
    /// 補間された値、またはエラー。
    fn interpolate(&self, x: T, grid: &[T], values: &[T]) -> Result<T, InterpolationError>;

    /// 補間器の名前。
    fn name(&self) -> &'static str;

    /// この補間器が外挿をサポートするか。
    fn supports_extrapolation(&self) -> bool;
}

/// 補間エラー。
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationError {
    /// 格子点が不足している。
    InsufficientPoints {
        /// 実際に得られた格子点数。
        got: usize,
        /// 必要な最小格子点数。
        need: usize,
    },
    /// 格子と値のサイズが一致しない。
    SizeMismatch {
        /// 格子点の数。
        grid_len: usize,
        /// 値の数。
        values_len: usize,
    },
    /// 範囲外（外挿禁止時）。
    OutOfBounds {
        /// 補間対象のx座標。
        x: f64,
        /// 格子の最小値。
        min: f64,
        /// 格子の最大値。
        max: f64,
    },
    /// 格子が単調でない。
    NonMonotonicGrid,
}

impl std::fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientPoints { got, need } => {
                write!(f, "Insufficient grid points: got {}, need {}", got, need)
            }
            Self::SizeMismatch {
                grid_len,
                values_len,
            } => {
                write!(
                    f,
                    "Size mismatch: grid has {} points, values has {} points",
                    grid_len, values_len
                )
            }
            Self::OutOfBounds { x, min, max } => {
                write!(f, "Out of bounds: x={} not in [{}, {}]", x, min, max)
            }
            Self::NonMonotonicGrid => {
                write!(f, "Grid points are not monotonically increasing")
            }
        }
    }
}

impl std::error::Error for InterpolationError {}

/// Flat補間器（最近傍補間）。
///
/// # Requirements: 3.3, 3.6
///
/// 最も近い格子点の値を返すシンプルな補間方式。
/// 外挿時は境界値を返す。
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatInterpolator;

impl FlatInterpolator {
    /// 新しいFlat補間器を作成。
    pub fn new() -> Self {
        Self
    }

    /// 格子点から最も近いインデックスを見つける。
    fn find_nearest_index<T: Float>(&self, x: T, grid: &[T]) -> usize {
        if grid.is_empty() {
            return 0;
        }

        let n = grid.len();

        // 範囲外の場合は境界を返す
        if x <= grid[0] {
            return 0;
        }
        if x >= grid[n - 1] {
            return n - 1;
        }

        // 二分探索でブラケットを見つける
        let mut lo = 0;
        let mut hi = n - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if grid[mid] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        // lo と hi のうち、x に近い方を選択
        let two = T::from(2.0).unwrap();
        let mid_point = (grid[lo] + grid[hi]) / two;
        if x <= mid_point {
            lo
        } else {
            hi
        }
    }
}

impl<T: Float + Send + Sync> VolCubeInterpolator<T> for FlatInterpolator {
    fn interpolate(&self, x: T, grid: &[T], values: &[T]) -> Result<T, InterpolationError> {
        // 入力検証
        if grid.is_empty() {
            return Err(InterpolationError::InsufficientPoints { got: 0, need: 1 });
        }
        if grid.len() != values.len() {
            return Err(InterpolationError::SizeMismatch {
                grid_len: grid.len(),
                values_len: values.len(),
            });
        }

        // 単一点の場合はその値を返す
        if grid.len() == 1 {
            return Ok(values[0]);
        }

        // 最も近い格子点のインデックスを見つける
        let idx = self.find_nearest_index(x, grid);
        Ok(values[idx])
    }

    fn name(&self) -> &'static str {
        "Flat"
    }

    fn supports_extrapolation(&self) -> bool {
        true // Flat補間は自然に外挿をサポート（境界値を返す）
    }
}

/// Linear補間器（線形補間）。
///
/// # Requirements: 3.3, 3.6
///
/// 隣接する2つの格子点間で線形補間を行う。
/// 外挿時は境界勾配を維持して線形外挿。
#[derive(Debug, Clone, Copy, Default)]
pub struct LinearInterpolator;

impl LinearInterpolator {
    /// 新しいLinear補間器を作成。
    pub fn new() -> Self {
        Self
    }

    /// 格子点でxを挟むブラケットインデックスと補間係数を見つける。
    ///
    /// # Returns
    ///
    /// (lo, hi, t) where lo < hi and t ∈ [0, 1] for interpolation.
    /// 外挿時は t < 0 または t > 1 になる可能性がある。
    fn find_bracket<T: Float>(&self, x: T, grid: &[T]) -> (usize, usize, T) {
        let n = grid.len();
        if n == 1 {
            return (0, 0, T::zero());
        }

        let x_min = grid[0];
        let x_max = grid[n - 1];

        // 左側外挿
        if x <= x_min {
            let dx = grid[1] - grid[0];
            if dx <= T::zero() {
                return (0, 1, T::zero());
            }
            let t = (x - x_min) / dx;
            return (0, 1, t);
        }

        // 右側外挿
        if x >= x_max {
            let dx = grid[n - 1] - grid[n - 2];
            if dx <= T::zero() {
                return (n - 2, n - 1, T::one());
            }
            let t = T::one() + (x - x_max) / dx;
            return (n - 2, n - 1, t);
        }

        // 内部: 二分探索でブラケットを見つける
        let mut lo = 0;
        let mut hi = n - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if grid[mid] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let dx = grid[hi] - grid[lo];
        let t = if dx <= T::zero() {
            T::zero()
        } else {
            (x - grid[lo]) / dx
        };

        (lo, hi, t)
    }
}

impl<T: Float + Send + Sync> VolCubeInterpolator<T> for LinearInterpolator {
    fn interpolate(&self, x: T, grid: &[T], values: &[T]) -> Result<T, InterpolationError> {
        // 入力検証
        if grid.len() < 2 {
            return Err(InterpolationError::InsufficientPoints {
                got: grid.len(),
                need: 2,
            });
        }
        if grid.len() != values.len() {
            return Err(InterpolationError::SizeMismatch {
                grid_len: grid.len(),
                values_len: values.len(),
            });
        }

        // ブラケットと補間係数を取得
        let (lo, hi, t) = self.find_bracket(x, grid);

        // 線形補間: v = v_lo * (1 - t) + v_hi * t
        let v_lo = values[lo];
        let v_hi = values[hi];
        let one = T::one();
        let result = v_lo * (one - t) + v_hi * t;

        Ok(result)
    }

    fn name(&self) -> &'static str {
        "Linear"
    }

    fn supports_extrapolation(&self) -> bool {
        true // 線形外挿をサポート
    }
}

/// 補間方式の静的ディスパッチ用enum。
///
/// # Requirements: 3.3, 3.6
///
/// 動的ディスパッチのオーバーヘッドを避けるため、
/// enum-based static dispatchパターンを使用。
#[derive(Debug, Clone, Copy, Default)]
pub enum Interpolator {
    /// Flat補間（最近傍）。
    #[default]
    Flat,
    /// 線形補間。
    Linear,
}

impl<T: Float + Send + Sync> VolCubeInterpolator<T> for Interpolator {
    fn interpolate(&self, x: T, grid: &[T], values: &[T]) -> Result<T, InterpolationError> {
        match self {
            Interpolator::Flat => FlatInterpolator.interpolate(x, grid, values),
            Interpolator::Linear => LinearInterpolator.interpolate(x, grid, values),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Interpolator::Flat => "Flat",
            Interpolator::Linear => "Linear",
        }
    }

    fn supports_extrapolation(&self) -> bool {
        match self {
            Interpolator::Flat => true,
            Interpolator::Linear => true, // 線形外挿をサポート
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InterpolationError Tests
    // =========================================================================

    #[test]
    fn test_interpolation_error_display_insufficient_points() {
        let err = InterpolationError::InsufficientPoints { got: 0, need: 2 };
        let msg = format!("{}", err);
        assert!(msg.contains("Insufficient"));
        assert!(msg.contains("0"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn test_interpolation_error_display_size_mismatch() {
        let err = InterpolationError::SizeMismatch {
            grid_len: 5,
            values_len: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("mismatch"));
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_interpolation_error_display_out_of_bounds() {
        let err = InterpolationError::OutOfBounds {
            x: 1.5,
            min: 0.0,
            max: 1.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("bounds"));
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_interpolation_error_display_non_monotonic() {
        let err = InterpolationError::NonMonotonicGrid;
        let msg = format!("{}", err);
        assert!(msg.contains("monotonic"));
    }

    // =========================================================================
    // FlatInterpolator Tests
    // =========================================================================

    #[test]
    fn test_flat_interpolator_new() {
        let interp = FlatInterpolator::new();
        assert_eq!(
            <FlatInterpolator as VolCubeInterpolator<f64>>::name(&interp),
            "Flat"
        );
        assert!(<FlatInterpolator as VolCubeInterpolator<f64>>::supports_extrapolation(&interp));
    }

    #[test]
    fn test_flat_interpolator_single_point() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64];
        let values = vec![0.5_f64];

        let result = interp.interpolate(0.5, &grid, &values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.5);

        let result = interp.interpolate(1.5, &grid, &values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.5);
    }

    #[test]
    fn test_flat_interpolator_empty_grid() {
        let interp = FlatInterpolator::new();
        let grid: Vec<f64> = vec![];
        let values: Vec<f64> = vec![];

        let result = interp.interpolate(0.5, &grid, &values);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::InsufficientPoints { got: 0, need: 1 })
        ));
    }

    #[test]
    fn test_flat_interpolator_size_mismatch() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![0.5_f64, 0.6];

        let result = interp.interpolate(1.5, &grid, &values);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::SizeMismatch {
                grid_len: 3,
                values_len: 2
            })
        ));
    }

    #[test]
    fn test_flat_interpolator_at_grid_points() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0, 4.0];
        let values = vec![10.0_f64, 20.0, 30.0, 40.0];

        // 格子点上の値は正確に返される
        for (i, &g) in grid.iter().enumerate() {
            let result = interp.interpolate(g, &grid, &values).unwrap();
            assert_eq!(result, values[i]);
        }
    }

    #[test]
    fn test_flat_interpolator_between_grid_points() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0, 4.0];
        let values = vec![10.0_f64, 20.0, 30.0, 40.0];

        // 1.25 は 1.0 に近いので 10.0 を返す
        let result = interp.interpolate(1.25, &grid, &values).unwrap();
        assert_eq!(result, 10.0);

        // 1.75 は 2.0 に近いので 20.0 を返す
        let result = interp.interpolate(1.75, &grid, &values).unwrap();
        assert_eq!(result, 20.0);

        // 2.5 は 2.0 と 3.0 の中間なので、2.0 の値 (20.0) を返す（等距離時は左側）
        let result = interp.interpolate(2.5, &grid, &values).unwrap();
        assert_eq!(result, 20.0);
    }

    #[test]
    fn test_flat_interpolator_extrapolation_left() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 左側外挿: 左端の値を返す
        let result = interp.interpolate(0.0, &grid, &values).unwrap();
        assert_eq!(result, 10.0);

        let result = interp.interpolate(-1.0, &grid, &values).unwrap();
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_flat_interpolator_extrapolation_right() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 右側外挿: 右端の値を返す
        let result = interp.interpolate(4.0, &grid, &values).unwrap();
        assert_eq!(result, 30.0);

        let result = interp.interpolate(10.0, &grid, &values).unwrap();
        assert_eq!(result, 30.0);
    }

    #[test]
    fn test_flat_interpolator_two_points() {
        let interp = FlatInterpolator::new();
        let grid = vec![0.0_f64, 1.0];
        let values = vec![100.0_f64, 200.0];

        // 左端
        assert_eq!(interp.interpolate(0.0, &grid, &values).unwrap(), 100.0);
        // 右端
        assert_eq!(interp.interpolate(1.0, &grid, &values).unwrap(), 200.0);
        // 中点の左側
        assert_eq!(interp.interpolate(0.4, &grid, &values).unwrap(), 100.0);
        // 中点
        assert_eq!(interp.interpolate(0.5, &grid, &values).unwrap(), 100.0);
        // 中点の右側
        assert_eq!(interp.interpolate(0.6, &grid, &values).unwrap(), 200.0);
    }

    // =========================================================================
    // LinearInterpolator Tests
    // =========================================================================

    #[test]
    fn test_linear_interpolator_new() {
        let interp = LinearInterpolator::new();
        assert_eq!(
            <LinearInterpolator as VolCubeInterpolator<f64>>::name(&interp),
            "Linear"
        );
        assert!(<LinearInterpolator as VolCubeInterpolator<f64>>::supports_extrapolation(&interp));
    }

    #[test]
    fn test_linear_interpolator_two_points() {
        let interp = LinearInterpolator::new();
        let grid = vec![0.0_f64, 1.0];
        let values = vec![0.0_f64, 100.0];

        // 左端
        assert_eq!(interp.interpolate(0.0, &grid, &values).unwrap(), 0.0);
        // 右端
        assert_eq!(interp.interpolate(1.0, &grid, &values).unwrap(), 100.0);
        // 中点
        let result = interp.interpolate(0.5, &grid, &values).unwrap();
        assert!((result - 50.0).abs() < 1e-10);
        // 25%点
        let result = interp.interpolate(0.25, &grid, &values).unwrap();
        assert!((result - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolator_at_grid_points() {
        let interp = LinearInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0, 4.0];
        let values = vec![10.0_f64, 20.0, 30.0, 40.0];

        // 格子点上の値は正確に返される
        for (i, &g) in grid.iter().enumerate() {
            let result = interp.interpolate(g, &grid, &values).unwrap();
            assert!((result - values[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_linear_interpolator_between_grid_points() {
        let interp = LinearInterpolator::new();
        let grid = vec![0.0_f64, 1.0, 2.0, 3.0];
        let values = vec![0.0_f64, 10.0, 20.0, 30.0];

        // 0.5: 0 + 0.5 * (10 - 0) = 5
        let result = interp.interpolate(0.5, &grid, &values).unwrap();
        assert!((result - 5.0).abs() < 1e-10);

        // 1.25: 10 + 0.25 * (20 - 10) = 12.5
        let result = interp.interpolate(1.25, &grid, &values).unwrap();
        assert!((result - 12.5).abs() < 1e-10);

        // 2.75: 20 + 0.75 * (30 - 20) = 27.5
        let result = interp.interpolate(2.75, &grid, &values).unwrap();
        assert!((result - 27.5).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolator_extrapolation_left() {
        let interp = LinearInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 左側外挿: 勾配 = (20-10)/(2-1) = 10
        // x=0: 10 + (0-1)*10 = 0
        let result = interp.interpolate(0.0, &grid, &values).unwrap();
        assert!((result - 0.0).abs() < 1e-10);

        // x=-1: 10 + (-1-1)*10 = -10
        let result = interp.interpolate(-1.0, &grid, &values).unwrap();
        assert!((result - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolator_extrapolation_right() {
        let interp = LinearInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 右側外挿: 勾配 = (30-20)/(3-2) = 10
        // x=4: 30 + (4-3)*10 = 40
        let result = interp.interpolate(4.0, &grid, &values).unwrap();
        assert!((result - 40.0).abs() < 1e-10);

        // x=5: 30 + (5-3)*10 = 50
        let result = interp.interpolate(5.0, &grid, &values).unwrap();
        assert!((result - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolator_insufficient_points() {
        let interp = LinearInterpolator::new();

        // 空の格子
        let grid: Vec<f64> = vec![];
        let values: Vec<f64> = vec![];
        let result = interp.interpolate(0.5, &grid, &values);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::InsufficientPoints { got: 0, need: 2 })
        ));

        // 単一点（線形補間には不足）
        let grid = vec![1.0_f64];
        let values = vec![10.0_f64];
        let result = interp.interpolate(0.5, &grid, &values);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::InsufficientPoints { got: 1, need: 2 })
        ));
    }

    #[test]
    fn test_linear_interpolator_size_mismatch() {
        let interp = LinearInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0];

        let result = interp.interpolate(1.5, &grid, &values);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::SizeMismatch {
                grid_len: 3,
                values_len: 2
            })
        ));
    }

    #[test]
    fn test_linear_interpolator_non_uniform_grid() {
        let interp = LinearInterpolator::new();
        // 非一様格子
        let grid = vec![0.0_f64, 1.0, 4.0, 10.0];
        let values = vec![0.0_f64, 10.0, 40.0, 100.0];

        // [1.0, 4.0] 区間での補間: t = (2.5 - 1.0) / (4.0 - 1.0) = 0.5
        // value = 10 + 0.5 * (40 - 10) = 25
        let result = interp.interpolate(2.5, &grid, &values).unwrap();
        assert!((result - 25.0).abs() < 1e-10);

        // [4.0, 10.0] 区間での補間: t = (7.0 - 4.0) / (10.0 - 4.0) = 0.5
        // value = 40 + 0.5 * (100 - 40) = 70
        let result = interp.interpolate(7.0, &grid, &values).unwrap();
        assert!((result - 70.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolator_large_grid() {
        let interp = LinearInterpolator::new();
        let n = 1000;
        let grid: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let values: Vec<f64> = (0..n).map(|i| (i * 10) as f64).collect();

        // 中央付近での補間
        // x = 500.3: 5000 + 0.3 * (5010 - 5000) = 5003
        let result = interp.interpolate(500.3, &grid, &values).unwrap();
        assert!((result - 5003.0).abs() < 1e-10);
    }

    // =========================================================================
    // Interpolator Enum Tests
    // =========================================================================

    #[test]
    fn test_interpolator_enum_default() {
        let interp = Interpolator::default();
        assert!(matches!(interp, Interpolator::Flat));
        assert_eq!(
            <Interpolator as VolCubeInterpolator<f64>>::name(&interp),
            "Flat"
        );
    }

    #[test]
    fn test_interpolator_enum_flat() {
        let interp = Interpolator::Flat;
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        let result: Result<f64, InterpolationError> = interp.interpolate(1.25, &grid, &values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10.0);
    }

    #[test]
    fn test_interpolator_enum_linear() {
        let interp = Interpolator::Linear;
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 1.25 での線形補間: 10 + 0.25 * (20 - 10) = 12.5
        let result: Result<f64, InterpolationError> = interp.interpolate(1.25, &grid, &values);
        assert!(result.is_ok());
        assert!((result.unwrap() - 12.5).abs() < 1e-10);
    }

    #[test]
    fn test_interpolator_enum_supports_extrapolation() {
        assert!(<Interpolator as VolCubeInterpolator<f64>>::supports_extrapolation(
            &Interpolator::Flat
        ));
        assert!(<Interpolator as VolCubeInterpolator<f64>>::supports_extrapolation(
            &Interpolator::Linear
        ));
    }

    #[test]
    fn test_interpolator_enum_names() {
        assert_eq!(
            <Interpolator as VolCubeInterpolator<f64>>::name(&Interpolator::Flat),
            "Flat"
        );
        assert_eq!(
            <Interpolator as VolCubeInterpolator<f64>>::name(&Interpolator::Linear),
            "Linear"
        );
    }

    // =========================================================================
    // Edge Cases Tests
    // =========================================================================

    #[test]
    fn test_flat_interpolator_very_close_values() {
        let interp = FlatInterpolator::new();
        let grid = vec![1.0_f64, 2.0, 3.0];
        let values = vec![10.0_f64, 20.0, 30.0];

        // 1.2 は 1.0 に近いので 10.0 を返す
        let result = interp.interpolate(1.2, &grid, &values).unwrap();
        assert_eq!(result, 10.0);

        // 1.8 は 2.0 に近いので 20.0 を返す
        let result = interp.interpolate(1.8, &grid, &values).unwrap();
        assert_eq!(result, 20.0);
    }

    #[test]
    fn test_flat_interpolator_large_grid() {
        let interp = FlatInterpolator::new();
        let n = 1000;
        let grid: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let values: Vec<f64> = (0..n).map(|i| (i * 10) as f64).collect();

        // 中央付近での補間
        let result = interp.interpolate(500.3, &grid, &values).unwrap();
        assert_eq!(result, 5000.0); // 500 に近い

        let result = interp.interpolate(500.7, &grid, &values).unwrap();
        assert_eq!(result, 5010.0); // 501 に近い
    }

    #[test]
    fn test_flat_interpolator_negative_values() {
        let interp = FlatInterpolator::new();
        let grid = vec![-3.0_f64, -2.0, -1.0, 0.0, 1.0];
        let values = vec![30.0_f64, 20.0, 10.0, 0.0, -10.0];

        let result = interp.interpolate(-2.3, &grid, &values).unwrap();
        assert_eq!(result, 20.0); // -2.0 に近い

        let result = interp.interpolate(-0.3, &grid, &values).unwrap();
        assert_eq!(result, 0.0); // 0.0 に近い
    }
}

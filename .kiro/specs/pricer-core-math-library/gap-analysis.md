# Gap Analysis: pricer-core-math-library

## 概要

本分析は、`pricer_core::math`モジュールの拡充要件と既存コードベースのギャップを調査し、実装戦略を提案する。

## 1. 現状調査

### 1.1 既存の`pricer_core::math`構造

```
crates/pricer_core/src/math/
├── mod.rs              # メインモジュール（smoothing, interpolators, numeric, solvers）
├── numeric.rs          # 型変換ユーティリティ（from_f64, from_i32, from_usize）
├── smoothing.rs        # スムース関数（smooth_max, smooth_min, smooth_indicator, smooth_abs等）
├── interpolators/
│   ├── traits.rs       # Interpolator<T> trait
│   ├── linear.rs       # LinearInterpolator
│   ├── cubic_spline.rs # CubicSplineInterpolator
│   ├── monotonic.rs    # MonotonicInterpolator（Fritsch-Carlson）
│   ├── bilinear.rs     # BilinearInterpolator（2D）
│   └── smooth_interp.rs # smooth_interp関数
└── solvers/
    ├── config.rs       # SolverConfig<T>, LMConfig
    ├── newton_raphson.rs # NewtonRaphsonSolver
    ├── brent.rs        # BrentSolver
    └── levenberg_marquardt.rs # LevenbergMarquardtSolver
```

### 1.2 他クレートの関連アセット

| アセット | 場所 | 備考 |
|----------|------|------|
| `norm_cdf`, `norm_pdf` | `pricer_models::analytical::distributions` | Abramowitz-Stegun近似、精度1e-7 |
| `BlackScholes` | `pricer_models::analytical::black_scholes` | 価格 + Greeks（Delta, Gamma, Vega, Theta, Rho） |
| `Bachelier` | `pricer_models::analytical::bachelier` | 正規モデル価格 |
| `GarmanKohlhagen` | `pricer_models::analytical::garman_kohlhagen` | FXオプション（feature-gated） |
| `SabrModel` | `pricer_models::models::equity::sabr` | 確率モデル（Hagan近似あり） |
| `PricerRng` | `pricer_pricing::rng` | StdRng wrapper + Ziggurat正規乱数 |
| QMC placeholders | `pricer_pricing::rng::qmc` | Sobol未実装 |

### 1.3 既存パターンと規約

- **ジェネリック型**: `T: num_traits::Float`で統一
- **エラーハンドリング**: `thiserror`による構造化エラー型
- **テスト**: `approx::assert_relative_eq!`、プロパティベーステスト（proptest）
- **ドキュメント**: モジュールレベル + 関数レベルの詳細なdocコメント
- **命名**: British English（optimiser, serialisation等）

## 2. 要件フィージビリティ分析

### Requirement 1: 確率分布（Distribution）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 正規分布 CDF/PDF | `pricer_models`に存在 | **移動または再実装必要** | 高 |
| 正規分布 inverse CDF | なし | **新規実装必要**（Acklam近似） | 高 |
| 二変量正規分布 | なし | **新規実装必要**（Drezner-Wesolowsky） | 中 |
| 非心カイ二乗分布 | なし | **新規実装必要** | 中 |
| ガウシアンコピュラ | なし | **新規実装必要** | 低 |

**技術的課題**:
- 正規分布関数は`pricer_models`に存在。`pricer_core`に移動するか、再実装するかの判断が必要
- `pricer_core`は他のpricerクレートに依存できないため、移動が適切

### Requirement 2: 数値積分（Integrator）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| Gauss-Legendre求積法 | なし | **新規実装必要** | 高 |
| Gauss-Kronrod求積法 | なし | **新規実装必要** | 高 |
| 2次元積分 | なし | **新規実装必要** | 中 |
| Runge-Kutta（RK4, RK45） | なし | **新規実装必要** | 中 |
| 無限区間変換 | なし | **新規実装必要** | 低 |

**技術的課題**:
- Gauss-Legendre/Kronrodの重みと節点は事前計算可能（const配列）
- ジェネリッククロージャ `Fn(T) -> T` の設計

### Requirement 3: 有限差分（Calculus/FiniteDifference）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 前方/後方/中心差分 | なし | **新規実装必要** | 高 |
| 2階導関数 | なし | **新規実装必要** | 高 |
| 偏微分 | なし | **新規実装必要** | 中 |
| bump幅自動選択 | なし | **新規実装必要** | 低 |

**技術的課題**:
- 多変数関数の偏微分はインデックス指定の設計が必要

### Requirement 4: 最適化拡張（Optimiser）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| L-BFGS | なし | **新規実装必要** | 高 |
| Nelder-Mead（Amoeba） | なし | **新規実装必要** | 中 |
| 直線探索アルゴリズム | なし | **新規実装必要** | 中 |
| コールバック機能 | なし | **新規実装必要** | 低 |

**技術的課題**:
- L-BFGSはメモリ管理（履歴ベクトル）の設計が必要
- 既存の`LevenbergMarquardtSolver`との一貫したインターフェース設計

### Requirement 5: 1次元補間拡張（Interpolator 1D）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| フラット補間 | なし | **新規実装必要** | 高 |
| 対数線形補間 | なし | **新規実装必要** | 高 |
| Hermiteスプライン | なし | **新規実装必要** | 中 |
| Kahale補間 | なし | **新規実装必要**（Research Needed） | 低 |
| SVI補間 | なし | **新規実装必要** | 中 |
| 二分探索/線形探索 | なし | **新規実装必要** | 高 |
| 外挿モード | なし | **新規実装必要** | 中 |

**技術的課題**:
- Kahale補間はアービトラージフリー条件の研究が必要
- 既存の`Interpolator<T>` traitを拡張または継承

### Requirement 6: 2D/3D補間（Interpolator 2D/3D）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| バイリニア補間 | `BilinearInterpolator`存在 | なし | - |
| IDW（逆距離加重） | なし | **新規実装必要** | 中 |
| トリリニア3D補間 | なし | **新規実装必要** | 中 |
| レイヤード3D補間 | なし | **新規実装必要** | 低 |
| サーフェス微分 | なし | **新規実装必要** | 中 |

### Requirement 7: 金融関数（FinancialFunctions）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| Black-Scholes | `pricer_models`に存在 | **移動検討** | 高 |
| Bachelier | `pricer_models`に存在 | **移動検討** | 高 |
| SABR（Hagan近似） | `pricer_models`に部分的存在 | **補完または移動** | 中 |
| Normal SABR（Antonov） | なし | **新規実装必要**（Research Needed） | 中 |
| SVI | なし | **新規実装必要** | 中 |
| アービトラージフリー検証 | なし | **新規実装必要** | 低 |

**技術的課題**:
- `pricer_models`からの移動は依存関係の再設計が必要
- L1（pricer_core）とL2（pricer_models）の責務分離を維持

### Requirement 8: フィッティング（Fitting）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 線形最小二乗 | なし | **新規実装必要** | 中 |
| ガウシアンフィット | なし | **新規実装必要** | 低 |
| R²/残差 | なし | **新規実装必要** | 中 |

### Requirement 9: 線形代数（LinearAlgebra）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 行列演算 | なし | **新規実装必要** | 高 |
| コレスキー分解 | なし | **新規実装必要** | 高 |
| LU分解 | なし | **新規実装必要** | 中 |
| 行列式 | なし | **新規実装必要** | 中 |

**技術的課題**:
- 外部クレート（nalgebra, ndarray）の使用 vs 自前実装の判断
- Enzyme AD互換性の確保（自前実装が望ましい）

### Requirement 10: 乱数生成（RandomNumberGenerator）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| Mersenne Twister | `pricer_pricing`に`PricerRng`存在 | **移動検討** | 中 |
| シード再現性 | `pricer_pricing`に存在 | **移動検討** | 中 |
| 一様乱数 | `pricer_pricing`に存在 | **移動検討** | 中 |
| 正規乱数 | `pricer_pricing`にZiggurat存在 | **移動検討** | 中 |

**技術的課題**:
- `pricer_pricing`からの移動はEnzyme互換性の観点で検討必要
- L1に移動すると`pricer_pricing`以外のクレートも利用可能に

### Requirement 11: ルートファインダー拡張（Solver）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 二分法 | なし | **新規実装必要** | 高 |
| Backtracking Newton | なし | **新規実装必要** | 中 |
| 汎用ソルバー（自動選択） | なし | **新規実装必要** | 低 |

### Requirement 12: メッシュ生成（Mesh）

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| 1Dメッシュ（等間隔/対数） | なし | **新規実装必要** | 中 |
| 2Dメッシュ | なし | **新規実装必要** | 低 |
| グリッド細分化 | なし | **新規実装必要** | 低 |

### Requirement 13: ユーティリティ関数

| 機能 | 現状 | ギャップ | 優先度 |
|------|------|----------|--------|
| sign, clamp, lerp | 部分的（smooth関数） | **補完必要** | 高 |
| 階乗、組み合わせ | なし | **新規実装必要** | 中 |
| log_gamma, beta | なし | **新規実装必要** | 中 |

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象要件**: Requirement 5（1D補間拡張）、Requirement 11（ソルバー拡張）

**アプローチ**:
- 既存の`interpolators/`と`solvers/`ディレクトリに新規ファイルを追加
- 既存の`Interpolator<T>` traitを実装

**トレードオフ**:
- ✅ 既存パターンを踏襲、一貫性維持
- ✅ テストインフラを再利用可能
- ❌ 大量の新規ファイルで既存ディレクトリが肥大化

### Option B: 新規モジュール作成

**対象要件**: Requirement 1（分布）、Requirement 2（積分）、Requirement 3（有限差分）、Requirement 4（最適化）、Requirement 7（金融関数）、Requirement 8（フィッティング）、Requirement 9（線形代数）、Requirement 12（メッシュ）

**アプローチ**:
```
crates/pricer_core/src/math/
├── distributions/       # 新規：確率分布
├── integrators/         # 新規：数値積分
├── calculus/            # 新規：有限差分
├── optimisers/          # 新規：最適化アルゴリズム
├── financial/           # 新規：金融関数
├── fitting/             # 新規：フィッティング
├── linalg/              # 新規：線形代数
└── mesh/                # 新規：メッシュ生成
```

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ 各モジュールを独立してテスト可能
- ❌ ファイル数・ディレクトリ数の増加

### Option C: ハイブリッドアプローチ（推奨）

**アプローチ**:
1. **新規モジュール作成**: distributions, integrators, calculus, optimisers, linalg, mesh
2. **既存モジュール拡張**: interpolators/, solvers/
3. **金融関数は`pricer_models`に残す**: L1（基盤数学）とL2（金融ロジック）の責務分離を維持
4. **乱数生成は`pricer_pricing`に残す**: Enzyme互換性とMC専用性を維持

**フェーズ分割**:
- **Phase 1（基盤）**: distributions, calculus, 基本utility
- **Phase 2（数値計算）**: integrators, 補間拡張, ソルバー拡張
- **Phase 3（高度機能）**: optimisers, linalg, fitting, mesh

**トレードオフ**:
- ✅ A-I-P-Sアーキテクチャを維持
- ✅ 段階的実装でリスク軽減
- ❌ 金融関数（BS, Bachelier, SABR）の重複可能性

## 4. 複雑度とリスク評価

### 工数見積もり

| 要件 | 工数 | 根拠 |
|------|------|------|
| Req 1: 分布 | L | 複数の数学的近似の実装、テスト検証 |
| Req 2: 積分 | L | Gauss求積法の節点/重み計算、適応的積分 |
| Req 3: 有限差分 | S | 単純な数式、テスト容易 |
| Req 4: 最適化 | L | L-BFGSのメモリ管理、直線探索 |
| Req 5: 1D補間拡張 | M | 既存パターンあり、新規アルゴリズム |
| Req 6: 2D/3D補間 | M | 既存BilinearInterpolator拡張 |
| Req 7: 金融関数 | XL | 移動判断 + SABR/SVI実装 |
| Req 8: フィッティング | S | 線形代数依存 |
| Req 9: 線形代数 | L | 行列演算、分解アルゴリズム |
| Req 10: 乱数生成 | S | 移動のみ（新規実装なし） |
| Req 11: ソルバー拡張 | M | 二分法は単純、Backtracking Newtonは中程度 |
| Req 12: メッシュ | S | 単純なグリッド生成 |
| Req 13: ユーティリティ | S | 基本関数 |

**合計工数**: XL（2週間以上）

### リスク評価

| リスク要因 | レベル | 緩和策 |
|------------|--------|--------|
| Enzyme AD互換性 | 中 | 自前実装でブランチ回避、テストで検証 |
| 数値精度 | 中 | 参照実装との比較テスト |
| 依存関係変更 | 高 | 金融関数は`pricer_models`に残す |
| 既存コード破壊 | 低 | 新規モジュールのみ追加 |

## 5. 設計フェーズへの推奨事項

### 優先実装項目

1. **distributions**: 正規分布（CDF/PDF/inverse CDF）、二変量正規分布
2. **calculus**: 有限差分（前方/後方/中心/2階）
3. **integrators**: Gauss-Legendre、Gauss-Kronrod
4. **interpolators拡張**: フラット補間、対数線形補間、二分探索
5. **solvers拡張**: 二分法

### 研究必要項目（Research Needed）

- **Kahale補間**: アービトラージフリー条件の数学的背景
- **Normal SABR Antonov近似**: 実装詳細の文献調査
- **L-BFGS**: 2ループ再帰公式の実装詳細

### 設計判断項目

1. **金融関数の配置**: `pricer_core`に純粋数学関数のみ配置し、`pricer_models`に金融ロジックを残すか
2. **外部クレート使用**: 線形代数で`nalgebra`を使用するか、自前実装するか
3. **SIMDサポート**: SimdBSは除外。将来的なSIMD最適化は別途検討

## 6. 次のステップ

1. 本ギャップ分析を確認
2. `/kiro:spec-design pricer-core-math-library` で設計ドキュメントを生成
3. 設計で具体的なモジュール構造、API設計、テスト戦略を定義

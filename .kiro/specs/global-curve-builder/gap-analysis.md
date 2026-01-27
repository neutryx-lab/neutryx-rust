# Gap Analysis: Global Curve Builder

## 1. 現状調査（Current State Investigation）

### 1.1 関連ドメイン資産

#### ソルバー関連 (`pricer_core/src/math/solvers/`)

| ファイル | 内容 | 再利用可能性 |
|----------|------|--------------|
| [newton_raphson.rs](crates/pricer_core/src/math/solvers/newton_raphson.rs) | 1次元Newton-Raphsonソルバー | ❌ スカラーのみ、多次元非対応 |
| [backtracking_newton.rs](crates/pricer_core/src/math/solvers/backtracking_newton.rs) | バックトラッキング付きNewton法 | ⚠️ 1次元、アルゴリズムパターン参照可 |
| [levenberg_marquardt.rs](crates/pricer_core/src/math/solvers/levenberg_marquardt.rs) | LM法（最小二乗） | ⚠️ 多次元対応、収束ロジック参照可 |

#### 線形代数 (`pricer_core/src/math/linalg/`)

| ファイル | 内容 | 再利用可能性 |
|----------|------|--------------|
| [wrappers.rs](crates/pricer_core/src/math/linalg/wrappers.rs) | nalgebra ラッパー（LU, Cholesky, QR, inverse） | ✅ 直接再利用可能 |
| [error.rs](crates/pricer_core/src/math/linalg/error.rs) | `LinearAlgebraError` 列挙型 | ✅ 拡張可能 |

**linalg/wrappers.rs の主要関数:**
- `lu_solve<T: RealField>(a: &DMatrix<T>, b: &DVector<T>) -> Result<DVector<T>, ...>`
- `inverse<T: RealField>(a: &DMatrix<T>) -> Result<DMatrix<T>, ...>`
- `cholesky_solve`, `qr_solve` など

#### ブートストラップ (`pricer_models/src/market/calibration/bootstrapping/`)

| ファイル | 内容 | 再利用可能性 |
|----------|------|--------------|
| [engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs) | `SequentialBootstrapper<T>` | ⚠️ API参照、グローバル版で置換 |
| [instrument.rs](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs) | `BootstrapInstrument<T>` 列挙型 | ✅ CalibrationInstrumentトレイトの基盤 |

**SequentialBootstrapper の特徴:**
- 1商品ずつ逐次的に解く
- Newton-Raphson（Brentフォールバック）使用
- Jacobian未公開

**BootstrapInstrument の構成:**
- `OIS`, `IRS`, `FRA`, `Future` variants
- `residual(&self, curve) -> T` メソッド
- `residual_derivative(&self, curve) -> T` メソッド（スカラー微分）

#### エラー型 (`pricer_core/src/types/error.rs`)

```rust
pub enum SolverError {
    MaxIterationsExceeded { iterations: usize, residual: f64 },
    DerivativeNearZero,
    NoBracket,
    NumericalInstability(String),
    External(String),
}
```

**Gap:** `SingularJacobian`, `DimensionMismatch` バリアントが欠如

#### Enzyme AAD (`pricer_risk/src/enzyme/`)

| ファイル | 内容 | 再利用可能性 |
|----------|------|--------------|
| [shadow.rs](crates/pricer_risk/src/enzyme/shadow.rs) | Shadow Objectパターン | ✅ カーブ勾配蓄積に利用可 |
| [binder.rs](crates/pricer_risk/src/enzyme/binder.rs) | `MarketRiskCalculator`, `RiskResult<M>` | ✅ パターン参照 |
| [reverse.rs](crates/pricer_risk/src/enzyme/reverse.rs) | `ReverseAD<T>`, `GreeksResult<T>` | ⚠️ 商品Greeks用、カーブ感応度は別設計 |

**Shadow Object パターン:**
- `trait Shadow: Clone` with `zero_out()`, `create_shadow()`
- `SimpleYieldCurve`, `SimpleMarketData` 実装
- Finite difference fallback 対応

### 1.2 アーキテクチャパターンと制約

**A-I-P-S 依存関係ルール:**
- `pricer_core` (L1) → 他レイヤーに依存しない
- `pricer_models` (L2) → `pricer_core` のみに依存
- `pricer_risk` (L4) → `pricer_core`, `pricer_models` に依存可

**命名規則:**
- British English: `optimiser`, `serialisation`
- トレイト: 動詞または形容詞（`Solvable`, `Interpolatable`）
- エラー型: `{Domain}Error` パターン

**Float ジェネリクス:**
- `num_traits::Float` または `nalgebra::RealField`
- AD互換性のため `T` パラメータ必須

### 1.3 統合サーフェス

**データモデル:**
- `InterpolatedCurve<T>` (pricer_models)
- `SimpleYieldCurve` (pricer_risk/enzyme, f64固定)

**API/インターフェース:**
- `BootstrapConfig` (既存設定構造体)
- `Bootstrapper` トレイト (存在する場合)

---

## 2. 要件実現可能性分析

### 2.1 要件対資産マッピング

| 要件 | 技術要素 | 既存資産 | Gap | 複雑度 |
|------|----------|----------|-----|--------|
| Req 1: 多次元Newton-Raphson | ソルバー | 1D版のみ | **Missing** | Algorithm |
| Req 2: SystemOfEquations トレイト | 抽象化 | なし | **Missing** | Interface |
| Req 3: SolverResult構造体 | データ型 | なし | **Missing** | Simple |
| Req 4: CurveCalibrationProblem | 実装 | BootstrapInstrument | Partial | Workflow |
| Req 5: CalibrationInstrument トレイト | 抽象化 | BootstrapInstrument | Partial | Interface |
| Req 6: AAD陰関数定理 | Enzyme統合 | Shadow Object基盤 | **Missing** | Complex |
| Req 7: 線形代数演算 | 数値計算 | wrappers.rs | ✅ 充足 | - |
| Req 8: GlobalBootstrapper | 統合 | SequentialBootstrapper | **Missing** | Workflow |
| Req 9: エラーハンドリング | エラー型 | SolverError | Partial | Simple |
| Req 10: パフォーマンス | 最適化 | 未検証 | **Unknown** | Integration |
| Req 11: テスト | 品質保証 | 基盤あり | Partial | Testing |

### 2.2 Gap詳細

#### Missing: 多次元Newton-Raphsonソルバー (Req 1)

**現状:** 1次元ソルバー（newton_raphson.rs）のみ
**必要機能:**
- `solve<T>(x0: Array1<T>, f: F, J: JacobianFn) -> Result<SolverResult<T>, SolverError>`
- 収束判定: 絶対/相対許容誤差
- Jacobian逆行列（またはLU分解）の返却
- Float ジェネリクス対応

**実装難易度:** M（3-7日）

#### Missing: SystemOfEquations トレイト (Req 2)

**現状:** 統一インターフェースなし
**設計案:**
```rust
pub trait SystemOfEquations<T: Float> {
    fn evaluate(&self, x: &Array1<T>) -> Result<Array1<T>, SolverError>;
    fn jacobian(&self, x: &Array1<T>) -> Result<Array2<T>, SolverError>;
    fn dimension(&self) -> usize;

    // デフォルト実装: 数値Jacobian
    fn jacobian_numerical(&self, x: &Array1<T>, eps: T) -> Result<Array2<T>, SolverError>;
}
```

**実装難易度:** S（1-3日）

#### Missing: AAD陰関数定理統合 (Req 6)

**現状:** Shadow Objectパターンは実装済み、ただしソルバー統合なし
**必要機能:**
- `ImplicitSolver` with custom gradient rule
- Adjoint計算: `∂L/∂m = J⁻ᵀ · ∂L/∂x*`
- Enzyme `#[enzyme_rules]` または手動実装
- Finite difference fallback

**Research Needed:**
- Enzyme custom gradient ruleのRust API
- nalgebra::DMatrix との互換性
- チェックポイント戦略（メモリ効率）

**実装難易度:** L（1-2週間）

#### Partial: BootstrapInstrument → CalibrationInstrument (Req 4, 5)

**現状:**
- `BootstrapInstrument<T>` enumは存在
- `residual()` メソッドあり
- 複数カーブ（dual-curve）未対応

**必要な拡張:**
- トレイト抽象化
- `CurveSet` 引数サポート
- Jacobian行への寄与計算

**実装難易度:** M（3-7日）

#### Partial: SolverError拡張 (Req 9)

**現状バリアント:** `MaxIterationsExceeded`, `DerivativeNearZero`, etc.

**追加必要:**
```rust
SingularJacobian { min_pivot: f64 },
DimensionMismatch { expected: usize, got: usize },
```

**実装難易度:** S（1-3日）

### 2.3 制約と不明点

**アーキテクチャ制約:**
- `pricer_core`に金融ロジックを含めてはならない
- AAD統合は`pricer_risk`レイヤーで行う
- nalgebra依存はOK（既に使用）

**Research Needed:**
- [ ] Enzyme `#[enzyme_rules]` の現在のサポート状況
- [ ] `ndarray` vs `nalgebra` の選択（既存はnalgebra）
- [ ] スパース行列サポートの必要性（30商品程度なら密行列で十分か）

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象:**
- `pricer_core/src/math/solvers/` に `multidim_newton.rs` 追加
- `BootstrapInstrument` をトレイトに変換

**適用条件:**
- 既存solversパターンに準拠
- Float genericsを維持

**Trade-offs:**
- ✅ 既存パターン踏襲、学習コスト低
- ✅ solversモジュール内の一貫性維持
- ❌ 既存テストへの影響確認必要
- ❌ BootstrapInstrument変更は破壊的変更

**推奨度:** ⭐⭐⭐⭐

### Option B: 新規コンポーネント作成

**新規作成:**
- `pricer_core/src/math/systems/` モジュール（SystemOfEquations, MultidimensionalSolver）
- `pricer_models/src/market/calibration/global/` モジュール（GlobalBootstrapper）
- `pricer_risk/src/enzyme/implicit.rs`（ImplicitSolver）

**適用条件:**
- 明確な責務分離が必要
- 既存コードへの影響を最小化したい

**Trade-offs:**
- ✅ クリーンな責務分離
- ✅ 既存コードへの影響なし
- ✅ 独立したテストが容易
- ❌ ファイル数増加
- ❌ 重複コードのリスク（BootstrapInstrumentとCalibrationInstrument）

**推奨度:** ⭐⭐⭐⭐⭐

### Option C: ハイブリッドアプローチ

**段階的実装:**
1. **Phase 1:** 新規 `systems/` モジュールでソルバーコア実装
2. **Phase 2:** `CalibrationInstrument` トレイトを新規作成、`BootstrapInstrument` を実装
3. **Phase 3:** `GlobalBootstrapper` を `SequentialBootstrapper` と並行配置
4. **Phase 4:** AAD統合（Enzyme custom rule または fallback）

**適用条件:**
- 段階的リリースが求められる
- 後方互換性の維持が必須

**Trade-offs:**
- ✅ リスク分散
- ✅ 各段階で検証可能
- ✅ 後方互換性維持
- ❌ 計画の複雑化
- ❌ 中間状態の管理コスト

**推奨度:** ⭐⭐⭐⭐

---

## 4. 実装複雑度とリスク

### 工数見積もり

| コンポーネント | 工数 | 根拠 |
|---------------|------|------|
| SystemOfEquations トレイト | S | トレイト定義のみ |
| MultidimensionalSolver | M | アルゴリズム実装、既存パターン参照可 |
| SolverResult構造体 | S | 単純なデータ構造 |
| CalibrationInstrument トレイト | S | 既存BootstrapInstrument参照 |
| CurveCalibrationProblem | M | SystemOfEquations実装、商品評価 |
| GlobalBootstrapper | M | 既存APIとの互換性維持 |
| ImplicitSolver (AAD) | L | Enzyme統合、カスタム勾配ルール |
| テストスイート | M | 単体・統合・ベンチマーク |

**総工数: L〜XL（2-3週間）**

### リスク評価

| リスク要因 | レベル | 緩和策 |
|-----------|--------|--------|
| Enzyme custom rule APIの不確実性 | High | Finite difference fallback を先行実装 |
| nalgebra Float制約とAD互換性 | Medium | RealFieldバウンドで統一 |
| 既存SequentialBootstrapperとの結果差異 | Medium | 比較テストを早期実装 |
| パフォーマンス要件達成 | Medium | ベンチマークを継続的に実行 |

**総合リスク: Medium-High**（AAD統合の不確実性による）

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B（新規コンポーネント作成）** を推奨

**理由:**
1. 既存の`SequentialBootstrapper`との共存が可能
2. クリーンなレイヤー分離（Math → Model → Risk）
3. AAD統合を独立モジュールとして段階的に実装可能
4. テストの独立性確保

### 設計フェーズでの調査項目

1. **Enzyme統合方式の決定**
   - `#[enzyme_rules]` attributeの利用可否
   - 手動VJP実装との比較

2. **Float型戦略**
   - `nalgebra::RealField` vs `num_traits::Float + nalgebra互換`
   - AD型（Enzyme shadow type）との統合パターン

3. **データ構造選択**
   - `nalgebra::DMatrix<T>` vs `ndarray::Array2<T>`
   - 既存コードベースとの一貫性（nalgebra優先）

4. **CalibrationInstrumentの詳細設計**
   - Dual-curve対応の引数設計
   - 既存`BootstrapInstrument`との関係

### Key Decisions

| 決定項目 | 推奨 | 根拠 |
|----------|------|------|
| Jacobian格納形式 | LU分解 + explicit inverse | メモリ効率とAAD互換性のバランス |
| 数値Jacobianデフォルト | 有効 | 開発速度優先、後で解析的Jacobian追加可 |
| 後方互換性 | feature flagで切替 | 段階的移行を可能に |
| テスト戦略 | Sequential vs Global 比較テスト必須 | 数値的一貫性の検証 |

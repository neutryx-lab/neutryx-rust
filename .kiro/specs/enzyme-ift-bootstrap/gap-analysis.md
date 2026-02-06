# Gap Analysis: enzyme-ift-bootstrap

## 概要

本文書は、Enzyme AD による Global Bootstrapping の最適化と Implicit Function Theorem (IFT) 感度抽出機能の実装に向けた、要件と既存コードベース間のギャップ分析を提供します。

## 1. 現状調査結果

### 1.1 関連アセット

| モジュール | パス | 役割 |
|-----------|------|------|
| GlobalBootstrapper | `pricer_models/src/builder/curve/global.rs` | 全ピラー同時解法、J⁻¹ キャッシュ対応済 |
| CalibrationProblem | `pricer_models/src/builder/problem.rs` | SystemOfEquations 実装、JacobianMethod enum |
| CalibrationEngine | `pricer_models/src/builder/engine.rs` | LinearSolveStrategy プラガブル設計 |
| LinearSolveStrategy | `pricer_core/src/math/linalg/strategy.rs` | LUStrategy, LowerTriangularStrategy 実装 |
| ImplicitSolver | `pricer_risk/src/greeks/ad/implicit_solver.rs` | IFT 感度計算の既存実装 |
| Shadow trait | `pricer_risk/src/greeks/ad/shadow.rs` | AAD 勾配蓄積パターン |
| MarketRiskCalculator | `pricer_risk/src/greeks/ad/binder.rs` | AAD バインダー層 |
| Pricing Kernels | `pricer_risk/src/greeks/ad/kernel.rs` | `#[autodiff]` マクロ使用例 |

### 1.2 既存パターン・規約

- **依存方向**: A → I → P → S の単方向フロー厳守
- **ジェネリクス**: `T: RealField + Copy + Float` による AD 互換性
- **戦略パターン**: `LinearSolveStrategy<T>` トレイトによるアルゴリズム切替
- **Feature Gate**: `#[cfg(feature = "enzyme-ad")]` によるコンパイル時制御

### 1.3 インテグレーション境界

- `CalibrationEngine<T, S: LinearSolveStrategy<T>>` が中核の拡張ポイント
- `GlobalBootstrapResult` が `jacobian_inverse: Option<DMatrix<T>>` を既に保持
- `ImplicitSolver::compute_curve_sensitivities` が IFT 実装を既に提供

---

## 2. 要件-アセット マッピング

### Requirement 1: Enzyme AD による Jacobian 計算

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC1.1: JacobianMethod::AD で Enzyme reverse mode 使用 | `problem.rs:JacobianMethod` | **Missing** - スタブ実装のみ |
| AC1.2: 解析微分と 1e-12 相対誤差以内 | - | **Unknown** - 要ベンチマーク |
| AC1.3: enzyme-ad 有効時に AD をデフォルト選択 | `problem.rs:JacobianMethod::default()` | **Missing** |
| AC1.4: 失敗時の有限差分フォールバック | `problem.rs` | **Partial** - ログ警告未実装 |
| AC1.5: 全 BootstrapInterpolation 対応 | `market.rs:BootstrapInterpolation` | **Constraint** - MonotonicCubic, NaturalCubicSpline 未実装 |

### Requirement 2: 内挿スキームの微分可能実装

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC2.1: `discount_factor_with_gradient` メソッド | `YieldCurve` trait | **Missing** |
| AC2.2: LogLinear 解析微分 ∂DF(t)/∂DF_i | - | **Missing** |
| AC2.3: InterpolatorEnum に `#[autodiff]` 互換インターフェース | `interpolator.rs` | **Missing** |
| AC2.4: AD dual numbers の伝播 | `CalibrationProblem` | **Partial** - `T: RealField` 制約あり |
| AC2.5: 非対応時のコンパイル時エラー | - | **Missing** |

### Requirement 3: IFT 感度抽出

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC3.1: `ift_sensitivity` メソッド | `GlobalBootstrapResult` | **Missing** - メソッドなし |
| AC3.2: J⁻¹ キャッシュ | `GlobalBootstrapResult::jacobian_inverse` | **Exists** ✓ |
| AC3.3: バッチ市場パラメータ感度 | `ImplicitSolver::compute_curve_sensitivities` | **Partial** - 単一パラメータのみ |
| AC3.4: キャッシュなし時のエラー | - | **Missing** |
| AC3.5: bump-and-recalibrate と 1e-8 一致 | - | **Unknown** - 要検証 |

### Requirement 4: LinearSolveStrategy 拡張

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC4.1: `sparse_solve` メソッド (CSR) | `LinearSolveStrategy` trait | **Missing** |
| AC4.2: SparseCholeskyStrategy | `pricer_core::math::linalg` | **Missing** |
| AC4.3: 70% スパース時の自動選択 | `GlobalBootstrapper` | **Missing** |
| AC4.4: Sparse LU factorisation キャッシュ | - | **Missing** |
| AC4.5: GMRES による反復解法 | - | **Missing** |

### Requirement 5: 数値安定性保証

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC5.1: 条件数モニタリング | `GlobalBootstrapper` | **Missing** |
| AC5.2: Tikhonov 正則化 | - | **Missing** |
| AC5.3: `validate_jacobian_quality` | `CalibrationProblem` | **Missing** |
| AC5.4: AD 不安定時の中央差分切替 | - | **Missing** |
| AC5.5: `numerical_diagnostics` フィールド | `GlobalBootstrapResult` | **Missing** |

### Requirement 6: 設定とフィーチャーフラグ統合

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC6.1: `enzyme-ad` で AD 制御 | `problem.rs` | **Exists** ✓ - `#[cfg(feature)]` 済 |
| AC6.2: 無効時の設定非公開 | `GlobalBootstrapConfig` | **Missing** |
| AC6.3: `with_jacobian_method` ビルダー | `GlobalBootstrapConfig` | **Partial** - 検証なし |
| AC6.4: 非互換時のコンパイル時エラー | - | **Missing** |
| AC6.5: `ad_checkpoint_interval` | `CalibrationProblemConfig` | **Missing** |

### Requirement 7: pricer_risk AAD Binder 統合

| 受入基準 | 関連アセット | ステータス |
|---------|-------------|-----------|
| AC7.1: GlobalBootstrapResult を AAD binder 入力に | `binder.rs:MarketRiskCalculator` | **Missing** |
| AC7.2: キャッシュ J⁻¹ 使用 | `binder.rs` | **Missing** |
| AC7.3: Shadow trait for GlobalBootstrapResult | `shadow.rs` | **Missing** |
| AC7.4: トレードバッチ処理 | `binder.rs` | **Partial** - 基盤あり |
| AC7.5: オンデマンド再キャリブレーション | - | **Missing** |

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**アプローチ**: 現在の `CalibrationEngine`, `LinearSolveStrategy`, `ImplicitSolver` を直接拡張

**拡張対象ファイル**:
- `pricer_core/src/math/linalg/strategy.rs` - sparse_solve メソッド追加
- `pricer_models/src/builder/problem.rs` - Enzyme AD 統合
- `pricer_models/src/builder/curve/global.rs` - IFT メソッド追加
- `pricer_risk/src/greeks/ad/shadow.rs` - GlobalBootstrapResult 実装
- `pricer_risk/src/greeks/ad/binder.rs` - 入力型拡張

**トレードオフ**:
- ✅ 最小限のファイル追加
- ✅ 既存テストインフラ活用
- ✅ 学習コスト低
- ❌ strategy.rs が肥大化リスク
- ❌ 後方互換性の慎重な管理必要
- ❌ sparse 依存 (sprs/nalgebra-sparse) の pricer_core 追加

### Option B: 新規コンポーネント作成

**アプローチ**: 専用の `enzyme_calibration` モジュールと `sparse_linalg` サブモジュールを新設

**新規ファイル**:
- `pricer_core/src/math/linalg/sparse/mod.rs` - 疎行列アルゴリズム
- `pricer_models/src/builder/enzyme.rs` - Enzyme 専用 CalibrationProblem
- `pricer_models/src/builder/ift.rs` - IFT 感度抽出層
- `pricer_risk/src/greeks/ad/curve_binder.rs` - カーブ専用 AAD binder

**トレードオフ**:
- ✅ 責務の明確な分離
- ✅ 単体テスト容易
- ✅ 既存コードへの影響最小
- ❌ コード重複リスク
- ❌ 統合ポイントの設計必要
- ❌ 学習曲線・ナビゲーションコスト

### Option C: ハイブリッドアプローチ (推奨)

**アプローチ**: 疎行列は新規モジュール、IFT/AAD 統合は既存拡張

**Phase 1: 基盤構築**
- `pricer_core/src/math/linalg/sparse/` 新規作成
  - `mod.rs`, `csr.rs`, `strategies.rs`
- `LinearSolveStrategy` trait に feature-gated sparse メソッド追加

**Phase 2: Enzyme AD 統合**
- `CalibrationProblem` に `#[autodiff]` 対応メソッド追加
- `JacobianMethod::AutomaticDifferentiation` の完全実装
- `BootstrapInterpolation` の微分可能実装

**Phase 3: IFT & AAD Binder**
- `GlobalBootstrapResult::ift_sensitivity` メソッド追加
- `Shadow` trait 実装
- `MarketRiskCalculator` 拡張

**トレードオフ**:
- ✅ 段階的デリバリー可能
- ✅ 疎行列の独立テスト
- ✅ 既存 API との自然な統合
- ✅ リスク分散（Phase 毎にレビュー）
- ❌ 複数フェーズの調整必要
- ❌ 中間状態での一貫性管理

---

## 4. 複雑度・リスク評価

### 工数 (Effort): **L (1-2 週間)**

**根拠**:
- 疎行列サポートは新規実装（nalgebra-sparse/sprs 統合）
- Enzyme AD の `#[autodiff]` マクロは既存パターンあり
- IFT は `ImplicitSolver` の拡張で対応可能
- 7 要件・35 受入基準の広範囲なスコープ

### リスク: **Medium-High**

**High リスク要因**:
- Enzyme AD の LLVM 依存とクロスプラットフォームビルド複雑性
- 疎行列ライブラリ選定（sprs vs nalgebra-sparse）の技術調査必要
- 数値安定性要件（条件数、正則化）の検証コスト

**Medium リスク要因**:
- `LinearSolveStrategy` trait の API 変更による後方互換性
- AD dual numbers の interpolation 層伝播
- 既存テストとの整合性維持

---

## 5. 設計フェーズへの推奨事項

### 5.1 推奨アプローチ

**Option C (ハイブリッド)** を推奨

**理由**:
1. 疎行列機能は pricer_core の独立した関心事として分離すべき
2. IFT/AAD 統合は既存の `ImplicitSolver`, `Shadow` パターンの自然な拡張
3. Phase 分割により早期フィードバック取得可能

### 5.2 主要設計決定事項

| 決定事項 | 選択肢 | 推奨 |
|---------|--------|------|
| 疎行列ライブラリ | sprs / nalgebra-sparse / 自前実装 | **Research Needed** |
| Jacobian inverse ストレージ | 明示的逆行列 / LU 分解保持 | LU 分解保持（メモリ効率） |
| IFT API 配置 | GlobalBootstrapResult / ImplicitSolver | GlobalBootstrapResult（凝集度） |
| Feature gate 粒度 | enzyme-ad 単一 / enzyme-ad + sparse 分離 | 分離（独立利用可能） |

### 5.3 Research Needed 項目

1. **疎行列ライブラリ選定**: sprs vs nalgebra-sparse のパフォーマンス・API 比較
2. **Enzyme Windows サポート**: CI/CD パイプラインでの Windows ビルド可否
3. **BootstrapInterpolation 拡張**: MonotonicCubic, NaturalCubicSpline の AD 互換実装調査
4. **条件数計算**: 大規模行列での効率的な条件数推定アルゴリズム
5. **GMRES 実装**: nalgebra/sprs での反復解法サポート状況

---

## 6. サマリー

| カテゴリ | 値 |
|---------|-----|
| 要件数 | 7 |
| 受入基準数 | 35 |
| Exists | 3 (8.6%) |
| Partial | 5 (14.3%) |
| Missing | 24 (68.6%) |
| Unknown | 2 (5.7%) |
| Constraint | 1 (2.8%) |
| 推奨アプローチ | Option C (Hybrid) |
| 工数 | L (1-2 weeks) |
| リスク | Medium-High |

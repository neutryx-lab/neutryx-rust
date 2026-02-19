# Requirements Document

## Project Description (Input)
crates\pricer_models\src\modelsとcrates\pricer_models\src\analyticalを削除、もしくは整理したい。これらの中に本当にモデルと呼べるものはあるのか？例えばcrates\pricer_models\src\models\sabr.rsは近似解析解が載っているだけ。大体はAnalyticalかcrates\pricer_core\src内にfinanceフォルダでも作ってそこに格納すべきでは？

## Introduction

本仕様は `pricer_models` クレート内の `models/` および `analytical/` モジュールの構造的見直しを定義する。現状、これらのモジュールには以下の問題がある：

1. **概念的混乱**: `models/sabr.rs` は確率的ボラティリティモデル（モンテカルロ用）とHagan近似公式（解析解）の両方を含む
2. **重複**: `analytical/distributions.rs` と `pricer_core/math/distributions/` の機能が重複
3. **配置の不整合**: A-I-P-S アーキテクチャにおいて、数学的基盤（L1）とビジネスロジック（L2）の境界が曖昧

### 現状分析

**models/ ディレクトリ** (11ファイル):
- `gbm.rs`, `heston.rs`, `hull_white.rs`, `cir.rs`, `correlated.rs` → 真の確率過程モデル（モンテカルロシミュレーション用）
- `sabr.rs` → 確率過程 + Hagan近似公式（混在）
- `stochastic.rs`, `model_enum.rs`, `validation.rs`, `error.rs` → インフラ

**analytical/ ディレクトリ** (6ファイル):
- `black_scholes.rs`, `bachelier.rs`, `garman_kohlhagen.rs` → 閉形式プライシング公式
- `distributions.rs` → `norm_cdf`, `norm_pdf`, `norm_inv_cdf`（`pricer_core` と重複）
- `error.rs` → エラー型

---

## Requirements

### Requirement 1: モジュール責務の明確化

**Objective:** As a 開発者, I want モジュールの責務を明確に分離する, so that コードの保守性と理解容易性が向上する

#### Acceptance Criteria
1. The pricer_models shall 確率過程モデル（`StochasticModel` trait実装）のみを `models/` に配置する
2. The pricer_models shall 閉形式プライシング公式を `analytical/` に集約する
3. When SABRモデルを参照する場合, the pricer_models shall 確率過程部分と近似公式部分を分離して提供する
4. The pricer_core shall 純粋な数学関数（確率分布、特殊関数等）を `math/` 配下で提供する

### Requirement 2: 重複コードの排除

**Objective:** As a 開発者, I want 重複する数学関数を統合する, so that 保守コストが削減され、一貫性が保たれる

#### Acceptance Criteria
1. The pricer_models shall `analytical/distributions.rs` を削除し、`pricer_core::math::distributions` を使用する
2. If `norm_cdf`, `norm_pdf`, `norm_inv_cdf` が必要な場合, the analytical module shall `pricer_core::math::distributions::normal` から re-export する
3. The pricer_core shall すべての確率分布関数を `math/distributions/` モジュールで統一的に提供する

### Requirement 3: SABR分離

**Objective:** As a 量的アナリスト, I want SABRの確率過程部分と近似公式部分を明確に区別できる, so that 用途に応じた適切な機能を選択できる

#### Acceptance Criteria
1. The pricer_models/models shall `SABRModel`（`StochasticModel` trait実装、モンテカルロ用）を提供する
2. The pricer_models/analytical shall `SabrImpliedVol`（Hagan公式によるインプライドボラティリティ計算）を提供する
3. When `models/sabr.rs` を分離する際, the pricer_models shall 既存のAPI互換性を維持する re-export を提供する
4. The `SabrImpliedVol` shall パラメータ（alpha, beta, rho, nu）からストライク・満期に対するインプライドボラティリティを計算する

### Requirement 4: レイヤー境界の整備

**Objective:** As a アーキテクト, I want L1（pricer_core）とL2（pricer_models）の境界を明確にする, so that A-I-P-S アーキテクチャが維持される

#### Acceptance Criteria
1. The pricer_core shall 金融固有でない純粋な数学関数のみを含む
2. The pricer_models shall 金融固有のビジネスロジック（プライシング、モデル）を含む
3. While リファクタリング中, the pricer_models shall `pricer_core` への依存方向を維持する（逆依存を作らない）
4. The pricer_models shall `analytical/` モジュールを維持し、閉形式解析解の提供場所として明確化する

### Requirement 5: 後方互換性

**Objective:** As a 既存ユーザー, I want 既存のAPIが引き続き動作する, so that コードの移行コストを最小化できる

#### Acceptance Criteria
1. The pricer_models shall 既存の public API（`BlackScholes`, `GarmanKohlhagen`, `SABRParams`, `SABRModel` 等）を維持する
2. If モジュールパスが変更される場合, the pricer_models shall 旧パスからの re-export を提供する
3. The pricer_models shall 非推奨（deprecated）警告を追加し、移行期間を設ける
4. When `analytical/distributions` を削除する場合, the pricer_models/analytical shall `pricer_core::math::distributions::normal` の関数を re-export する

### Requirement 6: テストとドキュメント

**Objective:** As a 開発者, I want リファクタリング後もすべてのテストが通過する, so that 機能の退行を防止できる

#### Acceptance Criteria
1. The pricer_models shall 既存のユニットテストがすべて通過する状態を維持する
2. The pricer_models shall 新しいモジュール構造に対応したドキュメントコメントを更新する
3. If 新しいモジュールが追加される場合, the pricer_models shall 適切なモジュールレベルドキュメント（`//!` コメント）を含める

---

## Out of Scope

以下は本仕様の範囲外とする：

1. `pricer_core` への新しい `finance/` ディレクトリの追加（純粋な数学は `math/` で十分）
2. `models/` ディレクトリ全体の削除（確率過程モデルは有用）
3. パフォーマンス最適化（本仕様は構造的リファクタリングに焦点）
4. 新規モデルの追加

## Glossary

| 用語 | 定義 |
|------|------|
| 確率過程モデル | `StochasticModel` trait を実装し、モンテカルロシミュレーションで使用されるモデル |
| 閉形式解 | 数値近似なしに解析的に計算可能な価格・感度の公式 |
| Hagan公式 | SABRモデルパラメータからインプライドボラティリティを近似計算する公式 (Hagan et al., 2002) |
| L1/L2 | Pricer レイヤー階層。L1=pricer_core（基盤）、L2=pricer_models（ビジネスロジック） |

# Research & Design Decisions: analytical-models

---
**目的**: 技術設計を支える発見事項、アーキテクチャ調査、および意思決定の根拠を記録する。
---

## 概要

- **機能名**: `analytical-models`
- **Discovery スコープ**: Extension (既存システムへの拡張)
- **主要な発見事項**:
  1. `pricer_models/src/analytical/mod.rs` が空のプレースホルダーとして存在
  2. 既存の `VanillaOption<T: Float>` 構造体との統合が必要
  3. `pricer_core::math::smoothing` の関数群が AD 互換性のために利用可能

## Research Log

### 既存アーキテクチャ分析

- **Context**: analytical モジュールの統合ポイントを特定
- **調査対象**:
  - `crates/pricer_models/src/analytical/mod.rs` - 空のプレースホルダー
  - `crates/pricer_models/src/instruments/vanilla.rs` - VanillaOption 定義
  - `crates/pricer_core/src/math/smoothing.rs` - 既存の滑らかな近似関数
  - `crates/pricer_core/src/traits/priceable.rs` - Priceable トレイト
- **発見事項**:
  - `analytical` モジュールは Phase 2 用のプレースホルダーとしてコメントアウトされている
  - `VanillaOption` は `strike()`, `expiry()`, `payoff_type()`, `exercise_style()` アクセサを提供
  - 既存の smoothing 関数: `smooth_max`, `smooth_indicator`, `smooth_sqrt`, `smooth_log`, `smooth_pow`
  - `ExerciseStyle::European` の判定は `is_european()` メソッドで可能
- **影響**: Black-Scholes/Bachelier を `analytical` モジュール内に実装し、`VanillaOption` との統合インターフェースを提供

### 累積正規分布関数 (norm_cdf) の設計

- **Context**: Black-Scholes 公式には標準正規分布の CDF が必要
- **調査対象**: AD 互換な実装方法
- **発見事項**:
  - Abramowitz and Stegun 近似 (7.1.26) が 1e-7 精度で広く使用される
  - `erfc` 関数は `num_traits::Float` で利用可能
  - `norm_cdf(x) = 0.5 * erfc(-x / sqrt(2))` で実装可能
  - AD 互換性のため条件分岐を避ける必要あり
- **影響**: `erfc` ベースの実装を採用、`pricer_core::math` または `pricer_models::analytical` に配置

### エラーハンドリングパターン

- **Context**: 既存エラー型との統合
- **調査対象**:
  - `crates/pricer_core/src/types/error.rs` - `PricingError` 定義
  - `crates/pricer_models/src/instruments/error.rs` - `InstrumentError` 定義
- **発見事項**:
  - `PricingError::InvalidInput(String)` - 不正な入力
  - `PricingError::NumericalInstability(String)` - 数値不安定性
  - `PricingError::UnsupportedInstrument(String)` - 未対応商品
  - `thiserror` を使用した derive マクロパターン
- **影響**: `AnalyticalError` を新規作成し、`From<AnalyticalError> for PricingError` を実装

### AD 互換性要件

- **Context**: Enzyme AD との互換性確保
- **調査対象**: 既存の AD 互換コードパターン
- **発見事項**:
  - `T: Float` ジェネリクスを使用
  - `if` 条件分岐は Float 値に対して使用禁止
  - `smooth_indicator` を分岐の代わりに使用
  - `exp`, `ln`, `sqrt` 等の基本演算は AD 互換
- **影響**: Black-Scholes の d1/d2 計算および Greeks 計算で条件分岐を回避

## Architecture Pattern Evaluation

| オプション | 説明 | 強み | リスク/制限 | 備考 |
|-----------|------|------|------------|------|
| モジュール内クロージャ | 各モデルを独立した struct として実装 | シンプル、テスト容易 | 共通ロジック重複 | 採用 |
| トレイトベース抽象化 | `AnalyticalModel` トレイトを定義 | 拡張性高い | over-engineering の恐れ | 将来の検討事項 |
| enum ベース dispatch | `AnalyticalModelEnum` で統合 | Enzyme 最適化 | 現時点では過剰 | Phase 3+ で検討 |

## Design Decisions

### Decision: norm_cdf/norm_pdf の配置

- **Context**: 累積正規分布関数をどこに配置するか
- **検討した選択肢**:
  1. `pricer_core::math::distributions` - L1 に新規モジュール
  2. `pricer_models::analytical::distributions` - L2 の analytical 内
- **選択したアプローチ**: Option 2 - `pricer_models::analytical` 内に配置
- **根拠**:
  - `pricer_core` は既に math/smoothing, math/interpolators, math/solvers を持つ
  - 正規分布関数は analytical pricing 専用であり、L2 に配置が妥当
  - 将来的に L1 に移動可能（下位互換性を保持）
- **トレードオフ**: L1 に配置すれば他クレートから再利用可能だが、現時点では不要
- **フォローアップ**: 他モジュールから需要が出た場合に L1 へ移動を検討

### Decision: AnalyticalError の設計

- **Context**: analytical モジュール専用のエラー型
- **検討した選択肢**:
  1. `PricingError` を直接使用
  2. 新規 `AnalyticalError` を作成し `PricingError` に変換
- **選択したアプローチ**: Option 2 - 専用エラー型
- **根拠**:
  - より具体的なエラーメッセージ（`InvalidVolatility`, `InvalidSpot`）を提供
  - `PricingError` への `From` 実装で互換性維持
  - 既存パターン（`InstrumentError`, `MarketDataError`）に従う
- **トレードオフ**: 型が増えるが、エラーハンドリングが明確化

### Decision: Greeks 計算の実装方式

- **Context**: デルタ、ガンマ、ベガ等の感度計算
- **検討した選択肢**:
  1. 解析的公式のみ（手計算の導関数）
  2. AD による自動微分（Dual 数を使用）
  3. 両方をサポート
- **選択したアプローチ**: Option 1 - 解析的公式
- **根拠**:
  - Black-Scholes Greeks には閉形式解が存在
  - 解析的計算は AD より高速
  - AD 互換性はジェネリクスで確保（将来の AD ベース検証に対応）
- **フォローアップ**: Requirement 5.5 で AD vs 解析的 Greeks の比較テストを追加

## Risks & Mitigations

- **Risk 1**: d1/d2 計算で expiry ≈ 0 のとき数値不安定
  - **Mitigation**: expiry が非常に小さい場合は intrinsic value を返す（Requirement 1.5）
- **Risk 2**: norm_cdf の極端な入力 (|x| > 8) での精度
  - **Mitigation**: erfc ベース実装で数値安定性を確保（Requirement 4.5）
- **Risk 3**: Bachelier モデルでの負のフォワード価格
  - **Mitigation**: 負のフォワードを明示的にサポート（Requirement 2.4）

## References

- [Black-Scholes Model - Wikipedia](https://en.wikipedia.org/wiki/Black%E2%80%93Scholes_model) - 公式の参照
- [Bachelier Model - Wikipedia](https://en.wikipedia.org/wiki/Bachelier_model) - 正規モデルの参照
- [Abramowitz and Stegun, 7.1.26](https://mathworld.wolfram.com/NormalDistribution.html) - norm_cdf 近似式
- [num-traits Float trait](https://docs.rs/num-traits/latest/num_traits/float/trait.Float.html) - erfc メソッド

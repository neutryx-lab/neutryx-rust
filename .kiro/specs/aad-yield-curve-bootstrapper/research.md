# Research & Design Decisions

## Summary

- **Feature**: `aad-yield-curve-bootstrapper`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - 既存の `pricer_optimiser::bootstrapping` モジュールを拡張する形で実装
  - `pricer_core` に Newton-Raphson/Brent ソルバーと YieldCurve トレイトが既に存在
  - 陰関数定理による AAD 実装は Enzyme の `#[autodiff]` マクロとの統合が必要

## Research Log

### 既存カーブインフラストラクチャの調査

- **Context**: AAD ブートストラッパーの統合ポイントを特定するため
- **Sources Consulted**:
  - `crates/pricer_core/src/market_data/curves/traits.rs`
  - `crates/pricer_core/src/market_data/curves/interpolated.rs`
- **Findings**:
  - `YieldCurve<T: Float>` トレイトが `discount_factor`, `zero_rate`, `forward_rate` を定義
  - `InterpolatedCurve<T>` が Linear/LogLinear 補間をサポート
  - 全カーブ構造が `T: Float` ジェネリックで AD 互換
- **Implications**: 新しいブートストラップカーブは既存トレイトを実装し、ジェネリック性を維持する必要がある

### 既存ブートストラップモジュールの調査

- **Context**: 拡張ポイントと既存実装の制約を理解するため
- **Sources Consulted**:
  - `crates/pricer_optimiser/src/bootstrapping/mod.rs`
  - `crates/pricer_optimiser/src/bootstrapping/curve_builder.rs`
- **Findings**:
  - `CurveBootstrapper` が基本的な逐次ストリッピングを実装（f64 のみ）
  - `BootstrapConfig` に tolerance, max_iterations, interpolation が設定可能
  - `BootstrapResult` が discount_factors, pillars, residual を返却
  - 現在の実装は単純なスワップレートのみ対応、OIS/FRA/先物未対応
- **Implications**: ジェネリック化と市場商品タイプの拡張が必要

### 既存ソルバーの調査

- **Context**: AAD 対応ソルバーの設計方針を決定するため
- **Sources Consulted**:
  - `crates/pricer_core/src/math/solvers/newton_raphson.rs`
  - `crates/pricer_core/src/math/solvers/brent.rs`
- **Findings**:
  - `NewtonRaphsonSolver<T>` が `find_root` と `find_root_ad` (num-dual) を提供
  - `BrentSolver<T>` がブラケット法によるロバストな求解を提供
  - 両ソルバーが `T: Float` ジェネリック
- **Implications**: 陰関数定理による AAD は新しいソルバーラッパーとして実装

### AAD と陰関数定理の調査

- **Context**: ソルバー反復をテープに記録しない AAD 実装方法を確立するため
- **Sources Consulted**:
  - Griewank & Walther, "Evaluating Derivatives" (2008)
  - QuantLib AAD implementation notes
- **Findings**:
  - 陰関数定理: $f(x^*, \theta) = 0$ を満たす $x^*$ に対し、$\frac{\partial x^*}{\partial \theta} = -(\frac{\partial f}{\partial x})^{-1} \frac{\partial f}{\partial \theta}$
  - 逆伝播時: $\bar{\theta} += \bar{x} \times (-\frac{\partial f / \partial \theta}{\partial f / \partial x})$
  - これにより反復ステップを記録せず、最終結果のみで勾配計算可能
- **Implications**: カスタム `AdjointSolver` 構造体で陰関数定理ロジックをカプセル化

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Extension Pattern** | 既存 `bootstrapping` モジュールを拡張 | 既存コードとの一貫性、再利用性 | 大規模な変更が必要 | **Selected** |
| Separate Module | 新規 `aad_bootstrapping` モジュール | 独立性、影響範囲の限定 | コード重複、統合困難 | Rejected |
| Trait-based Abstraction | `Bootstrapper<T>` トレイト定義 | 柔軟性、テスト容易性 | 複雑性増加 | 部分採用 |

## Design Decisions

### Decision: ジェネリック型パラメータの設計

- **Context**: AAD 互換性と既存インフラとの整合性
- **Alternatives Considered**:
  1. `f64` 固定 — シンプルだが AAD 非対応
  2. `T: Float` — AD 互換だが Enzyme 制約あり
  3. `T: Float + DualNum` — 完全 AD 対応だが制約が厳しい
- **Selected Approach**: `T: Float` を基本とし、AAD 計算は専用メソッドで提供
- **Rationale**: Enzyme は f64 レベルで動作し、num-dual は `T: Float` で動作する二重モード
- **Trade-offs**: Enzyme モードでは一部ジェネリック性が制限される
- **Follow-up**: フィーチャーフラグ `enzyme-mode` と `num-dual-mode` で切り替え

### Decision: 市場商品の表現

- **Context**: OIS, IRS, FRA, 先物を統一的に扱う必要
- **Alternatives Considered**:
  1. 単一 `MarketInstrument` enum — 静的ディスパッチ、Enzyme 最適化
  2. `MarketInstrument` トレイト — 柔軟だが動的ディスパッチ
  3. 商品タイプ別構造体 — 型安全だが冗長
- **Selected Approach**: `BootstrapInstrument<T>` enum による静的ディスパッチ
- **Rationale**: A-I-P-S アーキテクチャの静的ディスパッチ原則に準拠
- **Trade-offs**: 新商品追加時に enum 拡張が必要
- **Follow-up**: フィーチャーフラグで商品タイプを制御

### Decision: 陰関数定理 AAD の実装位置

- **Context**: ソルバー反復をテープに記録しない AAD 計算
- **Alternatives Considered**:
  1. `pricer_core::solvers` に追加 — 汎用性高いが L1 への依存追加
  2. `pricer_optimiser::bootstrapping` 内 — ブートストラップ専用
  3. `pricer_pricing::enzyme` 経由 — Enzyme 統合に最適
- **Selected Approach**: `pricer_optimiser::bootstrapping` 内に `AdjointSolver` を実装
- **Rationale**: ブートストラップ固有のロジックを L2.5 に閉じ込める
- **Trade-offs**: 他の逆問題で再利用が困難
- **Follow-up**: 汎用化が必要な場合は `pricer_core` へ移動を検討

### Decision: 補間方式の拡張

- **Context**: 要件で Monotonic Cubic と Flat Forward が追加要求
- **Alternatives Considered**:
  1. `InterpolatedCurve` を拡張 — 既存コード修正
  2. 新しい `BootstrappedCurve` 型 — 独立性高い
  3. `InterpolationMethod` enum 拡張 — 最小変更
- **Selected Approach**: `pricer_optimiser` 内に `BootstrappedCurve<T>` を新設、`pricer_core` の Interpolator を再利用
- **Rationale**: L2.5 でブートストラップ固有のロジックをカプセル化、L1 の変更を最小化
- **Trade-offs**: 一部コード重複の可能性
- **Follow-up**: 共通補間ロジックを L1 に昇格させる場合は別 PR で対応

## Risks & Mitigations

- **Risk 1**: Enzyme AD と陰関数定理の統合が複雑 — Mitigation: num-dual モードで先行実装し検証
- **Risk 2**: マルチカーブ構築での依存関係エラー — Mitigation: CurveSet で依存順序を明示的に管理
- **Risk 3**: 負のレート環境での数値不安定性 — Mitigation: smooth_max/smooth_indicator でフォールバック処理

## References

- [Griewank & Walther, "Evaluating Derivatives" (2008)](https://epubs.siam.org/doi/book/10.1137/1.9780898717761) — 陰関数定理による AD の理論基盤
- [QuantLib YieldTermStructure](https://www.quantlib.org/docs/QuantLib-reference-manual/group__yieldtermstructures.html) — 業界標準のカーブ構築実装
- [Neutryx steering/tech.md](.kiro/steering/tech.md) — プロジェクトの技術スタック定義
- [Neutryx steering/structure.md](.kiro/steering/structure.md) — A-I-P-S アーキテクチャ定義

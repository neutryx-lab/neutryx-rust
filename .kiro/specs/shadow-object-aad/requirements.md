# Requirements Document

## Project Description (Input)
前回の「巨大な配列ですべてを管理する（Arena）」案は、確かにメモリ効率は最強ですが、実装と保守のコストが高すぎました。実務的な開発速度を損なっては本末転倒です。

ご要望の**「マーケットを配列に再配置する手間をなくす」**かつ**「ジェネリクス汚染を回避する」**ための、最もシンプルで強力なアプローチを提案します。

それは、**「シャドウ・オブジェクト（Shadow Object）」パターン**です。

---

### コンセプト：データ構造は「リッチ」に、カーネルは「プリミティブ」に

「マーケット全体を1つの配列にする」必要はありません。既存の `Vec<f64>`（例えば `YieldCurve` 内のレート配列）を**そのまま**使い、その参照（スライス）だけを計算カーネルに渡します。

Enzymeの強力な点は、**「ポインタ（参照）の先にあるデータ」も微分できる**ことです。

#### 新アーキテクチャの3つのルール

1. **データ構造は変えない**: `struct Market`, `struct YieldCurve` は今のままでOK。ジェネリクス `T` も不要です。
2. **Shadow（勾配）は `clone` で作る**: 計算開始時に、Market構造体と同じ形をした「勾配用構造体（全てゼロ）」を `clone` で作ります。
3. **カーネルは `&[f64]` を取る**: プライシング関数は、構造体そのものではなく、そこから取り出した `&[f64]`（スライス）を引数に取ります。

---

## Introduction

本仕様は、Neutryx derivatives pricing libraryにおけるEnzyme AAD（Automatic Adjoint Differentiation）統合のための「シャドウ・オブジェクト（Shadow Object）」パターンを定義する。

このパターンは以下の課題を解決する：
- **Arena方式の複雑性回避**: 巨大配列への詰め替え（Pack/Unpack）を不要にする
- **ジェネリクス汚染の防止**: 既存データ構造に型パラメータ `T` を追加しない
- **構造と勾配の一致**: 勾配オブジェクトが元のデータ構造と同一の型を持つ

**適用レイヤー**: pricer_risk (L4) の enzyme モジュール

---

## Requirements

### Requirement 1: Shadow Trait 定義

**Objective:** As a 開発者, I want 勾配オブジェクトを生成するための統一インターフェースを定義したい, so that 任意のマーケットデータ構造に対してゼロ初期化された勾配オブジェクトを作成できる

#### Acceptance Criteria

1. The Shadow module shall provide a `Shadow` trait that requires `Clone` bound on implementing types.
2. When `zero_out()` is called on a Shadow implementor, the Shadow module shall set all `f64` fields and `Vec<f64>` elements to `0.0`.
3. The Shadow module shall provide a `create_shadow()` method that clones the original object and calls `zero_out()`.
4. If a type implements `Shadow`, then the Shadow module shall guarantee that the shadow object has identical memory layout to the original.
5. The Shadow module shall support nested structures (structs containing other Shadow-implementing structs).

---

### Requirement 2: スライスベース・カーネル・インターフェース

**Objective:** As a 開発者, I want プライシング関数を `&[f64]` スライスを引数に取る形式で定義したい, so that Enzyme が直接ポインタを通じて微分を計算できる

#### Acceptance Criteria

1. The Kernel module shall define pricing functions that accept `&[f64]` slices for active inputs (微分対象).
2. The Kernel module shall define pricing functions that accept `&[f64]` slices for constant inputs (定数).
3. The Kernel module shall define pricing functions that write results to `&mut f64` output parameters.
4. When a kernel function is called, the Kernel module shall perform no heap allocation within the function body.
5. The Kernel module shall mark kernel functions with `#[no_mangle]` attribute for Enzyme visibility.
6. The Kernel module shall use only `f64` arithmetic operations (no Dual numbers or generic types).

---

### Requirement 3: AAD バインダー層

**Objective:** As a 開発者, I want Shadow オブジェクトとカーネル関数を接続するバインダー層を利用したい, so that 高レベルAPIから自動微分を実行できる

#### Acceptance Criteria

1. The Binder module shall accept market data structures (e.g., `YieldCurve`, `VolSurface`) and trade structures as input.
2. When `calculate_risk()` is called, the Binder module shall create a shadow object from the market data using the `Shadow` trait.
3. The Binder module shall extract `&[f64]` slices from market data structures and pass them to kernel functions.
4. The Binder module shall extract `&mut [f64]` slices from shadow objects for gradient accumulation.
5. When Enzyme autodiff completes, the Binder module shall return both the primal value (PV) and the shadow object containing gradients.
6. The Binder module shall use `ENZYME_DUP` flag for active (differentiable) inputs.
7. The Binder module shall use `ENZYME_CONST` flag for constant (non-differentiable) inputs.

---

### Requirement 4: ゼロコピー・データ受け渡し

**Objective:** As a パフォーマンス・エンジニア, I want カーネル呼び出し時にデータのコピーを発生させたくない, so that 大規模マーケットデータでも効率的に微分を計算できる

#### Acceptance Criteria

1. The Shadow Object AAD system shall pass pointers (`as_ptr()`, `as_mut_ptr()`) to kernel functions instead of copying data.
2. When extracting slices from market structures, the system shall use `&self.rates[..]` syntax for zero-copy access.
3. The system shall not require intermediate serialisation (Pack) of market data before kernel invocation.
4. The system shall not require deserialisation (Unpack) of gradients after kernel invocation.
5. While a kernel is executing, the system shall guarantee that source data remains valid and unmodified.

---

### Requirement 5: 型安全性とジェネリクス回避

**Objective:** As a コードベース・メンテナー, I want 既存のデータ構造にジェネリクス型パラメータを追加することなくAADを実現したい, so that コンパイル時間の増加と型の複雑化を防げる

#### Acceptance Criteria

1. The Shadow Object AAD system shall not require type parameter `T` on market data structures (e.g., `YieldCurve<T>`).
2. The Shadow Object AAD system shall use concrete `f64` type throughout the pricing kernel.
3. The Shadow Object AAD system shall not cause monomorphisation explosion from generic type expansion.
4. If a developer adds a new market data type, then the system shall require only implementing the `Shadow` trait (no generic modifications).
5. The system shall maintain backward compatibility with existing non-AAD code paths.

---

### Requirement 6: 勾配マッピングの直感性

**Objective:** As a クオンツ開発者, I want 勾配の位置が元のデータ構造と一致していてほしい, so that デバッグとリスク分析が直感的に行える

#### Acceptance Criteria

1. When AAD completes, the shadow object shall have identical field structure to the original market data.
2. The gradient for `market.rates[i]` shall be located at `d_market.rates[i]` in the shadow object.
3. The system shall not require ID mapping or index translation to locate gradients.
4. The system shall support named field access for gradients (e.g., `d_market.volatility`, `d_market.discount_factors`).
5. Where multiple curves exist in market data, the system shall preserve curve identity in the shadow object.

---

### Requirement 7: 部分微分サポート

**Objective:** As a リスク・マネージャー, I want 特定のマーケット要素だけを微分対象としたい, so that Delta、Vega、Rho を個別に計算できる

#### Acceptance Criteria

1. The Binder module shall support marking specific market components as active (differentiable) or constant.
2. When only rates are marked as active, the system shall compute rate sensitivities (Delta/DV01) only.
3. When only volatilities are marked as active, the system shall compute volatility sensitivities (Vega) only.
4. The system shall support combining multiple active components in a single AAD pass.
5. If a component is marked as constant, then its shadow values shall remain zero after AAD execution.

---

### Requirement 8: 既存 pricer_risk 統合

**Objective:** As a システム・アーキテクト, I want Shadow Object AAD を既存の pricer_risk (L4) enzyme モジュールに統合したい, so that 既存のリスク計算ワークフローを拡張できる

#### Acceptance Criteria

1. The Shadow Object AAD implementation shall reside in `pricer_risk::enzyme` module.
2. The implementation shall be compatible with existing `enzyme-ad` feature flag.
3. The implementation shall integrate with existing `GreeksEnzyme` trait infrastructure.
4. When building without `enzyme-ad` feature, the system shall compile without Enzyme dependencies.
5. The implementation shall follow A-I-P-S dependency rules (L4 may depend on L1-L3, not on S or A layers).

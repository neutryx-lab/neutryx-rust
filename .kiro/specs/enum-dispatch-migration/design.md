# Technical Design: enum-dispatch-migration

## Overview

**Purpose**: 本設計は、Neutryx デリバティブ価格計算ライブラリにおける `enum_dispatch` クレートの導入により、Enum-Trait 間のボイラープレートコードを排除する技術設計を定義する。

**Users**: クオンツ開発者およびライブラリメンテナが、新規バリアント追加時の手動 `match` 文実装を省略し、保守性を向上させる。

**Impact**: `pricer_models` および `pricer_pricing` クレートの4つの Enum に対し、手動トレイト実装を `enum_dispatch` マクロ生成コードに置換する。約139行のボイラープレートを削除し、Enzyme AD との互換性を維持する。

### Goals

- `enum_dispatch` クレートをワークスペースに導入し、4つの対象 Enum を移行する
- 既存の公開 API と動作を完全に維持する
- Enzyme AD との互換性を検証し、問題がある場合は該当 Enum を除外する
- コードベースの一貫性と品質基準を維持する

### Non-Goals

- `StochasticModelEnum` の移行（関連型制約のため技術的に不可能）
- 新規トレイトの設計または既存トレイトの変更
- パフォーマンス最適化（移行後の性能は同等を目標とする）

---

## Architecture

### Existing Architecture Analysis

現行アーキテクチャでは、Enum に対するトレイト実装が手動 `match` 文で行われている：

```text
pricer_models/src/market.rs
├── CurveEnum<T> impl YieldCurve<T>     → 3メソッド × 2バリアント = 6 match 文
├── FxCurveEnum<T> impl FxCurve<T>      → 3メソッド × 3バリアント = 9 match 文

pricer_pricing/src/methods/
├── path_dependent/payoff_type.rs
│   └── PathPayoffType<T> (inherent)    → 3メソッド × 4バリアント = 12 match 文
└── mc/workspace_enum.rs
    └── WorkspaceEnum impl PathWorkspaceTrait → 10+メソッド × 2バリアント = 20+ match 文
```

**保持すべきパターン**:
- A-I-P-S 依存方向（Pricer 層内で完結）
- ジェネリクス `<T: Float>` による型安全性
- Enzyme AD 互換の静的ディスパッチ

### Architecture Integration

- **選択パターン**: Macro-based trait forwarding（`enum_dispatch` による静的ディスパッチ）
- **ドメイン境界**: 各クレート内で独立した移行（`pricer_models` と `pricer_pricing`）
- **既存パターン保持**: ジェネリクス `<T: Float>`、Enzyme AD 互換性
- **Steering 準拠**: 静的ディスパッチ優先、A-I-P-S 依存方向維持

---

## Requirements Traceability

| Requirement | Summary | Components |
|-------------|---------|------------|
| 1.1-1.4 | 依存関係追加 | Cargo.toml |
| 2.1-2.3 | 対象 Enum 識別 | - |
| 3.1-3.5 | StochasticModelEnum | **除外** |
| 4.1-4.5 | CurveEnum 移行 | market.rs |
| 5.1-5.4 | PathPayoffType 移行 | payoff_type.rs |
| 6.1-6.4 | Enzyme AD 検証 | 全対象 |
| 7.1-7.5 | コード品質 | 全ファイル |
| 8.1-8.4 | 後方互換性 | 公開 API |

**Requirement 3（StochasticModelEnum）の除外**:
Gap Analysis により、`StochasticModel` トレイトが関連型（`State`, `Params`）を持つことが判明。`enum_dispatch` は関連型をサポートしないため、要件3は技術的に実現不可能。

---

## Components and Interfaces

### Summary Table

| Component | Domain/Layer | Intent | Req Coverage |
|-----------|--------------|--------|--------------|
| Workspace Cargo.toml | Build | enum_dispatch 依存追加 | 1.1-1.4 |
| YieldCurve trait | pricer_models/market | カーブトレイトにマクロ適用 | 4.1-4.2 |
| CurveEnum | pricer_models/market | enum_dispatch 属性付与 | 4.2-4.5 |
| FxCurve trait | pricer_models/market | FXカーブトレイトにマクロ適用 | 4.1-4.2 |
| FxCurveEnum | pricer_models/market | enum_dispatch 属性付与 | 4.2-4.5 |
| PathDependentPayoff trait | pricer_pricing/path_dependent | ペイオフトレイトにマクロ適用 | 5.1 |
| PathPayoffType | pricer_pricing/path_dependent | enum_dispatch 属性付与 | 5.2-5.4 |
| PathWorkspaceTrait | pricer_pricing/mc | ワークスペーストレイトにマクロ適用 | - |
| WorkspaceEnum | pricer_pricing/mc | enum_dispatch 属性付与 | - |

---

### pricer_models Layer

#### YieldCurve<T> Trait（移行対象）

**Before Migration**:
```rust
pub trait YieldCurve<T: Float> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError>;
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError>;
}
```

**After Migration**:
```rust
#[enum_dispatch]
pub trait YieldCurve<T: Float> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError>;
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError>;
}
```

---

#### CurveEnum<T>（移行対象）

**After Migration**:
```rust
#[enum_dispatch(YieldCurve<T>)]
pub enum CurveEnum<T: Float> {
    Flat(curves::FlatCurve<T>),
    Bootstrapped(curves::BootstrappedCurve<T>),
}
// impl YieldCurve for CurveEnum は自動生成（削除）
```

**Dependencies**:
- Outbound: curves::FlatCurve, curves::BootstrappedCurve — バリアント型 (P0)

---

### pricer_pricing Layer

#### PathDependentPayoff<T> Trait（移行対象）

**After Migration**:
```rust
#[enum_dispatch]
pub trait PathDependentPayoff<T: Float>: Send + Sync {
    fn compute(&self, path: &[T], observer: &PathObserver<T>) -> T;
    fn required_observations(&self) -> ObservationType;
    fn smoothing_epsilon(&self) -> T;
}
```

---

#### PathPayoffType<T>（移行対象）

**After Migration**:
```rust
#[enum_dispatch(PathDependentPayoff<T>)]
pub enum PathPayoffType<T: Float> {
    AsianArithmetic(AsianArithmeticPayoff<T>),
    AsianGeometric(AsianGeometricPayoff<T>),
    Barrier(BarrierPayoff<T>),
    Lookback(LookbackPayoff<T>),
}
// inherent methods を trait methods に変換
// is_asian(), is_barrier(), is_lookback() は inherent のまま維持
```

---

#### WorkspaceEnum（移行対象）

**After Migration**:
```rust
#[enum_dispatch(PathWorkspaceTrait)]
pub enum WorkspaceEnum {
    PathFirst(PathWorkspace),
    TimeStepFirst(TimeStepFirstWorkspace),
}
// 70+行の手動 impl 削除
```

---

## Data Models

本フィーチャーではデータモデルの変更は発生しない。既存の型定義はすべて維持される。

---

## Error Handling

### Error Strategy

移行後のエラー処理は既存と同一。`enum_dispatch` マクロはエラー型を変更しない。

### Compile-time Errors

`enum_dispatch` マクロは以下のコンパイルエラーを生成する可能性がある：

| エラー | 原因 | 対処 |
|-------|------|------|
| `trait not found` | トレイト定義が `#[enum_dispatch]` より後に出現 | トレイト定義を先に記述 |
| `variant type doesn't implement trait` | バリアント型がトレイトを未実装 | バリアント型に `impl Trait` を追加 |
| `associated types not supported` | トレイトに関連型が存在 | 移行対象から除外（StochasticModel） |

---

## Testing Strategy

### Unit Tests

1. **YieldCurve dispatch tests** - `CurveEnum::Flat` と `CurveEnum::Bootstrapped` の全メソッド呼び出し
2. **FxCurve dispatch tests** - `FxCurveEnum` 全バリアントのメソッド呼び出し
3. **PathPayoffType dispatch tests** - 各ペイオフタイプの `compute` 結果が移行前と同一
4. **WorkspaceEnum dispatch tests** - 全メソッドのバリアント間動作一貫性

### Integration Tests

1. **Bootstrapping integration** - `CurveEnum` を使用したカーブブートストラップ
2. **Monte Carlo path pricing** - `PathPayoffType` を使用したパス依存オプション価格計算
3. **Workspace layout switching** - `WorkspaceEnum` による PathFirst/TimeStepFirst 切り替え

### Enzyme AD Verification

1. **Nightly build verification** - `cargo +nightly build -p pricer_pricing --features all`
2. **AD correctness test** - `PathPayoffType` を使用した Greeks 計算結果を bump-and-revalue と比較
3. **Performance benchmark** - 移行前後の Monte Carlo 価格計算速度比較

# Design Document: enzyme-autodiff-integration

## Overview

**Purpose**: Enzyme LLVM-level ADの `#[autodiff]` マクロを pricer_pricing クレートに本格統合し、高性能Greeks計算を実現する。

**Users**: クオンツ開発者、デリバティブトレーダー、リスク管理チームが Monte Carlo シミュレーションでのリアルタイム Greeks 計算に使用する。

**Impact**: 現在の有限差分近似（Phase 3.0 placeholder）を Enzyme 自動微分に置き換え、5倍以上の高速化を実現。

### Goals

- Enzyme `#[autodiff_forward]` / `#[autodiff_reverse]` マクロの本格統合
- `MonteCarloPricer::price_with_greeks()` での Enzyme AD 利用
- num-dual / 有限差分との検証による正確性保証
- stable Rust との互換性維持（graceful degradation）

### Non-Goals

- 他クレート（pricer_core, pricer_models等）への Enzyme 依存導入
- カスタム LLVM パス開発
- GPU/CUDA 対応（将来検討）

---

## Architecture

### Existing Architecture Analysis

**現在の構造**:
- `enzyme/mod.rs`: `ADMode`, `Activity` enum、`gradient()` 有限差分 placeholder
- `mc/pricer.rs`: `price_with_greeks()` bump-and-revalue 実装
- `greeks/`: `GreeksConfig`, `GreeksMode`, `GreeksResult<T>`
- Feature flags: `enzyme-ad` (llvm-sys), `l1l2-integration`

**制約**:
- pricer_pricing は L3 として他 Pricer クレートから独立
- nightly-2025-01-15 ツールチェーン使用
- 静的ディスパッチ（enum）パターン維持

### Architecture Integration

- Selected pattern: Hybrid（既存拡張 + 新規サブモジュール）
- Domain boundaries: enzyme/ モジュール内で AD ロジックを完結
- Existing patterns preserved: `ADMode`, `Activity`, `GreeksMode` enum
- New components rationale: forward.rs/reverse.rs で Forward/Reverse mode を分離
- Steering compliance: L3 独立性維持、static dispatch 継続

---

## Requirements Traceability

| Requirement | Summary | Components |
|-------------|---------|------------|
| 1.1-1.4 | Nightly feature 有効化 | lib.rs, Cargo.toml |
| 2.1-2.5 | Forward mode AD | enzyme/forward.rs, wrappers.rs |
| 3.1-3.5 | Reverse mode AD | enzyme/reverse.rs, wrappers.rs |
| 4.1-4.6 | MC Pricer 統合 | mc/greeks_enzyme.rs |
| 5.1-5.5 | 検証・正確性 | verify_enzyme.rs |
| 6.1-6.5 | Enzyme 対応関数 | enzyme/smooth.rs |
| 7.1-7.5 | パフォーマンス | benches/enzyme_bench.rs |
| 8.1-8.5 | CI 統合 | .github/workflows/ci.yml |

---

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage |
|-----------|--------------|--------|--------------|
| enzyme/wrappers.rs | L3/AD | Enzyme マクロラッパー | 2.1-2.2, 3.1-3.2 |
| enzyme/forward.rs | L3/AD | Forward mode 型 | 2.3-2.5 |
| enzyme/reverse.rs | L3/AD | Reverse mode 型, GammaAD | 3.3-3.5, 4.2 |
| mc/greeks_enzyme.rs | L3/MC | Greeks 計算統合 + チェックポイント | 4.1-4.6 |
| verify_enzyme.rs | L3/Test | AD 検証 | 5.1-5.5 |

### L3/AD Layer

#### enzyme/wrappers.rs

**Responsibilities & Constraints**
- `#[autodiff_forward]` / `#[autodiff_reverse]` マクロ適用
- Activity annotation による微分対象指定
- 有限差分 fallback（enzyme-ad feature 無効時）

**Dependencies**
- External: Enzyme LLVM plugin — AD 実行 (P0)

**Service Interface**

```rust
/// Forward mode AD wrapper
#[cfg(feature = "enzyme-ad")]
#[autodiff_forward(d_price_spot, Dual, Const, Const, Const, Dual)]
pub fn price_european_primal(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    time: f64,
) -> f64;

/// Reverse mode AD wrapper
#[cfg(feature = "enzyme-ad")]
#[autodiff_reverse(d_price_all, Active, Duplicated, Const, Duplicated, Duplicated, Duplicated)]
pub fn price_european_adjoint(
    spot: &f64,
    d_spot: &mut f64,
    strike: f64,
    rate: &f64,
    d_rate: &mut f64,
    vol: &f64,
    d_vol: &mut f64,
    time: &f64,
    d_time: &mut f64,
) -> f64;

/// Fallback implementation (enzyme-ad disabled)
#[cfg(not(feature = "enzyme-ad"))]
pub fn price_european_delta_fd(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    time: f64,
    bump: f64,
) -> f64;
```

---

#### enzyme/forward.rs

**Service Interface**

```rust
/// Forward mode AD result
pub struct ForwardAD<T> {
    pub value: T,
    pub tangent: T,
}

impl<T: Float> ForwardAD<T> {
    pub fn constant(value: T) -> Self;
    pub fn variable(value: T, tangent: T) -> Self;
    pub fn derivative(&self) -> T;
}

pub fn compute_delta_forward(
    gbm: &GbmParams,
    payoff: &PayoffParams,
) -> ForwardAD<f64>;
```

---

#### enzyme/reverse.rs

**Service Interface**

```rust
/// Reverse mode AD gradients (first-order)
pub struct ReverseAD<T> {
    pub value: T,
    pub d_spot: T,
    pub d_rate: T,
    pub d_vol: T,
    pub d_time: T,
}

/// Second-order derivative result (Gamma)
pub struct GammaAD<T> {
    pub delta: T,
    pub gamma: T,
}

impl<T: Float> ReverseAD<T> {
    pub fn compute_all_greeks(
        gbm: &GbmParams,
        payoff: &PayoffParams,
    ) -> Self;
    pub fn to_greeks_result(&self) -> GreeksResult<T>;
}

impl<T: Float> GammaAD<T> {
    pub fn compute_gamma(
        gbm: &GbmParams,
        payoff: &PayoffParams,
    ) -> Self;
}
```

---

### L3/MC Layer

#### mc/greeks_enzyme.rs

**Responsibilities & Constraints**
- `price_with_greeks()` 内部での Enzyme 呼び出し
- 並列シミュレーション時の adjoint 集約
- path-dependent オプションでの Enzyme 自動チェックポイント活用

##### Checkpointing Strategy

- Enzyme 内部自動チェックポイント機能を使用（LLVM レベルで最適化）
- 既存 `checkpoint/` モジュールは Enzyme 外部の状態保存用途に限定
- path 長 1000+ での メモリ使用量監視が必要

**Service Interface**

```rust
/// Enzyme-based Greeks calculation trait
pub trait GreeksEnzyme {
    fn compute_greeks_enzyme(
        &self,
        gbm: &GbmParams,
        payoff: &PayoffParams,
        greeks: &[Greek],
    ) -> GreeksResult<f64>;
}

/// Thread-local adjoint accumulator
pub struct AdjointAccumulator {
    pub d_spot: f64,
    pub d_vol: f64,
    pub d_rate: f64,
    pub d_time: f64,
    pub count: usize,
}

impl AdjointAccumulator {
    pub fn new() -> Self;
    pub fn accumulate(&mut self, adj: &ReverseAD<f64>);
    pub fn reduce(self, other: Self) -> Self;
}
```

---

## Data Models

**Aggregates**:
- `GreeksResult<T>`: Greeks 計算結果の集約ルート
- `ForwardAD<T>`: Forward mode 計算の値オブジェクト
- `ReverseAD<T>`: Reverse mode 計算の値オブジェクト（一次微分）
- `GammaAD<T>`: Nested AD による二次微分（Gamma）の値オブジェクト

**Business Rules**:
- Greeks 計算は常に数値的に安定した結果を返す
- NaN/Inf は計算エラーとして扱う
- fallback mode は enzyme-ad 無効時に自動適用

---

## Error Handling

### Error Strategy

- **Enzyme 未インストール**: ビルド時エラー（feature gate 未満足）
- **数値エラー (NaN/Inf)**: `PricingError::NumericalInstability`
- **Activity ミスマッチ**: コンパイル時エラー（マクロ展開）

---

## Testing Strategy

### Unit Tests

- `enzyme/wrappers.rs`: 各 `#[autodiff_*]` 関数の勾配検証
- `enzyme/forward.rs`: `ForwardAD<T>` 演算テスト
- `enzyme/reverse.rs`: `ReverseAD<T>` 全勾配テスト
- `mc/greeks_enzyme.rs`: MC + Enzyme 統合テスト

### Integration Tests

- Enzyme vs num-dual 比較（相対誤差 1e-6 以内）
- Enzyme vs 有限差分比較
- Black-Scholes analytical Greeks 比較
- 並列実行時の結果一貫性

### Performance Tests

- `criterion` ベンチマーク: Enzyme vs 有限差分速度比較
- `iai-callgrind`: 命令数ベース回帰検出
- 10,000 パス MC: 100ms 以内完了確認

# Gap Analysis: enum-dispatch-migration

## 概要

本ドキュメントは、`enum_dispatch` クレート導入に向けた実装ギャップ分析の結果をまとめたものです。既存コードベースの調査と `enum_dispatch` の技術的制約を評価し、移行の実現可能性と推奨アプローチを提示します。

## 1. 現状調査

### 1.1 移行候補 Enum の発見

コードベース調査により、以下の Enum が移行候補として特定されました：

| Enum | クレート | トレイト | 手動 match 行数 | 優先度 |
|------|---------|---------|----------------|--------|
| `CurveEnum<T>` | `pricer_models` | `YieldCurve<T>` | ~21行 | 高 |
| `FxCurveEnum<T>` | `pricer_models` | `FxCurve<T>` | ~21行 | 高 |
| `PathPayoffType<T>` | `pricer_pricing` | `PathDependentPayoff<T>` | ~27行 | 高 |
| `WorkspaceEnum` | `pricer_pricing` | `PathWorkspaceTrait` | ~70行 | 中 |
| `StochasticModelEnum<T>` | `pricer_models` | (inherent methods) | ~100行 | **除外** |

### 1.2 移行候補詳細

#### CurveEnum<T> (pricer_models/src/market.rs:420)

```rust
pub enum CurveEnum<T: Float> {
    Flat(curves::FlatCurve<T>),
    Bootstrapped(curves::BootstrappedCurve<T>),
}

impl<T: Float> curves::YieldCurve<T> for CurveEnum<T> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        match self { /* 各バリアントに転送 */ }
    }
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> { ... }
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> { ... }
}
```

**特徴**: 2バリアント、3メソッド、`YieldCurve<T>` トレイト実装
**移行可能性**: ✅ 高（関連型なし、ジェネリクスサポート）

#### FxCurveEnum<T> (pricer_models/src/market.rs:714)

```rust
pub enum FxCurveEnum<T: Float> {
    Flat(fx_curves::FlatFxCurve<T>),
    IrpFlat(fx_curves::IrpFxCurve<T, ...>),
    IrpGeneric(fx_curves::IrpFxCurve<T, CurveEnum<T>, CurveEnum<T>>),
}
```

**特徴**: 3バリアント、`FxCurve<T>` トレイト実装
**移行可能性**: ✅ 高

#### PathPayoffType<T> (pricer_pricing/src/methods/path_dependent/payoff_type.rs)

```rust
pub enum PathPayoffType<T: Float> {
    AsianArithmetic(AsianArithmeticPayoff<T>),
    AsianGeometric(AsianGeometricPayoff<T>),
    Barrier(BarrierPayoff<T>),
    Lookback(LookbackPayoff<T>),
}
```

**特徴**: 4バリアント、`compute()`, `required_observations()`, `smoothing_epsilon()` メソッド
**移行可能性**: ✅ 高（`PathDependentPayoff` トレイト使用可能）

#### WorkspaceEnum (pricer_pricing/src/methods/mc/workspace_enum.rs)

```rust
pub enum WorkspaceEnum {
    PathFirst(PathWorkspace),
    TimeStepFirst(TimeStepFirstWorkspace),
}

impl PathWorkspaceTrait for WorkspaceEnum { /* 10+ メソッド */ }
```

**特徴**: 2バリアント、10以上のメソッド、ジェネリクスなし
**移行可能性**: ✅ 高

### 1.3 除外候補: StochasticModelEnum

```rust
pub trait StochasticModel<T: Float>: Differentiable {
    type State: StochasticState<T>;  // 関連型
    type Params: Clone;               // 関連型
    // ...
}
```

**除外理由**: `enum_dispatch` は**関連型 (associated types) をサポートしていない**。`StochasticModel` トレイトは `State` と `Params` の関連型を持つため、移行不可能。

現在の `StochasticModelEnum` は inherent methods（`model_name()`, `brownian_dim()` 等）を使用しており、トレイト実装ではないため、`enum_dispatch` の対象外。

### 1.4 追加の制約事項

#### cfg 属性との互換性

`StochasticModelEnum` は `#[cfg(feature = "...")]` 属性をバリアントに使用：

```rust
#[cfg(feature = "equity")]
GBM(GBMModel<T>),
#[cfg(feature = "rates")]
HullWhite(HullWhiteModel<T>),
```

`enum_dispatch` は cfg 属性を**サポートしている**（バリアントレベルで条件付きコンパイル可能）。

#### 同一クレート制約

`enum_dispatch` は**トレイトと Enum が同一クレート内に存在する必要がある**。調査した全ての候補はこの条件を満たしている。

## 2. 技術的要件分析

### 2.1 enum_dispatch の制約

| 制約 | 影響 | 対応策 |
|-----|------|-------|
| 関連型非サポート | `StochasticModel` トレイト移行不可 | 除外、手動実装維持 |
| 同一クレート必須 | 候補は全て条件を満たす | なし |
| ジェネリクス構文 | 特殊構文が必要 | ドキュメント整備 |
| std トレイト非サポート | 該当なし | - |
| IDE サポート不良 | 開発体験への影響 | ドキュメント整備 |

### 2.2 削減可能なボイラープレート量

| Enum | 現在の match 行数 | enum_dispatch 後 | 削減率 |
|------|------------------|------------------|--------|
| `CurveEnum<T>` | ~21行 | 0行 | 100% |
| `FxCurveEnum<T>` | ~21行 | 0行 | 100% |
| `PathPayoffType<T>` | ~27行 | 0行 | 100% |
| `WorkspaceEnum` | ~70行 | 0行 | 100% |
| **合計** | **~139行** | **0行** | **100%** |

### 2.3 Enzyme AD 互換性

調査が必要な項目：
- `enum_dispatch` マクロ展開後のコードが Enzyme LLVM プラグインで正しく微分可能か
- ベンチマークでの性能比較

**リスク評価**: 中（検証が必要だが、静的ディスパッチのため理論的には互換）

## 3. 実装アプローチオプション

### Option A: 段階的移行（推奨）

**説明**: 低リスクの Enum から順次移行し、各段階でテストと検証を実施

**対象と順序**:
1. `CurveEnum` → `YieldCurve` トレイト（最小バリアント数、シンプル）
2. `FxCurveEnum` → `FxCurve` トレイト（`CurveEnum` と類似）
3. `WorkspaceEnum` → `PathWorkspaceTrait`（ジェネリクスなし）
4. `PathPayoffType` → `PathDependentPayoff` トレイト

**トレードオフ**:
- ✅ リスク分散、問題発生時の切り分け容易
- ✅ 各段階でテスト・検証可能
- ❌ 全体完了まで時間がかかる

### Option B: 一括移行

**説明**: 全対象 Enum を一度に移行

**トレードオフ**:
- ✅ 一貫性のある変更
- ✅ 実装期間が短い
- ❌ 問題発生時の影響範囲が大きい
- ❌ Enzyme AD との互換性問題が発覚した場合のロールバックが複雑

### Option C: 選択的移行

**説明**: Enzyme AD 互換性が確実な Enum のみ移行、不確実なものは除外

**対象**: `CurveEnum`, `FxCurveEnum` のみ（Enzyme AD と直接関わらない）
**除外**: `PathPayoffType`, `WorkspaceEnum`（Monte Carlo 経路で使用）

**トレードオフ**:
- ✅ 最低リスク
- ❌ 効果が限定的
- ❌ コードベースの一貫性低下

## 4. 工数・リスク評価

### 工数見積もり

| 作業項目 | 工数 |
|---------|------|
| 依存関係追加・設定 | S (1-3日) |
| CurveEnum 移行 | S |
| FxCurveEnum 移行 | S |
| WorkspaceEnum 移行 | S |
| PathPayoffType 移行 | S |
| Enzyme AD 検証 | M (3-7日) |
| ドキュメント整備 | S |
| **合計** | **M (3-7日)** |

### リスク評価

| リスク | 重大度 | 発生確率 | 対策 |
|-------|--------|---------|------|
| Enzyme AD 非互換 | 高 | 低 | 検証テスト実施、問題時は手動実装に戻す |
| IDE サポート不良 | 低 | 高 | ドキュメント整備 |
| コンパイル時間増加 | 低 | 中 | ベンチマーク測定 |
| 後方互換性問題 | 中 | 低 | API 変更なし設計 |

**総合リスク評価**: **低〜中**

## 5. 設計フェーズへの推奨事項

### 採用アプローチ

**Option A（段階的移行）を推奨**

### 優先移行対象

1. `CurveEnum<T>` - 最もシンプル、広範囲で使用
2. `FxCurveEnum<T>` - 同様の構造
3. `WorkspaceEnum` - ジェネリクスなし
4. `PathPayoffType<T>` - 最後に移行（Enzyme AD 検証後）

### 除外対象

- `StochasticModelEnum<T>` - 関連型制約のため移行不可

### 設計フェーズでの調査項目

1. **Enzyme AD 互換性検証**
   - enum_dispatch 展開後コードの微分可能性テスト
   - bump-and-revalue との結果比較

2. **ジェネリクス構文の最適化**
   - `#[enum_dispatch(YieldCurve<T>)]` の正確な構文確認
   - トレイト境界の扱い

3. **cfg 属性の動作確認**
   - feature flag 付きバリアントの動作テスト

## 6. 参考情報

### 調査ソース

- [enum_dispatch crate documentation](https://docs.rs/enum_dispatch/latest/enum_dispatch/)
- [enum_dispatch repository](https://gitlab.com/antonok/enum_dispatch)
- [Rust Internals: Built-in enum dispatch discussion](https://internals.rust-lang.org/t/built-in-enum-dispatch/18447)
- [Choosing the Right Dispatch Method Guide](https://iifx.dev/en/articles/460132105/choosing-the-right-dispatch-method-a-guide-to-rust-s-enum-dispatch)

### 代替ライブラリ

- `enum_delegate` - 関連型の部分的サポート、クロスクレート対応
- `declarative_enum_dispatch` - 宣言的マクロ版、cfg サポート

---

## 7. 設計フェーズ決定事項

### 7.1 採用アプローチ

**Option A（段階的移行）を採用**

段階的移行により、各フェーズでのテスト・検証を可能にし、問題発生時の影響範囲を限定する。

### 7.2 移行順序（確定）

| Phase | 対象 | クレート | 理由 |
|-------|------|---------|------|
| 1 | 依存関係追加 | workspace | 基盤整備 |
| 2 | `CurveEnum` | pricer_models | 最もシンプル、Enzyme 非依存 |
| 3 | `FxCurveEnum` | pricer_models | CurveEnum と類似構造 |
| 4 | `WorkspaceEnum` | pricer_pricing | ジェネリクスなし、テスト容易 |
| 5 | `PathPayoffType` | pricer_pricing | Enzyme AD 検証後に移行 |
| 6 | Enzyme AD 検証 | - | 最終検証 |
| 7 | 品質確認 | - | CI/CD パイプライン |

### 7.3 要件修正

**Requirement 3（StochasticModelEnum）を除外**

Gap Analysis で判明した技術的制約（関連型非サポート）により、Requirement 3 は実現不可能と判断。要件ドキュメントには除外の理由を明記し、将来的な `enum_delegate` 等の代替手段を検討する余地を残す。

### 7.4 アーキテクチャ決定

| 決定事項 | 選択 | 理由 |
|---------|------|------|
| マクロ適用順序 | トレイト先、Enum 後 | `enum_dispatch` の仕様要件 |
| ジェネリクス構文 | `#[enum_dispatch(Trait<T>)]` | 公式ドキュメント準拠 |
| inherent methods 維持 | `is_asian()` 等 | トレイト化不要、現行設計維持 |
| ロールバック戦略 | 手動 impl 復元 | 移行前コードを Git 履歴から復元可能 |

### 7.5 リスク軽減策

| リスク | 軽減策 |
|-------|-------|
| Enzyme AD 非互換 | Phase 5 後に検証、問題時は Phase 4,5 をロールバック |
| コンパイルエラー | 各 Phase で `cargo build --workspace` 実行 |
| テスト失敗 | 各 Phase で `cargo test --workspace` 実行 |
| 性能劣化 | `criterion` ベンチマークで移行前後を比較 |

---

*作成日: 2026-01-30*
*分析者: Claude*
*設計フェーズ更新: 2026-01-30*

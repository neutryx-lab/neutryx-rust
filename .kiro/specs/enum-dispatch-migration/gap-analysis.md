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

#### CurveEnum<T> (c:\Users\khosh\Codes\neutryx-rust\crates\pricer_models\src\market.rs:420)

**特徴**: 2バリアント、3メソッド、`YieldCurve<T>` トレイト実装
**移行可能性**: Complete（関連型なし、ジェネリクスサポート）

#### FxCurveEnum<T> (c:\Users\khosh\Codes\neutryx-rust\crates\pricer_models\src\market.rs:714)

**特徴**: 3バリアント、`FxCurve<T>` トレイト実装
**移行可能性**: Complete

#### PathPayoffType<T> (c:\Users\khosh\Codes\neutryx-rust\crates\pricer_pricing\src\methods\path_dependent\payoff_type.rs)

**特徴**: 4バリアント、`compute()`, `required_observations()`, `smoothing_epsilon()` メソッド
**移行可能性**: Complete（`PathDependentPayoff` トレイト使用可能）

#### WorkspaceEnum (c:\Users\khosh\Codes\neutryx-rust\crates\pricer_pricing\src\methods\mc\workspace_enum.rs)

**特徴**: 2バリアント、10以上のメソッド、ジェネリクスなし
**移行可能性**: Complete

### 1.3 除外候補: StochasticModelEnum

**除外理由**: `enum_dispatch` は **関連型 (associated types) をサポートしていない**。`StochasticModel` トレイトは `State` と `Params` の関連型を持つため、移行不可能。

現在の `StochasticModelEnum` は inherent methods（`model_name()`, `brownian_dim()` 等）を使用しており、トレイト実装ではないため、`enum_dispatch` の対象外。

### 1.4 追加の制約事項

#### cfg 属性との互換性

`enum_dispatch` は cfg 属性を **サポートしている**（バリアントレベルで条件付きコンパイル可能）。

#### 同一クレート制約

`enum_dispatch` は **トレイトと Enum が同一クレート内に存在する必要がある**。調査した全ての候補はこの条件を満たしている。

## 2. 技術的要件分析

### 2.1 enum_dispatch の制約

| 制約 | 影響 | 対応策 |
|-----|------|-------|
| 関連型非サポート | `StochasticModel` トレイト移行不可 | 除外、手動実装維持 |
| 同一クレート必須 | 候補は全て条件を満たす | なし |
| ジェネリクス構文 | 特殊構文が必要 | ドキュメント整備 |

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

**対象と順序**:
1. `CurveEnum` → `YieldCurve` トレイト（最小バリアント数、シンプル）
2. `FxCurveEnum` → `FxCurve` トレイト（`CurveEnum` と類似）
3. `WorkspaceEnum` → `PathWorkspaceTrait`（ジェネリクスなし）
4. `PathPayoffType` → `PathDependentPayoff` トレイト

**トレードオフ**:
- Complete: リスク分散、問題発生時の切り分け容易
- Complete: 各段階でテスト・検証可能
- Missing: 全体完了まで時間がかかる

### Option B: 一括移行

**トレードオフ**:
- Complete: 一貫性のある変更
- Missing: 問題発生時の影響範囲が大きい

### Option C: 選択的移行

**対象**: `CurveEnum`, `FxCurveEnum` のみ（Enzyme AD と直接関わらない）
**除外**: `PathPayoffType`, `WorkspaceEnum`（Monte Carlo 経路で使用）

## 4. 工数・リスク評価

### 工数見積もり

**合計**: **M (3-7日)**

### リスク評価

| リスク | 重大度 | 発生確率 | 対策 |
|-------|--------|---------|------|
| Enzyme AD 非互換 | 高 | 低 | 検証テスト実施、問題時は手動実装に戻す |
| IDE サポート不良 | 低 | 高 | ドキュメント整備 |
| コンパイル時間増加 | 低 | 中 | ベンチマーク測定 |

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

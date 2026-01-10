# Gap Analysis: enzyme-autodiff-integration

## Overview

本ドキュメントは、Enzyme `#[autodiff]` 本格統合に向けた既存コードベースとのギャップ分析結果である。

---

## 1. Current State Investigation

### 1.1 既存アセット

#### enzyme モジュール (`pricer_pricing/src/enzyme/mod.rs`)

| コンポーネント | 状態 | 備考 |
|--------------|------|------|
| `ADMode` enum | ✅ 存在 | Inactive/Forward/Reverse |
| `Activity` enum | ✅ 存在 | Const/Dual/Active/Duplicated/DuplicatedOnly |
| `gradient()` 関数 | ⚠️ Placeholder | 有限差分近似を使用 |
| `gradient_with_step()` | ⚠️ Placeholder | 有限差分近似を使用 |
| `#[autodiff]` マクロ | ❌ 未実装 | Phase 4 対象 |

#### MonteCarloPricer (`pricer_pricing/src/mc/pricer.rs`)

| コンポーネント | 状態 | 備考 |
|--------------|------|------|
| `Greek` enum | ✅ 完成 | Delta/Gamma/Vega/Theta/Rho/Vanna/Volga |
| `PricingResult` | ✅ 完成 | 全Greeks対応 |
| `price_with_greeks()` | ⚠️ Bump-and-revalue | Enzyme未統合 |
| `compute_delta()` 等 | ⚠️ 有限差分 | 各Greek個別計算 |

#### Greeks モジュール (`pricer_pricing/src/greeks/`)

| コンポーネント | 状態 | 備考 |
|--------------|------|------|
| `GreeksConfig` | ✅ 完成 | Builder pattern |
| `GreeksMode` | ✅ 完成 | BumpAndRevalue/AAD/NumDual |
| `GreeksResult<T>` | ✅ 完成 | Generic over T for AD |

#### Smoothing 関数 (`pricer_core/src/math/smoothing.rs`)

| 関数 | 状態 | 備考 |
|------|------|------|
| `smooth_max` | ✅ 完成 | LogSumExp 実装 |
| `smooth_min` | ✅ 完成 | dual of smooth_max |
| `smooth_indicator` | ✅ 完成 | Sigmoid 実装 |
| `smooth_abs` | ✅ 完成 | |
| `smooth_relu` | ✅ 完成 | |

#### ビルド・CI設定

| ファイル | 状態 | 備考 |
|----------|------|------|
| `rust-toolchain.toml` | ✅ 設定済み | nightly-2025-01-15 |
| `Cargo.toml` features | ✅ 設定済み | `enzyme-ad = ["dep:llvm-sys"]` |
| `Dockerfile.nightly` | ✅ 完成 | Enzyme v0.0.100 ビルド |
| `ci.yml` | ✅ 完成 | 別ジョブでnightly/Enzyme |

### 1.2 コンベンション・パターン

- **静的ディスパッチ**: `enum` ベース（`Box<dyn Trait>` 禁止）
- **Generic over T**: `T: Float` で AD 互換性確保
- **Feature flags**: `enzyme-ad`, `l1l2-integration`, `serde`
- **テスト配置**: 同一ファイル内 `#[cfg(test)]`
- **エラー型**: `thiserror` 使用

---

## 2. Requirements Feasibility Analysis

### 2.1 要件別ギャップマッピング

| Req | 要件 | 既存資産 | ギャップ | タグ |
|-----|------|----------|----------|------|
| R1 | Nightly feature有効化 | rust-toolchain.toml | `#![feature(autodiff)]` 未追加 | **Missing** |
| R2 | Forward mode AD | `ADMode::Forward` 存在 | `#[autodiff_forward]` マクロ未使用 | **Missing** |
| R3 | Reverse mode AD | `ADMode::Reverse` 存在 | `#[autodiff_reverse]` マクロ未使用 | **Missing** |
| R4 | MC Pricer統合 | `price_with_greeks()` 存在 | Enzyme呼び出し未実装 | **Missing** |
| R5 | 検証・正確性 | `verify_enzyme.rs` 存在 | num-dual比較不完全 | **Partial** |
| R6 | Enzyme対応関数 | smoothing関数存在 | payoff関数にsmooth未適用箇所あり | **Partial** |
| R7 | パフォーマンス | criterion/iai設定済み | Enzyme用ベンチマーク未作成 | **Missing** |
| R8 | ビルド・CI | ci.yml設定済み | `-Z autodiff=Enable` 設定済み | ✅ Ready |

### 2.2 技術的制約

1. **Enzyme Nightly依存**: `#[autodiff]` は `#![feature(autodiff)]` が必要
2. **LLVM 18必須**: Enzyme プラグインは LLVM 18 にコンパイル
3. **Static dispatch必須**: Enzyme は concrete types に最適化
4. **Smooth関数必須**: 不連続関数は微分不可

### 2.3 Unknown / Research Needed

| 項目 | 調査内容 | 優先度 |
|------|----------|--------|
| `#[autodiff]` API安定性 | Rust nightly での API 変更リスク | High |
| `ForwardAD<T>` 設計 | tangent/primal ペアの最適表現 | Medium |
| スレッド安全性 | 並列MC でのadjoint 集約方法 | Medium |
| チェックポイント統合 | 既存 `checkpoint/` との連携 | Low |

---

## 3. Implementation Approach Options

### Option A: Extend Existing Components

**対象**: enzyme モジュールを直接拡張

**変更ファイル**:
- `pricer_pricing/src/lib.rs` - `#![feature(autodiff)]` 追加
- `pricer_pricing/src/enzyme/mod.rs` - `#[autodiff_*]` マクロ追加
- `pricer_pricing/src/mc/pricer.rs` - Enzyme 呼び出し追加

**トレードオフ**:
- ✅ 最小限のファイル追加
- ✅ 既存テストを活用可能
- ❌ enzyme/mod.rs が肥大化する可能性
- ❌ Feature flag 分岐が複雑化

### Option B: Create New Components

**新規作成**:
```
enzyme/
├── mod.rs          (既存)
├── forward.rs      (NEW: ForwardAD<T> 型)
├── reverse.rs      (NEW: ReverseAD<T> 型)
├── wrappers.rs     (NEW: #[autodiff] ラッパー関数)
└── integration.rs  (NEW: MC Pricer 統合)
```

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ テストが書きやすい
- ✅ 将来の拡張に対応しやすい
- ❌ ファイル数増加
- ❌ インターフェース設計が必要

### Option C: Hybrid Approach (推奨)

**Phase 1: 既存拡張**
- enzyme/mod.rs に `#[autodiff]` 基本ラッパー追加
- lib.rs に feature gate 付き `#![feature(autodiff)]`

**Phase 2: 新規コンポーネント**
- 型安全ラッパー (`ForwardAD<T>`, `ReverseAD<T>`) を別ファイルに分離
- MC Pricer 統合ロジックを integration.rs に移動

**トレードオフ**:
- ✅ 段階的な実装でリスク軽減
- ✅ Phase 1 で早期検証可能
- ✅ 既存コードへの影響を最小化
- ❌ 計画が複雑

---

## 4. Implementation Complexity & Risk

### Effort Estimate

| フェーズ | 見積もり | 根拠 |
|----------|----------|------|
| Phase 1: 基本統合 | **M** (3-7 days) | `#[autodiff]` マクロ追加、基本テスト |
| Phase 2: MC Pricer統合 | **M** (3-7 days) | `price_with_greeks()` 書き換え、検証 |
| Phase 3: 検証・ベンチマーク | **S** (1-3 days) | num-dual比較、パフォーマンス測定 |
| **Total** | **L** (1-2 weeks) | |

### Risk Assessment

| リスク | レベル | 理由 | 軽減策 |
|--------|--------|------|--------|
| Enzyme API変更 | **High** | Nightly feature は変更リスク | CI で早期検出、fallback 実装維持 |
| パフォーマンス未達 | **Medium** | 5x高速化目標は挑戦的 | ベンチマーク駆動開発 |
| LLVM環境依存 | **Medium** | 開発環境セットアップが複雑 | Docker 環境提供 |
| スレッド安全性 | **Low** | 既存 rayon 統合パターンあり | 既存パターン踏襲 |

**総合リスク**: **Medium-High**

---

## 5. Recommendations for Design Phase

### 推奨アプローチ

**Option C (Hybrid)** を推奨。

1. Phase 1 で `#[autodiff]` 基本動作を検証
2. Phase 2 で実プライシングへ統合
3. 有限差分 fallback を維持し、graceful degradation を保証

### 設計フェーズでの調査項目

1. **`#[autodiff]` マクロのシグネチャ**: Rust nightly の最新 API を確認
2. **ForwardAD<T> 設計**: tangent + primal の最適なメモリレイアウト
3. **並列集約**: adjoint 値のスレッドセーフな集約方法
4. **チェックポイント統合**: 既存 `CheckpointWorkspace` との連携

### 重要決定事項

- [ ] `#[autodiff]` の Activity 指定方法（型レベル vs 実行時）
- [ ] Greeks 計算の granularity（個別 vs 一括）
- [ ] Fallback mode の切り替え条件

---

## Changelog

| Date | Change |
|------|--------|
| 2026-01-10 | Gap analysis 作成 |

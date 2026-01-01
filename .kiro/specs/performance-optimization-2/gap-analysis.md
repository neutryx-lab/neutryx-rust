# Gap Analysis: performance-optimization-2

## Executive Summary

本ギャップ分析は、パフォーマンス改善要件と既存コードベースの現状を比較し、実装アプローチを評価する。

### 主要な発見事項

- **Monte Carlo基盤**: PathWorkspaceとRNG（Ziggurat）は実装済み。SIMDベクトル化が未実装
- **Enzyme AD**: Phase 3.0のプレースホルダー実装（有限差分）。真のEnzyme統合はPhase 4待ち
- **メモリ効率**: PathObserverとSoAは実装済み。スレッドローカルバッファプールが未実装
- **並列処理**: Rayon基盤は存在。ワークスチーリングチューニングが未実装
- **ベンチマーク**: Criterionベンチマークは存在。CIリグレッション検出が未実装

---

## 1. Current State Investigation

### 1.1 Key Files and Modules

| レイヤー | パス | 目的 | 状態 |
|---------|------|------|------|
| L3 | `pricer_kernel/src/mc/workspace.rs` | PathWorkspace（バッファ管理） | ✅ 実装済み |
| L3 | `pricer_kernel/src/rng/prng.rs` | PricerRng（Ziggurat） | ✅ 実装済み |
| L3 | `pricer_kernel/src/enzyme/mod.rs` | Enzyme AD bindings | ⚠️ プレースホルダー |
| L3 | `pricer_kernel/src/checkpoint/` | チェックポイント機構 | ⚠️ 構造のみ |
| L3 | `pricer_kernel/src/path_dependent/observer.rs` | ストリーミング統計 | ✅ 実装済み |
| L4 | `pricer_xva/src/parallel/mod.rs` | Rayon並列処理 | ✅ 基本実装 |
| L4 | `pricer_xva/src/soa/trade_soa.rs` | Structure of Arrays | ✅ 実装済み |
| - | `Cargo.toml` | ビルド設定（LTO, codegen-units） | ✅ 設定済み |
| - | `.github/workflows/ci.yml` | CI/ベンチマーク | ⚠️ 基本のみ |

### 1.2 Architecture Patterns

- **4層アーキテクチャ**: L1→L2→L3→L4の依存方向が厳守されている
- **静的ディスパッチ**: `enum`ベースでEnzyme互換性を確保
- **ゼロアロケーション設計**: PathWorkspaceによるバッファホイスティング
- **ストリーミング統計**: PathObserverによるメモリ効率的なパス観測

### 1.3 Integration Surfaces

- **RNG → PathWorkspace**: `fill_normal`でバッファを埋める
- **PathWorkspace → MonteCarloPricer**: パス生成とペイオフ計算
- **TradeSoA → Rayon**: 並列バッチ処理

---

## 2. Requirements Feasibility Analysis

### Requirement 1: Monte Carlo シミュレーション高速化

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| 2倍速度向上 | 基準なし | **Unknown** - ベースライン測定が必要 | M |
| ゼロアロケーション | PathWorkspace実装済み | ✅ **既存** | - |
| Ziggurat法 | rand_distr使用 | ✅ **既存** | - |
| SIMD最適化 | 未実装 | **Missing** - AVX2/512対応が必要 | L |
| バッファ再利用 | ensure_capacity実装 | ⚠️ **Partial** - 初期化コスト未最適化 | S |

**技術的考察**:
- 現在のRNG実装は`rand_distr::StandardNormal`を使用しており、内部的にZiggurat法を採用
- SIMDベクトル化には`packed_simd`または手動SIMD（`std::arch`）が必要
- パス生成のベクトル化は複雑度が高く、Enzyme互換性の検証が必要

### Requirement 2: Enzyme自動微分最適化

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| 有限差分比5倍高速化 | 有限差分のみ | **Missing** - Enzyme統合が必要 | XL |
| メモリ最小化 | 未実装 | **Research Needed** | L |
| バッチGreeks | 未実装 | **Missing** | M |
| `#[autodiff]`マクロ | 未実装 | **Missing** - nightly feature | L |
| num-dualフォールバック | 手動切替 | **Missing** - 自動切替が必要 | M |

**技術的考察**:
- 現在の`enzyme/mod.rs`は有限差分による近似実装
- 真のEnzyme統合には`#![feature(autodiff)]`とnightly Rustが必要
- CI環境では`rustc-codegen-llvm-enzyme`コンポーネントの可用性が変動する

### Requirement 3: メモリ効率改善

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| ストリーミング統計 | PathObserver実装済み | ✅ **既存** | - |
| SoAレイアウト | TradeSoA実装済み | ⚠️ **Partial** - L3への拡張が必要 | M |
| 線形以下メモリ | 未検証 | **Unknown** - プロファイリングが必要 | M |
| チェックポイント最小化 | 構造のみ | **Missing** - 実装が必要 | L |
| スレッドローカルバッファ | 未実装 | **Missing** | M |

**技術的考察**:
- PathObserverは算術/幾何平均、最大/最小をストリーミング計算
- CheckpointModuleの`state.rs`, `strategy.rs`は構造のみで中身が未実装
- `thread_local!`マクロでスレッドローカルバッファプールを実装可能

### Requirement 4: 並列処理最適化

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| 80%並列効率 | 未測定 | **Unknown** - 測定が必要 | S |
| ワークスチーリング | Rayon標準 | ✅ **既存**（暗黙的） | - |
| XVA並列計算 | 基本実装 | ⚠️ **Partial** | S |
| スレッドセーフRNG | シード管理あり | ⚠️ **Partial** - 改善の余地あり | M |
| 動的スケーリング | ParallelConfig | ⚠️ **Partial** | S |

**技術的考察**:
- Rayonは内部的にワークスチーリングを実装済み
- `ParallelConfig`で`parallel_threshold`を設定可能
- RNGのスレッドセーフ性は`PricerRng`がスレッドごとに作成される設計で確保

### Requirement 5: コンパイラ最適化活用

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| LTO有効化 | `lto = true` | ✅ **既存** | - |
| codegen-units = 1 | 設定済み | ✅ **既存** | - |
| target-cpu=native | 未設定 | **Missing** | S |
| 静的ディスパッチ | enum使用 | ✅ **既存** | - |
| PGOサポート | 未実装 | **Missing** | M |

**技術的考察**:
- `Cargo.toml`の`[profile.release]`でLTOとcodegen-units設定済み
- `RUSTFLAGS="-C target-cpu=native"`の設定は`.cargo/config.toml`で可能
- PGOは`RUSTFLAGS="-C profile-generate"`と`-C profile-use`で実現

### Requirement 6: ベンチマーク・計測インフラ

| 基準 | 現状 | ギャップ | 複雑度 |
|------|------|----------|--------|
| Criterion使用 | 4クレートで実装 | ✅ **既存** | - |
| CIリグレッション検出 | 未実装 | **Missing** | M |
| 領域カバレッジ | MC, Greeks, ワークスペース | ⚠️ **Partial** | S |
| 履歴比較グラフ | 未実装 | **Missing** | M |
| 閾値アラート | 未実装 | **Missing** | M |

**技術的考察**:
- `kernel_benchmarks.rs`にRNG、MC、Greeks、ワークスペースベンチマークが存在
- CIでベンチマーク実行するが、結果比較・リグレッション検出は未実装
- `bencher`や`criterion-compare`でCI統合可能

---

## 3. Implementation Approach Options

### Option A: 既存コンポーネント拡張

**適用範囲**:
- Requirement 5（コンパイラ最適化）: `.cargo/config.toml`設定追加
- Requirement 6（ベンチマーク）: 既存ベンチマークにCI統合を追加

**拡張対象ファイル**:
- `Cargo.toml`: PGOプロファイル追加
- `.cargo/config.toml`（新規）: `target-cpu=native`設定
- `.github/workflows/ci.yml`: ベンチマーク比較ステップ追加

**Trade-offs**:
- ✅ 最小限の変更で即効性
- ✅ 既存パターンを尊重
- ❌ SIMD/Enzyme統合には不十分

### Option B: 新規コンポーネント作成

**適用範囲**:
- Requirement 1（SIMD）: `pricer_kernel/src/simd/` モジュール新規作成
- Requirement 2（Enzyme）: `pricer_kernel/src/enzyme/` の完全再実装
- Requirement 3（バッファプール）: `pricer_kernel/src/pool/` 新規作成

**新規ファイル候補**:
```
pricer_kernel/src/
├── simd/
│   ├── mod.rs          # SIMD抽象化
│   ├── avx2.rs         # AVX2実装
│   └── fallback.rs     # スカラーフォールバック
├── pool/
│   ├── mod.rs          # バッファプールAPI
│   └── thread_local.rs # スレッドローカルプール
└── enzyme/
    ├── mod.rs          # Enzyme API（再設計）
    ├── forward.rs      # Forward-mode AD
    └── reverse.rs      # Reverse-mode AD
```

**Trade-offs**:
- ✅ クリーンな責務分離
- ✅ テスト容易性
- ❌ ファイル数増加
- ❌ 既存コードとの整合性維持が必要

### Option C: ハイブリッドアプローチ（推奨）

**フェーズ分割**:

| フェーズ | 範囲 | アプローチ |
|---------|------|-----------|
| Phase 1 | Req 5, 6 | Option A（拡張） |
| Phase 2 | Req 3, 4 | 混合（拡張 + 新規） |
| Phase 3 | Req 1, 2 | Option B（新規） |

**推奨理由**:
1. Phase 1で計測インフラを整備し、改善効果を定量化
2. Phase 2でメモリ・並列処理を改善し、Phase 3の基盤を準備
3. Phase 3でSIMD/Enzyme統合という最も複雑な作業に着手

**Trade-offs**:
- ✅ リスク分散
- ✅ 各フェーズで効果測定可能
- ❌ 総期間が長くなる可能性

---

## 4. Implementation Complexity & Risk

### Effort Estimation

| 要件 | 努力度 | 根拠 |
|------|--------|------|
| Req 1: Monte Carlo高速化 | **L** | SIMD実装とEnzyme互換性検証が必要 |
| Req 2: Enzyme AD最適化 | **XL** | nightly機能依存、複雑なLLVM統合 |
| Req 3: メモリ効率改善 | **M** | チェックポイント実装、バッファプール追加 |
| Req 4: 並列処理最適化 | **S** | 既存Rayon基盤の調整のみ |
| Req 5: コンパイラ最適化 | **S** | 設定ファイル変更のみ |
| Req 6: ベンチマークインフラ | **M** | CI統合とリグレッション検出追加 |

### Risk Assessment

| 要件 | リスク | 根拠 |
|------|--------|------|
| Req 1 | **Medium** | SIMD抽象化の複雑さ、プラットフォーム依存 |
| Req 2 | **High** | Enzyme可用性が不安定、nightly依存 |
| Req 3 | **Medium** | チェックポイント設計の複雑さ |
| Req 4 | **Low** | Rayonが成熟しており、変更範囲が限定的 |
| Req 5 | **Low** | 設定変更のみ |
| Req 6 | **Low** | 既存ツールの統合のみ |

---

## 5. Research Items for Design Phase

1. **SIMD互換性調査**: `packed_simd` vs `portable_simd` vs `std::arch`の比較
2. **Enzyme安定性**: nightly-2025-01-15でのEnzyme可用性と機能範囲
3. **PGOワークフロー**: Rustでのプロファイルガイド最適化の実践パターン
4. **ベンチマーク比較ツール**: `criterion-compare`、`bencher`のCI統合オプション
5. **スレッドローカルバッファ**: `thread_local!`のパフォーマンス特性

---

## 6. Recommendations

### 設計フェーズへの推奨事項

1. **Option C（ハイブリッドアプローチ）を採用**
   - 計測インフラを先に整備し、改善効果を可視化
   - 段階的に複雑度の高い要件に着手

2. **優先順位**:
   1. Req 6（ベンチマーク）→ Req 5（コンパイラ最適化）
   2. Req 4（並列処理）→ Req 3（メモリ効率）
   3. Req 1（Monte Carlo）→ Req 2（Enzyme）

3. **リスク軽減**:
   - Enzyme統合は機能フラグで分離し、fallbackを常に維持
   - SIMDはコンパイル時検出で自動フォールバック

### 次のステップ

```
/kiro:spec-design performance-optimization-2
```

設計フェーズで、各要件の詳細な技術設計を策定してください。

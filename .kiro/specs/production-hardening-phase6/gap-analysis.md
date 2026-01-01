# Gap Analysis: Production Hardening Phase 6

## Executive Summary

Phase 6の要件に対する既存コードベースの分析結果。CI/CDとドキュメントの基盤は既に存在するが、ベンチマーク、コードカバレッジ、リリース自動化が主要なギャップとして特定された。

---

## 1. Current State Investigation

### 1.1 Existing Assets

| カテゴリ | 既存アセット | パス |
|---------|-------------|------|
| README | プロジェクト概要、アーキテクチャ、クイックスタート | `README.md` |
| 貢献ガイド | SDD ワークフロー中心 | `CONTRIBUTING.md` |
| CI/CD | Stable/Nightly分離、Enzyme対応 | `.github/workflows/ci.yml` |
| Docker | Stable/Nightly Dockerfile | `docker/Dockerfile.{stable,nightly}` |
| Enzyme スクリプト | インストール・検証 | `scripts/install_enzyme.sh`, `scripts/verify_enzyme.sh` |
| API ドキュメント | `#![warn(missing_docs)]` 設定済 | 各crate `lib.rs` |

### 1.2 Conventions Extracted

**ドキュメント規約:**
- クレートレベルdocコメント: 全4クレートに存在
- モジュールレベル: 部分的
- 関数レベル: 不均一（一部欠損）

**CI/CD規約:**
- GitHub Actions使用
- Job分離: stable-layers, nightly-kernel, integration, docs
- キャッシュ戦略: cargo registry/index/build

**テスト規約:**
- `#[cfg(test)]` でテスト配置
- `approx`, `proptest` 使用

### 1.3 Integration Surfaces

- **クレート間依存**: L1 → L2 → L3 → L4 の単方向フロー
- **ツールチェーン分離**: stable (L1/L2/L4) vs nightly (L3)
- **Feature flags**: `num-dual-mode`, `enzyme-mode`, `serde`

---

## 2. Requirements Feasibility Analysis

### Requirement-to-Asset Map

| 要件 | 状態 | 既存アセット | ギャップ |
|-----|------|-------------|---------|
| **Req1: APIドキュメント** | Partial | `#![warn(missing_docs)]` | warn→deny変更、Examples追加 |
| **Req2: ユーザーガイド** | Partial | README.md, Dockerセットアップ | アーキテクチャ図強化 |
| **Req3: ベンチマーク** | **Missing** | なし | 全クレートにbenches/作成必要 |
| **Req4: CI/CD (Stable)** | Partial | ci.yml stable-layers job | クロスプラットフォーム未対応 |
| **Req5: CI/CD (Nightly)** | Complete | ci.yml nightly-kernel job | — |
| **Req6: 品質ゲート** | Partial | fmt/clippy/test | カバレッジ未実装 |
| **Req7: リリース自動化** | **Missing** | なし | release workflow作成必要 |
| **Req8: DX向上** | Partial | CONTRIBUTING.md | issue/PRテンプレート未作成 |

### Technical Needs by Requirement

**Req1: APIドキュメント**
- `#![deny(missing_docs)]` への変更
- 各パブリック関数に `# Examples` セクション追加
- `cargo doc` 警告ゼロ達成

**Req3: ベンチマーク (主要ギャップ)**
- `criterion` ベンチマーク作成
  - pricer_core: 補間処理 (linear, cubic_spline, bilinear)
  - pricer_models: Black-Scholes価格計算
  - pricer_kernel: MC パス生成、Enzyme vs num-dual
  - pricer_xva: ポートフォリオXVA計算

**Req6: コードカバレッジ**
- `cargo-tarpaulin` または `grcov` 統合
- CI ジョブでのカバレッジレポート生成

**Req7: リリース自動化 (主要ギャップ)**
- `release.yml` ワークフロー作成
- タグベースのリリースビルド
- CHANGELOG自動生成 (`git-cliff` または `conventional-changelog`)

### Complexity Signals

| 領域 | 複雑度 | 理由 |
|-----|--------|------|
| ベンチマーク作成 | Medium | Criterion API習熟、適切なベンチマーク設計 |
| カバレッジ統合 | Low | 既存ツール利用、CI設定のみ |
| リリース自動化 | Medium | ワークフロー設計、SemVer管理 |
| ドキュメント強化 | Low | 既存コードへのdocコメント追加 |
| クロスプラットフォームCI | Low | matrix戦略追加のみ |

---

## 3. Implementation Approach Options

### Option A: Incremental Enhancement

**対象**: 既存CI/CDとドキュメントの拡張

**アプローチ**:
1. 既存 `ci.yml` にカバレッジジョブ追加
2. 既存 `ci.yml` にmatrix戦略追加（Windows/macOS）
3. 各クレートに `benches/` ディレクトリ追加
4. 既存ソースに `#![deny(missing_docs)]` 適用

**Trade-offs**:
- ✅ 既存インフラ活用、リスク低
- ✅ 段階的導入可能
- ❌ ワークフロー肥大化の可能性

### Option B: Modular Workflows

**対象**: 目的別ワークフロー分離

**アプローチ**:
1. `.github/workflows/ci.yml` - 基本ビルド・テスト
2. `.github/workflows/coverage.yml` - カバレッジ専用
3. `.github/workflows/release.yml` - リリース専用
4. `.github/workflows/docs.yml` - ドキュメント生成

**Trade-offs**:
- ✅ 責務分離、メンテナンス性向上
- ✅ 必要なジョブのみ実行可能
- ❌ ファイル数増加

### Option C: Hybrid Approach (推奨)

**対象**: CI/CDは分離、ベンチマーク・ドキュメントは既存構造内

**アプローチ**:
1. **CI/CD**: `ci.yml` + `release.yml` の2ファイル構成
   - `ci.yml`: ビルド、テスト、カバレッジ、ドキュメント
   - `release.yml`: タグトリガー、リリースビルド
2. **ベンチマーク**: 各クレート内 `benches/` に配置
3. **ドキュメント**: 既存ファイルへのdocコメント追加

**Trade-offs**:
- ✅ バランスの取れた分離
- ✅ リリースフローの明確な分離
- ✅ 既存CI構造を維持

---

## 4. Implementation Complexity & Risk

### Effort Estimation

| 要件 | 工数 | 根拠 |
|-----|------|------|
| Req1: APIドキュメント | M | 全クレートの公開API確認、Examples追加 |
| Req2: ユーザーガイド | S | README強化、既存内容の整理 |
| Req3: ベンチマーク | L | 4クレート×複数ベンチマーク、設計必要 |
| Req4: CI/CD (Stable) | S | matrix追加のみ |
| Req5: CI/CD (Nightly) | — | 既に完了 |
| Req6: 品質ゲート | S | カバレッジツール統合 |
| Req7: リリース自動化 | M | 新規ワークフロー設計 |
| Req8: DX向上 | S | テンプレート追加 |

**総合工数**: M-L（ベンチマーク作成が主要作業）

### Risk Assessment

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| ベンチマーク設計の不適切さ | Medium | 業界標準の参照、段階的導入 |
| Enzyme CI不安定 | Medium | フォールバック機構（既に実装済） |
| カバレッジツール互換性 | Low | 複数ツール候補（tarpaulin, grcov） |
| クロスプラットフォーム問題 | Medium | Enzyme無効でのビルド確認 |

---

## 5. Recommendations for Design Phase

### Preferred Approach

**Option C (Hybrid)** を推奨:
- CI/CD基盤は既に成熟しており、リリースワークフローのみ分離
- ベンチマークは各クレート内で管理（Rust標準パターン）
- ドキュメントは既存構造を活用

### Key Design Decisions Required

1. **カバレッジツール選択**: `cargo-tarpaulin` vs `grcov`
2. **CHANGELOG生成ツール**: `git-cliff` vs `conventional-changelog`
3. **ベンチマーク対象関数の選定**: パフォーマンスクリティカルな処理の特定
4. **カバレッジ閾値**: 目標カバレッジ率の決定

### Research Items to Carry Forward

| 項目 | 理由 |
|------|------|
| Enzyme CI Windows/macOS対応 | クロスプラットフォームでのEnzyme可用性未確認 |
| criterion vs iai ベンチマーク | 代替ツールの評価 |
| crates.io公開要件 | 将来対応のための事前調査 |

---

## 6. Summary

### 主要ギャップ

1. **ベンチマーク**: 完全に欠如、criterion統合が必要
2. **リリース自動化**: ワークフロー未作成
3. **コードカバレッジ**: コメントアウト状態、有効化必要
4. **クロスプラットフォームCI**: Windows/macOS未対応

### 既存強み

1. **CI/CD基盤**: Stable/Nightly分離、Enzyme対応済
2. **ドキュメント**: README、CONTRIBUTING、クレートdocが存在
3. **Docker環境**: Stable/Nightly両対応

### 次のアクション

設計フェーズで以下を詳細化:
1. ベンチマーク対象関数とメトリクスの定義
2. release.ymlワークフローの詳細設計
3. カバレッジ統合の技術選定

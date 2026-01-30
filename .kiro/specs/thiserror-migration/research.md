# ギャップ分析レポート: thiserror-migration

## エグゼクティブサマリー

コードベースの徹底的な調査により、移行スコープが当初の想定よりも**大幅に小さい**ことが判明した。

| 項目 | 状況 |
|------|------|
| 移行対象ファイル | **1件のみ** (`pricer_pricing/src/graph/error.rs`) |
| 既に thiserror 使用中 | **27件** (全エラーファイルの 96%) |
| 依存関係変更 | **不要** (thiserror は既にワークスペース依存関係) |
| 推定工数 | **S (1日以下)** |
| リスク | **Low** |

---

## 1. 現状調査

### 1.1 対象ファイル詳細分析

**ファイル**: `crates/pricer_pricing/src/graph/error.rs` (111行)

```rust
// 現在の実装
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum GraphError {
    TradeNotFound(String),
    ExtractionFailed(String),
    Timeout,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for GraphError {}
```

**カスタムメソッド** (維持必須):
- `http_status_code()` → HTTP ステータスコードマッピング (404, 500)
- `message()` → 人間可読エラーメッセージ生成

### 1.2 使用箇所マッピング

| ファイル | 使用回数 | 使用パターン |
|---------|---------|-------------|
| extractor.rs | ~45箇所 | `Result<_, GraphError>`, `Err(GraphError::*)` |
| volcube_extractor.rs | ~10箇所 | 同上 |
| mod.rs (テスト) | ~15箇所 | `matches!`, `assert_eq!` |
| lib.rs | 1箇所 | re-export |

**テストカバレッジ**: 既存テストは `mod.rs` の Task 1.3 セクションに集約

### 1.3 参照パターン (既存 thiserror 実装)

`pricer_core/src/types/error.rs` が参照パターンとして適切:

```rust
// 基本パターン
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PricingError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// 構造化フィールドパターン
#[derive(Error, Debug, Clone, PartialEq)]
pub enum InterpolationError {
    #[error("Query point {x} outside valid domain [{min}, {max}]")]
    OutOfBounds { x: f64, min: f64, max: f64 },
}
```

### 1.4 依存関係状況

| 項目 | 現状 |
|------|------|
| ワークスペース定義 | `thiserror = "2.0"` (Cargo.toml:146) |
| pricer_pricing 依存関係 | `thiserror.workspace = true` (既に設定済み) |
| 追加設定 | **不要** |

---

## 2. 要件実現可能性分析

### 2.1 技術要件マッピング

| 要件 | 対応コンポーネント | ギャップ |
|------|-------------------|---------|
| R1: GraphError の thiserror 移行 | error.rs | ✅ 手動 Display/Error 削除 |
| R2: エラー変換の自動化 | N/A | ✅ `#[from]` 不要 (下位エラーなし) |
| R3: コード品質の維持 | CI パイプライン | ✅ 既存テストで検証 |
| R4: ステアリング整合性 | error-handling.md | ✅ パターン準拠 |
| R5: ガイドライン確立 | ステアリング | ✅ 既に記載済み |

### 2.2 制約事項

1. **メッセージ互換性**: `message()` の出力を `#[error("...")]` で再現する必要あり
2. **serde 互換性**: 既存の `#[cfg_attr(feature = "serde", ...)]` 属性を維持
3. **カスタムメソッド維持**: `http_status_code()` と `message()` は別 impl ブロックで保持

### 2.3 複雑性評価

| 側面 | 評価 | 理由 |
|------|------|------|
| コード変更量 | 極小 | 1ファイル、~15行の変更 |
| 依存関係 | なし | 既に設定済み |
| テスト影響 | 低 | 既存テストは動作互換 |
| 外部統合 | なし | 内部コンポーネントのみ |

---

## 3. 実装アプローチオプション

### Option A: 最小変更 (推奨)

**概要**: GraphError enum に `#[derive(Error)]` を追加し、手動 impl を削除

**変更内容**:
```rust
// Before
#[derive(Debug, Clone)]
pub enum GraphError { ... }

impl std::fmt::Display for GraphError { ... }
impl std::error::Error for GraphError {}

// After
#[derive(Error, Debug, Clone)]
pub enum GraphError {
    #[error("Trade '{0}' not found")]
    TradeNotFound(String),

    #[error("Graph extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Graph extraction timed out (exceeded 500ms limit)")]
    Timeout,
}
```

**トレードオフ**:
- ✅ 最小限の変更で目的達成
- ✅ 既存テストがそのまま動作
- ✅ `message()` メソッドを削除可能 (Display と同一のため)
- ❌ `message()` を使用する呼び出し元の修正が必要

### Option B: 互換性優先

**概要**: `#[derive(Error)]` 追加しつつ、`message()` メソッドを維持

**変更内容**:
- `#[derive(Error)]` を追加
- `#[error("...")]` で Display を定義
- `message()` は `self.to_string()` を返すラッパーとして維持

**トレードオフ**:
- ✅ 呼び出し元の変更不要
- ✅ 段階的移行が可能
- ❌ 重複コード（Display と message が同一）
- ❌ 技術的負債が残る

---

## 4. リスク評価

### 4.1 工数評価

| オプション | 工数 | 根拠 |
|-----------|------|------|
| Option A | **S (1-3時間)** | 単一ファイル、明確な変更パターン |
| Option B | **S (1日)** | 互換性レイヤー追加のため若干増 |

### 4.2 リスク評価

| リスク | 影響 | 確率 | 緩和策 |
|--------|------|------|--------|
| テスト失敗 | Low | Low | 既存テストで検証 |
| API 互換性破壊 | Medium | Low | Option B で回避可能 |
| ビルドエラー | Low | Very Low | thiserror 既に依存関係 |

**総合リスク**: **Low**

---

## 5. 設計フェーズへの推奨事項

### 5.1 推奨アプローチ

**Option A (最小変更)** を推奨

理由:
1. 目的（コード削減）に最も直接的
2. 変更範囲が最小限
3. リスクが最も低い
4. `message()` 呼び出し元は `.to_string()` に置換可能

### 5.2 設計フェーズでの確認事項

1. **`message()` 呼び出し元の影響範囲**
   - `http_status_code()` と組み合わせて使用されているか
   - REST API レスポンス生成に影響するか

2. **serde 互換性の維持**
   - `#[cfg_attr(feature = "serde", serde(tag = "error_type", content = "message"))]` の維持方法

### 5.3 調査不要項目

以下は設計フェーズでの追加調査が不要:

- thiserror のバージョン互換性 (v2.0 は安定版)
- 依存関係の追加
- 他クレートへの影響

---

## 6. 結論

本移行は**低リスク・低工数**のリファクタリングであり、以下の特徴を持つ:

- 移行対象は1ファイルのみ
- thiserror 依存関係は既に設定済み
- 既存テストで品質担保可能
- 推定工数: **S (1日以下)**
- 推定リスク: **Low**

設計フェーズでは Option A を基本方針として、`message()` メソッドの呼び出し元処理を決定することを推奨する。

# 設計ドキュメント: thiserror-migration

## Overview

**Purpose**: `GraphError` 型を thiserror マクロベースの定義に移行し、手動の `Display`/`Error` 実装を削除してボイラープレートコードを削減する。

**Users**: Neutryx 開発チームが `pricer_pricing` クレートの graph モジュールを使用する際に、一貫したエラー処理パターンの恩恵を受ける。

**Impact**: 既存の `crates/pricer_pricing/src/graph/error.rs` ファイル (111行) のうち、手動 impl ブロック約15行を削除し、マクロベースの定義に置換する。

### Goals

- `GraphError` を thiserror パターンに準拠させる
- 手動の `impl Display` および `impl Error` を削除
- 既存の `http_status_code()` メソッドを維持
- 既存テストの互換性を維持

### Non-Goals

- 既に thiserror を使用している他27ファイルの変更
- `GraphError` のバリアント構造の変更
- 新規エラー型の追加
- 呼び出し元コードの大幅なリファクタリング

## Architecture

### Existing Architecture Analysis

**現在の実装** (`crates/pricer_pricing/src/graph/error.rs`):

```rust
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum GraphError {
    TradeNotFound(String),
    ExtractionFailed(String),
    Timeout,
}

impl std::fmt::Display for GraphError { ... }  // 削除対象
impl std::error::Error for GraphError {}       // 削除対象
```

**維持すべきパターン**:
- `http_status_code()` → HTTP ステータスコードマッピング (REST API 用)
- serde feature flag による条件付きシリアライゼーション
- 既存のバリアント構造 (tuple variant)

### Architecture Pattern & Boundary Map

本変更は単一ファイルのリファクタリングであり、アーキテクチャ境界への影響はない。

**Steering 準拠**: `error-handling.md` の標準パターンに完全準拠

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Error Handling | thiserror 2.0 | マクロベースの Error/Display 導出 | ワークスペース依存関係として既存 |

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1 | `#[derive(Error)]` 使用 | GraphError | — | — |
| 1.2 | `#[error("...")]` 属性 | GraphError | — | — |
| 1.3 | 手動 impl 削除 | GraphError | — | — |
| 1.4 | カスタムメソッド維持 | GraphError | `http_status_code()` | — |
| 1.5 | テスト互換性 | graph/mod.rs | — | — |
| 3.1 | fmt チェック | CI | — | — |
| 3.2 | clippy チェック | CI | — | — |
| 3.3 | test チェック | CI | — | — |
| 4.1 | ステアリング準拠 | GraphError | — | — |

## Components and Interfaces

### Pricer Layer

#### GraphError

| Field | Detail |
|-------|--------|
| Intent | グラフ抽出操作のエラー型を thiserror パターンに移行 |
| Requirements | 1.1, 1.2, 1.3, 1.4, 4.1 |

**Responsibilities & Constraints**

- グラフ抽出操作 (`extract_graph`, `extract_affected_nodes`) のエラー表現
- HTTP ステータスコードへのマッピング (REST API レスポンス用)
- serde feature flag による条件付きシリアライゼーション

**Dependencies**

- Inbound: `extractor.rs`, `volcube_extractor.rs` — エラー生成 (P0)
- Outbound: なし
- External: `thiserror` 2.0 — マクロ導出 (P0)

**Contracts**: Service [ ] / API [ ] / Event [ ] / Batch [ ] / State [ ]

**Implementation Notes**

- **変更前後の対応**:

```rust
// === 変更前 ===
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "error_type", content = "message"))]
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

// === 変更後 ===
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "error_type", content = "message"))]
pub enum GraphError {
    #[error("Trade '{0}' not found")]
    TradeNotFound(String),

    #[error("Graph extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Graph extraction timed out (exceeded 500ms limit)")]
    Timeout,
}
```

- **`message()` メソッドの扱い**: Display と同一出力のため、`self.to_string()` を返すラッパーとして維持。将来的に削除検討。

```rust
impl GraphError {
    pub fn message(&self) -> String {
        self.to_string()
    }

    pub fn http_status_code(&self) -> u16 {
        match self {
            GraphError::TradeNotFound(_) => 404,
            GraphError::ExtractionFailed(_) => 500,
            GraphError::Timeout => 500,
        }
    }
}
```

- **Validation**: 既存テスト (`mod.rs` Task 1.3 セクション) で Display 出力を検証
- **Risks**: 低。API 互換性は維持。

## Error Handling

### Error Strategy

本設計自体がエラー型の移行であり、追加のエラーハンドリング戦略は不要。

### Error Categories and Responses

`GraphError` の HTTP ステータスコードマッピングは変更なし:

| Variant | HTTP Status | Description |
|---------|-------------|-------------|
| `TradeNotFound` | 404 | 指定された trade_id が存在しない |
| `ExtractionFailed` | 500 | グラフ抽出の内部エラー |
| `Timeout` | 500 | 500ms タイムアウト超過 |

## Testing Strategy

### Unit Tests

既存テスト (`crates/pricer_pricing/src/graph/mod.rs`) で以下を検証:

1. `GraphError::TradeNotFound` の Display 出力
2. `GraphError::ExtractionFailed` の Display 出力
3. `GraphError::Timeout` の Display 出力
4. `http_status_code()` のマッピング
5. `matches!` マクロによるパターンマッチ

### Integration Tests

既存の `extractor.rs` テストで `Result<_, GraphError>` の伝播を検証。

### CI Validation

| Check | Command | Purpose |
|-------|---------|---------|
| Format | `cargo fmt --all -- --check` | コードフォーマット |
| Lint | `cargo clippy --workspace -- -D warnings` | 静的解析 |
| Test | `cargo test --workspace` | 全テスト実行 |
| Doc | `cargo doc --workspace --no-deps` | ドキュメント生成 |

## Supporting References

詳細な調査結果は [research.md](research.md) を参照:

- 現状調査 (1.1 対象ファイル詳細分析)
- 使用箇所マッピング (1.2)
- 参照パターン (1.3)
- リスク評価 (4.2)

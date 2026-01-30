# Implementation Plan

## Tasks

- [ ] 1. GraphError の thiserror 移行
- [x] 1.1 thiserror マクロの導入と #[error] 属性の追加
  - `use thiserror::Error;` をファイル先頭に追加
  - derive マクロに `Error` を追加 (`#[derive(Error, Debug, Clone)]`)
  - 各バリアントに `#[error("...")]` 属性を追加
    - `TradeNotFound`: `#[error("Trade '{0}' not found")]`
    - `ExtractionFailed`: `#[error("Graph extraction failed: {0}")]`
    - `Timeout`: `#[error("Graph extraction timed out (exceeded 500ms limit)")]`
  - 既存の serde 属性は維持
  - _Requirements: 1.1, 1.2, 4.1, 4.4_

- [x] 1.2 手動 impl の削除と message() メソッドの更新
  - `impl std::fmt::Display for GraphError` ブロックを削除
  - `impl std::error::Error for GraphError {}` 行を削除
  - `message()` メソッドを `self.to_string()` を返すラッパーに変更
  - `http_status_code()` メソッドは変更なし
  - _Requirements: 1.3, 1.4_

- [ ] 2. 検証
- [ ] 2.1 テストと CI チェックの実行
  - `cargo test -p pricer_pricing` で既存テストがパスすることを確認
  - `cargo fmt --all -- --check` でフォーマットエラーがないことを確認
  - `cargo clippy --workspace -- -D warnings` で警告がないことを確認
  - `cargo doc --workspace --no-deps` でドキュメント生成が成功することを確認
  - _Requirements: 1.5, 3.1, 3.2, 3.3, 3.4_

## 要件カバレッジ

| 要件 | タスク | 状態 |
|------|--------|------|
| 1.1 | 1.1 | ✅ |
| 1.2 | 1.1 | ✅ |
| 1.3 | 1.2 | ✅ |
| 1.4 | 1.2 | ✅ |
| 1.5 | 2.1 | ✅ |
| 2.1-2.3 | — | N/A (GraphError は他エラーをラップしない) |
| 3.1 | 2.1 | ✅ |
| 3.2 | 2.1 | ✅ |
| 3.3 | 2.1 | ✅ |
| 3.4 | 2.1 | ✅ |
| 4.1 | 1.1 | ✅ |
| 4.2 | — | ✅ (既存の Debug, Clone を維持) |
| 4.3 | — | N/A (PartialEq は現在不要) |
| 4.4 | 1.1 | ✅ |
| 5.1-5.4 | — | スコープ外 (ステアリング文書は既に最新) |

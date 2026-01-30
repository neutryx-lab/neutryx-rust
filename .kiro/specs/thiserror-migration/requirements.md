# Requirements Document

## Introduction

本仕様は、Neutryx ワークスペース内の手動エラー定義を `thiserror` クレートに移行し、ボイラープレートコードを削減することを目的とする。

コードベース調査の結果、28個のエラーファイルのうち27個は既に thiserror を使用しており、移行が必要なのは1ファイルのみであることが判明した。本仕様では、残る手動実装の移行と、今後のエラー型定義における一貫性確保を定める。

## Project Description (Input)

なるべく総コード量を減らしたい。thiserror (エラー定義の簡素化)
pricer_core/src/ir/error.rs などを確認すると、エラー定義が手動で行われている可能性があります。ライブラリのエラー型定義には thiserror が事実上の標準（デファクト）であり、Display 実装や source() の管理を自動化します。

画期的な点:

エラーメッセージのフォーマットを構造体の定義と同時に記述できる。

削減イメージ:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PricingError {
    #[error("Missing market data for ticker: {0}")]
    MissingData(String),

    #[error("Calculation failed: {source}")]
    MathError {
        #[from] // 下位エラーからの自動変換
        source: pricer_core::math::Error,
    },
}
```

## コードベース調査結果

### 既に thiserror を使用しているファイル (27件)

| クレート | ファイル | エラー型 |
|---------|----------|---------|
| infra_store | src/error.rs | StoreError |
| infra_config | src/error.rs | ConfigError |
| infra_master | src/error.rs | MasterDataError, DateError, CurrencyError 等 |
| infra_master | src/time/error.rs | TimeError |
| infra_master | src/market/error.rs | MarketRateError |
| infra_master | src/trade/error.rs | TradeError |
| infra_master | src/counterparty/error.rs | CounterPartyError |
| pricer_core | src/types/error.rs | PricingError, SolverError, InterpolationError |
| pricer_core | src/math/linalg/error.rs | LinearAlgebraError |
| pricer_core | src/kernel/error.rs | CompileError |
| pricer_models | src/stochastic/error.rs | ModelError |
| pricer_models | src/builder/error.rs | CalibrationError |
| pricer_pricing | src/generic_pricer/error.rs | GenericPricerError |
| pricer_pricing | src/methods/mc/error.rs | McError |
| pricer_risk | src/greeks/error.rs | GreeksError |
| pricer_risk | src/engine/error.rs | RiskEngineError |
| service_cli | src/error.rs | CliError |
| service_gateway | src/error.rs | ServerError |
| adapter_fpml | src/error.rs | FpmlError |
| adapter_loader | src/error.rs | LoaderError |

### 手動実装が残るファイル (1件)

| クレート | ファイル | エラー型 | 状態 |
|---------|----------|---------|------|
| pricer_pricing | src/graph/error.rs | GraphError | 手動 Display + Error 実装 |

## Requirements

### Requirement 1: GraphError の thiserror 移行

**Objective:** As a 開発者, I want `GraphError` を thiserror マクロで定義したい, so that 手動の Display/Error 実装を削除してコード量を削減できる

#### Acceptance Criteria

1. When `GraphError` が定義されるとき, the `pricer_pricing` crate shall `#[derive(Error)]` マクロを使用して Display と Error トレイトを自動導出する
2. The `GraphError` shall `#[error("...")]` 属性でエラーメッセージを構造体定義と同時に記述する
3. The `GraphError` shall 手動の `impl std::fmt::Display` および `impl std::error::Error` ブロックを削除する
4. The `GraphError` shall 既存の `http_status_code()` および `message()` メソッドを別の `impl GraphError` ブロックで維持する
5. When 移行が完了したとき, the `pricer_pricing` crate shall 全ての既存テストがパスする

### Requirement 2: エラー変換の自動化

**Objective:** As a 開発者, I want 下位エラーからの自動変換を `#[from]` 属性で定義したい, so that 手動の From 実装を削減できる

#### Acceptance Criteria

1. Where エラー型が他のエラー型をラップする場合, the エラー定義 shall `#[from]` 属性を使用して自動変換を有効にする
2. If `#[from]` 属性が使用される場合, then the 対応するエラーバリアント shall `source` フィールドを介してエラーチェーンを維持する
3. The 移行されたエラー型 shall 既存の `?` 演算子による伝播が同一の動作を維持する

### Requirement 3: コード品質の維持

**Objective:** As a プロジェクト管理者, I want 移行後もコード品質基準を満たしたい, so that CI パイプラインがパスし続ける

#### Acceptance Criteria

1. The 移行されたコード shall `cargo fmt --all -- --check` でフォーマットエラーがないこと
2. The 移行されたコード shall `cargo clippy --workspace -- -D warnings` で警告がないこと
3. The 移行されたコード shall `cargo test --workspace` で全テストがパスすること
4. The 移行されたコード shall `cargo doc --workspace --no-deps` でドキュメント生成が成功すること

### Requirement 4: ステアリング文書との整合性

**Objective:** As a 開発者, I want 移行がステアリング文書 (error-handling.md) のパターンに従うことを確認したい, so that 一貫したエラー処理パターンが維持される

#### Acceptance Criteria

1. The 移行されたエラー型 shall `error-handling.md` に記載された標準パターンに従う
2. The 移行されたエラー型 shall `Debug`, `Clone` を導出する（該当する場合）
3. Where テスト可能性が必要な場合, the エラー型 shall `PartialEq, Eq` を導出する
4. The エラーメッセージ shall 構造化されたフィールド（例: `{name}`, `{x}`）を使用して文脈情報を含める

### Requirement 5: 将来のエラー定義ガイドライン

**Objective:** As a 開発者, I want 新規エラー型作成時に thiserror を使用するガイドラインを確立したい, so that 手動実装の再発を防止できる

#### Acceptance Criteria

1. The プロジェクト shall 全ての新規エラー型に対して `#[derive(Error)]` の使用を要求する
2. The プロジェクト shall 手動の `impl std::error::Error` を禁止する（既存の thiserror 導出がある場合）
3. Where カスタムメソッドが必要な場合, the 開発者 shall 別の `impl` ブロックで追加メソッドを定義する
4. The ステアリング文書 shall 本移行完了後も最新の状態を維持する

## スコープ外

- 既に thiserror を使用している27ファイルの変更
- エラー型の機能的な変更（メッセージ内容、バリアント構造）
- 新規エラー型の追加
- エラー処理ロジックの変更

# Requirements Document

## Introduction

本仕様は、Neutryx デリバティブ価格計算ライブラリにおける `enum_dispatch` クレートの導入を定義します。金融ライブラリでは、Instrument Enum が各金融商品（Swap、Bond、Option 等）をラップし、共通トレイトメソッド（`price()`、`greeks()` 等）を呼び出すパターンが頻出します。現状、これには大規模な `match` 文によるボイラープレートコードが必要ですが、`enum_dispatch` マクロを導入することで、トレイト実装を各 Enum バリアントへ自動転送し、このボイラープレートを排除します。

本移行は、Enzyme AD との互換性を維持しつつ、既存の静的ディスパッチパターンを強化し、コードの可読性・保守性を向上させることを目的とします。

## Project Description (Input)
2. enum_dispatch (Enum のボイラープレート削減)
金融ライブラリでは、「Instrument Enum が Swap や Bond をラップし、それぞれの price() メソッドを呼び出す」というパターンが頻出します。通常、これには巨大な match 文が必要ですが、enum_dispatch はこれを消滅させます。

画期的な点:

トレイトの実装を Enum の各バリアントに自動転送します。

実行時コストはゼロ（動的ディスパッチ Box<dyn Trait> ではなく、静的な展開が行われるため高速）。

削減イメージ:

```rust
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
trait Pricer {
    fn price(&self) -> f64;
}

// Enum定義に属性をつけるだけ
#[enum_dispatch(Pricer)]
enum Instrument {
    Swap(SwapTrade),
    Bond(BondTrade),
    Option(OptionTrade),
}

// これが不要になる ↓
// impl Pricer for Instrument {
//     fn price(&self) -> f64 {
//         match self {
//             Instrument::Swap(t) => t.price(),
//             Instrument::Bond(t) => t.price(),
//             ...
//         }
//     }
// }
```

## Requirements

### Requirement 1: 依存関係の追加と設定

**Objective:** As a ライブラリ開発者, I want `enum_dispatch` クレートをワークスペースの依存関係として追加する, so that 全ての対象クレートで一貫してマクロを利用できる

#### Acceptance Criteria
1. The Neutryx workspace shall `[workspace.dependencies]` セクションに `enum_dispatch` を定義する
2. When `enum_dispatch` を使用するクレートを追加する場合, the crate shall `{ workspace = true }` 継承パターンを使用する
3. The `enum_dispatch` dependency shall バージョン `0.3` 以上を指定する
4. The workspace shall `cargo build --workspace` でエラーなくコンパイルする

### Requirement 2: 対象 Enum の識別

**Objective:** As a ライブラリ開発者, I want 手動 `match` 文でトレイト転送を行っている既存 Enum を特定する, so that 移行対象を明確化し、優先順位をつけられる

#### Acceptance Criteria
1. The migration scope shall Pricer 層の以下の Enum を含む:
   - `StochasticModelEnum` (`pricer_models::stochastic`)
   - `CurveEnum` (`pricer_models::market`)
   - `PathPayoffType` (`pricer_pricing::path_dependent`)
2. The migration scope shall Infra 層の以下の Enum を含む（該当する場合）:
   - `InstrumentType` または同等の Enum (`infra_master::trade`)
3. When 対象 Enum を選定する場合, the selection criteria shall 以下を満たすこと:
   - 3つ以上のバリアントを持つ
   - 手動 `match` 文によるトレイト impl が存在する
   - Enzyme AD との互換性要件を満たす

### Requirement 3: `StochasticModelEnum` の移行

**Objective:** As a クオンツ開発者, I want `StochasticModelEnum` の `StochasticModel` トレイト実装を `enum_dispatch` に移行する, so that 新規確率モデル追加時のボイラープレートを排除できる

#### Acceptance Criteria
1. The `StochasticModel` trait shall `#[enum_dispatch]` 属性で注釈する
2. The `StochasticModelEnum` enum shall `#[enum_dispatch(StochasticModel)]` 属性で注釈する
3. When 移行完了時, the codebase shall 手動 `match` 文による `StochasticModel` impl を含まない
4. The migrated code shall 既存の全てのユニットテストをパスする
5. The migrated code shall Enzyme AD モード（nightly ビルド）でコンパイルする

### Requirement 4: `CurveEnum` の移行

**Objective:** As a クオンツ開発者, I want `CurveEnum` の `YieldCurve` トレイト実装を `enum_dispatch` に移行する, so that 新規カーブタイプ追加時の保守性を向上させる

#### Acceptance Criteria
1. The `YieldCurve` trait shall `#[enum_dispatch]` 属性で注釈する
2. The `CurveEnum` enum shall `#[enum_dispatch(YieldCurve)]` 属性で注釈する
3. When 移行完了時, the codebase shall `CurveEnum` の手動 `impl YieldCurve` ブロックを含まない
4. The migrated code shall ブートストラップ機能の全テストをパスする
5. If ジェネリクス `<T: Float>` を使用している場合, the migration shall `enum_dispatch` のジェネリクスサポートを適切に処理する

### Requirement 5: `PathPayoffType` の移行

**Objective:** As a クオンツ開発者, I want `PathPayoffType` のペイオフ関連トレイト実装を `enum_dispatch` に移行する, so that パス依存オプションの拡張性を向上させる

#### Acceptance Criteria
1. The `PathDependentPayoff` trait（または同等のトレイト）shall `#[enum_dispatch]` 属性で注釈する
2. The `PathPayoffType` enum shall 対応する `#[enum_dispatch(...)]` 属性で注釈する
3. When Asian/Barrier/Lookback ペイオフを計算する場合, the enum_dispatch implementation shall 既存の手動 `match` と同一の結果を返す
4. The migrated code shall Monte Carlo シミュレーションのベンチマークで性能劣化がない

### Requirement 6: Enzyme AD 互換性の検証

**Objective:** As a パフォーマンス担当開発者, I want `enum_dispatch` が Enzyme LLVM プラグインと互換性があることを検証する, so that AD ベースの Greeks 計算が正常に動作することを保証できる

#### Acceptance Criteria
1. When `enzyme-ad` フィーチャーでビルドする場合, the migrated enums shall コンパイルエラーなくビルドする
2. The Enzyme AD shall `enum_dispatch` で生成されたコードに対して正しく微分を計算する
3. If 互換性の問題が発見された場合, the implementation shall 該当 Enum を移行対象から除外し、手動 `match` を維持する
4. The verification shall bump-and-revalue との比較テストを実施する

### Requirement 7: コード品質と一貫性

**Objective:** As a ライブラリメンテナ, I want 移行後のコードが既存のコード品質基準を満たす, so that コードベースの一貫性と保守性を維持できる

#### Acceptance Criteria
1. The migrated code shall `cargo clippy --workspace -- -D warnings` をパスする
2. The migrated code shall `cargo fmt --all -- --check` をパスする
3. The documentation shall 移行した各トレイトと Enum に対して適切なドキュメントコメントを含む
4. When 新規 Enum バリアントを追加する場合, the developer experience shall トレイト impl の自動生成により改善される
5. The codebase shall 移行対象の手動 `match` によるトレイト転送パターンを排除する

### Requirement 8: 既存 API の後方互換性

**Objective:** As a ライブラリ利用者, I want 移行後も既存の公開 API が変更なく動作する, so that 依存コードの修正なしにアップグレードできる

#### Acceptance Criteria
1. The public API shall 関数シグネチャ、型定義、エクスポートに変更がない
2. When 既存コードから移行後の Enum を使用する場合, the behavior shall 移行前と同一である
3. The migration shall セマンティックバージョニングにおいて破壊的変更とならない
4. If 内部実装の変更が外部に影響する場合, the documentation shall その変更を明記する

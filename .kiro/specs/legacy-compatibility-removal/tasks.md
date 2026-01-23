# Implementation Plan

## Task List

- [x] 1. Deprecated convention モジュールの削除
- [x] 1.1 deprecated モジュールファイルとディレクトリを削除
  - infra_master から後方互換性シムを完全に削除
  - lib.rs からモジュール宣言と関連ドキュメントコメントを削除
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2. pricer_core クロスレイヤー re-export の削除
- [x] 2.1 (P) types/mod.rs から infra_master re-export を削除
  - BusinessDayConvention, Currency, Date, DayCounter の re-export 文を削除
  - _Requirements: 2.1_

- [x] 2.2 (P) types/time.rs から infra_master re-export を削除
  - BusinessDayConvention, Date, DayCounter の re-export 文を削除
  - DayCountConvention 型は pricer_core 独自型として保持
  - _Requirements: 2.2, 2.5_

- [x] 2.3 (P) types/error.rs から infra_master re-export を削除
  - CurrencyError, DateError の re-export 文を削除
  - _Requirements: 2.3_

- [x] 2.4 関連ドキュメントコメントを更新
  - re-export に言及するコメントを削除または修正
  - _Requirements: 2.4_

- [x] 3. pricer_models クロスレイヤー re-export の削除
- [x] 3.1 lib.rs から SwapDirection, TradeDirection の re-export を削除
  - infra_master 型の re-export 文を削除
  - SwapDirectionExt, TradeDirectionExt trait の re-export は保持
  - _Requirements: 3.1, 3.2_

- [x] 3.2 関連ドキュメントコメントを更新
  - re-export に言及するコメントを修正
  - _Requirements: 3.3_

- [ ] 4. 責務外テストの削除
- [ ] 4.1 module_exports.rs から責務違反テスト関数を削除
  - infra_master 型をテストするテスト関数を削除
  - pricer_core 独自機能のテストは保持
  - 削除後にファイルが空になる場合はファイル自体を削除
  - _Requirements: 4.1, 4.2_

- [ ] 4.2 テストファイルのインポート修正
  - 削除に伴い不要になったインポートを整理
  - 残存テストに必要なインポートを確認
  - _Requirements: 4.3_

- [ ] 5. pricer_models 重複型定義の削除
- [ ] 5.1 BusinessDayAdjustment enum を削除し infra_master 型に置換
  - date_utils.rs から BusinessDayAdjustment 定義を削除
  - DateCalculator で BusinessDayConvention を使用するよう修正
  - Following, ModifiedFollowing, Preceding のマッピングを適用
  - _Requirements: 5.1, 5.2, 5.5_

- [ ] 5.2 DayCount enum を削除し infra_master 型に置換
  - date_utils.rs から DayCount 定義を削除
  - DateCalculator で DayCounter を使用するよう修正
  - Act360 → Actual360, Act365Fixed → Actual365Fixed, Thirty360 → Thirty360Bond のマッピングを適用
  - _Requirements: 5.3, 5.4, 5.5_

- [ ] 5.3 bootstrapping モジュールの re-export を更新
  - mod.rs から削除された型の re-export を除去
  - SpotDateConvention は保持
  - _Requirements: 5.6_

- [ ] 6. CurrencyPair を FxRate にリネーム
- [ ] 6.1 currency_pair.rs で CurrencyPair<T> を FxRate<T> にリネーム
  - 構造体名を変更
  - 関連する impl ブロックと trait 実装を更新
  - ドキュメントコメントで infra_master::CurrencyPair との違いを明記
  - _Requirements: 6.1, 6.2, 6.4_

- [ ] 6.2 FxRate への参照を全箇所で更新
  - pricer_core 内の全参照を新しい名前に更新
  - テストコードの参照も更新
  - _Requirements: 6.3_

- [ ] 10. ID Newtype パターンの統一（Stringly Typed 解消）
- [ ] 10.1 infra_master::ids モジュールを新設
  - src/ids/mod.rs を作成
  - define_id! マクロで共通実装を生成
  - TradeId, CounterpartyId, PortfolioId, BookId, IssuerId, NettingSetId, LegalEntityId, CcpId を定義
  - 各型に Clone, Debug, PartialEq, Eq, Hash, Display, From<&str>, From<String> を実装
  - CounterpartyId と NettingSetId のみ Default を実装（既存互換性）
  - _Requirements: 10.1, 10.2_

- [ ] 10.2 infra_master::lib.rs で ids モジュールを公開
  - `pub mod ids;` を追加
  - `pub use ids::*;` で全 ID 型をルートエクスポート
  - _Requirements: 10.8_

- [ ] 10.3 trade.rs から TradeId 型エイリアスを削除
  - `pub type TradeId = String;` を削除
  - `use crate::ids::TradeId;` に置き換え
  - Trade 構造体と関連メソッドを更新
  - _Requirements: 10.3_

- [ ] 10.4 TradeMetadata を Newtype ID で更新
  - counterparty: Option<String> → Option<CounterpartyId>
  - portfolio: Option<String> → Option<PortfolioId>
  - book: Option<String> → Option<BookId>
  - with_counterparty, with_portfolio, with_book メソッドを更新
  - _Requirements: 10.4_

- [ ] 10.5 TradeType::Bond を Newtype ID で更新
  - issuer_id: Option<String> → Option<IssuerId>
  - _Requirements: 10.5_

- [ ] 10.6 trade.rs のテストを更新
  - TradeId::new() を使用するよう変更
  - TradeMetadata のテストを ID 型で更新
  - _Requirements: 10.4, 10.5_

- [ ] 10.7 pricer_risk::portfolio::ids を re-export 方式に変更
  - 既存の TradeId, CounterpartyId, NettingSetId 定義を削除
  - infra_master::ids からの re-export に置き換え
  - テストが依然としてパスすることを確認
  - _Requirements: 10.6_

- [ ] 10.8 service_gateway のテストを更新
  - TradeId::new() の使用箇所を確認・更新
  - _Requirements: 10.7_

- [ ] 7. 依存コードのインポート更新
- [ ] 7.1 pricer_risk クレートのインポートを更新
  - pricer_core::types からのインポートを infra_master に変更
  - Date, Currency, DayCounter, BusinessDayConvention を直接インポート
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 7.2 (P) service_cli クレートのインポートを更新
  - infra_master から型を直接インポートするよう変更
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 7.3 (P) adapter_feeds クレートのインポートを更新
  - infra_master から型を直接インポートするよう変更
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 7.4 その他の依存クレートを検証・更新
  - pricer_models 内の provider.rs を更新
  - demo クレートがある場合は更新
  - SwapDirection, TradeDirection のインポート先を更新
  - _Requirements: 7.5, 7.6, 7.7, 7.8, 7.9_

- [ ] 8. ステアリングドキュメントの更新
- [ ] 8.1 structure.md を更新
  - pricer_core セクションから re-export の記述を削除
  - pricer_models の date_utils モジュール記述を更新
  - 各クレートの責務境界を明確化
  - _Requirements: 8.1, 8.2, 8.4_

- [ ] 8.2 infra_master::convention への参照を削除
  - 削除されたモジュールへの言及を全て除去
  - _Requirements: 8.3_

- [ ] 9. 最終ビルド・テスト検証
- [ ] 9.1 ワークスペース全体のビルド検証
  - cargo build --workspace が成功することを確認
  - cargo clippy --workspace -- -D warnings が警告なしで完了
  - _Requirements: 9.1, 9.3_

- [ ] 9.2 ワークスペース全体のテスト検証
  - cargo test --workspace が全テストパス
  - deprecated 警告が残っていないことを確認
  - _Requirements: 9.2, 9.5_

- [ ] 9.3 ドキュメント生成の検証
  - cargo doc --workspace --no-deps がエラーなしで完了
  - _Requirements: 9.4_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 1.1 |
| 2 | 2.1, 2.2, 2.3, 2.4 |
| 3 | 3.1, 3.2 |
| 4 | 4.1, 4.2 |
| 5 | 5.1, 5.2, 5.3 |
| 6 | 6.1, 6.2 |
| 7 | 7.1, 7.2, 7.3, 7.4 |
| 8 | 8.1, 8.2 |
| 9 | 9.1, 9.2, 9.3 |
| 10 | 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8 |

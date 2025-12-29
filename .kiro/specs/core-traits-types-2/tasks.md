# Implementation Plan

## Tasks

- [x] 1. serde Feature Flag の設定
- [x] 1.1 Cargo.toml に serde feature flag を追加
  - serde をオプショナル依存として設定し、default feature で有効化
  - chrono の serde feature も連動して有効化
  - feature flag 無効時にコンパイルが通ることを確認
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 2. エラー型の実装
- [x] 2.1 (P) DateError 列挙型の実装
  - 無効な日付コンポーネント用の InvalidDate variant を追加（year, month, day をフィールドに持つ）
  - 文字列パース失敗用の ParseError variant を追加
  - Display トレイトで人間が読みやすいエラーメッセージを生成
  - std::error::Error トレイトを実装
  - _Requirements: 7.1, 7.3, 7.4_

- [x] 2.2 (P) CurrencyError 列挙型の実装
  - 未知の通貨コード用の UnknownCurrency variant を追加
  - 文字列パース失敗用の ParseError variant を追加
  - Display トレイトで入力値を含むエラーメッセージを生成
  - std::error::Error トレイトを実装
  - _Requirements: 7.2, 7.3, 7.5_

- [x] 3. Date 構造体の実装
- [x] 3.1 Date 構造体の基本定義
  - chrono::NaiveDate をラップする newtype 構造体を定義
  - Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash を derive
  - serde feature 有効時に Serialize, Deserialize を条件付き derive（transparent で ISO 8601 形式）
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 3.2 Date コンストラクタの実装
  - from_ymd で年月日から日付を作成、無効な場合は DateError::InvalidDate を返す
  - today でシステム日付を取得
  - parse で ISO 8601 形式文字列をパース、失敗時は DateError::ParseError を返す
  - _Requirements: 1.6, 1.7_

- [x] 3.3 Date アクセサと演算の実装
  - year, month, day で各コンポーネントを取得
  - into_inner で内部 NaiveDate を取得
  - Sub トレイトで二つの日付の差分を日数（i64）として返す
  - FromStr トレイトを parse メソッドに委譲
  - Display トレイトで ISO 8601 形式を出力
  - _Requirements: 1.5, 1.6_

- [x] 4. Currency 列挙型の実装
- [x] 4.1 Currency 列挙型の基本定義
  - USD, EUR, GBP, JPY, CHF の 5 つの variant を持つ enum を定義
  - #[non_exhaustive] で将来の拡張に備える
  - Copy, Clone, Debug, PartialEq, Eq, Hash を derive
  - serde feature 有効時に Serialize, Deserialize を条件付き derive
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.7_

- [x] 4.2 Currency メソッドの実装
  - code で ISO 4217 の 3 文字通貨コードを返す
  - decimal_places で通貨ごとの標準小数点以下桁数を返す（JPY は 0、その他は 2）
  - FromStr で大文字小文字を無視してパース、未知のコードは CurrencyError::UnknownCurrency を返す
  - Display で ISO 4217 コードを出力
  - _Requirements: 2.5, 2.6_

- [x] 5. DayCountConvention の拡張
- [x] 5.1 DayCountConvention に name メソッドを追加
  - 各 variant に対応する業界標準名称を返す（"ACT/365", "ACT/360", "30/360"）
  - _Requirements: 3.4_

- [x] 5.2 DayCountConvention のカスタムシリアライゼーション
  - Serialize で業界標準名称を出力
  - Deserialize で大文字小文字を無視してパース
  - エイリアス（"Actual/365", "Act365" など）もサポート
  - 未知の規約名に対しては説明的なエラーメッセージを返す
  - _Requirements: 3.1, 3.2, 3.3, 3.5_

- [x] 5.3 Date 型を使用した year fraction 計算
  - year_fraction_dates メソッドを追加し、Date 型で日付を受け取る
  - start > end の場合は panic せず負の値を返す
  - time_to_maturity_dates 便利関数を追加
  - 結果は既存の year_fraction(NaiveDate) と同等であることを保証
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 6. モジュール公開と統合
- [x] 6.1 types モジュールの公開設定
  - types/mod.rs で Date, Currency, DateError, CurrencyError を re-export
  - 既存の DayCountConvention, PricingError との整合性を確認
  - lib.rs からの公開パスを確認
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 7. テストと検証
- [x] 7.1 (P) Date 単体テスト
  - from_ymd の正常系（通常日付、閏年 2 月 29 日）と異常系（2 月 30 日、13 月）
  - 日付減算で正しい日数が返ることを検証
  - serde 往復テスト（シリアライズ→デシリアライズで同値）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

- [x] 7.2 (P) Currency 単体テスト
  - 全 variant の code と decimal_places の正確性
  - FromStr の大文字小文字無視パース
  - serde 往復テスト
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [x] 7.3 (P) DayCountConvention 拡張テスト
  - name メソッドの正確性
  - カスタム serde のエイリアスパース
  - year_fraction_dates と year_fraction の結果一致
  - start > end での負値返却（panic しないことを確認）
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 4.3, 4.4_

- [x] 7.4 Feature Flag テスト
  - --no-default-features でコンパイル成功を確認
  - serde feature 有効時の derive が機能することを確認
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 7.5 (P) ドキュメンテーション検証
  - 全 doc comment が British English であることを確認
  - Date, Currency に Examples セクションがあることを確認
  - cargo doc --no-deps でドキュメント生成が成功することを確認
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x]* 7.6 (P) Property-based テスト
  - Date::from_ymd で作成した日付は常に valid
  - Currency パース往復の恒等性
  - year_fraction_dates の結果が常に有限値
  - _Requirements: 1.7, 2.5, 4.4_

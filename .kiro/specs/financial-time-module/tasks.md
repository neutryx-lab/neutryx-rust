# Implementation Plan

## Overview

Financial Time Module の実装タスク。3 フェーズ（構造移行 → 機能拡張 → Trait 化）で段階的に実装する。

---

## Phase 1: 構造移行

- [x] 1. time/ サブディレクトリ構造の作成
- [x] 1.1 (P) エラー型モジュールの作成
  - `TimeError` enum を定義し、`InvalidDate`, `ParseError`, `CalculationError`, `CalendarError` バリアントを含める
  - `thiserror::Error` を derive し、各バリアントに適切なエラーメッセージを設定
  - `Debug`, `Clone`, `PartialEq`, `Eq` を derive
  - `DateError` を deprecated alias として定義
  - 単体テストを追加（Display, Clone, PartialEq の検証）
  - _Requirements: 2_

- [x] 1.2 (P) 日付型モジュールの移行
  - 既存の `date.rs` を `time/types.rs` に移動
  - import パスを新しいモジュール構造に合わせて更新
  - `TimeError` を使用するように変更（`DateError` から）
  - 既存のテストが全てパスすることを確認
  - _Requirements: 1_

- [x] 1.3 (P) 日数計算規約モジュールの移行
  - 既存の `day_count.rs` を `time/day_counters.rs` に移動
  - `DayCounter` にリネーム（`DayCountConvention` から）
  - `DayCountConvention` を deprecated alias として定義
  - 既存のテストが全てパスすることを確認
  - _Requirements: 7_

- [x] 1.4 (P) 営業日規約モジュールの移行
  - 既存の `business_day.rs` を `time/calendars.rs` に統合準備
  - `BusinessDayConvention` の 5 種の営業日調整規約を維持
  - 既存の `name()`, `code()`, `FromStr`, `Display` 実装を維持
  - 既存のテストが全てパスすることを確認
  - _Requirements: 6_

- [x] 1.5 (P) 期間・テナーモジュールの移行
  - 既存の `tenor.rs` と `period.rs` を `time/period.rs` に統合
  - `Tenor` enum（17 種）と `EndOfMonthRule` を維持
  - 既存 `Period` を `AccrualPeriod` にリネーム
  - 既存のテストが全てパスすることを確認
  - _Requirements: 9, 10, 11_

- [x] 1.6 モジュール定義と re-exports の作成
  - `time/mod.rs` を作成し、すべてのサブモジュールを宣言
  - 公開 API として必要な型を re-export
  - deprecated aliases を `time/mod.rs` に追加
  - _Requirements: 1_

- [x] 1.7 lib.rs の更新と後方互換性の確保
  - `pub mod time;` を `lib.rs` に追加
  - クレートルートで time 型を re-export
  - 旧パスからの deprecated re-exports を設定
  - すべての既存 doctest と integration test がパスすることを確認
  - _Requirements: 1, 12_

---

## Phase 2: 機能拡張

- [x] 2. Date 型への Excel Serial 変換機能の追加
- [x] 2.1 to_serial メソッドの実装
  - Excel serial date（1900-01-01 = 1）への変換メソッドを追加
  - Excel の leap year bug（1900-02-28 以降 +1）を考慮した補正を実装
  - 1900-01-01 = 1, 1900-02-28 = 59, 1900-03-01 = 61 のテストケースを追加
  - _Requirements: 3_

- [x] 2.2 from_serial メソッドの実装
  - serial number から Date を構築するメソッドを追加
  - 無効な serial number（負数、範囲外）に対して `TimeError::CalculationError` を返却
  - serial 60（1900-02-29、無効日付）に対するエラーハンドリングを実装
  - 往復変換テスト（to_serial → from_serial）を追加
  - _Requirements: 3_

- [x] 3. 汎用 Period と TimeUnit の実装
- [x] 3.1 (P) TimeUnit enum の定義
  - `Days`, `Weeks`, `Months`, `Years` の 4 種を定義
  - `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` を derive
  - Serde feature flag に対応
  - `Display` trait を実装（"D", "W", "M", "Y"）
  - _Requirements: 8_

- [x] 3.2 (P) Period struct の定義
  - `length: i32` と `units: TimeUnit` フィールドを持つ構造体を定義
  - `new(length, units)` コンストラクタを実装
  - 便利メソッド `days(n)`, `weeks(n)`, `months(n)`, `years(n)` を実装
  - `Display` trait を実装（例: "3M", "1Y"）
  - _Requirements: 8_

- [x] 3.3 Date + Period の Add trait 実装
  - `Date + Period -> Date` の演算を実装
  - Days, Weeks は単純な日数加算
  - Months, Years は chrono の月加算を使用
  - 月末調整（1月31日 + 1ヶ月 = 2月28/29日）のテストを追加
  - _Requirements: 8_

- [x] 3.4 Tenor::to_period メソッドの追加
  - 既存 Tenor enum に `to_period(&self) -> Period` メソッドを追加
  - 各 Tenor を適切な Period に変換（例: ThreeMonths → Period::months(3)）
  - テストを追加
  - _Requirements: 9_

---

## Phase 3: Calendar Trait 化

- [x] 4. Calendar trait と ConcreteCalendar の実装
- [x] 4.1 Calendar trait の定義
  - `is_business_day(&self, date: Date) -> bool` を必須メソッドとして定義
  - `Send + Sync` 要件を追加
  - デフォルト実装: `is_holiday`, `next_business_day`, `prev_business_day`, `add_business_days`, `adjust`
  - 各デフォルトメソッドが `is_business_day` に基づいて動作することを確認
  - _Requirements: 4_

- [x] 4.2 ConcreteCalendar struct の実装
  - 既存の `Calendar` struct を `ConcreteCalendar` にリネーム
  - `Calendar` trait を実装
  - `CalendarId` による 5 種のカレンダー（Target, NewYork, Tokyo, London, WeekendOnly）をサポート
  - `get(id)` ファクトリメソッドを維持
  - 既存のテストが全てパスすることを確認
  - _Requirements: 4_

- [x] 4.3 BusinessDayConvention の adjust 動作検証
  - `Following`: 次の営業日への調整テスト
  - `ModifiedFollowing`: 月をまたがない検証テスト
  - `Preceding`: 前の営業日への調整テスト
  - `ModifiedPreceding`: 月をまたがない検証テスト
  - `Unadjusted`: 調整なしテスト
  - _Requirements: 6, 13_

- [x] 5. JointCalendar の実装
- [x] 5.1 JointCalendarRule enum の定義
  - `JoinHolidays`: 全カレンダーが営業日の場合のみ営業日（休日の和集合）
  - `JoinBusinessDays`: いずれかが営業日なら営業日（営業日の和集合）
  - Serde feature flag に対応
  - _Requirements: 5_

- [x] 5.2 JointCalendar struct の実装
  - `Vec<Box<dyn Calendar>>` と `JointCalendarRule` を保持
  - `new(calendars, rule)` コンストラクタを実装
  - `Calendar` trait を実装
  - _Requirements: 5_

- [x] 5.3 JointCalendar の結合ロジックテスト
  - `JoinHolidays`: NY と Tokyo の両方が営業日の場合のみ営業日となることを検証
  - `JoinBusinessDays`: いずれかが営業日なら営業日となることを検証
  - 結合ルールに従った `adjust`, `next_business_day`, `prev_business_day`, `add_business_days` の動作を検証
  - _Requirements: 5, 13_

---

## Phase 4: 統合とテスト

- [x] 6. DayCounter のテスト強化
- [x] 6.1 day_count メソッドの追加
  - `day_count(&self, d1: Date, d2: Date) -> i64` メソッドを追加
  - 既存の `year_fraction` メソッドとの整合性を確認
  - _Requirements: 7_

- [x] 6.2 30/360 計算の検証テスト
  - US Bond Basis vs European の違いを検証
  - 月末日のエッジケースをテスト
  - QuantLib の既知のテストケースとの照合
  - _Requirements: 7, 13_

- [x] 7. 統合テストと後方互換性の最終確認
- [x] 7.1 後方互換性のコンパイルテスト
  - 旧型名（`DateError`, `DayCountConvention`, `Calendar`）でのコンパイル成功を確認
  - deprecated 警告が正しく表示されることを確認
  - _Requirements: 12_

- [x] 7.2 既存テストの完全パス確認
  - `cargo test -p infra_master` で全テストがパスすることを確認
  - doctests が全てパスすることを確認
  - _Requirements: 13_

- [x] 7.3 依存クレートとの連携確認
  - `pricer_core` からの deprecated re-exports が正常に動作することを確認
  - `pricer_models` からの re-exports が正常に動作することを確認
  - _Requirements: 12_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7 |
| 2 | 1.1 |
| 3 | 2.1, 2.2 |
| 4 | 4.1, 4.2 |
| 5 | 5.1, 5.2, 5.3 |
| 6 | 1.4, 4.3 |
| 7 | 1.3, 6.1, 6.2 |
| 8 | 3.1, 3.2, 3.3 |
| 9 | 1.5, 3.4 |
| 10 | 1.5 |
| 11 | 1.5 |
| 12 | 1.7, 7.1, 7.3 |
| 13 | 4.3, 5.3, 6.2, 7.2 |

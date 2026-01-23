# Implementation Plan

## Task Format Template

> **Parallel marker**: `(P)` はデータ依存なしで並行実行可能なタスクを示す。

---

## Tasks

- [x] 1. エラー型の定義
- [x] 1.1 (P) カーブエンジン統合エラー型を作成する
  - 設定エラー（無効なフィールド名と理由を含む）を定義
  - Instrumentエラー（テナーと理由を含む）を定義
  - Bootstrapエラーのラッピングを実装
  - 補間エラー、キャッシュエラー、設定パースエラーを定義
  - 循環依存エラーと無効な組み合わせエラーを定義
  - `thiserror`クレートでエラー型を派生
  - エラーヘルパーメソッド（`configuration()`, `instrument()`, `is_bootstrap_error()`）を提供
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

- [x] 2. 設定型の定義
- [x] 2.1 (P) パラメータ表現列挙型を作成する
  - LogDiscountFactor（デフォルト）、ZeroRate、InstantaneousForwardの3種を定義
  - serde対応（feature-gated）を追加
  - 各表現の数学的意味をドキュメント化
  - _Requirements: 2.1, 2.3, 2.4_

- [x] 2.2 拡張カーブ設定構造体を作成する
  - 既存の`GenericBootstrapConfig<T>`を包含
  - パラメータ表現種別フィールドを追加
  - 外挿設定と負金利許可フラグを管理
  - 設定バリデーションメソッドを実装（補間器との互換性チェック）
  - デフォルト設定生成メソッドを提供
  - serde対応（flatten構造）を追加
  - 無効な組み合わせ時にエラーを返す
  - _Requirements: 2.1, 2.2, 2.5, 2.6, 2.7_

- [x] 3. カーブ定義の実装
- [x] 3.1 Instrument仕様構造体を作成する
  - カーブ構築用Instrument種別列挙型（OIS, IRS, FRA, Future, Deposit）を定義
  - Instrument仕様にテナー（満期）を含める
  - オプションのConvexity調整フィールドを追加
  - serde対応を追加
  - _Requirements: 1.2, 1.3_

- [x] 3.2 カーブ定義構造体を作成する
  - Indexキー（文字列）とRateIndex参照を保持
  - Instrument仕様リストを含める
  - 参照コンベンション（SwapConvention）を保持
  - Instrument仕様を満期順にソートするメソッドを提供
  - infra_masterのRateIndexとの互換性を確保
  - _Requirements: 1.1, 1.4, 1.6_

- [x] 3.3 カーブ定義のシリアライゼーションを実装する
  - JSONファイルからのロードメソッドを実装
  - 組み込みIndex用のデフォルト定義取得メソッドを実装
  - 未知のIndex指定時に`UnknownIndex`エラーを返す
  - スキーマバリデーションを実装
  - _Requirements: 1.5, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [x] 4. Instrument変換アダプターの実装
- [x] 4.1 OISとIRS Instrumentの変換を実装する
  - `infra_master::trade::convention::SwapConvention`からOIS用`BootstrapInstrument`を生成
  - IRS用にFixed LegとFloat Legのキャッシュフロースケジュールを`infra_master::trade::Cashflow`で展開
  - コンベンション（DayCount, BDC, PaymentFrequency）を適用
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 4.2 FRAとFuture Instrumentの変換を実装する
  - FRA定義から期間（start, end）とレートを抽出
  - Future定義から価格と満期を抽出しConvexity調整を適用
  - Instrument定義が不完全な場合に`IncompleteInstrumentDefinition`エラーを返す
  - _Requirements: 3.4, 3.5, 3.6_

- [x] 4.3 統合変換メソッドを実装する
  - CurveDefinitionとレート配列からBootstrapInstrumentリストを生成
  - レート配列とInstrument定義の整合性を検証
  - 生成されたInstrumentの満期が正順であることを保証
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 5. 結果キャッシュの実装
- [ ] 5.1 (P) キャッシュキーとハッシュ計算を実装する
  - RateIndex、入力レート配列のハッシュ、設定のハッシュをキーとして定義
  - `ordered-float`を使用してf64配列の決定論的ハッシュを計算
  - 同一入力で同一ハッシュ、異なる入力で異なるハッシュを保証
  - _Requirements: 7.1_

- [ ] 5.2 LRU結果キャッシュを実装する
  - `lru::LruCache`を使用してカーブをキャッシュ
  - キャッシュサイズ上限（エントリ数）を設定可能にする
  - LRU方式でキャッシュエビクションを実行
  - キャッシュのルックアップと挿入メソッドを提供
  - キャッシュクリアメソッドを提供
  - _Requirements: 7.2, 7.3, 7.4, 7.5, 7.6_

- [ ] 5.3 キャッシュのスレッドセーフ対応を実装する
  - `parking_lot::RwLock`で読み書きロックを実装
  - 読み取り操作は並行可能、書き込み操作は排他制御
  - マルチスレッド環境でのスレッドセーフなアクセスを保証
  - _Requirements: 7.7_

- [ ] 5.4 キャッシュ統計を実装する
  - ヒット数、ミス数、エントリ数を追跡
  - ヒット率計算メソッドを提供
  - 統計取得メソッドを提供
  - _Requirements: 7.8_

- [ ] 6. YieldCurveトレイト拡張
- [ ] 6.1 (P) 瞬間フォワードレートメソッドを追加する
  - `instantaneous_forward(t: T) -> Result<T, MarketDataError>`をトレイトに追加
  - 数学的定義をドキュメント化: f(t) = -d/dt ln(D(t))
  - デフォルト実装で`NotImplemented`エラーを返す
  - BootstrappedCurveで補間器の解析微分を使用した実装を提供
  - LogDF表現とZeroRate表現の両方に対応
  - _Requirements: 5.4_

- [ ] 6.2 Pillarアクセスメソッドを追加する
  - `pillar_count()`と`pillars()`をトレイトに追加
  - デフォルト実装で`None`を返す
  - BootstrappedCurveで具体的な実装を提供
  - pillar点の一覧と対応するパラメータ値を取得可能にする
  - _Requirements: 5.6_

- [ ] 6.3 カーブインターフェースの整合性を検証する
  - discount_factor、zero_rate、forward_rateの既存メソッドとの整合性を確認
  - 時間tがカーブ範囲外かつ外挿禁止時に`OutOfBounds`エラーを返すことを確認
  - ジェネリック型`T`（f64, Dual等）のサポートを確認
  - _Requirements: 5.1, 5.2, 5.3, 5.5, 5.7_

- [ ] 7. カーブエンジンの実装
- [ ] 7.1 基本カーブ構築オーケストレーションを実装する
  - CurveDefinitionと入力レートからカーブを構築する主要メソッドを実装
  - InstrumentAdapterを使用してBootstrapInstrumentに変換
  - 既存のSequentialBootstrapperを使用してBootstrapを実行
  - 構築結果として残差ベクトルと収束ステータスを返す
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [ ] 7.2 キャッシュ統合を実装する
  - キャッシュなしとキャッシュ付きの両方でエンジンを作成可能にする
  - カーブ構築前にキャッシュをルックアップ
  - キャッシュヒット時は再計算をスキップ
  - キャッシュミス時は構築後にキャッシュに格納
  - キャッシュ統計取得とクリアメソッドを提供
  - _Requirements: 7.2, 7.3_

- [ ] 7.3 感度計算対応を実装する
  - 感度付きカーブ構築メソッドを提供
  - 既存のSensitivityBootstrapperと統合
  - Implicit Function Theoremを使用してAD tapeにsolver反復を記録しない
  - 各pillarのDF/パラメータに対する入力レート感度（Jacobian）を計算
  - `BootstrapResultWithSensitivities`を返す
  - Dual型でのインスタンス化をサポート
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 8. マルチカーブ対応の拡張
- [ ] 8.1 CurveSetを拡張する
  - Index名でカーブを検索するメソッドを追加
  - 複数カーブの集合を管理
  - _Requirements: 8.3, 8.4_

- [ ] 8.2 MultiCurveBuilderを拡張する
  - Discountカーブ（OIS-based）とProjectionカーブ（IBOR-based）の同時構築をサポート
  - Projectionカーブ構築時に別途構築されたDiscountカーブを参照
  - 依存関係に従って構築順序を自動決定
  - 循環依存が検出された場合に`CircularDependency`エラーを返す
  - _Requirements: 8.1, 8.2, 8.5, 8.6_

- [ ] 9. 統合テスト
- [ ] 9.1 単体テストを作成する
  - CurveDefinition::load_from_json()の有効/無効なJSONでのロードテスト
  - InstrumentAdapter::convert()のOIS/IRS/FRA/Future各種変換テスト
  - CurveResultCache::lookup()/insert()のキャッシュヒット/ミステスト
  - CurveConfig::validate()の有効/無効な設定組み合わせテスト
  - CurveKeyハッシュの決定性テスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

- [ ] 9.2 統合テストを作成する
  - CurveEngine::build_curve()の定義→変換→構築の一連フローテスト
  - キャッシュ統合テスト（2回目の呼び出しでキャッシュヒット確認）
  - MultiCurveBuilderのOIS Discount + Tenor Curveの同時構築テスト
  - SensitivityBootstrapperとの統合テスト（感度計算の整合性）
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

- [ ]* 9.3 パフォーマンステストを作成する
  - キャッシュヒット時の応答時間テスト（構築時の10%以下）
  - 並列アクセス時のスループット評価（RwLock競合）
  - メモリ使用量テスト（100カーブキャッシュ時のフットプリント）
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1 (Index-Curve Definition) | 3.1, 3.2, 3.3, 9.1 |
| 2 (Curve Parameter Config) | 2.1, 2.2, 9.1 |
| 3 (Instrument Integration) | 4.1, 4.2, 4.3, 9.1 |
| 4 (Bootstrap Engine) | 7.1, 9.2 |
| 5 (Generic Curve Interface) | 6.1, 6.2, 6.3 |
| 6 (AD Computation Graph) | 7.3, 9.2 |
| 7 (Curve Caching) | 5.1, 5.2, 5.3, 5.4, 7.2, 9.1, 9.3 |
| 8 (Multi-Curve Support) | 8.1, 8.2, 9.2 |
| 9 (Error Handling) | 1.1 |
| 10 (Config Serialisation) | 3.3 |

---

## Implementation Notes

- **並行実行可能タスク**: 1.1, 2.1, 5.1, 6.1 はデータ依存がなく並行実行可能
- **依存関係**: タスク3以降はタスク1（エラー型）に依存、タスク7はタスク3-6に依存
- **既存コードへの影響**: YieldCurveトレイト拡張はデフォルト実装で後方互換性を維持
- **新規依存**: `lru` ^0.12, `ordered-float` ^4.0 をpricer_modelsに追加

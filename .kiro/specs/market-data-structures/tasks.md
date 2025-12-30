# Implementation Plan

## Tasks

- [ ] 1. モジュール基盤とエラー型の実装
- [ ] 1.1 market_data モジュール構造の作成
  - pricer_core に market_data モジュールを追加
  - サブモジュール構成を定義（curves, surfaces, error）
  - lib.rs に公開エクスポートを追加
  - _Requirements: 7.1_

- [ ] 1.2 MarketDataError 型の実装
  - InvalidMaturity、InvalidStrike、InvalidExpiry、OutOfBounds バリアントを定義
  - InsufficientData バリアントを追加
  - InterpolationError からの From 変換を実装
  - PricingError への統合変換を実装
  - thiserror による Display 実装
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 2. YieldCurve 抽象化の実装
- [ ] 2.1 (P) YieldCurve trait の定義
  - T: Float ジェネリック trait を定義
  - discount_factor メソッドを必須として宣言
  - zero_rate のデフォルト実装を提供（-ln(D(t))/t）
  - forward_rate のデフォルト実装を提供
  - 負の満期に対するエラーハンドリング仕様を明記
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [ ] 2.2 FlatCurve 構造体の実装
  - 定数金利パラメータを保持する構造体を作成
  - new コンストラクタと rate アクセサを実装
  - Copy, Clone, Debug derive を追加
  - _Requirements: 2.1, 2.2_

- [ ] 2.3 FlatCurve の YieldCurve 実装
  - discount_factor を exp(-r * t) として実装
  - 負の満期チェックとエラー返却
  - zero_rate と forward_rate のオーバーライド（効率化）
  - f64 および Dual64 でのテスト
  - _Requirements: 2.3, 2.4, 2.5, 8.1_

- [ ] 3. InterpolatedCurve の実装
- [ ] 3.1 CurveInterpolation 列挙型の定義
  - Linear、LogLinear バリアントを定義
  - 将来の拡張用に CubicSpline を予約
  - _Requirements: 3.4, 3.5_

- [ ] 3.2 InterpolatedCurve 構造体の実装
  - テナーとレートのピラーポイントを格納
  - 補間方式と外挿許可フラグを設定
  - コンストラクタで入力検証（ソート済み、最小2点）
  - domain メソッドでテナー範囲を返却
  - _Requirements: 3.1, 3.2, 3.6_

- [ ] 3.3 InterpolatedCurve の補間ロジック実装
  - LinearInterpolator との統合
  - Linear 補間によるゼロレート導出
  - LogLinear 補間による割引因子導出
  - 外挿時のフラット外挿またはエラー返却
  - _Requirements: 3.3, 3.4, 3.5, 3.6_

- [ ] 3.4 InterpolatedCurve の YieldCurve 実装
  - 補間されたゼロレートから割引因子を計算
  - 境界条件とエラーハンドリング
  - 単体テストで補間精度を検証
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 4. VolatilitySurface 抽象化の実装
- [ ] 4.1 (P) VolatilitySurface trait の定義
  - T: Float ジェネリック trait を定義
  - volatility(strike, expiry) メソッドを宣言
  - strike_domain と expiry_domain メソッドを定義
  - 無効ストライク/満期に対するエラー仕様を明記
  - _Requirements: 4.1, 4.2, 4.4, 4.5_

- [ ] 4.2 FlatVol 構造体の実装
  - 定数ボラティリティパラメータを保持
  - new コンストラクタと sigma アクセサを実装
  - Copy, Clone, Debug derive を追加
  - _Requirements: 5.1, 5.2_

- [ ] 4.3 FlatVol の VolatilitySurface 実装
  - 全ストライク/満期で定数ボラティリティを返却
  - 正のストライク/満期のみ受け入れ
  - ドメインを (0, infinity) として報告
  - f64 および Dual64 でのテスト
  - _Requirements: 5.3, 5.4, 8.1_

- [ ] 5. InterpolatedVolSurface の実装
- [ ] 5.1 InterpolatedVolSurface 構造体の実装
  - ストライク、満期、ボラティリティグリッドを格納
  - 外挿許可フラグを設定
  - コンストラクタでグリッド次元検証
  - _Requirements: 6.1, 6.2_

- [ ] 5.2 InterpolatedVolSurface の補間ロジック実装
  - BilinearInterpolator との統合
  - 2D バイリニア補間でボラティリティ導出
  - 満期スライスごとのスマイル補間オプション
  - _Requirements: 6.3, 6.4, 6.5_

- [ ] 5.3 InterpolatedVolSurface の VolatilitySurface 実装
  - volatility メソッドで 2D 補間を実行
  - 境界外クエリの処理（flat 外挿またはエラー）
  - ドメインメソッドでグリッド範囲を返却
  - 単体テストで補間精度を検証
  - _Requirements: 6.1, 6.4, 6.6_

- [ ] 6. AD 互換性と統合テスト
- [ ] 6.1 Dual64 による AD 互換性テスト
  - FlatCurve と Dual64 で金利感応度伝播を検証
  - InterpolatedCurve で補間を通じた微分連続性を検証
  - FlatVol と Dual64 で vega 計算を検証
  - InterpolatedVolSurface で 2D 補間の微分を検証
  - _Requirements: 8.1, 8.2, 8.4, 8.5_

- [ ] 6.2 smoothing 関数の活用確認
  - 分岐が必要な箇所で smooth_max / smooth_abs の使用を確認
  - AD テープ一貫性の検証
  - _Requirements: 8.3, 8.4_

- [ ] 6.3 モジュール公開エクスポートの最終確認
  - lib.rs からの公開 API を確認
  - ドキュメントコメントの追加（British English）
  - cargo test で全テストを実行
  - _Requirements: 1.1, 4.1, 7.1_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.5 | 2.1 |
| 2.1-2.5 | 2.2, 2.3 |
| 3.1-3.6 | 3.1, 3.2, 3.3, 3.4 |
| 4.1-4.5 | 4.1 |
| 5.1-5.4 | 4.2, 4.3 |
| 6.1-6.6 | 5.1, 5.2, 5.3 |
| 7.1-7.5 | 1.1, 1.2 |
| 8.1-8.5 | 2.3, 4.3, 6.1, 6.2 |

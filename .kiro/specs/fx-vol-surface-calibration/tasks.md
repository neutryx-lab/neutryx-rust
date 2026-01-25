# Implementation Plan

## Phase 1: コアタイプ定義（Infraレイヤー）

- [x] 1. FXボラティリティインストルメント型の実装
- [x] 1.1 (P) デルタ値の型安全なnewtypeを作成し、0-50%範囲のバリデーションを実装
  - デルタ値を表す`Delta`newtypeを作成
  - 範囲外（0以下または50超）の場合はエラーを返す
  - `InvalidDelta`エラーバリアントを定義
  - _Requirements: 1.6, 14.1_

- [x] 1.2 (P) FXボラティリティのマーケットコンベンションを定義
  - デルタタイプ（スポットデルタ、プレミアム調整デルタ）のenum定義
  - カットオフタイム、カレンダー、日数計算コンベンションを含む`FxVolConvention`構造体
  - EURUSDとUSDJPYのデフォルトコンベンションを設定
  - _Requirements: 1.4, 1.5_

- [x] 1.3 ATM、Butterfly、RiskReversal、デルタクォートのインストルメント型を実装
  - `FxVolInstrument` enumを作成（ATM、Butterfly、RiskReversal、DeltaQuotedバリアント）
  - 各バリアントに通貨ペア、満期、コンベンション、クォート値を含める
  - ビルダーパターンでfluent APIを提供
  - 満期日バリデーション（将来日付であること）
  - _Requirements: 1.1, 1.2, 1.3, 1.7_

- [x] 2. FXスワップインストルメント型の実装
- [x] 2.1 (P) スワップポイントの型とフォワードレート変換ロジックを実装
  - `SwapPoints`newtypeをスケーリングファクター付きで作成
  - フォワードレート計算: F = S + swap_points / scaling_factor
  - 通貨ペア固有のスケーリング（USDJPY: 100、EURUSD: 10000）
  - _Requirements: 7.4, 14.1_

- [x] 2.2 FXスワップインストルメントを標準テナー（ON〜1Y）サポート付きで実装
  - `FxSwapInstrument`構造体（通貨ペア、near/far日付、スポットレート、スワップポイント）
  - 標準テナーenum（ON、TN、SN、1W、2W、1M、2M、3M、6M、9M、1Y）
  - `FxSwapConvention`（スポットラグ、決済カレンダー、営業日コンベンション）
  - 日付バリデーション（near < far）と`InvalidDates`エラー
  - `implied_forward_rate()`メソッドの実装
  - _Requirements: 7.1, 7.2, 7.3, 7.5, 7.6, 7.7_

- [x] 3. クロスカレンシーベーシススワップ型の実装
- [x] 3.1 (P) ベーシススプレッドの型とベーシスポイント変換を実装
  - `BasisSpread`newtypeをベーシスポイント単位で作成
  - デシマル変換メソッド（bps / 10000）
  - _Requirements: 8.5, 14.1_

- [x] 3.2 XCCYベーシススワップを中長期テナー（2Y〜30Y）サポート付きで実装
  - `CrossCurrencyBasisSwap`構造体（国内/外貨、想定元本、満期、レッグ詳細、スプレッド）
  - `XccyLeg`構造体（通貨、レートインデックス、支払頻度、日数計算）
  - `XccyBasisConvention`（想定元本交換、MTMフラグ、スプレッドレッグ指定）
  - 通貨ミスマッチエラー検出
  - 標準テナーenum（2Y、3Y、4Y、5Y、7Y、10Y、15Y、20Y、25Y、30Y）
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.6, 8.7_

- [x] 4. VolSurface設定型の実装
- [x] 4.1 補間器タイプと満期補間メソッドのenumを定義
  - `InterpolatorType` enum（Sabr、SviRaw、Flat、Linear、CubicSpline）
  - `ExpiryInterpolation` enum（Linear、FlatForward、CubicSpline）
  - `ExtrapolationPolicy` enum（Flat、Linear、Error）
  - _Requirements: 2.3, 2.5_
  - **実装済み**: `volcube/config.rs` と `fx_calibration/types.rs`

- [x] 4.2 FXボラティリティサーフェス設定をSABRパラメータ付きで実装
  - `FxVolSurfaceConfig`構造体（補間器タイプ、満期補間、外挿ポリシー）
  - `SabrConfig`（alpha、beta、rho、nu、ベータ固定/カリブレーション選択）
  - 非互換補間器組み合わせのエラーハンドリング
  - G10通貨ペア向けデフォルト設定
  - _Requirements: 2.1, 2.2, 2.4, 2.6, 2.7_
  - **実装済み**: `fx_calibration/config.rs` (FxVolSurfaceConfig with builder pattern, presets, validation)

- [x] 5. 型安全性とエラーハンドリングの基盤実装
- [x] 5.1 (P) 追加のnewtypeを実装（Strike、Vol、ForwardPoints）
  - `Strike`newtype（正の値バリデーション）
  - `Vol`newtype（正の値バリデーション）
  - `ForwardPoints`newtype
  - 各型にバリデーションとエラーを実装
  - _Requirements: 14.1_
  - **実装済み**: `fx_calibration/types.rs` (Strike, Vol, ForwardPoints, ExpiryInterpolation with conversion methods)

- [x] 5.2 (P) FXカリブレーションエラー型をthiserrorで実装
  - `FxCalibrationError` enum（カーブとボラティリティ両方の失敗モード）
  - `thiserror::Error`トレイト実装
  - 数値不安定性エラーバリアント
  - JSON応答用の`serde::Serialize`実装
  - _Requirements: 14.2, 14.3, 14.4, 14.5, 14.6, 14.7_
  - **実装済み**: `fx_calibration/error.rs` (12 error variants with thiserror, convenience constructors)

## Phase 2: ビルダーコンポーネント（Pricerレイヤー）

- [x] 6. FXカーブトレイトとカリブレーション済みカーブの実装
- [x] 6.1 FXフォワードカーブトレイトを定義
  - `FxCurve<T>`トレイト（forward_rate、forward_points、spot_rate、discount_factor_domestic/foreign、currency_pair）
  - ジェネリック`T: Float`でAAD互換性を維持
  - エラー型定義（extrapolation beyond bounds等）
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6, 9.7_
  - **実装済み**: `fx_calibration/curve.rs` (FxCurve trait, FxCurveError, ExtrapolationPolicy)

- [x] 6.2 カリブレーション済みFXカーブを補間付きで実装
  - `CalibratedFxCurve<T>`構造体（通貨ペア、スポットレート、フォワードポイント曲線、ディスカウントカーブ参照）
  - `FxCurve<T>`トレイト実装
  - フォワードポイント補間ロジック
  - 外挿ポリシー適用
  - _Requirements: 9.5_
  - **実装済み**: `fx_calibration/curve.rs` (CalibratedFxCurve, SimpleFxCurve)

- [ ] 7. FXフォワードカーブビルダーの実装
- [ ] 7.1 短期FXスワップからのフォワードポイントブートストラップを実装
  - FXスワップからフォワードポイントを抽出するロジック
  - ディスカウントカーブを使用した暗黙フォワードレート計算
  - 入力インストルメントのリプライシングエラー検証
  - _Requirements: 10.2, 10.7_

- [ ] 7.2 XCCYベーシススワップからの長期フォワードポイントブートストラップを実装
  - `SequentialBootstrapper<T>`を再利用した解法
  - ベーシススプレッド適用ロジック
  - パーレートにリプライスするフォワードポイント求解
  - _Requirements: 10.2, 10.8_

- [ ] 7.3 短期・長期テナーブレンディングとビルダーAPIを完成
  - `FxForwardCurveBuilder<T>`メソッドチェーン（new → with_spot_rate → with_domestic_curve → with_foreign_curve → with_fx_swaps → with_xccy_basis_swaps → build）
  - 1Y-2Y遷移域でのスムーズなブレンディング
  - テナー重複時の優先順位設定（デフォルト: ≤1YはFXスワップ優先）
  - ディスカウントカーブ欠落エラー
  - _Requirements: 10.1, 10.3, 10.4, 10.5, 10.6_

- [ ] 8. カリブレーション済みFXボラティリティサーフェスの実装
- [ ] 8.1 VolatilitySurfaceトレイトを実装してストライク空間でのボラティリティ取得を可能に
  - `CalibratedFxVolSurface<T>`構造体（通貨ペア、参照日、スマイルマップ、FXカーブ参照）
  - `VolatilitySurface`トレイト実装（vol(expiry, strike) -> T）
  - カリブレーション済みポイントでの正確な値返却
  - 補間ポイントでの設定済み補間器使用
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 8.2 デルタ空間でのボラティリティクエリとスマイル抽出を実装
  - `vol_by_delta(expiry, delta) -> T`メソッド
  - デルタ-ストライク変換ロジック（FXカーブ使用）
  - `smile(expiry) -> VolSmile<T>`メソッド
  - 外挿境界外クエリのポリシー適用
  - ジェネリック`T: Float`でAAD互換性維持
  - _Requirements: 4.4, 4.5, 4.6, 4.7_

- [ ] 9. FXボラティリティサーフェスビルダーの実装
- [ ] 9.1 SABRカリブレーション機能を満期ごとに実装
  - 満期ごとのインストルメントグルーピング
  - ATMボラティリティ解決
  - SABRパラメータ（alpha、beta、rho、nu）カリブレーション
  - BF/RRフィッティングエラー最小化
  - _Requirements: 3.4, 2.2_

- [ ] 9.2 FxVolSurfaceBuilderのメソッドチェーンと診断機能を実装
  - `FxVolSurfaceBuilder<T>`メソッドチェーン（new → with_instruments → with_config → with_fx_curve → build）
  - `CalibrationDiagnostics`（反復回数、残差、収束状態、インストルメントごとエラー）
  - 収束失敗時の診断情報付きエラー
  - 重複満期インストルメントの最新クォート使用
  - 増分カリブレーション（既存サーフェスへのインストルメント追加）
  - _Requirements: 3.1, 3.2, 3.3, 3.5, 3.6, 3.7_

- [ ] 10. 遅延評価とキャッシュ最適化の実装
- [ ] 10.1 LazyFxVolSurfaceラッパーを遅延カリブレーション付きで実装
  - `LazyFxVolSurface<T>`構造体（ビルダー、キャッシュ、統計）
  - 初回`vol()`呼び出し時のカリブレーショントリガー
  - キャッシュ済み結果の再カリブレーションなし返却
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 10.2 キャッシュ無効化とスレッドセーフアクセスを実装
  - `invalidate()`メソッドでキャッシュクリアと再カリブレーション強制
  - 基礎インストルメントクォート変更時の自動無効化
  - `Arc<RwLock<>>`ベースのスレッドセーフキャッシング
  - `CacheStats`（ヒット、ミス、無効化回数）
  - _Requirements: 5.4, 5.5, 5.6, 5.7_

- [ ] 11. AAD計算グラフサポートの実装
- [ ] 11.1 ボラティリティサーフェス感応度計算を実装
  - `VolSurfaceSensitivity<T>`構造体（dVol/dATM、dVol/dBF、dVol/dRR）
  - BF/RRクォートに対する勾配計算
  - 不連続操作のスムーズ近似
  - _Requirements: 6.3, 6.4, 6.5_

- [ ] 11.2 Differentiableトレイト実装とフォワード/リバースモード微分を実装
  - `CalibratedFxVolSurface`への`Differentiable`トレイト実装
  - ADモード時のインストルメントクォートからボラティリティ出力への計算グラフ構築
  - タンジェント（フォワード）とアジョイント（リバース）両モードサポート
  - D3.js互換JSON出力
  - _Requirements: 6.1, 6.2, 6.6, 6.7_

## Phase 3: 統合（Pricerレイヤー）

- [ ] 12. FxMarketBuilderエンドツーエンドオーケストレーションの実装
- [ ] 12.1 OISカーブ構築の統合（既存CurveEngine使用）
  - `FxMarketBuilder<T>`構造体（通貨ペア、OISインストルメント、FXインストルメント、ボラティリティインストルメント）
  - 既存`CurveEngine`を使用したOISカーブブートストラップ
  - 事前構築済みカーブ使用オプション（`with_prebuilt_domestic_curve`、`with_prebuilt_foreign_curve`）
  - _Requirements: 11.1, 11.3, 11.7_

- [ ] 12.2 依存チェーン実行と部分ビルドメソッドを実装
  - `build()`メソッド: (1) 国内OISカーブ → (2) 外貨OISカーブ → (3) FXフォワードカーブ → (4) ボラティリティサーフェス
  - 部分ビルドメソッド（`build_discount_curves`、`build_fx_curve`、`build_vol_surface`）
  - 中間ステップ失敗時のエラー（失敗ステップと部分結果を含む）
  - 遅延評価モードサポート
  - _Requirements: 11.2, 11.4, 11.5, 11.6_

- [ ] 12.3 FxMarket結果型を定義
  - `FxMarket<T>`構造体（通貨ペア、国内ディスカウントカーブ、外貨ディスカウントカーブ、FXフォワードカーブ、オプショナルボラティリティサーフェス）
  - 全コンポーネントへのアクセサーメソッド
  - _Requirements: 11.8_

## Phase 4: WebApp統合とクリーンアップ

- [ ] 13. Demo WebAppエンドポイントの実装
- [ ] 13.1 (P) FXカーブ構築APIエンドポイントを実装
  - `/api/fxcurve/build`エンドポイント（FXスワップ、XCCYベーシススワップ、ディスカウントカーブ受付）
  - JSON形式でテナーごとのフォワードポイント返却
  - カリブレーション診断（反復回数、残差、収束状態）
  - 失敗時HTTP 422と詳細エラーメッセージ
  - _Requirements: 12.1, 12.2, 12.6, 12.7_

- [ ] 13.2 (P) ボラティリティサーフェスカリブレーションAPIエンドポイントを実装
  - `/api/fxvol/calibrate`エンドポイント（ボラティリティインストルメント、設定、FXカーブ参照受付）
  - 3D可視化用JSON形式でサーフェスデータ返却
  - `/api/fxvol/smile`エンドポイント（指定満期のスマイルデータ返却）
  - カリブレーション診断表示
  - _Requirements: 12.3, 12.4, 12.5, 12.6, 12.7_

- [ ] 13.3 リアルタイムサーフェス更新とインタラクティブUI機能を実装
  - インストルメントクォート変更時のWebSocketリアルタイムサーフェス更新
  - FXスワップポイント、ベーシススプレッド、BF/RRクォート編集UI
  - 即時再カリブレーション結果表示
  - _Requirements: 12.8, 12.9_

- [ ] 14. 既存実装のクリーンアップと移行
- [ ] 14.1 非推奨FxVolatilitySurface実装を特定・削除
  - 代替される既存実装の特定
  - 新APIへの依存モジュール更新
  - 不要な`fxvol_types.rs`、`fxvol_handlers.rs`の削除（機能が置き換えられた場合）
  - _Requirements: 13.1, 13.2, 13.3_

- [ ] 14.2 infra_masterのFxSwap統合と既存テスト更新
  - 既存`FxSwap`と拡張定義の統合（必要な場合）
  - 新APIに対する全既存テストのパスまたは更新確認
  - 公開API破壊時の移行ガイド作成
  - _Requirements: 13.4, 13.5, 13.6_

- [ ] 14.3 コードベースの検証とsteeringドキュメント更新
  - `cargo clippy --all-targets`実行と警告解消
  - `cargo test --workspace`実行とリグレッションなし確認
  - `structure.md`、`roadmap.md`のアーキテクチャ変更反映
  - _Requirements: 13.7, 13.8_

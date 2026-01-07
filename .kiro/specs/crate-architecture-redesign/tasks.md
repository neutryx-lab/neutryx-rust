# Implementation Plan

## Task 1: クレート名変更とワークスペース再構成

- [x] 1.1 pricer_kernelからpricer_engineへのクレート名変更
  - Cargo.tomlのパッケージ名をpricer_engineに更新
  - crates/ディレクトリをpricer_kernelからpricer_engineにリネーム
  - ワークスペースCargo.tomlのmembersを更新
  - 旧名でのエイリアスを提供（deprecation警告付き）
  - 全クレートの依存関係参照を更新
  - _Requirements: 7.1_

- [x] 1.2 (P) pricer_xvaからpricer_riskへのクレート名変更
  - Cargo.tomlのパッケージ名をpricer_riskに更新
  - crates/ディレクトリをpricer_xvaからpricer_riskにリネーム
  - ワークスペースCargo.tomlのmembersを更新
  - 旧名でのエイリアスを提供（deprecation警告付き）
  - 全クレートの依存関係参照を更新
  - _Requirements: 7.1_

- [x] 1.3 依存関係グラフの検証とビルド確認
  - cargo buildで全クレートのビルド成功を確認
  - cargo testで既存テストのパス確認
  - L1→L2→L3→L4の依存方向のみであることを検証
  - 循環依存が発生していないことを確認
  - _Requirements: 7.5_

## Task 2: Feature Flag設定とアセットクラス別条件付きコンパイル

- [ ] 2.1 pricer_modelsへのfeature flag追加
  - Cargo.tomlにequity, rates, credit, fx, commodity, exoticフィーチャーを定義
  - default = ["equity"]を設定
  - all = ["equity", "rates", "credit", "fx", "commodity", "exotic"]を設定
  - 各フィーチャー間の依存関係を適切に設定
  - _Requirements: 7.4_

- [ ] 2.2 instruments/配下のアセットクラス別サブモジュール作成
  - equity/サブモジュールを作成し、既存のVanillaOption, Barrier, Asian, Lookbackを移動
  - rates/サブモジュールを作成（スケルトン）
  - credit/サブモジュールを作成（スケルトン）
  - fx/サブモジュールを作成（スケルトン）
  - commodity/サブモジュールを作成（スケルトン）
  - exotic/サブモジュールを作成（スケルトン）
  - 各サブモジュールを対応するfeature flagでゲート
  - _Requirements: 7.2, 1.1_

- [ ] 2.3 models/配下のモデルカテゴリ別サブモジュール作成
  - equity/サブモジュールを作成し、既存のGBMを移動
  - rates/サブモジュールを作成（スケルトン）
  - hybrid/サブモジュールを作成（スケルトン）
  - 既存のStochasticModel traitとStochasticModelEnumを維持
  - _Requirements: 7.3_

## Task 3: マルチカーブ市場データ基盤

- [ ] 3.1 CurveNameとCurveEnumの定義
  - カーブ名の列挙型を定義（OIS, SOFR, TONAR, Euribor, Forward, Discount, Custom）
  - 既存のFlatCurve, InterpolatedCurveを統合するenum型を定義
  - 全型をジェネリックFloat型で維持しAD互換性を確保
  - _Requirements: 2.5_

- [ ] 3.2 CurveSet構造体の実装
  - 複数カーブを名前付きで管理するコンテナを実装
  - カーブの登録・取得メソッドを実装
  - ディスカウントカーブとフォワードカーブの分離取得を実装
  - _Requirements: 2.1, 2.2_

- [ ] 3.3 CreditCurveトレイトとHazardRateCurve実装
  - ハザードレート・生存確率・デフォルト確率の計算インターフェースを定義
  - テナーとハザードレートの期間構造を持つ実装を作成
  - 生存確率の積分計算を実装
  - _Requirements: 2.3, 5.3_

- [ ] 3.4 MarketDataError拡張
  - カーブ未検出エラーを追加
  - 無効な満期エラーを追加
  - 補間失敗エラーを追加
  - 欠損データの詳細を含むエラーメッセージを提供
  - _Requirements: 2.4_

- [ ] 3.5 (P) FxVolatilitySurface実装
  - デルタ×満期グリッドでのボラティリティサーフェスを実装
  - VolatilitySurfaceトレイトを実装
  - ATMボラティリティ取得メソッドを追加
  - _Requirements: 6.5_

## Task 4: Instrumentトレイトと階層的Enum再構成

- [ ] 4.1 Instrumentトレイトの定義
  - 価格計算・Greeks計算・キャッシュフロー取得メソッドを定義
  - 満期・通貨取得メソッドを追加
  - 全商品が実装すべき共通インターフェースを確立
  - _Requirements: 1.3_

- [ ] 4.2 InstrumentEnumの階層的enum構造への再構成
  - トップレベルのアセットクラス別enumを定義（Equity, Rates, Credit, Fx, Commodity, Exotic）
  - 株式商品のサブenumを定義
  - 既存の商品をEquityサブenumに統合
  - トレイト実装をenumに委譲
  - 静的ディスパッチを維持
  - _Requirements: 1.2, 1.5_

- [ ] 4.3 後方互換性の維持
  - 既存のInstrument enum APIを維持
  - 新旧両方のアクセスパターンをサポート
  - deprecation警告で移行を促進
  - _Requirements: 1.5_

## Task 5: スケジュール生成モジュール

- [ ] 5.1 基本型の定義
  - スケジュール構造体を定義（期間リスト、支払日、計算期間開始・終了）
  - 期間構造体を定義（開始、終了、支払日、日数計算規約）
  - 頻度列挙型を定義（年次、半期、四半期、月次、週次、日次）
  - _Requirements: 1.4_

- [ ] 5.2 BusinessDayConventionとDayCountConvention拡張
  - 営業日調整規約の列挙型を定義（Following, ModifiedFollowing, Preceding, ModifiedPreceding, Unadjusted）
  - 既存の日数計算規約にAct360, Act365, Thirty360を追加
  - _Requirements: 4.5_

- [ ] 5.3 ScheduleBuilderの実装
  - ビルダーパターンでスケジュール生成器を実装
  - 開始日、終了日、頻度、営業日調整規約、日数計算規約の設定メソッドを実装
  - IMM日付生成をサポート
  - _Requirements: 4.5_

## Task 6: 金利デリバティブ商品

- [ ] 6.1 (P) InterestRateSwap構造体の実装
  - IRS構造体を定義（ノーショナル、固定レグ、変動レグ、通貨）
  - 固定レグ構造体を定義（スケジュール、固定レート、日数計算規約）
  - 変動レグ構造体を定義（スケジュール、スプレッド、インデックス、日数計算規約）
  - レートインデックス列挙型を定義（SOFR, TONAR, Euribor3M, Euribor6M）
  - 金利商品サブenumにSwapバリアントを追加
  - _Requirements: 4.1_

- [ ] 6.2 (P) Swaption構造体の実装
  - Swaption構造体を定義（原資産スワップ、満期、ストライク、オプションタイプ）
  - Swaptionタイプ列挙型を定義（Payer, Receiver）
  - 金利商品サブenumにSwaptionバリアントを追加
  - _Requirements: 4.3_

- [ ] 6.3 (P) Cap/Floor構造体の実装
  - Cap構造体を定義（ノーショナル、スケジュール、ストライク、インデックス）
  - Floor構造体を定義（同様）
  - Collar構造体を定義（キャップストライク、フロアストライク）
  - 金利商品サブenumにCap, Floorバリアントを追加
  - _Requirements: 4.3_

- [ ] 6.4 IRS評価ロジックの実装
  - フォワードレート計算ロジックを実装
  - 変動レグと固定レグのキャッシュフロー計算を実装
  - CurveSetからディスカウント・フォワードカーブを取得して評価
  - _Requirements: 4.2_

- [ ] 6.5 Swaption解析解の実装
  - Black76モデルによるSwaption価格計算を実装
  - Bachelierモデル（正規モデル）によるSwaption価格計算を実装
  - 解析解モジュールに追加
  - _Requirements: 4.4_

## Task 7: クレジットデリバティブ商品

- [ ] 7.1 (P) CreditDefaultSwap構造体の実装
  - CDS構造体を定義（参照エンティティ、ノーショナル、スプレッド、満期）
  - クレジット商品サブenumにCdsバリアントを追加
  - Instrumentトレイトを実装
  - _Requirements: 5.1_

- [ ] 7.2 CDS評価ロジックの実装
  - ハザードレートカーブから生存確率を計算
  - プロテクションレグPV計算を実装
  - プレミアムレグPV計算を実装
  - _Requirements: 5.2_

- [ ] 7.3 デフォルトシミュレーションのMC拡張
  - 生存確率の逆関数でデフォルト時刻をサンプリング
  - MCエンジンにデフォルトイベント処理を追加
  - _Requirements: 5.4_

## Task 8: 為替デリバティブ商品

- [ ] 8.1 CurrencyPair構造体の実装
  - 通貨ペア構造体を定義（ベース通貨、クォート通貨）
  - スポットレート管理機能を追加
  - 既存のCurrency列挙型を活用
  - _Requirements: 6.3_

- [ ] 8.2 (P) FxOption構造体の実装
  - FXオプション構造体を定義（通貨ペア、ストライク、満期、オプションタイプ）
  - 為替商品サブenumにOptionバリアントを追加
  - Instrumentトレイトを実装
  - _Requirements: 6.1_

- [ ] 8.3 (P) FxForward構造体の実装
  - FXフォワード構造体を定義（通貨ペア、フォワードレート、満期、ノーショナル）
  - 為替商品サブenumにForwardバリアントを追加
  - _Requirements: 6.1_

- [ ] 8.4 Garman-Kohlhagenモデルの実装
  - GK公式によるFXオプション価格計算を実装
  - 国内・外国金利カーブを使用したディスカウント
  - 解析解モジュールに追加
  - _Requirements: 6.2_

## Task 9: 確率モデル拡張

- [ ] 9.1 StochasticModelトレイトへのnum_factors追加
  - ファクター数取得メソッドを追加
  - 既存のGBMでnum_factors = 1を実装
  - 1ファクター/2ファクター/マルチファクターモデルの統一的扱い
  - _Requirements: 3.1_

- [ ] 9.2 Hull-White 1Fモデルの実装
  - Hull-White構造体を定義（平均回帰速度、ボラティリティ、初期カーブ）
  - StochasticModelトレイトを実装
  - 短期金利パス生成のevolve_stepを実装
  - 金利モデルサブモジュールに追加
  - _Requirements: 3.2_

- [ ] 9.3 (P) CIRモデルの実装
  - Cox-Ingersoll-Ross構造体を定義
  - StochasticModelトレイトを実装
  - Feller条件のバリデーションを追加
  - 金利モデルサブモジュールに追加
  - _Requirements: 3.2_

- [ ] 9.4 StochasticModelEnumへの新モデル追加
  - HullWhite, CIRバリアントをStochasticModelEnumに追加
  - 静的ディスパッチを維持
  - enumバリアント追加のみで拡張可能な構造を確認
  - _Requirements: 3.3_

- [ ] 9.5 CorrelatedModels（Cholesky分解）の実装
  - 相関モデル構造体を定義（モデルリスト、相関行列、Cholesky分解済み行列）
  - 相関行列のCholesky分解を実装
  - 相関ブラウン運動の生成を実装
  - ハイブリッドモデルサブモジュールに追加
  - _Requirements: 3.4_

## Task 10: キャリブレーション基盤

- [ ] 10.1 Levenberg-Marquardtソルバーの実装
  - LMソルバー構造体を定義
  - 非線形最小二乗法の反復計算を実装
  - 収束判定とパラメータ更新ロジックを実装
  - 数学ソルバーモジュールに追加
  - _Requirements: 8.3_

- [ ] 10.2 Calibratorトレイトの定義
  - キャリブレーション・目的関数・制約条件メソッドを定義
  - キャリブレーション結果構造体を定義（収束フラグ、イテレーション数、残差、最終パラメータ）
  - 制約条件構造体を定義
  - _Requirements: 8.1_

- [ ] 10.3 CalibrationError型の実装
  - キャリブレーションエラー構造体を定義（種別、残差、イテレーション数、メッセージ）
  - エラー種別列挙型を定義（NotConverged, InvalidConstraint, NumericalInstability, InsufficientData）
  - 残差、イテレーション数、収束判定基準を含む詳細情報を提供
  - _Requirements: 8.4_

- [ ] 10.4 SwaptionCalibratorの実装
  - Swaptionキャリブレータ構造体を定義
  - Swaptionボラティリティサーフェスへのキャリブレーションを実装
  - Hull-Whiteモデルのパラメータ（平均回帰速度、ボラティリティ）を推定
  - _Requirements: 8.2_

- [ ] 10.5 Enzyme ADを活用した勾配計算
  - キャリブレーション目的関数のAD対応
  - 勾配計算によるキャリブレーション高速化
  - enzyme-modeでのテスト
  - _Requirements: 8.5_

## Task 11: リスクファクター管理

- [ ] 11.1 RiskFactorトレイトの定義
  - リスクファクタートレイトを定義（ファクター種別、バンプ、シナリオ適用）
  - リスクファクター種別列挙型を定義（InterestRate, Credit, Fx, Equity, Commodity, Volatility）
  - トレイトモジュールに追加
  - _Requirements: 9.1_

- [ ] 11.2 BumpScenarioとRiskFactorShiftの実装
  - リスクファクターシフト構造体を定義（ファクター種別、シフト種別、値）
  - シフト種別列挙型を定義（Absolute, Relative, Parallel, Twist, Butterfly）
  - 各リスクファクターの独立・同時シフトをサポート
  - _Requirements: 9.2_

- [ ] 11.3 GreeksAggregatorの実装
  - Greeks集計構造体を定義
  - 集計手法列挙型を定義（Simple, RiskWeighted, CorrelationAdjusted）
  - ポートフォリオレベルDelta, Gamma, Vegaを計算するメソッドを実装
  - ポートフォリオGreeks構造体を定義
  - _Requirements: 9.3_

- [ ] 11.4 ScenarioEngineの実装
  - シナリオエンジン構造体を定義
  - シナリオ構造体を定義（名前、シフトリスト）
  - シナリオ実行・全シナリオ実行メソッドを実装
  - シナリオPnL構造体を定義
  - _Requirements: 9.4_

- [ ] 11.5 プリセットシナリオの追加
  - パラレルシフト（+1bp, +10bp, +100bp）を追加
  - ツイストシナリオ（短期↑長期↓）を追加
  - バタフライシナリオ（中期スパイク）を追加
  - プリセットモジュールに追加
  - _Requirements: 9.5_

## Task 12: エキゾチックデリバティブ商品

- [ ] 12.1 (P) VarianceSwap構造体の実装
  - バリアンススワップ構造体を定義（ストライク、バリアンスノーショナル、観測日）
  - ボラティリティスワップ構造体を定義（バリアンススワップとの違いを明確化）
  - エキゾチック商品サブenumにVarianceSwap, VolatilitySwapバリアントを追加
  - _Requirements: 11.1, 11.8_

- [ ] 12.2 VarianceSwap評価ロジックの実装
  - ログリターンの二乗和から実現バリアンスを計算
  - レプリケーションまたはMCで公正バリアンスストライクを算出
  - _Requirements: 11.2_

- [ ] 12.3 (P) Cliquet構造体の実装
  - クリケット構造体を定義（リセット日、ローカルキャップ/フロア、グローバルキャップ/フロア）
  - エキゾチック商品サブenumにCliquetバリアントを追加
  - _Requirements: 11.3_

- [ ] 12.4 (P) Autocallable構造体の実装
  - オートコーラブル構造体を定義（観測日、早期償還バリア、クーポン条件、ノックインプット）
  - エキゾチック商品サブenumにAutocallableバリアントを追加
  - _Requirements: 11.4_

- [ ] 12.5 (P) Rainbow（BestOf/WorstOf）構造体の実装
  - レインボー構造体を定義（原資産リスト、ペイオフタイプ、相関）
  - ペイオフタイプ列挙型を定義（BestOf, WorstOf, Spread）
  - エキゾチック商品サブenumにRainbowバリアントを追加
  - _Requirements: 11.5_

- [ ] 12.6 (P) QuantoOption構造体の実装
  - クオントオプション構造体を定義（原資産通貨、決済通貨、クオント調整）
  - エキゾチック商品サブenumにQuantoバリアントを追加
  - _Requirements: 11.6_

- [ ] 12.7 Longstaff-Schwartz法の実装
  - LSM構造体を定義（基底関数タイプ、基底関数数、two-pass使用フラグ）
  - 基底関数タイプ列挙型を定義（Polynomial, Laguerre, Hermite）
  - 継続価値計算メソッドを実装
  - 行使境界探索メソッドを実装
  - アメリカンオプションモジュールに追加
  - _Requirements: 11.7_

- [ ] 12.8 BermudanSwaption評価の実装
  - バミューダンSwaption構造体を定義
  - Longstaff-Schwartz法による早期行使境界推定
  - エキゾチック商品サブenumにBermudanバリアントを追加
  - _Requirements: 11.7_

## Task 13: XVA拡張

- [ ] 13.1 Wrong-Way Riskモジュールの実装
  - Wrong-Way Risk構造体を定義
  - CVA計算でのWWR考慮オプションを追加
  - 相関パラメータによるエクスポージャー調整
  - XVAモジュールに追加
  - _Requirements: 5.5_

- [ ] 13.2 (P) マルチカレンシーXVA対応
  - 各取引の決済通貨と評価通貨の変換を自動処理
  - CurrencyPairを使用した為替レート適用
  - _Requirements: 6.4_

## Task 14: パフォーマンス最適化

- [ ] 14.1 SoAレイアウトの維持確認
  - pricer_riskでのSoA構造の維持を確認
  - 新規構造体でもSoA対応を検討
  - _Requirements: 10.1_

- [ ] 14.2 (P) Rayon並列化の適用確認
  - Portfolio評価の並列化が維持されていることを確認
  - 新規アセットクラスでの並列評価テスト
  - _Requirements: 10.2_

- [ ] 14.3 (P) ワークスペースバッファパターンの適用
  - 新規評価パスでのバッファ再利用を確認
  - メモリアロケーション最小化の検証
  - _Requirements: 10.3_

- [ ] 14.4 (P) チェックポイント機能の統合確認
  - 新規モデル・商品でのcheckpointing対応
  - メモリ使用量と再計算のトレードオフ設定
  - _Requirements: 10.4_

- [ ] 14.5 criterionベンチマーク追加
  - IRS評価ベンチマーク追加
  - CDS評価ベンチマーク追加
  - FXオプション評価ベンチマーク追加
  - Swaption評価ベンチマーク追加
  - パフォーマンス回帰検出の仕組み確認
  - _Requirements: 10.5_

## Task 15: 統合テスト

- [ ] 15.1 IRS評価の統合テスト
  - Schedule生成 → CurveSet構築 → price()呼び出し → 既知値との比較
  - HullWhiteモデルでのMC評価テスト
  - _Requirements: 4.1, 4.2_

- [ ] 15.2 (P) Swaption評価の統合テスト
  - HullWhiteキャリブレーション → MC価格 → Black76解析解との比較
  - キャリブレーション収束確認
  - _Requirements: 4.3, 4.4, 8.2_

- [ ] 15.3 (P) CDS評価の統合テスト
  - HazardRateCurve構築 → プロテクション/プレミアムレグPV計算
  - 既知のCDSスプレッドとの整合性確認
  - _Requirements: 5.1, 5.2_

- [ ] 15.4 (P) FXオプション評価の統合テスト
  - Garman-Kohlhagenモデルでの価格計算
  - マルチカレンシーポートフォリオでのXVA計算
  - _Requirements: 6.1, 6.2, 6.4_

- [ ] 15.5 全アセットクラス横断のPortfolio XVAテスト
  - Equity + Rates + Credit + FX商品の混合ポートフォリオ
  - ExposureProfile生成 → CVA/DVA/FVA計算
  - 各リスクファクターの感応度確認
  - _Requirements: 9.3, 9.4, 5.5_

- [ ] 15.6 (P) エキゾチック商品の統合テスト
  - VarianceSwap評価とGreeks計算
  - Autocallable/Cliquetの早期償還シミュレーション
  - BermudanSwaptionのLSM評価
  - _Requirements: 11.1, 11.2, 11.7_
